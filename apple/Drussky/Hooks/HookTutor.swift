import Foundation

/// Asks the tutor for keyword mnemonics and reads them back from its JSON.
@MainActor
enum HookTutor {
    enum Failure: LocalizedError {
        case unreadable
        var errorDescription: String? { "The tutor's hook didn't come back in a form I could use. Try again, or write your own." }
    }

    /// The hook contract. Parsed tolerantly (`parse`, `parseBatch`): other key names are accepted,
    /// prose around the object is ignored.
    static let system = """
    You write keyword mnemonics ("hooks") for one adult English speaker, Ember, a beginner in Russian: quick, \
    curious, fond of linguistics, mathematics and science fiction. A hook links the SOUND of a Russian word \
    to its MEANING through an English keyword that sounds like the word (or its stressed syllable), set in one \
    concrete, visual, slightly absurd image. Example: мы́ло (soap) → "MEAL-o — wash your hands before the meal". \
    Write the sound-alike in CAPITALS, then an em dash, then the image. At most 16 words. The sound-alike must \
    echo the stressed syllable of the Russian as it is pronounced, not as it is spelled. No vulgarity, no cultural \
    stereotypes, nothing about Ember. One emoji when an image for it exists, else "". Reply with ONE JSON \
    object and nothing else.
    """

    static func prompt(for word: Course.Word) -> String {
        """
        {"ru": "\(word.ru)", "en": "\(word.en)"}
        Reply: {"hook": "KEYWORD — the image", "emoji": "one emoji or empty"}
        """
    }

    static func batchPrompt(for words: [Course.Word]) -> String {
        let list = words.map { "{\"ru\": \"\($0.ru)\", \"en\": \"\($0.en)\"}" }.joined(separator: ",\n")
        return """
        Write one hook for EACH of these words. Reply: {"hooks": [{"ru": "the word exactly as given", "hook": "KEYWORD — the image", "emoji": "one emoji or empty"}, …]}
        [
        \(list)
        ]
        """
    }

    /// One hook for one word.
    static func suggest(for word: Course.Word, at date: Date, progress: @escaping (Int) -> Void = { _ in }) async throws -> MemoryHook {
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: prompt(for: word))], system: system, maxTokens: 300, effort: "low") {
            raw += chunk
            progress(raw.count)
        }
        guard let h = parse(raw, at: date) else { throw Failure.unreadable }
        return h
    }

    /// Hooks for many words in one call. Words the reply leaves out are simply absent.
    static func suggestBatch(for words: [Course.Word], at date: Date) async throws -> [String: MemoryHook] {
        guard !words.isEmpty else { return [:] }
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: batchPrompt(for: words))], system: system,
                                            maxTokens: 120 * words.count + 400, effort: "low") {
            raw += chunk
        }
        let hooks = parseBatch(raw, at: date)
        guard !hooks.isEmpty else { throw Failure.unreadable }
        return hooks
    }

    // MARK: Parsing

    private static func str(_ v: Any?) -> String? {
        if let s = v as? String { return s.trimmingCharacters(in: .whitespacesAndNewlines) }
        return nil
    }

    /// A hook object: `hook` (or `text`, `mnemonic`) is the one required field.
    static func hook(from any: [String: Any], at date: Date) -> MemoryHook? {
        guard let text = str(any["hook"] ?? any["text"] ?? any["mnemonic"]), !text.isEmpty else { return nil }
        let emoji = str(any["emoji"]).flatMap { e -> String? in
            // One emoji only: the first grapheme that is one.
            e.first(where: { $0.unicodeScalars.contains { $0.properties.isEmojiPresentation || $0.properties.isEmoji && $0.value > 0x2000 } }).map(String.init)
        }
        return MemoryHook(text: text, emoji: emoji, source: .tutor, created: date, model: Tutor.model)
    }

    static func parse(_ reply: String, at date: Date) -> MemoryHook? {
        guard let json = PageWriter.extractJSON(reply), let data = json.data(using: .utf8),
              let any = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        if let h = hook(from: any, at: date) { return h }
        // A single-item batch shape.
        if let list = any["hooks"] as? [Any], let first = list.first as? [String: Any] { return hook(from: first, at: date) }
        return nil
    }

    /// Word key (`RussianText.key`) → hook, for every item with a readable word and hook.
    static func parseBatch(_ reply: String, at date: Date) -> [String: MemoryHook] {
        guard let json = PageWriter.extractJSON(reply), let data = json.data(using: .utf8),
              let any = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return [:] }
        var out: [String: MemoryHook] = [:]
        for item in (any["hooks"] as? [Any]) ?? [] {
            guard let o = item as? [String: Any], let ru = str(o["ru"] ?? o["word"] ?? o["russian"]), let h = hook(from: o, at: date) else { continue }
            let key = RussianText.key(ru)
            if !key.isEmpty { out[key] = h }
        }
        return out
    }
}

/// Hooks for the words that have lapsed most, written in one batched call when a key is on the
/// device, at most once a day, and only for words still without one.
@MainActor
enum HookBatch {
    static let minimumWords = 5
    static let every: TimeInterval = 86_400

    static func isDue(_ book: Book) -> Bool {
        guard let key = Keychain.apiKey, !key.isEmpty else { return false }
        if let last = book.hooks.batchedAt, book.now().timeIntervalSince(last) < every { return false }
        return book.wordsWantingHooks(limit: 60).count >= minimumWords
    }

    static func runIfDue(_ book: Book) {
        guard isDue(book) else { return }
        Task { _ = try? await run(book) }
    }

    /// Writes hooks for up to 60 lapsed words; returns how many arrived. Words that already have
    /// a hook by the time the reply lands keep theirs.
    @discardableResult
    static func run(_ book: Book, limit: Int = 60) async throws -> Int {
        let words = book.wordsWantingHooks(limit: limit)
        book.hooks.batchedAt = book.now()
        guard !words.isEmpty else { return 0 }
        let hooks = try await HookTutor.suggestBatch(for: words, at: book.now())
        var n = 0
        for (key, h) in hooks where book.hooks.hooks[key] == nil && book.word(key) != nil {
            book.hooks.hooks[key] = h
            n += 1
        }
        return n
    }
}
