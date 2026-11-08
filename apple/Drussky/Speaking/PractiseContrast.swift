import Foundation

extension Book {
    /// Inserts a Sound pairs page after the current one and turns to it: every contrast the
    /// learner finds hard, or only `contrast` (a pairs.json id) when a speaking tip asks for it.
    /// false when too few pairs are written in letters already met.
    @discardableResult
    func insertSoundPairsPage(contrast: String? = nil, voices: (String, Bool) -> Int) -> Bool {
        let data = SoundPairs.load()
        let exercises = soundPairsExercises(from: data, only: contrast.map { [$0] }, voices: voices)
        guard exercises.filter(\.isSoundPairTrial).count >= 4 else { return false }
        let names = Set(exercises.compactMap(\.hint)).compactMap { data.contrast($0)?.name }.sorted()
        insertWrittenPage(title: "Sound pairs", intro: "Two words, one sound apart: \(names.joined(separator: " · ")). Hear one, pick it, then hear both. Each time a different voice.",
                          unitTitle: "Listening", exercises: exercises, newWords: [])
        return true
    }
}
