import Foundation
import LearningCore

/// Reading handwriting as something to keep up: once the cursive unit has begun, a known word
/// that has never been read in another hand comes back as a read_handwriting item (lowest
/// priority, so it takes the last of the day's review places), picked among look-alikes at
/// first and printed back once it has been read twice. After the first answer FSRS owns it like
/// any other modality: a weak read-cursive memory comes due on its own.
extension Book {
    /// The cursive unit's first page is finished.
    var cursiveUnlocked: Bool {
        pages.contains { $0.sourceID == "\(Self.cursiveUnitID)-p1" && $0.isFinished }
    }

    static let cursiveUnitID = "u31"

    /// At most this many never-read words are offered per weave, after everything that is due.
    static let cursiveSeedsPerWeave = 2

    /// Known single words with no read-cursive memory yet, the most confusable hands first
    /// (letters that blur in cursive: и ш л м т п н, а о, д б), stable in order.
    func cursiveSeeds(excluding due: Set<String>) -> [DueReview] {
        guard cursiveUnlocked else { return [] }
        let tricky = CursiveTraps.minimLetters
        let candidates = reviewCandidates().filter { c in
            c.id.hasPrefix("word:") && !c.id.contains(" ") && !due.contains(c.id) && c.modalities.contains(.readCursive)
                && c.components.allSatisfy { knowledge.model.trace($0, .readCursive) == nil }
        }
        return candidates
            .sorted { a, b in
                let ta = a.id.filter(tricky.contains).count, tb = b.id.filter(tricky.contains).count
                return ta != tb ? ta > tb : a.id < b.id
            }
            .prefix(Self.cursiveSeedsPerWeave)
            .map { DueReview(id: $0.id, modality: .readCursive, priority: 0.1, relearning: false) }
    }

    /// Times the word has been read in another hand so far.
    func cursiveReadings(of key: String) -> Int {
        knowledge.domain.reviewComponents(forTarget: key).compactMap { knowledge.model.trace($0, .readCursive)?.reviews.count }.max() ?? 0
    }

    /// The read_handwriting review for a word: pick among look-alikes until it has been read
    /// twice, then print it back. The hand is fixed by `variant` so a page re-opens unchanged.
    func cursiveReviewExercise(for key: String, word: String, variant: Int) -> Exercise {
        var ex = Exercise(kind: .readHandwriting)
        ex.text = word
        ex.seed = Int(truncatingIfNeeded: HandwritingSample.seed(for: word, index: variant) & 0x7FFF_FFFF)
        if cursiveReadings(of: key) < 2 {
            var rng = SeededGenerator(seed: UInt64(ex.seed ?? 0))
            let traps = CursiveTraps.distractors(for: word, pool: handwritingWords(), count: 2, rng: &rng)
            if traps.count == 2 { ex.options = ([word] + traps).shuffled(using: &rng) }
        }
        ex.reviewKey = key
        return ex
    }
}
