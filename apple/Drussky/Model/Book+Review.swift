import Foundation
import LearningCore

/// Review and fading, read from LearningCore's FSRS traces (per component × modality). Book
/// only says what there is to recall (`targets`) and turns a due item into an exercise.
extension Book {
    /// Retention target from Settings (Day): the one knob on when reviews come due.
    var reviewPolicy: ReviewPolicy { ReviewPolicy(retention: settings.desiredRetention ?? 0.9) }
    /// Modalities in which recall (not copying) is practised; fading looks at these.
    static let recallModalities: Set<Modality> = [.hearWrite, .meaningWrite]

    /// The components whose traces schedule a review candidate (`letter:*`, `word:*`,
    /// `gram:*`, `root:*`, or `construction-choice`).
    func reviewComponents(for key: String) -> [ComponentID] {
        let (kind, value) = split(key)
        if kind == "gram", !value.isEmpty { return [ComponentID(.construction, value)] }
        if kind == "root", !value.isEmpty { return [ComponentID(.root, value)] }
        return knowledge.domain.reviewComponents(forTarget: key)
    }

    /// Words and phrases the learner has met: practised in the knowledge model or kept as a target.
    var wordsMetCount: Int {
        var keys = Set(knowledge.practisedWords.map(\.key))
        for k in targets.keys where k.hasPrefix("word:") { keys.insert(String(k.dropFirst(5))) }
        return keys.count
    }

    /// The learner's weakest words from FSRS traces (relearning items first, then lowest recall).
    func weakestWords(limit: Int = 8) -> [Course.Word] {
        guard limit > 0 else { return [] }
        let t = now()
        var keys = Set(knowledge.practisedWords.map(\.key))
        for k in targets.keys where k.hasPrefix("word:") { keys.insert(String(k.dropFirst(5))) }
        let scored: [(word: Course.Word, relearning: Bool, strength: Double, key: String)] = keys.compactMap { key -> (word: Course.Word, relearning: Bool, strength: Double, key: String)? in
            guard !(RussianText.isFunctionWord(key) && !key.contains(" ")), let w = word(key) else { return nil }
            let comps = knowledge.domain.reviewComponents(forTarget: "word:\(key)")
            guard !comps.isEmpty else { return nil }
            let strength = comps.compactMap { knowledge.model.strength($0, at: t) }.min() ?? 1.0
            let relearning = comps.contains { c in
                knowledge.model.traces(of: c).values.contains(where: \.isRelearning)
            }
            return (w, relearning, strength, key)
        }
        return scored.sorted { a, b in
            if a.relearning != b.relearning { return a.relearning && !b.relearning }
            if a.strength != b.strength { return a.strength < b.strength }
            return a.key < b.key
        }.prefix(limit).map(\.word)
    }

    /// Each target as LearningCore sees it: the components its review exercises and the
    /// modalities it can be reviewed in, including practised grammar cells (`gram:*`) and
    /// word-family roots (`root:*`).
    func reviewCandidates() -> [ReviewCandidate] {
        let domain = knowledge.domain
        let base: [ReviewCandidate]
        let cacheValid: Bool = {
            guard let cached = candidateCache else { return false }
            if cursiveUnlocked,
               !cached.contains(where: { $0.modalities.contains(.readCursive) }),
               cached.contains(where: { $0.id.hasPrefix("word:") && !$0.id.contains(" ") && !RussianText.isFunctionWord(String($0.id.dropFirst(5))) }) {
                return false
            }
            return true
        }()
        if cacheValid, let cached = candidateCache {
            base = cached
        } else {
            let built = targets.values.sorted { $0.key < $1.key }.compactMap { t -> ReviewCandidate? in
                let components = domain.reviewComponents(forTarget: t.key)
                let modalities = reviewModalities(for: t.key)
                guard !components.isEmpty, !modalities.isEmpty else { return nil }
                return ReviewCandidate(id: t.key, components: components, modalities: modalities, introduced: t.introduced,
                                       firstRecallDelay: t.kept == true ? 86_400 : nil)
            }
            candidateCache = built
            base = built
        }
        var out = base
        var seen = Set(base.map(\.id))
        let p = domain.paradigms
        for c in p.data.components {
            let comp = ComponentID(.construction, c.id)
            let tTransform = knowledge.model.trace(comp, .transform)
            let tClassify = knowledge.model.trace(comp, .classify)
            guard tTransform != nil || tClassify != nil else { continue }
            let key = "gram:\(c.id)"
            guard seen.insert(key).inserted else { continue }
            let modalities = reviewModalities(for: key)
            guard !modalities.isEmpty else { continue }
            let intro = [tTransform?.reviews.first?.t, tClassify?.reviews.first?.t].compactMap { $0 }.min() ?? now()
            out.append(ReviewCandidate(id: key, components: [comp], modalities: modalities, introduced: intro))
        }
        let m = domain.morphology
        for r in m.roots {
            let comp = ComponentID(.root, r.id)
            guard let tBuild = knowledge.model.trace(comp, .build) else { continue }
            let key = "root:\(r.id)"
            guard seen.insert(key).inserted else { continue }
            let modalities = reviewModalities(for: key)
            guard !modalities.isEmpty else { continue }
            let intro = tBuild.reviews.first?.t ?? now()
            out.append(ReviewCandidate(id: key, components: [comp], modalities: modalities, introduced: intro))
        }
        return out
    }

    /// What is due now, most urgent first. Items already waiting, unanswered, on the last two
    /// pages aren't offered again; skipping them leaves them due (only an answer moves them).
    func dueReviews() -> [DueReview] {
        let waiting = Set(pages.suffix(2).flatMap { page in
            page.exercises.indices.compactMap { i in page.results[i].status.isComplete ? nil : page.exercises[i].reviewKey }
        })
        let due = knowledge.model.dueReviews(reviewCandidates(), at: now(), policy: reviewPolicy)
            // Construction choices come due only while an unseen sentence is left to decide.
            .filter { $0.id != ConstructionChoice.key || freshConstructionChoices.contains { !ConstructionChoice.reservedForProbe($0) } }
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
    static func ambiguousGlosses(in lexicon: [Course.Word]) -> Set<String> {
        var seen: [String: Int] = [:]
        for w in lexicon { seen[w.en.lowercased(), default: 0] += 1 }
        return Set(seen.filter { $0.value > 1 }.keys)
    }

    private func askable(_ word: Course.Word) -> Bool { !word.en.isEmpty && !ambiguousGlosses.contains(word.en.lowercased()) }

    /// The modalities a target's review exercises can actually test.
    func reviewModalities(for key: String) -> Set<Modality> {
        if key == ConstructionChoice.key { return [.constructionChoice] }
        let (kind, value) = split(key)
        if kind == "letter" {
            guard let letter = course.letter(value) else { return [] }
            return sayable(letter) ? [.hearWrite, .hearPick] : [.hearPick]
        }
        if kind == "gram" {
            guard let first = grammarReviewTargets(for: value).first else { return [] }
            var m: Set<Modality> = [.transform]
            if Set(first.word.forms.values).count >= 2 { m.insert(.classify) }
            return m
        }
        if kind == "root" {
            return rootReviewEntries(for: value).isEmpty ? [] : [.build]
        }
        guard kind == "word", !value.isEmpty else { return [] }
        // A function word (Model/RussianText.swift) is met inside phrases; alone it is only
        // recognised by its meaning, never written, said or picked out by ear.
        if RussianText.isFunctionWord(value) { return word(value)?.en.isEmpty == false ? [.readPick] : [] }
        guard let word = word(value) else { return [.hearWrite] }
        var m: Set<Modality> = [.hearWrite, .say, .hearPick]
        if askable(word) { m.insert(.meaningWrite) }
        if !word.en.isEmpty { m.insert(.readPick) }
        if cursiveUnlocked, !value.contains(" ") { m.insert(.readCursive) }  // Cursive/CursiveReviews.swift
        return m
    }

    private func grammarReviewTargets(for componentID: String) -> [(word: Paradigms.Word, table: Paradigms.Table, cell: String)] {
        let p = knowledge.domain.paradigms
        guard !p.isEmpty, !componentID.isEmpty else { return [] }
        let comp = ComponentID(.construction, componentID)
        func matching(in words: [Paradigms.Word]) -> [(word: Paradigms.Word, table: Paradigms.Table, cell: String)] {
            var hits: [(word: Paradigms.Word, table: Paradigms.Table, cell: String)] = []
            for w in words {
                for cell in w.forms.keys.sorted() where !w.answers(cell).isEmpty {
                    guard p.components(word: w, cell: cell).first == comp,
                          let table = p.table(word: w, cell: cell) else { continue }
                    hits.append((w, table, cell))
                }
            }
            return hits
        }
        let strict = matching(in: knownParadigmWords(from: p))
        if !strict.isEmpty { return strict }
        let known = knownLemmas
        let byLemma = matching(in: p.words.values.filter { known.contains($0.key) }.sorted { $0.key < $1.key })
        if !byLemma.isEmpty { return byLemma }
        return matching(in: p.words.values.sorted { $0.key < $1.key })
    }

    private func rootReviewEntries(for rootID: String) -> [Morphology.Entry] {
        let m = knowledge.domain.morphology
        guard !rootID.isEmpty else { return [] }
        let all = m.members(of: rootID).compactMap { m.entry(lemma: $0) }
            .filter { m.tiles(for: $0) != nil && !$0.en.isEmpty }
        guard !all.isEmpty else { return [] }
        let known = knownLemmas
        let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
        let knownMet = all.filter { known.contains($0.key) && (met.isEmpty || RussianText.letters(in: $0.ru).isSubset(of: met)) }
        if !knownMet.isEmpty { return knownMet }
        let knownAny = all.filter { known.contains($0.key) }
        if !knownAny.isEmpty { return knownAny }
        let metOnly = all.filter { met.isEmpty || RussianText.letters(in: $0.ru).isSubset(of: met) }
        return metOnly.isEmpty ? all : metOnly
    }

    /// A review exercise for a target in a modality (nil: a first recall, varied by `variant`).
    func reviewExercise(for key: String, modality: Modality?, variant: Int, forProbe: Bool = false) -> Exercise {
        if key == ConstructionChoice.key {
            let fresh = freshConstructionChoices.filter { forProbe || !ConstructionChoice.reservedForProbe($0) }
            guard !fresh.isEmpty else {
                var note = Exercise(kind: .explain)
                note.text = "These sentence choices are all in your notebook. Return to their examples."
                return note
            }
            var choice = fresh[abs(variant % fresh.count)]
            // Rotate the answers too: their position cannot identify the case.
            let options = choice.options ?? []
            let offset = abs(variant % max(1, options.count))
            choice.options = Array(options.dropFirst(offset) + options.prefix(offset))
            choice.answerIndex = ((choice.answerIndex ?? 0) - offset + options.count) % options.count
            choice.reviewKey = key
            return choice
        }
        let (kind, value) = split(key)
        var ex: Exercise
        if kind == "gram", let pick = grammarReviewTargets(for: value).enumerated().first(where: { $0.offset == abs(variant) % max(1, grammarReviewTargets(for: value).count) })?.element {
            let p = knowledge.domain.paradigms
            let met = Set(course.letters.filter { letterKnown($0.upper) }.map(\.upper))
            var rng = SeededGenerator(seed: UInt64(bitPattern: Int64(variant &+ 1)))
            if modality == .classify {
                let answer = pick.word.forms[pick.cell] ?? pick.word.answers(pick.cell).first ?? ""
                var opts = [answer]
                for f in pick.word.forms.sorted(by: { $0.key < $1.key }).map(\.value) where !opts.contains(f) && opts.count < 4 {
                    opts.append(f)
                }
                var pf = Exercise(kind: .paradigmFill)
                pf.word = pick.word.ru
                pf.paradigm = pick.table.id
                pf.forms = pick.word.forms
                pf.cells = [pick.cell]
                pf.options = opts.shuffled(using: &rng)
                ex = pf
            } else {
                var tr = Exercise(kind: .transform)
                tr.word = pick.word.ru
                tr.paradigm = pick.table.id
                tr.cue = pick.cell
                tr.answerText = pick.word.answers(pick.cell).first ?? pick.word.forms[pick.cell] ?? ""
                if let gap = p.uses.flatMap(\.gaps).filter({ $0.word == pick.word.key && $0.cell == pick.cell && (met.isEmpty || RussianText.letters(in: $0.before + $0.after).isSubset(of: met)) }).shuffled(using: &rng).first {
                    tr.before = gap.before
                    tr.after = gap.after
                    tr.hint = gap.hint
                }
                ex = tr
            }
        } else if kind == "root", !rootReviewEntries(for: value).isEmpty {
            let entries = rootReviewEntries(for: value)
            let entry = entries[abs(variant) % entries.count]
            let m = knowledge.domain.morphology
            var rng = SeededGenerator(seed: UInt64(bitPattern: Int64(variant &+ 1)))
            if let built = buildWord(for: entry, from: m, rng: &rng) {
                ex = built
            } else {
                let parts = m.tiles(for: entry) ?? [entry.ru]
                var bw = Exercise(kind: .buildWord)
                bw.en = entry.en
                bw.answerText = entry.ru
                bw.parts = parts
                bw.options = parts.shuffled(using: &rng)
                ex = bw
            }
        } else if kind == "letter", let letter = course.letter(value) {
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
            let first: Modality = RussianText.isFunctionWord(value) ? .readPick : rotation[variant % 3]
            switch modality ?? first {
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
        var metSet = Set(targets.keys.filter { $0.hasPrefix("word:") }.map { String($0.dropFirst(5)) })
        for id in knowledge.practisedWords { metSet.insert(id.key) }
        let met = metSet.sorted()
        var pool = met.compactMap { self.word($0) }.filter { RussianText.key($0.ru) != own }
        if pool.count < count { pool += course.keyedLexicon.filter { $0.key != own }.map(\.word) }  // keys made once, not per call
        // The count × 3 closest in length (earlier entries first among equals), then a few at random.
        let length = word.ru.count
        var closest: [(word: Course.Word, distance: Int)] = []
        for w in pool {
            let d = abs(w.ru.count - length)
            guard closest.count < count * 3 || d < closest[closest.count - 1].distance else { continue }
            let at = closest.firstIndex { $0.distance > d } ?? closest.count
            closest.insert((w, d), at: at)
            if closest.count > count * 3 { closest.removeLast() }
        }
        var out: [String] = []
        for w in closest.map(\.word).shuffled() where !out.contains(w.ru) && out.count < count { out.append(w.ru) }
        return out
    }
}
