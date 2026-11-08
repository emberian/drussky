import Foundation

/// One syllable as spoken: where it is in its recording and how it was said.
struct SpokenSyllable: Sendable, Codable, Hashable {
    var start: Double
    var end: Double
    /// Seconds of the vowel (voiced, within 6 dB of the syllable's loudest vowel-band frame).
    var length: Double
    /// Loudest vowel-band level, dB.
    var loudness: Float
    /// Median pitch in semitones relative to the speaker's own median; nil when unvoiced.
    var pitch: Float?
    /// Pitch across the syllable, eight points (semitones re the speaker's median).
    var trace: [Float?]
    /// 0…1 within the utterance: length, loudness and pitch together (1 = most prominent).
    var prominence: Double
}

/// The model and the learner, syllable by syllable: the model's syllables are found from its
/// loudness peaks (one per written vowel), then carried onto the take through a time warp of
/// the two recordings, so syllables line up however fast or slowly either was said.
struct ProsodyComparison: Sendable {
    var syllables: [TextSyllable]
    var model: [SpokenSyllable]?
    var take: [SpokenSyllable]
    var takeSpeech: ClosedRange<Double>?
    var modelSpeech: ClosedRange<Double>?
    /// How well the two recordings matched in the warp (lower is closer); nil without a model.
    var alignmentCost: Float?

    /// Words whose stress the take puts elsewhere: (word, expected syllable, the take's).
    ///
    /// With a model, the take is judged against it rather than in the abstract: syllable j has
    /// taken the stress from the written one e when, compared with the model, j has grown longer
    /// and louder relative to e (lengths as log ratios over ~1.35×, loudness over ~6 dB) and is
    /// now the more prominent of the two. A voice's own habits (a long final syllable, a falling
    /// pitch on the stress) are in the model too, so they cancel. Without a model, only a clear
    /// lead of another syllable counts.
    var stressMisses: [(word: Int, expected: Int, heard: Int)] {
        var out: [(Int, Int, Int)] = []
        for w in Set(syllables.map(\.word)).sorted() {
            let idx = syllables.indices.filter { syllables[$0].word == w }
            guard idx.count >= 2, let e = idx.first(where: { syllables[$0].isStressed }) else { continue }
            if let model {
                var best: (Int, Double)?
                for j in idx where j != e && take[j].prominence > take[e].prominence {
                    let len = (log(take[j].length) - log(take[e].length)) - (log(model[j].length) - log(model[e].length))
                    let loud = Double((take[j].loudness - take[e].loudness) - (model[j].loudness - model[e].loudness))
                    let evidence = len / 0.3 + loud / 6
                    if evidence > 1.5, evidence > (best?.1 ?? 0) { best = (j, evidence) }
                }
                if let (j, _) = best { out.append((w, e, j)) }
            } else if let j = idx.max(by: { take[$0].prominence < take[$1].prominence }), j != e,
                      take[j].prominence - take[e].prominence > 0.35 {
                out.append((w, e, j))
            }
        }
        return out
    }

    /// The syllable the take stresses most: where a stress went astray, that one; with a model
    /// and no stray stress, the written stress of the main word; otherwise the most prominent.
    var takeStress: Int? {
        if let miss = stressMisses.first { return miss.heard }
        if model != nil {
            let words = Dictionary(grouping: syllables.indices, by: { syllables[$0].word })
            if let main = words.values.filter({ $0.count >= 2 }).max(by: { $0.count < $1.count }),
               let e = main.first(where: { syllables[$0].isStressed }) { return e }
        }
        return take.indices.max { take[$0].prominence < take[$1].prominence }
    }

    /// A clear climb in the model (≥ 4 semitones above the syllables before it) that the take
    /// doesn't make (under 1.5): the syllable, if any.
    var missedRise: Int? {
        guard let model, model.count >= 2 else { return nil }
        var best: (Int, Float)?
        for k in 1..<model.count {
            guard let p = model[k].pitch else { continue }
            let before = model[..<k].compactMap(\.pitch)
            guard !before.isEmpty else { continue }
            let rise = p - before.reduce(0, +) / Float(before.count)
            if rise >= 4, rise > (best?.1 ?? 0) { best = (k, rise) }
        }
        guard let (k, _) = best else { return nil }
        let before = take[..<k].compactMap(\.pitch)
        guard let p = take[k].pitch, !before.isEmpty else { return nil }
        return p - before.reduce(0, +) / Float(before.count) < 1.5 ? k : nil
    }

    // MARK: Building

    static func compute(target: String, take x: [Float], model y: [Float]?, takeSpans: [ClosedRange<Double>]? = nil) -> ProsodyComparison? {
        let syllables = Syllables.split(target)
        guard !syllables.isEmpty else { return nil }
        let takeA = Acoustics(samples: x)
        guard let takeBounds = takeA.speechBounds() else { return nil }
        let takePitch = Pitch.clean(Pitch.analyse(x, sampleRate: Acoustics.sampleRate, hop: Acoustics.hop))
        let n = syllables.count

        var modelSpoken: [SpokenSyllable]?
        var modelSpeech: ClosedRange<Double>?
        var cost: Float?
        var spans: [Range<Int>]?
        if let takeSpans, takeSpans.count == n {
            spans = takeSpans.map { Acoustics.frame($0.lowerBound)..<max(Acoustics.frame($0.lowerBound) + 1, Acoustics.frame($0.upperBound)) }
        }
        if let y {
            let modelA = Acoustics(samples: y)
            if let mb = modelA.speechBounds() {
                let modelPitch = Pitch.clean(Pitch.analyse(y, sampleRate: Acoustics.sampleRate, hop: Acoustics.hop))
                let mSpans = segment(modelA, pitch: modelPitch, within: mb, count: n)
                modelSpoken = measure(modelA, pitch: modelPitch, spans: mSpans)
                modelSpeech = Acoustics.time(mb.lowerBound)...Acoustics.time(mb.upperBound)
                if spans == nil, let al = DTW.align(Array(modelA.mfcc[mb]), Array(takeA.mfcc[takeBounds])) {
                    cost = al.cost
                    let prior = mSpans.dropFirst().map { Int(al.map($0.lowerBound - mb.lowerBound).rounded()) }
                    spans = segment(takeA, pitch: takePitch, within: takeBounds, count: n, prior: prior)
                }
            }
        }
        let takeSpansFrames = spans ?? segment(takeA, pitch: takePitch, within: takeBounds, count: n)
        let takeSpoken = measure(takeA, pitch: takePitch, spans: takeSpansFrames)
        return ProsodyComparison(syllables: syllables, model: modelSpoken, take: takeSpoken,
                                 takeSpeech: Acoustics.time(takeBounds.lowerBound)...Acoustics.time(takeBounds.upperBound),
                                 modelSpeech: modelSpeech, alignmentCost: cost)
    }

    /// Splits `bounds` into `count` syllables, choosing the cuts together (dynamic programming)
    /// so that every syllable keeps some voice and lasts at least 40 ms. A cut scores for how
    /// deep a dip of vowel-band loudness it sits in (below the lower of the highest peaks on
    /// either side: a stop's closure, a hiss, a nasal) and for how much the spectrum changes
    /// across it (a smooth sonorant like л between vowels barely dips but changes colour).
    /// With `prior` (cuts carried from the model through the time warp, frames into `bounds`),
    /// each cut is also drawn toward its carried place (−1 per 50 ms away): the warp says
    /// roughly where, the take's own sound says exactly where.
    /// Each cut then moves back to where its dip begins, so a consonant starts the syllable
    /// after it (мо|ло|ко, спа|си|бо), as the written syllables do.
    static func segment(_ a: Acoustics, pitch: [Float?], within bounds: Range<Int>, count: Int, prior: [Int]? = nil) -> [Range<Int>] {
        let minLength = 4
        guard count > 1, bounds.count >= count * minLength else { return even(bounds, count) }
        let s = smooth(Array(a.vowelBand[bounds]), radius: 2)
        let len = s.count
        var prefixMax = s, suffixMax = s
        for t in 1..<len { prefixMax[t] = max(prefixMax[t - 1], s[t]) }
        for t in stride(from: len - 2, through: 0, by: -1) { suffixMax[t] = max(suffixMax[t + 1], s[t]) }
        let level = (0..<len).map { min(prefixMax[$0], suffixMax[$0]) }
        let mfcc = Array(a.mfcc[bounds])
        let change: [Double] = (0..<len).map { t in
            let w = 3
            guard t >= w, t + w <= len else { return 0 }
            var d: Double = 0
            for c in 0..<Acoustics.coefficients {
                var l: Float = 0, r: Float = 0
                for k in 0..<w { l += mfcc[t - 1 - k][c]; r += mfcc[t + k][c] }
                d += Double((l - r) / Float(w)) * Double((l - r) / Float(w))
            }
            return d.squareRoot()
        }
        let typical = max(1e-6, change.sorted()[min(len - 1, len * 9 / 10)])
        let score: [Double] = (0..<len).map { Double(level[$0] - s[$0]) / 6 + change[$0] / typical }
        // A syllable needs a vowel: voiced frames within 15 dB of the utterance's loudest.
        let peak = s.max() ?? 0
        var voicedPrefix = [0]
        for t in 0..<len {
            let i = bounds.lowerBound + t
            let strong = pitch.indices.contains(i) && pitch[i] != nil && s[t] > peak - 15
            voicedPrefix.append(voicedPrefix[t] + (strong ? 1 : 0))
        }
        let needVoice = voicedPrefix[len] >= count * 4
        func valid(_ from: Int, _ to: Int) -> Bool {
            to - from >= minLength && (!needVoice || voicedPrefix[to] - voicedPrefix[from] >= 3)
        }
        // best[k][t]: most depth with k cuts, the last at t (cut 0 is the start).
        let none = -Double.infinity
        var best = [[Double]](repeating: [Double](repeating: none, count: len + 1), count: count)
        var back = [[Int]](repeating: [Int](repeating: -1, count: len + 1), count: count)
        best[0][0] = 0
        for k in 1..<count {
            for t in 1..<len {
                var top = none, from = -1
                for u in 0..<t where best[k - 1][u] > none && valid(u, t) {
                    if best[k - 1][u] > top { top = best[k - 1][u]; from = u }
                }
                let pull = prior.map { k - 1 < $0.count ? -Double(abs(t - $0[k - 1])) / 5 : 0 } ?? 0
                if from >= 0 { best[k][t] = top + score[t] + pull; back[k][t] = from }
            }
        }
        var last = -1, top = none
        for t in 1..<len where best[count - 1][t] > none && valid(t, len) {
            if best[count - 1][t] > top { top = best[count - 1][t]; last = t }
        }
        guard last >= 0 else { return even(bounds, count) }
        var cuts = [last]
        var k = count - 1
        while k > 1 { cuts.append(back[k][cuts.last!]); k -= 1 }
        // Back to the start of each dip, never past the previous cut's syllable minimum.
        var edges = [0]
        for c in cuts.reversed() {
            var t = c
            while t - 1 > edges.last! + minLength && s[t - 1] < level[t - 1] - 6 && s[t - 1] <= s[t] + 3 { t -= 1 }
            edges.append(t)
        }
        edges.append(len)
        return (0..<count).map { (bounds.lowerBound + edges[$0])..<(bounds.lowerBound + edges[$0 + 1]) }
    }

    static func even(_ bounds: Range<Int>, _ count: Int) -> [Range<Int>] {
        let step = Double(bounds.count) / Double(max(1, count))
        return (0..<count).map { k in
            let a = bounds.lowerBound + Int(Double(k) * step), b = bounds.lowerBound + Int(Double(k + 1) * step)
            return a..<max(a + 1, b)
        }
    }

    static func smooth(_ v: [Float], radius: Int) -> [Float] {
        v.indices.map { i in
            let lo = max(0, i - radius), hi = min(v.count - 1, i + radius)
            return v[lo...hi].reduce(0, +) / Float(hi - lo + 1)
        }
    }

    static func measure(_ a: Acoustics, pitch: [Float?], spans: [Range<Int>]) -> [SpokenSyllable] {
        let voiced = pitch.compactMap { $0 }.sorted()
        let median = voiced.isEmpty ? nil : voiced[voiced.count / 2]
        func semitones(_ i: Int) -> Float? {
            guard let median, i >= 0, i < pitch.count, let f = pitch[i] else { return nil }
            return 12 * log2(f / median)
        }
        var out: [SpokenSyllable] = spans.map { span in
            let r = span.clamped(to: 0..<a.count)
            let loud = r.isEmpty ? -120 : a.vowelBand[r].max() ?? -120
            let vowel = r.filter { a.vowelBand[$0] > loud - 6 && (pitch.indices.contains($0) ? pitch[$0] != nil : false) }.count
            let st = r.compactMap(semitones).sorted()
            let trace: [Float?] = (0..<8).map { k in
                guard !r.isEmpty else { return nil }
                let i = r.lowerBound + Int(Double(k) / 8 * Double(r.count))
                return semitones(i)
            }
            return SpokenSyllable(start: Acoustics.time(span.lowerBound), end: Acoustics.time(span.upperBound),
                                  length: Double(max(vowel, 1)) / Acoustics.rate, loudness: loud,
                                  pitch: st.isEmpty ? nil : st[st.count / 2], trace: trace, prominence: 1)
        }
        guard out.count > 1 else { return out }
        func z(_ v: [Double]) -> [Double] {
            let mean = v.reduce(0, +) / Double(v.count)
            let sd = (v.map { ($0 - mean) * ($0 - mean) }.reduce(0, +) / Double(v.count)).squareRoot()
            return v.map { sd > 1e-6 ? ($0 - mean) / sd : 0 }
        }
        let zl = z(out.map { log($0.length) }), zd = z(out.map { Double($0.loudness) })
        let pitches = out.map { Double($0.pitch ?? -99) }
        let known = pitches.filter { $0 > -98 }
        let pmean = known.isEmpty ? 0 : known.reduce(0, +) / Double(known.count)
        let zp = z(pitches.map { $0 > -98 ? $0 : pmean })
        let score = out.indices.map { 0.55 * zl[$0] + 0.35 * zd[$0] + 0.1 * abs(zp[$0]) }
        let lo = score.min() ?? 0, hi = score.max() ?? 1
        for i in out.indices { out[i].prominence = hi - lo > 1e-6 ? (score[i] - lo) / (hi - lo) : 1 }
        return out
    }
}
