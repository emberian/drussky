import Foundation

/// A name from a vocabulary the domain defines (component kinds, modalities, edge kinds,
/// channels). The core never enumerates them; it only compares them.
public protocol OpenName: RawRepresentable, Hashable, Codable, Sendable, Comparable, ExpressibleByStringLiteral,
                          CustomStringConvertible where RawValue == String {
    init(rawValue: String)
}

public extension OpenName {
    init(stringLiteral value: String) { self.init(rawValue: value) }
    var description: String { rawValue }
    static func < (a: Self, b: Self) -> Bool { a.rawValue < b.rawValue }
}

/// What sort of thing a component is: a letter, a word, a grammar pattern, a proof technique…
public struct ComponentKind: OpenName {
    public var rawValue: String
    public init(rawValue: String) { self.rawValue = rawValue }
}

/// How a component is being exercised: hear→write, see→say, recognize… Memory is per
/// (component, modality), since knowing a word by ear is not knowing how to write it.
public struct Modality: OpenName {
    public var rawValue: String
    public init(rawValue: String) { self.rawValue = rawValue }
}

/// A typed id, written `kind:key` ("letter:Ж", "lex:читать"). Encodes as that one string.
public struct ComponentID: Hashable, Comparable, Codable, Sendable, LosslessStringConvertible, Identifiable {
    public var id: Self { self }
    public let kind: ComponentKind
    public let key: String

    public init(_ kind: ComponentKind, _ key: String) {
        self.kind = kind
        self.key = key
    }

    public init?(_ description: String) {
        let parts = description.split(separator: ":", maxSplits: 1).map(String.init)
        guard parts.count == 2, !parts[0].isEmpty else { return nil }
        self.init(ComponentKind(rawValue: parts[0]), parts[1])
    }

    public var description: String { "\(kind.rawValue):\(key)" }
    public static func < (a: Self, b: Self) -> Bool { a.description < b.description }

    public init(from decoder: Decoder) throws {
        let s = try decoder.singleValueContainer().decode(String.self)
        guard let id = ComponentID(s) else {
            throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "not kind:key: \(s)"))
        }
        self = id
    }

    public func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        try c.encode(description)
    }
}

/// One knowledge component: the smallest thing worth remembering separately.
public struct Component: Codable, Sendable, Hashable, Identifiable {
    public var id: ComponentID
    /// What to draw for it ("Ж", "кот", "past -л -ла -ли").
    public var label: String
    /// Meaning or explanation.
    public var gloss: String?
    /// Position of its first appearance in the curriculum (smaller is earlier); Int.max if never taught.
    public var order: Int
    /// The curriculum grouping it first appears in.
    public var unit: String?

    public init(id: ComponentID, label: String, gloss: String? = nil, order: Int = .max, unit: String? = nil) {
        self.id = id
        self.label = label
        self.gloss = gloss
        self.order = order
        self.unit = unit
    }
}

public struct EdgeKind: OpenName {
    public var rawValue: String
    public init(rawValue: String) { self.rawValue = rawValue }

    /// `from` is an inflected or variant form of the `to` lexeme.
    public static let formOf: EdgeKind = "form-of"
    /// `from` is built out of `to` (a word uses a letter). `to` comes first.
    public static let uses: EdgeKind = "uses"
    /// `from` is a part of the whole `to` (a word in a phrase). `from` comes first.
    public static let partOf: EdgeKind = "part-of"
    /// `from` is an instance of the pattern `to`.
    public static let exemplifies: EdgeKind = "exemplifies"
    /// `from` should be learned before `to`.
    public static let prerequisite: EdgeKind = "prerequisite"
    /// Learned from evidence: the learner produced or chose `to` when `from` was wanted.
    public static let confusedWith: EdgeKind = "confused-with"
}

public struct Edge: Codable, Sendable, Hashable {
    public var from: ComponentID
    public var to: ComponentID
    public var kind: EdgeKind
    public var weight: Double
    public var note: String?

    public init(_ from: ComponentID, _ to: ComponentID, _ kind: EdgeKind, weight: Double = 1, note: String? = nil) {
        self.from = from
        self.to = to
        self.kind = kind
        self.weight = weight
        self.note = note
    }

    /// The (earlier, later) pair when this edge orders learning, else nil.
    public var dependency: (before: ComponentID, after: ComponentID)? {
        switch kind {
        case .prerequisite, .partOf: (from, to)
        case .uses: (to, from)
        default: nil
        }
    }
}

/// The structural graph a domain extracts from its curriculum. Learned edges live in the model.
public struct KnowledgeGraph: Sendable {
    public private(set) var components: [ComponentID: Component]
    public private(set) var edges: [Edge]
    private var outgoing: [ComponentID: [Int]] = [:]
    private var incoming: [ComponentID: [Int]] = [:]

    public init(components: [Component], edges: [Edge]) {
        var byID: [ComponentID: Component] = [:]
        for c in components {
            // First declaration wins, but an earlier curriculum position is kept.
            if let existing = byID[c.id] {
                var merged = existing
                merged.order = min(existing.order, c.order)
                if merged.gloss == nil { merged.gloss = c.gloss }
                byID[c.id] = merged
            } else {
                byID[c.id] = c
            }
        }
        self.components = byID
        let known = Set(byID.keys)
        self.edges = Array(Set(edges.filter { known.contains($0.from) && known.contains($0.to) && $0.from != $0.to }))
            .sorted { ($0.from, $0.to, $0.kind) < ($1.from, $1.to, $1.kind) }
        for (i, e) in self.edges.enumerated() {
            outgoing[e.from, default: []].append(i)
            incoming[e.to, default: []].append(i)
        }
    }

    public subscript(id: ComponentID) -> Component? { components[id] }

    public func components(of kind: ComponentKind) -> [Component] {
        components.values.filter { $0.id.kind == kind }.sorted { ($0.order, $0.id) < ($1.order, $1.id) }
    }

    public func edges(from id: ComponentID, _ kind: EdgeKind? = nil) -> [Edge] {
        (outgoing[id] ?? []).map { edges[$0] }.filter { kind == nil || $0.kind == kind }
    }

    public func edges(to id: ComponentID, _ kind: EdgeKind? = nil) -> [Edge] {
        (incoming[id] ?? []).map { edges[$0] }.filter { kind == nil || $0.kind == kind }
    }

    /// Everything that should be learned before `id`, directly.
    public func prerequisites(of id: ComponentID) -> [ComponentID] {
        let all = edges(from: id) + edges(to: id)
        return Array(Set(all.compactMap { e in e.dependency.flatMap { $0.after == id ? $0.before : nil } })).sorted()
    }

    /// Everything that directly depends on `id`.
    public func dependents(of id: ComponentID) -> [ComponentID] {
        let all = edges(from: id) + edges(to: id)
        return Array(Set(all.compactMap { e in e.dependency.flatMap { $0.before == id ? $0.after : nil } })).sorted()
    }

    /// Adds components and edges (e.g. words the learner kept after the curriculum was built).
    public func merging(components extra: [Component], edges extraEdges: [Edge]) -> KnowledgeGraph {
        KnowledgeGraph(components: Array(components.values) + extra, edges: edges + extraEdges)
    }
}
