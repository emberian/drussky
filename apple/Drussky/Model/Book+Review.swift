import Foundation
import LearningCore

/// Review and fading, read from LearningCore's FSRS traces (per component × modality). Book
/// only says what there is to recall (`targets`) and turns a due item into an exercise.
extension Book {
    /// Retention target from Settings (Day): the one knob on when reviews come due.
    var reviewPolicy: ReviewPolicy { ReviewPolicy(retention: settings.desiredRetention ?? 0.9) }
    /// Modalities in which recall (not copying) is practised; fading looks at these.
    static let recallModalities: Set<Modality> = [.hearWrite, .meaningWrite]

    /// Each target as LearningCore sees it: the components its review exercises and the
    /// modalities it can be reviewed in.
    func reviewCandidates() -> [ReviewCandidate] {
        if let cached = candidateCache { return cached }
        let domain = knowledge.domain
        let out = targets.values.sorted { $0.key < $1.key }.compactMap { t -> ReviewCandidate? in
            let components = domain.reviewComponents(forTarget: t.key)
            let modalities = reviewModalities(for: t.key)
            guard !components.isEmpty, !modalities.isEmpty else { return nil }
            return ReviewCandidate(id: t.key, components: components, modalities: modalities, introduced: t.introduced,
                                   firstRecallDelay: t.kept == true ? 86_400 : nil)
        }
        candidateCache = out
        return out
    }

    /// What is due now, most urgent first. Items already waiting, unanswered, on the last two
    /// pages aren't offered again; skipping them leaves them due (only an answer moves them).
    func dueReviews() -> [DueReview] {
        let waiting = Set(pages.suffix(2).flatMap { page in
            page.exercises.indices.compactMap { i in page.results[i].status.isComplete ? nil : page.exercises[i].reviewKey }
        })
        let due = knowledge.model.dueReviews(reviewCandidates(), at: now(), policy: reviewPolicy)
        // Words never yet read in another hand, after everything due (Cursive/CursiveReviews.swift).
        return (due + cursiveSeeds(excluding: Set(due.map(\.id)))).filter { !waiting.contains($0.id) }
    }

    var dueCount: Int { dueReviews().count }

    /// Expertise reversal: scaffolds that help a novice get in the way once an item is secure.
    /// A word whose recall is strong now (FSRS: present retrievability and stability in a recall
    /// modality, no unrecovered lapse) loses its support: trace → copy, copy → write it from hearing.
    func faded(_ ex: Exercise) -> Exercise {
        // A cursive trace or copy is there to show the hand, not to prop up a word: it never fades.
        guard [.trace, .copy].contains(ex.kind), ex.cursive != true, let key = memoryKey(for: ex), targets[key] != nil else { return ex }
        let components = knowledge.domain.reviewComponents(forTarget: key)
        guard knowledge.model.isSecure(components, in: Self.recallModalities, at: now()) else { return ex }
        var e = ex
        if ex.kind == .trace { e.kind = .copy; e.times = 1 } else { e.kind = .dictation; e.times = nil }
        return e
    }

    private func split(_ key: String) -> (kind: String, value: String) {
        let parts = key.split(separator: ":", maxSplits: 1).map(String.init)
        return (parts.first ?? "", parts.count > 1 ? parts[1] : "")
    }

    private func sayable(_ letter: Course.Letter) -> Bool { RussianText.key(letter.say).hasPrefix(letter.lower) }

    /// English glosses shared by more than one entry can't be asked as "write it in Russian".
    private var ambiguousGlosses: Set<String> {
        var seen: [String: Int] = [:]
        for w in course.lexicon { seen[w.en.lowercased(), default: 0] += 1 }
        return Set(seen.filter { $0.value > 1 }.keys)
    }

    private func askable(_ word: Course.Word) -> Bool { !word.en.isEmpty && !ambiguousGlosses.contains(word.en.lowercased()) }

    /// The modalities a target's review exercises can actually test.
    func reviewModalities(for key: String) -> Set<Modality> {
        let (kind, value) = split(key)
        if kind == "letter" {
            guard let letter = course.letter(value) else { return [] }
            return sayable(letter) ? [.hearWrite, .hearPick] : [.hearPick]
        }
        guard kind == "word", !value.isEmpty else { return [] }
        guard let word = word(value) else { return [.hearWrite] }
        var m: Set<Modality> = [.hearWrite, .say, .hearPick]
        if askable(word) { m.insert(.meaningWrite) }
        if !word.en.isEmpty { m.insert(.readPick) }
        if cursiveUnlocked, !value.contains(" ") { m.insert(.readCursive) }  // Cursive/CursiveReviews.swift
        return m
    }

    /// A review exercise for a target in a modality (nil: a first recall, varied by `variant`).
    func reviewExercise(for key: String, modality: Modality?, variant: Int) -> Exercise {
        let (kind, value) = split(key)
        var ex: Exercise
        if kind == "letter", let letter = course.letter(value) {
            // Dictate the letter only when its spoken form begins with it (а, ра); signs and
            // letters voiced inside a word (ы in "ты") are recalled by choosing what was heard.
            let m = modality ?? (variant % 2 == 0 ? .hearWrite : .hearPick)
            if m == .hearWrite && sayable(letter) {
                ex = Exercise(kind: .dictation)
                ex.text = letter.lower
                ex.say = letter.say
            } else {
                ex = Exercise(kind: .listenPick)
                ex.say = letter.say
                let others = course.letters.filter { $0.upper != letter.upper && letterKnown($0.upper) }
                    .shuffled().prefix(2).map(\.say)
                ex.options = ([letter.say] + others).shuffled()
            }
        } else if let word = word(value) {
            let rotation: [Modality] = askable(word) ? [.meaningWrite, .hearWrite, .say] : [.hearWrite, .hearWrite, .say]
            switch modality ?? rotation[variant % 3] {
            case .meaningWrite where askable(word):
                ex = Exercise(kind: .translate); ex.en = word.en; ex.answerText = word.ru
            case .say:
                ex = Exercise(kind: .readAloud); ex.text = word.ru
            case .hearPick:
                ex = Exercise(kind: .listenPick); ex.say = word.ru
                ex.options = ([word.ru] + distractors(for: word, count: 2)).shuffled()
            case .readCursive:
                ex = cursiveReviewExercise(for: key, word: word.ru, variant: variant)
            case .readPick where !word.en.isEmpty:
                let others = distractors(for: word, count: 2).compactMap { self.word($0)?.en }
                    .filter { $0.lowercased() != word.en.lowercased() }
                let options = ([word.en] + others).shuffled()
                ex = Exercise(kind: .readMeaning); ex.text = word.ru; ex.options = options
                ex.answerIndex = options.firstIndex(of: word.en)
            default:
                ex = Exercise(kind: .dictation); ex.text = word.ru
            }
        } else {
            // A word no longer in the lexicon (or an inflected answer): still recall it by ear.
            ex = Exercise(kind: .dictation)
            ex.text = value
        }
        ex.reviewKey = key
        return ex
    }

    /// Other words the learner has met, closest in length first (for choosing by ear or meaning).
    private func distractors(for word: Course.Word, count: Int) -> [String] {
        let own = RussianText.key(word.ru)
        let met = targets.keys.filter { $0.hasPrefix("word:") }.map { String($0.dropFirst(5)) }
        var pool = met.compactMap { self.word($0) }.filter { RussianText.key($0.ru) != own }
        if pool.count < count { pool += course.lexicon.filter { RussianText.key($0.ru) != own } }
        let sorted = pool.sorted { abs($0.ru.count - word.ru.count) < abs($1.ru.count - word.ru.count) }
        var out: [String] = []
        for w in sorted.prefix(count * 3).shuffled() where !out.contains(w.ru) && out.count < count { out.append(w.ru) }
        return out
    }
}
