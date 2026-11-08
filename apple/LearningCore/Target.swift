import Foundation

/// Something the learner wants to be able to do, as a weighted set of components: a text to
/// read (each running word a unit, weighted by how much it matters), a proof to follow, a
/// score to play. The core knows nothing about what the units are; the domain builds them.
public struct Target: Sendable, Hashable {
    /// One slot that must be known: it counts as known when any candidate is retrievable.
    public struct Unit: Sendable, Hashable {
        /// What learning it would mean (a lexeme): units with the same key are one thing to learn.
        public var key: ComponentID
        /// Components whose memory covers this slot (an inflected form, or its lexeme).
        public var candidates: [ComponentID]
        public var weight: Double

        public init(key: ComponentID, candidates: [ComponentID]? = nil, weight: Double = 1) {
            self.key = key
            self.candidates = candidates ?? [key]
            self.weight = weight
        }
    }

    public var units: [Unit]
    /// Consecutive stretches of units that stand alone (sentences of a text), as ranges into `units`.
    public var segments: [Range<Int>]

    public init(units: [Unit], segments: [Range<Int>] = []) {
        self.units = units
        self.segments = segments
    }
}

/// How far a learner is from a target, at one moment.
public struct Distance: Sendable, Hashable {
    public struct Rung: Sendable, Hashable {
        public var key: ComponentID
        /// Summed weight of the units this key would unlock.
        public var value: Double
        /// How many units it covers.
        public var count: Int
    }

    /// Share of total weight whose unit is known (running words, if weights are 1).
    public var coverage: Double
    /// Distinct keys in the target, and how many of them are known.
    public var keys: Int
    public var knownKeys: Int
    /// Known share of each segment, in segment order.
    public var segmentCoverage: [Double]
    /// Unknown keys, most valuable first.
    public var ladder: [Rung]

    public var unknownKeys: Int { keys - knownKeys }
    /// Segments at or above `threshold` known.
    public func segmentsReadable(at threshold: Double) -> Int { segmentCoverage.filter { $0 >= threshold }.count }
}

extension KnowledgeModel {
    /// A unit is known when any of its candidates has strength ≥ `threshold` (best modality).
    public func knows(_ unit: Target.Unit, at date: Date, threshold: Double = 0.5) -> Bool {
        unit.candidates.contains { (strength($0, at: date) ?? 0) >= threshold }
    }

    /// The distance to a target now. A key counts as known when every unit of it is known;
    /// the ladder ranks unknown keys by the weight they would unlock (then by first appearance).
    public func distance(to target: Target, at date: Date, threshold: Double = 0.5, ladder limit: Int = 20) -> Distance {
        var total = 0.0, known = 0.0
        var keyWeight: [ComponentID: Double] = [:], keyCount: [ComponentID: Int] = [:], keyKnown: [ComponentID: Bool] = [:]
        var firstSeen: [ComponentID: Int] = [:]
        var unitKnown: [Bool] = []
        unitKnown.reserveCapacity(target.units.count)
        for (i, u) in target.units.enumerated() {
            let k = knows(u, at: date, threshold: threshold)
            unitKnown.append(k)
            total += u.weight
            if k { known += u.weight }
            keyWeight[u.key, default: 0] += u.weight
            keyCount[u.key, default: 0] += 1
            keyKnown[u.key] = (keyKnown[u.key] ?? true) && k
            if firstSeen[u.key] == nil { firstSeen[u.key] = i }
        }
        let segments = target.segments.map { r -> Double in
            let slice = r.clamped(to: 0..<target.units.count)
            guard !slice.isEmpty else { return 0 }
            let w = slice.reduce(0.0) { $0 + target.units[$1].weight }
            let kw = slice.reduce(0.0) { $0 + (unitKnown[$1] ? target.units[$1].weight : 0) }
            return w > 0 ? kw / w : 0
        }
        let ladder = keyWeight.keys.filter { keyKnown[$0] == false }
            .sorted { a, b in
                let wa = keyWeight[a]!, wb = keyWeight[b]!
                return wa != wb ? wa > wb : firstSeen[a]! < firstSeen[b]!
            }
            .prefix(limit)
            .map { Distance.Rung(key: $0, value: keyWeight[$0]!, count: keyCount[$0]!) }
        return Distance(coverage: total > 0 ? known / total : 0, keys: keyWeight.count,
                        knownKeys: keyKnown.values.filter { $0 }.count, segmentCoverage: segments, ladder: Array(ladder))
    }
}
