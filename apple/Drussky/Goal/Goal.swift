import Foundation

/// A text the learner wants to read for real — "your book" — stored in the notebook. The
/// course is pulled toward it: the inside cover shows the distance, ＋ writes pages from its
/// own sentences, and when its first page becomes readable the book opens it.
///
/// Policies (all thresholds live here):
/// - A word is *known* when LearningCore's strength for its form or lemma is ≥ 0.5.
/// - A sentence is *readable* at ≥ 0.8 of its words known; a pull page's passage is 2–4
///   consecutive readable sentences (if none, the page teaches the words through phrases).
/// - The *first page* is the first `firstPageSentences` sentences; it opens as a special page
///   once every one of them is ≥ 0.9 known, once.
/// - The ladder values an unknown lemma by its occurrences in the text, plus
///   `firstPageBonus` per occurrence on the first page, so the first page comes sooner.
/// - Automatic pull (`Book.goalPullDue`): on reaching a brand-new course page that teaches no
///   unknown goal word, when a goal has a readable passage, no goal page was written in the
///   last two pages, and the tutor has a key.
struct Goal: Codable, Sendable, Hashable, Identifiable {
    enum Origin: String, Codable, Sendable { case library, pasted, url }

    /// One token the tutor lemmatized: dictionary form and a short English gloss.
    struct LemmaEntry: Codable, Sendable, Hashable {
        var lemma: String
        var en: String?
    }

    var id: String
    var title: String
    var author: String?
    var origin: Origin
    /// The library text id, or the URL, that it came from.
    var source: String?
    var text: String
    var created: Date
    /// token (RussianText.key) → lemma and gloss, from the tutor; filled in batches, cached here.
    var lemmaCache: [String: LemmaEntry]? = nil
    /// When the first page opened as a page of the book (nil: not yet).
    var firstPageOpened: Date? = nil
    /// Ids of pages written toward this goal, in order.
    var pulledPages: [String]? = nil
    /// Sentence indices that have already been a pulled page's passage.
    var usedSentences: [Int]? = nil

    static let knownThreshold = 0.5
    static let readableThreshold = 0.8
    static let firstPageThreshold = 0.9
    static let firstPageSentences = 5
    static let firstPageBonus = 2.0
    static let passageSentences = 2...4

    init(title: String, author: String? = nil, origin: Origin, source: String? = nil, text: String, created: Date) {
        id = "g" + UUID().uuidString.prefix(8).lowercased()
        self.title = title
        self.author = author
        self.origin = origin
        self.source = source
        self.text = Goal.clean(text)
        self.created = created
    }

    init(library t: Library.Text, created: Date) {
        self.init(title: t.title, author: t.author, origin: .library, source: t.id, text: t.text, created: created)
    }

    var byline: String { [author, title.isEmpty ? nil : "«\(title)»"].compactMap { $0 }.joined(separator: ", ") }

    /// Normalizes line endings and whitespace; keeps paragraph breaks.
    static func clean(_ s: String) -> String {
        let unified = s.replacingOccurrences(of: "\r\n", with: "\n").replacingOccurrences(of: "\r", with: "\n")
        let paragraphs = unified.components(separatedBy: "\n").map { line in
            line.split(whereSeparator: { $0 == " " || $0 == "\t" || $0 == "\u{00A0}" }).joined(separator: " ")
        }
        var out: [String] = []
        for p in paragraphs {
            if p.isEmpty { if out.last != "" { out.append("") } } else { out.append(p) }
        }
        while out.last == "" { out.removeLast() }
        while out.first == "" { out.removeFirst() }
        return out.joined(separator: "\n")
    }
}
