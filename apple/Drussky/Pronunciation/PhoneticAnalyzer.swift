import Accelerate
import Foundation

enum PronunciationError: Error { case unreadableAudio, nothingHeard, noRecognizer }

/// Sound-by-sound pronunciation feedback for Russian, on device.
///
/// 1. The take is trimmed to its speech and run through `PhoneRecognizer` (CTC posteriors over
///    IPA symbols, 50 frames/s).
/// 2. `RussianG2P` gives the sounds the target should have.
/// 3. Each expected sound is scored GOP-style: the log-likelihood of the whole expected
///    sequence against the same sequence with that one sound replaced by each named mistake
///    (ы→и, soft→hard, щ→ш, a full unstressed о, a voiced final consonant…) or deleted
///    (`Scorer`). The margin decides the verdict and which mistake it was.
/// 4. A free decode aligned to the expectation with phonetic distances finds added sounds.
/// 5. `Prosody` measures each syllable's length, loudness and pitch from the alignment, and
///    vowel fullness from the posteriors, to say which syllable carried the stress.
/// 6. `Tips` turns the two most important findings into advice tied to pairs.json contrasts.
///
/// Without the recognizer it compares words recognised by `transcribe` instead.
final class PhoneticAnalyzer: PronunciationAnalyzer, Sendable {
    let recognizer: PhoneRecognizer?
    let transcribe: @Sendable (URL, String) async -> String?

    init(recognizer: PhoneRecognizer? = PhoneRecognizer.bundled,
         transcribe: @escaping @Sendable (URL, String) async -> String? = { _, _ in nil }) {
        self.recognizer = recognizer
        self.transcribe = transcribe
    }

    var isPhonetic: Bool { recognizer != nil }

    func analyze(take: URL, target: String, model: URL?) async throws -> PronunciationReport {
        guard let x = Pitch.samples(take), !x.isEmpty else { throw PronunciationError.unreadableAudio }
        return try await analyze(samples: x, take: take, target: target)
    }

    func analyze(samples x: [Float], take: URL? = nil, target: String) async throws -> PronunciationReport {
        let expected = RussianG2P.transcribe(target)
        let speech = Speech.find(x)
        guard let recognizer else {
            guard let take, let heard = await transcribe(take, target) else { throw PronunciationError.noRecognizer }
            return WordFallback.report(target: target, expected: expected, transcript: heard, speech: speech)
        }
        // 0.3 s either side of the speech, from the take where it has it, else silence: the
        // recognizer misreads the edges of audio that starts or stops mid-sound.
        let margin = Int(0.3 * 16_000)
        let lo = max(0, speech.start - margin), hi = min(x.count, speech.end + margin)
        let padBefore = margin - (speech.start - lo), padAfter = margin - (hi - speech.end)
        let segment = [Float](repeating: 0, count: padBefore) + x[lo..<hi] + [Float](repeating: 0, count: padAfter)
        let post = try await recognizer.posteriors(segment)
        guard post.frames > 4 else { throw PronunciationError.nothingHeard }
        // Times in the report are seconds into the trimmed take (from speech onset).
        let shift = Double(margin) / 16_000
        let scorer = Scorer(post: post, expected: expected, recognizer: recognizer)
        let scored = scorer.run()
        var sounds = scored.judgements
        let heard = scorer.freeDecode()
        sounds = Scorer.insertExtras(into: sounds, expected: expected, heard: heard, scored: scored)
        let timeFix: (Phone) -> Phone = { p in
            var q = p
            q.start = p.start.map { max(0, $0 - shift) }
            q.end = p.end.map { max(0, $0 - shift) }
            return q
        }
        sounds = sounds.map { j in
            var j = j
            j.expected = j.expected.map(timeFix)
            j.heard = j.heard.map(timeFix)
            return j
        }
        let prosody = Prosody.measure(samples: segment, expected: expected, spans: scored.spans, chosen: scored.chosen, post: post, scorer: scorer,
                                      timeShift: shift)
        let goodCount = sounds.filter { $0.expected != nil && ($0.verdict == .good || $0.verdict == .close) }.count
        let expectedCount = max(1, sounds.filter { $0.expected != nil }.count)
        let tips = Tips.make(target: target, expected: expected, sounds: sounds, mistakes: scored.mistakes,
                             stress: prosody.findings)
        return PronunciationReport(
            target: target, transcript: nil,
            expected: sounds.compactMap(\.expected),
            heard: heard.map { timeFix($0.phone) },
            sounds: sounds, syllables: prosody.syllables, stressedSyllable: prosody.stressedSyllable,
            accuracy: Double(goodCount) / Double(expectedCount), tips: tips,
            speechStart: Double(speech.start) / 16_000, speechEnd: Double(speech.end) / 16_000)
    }
}

/// Where the speech is: 10 ms RMS above 8% of the loudest frame (as `Envelope` trims).
enum Speech {
    static func find(_ x: [Float]) -> (start: Int, end: Int) {
        let hop = 160
        let n = x.count / hop
        guard n > 0 else { return (0, x.count) }
        var rms = [Float](repeating: 0, count: n)
        x.withUnsafeBufferPointer { p in
            for i in 0..<n { vDSP_rmsqv(p.baseAddress! + i * hop, 1, &rms[i], vDSP_Length(hop)) }
        }
        guard let peak = rms.max(), peak > 1e-5,
              let first = rms.firstIndex(where: { $0 > peak * 0.08 }),
              let last = rms.lastIndex(where: { $0 > peak * 0.08 }) else { return (0, x.count) }
        return (first * hop, min(x.count, (last + 1) * hop))
    }
}

/// GOP-style scoring of each expected sound against named alternatives.
struct Scorer {
    let post: Posteriors
    let expected: Transcription
    let recognizer: PhoneRecognizer
    let options: [SoundOptions]
    /// Token spellings per sound: good ones, and per mistake its spellings not shared with a good one.
    let good: [[[Int]]]
    let wrong: [[(Mistake, [[Int]])]]

    struct Result {
        var judgements: [PhoneJudgement]
        /// Per expected sound: the frames its chosen spelling occupied.
        var spans: [ClosedRange<Int>?]
        /// Per expected sound: the mistake that won, with its margin (negative = mistake more likely).
        var mistakes: [(Mistake?, Float)]
        var chosen: [[Int]]
    }

    init(post: Posteriors, expected: Transcription, recognizer: PhoneRecognizer) {
        self.post = post
        self.expected = expected
        self.recognizer = recognizer
        options = expected.sounds.map(PhoneSet.options)
        var g: [[[Int]]] = [], w: [[(Mistake, [[Int]])]] = []
        for o in options {
            var goodSp: [[Int]] = []
            for ipa in o.good { for sp in PhoneSet.spellings(ipa) { if let t = recognizer.tokens(sp), !goodSp.contains(t) { goodSp.append(t) } } }
            if goodSp.isEmpty { goodSp = [[]] }
            g.append(goodSp)
            w.append(o.errors.compactMap { m in
                let sps = PhoneSet.spellings(m.ipa).compactMap(recognizer.tokens).filter { !goodSp.contains($0) }
                return sps.isEmpty ? nil : (m, sps)
            })
        }
        good = g
        wrong = w
    }

    func flat(_ seq: [[Int]]) -> [Int] { seq.flatMap { $0 } }

    /// Frames covering sounds lo...hi of `seq` under `spans`, padded a little.
    func window(_ spans: [ClosedRange<Int>?], _ lo: Int, _ hi: Int) -> Range<Int> {
        let a = (lo...hi).compactMap { spans[$0]?.lowerBound }.min() ?? 0
        let b = (lo...hi).compactMap { spans[$0]?.upperBound }.max() ?? post.frames - 1
        let from = lo == 0 ? 0 : max(0, a - 2), to = hi == spans.count - 1 ? post.frames : min(post.frames, b + 3)
        return from..<max(from + 1, to)
    }

    /// Sound spans from a token-level alignment of `seq`.
    func soundSpans(_ seq: [[Int]]) -> [ClosedRange<Int>?] {
        let tok = CTC.align(post, flat(seq))
        var out: [ClosedRange<Int>?] = []
        var k = 0
        for s in seq {
            let parts = tok[k..<(k + s.count)].compactMap { $0 }
            out.append(parts.isEmpty ? nil : (parts.map(\.lowerBound).min()!...parts.map(\.upperBound).max()!))
            k += s.count
        }
        return out
    }

    func localLL(_ seq: [[Int]], spans: [ClosedRange<Int>?], at i: Int, with alt: [Int]?) -> Float {
        let lo = max(0, i - 2), hi = min(seq.count - 1, i + 2)
        var sub = Array(seq[lo...hi])
        if let alt { sub[i - lo] = alt } else { sub.remove(at: i - lo) }
        return CTC.logLikelihood(post, flat(sub), frames: window(spans, lo, hi))
    }

    func run() -> Result {
        let n = expected.sounds.count
        guard n > 0 else { return Result(judgements: [], spans: [], mistakes: [], chosen: []) }
        var seq = good.map { $0[0] }
        var spans = soundSpans(seq)
        // Pick the best-fitting correct spelling of each sound (l or ɫ, tʲ or tsʲ…).
        for i in 0..<n where good[i].count > 1 {
            var best = seq[i], bestLL = -Float.infinity
            for g in good[i] {
                let ll = localLL(seq, spans: spans, at: i, with: g)
                if ll > bestLL { bestLL = ll; best = g }
            }
            seq[i] = best
        }
        spans = soundSpans(seq)
        var judgements: [PhoneJudgement] = []
        var mistakes: [(Mistake?, Float)] = []
        var heardSeq = seq
        for i in 0..<n {
            let base = localLL(seq, spans: spans, at: i, with: seq[i])
            var bestAlt: (Mistake?, [Int]?, Float) = (nil, nil, -Float.infinity)
            for (m, sps) in wrong[i] {
                for sp in sps {
                    let ll = localLL(seq, spans: spans, at: i, with: sp)
                    if ll > bestAlt.2 { bestAlt = (m, sp, ll) }
                }
            }
            let del = localLL(seq, spans: spans, at: i, with: nil)
            let named = bestAlt
            if del > bestAlt.2 + Self.deletionBias { bestAlt = (nil, nil, del) }
            let margin = base - bestAlt.2
            let s = expected.sounds[i]
            var verdict: PhoneJudgement.Verdict
            if margin >= Self.goodMargin { verdict = .good }
            else if margin >= threshold(bestAlt.0, at: i) { verdict = .close }
            else { verdict = bestAlt.1 == nil ? .missing : .substituted }
            if verdict == .missing, margin > -Self.missingMargin || s.optional { verdict = .close }
            let span = spans[i]
            let mine = Phone(ipa: s.ipa, start: span.map { post.time($0.lowerBound) - 0.01 },
                             end: span.map { post.time($0.upperBound) + 0.01 }, confidence: Double(1 / (1 + Foundation.exp(-margin))))
            var heard: Phone? = mine
            if verdict == .substituted, let m = bestAlt.0 {
                heard = Phone(ipa: m.ipa, start: mine.start, end: mine.end, confidence: Double(1 / (1 + Foundation.exp(margin))))
                heardSeq[i] = bestAlt.1 ?? seq[i]
            } else if verdict == .missing {
                heard = nil
            }
            judgements.append(PhoneJudgement(expected: mine, heard: heard, verdict: verdict, letterRange: s.letters))
            let winner: Mistake? = verdict == .substituted ? bestAlt.0 : (verdict == .close ? named.0 : nil)
            mistakes.append((winner, margin))
        }
        return Result(judgements: judgements, spans: spans, mistakes: mistakes, chosen: seq)
    }

    /// How far below zero the margin must go before this mistake is called. ZIPA's training
    /// transcriptions rarely mark softness before a vowel (it writes мил as [mil]), so a
    /// soft consonant before a vowel reads as hard on native speech too: there only a clear
    /// preference for the hard sound counts (calibrated on the hard/soft pairs, README).
    func threshold(_ m: Mistake?, at i: Int) -> Float {
        guard let m else { return Self.substitutionMargin }
        // Unnamed mistakes (a vowel heard as some other vowel, l as w) need clearer evidence.
        if m.kind == .other { return -2 }
        // A final consonant left voiced: natives in connected speech are often heard so too.
        if m.kind == .voicing, expected.sounds[i].underlying != nil { return -2 }
        guard m.kind == .hardened else { return Self.substitutionMargin }
        let next = i + 1 < expected.sounds.count ? expected.sounds[i + 1] : nil
        // Word-finally ZIPA marks тʲ reliably; final лʲ нʲ сʲ мʲ much less (README).
        if next?.isVowel != true, ["tʲ", "dʲ"].contains(expected.sounds[i].ipa) { return -2 }
        return -5
    }

    /// Margins in nats of local log-likelihood. Calibrated on the bundled native recordings
    /// (see README): ≥ 2 good, 0…2 close, below 0 the named mistake or deletion wins.
    static let goodMargin: Float = 2
    static let substitutionMargin: Float = 0
    /// A deletion must beat the best named mistake by this much before a sound is "missing":
    /// short sounds are cheap to delete in CTC.
    static let deletionBias: Float = 1.5
    /// …and the sound itself must be clearly worse than nothing.
    static let missingMargin: Float = 3

    /// Vowel fullness at an expected vowel: log-likelihood of the full vowel minus the reduced one.
    func fullness(at i: Int, spans: [ClosedRange<Int>?], seq: [[Int]], full: String, reduced: String) -> Float? {
        guard let f = recognizer.tokens(full), let r = recognizer.tokens(reduced) else { return nil }
        return localLL(seq, spans: spans, at: i, with: f) - localLL(seq, spans: spans, at: i, with: r)
    }

    struct Heard { var phone: Phone; var peak: Float }

    /// The recognizer's own best guess, read back into canonical phones.
    func freeDecode() -> [Heard] {
        let toks = CTC.greedy(post)
        var out: [Heard] = []
        var i = 0
        let spellingTable: [([Int], String)] = PhoneSet.inventory.flatMap { ipa in
            PhoneSet.spellings(ipa).compactMap { sp in recognizer.tokens(sp).map { ($0, ipa) } }
        }.sorted { $0.0.count > $1.0.count }
        while i < toks.count {
            var matched = false
            for (sp, ipa) in spellingTable where i + sp.count <= toks.count && sp.count > 0 {
                if (0..<sp.count).allSatisfy({ toks[i + $0].token == sp[$0] }) {
                    let fr = toks[i].frames.lowerBound...toks[i + sp.count - 1].frames.upperBound
                    out.append(Heard(phone: Phone(ipa: ipa, start: post.time(fr.lowerBound) - 0.01, end: post.time(fr.upperBound) + 0.01,
                                                  confidence: Double(toks[i].peak)), peak: toks[i].peak))
                    i += sp.count; matched = true; break
                }
            }
            if !matched {
                let t = toks[i]
                let sym = recognizer.token(t.token)
                if !["ʲ", "ː", "̴", "▁"].contains(sym) {
                    out.append(Heard(phone: Phone(ipa: sym, start: post.time(t.frames.lowerBound) - 0.01, end: post.time(t.frames.upperBound) + 0.01,
                                                  confidence: Double(t.peak)), peak: t.peak))
                }
                i += 1
            }
        }
        return out
    }

    /// Adds `.extra` judgements for confidently heard sounds the alignment cannot place.
    static func insertExtras(into sounds: [PhoneJudgement], expected: Transcription, heard: [Heard], scored: Result) -> [PhoneJudgement] {
        let exp = expected.sounds.map(\.ipa)
        let alignment = Align.run(exp, heard.map(\.phone.ipa))
        var extrasBefore: [Int: [PhoneJudgement]] = [:]
        var nextExpected = 0
        for step in alignment {
            switch step {
            case .match(let e, _), .substitute(let e, _): nextExpected = e + 1
            case .delete(let e): nextExpected = e + 1
            case .insert(let h):
                let hd = heard[h]
                // Only clear, vowel-like or long additions; ignore a stray glide or blank-ish blip.
                guard hd.peak > 0.7, PhoneSet.isVowel(hd.phone.ipa) else { continue }
                // Not next to a sound already judged wrong (that substitution explains it).
                let near = [nextExpected - 1, nextExpected].filter { $0 >= 0 && $0 < sounds.count }
                if near.contains(where: { sounds[$0].verdict == .substituted || sounds[$0].verdict == .missing }) { continue }
                let letters = nextExpected < expected.sounds.count ? expected.sounds[nextExpected].letters
                    : expected.sounds.last.map { $0.letters.upperBound..<$0.letters.upperBound }
                extrasBefore[nextExpected, default: []].append(PhoneJudgement(expected: nil, heard: hd.phone, verdict: .extra, letterRange: letters))
            }
        }
        var out: [PhoneJudgement] = []
        for (i, s) in sounds.enumerated() {
            out += extrasBefore[i] ?? []
            out.append(s)
        }
        out += extrasBefore[sounds.count] ?? []
        return out
    }
}

/// Weighted edit alignment of two phone strings by articulatory distance.
enum Align {
    enum Step: Equatable { case match(Int, Int), substitute(Int, Int), delete(Int), insert(Int) }

    static func run(_ a: [String], _ b: [String], gap: Double = 0.6) -> [Step] {
        let n = a.count, m = b.count
        var d = [[Double]](repeating: [Double](repeating: 0, count: m + 1), count: n + 1)
        for i in 0...n { d[i][0] = Double(i) * gap }
        for j in 0...m { d[0][j] = Double(j) * gap }
        if n > 0 && m > 0 {
            for i in 1...n { for j in 1...m {
                d[i][j] = min(d[i - 1][j - 1] + PhoneSet.distance(a[i - 1], b[j - 1]), d[i - 1][j] + gap, d[i][j - 1] + gap)
            } }
        }
        var steps: [Step] = []
        var i = n, j = m
        while i > 0 || j > 0 {
            if i > 0 && j > 0 && abs(d[i][j] - (d[i - 1][j - 1] + PhoneSet.distance(a[i - 1], b[j - 1]))) < 1e-9 {
                steps.append(PhoneSet.distance(a[i - 1], b[j - 1]) <= 0.1 ? .match(i - 1, j - 1) : .substitute(i - 1, j - 1))
                i -= 1; j -= 1
            } else if i > 0 && abs(d[i][j] - (d[i - 1][j] + gap)) < 1e-9 {
                steps.append(.delete(i - 1)); i -= 1
            } else {
                steps.append(.insert(j - 1)); j -= 1
            }
        }
        return steps.reversed()
    }
}
