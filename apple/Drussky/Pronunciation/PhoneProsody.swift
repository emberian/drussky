import Accelerate
import Foundation

/// Which syllable the learner stressed. Russian stress is mostly length and loudness, with
/// pitch movement on the stressed syllable, and vowel quality: only a stressed о is a full [o],
/// unstressed ones shrink to [ɐ]/[ə]. Each syllable gets a prominence from its vowel's length,
/// loudness and pitch (z-scored within the take), plus a fullness term from the recognizer:
/// how much more its vowel sounds like the full vowel than the reduced one.
enum Prosody {
    struct Finding: Sendable {
        var word: Int
        var expected: Int     // syllable index
        var heard: Int
        var strength: Float
        var confidence: Double

        init(word: Int, expected: Int, heard: Int, strength: Float, confidence: Double? = nil) {
            self.word = word
            self.expected = expected
            self.heard = heard
            self.strength = strength
            self.confidence = confidence ?? Double(1 / (1 + Foundation.exp(-strength)))
        }
    }

    struct Result {
        var syllables: [SyllableJudgement]
        var stressedSyllable: Int?
        var findings: [Finding]
        var scores: [Float]
    }

    /// Vowel quality is the stronger cue on these recordings (README): a two-syllable word's
    /// prosodic z-scores are ±1 each, so prosody alone decides only where quality says nothing.
    static let fullnessWeight: Float = 1.5
    static let prosodyWeight: Float = 0.5
    /// How much the winning syllable must beat the expected one before it is called a moved stress.
    static let stressMargin: Float = 1.5

    static func measure(samples x: [Float], expected: Transcription, spans: [ClosedRange<Int>?], chosen: [[Int]],
                        post: Posteriors, scorer: Scorer, timeShift: Double) -> Result {
        let syl = expected.syllables
        guard !syl.isEmpty else { return Result(syllables: [], stressedSyllable: nil, findings: [], scores: []) }
        let sounds = expected.sounds
        // Nucleus of each syllable and its region, bounded by the neighbouring sounds' spans.
        var regions: [(Double, Double)?] = Array(repeating: nil, count: syl.count)
        var nucleus: [Int?] = Array(repeating: nil, count: syl.count)
        for (i, s) in sounds.enumerated() where s.isVowel {
            guard let k = s.syllable, k < syl.count, let sp = spans[i] else { continue }
            nucleus[k] = i
            let prevEnd = (0..<i).reversed().lazy.compactMap { spans[$0]?.upperBound }.first
            let nextStart = ((i + 1)..<sounds.count).lazy.compactMap { spans[$0]?.lowerBound }.first
            let a = prevEnd.map { (post.time($0) + post.time(sp.lowerBound)) / 2 } ?? post.time(sp.lowerBound) - 0.04
            let b = nextStart.map { (post.time(sp.upperBound) + post.time($0)) / 2 } ?? post.time(sp.upperBound) + 0.06
            regions[k] = (max(0, a), max(a + 0.02, b))
        }
        // Loudness (dB, lightly smoothed) and pitch per 10 ms.
        let hop = 160
        let n = x.count / hop
        var rms = [Float](repeating: 0, count: max(n, 1))
        x.withUnsafeBufferPointer { p in
            for i in 0..<n { vDSP_rmsqv(p.baseAddress! + i * hop, 1, &rms[i], vDSP_Length(hop)) }
        }
        let raw = rms.map { 20 * log10(max($0, 1e-6)) }
        let db = raw.indices.map { i in raw[max(0, i - 2)...min(raw.count - 1, i + 2)].reduce(0, +) / Float(min(raw.count - 1, i + 2) - max(0, i - 2) + 1) }
        let f0 = Pitch.clean(Pitch.analyse(x, sampleRate: 16_000, hop: hop))
        let voiced = f0.compactMap { $0 }.sorted()
        let median = voiced.isEmpty ? 1 : voiced[voiced.count / 2]
        // The recognizer places each vowel roughly; the acoustic nucleus is the loudness peak
        // inside that syllable's territory (halfway to the neighbouring vowels), and its length
        // is how long the voiced signal stays within 6 dB of that peak.
        let centers: [Double?] = regions.map { $0.map { ($0.0 + $0.1) / 2 } }
        var dur = [Float](), loud = [Float](), pitch = [Float]()
        for k in regions.indices {
            guard let c = centers[k], n > 2 else { dur.append(.nan); loud.append(.nan); pitch.append(.nan); continue }
            let prev = (0..<k).reversed().lazy.compactMap { centers[$0] }.first
            let next = ((k + 1)..<regions.count).lazy.compactMap { centers[$0] }.first
            let a = prev.map { ($0 + c) / 2 } ?? max(0, c - 0.15), b = next.map { ($0 + c) / 2 } ?? c + 0.15
            let lo = max(0, min(n - 1, Int(a * 100))), hi = max(lo + 1, min(n, Int(b * 100)))
            let peakIdx = (lo..<hi).max { db[$0] < db[$1] } ?? lo
            let peak = db[peakIdx]
            var l = peakIdx, r = peakIdx
            func on(_ i: Int) -> Bool { db[i] > peak - 6 && (i >= f0.count || f0[i] != nil || db[i] > peak - 3) }
            while l - 1 >= lo && on(l - 1) { l -= 1 }
            while r + 1 < hi && on(r + 1) { r += 1 }
            dur.append(Float(r - l + 1) * 0.01)
            loud.append(peak)
            let p = f0[min(l, f0.count)..<min(r + 1, f0.count)].compactMap { $0 }
            pitch.append(p.isEmpty ? .nan : 12 * log2((p.max() ?? median) / median))
        }
        let z = { (v: [Float]) -> [Float] in
            let ok = v.filter { !$0.isNaN }
            guard ok.count >= 2 else { return v.map { _ in 0 } }
            let m = ok.reduce(0, +) / Float(ok.count)
            let sd = max(1e-3, sqrt(ok.map { ($0 - m) * ($0 - m) }.reduce(0, +) / Float(ok.count)))
            return v.map { $0.isNaN ? 0 : ($0 - m) / sd }
        }
        let zd = z(dur), zl = z(loud), zp = z(pitch)
        var prom = [Float](repeating: 0, count: syl.count)
        for k in syl.indices { prom[k] = prosodyWeight * (zd[k] + zl[k] + 0.5 * zp[k]) }
        // Vowel fullness from the posteriors.
        var full = [Float](repeating: 0, count: syl.count)
        for k in syl.indices {
            guard let i = nucleus[k] else { continue }
            let s = sounds[i]
            let u = s.underlying ?? s.ipa
            let afterSoft = i > 0 && (sounds[i - 1].ipa.hasSuffix("ʲ") || ["j", "tɕ", "ɕː"].contains(sounds[i - 1].ipa))
            var f: Float?
            switch u {
            case "o": f = scorer.fullness(at: i, spans: spans, seq: chosen, full: "o", reduced: "ɐ")
            // [a] against [ɐ] says little (the recognizer writes ɐ as a), so only я counts.
            case "a": f = afterSoft ? scorer.fullness(at: i, spans: spans, seq: chosen, full: "a", reduced: "ɪ") : nil
            case "e", "ɛ": f = scorer.fullness(at: i, spans: spans, seq: chosen, full: "e", reduced: "ɪ").map { $0 * 0.5 }
            default: f = nil
            }
            if let f { full[k] = max(-6, min(6, f)) / 3 }
        }
        let score = syl.indices.map { prom[$0] + fullnessWeight * full[$0] }
        if ProcessInfo.processInfo.environment["PRON_DEBUG"] != nil {
            for k in syl.indices { print("  syl", syl[k].text, "dur", dur[k], "loud", loud[k], "pitch", pitch[k], "full", full[k], "score", score[k], "region", regions[k] as Any) }
        }
        let lo = prom.min() ?? 0, hi = prom.max() ?? 1
        var judged: [SyllableJudgement] = []
        for (k, s) in syl.enumerated() {
            let r = regions[k]
            judged.append(SyllableJudgement(
                text: s.text, isStressed: s.stressed,
                start: r.map { max(0, $0.0 - timeShift) }, end: r.map { max(0, $0.1 - timeShift) },
                prominence: r == nil ? nil : Double(hi > lo ? (prom[k] - lo) / (hi - lo) : 1)))
        }
        // Per word: the syllable that won, against the one that should have.
        var findings: [Finding] = []
        let words = Set(syl.map(\.word)).sorted()
        for w in words {
            let ks = syl.indices.filter { syl[$0].word == w }
            guard ks.count >= 2, syl[ks[0]].stressKnown, let exp = ks.first(where: { syl[$0].stressed }),
                  ks.allSatisfy({ regions[$0] != nil }) else { continue }
            let best = ks.max { score[$0] < score[$1] }!
            if best != exp, score[best] - score[exp] > stressMargin {
                findings.append(Finding(word: w, expected: exp, heard: best, strength: score[best] - score[exp]))
            }
        }
        let measured = syl.indices.filter { regions[$0] != nil }
        let stressed = measured.max { score[$0] < score[$1] }
        return Result(syllables: judged, stressedSyllable: stressed, findings: findings, scores: score)
    }
}
