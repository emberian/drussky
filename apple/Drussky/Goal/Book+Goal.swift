import Foundation
import LearningCore

/// Analyses are rebuilt only when the goal text, its lemma table or the domain's words change.
@MainActor
enum GoalAnalysisCache {
    static var entries: [String: (stamp: String, analysis: GoalAnalysis)] = [:]
}

extension BookPage {
    /// The page that opened the first page of a goal text.
    var isGoalFirstPage: Bool { sourceID?.hasPrefix("goal-first:") ?? false }
    /// A page written toward a goal (including its first page).
    var isGoalPage: Bool { sourceID?.hasPrefix("goal") ?? false }
}

extension Book {
    // MARK: Goals

    var currentGoal: Goal? { goals.first { $0.id == currentGoalID } ?? goals.first }

    func addGoal(_ goal: Goal) {
        goals.append(goal)
        currentGoalID = goal.id
    }

    func removeGoal(id: String) {
        goals.removeAll { $0.id == id }
        if currentGoalID == id { currentGoalID = goals.first?.id }
    }

    /// Changes the stored goal with this id in place. Work that spans an await (lemma filling, a
    /// page being written) never holds a whole goal across it: it patches the fields it owns, so
    /// two such updates landing in either order keep each other's progress.
    func updateGoal(id: String, _ patch: (inout Goal) -> Void) {
        guard let i = goals.firstIndex(where: { $0.id == id }) else { return }
        var g = goals[i]
        patch(&g)
        goals[i] = g
    }

    /// Lemma entries the tutor supplied, merged into the goal's table (newer entries win).
    func mergeLemmas(goalID: String, _ entries: [String: Goal.LemmaEntry]) {
        guard !entries.isEmpty else { return }
        updateGoal(id: goalID) { g in g.lemmaCache = (g.lemmaCache ?? [:]).merging(entries) { _, new in new } }
    }

    // MARK: Distance

    func analysis(for goal: Goal) -> GoalAnalysis {
        let stamp = "\(goal.text.count)|\(goal.lemmaCache?.count ?? 0)|\(knowledge.domain.words.count)"
        if let e = GoalAnalysisCache.entries[goal.id], e.stamp == stamp { return e.analysis }
        let a = GoalAnalysis(goal: goal, domain: knowledge.domain)
        GoalAnalysisCache.entries[goal.id] = (stamp, a)
        return a
    }

    func distance(for goal: Goal) -> GoalDistance {
        analysis(for: goal).distance(model: knowledge.model, at: now())
    }

    /// How many of the goal's unknown words a page teaches (by the words its exercises produce).
    func goalYield(of page: BookPage, toward goal: Goal) -> Int {
        let d = distance(for: goal)
        let lemmaOf = d.analysis.lemmaByKey
        var taught = Set<String>()
        for ex in page.exercises {
            guard let key = memoryKey(for: ex), key.hasPrefix("word:") else { continue }
            for t in key.dropFirst(5).split(separator: " ") {
                let token = String(t)
                taught.insert(lemmaOf[token] ?? knowledge.domain.lemmas.lemma(token) ?? token)
            }
        }
        return taught.intersection(d.unknownLemmas).count
    }

    /// Automatic pull (policy in Goal.swift): the learner has just reached a brand-new course
    /// page that teaches nothing toward the goal, while a readable stretch of the goal waits.
    func goalPullDue(reaching page: BookPage) -> Bool {
        guard let goal = currentGoal, page.sourceID != nil, !page.isGoalPage,
              page.results.allSatisfy({ $0.status == .open }), index(ofPage: page.id) == pages.count - 1,
              !pages.suffix(3).dropLast().contains(where: \.isGoalPage),
              Keychain.apiKey?.isEmpty == false else { return false }
        let d = distance(for: goal)
        guard d.passageWindow(avoiding: Set(goal.usedSentences ?? [])) != nil else { return false }
        return goalYield(of: page, toward: goal) == 0
    }

    // MARK: Pages toward the goal

    /// Records a page written toward the goal and the sentences its passage used, on the goal as
    /// it is now (the brief's snapshot may predate lemma entries filled in meanwhile).
    func notePulled(pageID: String, goalID: String, sentences: Range<Int>?) {
        updateGoal(id: goalID) { g in
            g.pulledPages = (g.pulledPages ?? []) + [pageID]
            if let sentences { g.usedSentences = Array(Set((g.usedSentences ?? []) + Array(sentences))).sorted() }
        }
    }

    /// Inserts a page written toward `goal`. A page the book pulled by itself (`auto`), on reaching
    /// a brand-new course page, goes in front of that page: the learner reads it first and turns
    /// forward onto the course page, which otherwise stayed behind untouched while its reviews
    /// still counted toward the day and its source counted as delivered.
    func insertGoalPage(_ written: PageWriter.Written, exercises: [Exercise], goal: Goal, sentences: Range<Int>?, auto: Bool = false) {
        insertWrittenPage(title: written.title, intro: written.intro, unitTitle: "Toward your book",
                          exercises: exercises, newWords: written.new_words ?? [], sourceID: "goal:\(goal.id)",
                          beforeCurrentIfUntouched: auto)
        if let id = page?.id { notePulled(pageID: id, goalID: goal.id, sentences: sentences) }
    }

    /// The first page of the goal as a passage with tap-to-gloss: the original sentences, every
    /// word the learner doesn't know yet glossed from the tutor's table or the course.
    func firstPageExercise(for goal: Goal) -> Exercise {
        let d = distance(for: goal)
        let a = d.analysis
        let range = a.firstPageSentences
        var text = ""
        var lastParagraph = -1
        for i in range {
            let s = a.sentences[i]
            if !text.isEmpty { text += s.paragraph != lastParagraph ? "\n" : " " }
            text += s.text
            lastParagraph = s.paragraph
        }
        var glosses: [String: String] = [:]
        for i in range {
            for ti in a.sentences[i].tokens {
                let t = a.tokens[ti]
                if let g = a.glosses[t.lemma] { glosses[t.key] = g }
            }
        }
        var ex = Exercise(kind: .passage)
        ex.text = text
        ex.glosses = glosses.isEmpty ? nil : glosses
        return ex
    }

    /// When the first page is readable (≥ 0.98 production-backed form coverage in each of its
    /// sentences) and hasn't opened yet, opens it as a page beside the current one: in front of it
    /// when that page is untouched (it was just made for the learner, who turns forward onto it
    /// afterwards), after it otherwise. With `force`, opens it regardless (QA). Returns whether a
    /// page was inserted.
    @discardableResult
    func openFirstPageIfReady(force: Bool = false) -> Bool {
        guard let goal = currentGoal, goal.firstPageOpened == nil else { return false }
        let d = distance(for: goal)
        guard force || d.firstPageReady else { return false }
        let ex = firstPageExercise(for: goal)
        insertWrittenPage(title: goal.title, intro: "Try the first page of your book. Take it slowly; tap any word you want to check.",
                          unitTitle: "Your book · \(goal.author ?? "")".trimmingCharacters(in: CharacterSet(charactersIn: " ·")),
                          exercises: [ex], newWords: [], sourceID: "goal-first:\(goal.id)", beforeCurrentIfUntouched: true)
        updateGoal(id: goal.id) { $0.firstPageOpened = now() }
        return true
    }
}
