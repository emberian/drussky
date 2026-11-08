import Foundation

/// QA launch arguments for reading pages (simulator screenshots):
///   -qa-structure <library id>   a reading page of that text as written, with «Структу́ра» already on,
///                                followed by its referent questions (Structure.page)
///   -qa-library <library id>     the same page read plainly (the stress layer, Model/LibraryStress.swift)
///   -qa-ear                      an ear-training page from the first Tatoeba sentences, clips or not
///   -qa-stress-off               «Show stress» off (the page as it read before the stress layer)
extension QA {
    static func applyStructure(to book: Book) {
        if has("-qa-stress-off") { book.settings.showStress = false }
        if has("-qa-ear"), let sentences = Library.load().sentences {
            var exercises: [Exercise] = []
            for (i, s) in sentences.prefix(6).enumerated() {
                var e = Exercise(kind: i % 2 == 0 ? .dictation : .listenPick)
                if e.kind == .dictation { e.text = s.ru } else { e.say = s.ru; e.options = [s.ru] + sentences.dropFirst(i + 1).prefix(2).map(\.ru) }
                exercises.append(e)
            }
            book.insertWrittenPage(title: "Ear training", intro: "Real voices saying sentences built from words you know.",
                                   unitTitle: "Listening", exercises: exercises, newWords: [], sourceID: "qa-ear")
        }
        for (flag, structure) in [("-qa-structure", true), ("-qa-library", false)] {
            guard let id = string(flag), let text = Library.load().texts.first(where: { $0.id == id }) else { continue }
            let page = Structure.shared.page(for: text)
            if structure { book.settings.showStructure = true }
            book.insertWrittenPage(title: page.title, intro: page.intro, unitTitle: "Library · \(text.level)",
                                   exercises: page.exercises, newWords: [], sourceID: "structure:\(text.id)")
        }
    }
}
