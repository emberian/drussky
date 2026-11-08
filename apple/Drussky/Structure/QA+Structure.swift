import Foundation

/// QA launch argument for the structure layer (simulator screenshots):
///   -qa-structure <library id>   a reading page of that text as written, with «Структу́ра» already on,
///                                followed by its referent questions (Structure.page)
extension QA {
    static func applyStructure(to book: Book) {
        guard let id = string("-qa-structure"), let text = Library.load().texts.first(where: { $0.id == id }) else { return }
        let page = Structure.shared.page(for: text)
        book.settings.showStructure = true
        book.insertWrittenPage(title: page.title, intro: page.intro, unitTitle: "Library · \(text.level)",
                               exercises: page.exercises, newWords: [], sourceID: "structure:\(text.id)")
    }
}
