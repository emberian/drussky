import Foundation

/// FSRS's four ratings.
public enum Grade: Int, Codable, Sendable, Comparable, CaseIterable {
    case again = 1, hard = 2, good = 3, easy = 4
    public static func < (a: Grade, b: Grade) -> Bool { a.rawValue < b.rawValue }
    public var isSuccess: Bool { self != .again }
}

/// Where a confusion was observed: what the learner wrote, said, picked, or heard.
public struct Channel: OpenName {
    public var rawValue: String
    public init(rawValue: String) { self.rawValue = rawValue }
}

/// The learner produced or chose `produced` when `target` was wanted.
public struct Confusion: Codable, Sendable, Hashable {
    public var target: ComponentID
    public var produced: ComponentID
    public var channel: Channel

    public init(target: ComponentID, produced: ComponentID, channel: Channel) {
        self.target = target
        self.produced = produced
        self.channel = channel
    }
}

/// One line of the append-only evidence log. The JSON shape is a stable contract
/// (LearningCore/README.md): fields are only ever added, never renamed or repurposed.
public struct LearningEvent: Codable, Sendable, Hashable, Identifiable {
    public enum Kind: String, Codable, Sendable {
        /// A try that did not finish the item (a wrong pick, a failed check).
        case attempt
        /// The item finished; carries the grade that feeds memory.
        case outcome
        /// The learner asked for help on the item (tutor, hint). Caps the next outcome at `hard`.
        case help
    }

    public static let schemaVersion = 1

    public var v: Int = LearningEvent.schemaVersion
    public var id: String
    public var t: Date
    public var type: Kind
    /// "live" when recorded as it happened, "backfill" when reconstructed from older records.
    public var source: String
    /// Stable id of the thing answered (one exercise on one page).
    public var item: String
    /// The domain's name for the activity (an exercise kind).
    public var activity: String?
    public var modality: Modality?
    public var components: [ComponentID]
    public var grade: Grade?
    /// For attempts: whether this try was right.
    public var correct: Bool?
    /// Attempts so far on the item, including this one.
    public var attempt: Int?
    public var expected: String?
    /// What the learner produced or chose (recognizer output, choice text).
    public var response: String?
    /// Seconds since the previous recorded activity in the same session context, when known.
    public var latency: Double?
    /// Response latency in milliseconds: from the prompt (the exercise shown, the clip's end, the
    /// cue to speak) to the response (the stroke, the tap, the voice's onset), as the domain
    /// measures it for the activity. Absent when the activity has no such moment (free writing,
    /// a conversation) or the record predates the field. Right and fast is "compiled"
    /// (`CompilationPolicy`), distinct from right.
    public var latencyMs: Int?
    /// Help used: "revealed", "tutor", …
    public var help: [String]?
    public var confusions: [Confusion]?
    /// The item was scheduled review rather than new material.
    public var review: Bool?
    /// Who decided the outcome was right or wrong: "self" (the learner, after seeing the model
    /// answer) or "reader" (a recognizer). Absent on outcomes the domain judged by comparison.
    public var judge: String?
    /// How sure the record is of the grade, 0–1: a self-verdict the reader agreed with is 1,
    /// one the reader could not confirm less, one it contradicted least.
    public var confidence: Double?

    public init(id: String = UUID().uuidString, t: Date, type: Kind, source: String = "live", item: String,
                activity: String? = nil, modality: Modality? = nil, components: [ComponentID], grade: Grade? = nil,
                correct: Bool? = nil, attempt: Int? = nil, expected: String? = nil, response: String? = nil,
                latency: Double? = nil, help: [String]? = nil, confusions: [Confusion]? = nil, review: Bool? = nil,
                judge: String? = nil, confidence: Double? = nil, latencyMs: Int? = nil) {
        self.judge = judge
        self.confidence = confidence
        self.latencyMs = latencyMs
        self.id = id
        self.t = t
        self.type = type
        self.source = source
        self.item = item
        self.activity = activity
        self.modality = modality
        self.components = components
        self.grade = grade
        self.correct = correct
        self.attempt = attempt
        self.expected = expected
        self.response = response
        self.latency = latency
        self.help = help
        self.confusions = confusions
        self.review = review
    }
}

/// Events as JSON lines in one file. Appends never rewrite earlier lines; a line that fails to
/// decode is skipped (and kept in the file), so one bad write can never lose the rest.
public struct EventLog: Sendable {
    public let url: URL

    public init(url: URL) { self.url = url }

    public static func encoder() -> JSONEncoder {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
        return e
    }

    public static func decoder() -> JSONDecoder {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        return d
    }

    public func load() -> [LearningEvent] {
        guard let data = try? Data(contentsOf: url) else { return [] }
        let decoder = Self.decoder()
        return data.split(separator: UInt8(ascii: "\n")).compactMap { try? decoder.decode(LearningEvent.self, from: Data($0)) }
    }

    @discardableResult
    public func append(_ events: [LearningEvent]) -> Bool {
        guard !events.isEmpty else { return true }
        let encoder = Self.encoder()
        var blob = Data()
        for e in events {
            guard let line = try? encoder.encode(e) else { continue }
            blob.append(line)
            blob.append(UInt8(ascii: "\n"))
        }
        let fm = FileManager.default
        if !fm.fileExists(atPath: url.path) {
            try? fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            return fm.createFile(atPath: url.path, contents: blob)
        }
        guard let handle = try? FileHandle(forUpdating: url) else { return false }
        defer { try? handle.close() }
        do {
            let end = try handle.seekToEnd()
            // A previous write cut short mid-line: start on a fresh line.
            if end > 0 {
                try handle.seek(toOffset: end - 1)
                if try handle.read(upToCount: 1) != Data([UInt8(ascii: "\n")]) { blob.insert(UInt8(ascii: "\n"), at: 0) }
                try handle.seekToEnd()
            }
            try handle.write(contentsOf: blob)
            return true
        } catch {
            return false
        }
    }
}
