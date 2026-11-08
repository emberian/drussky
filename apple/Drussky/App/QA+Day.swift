import Foundation
import LearningCore

/// QA flags for the day's shape (after -qa-reset -qa-complete N):
///   -qa-day N     the pages were done N days ago; the page is turned: the morning page
///   -qa-away D    the same, D days (7 or more: the welcome-back page)
///   -qa-probes    a synthetic set of answered probes, so the cover chart has something to show
///   -qa-tired     the current page is marked shortened (the desk line)
extension QA {
    static var dayShift: Int? { value("-qa-day") ?? value("-qa-away") }

    static func openDay(_ book: Book) {
        if dayShift != nil { book.turnForward() }
        if has("-qa-probes") { syntheticProbes(book) }
        if has("-qa-tired"), let page = book.page {
            book.day.shortened = [page.id: "Shorter page today — you've been at it a while. (Answers take twice as long as when you started.)"]
            book.day.lastShortened = book.now()
        }
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
