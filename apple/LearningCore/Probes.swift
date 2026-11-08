import Foundation

/// Measured retention: now and then a sample of learned items is probed at fixed delays,
/// outside the review schedule, and what FSRS predicted is set against what happened.
public struct ProbePolicy: Sendable, Equatable {
    public var enabled: Bool
    /// A new probe set is drawn this often.
    public var every: TimeInterval
    /// Each sampled item is probed once per delay.
    public var delays: [TimeInterval]
    /// Items per set, spread over the range of present recall (thirds).
    public var sampleSize: Int
    /// Probe outcomes before any nudge to the memory model's parameters.
    public var minObservations: Int

    public init(enabled: Bool = true, every: TimeInterval = 7 * 86_400,
                delays: [TimeInterval] = [86_400, 7 * 86_400, 30 * 86_400], sampleSize: Int = 6, minObservations: Int = 12) {
        self.enabled = enabled
        self.every = every
        self.delays = delays
        self.sampleSize = max(1, sampleSize)
        self.minObservations = max(1, minObservations)
    }
}

/// One scheduled probe of one candidate in one modality. Looks like an ordinary exercise on the
/// page; this record is the only mark.
public struct Probe: Codable, Sendable, Hashable, Identifiable {
    public var id: String
    /// The review candidate's id.
    public var candidate: String
    public var modality: Modality
    /// When its set was drawn.
    public var drawn: Date
    /// Not before this.
    public var due: Date
    /// The item ("page#index") it was placed on, once placed.
    public var item: String?
    /// Predicted recall just before it was answered.
    public var predicted: Double?
    /// Whether recall succeeded (a grade above `again`). nil while unanswered or if it went ungraded.
    public var observed: Bool?
    public var answered: Date?
    /// Assessment provenance. Optional so earlier notebooks continue to decode.
    public var lastExposure: Date? = nil
    public var firstAttempt: Bool? = nil
    public var assistance: [String]? = nil
    public var context: String? = nil

    public init(id: String = UUID().uuidString, candidate: String, modality: Modality, drawn: Date, due: Date) {
        self.id = id
        self.candidate = candidate
        self.modality = modality
        self.drawn = drawn
        self.due = due
    }

    public var isPending: Bool { item == nil }
    public var isObserved: Bool { predicted != nil && observed != nil }
}

/// Deterministic pseudo-randomness for sampling (SplitMix64), so a seed reproduces a draw.
struct SeededGenerator: RandomNumberGenerator {
    private var state: UInt64
    init(seed: UInt64) { state = seed }
    mutating func next() -> UInt64 {
        state &+= 0x9E37_79B9_7F4A_7C15
        var z = state
        z = (z ^ (z >> 30)) &* 0xBF58_476D_1CE4_E5B9
        z = (z ^ (z >> 27)) &* 0x94D0_49BB_1331_11EB
        return z ^ (z >> 31)
    }
}

extension KnowledgeModel {
    /// Draws a probe set: candidates practised in at least one of their modalities, stratified by
    /// present recall into thirds (low, middle, high) so weak and strong memories are both measured,
    /// one probe per delay for each, in a modality the candidate has been practised in.
    public func drawProbes(_ candidates: [ReviewCandidate], at date: Date, policy: ProbePolicy = ProbePolicy(),
                           seed: UInt64) -> [Probe] {
        var rng = SeededGenerator(seed: seed)
        let scored: [(ReviewCandidate, Modality, Double)] = candidates.compactMap { c in
            let practised = c.modalities.sorted().compactMap { m -> (Modality, Double)? in
                let rs = c.components.compactMap { retrievability($0, m, at: date) }
                guard rs.count == c.components.count, !rs.isEmpty else { return nil }
                return (m, rs.min()!)
            }
            guard !practised.isEmpty else { return nil }
            let pick = practised[Int(rng.next() % UInt64(practised.count))]
            return (c, pick.0, pick.1)
        }.sorted { $0.2 != $1.2 ? $0.2 < $1.2 : $0.0.id < $1.0.id }
        guard !scored.isEmpty else { return [] }
        let n = scored.count, cut1 = n / 3, cut2 = n - n / 3
        let thirds = [Array(scored[0..<cut1]), Array(scored[cut1..<cut2]), Array(scored[cut2..<n])].filter { !$0.isEmpty }
        var chosen: [(ReviewCandidate, Modality, Double)] = []
        var exhausted = Set<Int>()
        var round = 0
        // Round-robin over the bands: low, middle, high, low, …
        while chosen.count < min(policy.sampleSize, n), exhausted.count < thirds.count {
            let b = round % thirds.count
            round += 1
            guard !exhausted.contains(b) else { continue }
            let remaining = thirds[b].filter { r in !chosen.contains { $0.0.id == r.0.id } }
            if remaining.isEmpty { exhausted.insert(b); continue }
            chosen.append(remaining[Int(rng.next() % UInt64(remaining.count))])
        }
        return chosen.sorted { $0.0.id < $1.0.id }.flatMap { c, m, _ in
            policy.delays.map { Probe(candidate: c.id, modality: m, drawn: date, due: date.addingTimeInterval($0)) }
        }
    }
}

/// The probes drawn so far and their outcomes; what an app persists.
public struct ProbeLedger: Codable, Sendable, Equatable {
    public var lastDrawn: Date?
    public var probes: [Probe]

    public init(lastDrawn: Date? = nil, probes: [Probe] = []) {
        self.lastDrawn = lastDrawn
        self.probes = probes
    }

    public func isDrawDue(at date: Date, policy: ProbePolicy) -> Bool {
        guard policy.enabled else { return false }
        guard let lastDrawn else { return true }
        return date.timeIntervalSince(lastDrawn) >= policy.every
    }

    /// Probes whose time has come and that are not yet on a page, earliest first.
    public func pending(at date: Date) -> [Probe] {
        probes.filter { $0.isPending && $0.due <= date }.sorted { ($0.due, $0.id) < ($1.due, $1.id) }
    }

    public mutating func add(_ drawn: [Probe], at date: Date) {
        lastDrawn = date
        probes += drawn
    }

    public mutating func place(_ id: String, on item: String) {
        guard let i = probes.firstIndex(where: { $0.id == id }) else { return }
        probes[i].item = item
    }

    /// Records what happened on `item`, if a probe was placed there. `observed` nil: the item went
    /// ungraded (skipped); the probe is kept but counts for nothing.
    @discardableResult
    public mutating func observe(item: String, predicted: Double?, observed: Bool?, at date: Date,
                                 lastExposure: Date? = nil, firstAttempt: Bool? = nil,
                                 assistance: [String]? = nil, context: String? = nil) -> Probe? {
        guard let i = probes.firstIndex(where: { $0.item == item && $0.answered == nil }) else { return nil }
        probes[i].predicted = predicted
        probes[i].observed = observed
        probes[i].answered = date
        probes[i].lastExposure = lastExposure
        probes[i].firstAttempt = firstAttempt
        probes[i].assistance = assistance
        probes[i].context = context
        return probes[i]
    }

    public var observations: [Calibration.Observation] {
        probes.compactMap { p in
            guard let r = p.predicted, let ok = p.observed else { return nil }
            return Calibration.Observation(modality: p.modality, predicted: r, observed: ok)
        }
    }
}

/// Predicted recall against observed outcomes: a reliability curve per modality and overall.
public struct Calibration: Codable, Sendable, Equatable {
    public struct Observation: Codable, Sendable, Hashable {
        public var modality: Modality
        public var predicted: Double
        public var observed: Bool
        public init(modality: Modality, predicted: Double, observed: Bool) {
            self.modality = modality
            self.predicted = predicted
            self.observed = observed
        }
    }

    /// One band of predicted recall: what was predicted on average and what happened.
    public struct Bin: Codable, Sendable, Equatable {
        public var lower: Double
        public var upper: Double
        public var predicted: Double
        public var observed: Double
        public var count: Int
    }

    public struct Series: Codable, Sendable, Equatable {
        /// nil for all modalities together.
        public var modality: Modality?
        public var bins: [Bin]
        public var count: Int
        /// Mean predicted recall over its observations.
        public var predicted: Double
        /// Share recalled.
        public var observed: Double
        /// Mean squared error of the predictions (0 perfect, 0.25 is guessing).
        public var brier: Double
        /// Count-weighted mean |predicted − observed| over bins (expected calibration error).
        public var error: Double
        /// predicted − observed: positive when the model thinks the learner remembers more than they do.
        public var overconfidence: Double { predicted - observed }
    }

    public var series: [Series]
    public var overall: Series?
    public var binCount: Int

    public init(observations: [Observation], binCount: Int = 5) {
        let n = max(1, binCount)
        self.binCount = n
        var byModality: [Modality: [Observation]] = [:]
        for o in observations { byModality[o.modality, default: []].append(o) }
        series = byModality.keys.sorted().compactMap { m in Self.series(byModality[m]!, modality: m, bins: n) }
        overall = Self.series(observations, modality: nil, bins: n)
    }

    static func series(_ obs: [Observation], modality: Modality?, bins n: Int) -> Series? {
        guard !obs.isEmpty else { return nil }
        var bins: [Bin] = []
        for b in 0..<n {
            let lo = Double(b) / Double(n), hi = Double(b + 1) / Double(n)
            let inBin = obs.filter { $0.predicted >= lo && ($0.predicted < hi || (b == n - 1 && $0.predicted <= hi)) }
            guard !inBin.isEmpty else { continue }
            bins.append(Bin(lower: lo, upper: hi, predicted: inBin.map(\.predicted).reduce(0, +) / Double(inBin.count),
                            observed: Double(inBin.filter(\.observed).count) / Double(inBin.count), count: inBin.count))
        }
        let count = Double(obs.count)
        let predicted = obs.map(\.predicted).reduce(0, +) / count
        let observed = Double(obs.filter(\.observed).count) / count
        let brier = obs.reduce(0) { $0 + pow($1.predicted - ($1.observed ? 1 : 0), 2) } / count
        let error = bins.reduce(0) { $0 + abs($1.predicted - $1.observed) * Double($1.count) } / count
        return Series(modality: modality, bins: bins, count: obs.count, predicted: predicted, observed: observed, brier: brier, error: error)
    }
}

/// A bounded, slow correction of FSRS from measured retention: one small step per reckoning,
/// never past ±30% of the published stabilities.
public enum ParameterNudge {
    public static let step = 0.02
    public static let bounds = 0.7...1.3
    /// Overconfidence (predicted − observed) smaller than this is left alone.
    public static let deadband = 0.05

    /// The next stability scale from the current one and the overall calibration. Over-confident
    /// (the learner forgets more than predicted): stabilities shrink, reviews come sooner.
    public static func scale(current: Double, calibration: Calibration, minObservations: Int) -> Double {
        guard let all = calibration.overall, all.count >= minObservations else { return current }
        var k = current
        if all.overconfidence > deadband { k *= 1 - step }
        else if all.overconfidence < -deadband { k *= 1 + step }
        return min(bounds.upperBound, max(bounds.lowerBound, k))
    }
}

extension FSRS {
    /// The same scheduler with every stability it would produce scaled by `k`: the four initial
    /// stabilities, and the growth factor (e^w8) on each successful review.
    public func scalingStability(by k: Double) -> FSRS {
        guard k > 0, k != 1 else { return self }
        var w = self.w
        for i in 0..<4 { w[i] *= k }
        w[8] += log(k)
        return FSRS(w: w)
    }
}
