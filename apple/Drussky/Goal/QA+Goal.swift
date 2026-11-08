import Foundation

/// QA launch arguments for the goal (simulator screenshots):
///   -qa-goal <library id>   make that library text the learner's book
///   -qa-goal-first          open its first page now, ready or not
///   -qa-goal-pull           insert an offline page toward it
extension QA {
    static func applyGoal(to book: Book) {
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-qa-goal"), i + 1 < args.count,
           let text = Library.load().texts.first(where: { $0.id == args[i + 1] }) {
            if book.goals.contains(where: { $0.source == text.id }) == false {
                book.addGoal(Goal(library: text, created: book.now()))
            }
        }
        guard let goal = book.currentGoal else { return }
        if has("-qa-goal-pull") {
            let d = book.distance(for: goal)
            let brief = GoalPull.Brief(goal: goal, distance: d,
                                       patterns: d.patterns(domain: book.knowledge.domain, model: book.knowledge.model, at: book.now()))
            let met = Set(book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper))
            let page = GoalPull.localPage(brief, met: met, allMet: met.count >= book.course.letters.count)
            book.insertWrittenPage(title: page.title, intro: page.intro, unitTitle: "Toward your book", exercises: page.exercises,
                                   newWords: [], sourceID: "goal:\(goal.id)")
        }
        if has("-qa-goal-first") { book.openFirstPageIfReady(force: true) }
    }
}
