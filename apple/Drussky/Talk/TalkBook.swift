import Foundation

/// A conversation as it appears in "My Russian".
struct TalkRecord: Identifiable, Hashable {
    var ref: ExerciseRef
    var pageNumber: Int
    var conversation: TalkConversation
    var id: String { ref.item }
}

extension Book {
    /// ＋ → Talk: a page with one conversation, placed right after the current page.
    func insertTalkPage() {
        let ex = Exercise(kind: .talk)
        insertWrittenPage(title: "Разгово́р", intro: "Pick a scene. Read or replay your tutor's line, then tap the microphone when you're ready to answer. You can type instead.",
                          unitTitle: "My Russian", exercises: [ex], newWords: [])
    }

    /// Every conversation ever held, newest first.
    var talkRecords: [TalkRecord] {
        var out: [TalkRecord] = []
        for page in pages {
            for (i, r) in page.results.enumerated() where page.exercises[i].kind == .talk {
                if let t = r.talk, !t.lines.isEmpty {
                    out.append(TalkRecord(ref: ExerciseRef(pageID: page.id, index: i), pageNumber: page.number, conversation: t))
                }
            }
        }
        return out.sorted { ($0.conversation.started ?? .distantPast) > ($1.conversation.started ?? .distantPast) }
    }

    /// Writes the conversation as it stands (every line lands here as it happens, so a page turn
    /// or a crash loses nothing said so far).
    func saveTalk(_ ref: ExerciseRef, _ conversation: TalkConversation) {
        record(ref) { r in
            r.talk = conversation
            r.attempts = max(r.attempts, conversation.learnerTurns)
        }
    }

    /// The red pen landed: the exercise finishes, and every corrected form and new word the
    /// tutor used joins the learner's words.
    func finishTalk(_ ref: ExerciseRef, _ conversation: TalkConversation) {
        guard !isClosed else { return }
        for line in conversation.lines {
            for w in line.newWords ?? [] where !w.en.isEmpty && !RussianText.letters(in: w.ru).isEmpty { keep(w.ru, en: w.en) }
            let correction = line.correction.flatMap { $0.canGrade && $0.isConsistent(with: line.ru) ? $0 : nil }
            for e in correction?.edits ?? [] where !RussianText.key(e.replacement).isEmpty && !RussianText.letters(in: e.replacement).isEmpty {
                keep(e.replacement, en: e.en ?? "")
            }
        }
        let clean = conversation.isMarked && !conversation.learnerLines.isEmpty && conversation.learnerLines.allSatisfy {
            guard let c = $0.correction else { return false }
            return c.canGrade && c.isConsistent(with: $0.ru) && c.edits.isEmpty
        }
        record(ref) { r in
            r.talk = conversation
            r.status = clean ? .correct : .done
        }
    }

    /// Ended without the pen (no network afterwards, or the learner chose so): finished, ungraded.
    func keepTalkUnmarked(_ ref: ExerciseRef) {
        guard result(ref)?.talk != nil else { return }
        record(ref) { $0.status = .done }
    }

    /// Nothing said: left for now.
    func skipTalk(_ ref: ExerciseRef) {
        record(ref) { $0.status = .skipped }
    }

    /// Resolves a page's `sourceID` ("u13-p1", "g1", "u22-p1", "u31-p1") to its containing
    /// unit's `id` in `course.units` via `course.pageSequence` first, then unit ID or prefix.
    func unitID(forSourceID sourceID: String) -> String? {
        if let u = course.pageSequence.first(where: { $0.page.id == sourceID })?.unit.id { return u }
        if course.units.contains(where: { $0.id == sourceID }) { return sourceID }
        let prefix = String(sourceID.split(separator: "-").first ?? "")
        return course.units.contains(where: { $0.id == prefix }) ? prefix : nil
    }

    /// Position in `course.units` of the furthest unit the learner has reached in course order.
    /// Visiting the cursive unit (`u31`) early does not advance the course position past units
    /// not yet reached.
    var currentUnitIndex: Int {
        let indices = pages.compactMap(\.sourceID).compactMap { id -> Int? in
            guard let unitID = unitID(forSourceID: id) else { return nil }
            return course.units.firstIndex { $0.id == unitID }
        }
        guard !indices.isEmpty else { return 0 }
        let cursiveIndex = course.units.firstIndex { $0.id == Self.cursiveUnitID }
        let nonCursive = indices.filter { $0 != cursiveIndex }
        if let maxNonCursive = nonCursive.max() {
            if let c = cursiveIndex, indices.contains(c), maxNonCursive >= c - 1 {
                return max(maxNonCursive, c)
            }
            return maxNonCursive
        }
        return indices.max() ?? 0
    }

    /// The `id` of the furthest course unit reached in `course.units` order.
    var currentUnitID: String {
        course.units.indices.contains(currentUnitIndex) ? course.units[currentUnitIndex].id : (course.units.first?.id ?? "u1")
    }

    /// Whether the learner has reached `unitID` (a unit id like `"u13"` or page id like `"g1"`)
    /// in `course.units` order, or visited that unit/page directly.
    func hasReachedUnit(_ unitID: String) -> Bool {
        let sourceIDs = Set(pages.compactMap(\.sourceID))
        if sourceIDs.contains(unitID) { return true }
        let resolved = self.unitID(forSourceID: unitID) ?? unitID
        if resolved == Self.cursiveUnitID {
            return sourceIDs.contains { self.unitID(forSourceID: $0) == Self.cursiveUnitID }
                || (course.units.firstIndex { $0.id == Self.cursiveUnitID }.map { $0 <= currentUnitIndex } ?? false)
        }
        guard let targetIndex = course.units.firstIndex(where: { $0.id == resolved }) else { return false }
        return targetIndex <= currentUnitIndex
    }

    /// Whether the learner has reached unit `"u\(number)"` in `course.units` order (not by raw integer comparison).
    func hasReachedUnit(_ number: Int) -> Bool {
        hasReachedUnit("u\(number)")
    }
}

/// Course-order scene gating for Talk: scenes unlock when their unit is reached in `course.units`
/// order, rather than by comparing raw `u<N>` integers.
enum TalkScenes {
    struct Entry: Sendable, Hashable {
        var afterUnit: Int
        var scene: TalkScene
        var unitID: String { "u\(afterUnit)" }
    }

    static let starters: [Entry] = [
        Entry(afterUnit: 1, scene: TalkScene(title: "Here and there", ru: "Тут и там",
                                             goal: "Find out what is here and what is over there",
                                             frames: [TalkFrame(ru: "Что э́то?", en: "what is this?"),
                                                      TalkFrame(ru: "Где …?", en: "where is …?")],
                                             secret: "The cat is here; the house is over there")),
        Entry(afterUnit: 13, scene: TalkScene(title: "At the table", ru: "За столо́м",
                                              goal: "Ask what is on the table and ask for water or bread",
                                              frames: [TalkFrame(ru: "Что на столе́?", en: "what is on the table?"),
                                                       TalkFrame(ru: "Да́йте, пожа́луйста, …", en: "please give me …")],
                                              secret: "There is bread, soup, and water on the table")),
        Entry(afterUnit: 14, scene: TalkScene(title: "At breakfast", ru: "За за́втраком",
                                              goal: "Find out what there is to drink and order one thing",
                                              frames: [TalkFrame(ru: "Что есть …?", en: "what is there …?"),
                                                       TalkFrame(ru: "Я хочу́ …", en: "I want …")],
                                              secret: "There is tea and milk, but no coffee")),
        Entry(afterUnit: 8, scene: TalkScene(title: "Whose is it?", ru: "Чьё э́то?",
                                             goal: "Find out where the book and the letter are",
                                             frames: [TalkFrame(ru: "Где о́н / она́ / оно́?", en: "where is it?")],
                                             secret: "The book is on the table and the letter is in the bag")),
        Entry(afterUnit: 12, scene: TalkScene(title: "Speaking Russian", ru: "По-ру́сски",
                                              goal: "Find out which languages your partner speaks",
                                              frames: [TalkFrame(ru: "Вы говори́те по-…?", en: "do you speak …?")],
                                              secret: "Speaks Russian and a little English")),
    ]

    @MainActor
    static func currentUnitIndex(in book: Book) -> Int { book.currentUnitIndex }

    @MainActor
    static func isUnlocked(afterUnit: Int, in book: Book) -> Bool { book.hasReachedUnit(afterUnit) }

    @MainActor
    static func isUnlocked(unit: String, in book: Book) -> Bool { book.hasReachedUnit(unit) }

    @MainActor
    static func available(for book: Book) -> [TalkScene] {
        starters.filter { isUnlocked(afterUnit: $0.afterUnit, in: book) }.map(\.scene)
    }
}
