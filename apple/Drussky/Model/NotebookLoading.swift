import Foundation

/// Why a notebook that exists could not be opened. The original file is never modified;
/// `backup` is a verified byte-for-byte copy when one could be made.
struct NotebookProblem: Error, Sendable, Equatable {
    enum Kind: String, Sendable {
        /// book.json exists but could not be read (protected, I/O error).
        case unreadable
        /// It was read but is not a notebook this app can decode.
        case undecodable
        /// It was written by a newer Drussky; saving over it could drop what this one doesn't know.
        case newerVersion
    }
    var kind: Kind
    var detail: String
    var backup: URL?

    var message: String {
        switch kind {
        case .unreadable: "Your notebook is there but couldn't be read just now (\(detail)). Nothing has been changed."
        case .undecodable: "Your notebook couldn't be understood (\(detail)). Nothing has been changed."
        case .newerVersion: "This notebook was written by a newer Drussky (format \(detail)). Update the app to open it; nothing has been changed."
        }
    }
}

/// What opening the notebook folder found.
enum NotebookLoad {
    case missing
    case loaded(BookState, repairs: [String])
    case unavailable(NotebookProblem)
}

struct NotebookError: LocalizedError {
    var message: String
    var errorDescription: String? { message }
}

extension BookStore {
    static func encoder() -> JSONEncoder {
        let encoder = JSONEncoder()
        encoder.dateEncodingStrategy = .iso8601
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        return encoder
    }

    static func decoder() -> JSONDecoder {
        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        return decoder
    }

    /// Distinguishes a missing notebook from one that exists but can't be used, and never
    /// touches the original. A decodable notebook is migrated to the current format and
    /// checked for invariants; when anything had to be repaired, a verified copy of the
    /// original is written first.
    func load() -> NotebookLoad {
        let fm = FileManager.default
        guard fm.fileExists(atPath: stateURL.path) else { return .missing }
        let data: Data
        do { data = try Data(contentsOf: stateURL) } catch {
            return .unavailable(NotebookProblem(kind: .unreadable, detail: (error as NSError).localizedDescription))
        }
        switch Self.decode(data) {
        case .failure(let problem):
            var p = problem
            p.backup = try? verifiedCopy(of: data, named: "book-unreadable")
            if let backup = p.backup {
                try? "\(problem.detail)\n".write(to: backup.appendingPathExtension("error.txt"), atomically: true, encoding: .utf8)
            }
            return .unavailable(p)
        case .success(var state):
            let repairs = NotebookValidation.repair(&state)
            if !repairs.isEmpty {
                // The live notebook is about to diverge from the file: keep the original first.
                guard let backup = try? verifiedCopy(of: data, named: "book-before-repair") else {
                    return .unavailable(NotebookProblem(kind: .unreadable, detail: "it needs repair and a backup copy could not be written"))
                }
                try? repairs.joined(separator: "\n").write(to: backup.appendingPathExtension("repairs.txt"), atomically: true, encoding: .utf8)
            }
            return .loaded(state, repairs: repairs)
        }
    }

    /// Reads the format version first and refuses notebooks from a newer app, then decodes and
    /// migrates older formats forward.
    static func decode(_ data: Data) -> Result<BookState, NotebookProblem> {
        struct Header: Decodable { var version: Int? }
        let decoder = Self.decoder()
        let version: Int
        do { version = try decoder.decode(Header.self, from: data).version ?? 1 } catch {
            return .failure(NotebookProblem(kind: .undecodable, detail: Self.describe(error)))
        }
        guard version <= BookState.currentVersion else {
            return .failure(NotebookProblem(kind: .newerVersion, detail: "\(version)"))
        }
        do {
            var state = try decoder.decode(BookState.self, from: data)
            BookMigration.migrate(&state, from: version)
            return .success(state)
        } catch {
            return .failure(NotebookProblem(kind: .undecodable, detail: Self.describe(error)))
        }
    }

    private static func describe(_ error: Error) -> String {
        if let e = error as? DecodingError {
            switch e {
            case .dataCorrupted(let c):
                return c.codingPath.isEmpty ? "it is not valid JSON" : "bad value at \(c.codingPath.map(\.stringValue).joined(separator: "."))"
            case .keyNotFound(let k, let c): return "missing \(k.stringValue) at \(c.codingPath.map(\.stringValue).joined(separator: "."))"
            case .typeMismatch(_, let c), .valueNotFound(_, let c):
                return "unexpected value at \(c.codingPath.map(\.stringValue).joined(separator: ".")): \(c.debugDescription)"
            @unknown default: return "\(e)"
            }
        }
        return (error as NSError).localizedDescription
    }

    /// Writes `data` beside the notebook under a unique name and reads it back to compare.
    func verifiedCopy(of data: Data, named prefix: String) throws -> URL {
        // Opening the same broken notebook again doesn't pile up identical copies.
        let existing = (try? FileManager.default.contentsOfDirectory(at: root, includingPropertiesForKeys: [.fileSizeKey])) ?? []
        for url in existing where url.lastPathComponent.hasPrefix(prefix + "-") && url.pathExtension == "json" {
            if (try? url.resourceValues(forKeys: [.fileSizeKey]).fileSize) == data.count, (try? Data(contentsOf: url)) == data { return url }
        }
        let url = root.appendingPathComponent("\(prefix)-\(Self.stamp()).json")
        try data.write(to: url, options: .withoutOverwriting)
        guard try Data(contentsOf: url) == data else { throw NotebookError(message: "backup copy did not read back identically") }
        return url
    }

    /// Moves book.json aside under a unique name (a rename, so no bytes are rewritten) and
    /// confirms it arrived, so a fresh notebook can start without touching the old one.
    @discardableResult
    func setAside() throws -> URL {
        let fm = FileManager.default
        let aside = root.appendingPathComponent("book-set-aside-\(Self.stamp()).json")
        try fm.moveItem(at: stateURL, to: aside)
        guard fm.fileExists(atPath: aside.path), !fm.fileExists(atPath: stateURL.path) else {
            throw NotebookError(message: "the notebook could not be moved aside")
        }
        return aside
    }

    static func stamp() -> String {
        let t = ISO8601DateFormatter().string(from: Date()).replacingOccurrences(of: ":", with: "-")
        return t + "-" + UUID().uuidString.prefix(6).lowercased()
    }

    func saveState(_ state: BookState) throws {
        let data = try Self.encoder().encode(state)
        try data.write(to: stateURL, options: .atomic)
    }
}

/// Format migrations, dispatched on the version the file was written with.
enum BookMigration {
    static func migrate(_ state: inout BookState, from version: Int) {
        state.version = BookState.currentVersion
    }
}

/// Invariants that JSON decoding alone doesn't establish. Repairs only ever drop or pad what
/// the views and model can't use; the original file is copied aside before any of it is saved.
enum NotebookValidation {
    static let memoryKinds: Set<String> = ["letter", "word"]

    static func isWellFormedKey(_ key: String) -> Bool {
        let parts = key.split(separator: ":", maxSplits: 1, omittingEmptySubsequences: false)
        guard parts.count == 2, memoryKinds.contains(String(parts[0])) else { return false }
        return !parts[1].trimmingCharacters(in: .whitespaces).isEmpty
    }

    static func repair(_ state: inout BookState) -> [String] {
        var repairs: [String] = []
        // Memories: one per well-formed key.
        var byKey: [String: Memory] = [:]
        var order: [String] = []
        for m in state.memories {
            guard isWellFormedKey(m.key) else { repairs.append("dropped malformed memory key \"\(m.key)\""); continue }
            if let existing = byKey[m.key] {
                repairs.append("merged duplicate memory \"\(m.key)\"")
                var merged = existing.seen >= m.seen ? existing : m
                merged.reps = max(existing.reps, m.reps)
                merged.lapses = max(existing.lapses, m.lapses)
                merged.seen = min(existing.seen, m.seen)
                byKey[m.key] = merged
            } else {
                byKey[m.key] = m
                order.append(m.key)
            }
        }
        if !repairs.isEmpty { state.memories = order.compactMap { byKey[$0] } }
        // Every exercise has exactly one result.
        for i in state.pages.indices {
            let page = state.pages[i]
            if page.results.count < page.exercises.count {
                repairs.append("page \(page.id): added \(page.exercises.count - page.results.count) missing result(s)")
                state.pages[i].results += Array(repeating: ExerciseResult(), count: page.exercises.count - page.results.count)
            } else if page.results.count > page.exercises.count {
                repairs.append("page \(page.id): dropped \(page.results.count - page.exercises.count) result(s) with no exercise")
                state.pages[i].results = Array(page.results.prefix(page.exercises.count))
            }
        }
        if state.courseCursor < 0 { state.courseCursor = 0; repairs.append("reset a negative course cursor") }
        let lastPage = max(0, state.pages.count - 1)
        if state.currentPage < 0 || state.currentPage > lastPage {
            state.currentPage = min(max(0, state.currentPage), lastPage)
        }
        return repairs
    }
}
