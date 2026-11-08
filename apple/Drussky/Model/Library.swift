import Foundation

/// Real texts beyond the course (library.json), plus the private pack when present, plus the
/// shelves bundled beside it (math.json: «Матема́тика»), each text knowing which shelf it is on.
struct Library: Codable, Sendable {
    var version: Int
    var texts: [Text]
    var sentences: [Sentence]?
    /// A shelf's own file names its collection (COURSE_SCHEMA.md, "Original reading collections").
    var collection: Collection? = nil
    /// The shelves `load()` merged in, in order; nil in a file.
    var shelves: [Collection]? = nil

    struct Text: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var title: String
        var title_en: String?
        var author: String
        var year: Int?
        var level: String
        var topic: String
        var text: String
        var summary_en: String?
        var license: String
        var source: String
        /// A human reading of the text: an Audio/ clip, its word timings file, and who read it
        /// (scripts/narration.py; Narration.swift). Absent when only the synthetic voice can read it.
        var narration: String? = nil
        var words: String? = nil
        var reader: String? = nil
        /// Set when loaded from the private pack (personal study only).
        var isPrivate: Bool? = nil

        /// Authored reading support, on a shelf's texts only (library.json has none): the rung
        /// and the course page it is meant to follow, a full translation, glosses per written
        /// form, three questions, and how its phrases and notation are read.
        var rung: Int? = nil
        var after_page: String? = nil
        var en: String? = nil
        var glosses: [String: String]? = nil
        var questions: [Question]? = nil
        var how_read: [Frame]? = nil
        var notation: [Notation]? = nil
        /// The shelf (collection id) it was loaded from; nil for the library proper.
        var shelf: String? = nil

        var isNarrated: Bool { narration != nil && words != nil }
        /// A reading written for the learner, with its support, rather than a found text.
        var isAuthoredReading: Bool { en != nil && glosses != nil }
    }

    struct Sentence: Codable, Sendable, Hashable {
        var ru: String
        var en: String
        var audio: String?
        var speaker: String?
        var license: String?
        var source: String?
        /// Tatoeba's record of who wrote each half and its licence (scripts/fetch_provenance.py);
        /// the speaker above is not the author. Shown on «Исто́чники · Sources».
        var ru_author: String?
        var en_author: String?
        var ru_license: String?
        var en_license: String?
        var en_source: String?
    }

    struct Collection: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var title: String
        var title_en: String?
        var description_en: String?
    }

    /// A comprehension question about a reading: `kind` is gist, detail or inference.
    struct Question: Codable, Sendable, Hashable {
        var kind: String
        var prompt: String
        var options: [String]
        var answer: Int
        var explanation: String?
    }

    /// How a phrase of the reading is read: the stressed Russian, its English, and the grammar behind it.
    struct Frame: Codable, Sendable, Hashable {
        var ru: String
        var en: String
        var note: String?
    }

    /// How a piece of notation is said in Russian.
    struct Notation: Codable, Sendable, Hashable {
        var symbol: String
        var ru: String
        var en: String
    }

    static let levels = ["A1", "A2", "B1", "B2", "C1", "C2"]
    /// Shelves bundled beside library.json, by resource name.
    static let shelfFiles = ["math"]

    /// Bundled library merged with PrivatePack/library.json if the build carries one, and the shelves.
    static func load() -> Library {
        func read(_ url: URL?) -> Library? {
            guard let url, let data = try? Data(contentsOf: url) else { return nil }
            return try? JSONDecoder().decode(Library.self, from: data)
        }
        var lib = read(Bundle.main.url(forResource: "library", withExtension: "json")) ?? Library(version: 1, texts: [], sentences: [])
        if let pack = read(Bundle.main.url(forResource: "PrivatePack", withExtension: nil)?.appendingPathComponent("library.json")) {
            lib.texts += pack.texts.map { var t = $0; t.isPrivate = true; return t }
            lib.sentences = (lib.sentences ?? []) + (pack.sentences ?? [])
        }
        for name in shelfFiles {
            guard let shelf = read(Bundle.main.url(forResource: name, withExtension: "json")), let c = shelf.collection else { continue }
            lib.texts += shelf.texts.map { var t = $0; t.shelf = c.id; return t }
            lib.sentences = (lib.sentences ?? []) + (shelf.sentences ?? [])
            lib.shelves = (lib.shelves ?? []) + [c]
        }
        return lib
    }

    /// The library proper: texts on no shelf.
    var loose: [Text] { texts.filter { $0.shelf == nil } }

    /// One shelf's texts, in rung order.
    func texts(onShelf id: String) -> [Text] {
        texts.filter { $0.shelf == id }.sorted { ($0.rung ?? 0, $0.id) < ($1.rung ?? 0, $1.id) }
    }
}

/// A graded serial (series.json): episodes in order, each a stressed passage with its glosses
/// and translation, three questions and a diary prompt, meant to follow a course page.
struct Serial: Codable, Sendable, Hashable, Identifiable {
    var id: String
    var title: String
    var title_en: String?
    var summary_en: String?
    /// What one item is called («эпизо́д» unless the file says, e.g., «диало́г»).
    var episode_word: String? = nil
    /// What one item is called in English ("episode" unless the file says, e.g., "dialogue").
    var episode_word_en: String? = nil
    var episodes: [Episode]

    struct Episode: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var number: Int
        var title: String
        var title_en: String?
        var after_page: String
        var text: String
        var en: String
        var glosses: [String: String]
        /// Ordinary read_meaning exercises, in the course's own format.
        var questions: [Exercise]
        var prompt: String
        /// One to three things to hear in the dialogue, stressed Russian with the English after.
        var notice: [String]? = nil
    }

    var itemWord: String { episode_word ?? "эпизо́д" }
    /// The same word for an English line ("24 episodes", "30 dialogues").
    var itemWordEn: String { episode_word_en ?? (["диало́г": "dialogue"][episode_word ?? ""] ?? "episode") }
    /// «Ста́нция · эпизо́д 3»
    func place(of ep: Episode) -> String { "\(title) · \(itemWord) \(ep.number)" }

    private struct File: Codable { var version: Int; var series: [Serial] }

    /// The bundled serials, in this order: «Ста́нция» (series.json), «Ку́хня» (kitchen.json).
    static let files = ["series", "kitchen"]

    static func load() -> [Serial] {
        files.flatMap { name -> [Serial] in
            guard let url = Bundle.main.url(forResource: name, withExtension: "json"), let data = try? Data(contentsOf: url),
                  let file = try? JSONDecoder().decode(File.self, from: data) else { return [] }
            return file.series
        }
    }
}
