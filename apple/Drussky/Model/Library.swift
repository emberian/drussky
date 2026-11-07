import Foundation

/// Real texts beyond the course (library.json), plus the private pack when present.
struct Library: Codable, Sendable {
    var version: Int
    var texts: [Text]
    var sentences: [Sentence]?

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
        /// Set when loaded from the private pack (personal study only).
        var isPrivate: Bool? = nil
    }

    struct Sentence: Codable, Sendable, Hashable {
        var ru: String
        var en: String
        var audio: String?
        var speaker: String?
        var license: String?
        var source: String?
    }

    static let levels = ["A1", "A2", "B1", "B2", "C1", "C2"]

    /// Bundled library merged with PrivatePack/library.json if the build carries one.
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
        return lib
    }
}
