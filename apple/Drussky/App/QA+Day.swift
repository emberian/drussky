import Foundation
import LearningCore

/// QA flags for the day's shape (after -qa-reset -qa-complete N):
///   -qa-day N     the pages were done N days ago; the page is turned: the morning page
///   -qa-away D    the same, D days (7 or more: the welcome-back page)
///   -qa-probes    a synthetic set of answered probes, so the cover chart has something to show
///   -qa-tired     the offer to make the rest of the page lighter stands on the current page
extension QA {
    static var dayShift: Int? { value("-qa-day") ?? value("-qa-away") }

    static func openDay(_ book: Book) {
        if dayShift != nil { book.turnForward() }
        if has("-qa-probes") { syntheticProbes(book) }
        if has("-qa-tired") { poseLighterOffer(book) }
    }

    /// The offer needs untouched reviews to change: when the current page has none, a written page
    /// of review dictations stands in (the same shape DayTests use), with its first two answered.
    private static func poseLighterOffer(_ book: Book) {
        if let page = book.page, book.lightening(of: page).isEmpty || page.isFinished {
            let words = ["кот", "мама", "окно", "она", "книга", "дом", "сок", "нос", "рот", "стол"].filter { book.word($0) != nil }
            let exercises: [Exercise] = words.map { w in
                var d = Exercise(kind: .dictation); d.text = w; d.reviewKey = "word:\(w)"; return d
            }
            book.insertWrittenPage(title: "A long page", intro: nil, unitTitle: "Повторение · Review", exercises: exercises, newWords: [])
            for i in 0..<2 where i < words.count {
                book.record(page: book.currentPage, exercise: i) { $0.attempts = 1; $0.recognized = words[i]; $0.status = .correct }
            }
        }
        guard let page = book.page else { return }
        book.day.lighterOffer = page.id
    }

    /// Thirty answered probes with a plausible spread: slightly overconfident predictions.
    private static func syntheticProbes(_ book: Book) {
        var ledger = ProbeLedger(lastDrawn: book.now())
        var rng = SystemRandomNumberGenerator()
        let modalities: [Modality] = [.hearWrite, .meaningWrite, .say, .hearPick]
        for i in 0..<30 {
            let m = modalities[i % modalities.count]
            var p = Probe(candidate: "word:qa\(i)", modality: m, drawn: book.now().addingTimeInterval(-8 * 86_400),
                          due: book.now().addingTimeInterval(-86_400))
            let predicted = 0.35 + 0.6 * Double(i % 10) / 9
            p.item = "qa#\(i)"
            p.predicted = predicted
            p.observed = Double.random(in: 0...1, using: &rng) < predicted - 0.1
            p.answered = book.now()
            ledger.probes.append(p)
        }
        book.day.probes = ledger
    }
}
