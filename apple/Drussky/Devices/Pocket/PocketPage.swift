import Foundation
import LearningCore

/// A page for the phone in one hand: listening and speaking only, nothing to write. Built on
/// the device from what is due and what the learner knows, sized for five to ten minutes.
///
///   due items     listen-pick or say it (a dictation or translation due is *said* instead of written)
///   ear training  real voices saying sentences of known words: pick which, or say it after them
///   sound pairs   minimal pairs on the contrasts that go worst
///   a passage     one text already read, to listen to again
enum PocketPage {
    static let kinds: Set<ExerciseKind> = [.listenPick, .readAloud, .soundPair, .passage]
    static let unit = "Карман · Pocket"
    static let intro = "Listening and speaking only — nothing to write. Tap the speaker, pick what you heard, or say it; every take is kept."

    /// Roughly how long one exercise of a kind takes, to size the page.
    static func seconds(_ ex: Exercise) -> Int {
        switch ex.kind {
        case .listenPick: 15
        case .readAloud: 35
        case .soundPair: ex.isSoundPairTrial ? 12 : 30
        case .passage: 90
        default: 20
        }
    }

    /// The exercises for a pocket page, or fewer than four when there isn't enough to listen to.
    @MainActor
    static func exercises(book: Book, sentences: [Library.Sentence], hasAudio: (String) -> Bool,
                          voices: (String, Bool) -> Int, minutes: Int = 7) -> [Exercise] {
        let budget = max(5, min(10, minutes)) * 60
        var total = 0
        var out: [Exercise] = []
        func take(_ ex: Exercise) { guard kinds.contains(ex.kind) else { return }; out.append(ex); total += seconds(ex) }

        // Due items, most urgent first, in a hearing or speaking form; at most ~60% of the time.
        let due = Array(book.dueReviews().prefix(book.reviewAllowance()))
        for (i, item) in due.enumerated() where total < budget * 6 / 10 {
            if let ex = spoken(book: book, review: item, variant: i, hasAudio: hasAudio) { take(ex) }
        }
        // Ear training: a listening pick, then one to say after the speaker.
        let ear = book.earTraining(from: sentences, hasAudio: hasAudio).map { ex -> Exercise in
            guard ex.kind == .dictation else { return ex }
            var say = Exercise(kind: .readAloud); say.text = ex.text; return say
        }
        for ex in ear.prefix(4) where total < budget * 8 / 10 { take(ex) }
        // Sound pairs on the weakest contrasts: the card that names the contrast, then its trials.
        let pairs = book.soundPairsExercises(from: SoundPairs.load(), trials: 4, voices: voices)
        for ex in pairs.prefix(5) where total < budget { take(ex) }
        // One passage already read, to hear again (it needs the most time, so only if it fits).
        if total + 90 <= budget, let passage = book.pages.reversed().lazy
            .flatMap({ $0.exercises }).first(where: { $0.kind == .passage && ($0.text ?? "").count <= 400 }) {
            var p = passage; p.reviewKey = nil
            take(p)
        }
        // Open with something to hear, not something to say.
        if let first = out.firstIndex(where: { $0.kind == .listenPick }), first > 0 { out.insert(out.remove(at: first), at: 0) }
        return out
    }

    /// A due review as something heard or said: letters by ear; words said aloud when a writing
    /// recall is due, picked by ear otherwise; anything not in the lexicon said aloud if recorded.
    @MainActor
    private static func spoken(book: Book, review: DueReview, variant: Int, hasAudio: (String) -> Bool) -> Exercise? {
        let parts = review.id.split(separator: ":", maxSplits: 1).map(String.init)
        guard parts.count == 2 else { return nil }
        if parts[0] == "letter" { return book.reviewExercise(for: review.id, modality: .hearPick, variant: variant) }
        guard parts[0] == "word" else { return nil }
        let can = book.reviewModalities(for: review.id)
        if book.word(parts[1]) == nil {
            guard hasAudio(parts[1]), !RussianText.isFunctionWord(parts[1]) else { return nil }
            var ex = Exercise(kind: .readAloud); ex.text = parts[1]; ex.reviewKey = review.id
            return ex
        }
        let m: Modality = switch review.modality {
        case .some(.hearPick), .some(.readPick): .hearPick
        case .some(.say): .say
        default: variant % 3 == 2 && can.contains(.hearPick) ? .hearPick : .say
        }
        guard can.contains(m) else { return nil }
        return book.reviewExercise(for: review.id, modality: m, variant: variant)
    }

    /// Inserts a pocket page after the current one, or says why it couldn't.
    @MainActor
    static func insert(book: Book, voice: Voice) -> String? {
        let exercises = exercises(book: book, sentences: Library.load().sentences ?? [], hasAudio: voice.hasRecording,
                                  voices: { voice.pairVoices($0, exactStress: $1).count })
        guard exercises.count >= 4 else {
            return "Not enough to listen to yet — a few more pages and the pocket fills up."
        }
        book.insertWrittenPage(title: "In my pocket", intro: intro, unitTitle: unit, exercises: exercises, newWords: [])
        return nil
    }
}
