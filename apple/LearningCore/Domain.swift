import Foundation

/// What a subject (a language, a branch of mathematics…) supplies to the core. The core holds
/// no knowledge of the subject; everything subject-specific is behind these four calls.
public protocol KnowledgeDomain: Sendable {
    /// One graded thing the learner did, in the subject's own terms.
    associatedtype Attempt

    /// Stable name, written into exports ("drussky-ru").
    var name: String { get }
    /// The modalities this subject exercises, in display order.
    var modalities: [Modality] { get }
    /// Components and structural edges extracted from the curriculum.
    var graph: KnowledgeGraph { get }

    /// The evidence an attempt yields: which components it touched, in which modality, with
    /// what grade, and any confusions it reveals. nil for ungraded activity.
    func evidence(for attempt: Attempt) -> LearningEvent?

    /// Answer-comparison form ("Кни́га." → "книга").
    func normalize(_ answer: String) -> String

    /// The components each token of a free text would exercise, for coverage ("you know 80%").
    func tokens(in text: String) -> [[ComponentID]]
}

/// The graph export (stable JSON contract, LearningCore/README.md).
public struct GraphExport: Codable, Sendable {
    public struct ModalityMetrics: Codable, Sendable {
        public var retrievability: Double
        public var stability: Double
        public var difficulty: Double
        public var reviews: Int
        public var lapses: Int
        public var last: Date
        public var due: Date?
    }
    public struct Node: Codable, Sendable {
        public var id: ComponentID
        public var kind: ComponentKind
        public var label: String
        public var gloss: String?
        public var order: Int?
        public var unit: String?
        public var strength: Double?
        public var events: Int
        public var modalities: [String: ModalityMetrics]
    }

    public var format = "learningcore.graph"
    public var version = 1
    public var domain: String
    public var generated: Date
    public var modalities: [Modality]
    public var nodes: [Node]
    public var edges: [Edge]
    public var transfer: [TransferStat]
    public var frontier: [ComponentID]
    /// Measured retention from probes (Probes.swift), when any have been answered.
    public var calibration: Calibration? = nil

    public init(domain: String, modalities: [Modality], graph: KnowledgeGraph, model: KnowledgeModel, at date: Date) {
        self.domain = domain
        self.generated = date
        self.modalities = modalities
        nodes = graph.components.values.sorted { ($0.order, $0.id) < ($1.order, $1.id) }.map { c in
            var ms: [String: ModalityMetrics] = [:]
            for (m, t) in model.traces(of: c.id) {
                guard let s = t.state, let last = t.last else { continue }
                ms[m.rawValue] = ModalityMetrics(retrievability: t.retrievability(at: date), stability: s.stability,
                                                 difficulty: s.difficulty, reviews: t.reviews.count, lapses: t.lapses,
                                                 last: last, due: t.due)
            }
            return Node(id: c.id, kind: c.id.kind, label: c.label, gloss: c.gloss, order: c.order == .max ? nil : c.order,
                        unit: c.unit, strength: model.strength(c.id, at: date), events: model.history(of: c.id).count,
                        modalities: ms)
        }
        let known = Set(graph.components.keys)
        edges = graph.edges + model.confusionEdges(at: date, minWeight: 0).filter { known.contains($0.from) && known.contains($0.to) }
        transfer = model.transfer
        frontier = model.frontier(graph, at: date)
    }

    public func json() -> Data {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        e.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        return (try? e.encode(self)) ?? Data()
    }
}
