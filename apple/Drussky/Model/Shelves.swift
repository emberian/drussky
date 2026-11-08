import Foundation

/// The shelves beside the course: «Ста́нция», a serial read one episode at a time, and
/// «Матема́тика», readings in rungs. Each item is meant to follow one course page (`after_page`)
/// and becomes an ordinary page: the passage with its glosses and translation, its questions as
/// read_meaning exercises, and (for an episode) a diary line about what comes next — so what the
/// learner does there reaches the knowledge tracker like anything else on a page. Nothing is
/// gated: the Library opens any item, ready or not; the course only offers.
enum Shelves {
    static let serials = Serial.load()
    static let library = Library.load()

    /// One thing on a shelf, as the Library, the ＋ menu and a page's footer name it.
    struct Item: Hashable, Identifiable, Sendable {
        enum Kind: Sendable { case episode, reading }
        var kind: Kind
        var sourceID: String
        var afterPage: String
        /// «Ста́нция · эпизо́д 3» / «Матема́тика · ступе́нь 2»
        var place: String
        var title: String
        var titleEn: String?
        var id: String { sourceID }
    }

    static func sourceID(episode: Serial.Episode) -> String { "episode:\(episode.id)" }
    static func sourceID(reading: Library.Text) -> String { "reading:\(reading.id)" }

    static func item(_ ep: Serial.Episode, in serial: Serial) -> Item {
        Item(kind: .episode, sourceID: sourceID(episode: ep), afterPage: ep.after_page,
             place: serial.place(of: ep), title: ep.title, titleEn: ep.title_en)
    }

    static func item(_ text: Library.Text, on shelf: Library.Collection) -> Item {
        Item(kind: .reading, sourceID: sourceID(reading: text), afterPage: text.after_page ?? "",
             place: "\(shelf.title) · ступе́нь \(text.rung ?? 0)", title: text.title, titleEn: text.title_en)
    }

    /// Every shelf item in its shelf's order.
    static var items: [Item] {
        serials.flatMap { s in s.episodes.map { item($0, in: s) } }
            + (library.shelves ?? []).flatMap { c in library.texts(onShelf: c.id).map { item($0, on: c) } }
    }

    static func item(sourceID: String) -> Item? { items.first { $0.sourceID == sourceID } }

    // MARK: Pages

    /// The English title as the first words of an intro: "A set. " / "The Milk Boiled Over! " / "".
    static func lead(_ titleEn: String?) -> String {
        guard let t = titleEn, !t.isEmpty else { return "" }
        return t.last.map { ".!?…".contains($0) } == true ? "\(t) " : "\(t). "
    }

    struct Page {
        var title: String
        var intro: String
        var unitTitle: String
        var exercises: [Exercise]
        var sourceID: String
    }

    /// An episode as a page: the passage, its three questions, and «Что бу́дет да́льше?».
    static func page(for ep: Serial.Episode, in serial: Serial) -> Page {
        var passage = Exercise(kind: .passage)
        passage.text = ep.text
        passage.en = ep.en
        passage.glosses = ep.glosses
        passage.notice = (ep.notice?.isEmpty ?? true) ? nil : ep.notice
        var diary = Exercise(kind: .diary)
        diary.prompt = ep.prompt
        let about = lead(ep.title_en)
        let hear = passage.notice == nil ? "" : " Once you have read it, what to hear in it is listed under the text."
        return Page(title: ep.title,
                    intro: "\(about)Read it; tap any word you want to check.\(hear) Then three questions about what it said, and a line of your own about what comes next.",
                    unitTitle: serial.place(of: ep),
                    exercises: [passage] + ep.questions.map { var q = $0; q.kind = .readMeaning; return q } + [diary],
                    sourceID: sourceID(episode: ep))
    }

    /// A reading as a page: the passage with how its phrases and notation are read, then its questions.
    static func page(for text: Library.Text, on shelf: Library.Collection) -> Page {
        var passage = Exercise(kind: .passage)
        passage.text = text.text
        passage.en = text.en
        passage.glosses = text.glosses
        let frames = (text.how_read ?? []).map { Exercise.Frame(ru: $0.ru, en: $0.en, note: $0.note) }
            + (text.notation ?? []).map { Exercise.Frame(ru: $0.ru, en: $0.en, symbol: $0.symbol) }
        passage.frames = frames.isEmpty ? nil : frames
        let questions = (text.questions ?? []).map { q -> Exercise in
            var e = Exercise(kind: .readMeaning)
            e.text = q.prompt
            e.options = q.options
            e.answerIndex = q.answer
            e.hint = q.explanation
            return e
        }
        let about = lead(text.title_en)
        let frameNote = frames.isEmpty ? "" : " How its phrases are read is listed under the text."
        return Page(title: text.title,
                    intro: "\(about)Read it; tap any word you want to check.\(frameNote) Then three questions about what it says.",
                    unitTitle: "\(shelf.title) · ступе́нь \(text.rung ?? 0)",
                    exercises: [passage] + questions,
                    sourceID: sourceID(reading: text))
    }
}

extension Book {
    /// A course page the learner has finished.
    func finished(coursePage id: String) -> Bool {
        pages.contains { $0.sourceID == id && $0.isFinished }
    }

    /// Whether the course has reached an item: its `after_page` is done.
    func isReady(_ item: Shelves.Item) -> Bool { finished(coursePage: item.afterPage) }

    /// The item is already a page in this notebook (opened counts as read; the page keeps its results).
    func hasOpened(_ item: Shelves.Item) -> Bool { pages.contains { $0.sourceID == item.sourceID } }

    /// The course page an item follows, named for the learner.
    func pageName(_ id: String) -> String? {
        course.pageSequence.first { $0.page.id == id }.map { "\($0.unit.title) · \($0.page.title)" }
    }

    /// The first episode of a serial not yet opened.
    func nextEpisode(of serial: Serial) -> Serial.Episode? {
        serial.episodes.first { !hasOpened(Shelves.item($0, in: serial)) }
    }

    /// What the course can offer now: on each shelf, the first unopened item, when it is ready.
    /// A later item being ready while an earlier one waits unread is still that earlier one.
    func shelfReady() -> [Shelves.Item] {
        var out: [Shelves.Item] = []
        for serial in Shelves.serials {
            if let ep = nextEpisode(of: serial) {
                let item = Shelves.item(ep, in: serial)
                if isReady(item) { out.append(item) }
            }
        }
        for shelf in Shelves.library.shelves ?? [] {
            if let next = Shelves.library.texts(onShelf: shelf.id).map({ Shelves.item($0, on: shelf) }).first(where: { !hasOpened($0) }),
               isReady(next) { out.append(next) }
        }
        return out
    }

    /// Items ready now that no earlier page has offered yet: written onto a new page's footer
    /// when it is composed (`appendPage`), once; never inserted.
    func shelfOffers() -> [String] {
        let offered = Set(pages.flatMap { $0.offers ?? [] })
        return shelfReady().map(\.sourceID).filter { !offered.contains($0) }
    }

    /// Opens an item as a page right after the current one.
    func open(_ item: Shelves.Item) {
        switch item.kind {
        case .episode:
            for serial in Shelves.serials {
                if let ep = serial.episodes.first(where: { Shelves.sourceID(episode: $0) == item.sourceID }) {
                    insert(Shelves.page(for: ep, in: serial)); return
                }
            }
        case .reading:
            for shelf in Shelves.library.shelves ?? [] {
                if let text = Shelves.library.texts(onShelf: shelf.id).first(where: { Shelves.sourceID(reading: $0) == item.sourceID }) {
                    insert(Shelves.page(for: text, on: shelf)); return
                }
            }
        }
    }

    private func insert(_ page: Shelves.Page) {
        insertWrittenPage(title: page.title, intro: page.intro, unitTitle: page.unitTitle, exercises: page.exercises,
                          newWords: [], sourceID: page.sourceID)
    }
}
