import Foundation

/// Minimal pairs for high-variability listening (Resources/pairs.json): words that differ only
/// in one sound English speakers merge — ы/и, hard/soft, ш/щ, ж/ш, unstressed о, stress.
struct SoundPairs: Codable, Sendable {
    struct Contrast: Codable, Sendable, Hashable {
        var id: String
        var name: String
        var note: String
    }
    struct Word: Codable, Sendable, Hashable {
        var ru: String
        var en: String
    }
    struct Pair: Codable, Sendable, Hashable {
        var contrast: String
        var words: [Word]
        /// The two words sound alike (луг / лук): taught, never asked.
        var same: Bool? = nil
        /// The members differ only in stress, so their audio must match the stress exactly.
        var exactStress: Bool { Set(words.map { RussianText.key($0.ru) }).count == 1 }
    }
    var version: Int
    var contrasts: [Contrast]
    var pairs: [Pair]

    func contrast(_ id: String) -> Contrast? { contrasts.first { $0.id == id } }

    static func load(bundle: Bundle = .main) -> SoundPairs {
        guard let url = bundle.url(forResource: "pairs", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let pairs = try? JSONDecoder().decode(SoundPairs.self, from: data) else {
            return SoundPairs(version: 1, contrasts: [], pairs: [])
        }
        return pairs
    }
}

/// The learner's first-choice accuracy on one contrast.
struct ContrastStat: Codable, Sendable, Hashable {
    var right = 0
    var wrong = 0
    var last: Date? = nil
    var trials: Int { right + wrong }
    /// Smoothed accuracy (one imagined right and one wrong), so a single trial isn't 0% or 100%.
    var accuracy: Double { Double(right + 1) / Double(trials + 2) }
}

extension RussianText {
    /// Audio key for a pair word: like `key`, but keeps the stress mark, so за́мок and замо́к
    /// never share a recording (Audio/index-pairs.json).
    static func pairKey(_ s: String) -> String {
        let lowered = s.precomposedStringWithCanonicalMapping.lowercased()
        let punctuation = Set(".,?!—–-\"«»:;…()")
        let cleaned = String(lowered.map { punctuation.contains($0) ? " " : $0 })
        return cleaned.split(whereSeparator: \.isWhitespace).joined(separator: " ")
    }
}

/// Sound-pair exercises use `Exercise` fields as follows:
///   hint      contrast id          options  the pair, in a fixed order
///   say       the word played (nil on the labelled teaching card)
///   answerIndex  index of `say` in options
///   glosses   pairKey(word) → meaning
///   text      the contrast's note (teaching card); examples: every pair to compare
///   times     which speaker plays this trial
extension Exercise {
    var isSoundPairTrial: Bool { kind == .soundPair && say != nil }
}

extension Book {
    /// Accuracy per contrast id, from first choices on sound-pair trials.
    var contrastStats: [String: ContrastStat] { soundStats }

    /// Called by `record` when a sound-pair trial is first completed.
    func noteSoundPair(_ exercise: Exercise, status: ExerciseResult.Status) {
        guard exercise.isSoundPairTrial, let id = exercise.hint else { return }
        var s = soundStats[id] ?? ContrastStat()
        if status == .correct { s.right += 1 } else { s.wrong += 1 }
        s.last = now()
        soundStats[id] = s
    }

    /// A page of minimal-pair trials, weighted toward the contrasts the learner gets wrong.
    /// Only pairs written in letters already met, and with recordings of both words.
    /// `voices(form, exactStress)` is how many recordings exist for a word.
    func soundPairsExercises(from data: SoundPairs, trials total: Int = 10,
                             voices: (String, Bool) -> Int) -> [Exercise] {
        let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
        func readable(_ p: SoundPairs.Pair) -> Bool {
            p.words.allSatisfy { RussianText.letters(in: $0.ru).isSubset(of: met) }
        }
        let askable = data.pairs.filter { p in
            p.same != true && p.words.count == 2 && readable(p) && p.words.allSatisfy { voices($0.ru, p.exactStress) > 0 }
        }
        let byContrast = Dictionary(grouping: askable, by: \.contrast)
        guard !byContrast.isEmpty else { return [] }

        // Weakest first: lower smoothed accuracy, then fewer trials; untried contrasts count as 50%.
        let stats = soundStats
        let ranked = byContrast.keys.sorted {
            let a = stats[$0] ?? ContrastStat(), b = stats[$1] ?? ContrastStat()
            return a.accuracy != b.accuracy ? a.accuracy < b.accuracy : a.trials < b.trials
        }
        let chosen = Array(ranked.prefix(3))
        // Trials in proportion to error rate, at least two per contrast.
        let weights = chosen.map { pow(1 - (stats[$0] ?? ContrastStat()).accuracy, 2) + 0.05 }
        let sum = weights.reduce(0, +)
        var counts = weights.map { max(2, Int((Double(total) * $0 / sum).rounded())) }
        while counts.reduce(0, +) > total, let i = counts.indices.max(by: { counts[$0] < counts[$1] }), counts[i] > 2 { counts[i] -= 1 }
        while counts.reduce(0, +) < total { counts[0] += 1 }

        var teaching: [Exercise] = []
        var trials: [Exercise] = []
        for (c, id) in chosen.enumerated() {
            guard let contrast = data.contrast(id), let pairs = byContrast[id] else { continue }
            let stat = stats[id] ?? ContrastStat()
            if stat.trials < 12 {
                var t = Exercise(kind: .soundPair)
                t.hint = id
                t.text = contrast.note
                let shown = pairs.shuffled().prefix(4)
                let alike = data.pairs.filter { $0.contrast == id && $0.same == true && readable($0)
                    && $0.words.allSatisfy { voices($0.ru, false) > 0 } }
                t.examples = (shown + alike.prefix(1)).flatMap { p in
                    p.words.map { Course.Example(ru: $0.ru, en: $0.en) }
                }
                t.options = shown.first?.words.map(\.ru)
                teaching.append(t)
            }
            // Each member equally often, pairs cycled so as many as possible are heard.
            var order = pairs.shuffled()
            while order.count < counts[c] { order += pairs.shuffled() }
            for n in 0..<counts[c] {
                let p = order[n]
                let member = (n + c) % 2 == 0 ? 0 : 1
                var e = Exercise(kind: .soundPair)
                e.hint = id
                e.options = p.words.map(\.ru)
                e.say = p.words[member].ru
                e.answerIndex = member
                e.glosses = Dictionary(uniqueKeysWithValues: p.words.map { (RussianText.pairKey($0.ru), $0.en) })
                e.times = Int.random(in: 0..<max(1, voices(p.words[member].ru, p.exactStress)))
                trials.append(e)
            }
        }
        // Shuffle trials but keep a run of the same contrast from exceeding three.
        trials.shuffle()
        return teaching + trials
    }
}
