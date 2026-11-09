import Foundation

/// Memory is kept per (component, modality).
public struct MemoryKey: Hashable, Sendable, Codable, Comparable {
    public var component: ComponentID
    public var modality: Modality
    public init(_ component: ComponentID, _ modality: Modality) {
        self.component = component
        self.modality = modality
    }
    public static func < (a: Self, b: Self) -> Bool { (a.component, a.modality) < (b.component, b.modality) }
}

public struct Review: Codable, Sendable, Hashable {
    public var t: Date
    public var grade: Grade
    /// The state after this review.
    public var state: FSRS.State
    /// Retrievability just before it (nil for the first review).
    public var recallBefore: Double?
    /// How long the answer took (`LearningEvent.latencyMs`), when measured.
    public var latencyMs: Int?
}

/// The review history of one (component, modality), and what FSRS makes of it — plus how fast
/// the right answers come (`latencyEMA`), which FSRS does not see: a form produced after six
/// seconds of working it out and one produced in 600 ms are the same grade and a different
/// state of knowledge (`CompilationPolicy`).
public struct MemoryTrace: Codable, Sendable, Hashable {
    public private(set) var reviews: [Review] = []
    /// Exponential moving average of the latency of right answers, in milliseconds; nil until one
    /// was measured.
    public private(set) var latencyEMA: Double?
    /// Right answers whose latency was measured.
    public private(set) var latencySamples = 0

    public var state: FSRS.State? { reviews.last?.state }
    public var last: Date? { reviews.last?.t }
    public var lapses: Int { reviews.dropFirst().filter { $0.grade == .again }.count }
    public var due: Date? {
        guard let last, let s = state?.stability else { return nil }
        return last.addingTimeInterval(FSRS.interval(stability: s) * 86_400)
    }

    /// The last review at or before `date`.
    public func review(at date: Date) -> Review? {
        var lo = 0, hi = reviews.count
        while lo < hi {
            let mid = (lo + hi) / 2
            if reviews[mid].t <= date { lo = mid + 1 } else { hi = mid }
        }
        return lo == 0 ? nil : reviews[lo - 1]
    }

    /// 0 before the first review.
    public func retrievability(at date: Date) -> Double {
        guard let r = review(at: date) else { return 0 }
        return FSRS.retrievability(elapsedDays: date.timeIntervalSince(r.t) / 86_400, stability: r.state.stability)
    }

    /// Present strength of demonstrated recall at `date`: 0 before the first review or when every
    /// review at or before `date` was `.again` (never yet recalled), otherwise `retrievability(at: date)`.
    public func strength(at date: Date) -> Double {
        guard let r = review(at: date),
              reviews.contains(where: { $0.t <= date && $0.grade != .again }) else { return 0 }
        return FSRS.retrievability(elapsedDays: date.timeIntervalSince(r.t) / 86_400, stability: r.state.stability)
    }

    mutating func add(_ grade: Grade, at t: Date, fsrs: FSRS, latencyMs: Int? = nil, alpha: Double = CompilationPolicy.defaultAlpha) {
        let prev = reviews.last
        let elapsed = prev.map { t.timeIntervalSince($0.t) / 86_400 } ?? 0
        let before = prev.map { FSRS.retrievability(elapsedDays: elapsed, stability: $0.state.stability) }
        reviews.append(Review(t: t, grade: grade, state: fsrs.next(prev?.state, elapsedDays: elapsed, grade: grade),
                              recallBefore: before, latencyMs: latencyMs))
        // Only a right answer says how fast the knowledge comes; a wrong one measures something else.
        if let latencyMs, latencyMs >= 0, grade >= .good {
            let x = Double(latencyMs)
            latencyEMA = latencyEMA.map { $0 + alpha * (x - $0) } ?? x
            latencySamples += 1
        }
    }

    /// Share of the last `window` reviews answered right first time (good or easy).
    public func recentAccuracy(window: Int) -> Double? {
        let recent = reviews.suffix(max(1, window))
        guard !recent.isEmpty else { return nil }
        return Double(recent.filter { $0.grade >= .good }.count) / Double(recent.count)
    }
}

public struct ConfusionStat: Codable, Sendable, Hashable {
    public var target: ComponentID
    public var produced: ComponentID
    public var count = 0
    public var times: [Date] = []
    public var channels: Set<Channel> = []

    /// Each observation counts 1, halving every `halfLife` days.
    public func weight(at date: Date, halfLife: Double = 30) -> Double {
        times.filter { $0 <= date }.reduce(0) { $0 + pow(0.5, date.timeIntervalSince($1) / 86_400 / halfLife) }
    }
}

public typealias ConfusionEdge = ConfusionStat

/// Does strength in one modality carry over to the first try in another?
public struct TransferStat: Sendable, Hashable, Codable {
    public var from: Modality
    public var to: Modality
    /// First tries in `to` on components already strong (R ≥ 0.7) in `from`.
    public var primedSuccess = 0
    public var primedTrials = 0
    /// First tries in `to` on components with no evidence in any modality.
    public var coldSuccess = 0
    public var coldTrials = 0

    /// Smoothed success rates (one imagined success and one failure).
    public var primedRate: Double { Double(primedSuccess + 1) / Double(primedTrials + 2) }
    public var coldRate: Double { Double(coldSuccess + 1) / Double(coldTrials + 2) }
    public var lift: Double { primedRate - coldRate }
}

/// Everything the evidence says, folded event by event. Pure value: replaying the same log
/// gives the same model.
public struct KnowledgeModel: Codable, Sendable, Equatable {
    public let fsrs: FSRS
    public static let primedThreshold = 0.7

    public private(set) var events: [LearningEvent] = []
    /// `events` are in time order (as applied; a backfilled outcome older than its neighbours
    /// clears this). Readers of the session's tail (`Fatigue`) may then stop at the first gap.
    public private(set) var eventsAreTimeOrdered = true
    public private(set) var traces: [MemoryKey: MemoryTrace] = [:]
    public private(set) var confusions: [Pair: ConfusionStat] = [:]
    private var transferCounts: [ModalityPair: TransferStat] = [:]
    private var byComponent: [ComponentID: [Int]] = [:]
    private var modalitiesOf: [ComponentID: Set<Modality>] = [:]
    private var helped: Set<String> = []
    private var seenIDs: Set<String> = []

    public struct Pair: Codable, Hashable, Sendable { public var target: ComponentID; public var produced: ComponentID }
    struct ModalityPair: Codable, Hashable, Sendable { var from: Modality; var to: Modality }

    public struct Snapshot: Codable, Sendable, Equatable {
        public static let currentVersion = 2
        public var version: Int = Snapshot.currentVersion
        public var fsrs: FSRS
        public var prefixDigest: String?
        public var byteOffset: UInt64
        public var lastEventID: String?
        public var model: KnowledgeModel

        public init(version: Int = Snapshot.currentVersion, fsrs: FSRS? = nil, byteOffset: UInt64,
                    lastEventID: String? = nil, prefixDigest: String? = nil, model: KnowledgeModel) {
            self.version = version
            self.fsrs = fsrs ?? model.fsrs
            self.prefixDigest = prefixDigest
            self.byteOffset = byteOffset
            self.lastEventID = lastEventID ?? model.events.last?.id
            self.model = model
        }
    }

    public init(events: [LearningEvent] = [], fsrs: FSRS = FSRS()) {
        self.fsrs = fsrs
        let ordered = events.enumerated().sorted { ($0.element.t, $0.offset) < ($1.element.t, $1.offset) }.map(\.element)
        for e in ordered { apply(e) }
    }

    public var items: Set<String> { Set(events.map(\.item)) }

    /// Folds by event time, preserving arrival order for ties. Late evidence refolds all derived
    /// state, including help and transfer accounting; duplicate ids remain idempotent.
    public mutating func apply(_ e: LearningEvent) {
        guard !seenIDs.contains(e.id) else { return }
        if let last = events.last, e.t < last.t {
            self = KnowledgeModel(events: events + [e], fsrs: fsrs)
            return
        }
        seenIDs.insert(e.id)
        let index = events.count
        if let last = events.last, e.t < last.t { eventsAreTimeOrdered = false }
        events.append(e)
        for c in Set(e.components) { byComponent[c, default: []].append(index) }
        for x in e.confusions ?? [] {
            let key = Pair(target: x.target, produced: x.produced)
            var s = confusions[key] ?? ConfusionStat(target: x.target, produced: x.produced)
            s.count += 1
            s.times.append(e.t)
            s.channels.insert(x.channel)
            confusions[key] = s
            for c in [x.target, x.produced] where !e.components.contains(c) { byComponent[c, default: []].append(index) }
        }
        switch e.type {
        case .help:
            helped.insert(e.item)
        case .attempt:
            break
        case .outcome:
            guard var grade = e.grade, let m = e.modality else { return }
            if helped.remove(e.item) != nil { grade = min(grade, .hard) }
            for c in Set(e.components) {
                let key = MemoryKey(c, m)
                if traces[key] == nil { noteFirstTry(c, in: m, success: grade >= .good, at: e.t) }
                var trace = traces[key] ?? MemoryTrace()
                trace.add(grade, at: e.t, fsrs: fsrs, latencyMs: e.latencyMs)
                traces[key] = trace
                modalitiesOf[c, default: []].insert(m)
            }
        }
    }

    private mutating func noteFirstTry(_ c: ComponentID, in m: Modality, success: Bool, at t: Date) {
        let others = modalitiesOf[c] ?? []
        if others.isEmpty {
            // Cold for every modality pair ending in m.
            let froms = Set(traces.keys.map(\.modality)).subtracting([m])
            for from in froms {
                var s = transferCounts[ModalityPair(from: from, to: m)] ?? TransferStat(from: from, to: m)
                s.coldTrials += 1
                if success { s.coldSuccess += 1 }
                transferCounts[ModalityPair(from: from, to: m)] = s
            }
            return
        }
        for from in others where from != m {
            guard let trace = traces[MemoryKey(c, from)], trace.retrievability(at: t) >= Self.primedThreshold else { continue }
            var s = transferCounts[ModalityPair(from: from, to: m)] ?? TransferStat(from: from, to: m)
            s.primedTrials += 1
            if success { s.primedSuccess += 1 }
            transferCounts[ModalityPair(from: from, to: m)] = s
        }
    }

    // MARK: Memory

    public func trace(_ c: ComponentID, _ m: Modality) -> MemoryTrace? { traces[MemoryKey(c, m)] }

    public func traces(of c: ComponentID) -> [Modality: MemoryTrace] {
        Dictionary(uniqueKeysWithValues: (modalitiesOf[c] ?? []).compactMap { m in traces[MemoryKey(c, m)].map { (m, $0) } })
    }

    public func retrievability(_ c: ComponentID, _ m: Modality, at date: Date) -> Double? {
        trace(c, m).map { $0.retrievability(at: date) }
    }

    /// Best demonstrated strength over modalities; nil when there is no evidence at all.
    public func strength(_ c: ComponentID, at date: Date, in modalities: Set<Modality>? = nil) -> Double? {
        let rs = (modalitiesOf[c] ?? []).filter { modalities?.contains($0) ?? true }
            .compactMap { trace(c, $0)?.strength(at: date) }
        return rs.max()
    }

    public func hasEvidence(_ c: ComponentID) -> Bool { byComponent[c] != nil }

    /// Retrievability in `m` if practised there; otherwise an estimate from the strongest other
    /// modality scaled by how well that modality has transferred to `m` so far. No estimate
    /// until at least `minTrials` primed first tries back the transfer rate.
    public func estimatedRetrievability(_ c: ComponentID, _ m: Modality, at date: Date,
                                        minTrials: Int = 3) -> (value: Double, estimated: Bool)? {
        if let r = retrievability(c, m, at: date) { return (r, false) }
        let best = (modalitiesOf[c] ?? []).compactMap { from -> Double? in
            guard let r = retrievability(c, from, at: date) else { return nil }
            let stat = transferCounts[ModalityPair(from: from, to: m)] ?? TransferStat(from: from, to: m)
            guard stat.primedTrials >= minTrials else { return nil }
            return r * stat.primedRate
        }.max()
        return best.map { ($0, true) }
    }

    /// Events that touched `c` (as a component, or as what was produced instead of the target).
    public func history(of c: ComponentID) -> [LearningEvent] { (byComponent[c] ?? []).map { events[$0] } }

    public var transfer: [TransferStat] {
        transferCounts.values.sorted { ($0.from, $0.to) < ($1.from, $1.to) }
    }

    // MARK: Confusion

    /// Learned confused-with edges, weighted by decayed count, strongest first.
    public func confusionEdges(at date: Date, minWeight: Double = 0.25) -> [Edge] {
        confusions.values.compactMap { s in
            let w = s.weight(at: date)
            return w >= minWeight ? Edge(s.target, s.produced, .confusedWith, weight: w,
                                         note: s.channels.map(\.rawValue).sorted().joined(separator: ",")) : nil
        }.sorted { $0.weight != $1.weight ? $0.weight > $1.weight : ($0.from, $0.to) < ($1.from, $1.to) }
    }

    /// Unordered pairs with their combined weight, for "learn these side by side".
    public func contrastPairs(at date: Date, limit: Int = 5) -> [(ComponentID, ComponentID, Double)] {
        var merged: [Set<ComponentID>: Double] = [:]
        for e in confusionEdges(at: date, minWeight: 0) { merged[[e.from, e.to], default: 0] += e.weight }
        return merged.compactMap { pair, w -> (ComponentID, ComponentID, Double)? in
            let ab = pair.sorted()
            return ab.count == 2 ? (ab[0], ab[1], w) : nil
        }
        .sorted { $0.2 != $1.2 ? $0.2 > $1.2 : ($0.0, $0.1) < ($1.0, $1.1) }
        .prefix(limit).map { $0 }
    }

    public func confusionWeight(of c: ComponentID, at date: Date) -> Double {
        confusions.values.filter { $0.target == c || $0.produced == c }.reduce(0) { $0 + $1.weight(at: date) }
    }

    // MARK: What to do next

    /// Weakest practised memories, lowest retrievability first.
    public func weakest(at date: Date, kinds: Set<ComponentKind>? = nil, limit: Int = 10) -> [(MemoryKey, Double)] {
        let scored: [(MemoryKey, Double)] = traces.compactMap { key, trace in
            guard kinds?.contains(key.component.kind) ?? true else { return nil }
            return (key, trace.retrievability(at: date))
        }
        let sorted = scored.sorted { x, y in x.1 != y.1 ? x.1 < y.1 : x.0 < y.0 }
        return Array(sorted.prefix(limit))
    }

    /// Confusion-aware review order: due memories by forgetting plus confusion pressure, with
    /// each one's strongest confusion partner pulled in right after it, so the two are told
    /// apart side by side rather than met days apart.
    public func reviewOrder(at date: Date, retention: Double = 0.9, limit: Int = 12) -> [MemoryKey] {
        let due = traces.filter { $0.value.retrievability(at: date) < retention }
        func priority(_ k: MemoryKey) -> Double {
            (1 - (due[k]?.retrievability(at: date) ?? 1)) + 0.15 * min(3, confusionWeight(of: k.component, at: date))
        }
        var queue = due.keys.sorted { priority($0) != priority($1) ? priority($0) > priority($1) : $0 < $1 }
        var out: [MemoryKey] = []
        while let k = queue.first, out.count < limit {
            queue.removeFirst()
            out.append(k)
            let partners = confusions.values
                .filter { $0.target == k.component || $0.produced == k.component }
                .sorted { $0.weight(at: date) > $1.weight(at: date) }
                .map { $0.target == k.component ? $0.produced : $0.target }
            if let p = partners.first, let i = queue.firstIndex(where: { $0.component == p }) {
                out.append(queue.remove(at: i))
            } else if let p = partners.first, traces[MemoryKey(p, k.modality)] != nil, !out.contains(MemoryKey(p, k.modality)) {
                out.append(MemoryKey(p, k.modality))
            }
        }
        return Array(out.prefix(limit))
    }

    /// Not yet learned, but everything it depends on is: what is ready to learn next.
    /// Earliest in the curriculum first, then the ones that unlock the most.
    public func frontier(_ graph: KnowledgeGraph, at date: Date, ready: Double = 0.8,
                         kinds: Set<ComponentKind>? = nil, limit: Int = 12) -> [ComponentID] {
        let candidates = graph.components.values.filter { c in
            (kinds?.contains(c.id.kind) ?? true)
                && (strength(c.id, at: date) ?? 0) < 0.5
                && !traces(of: c.id).values.contains(where: \.isRelearning)
        }
        let unlocked = candidates.filter { c in
            let pre = graph.prerequisites(of: c.id)
            return !pre.isEmpty && pre.allSatisfy { (strength($0, at: date) ?? 0) >= ready }
        }
        // Sort keys once per candidate, not per comparison: `dependents(of:)` walks the edges.
        let keyed = unlocked.map { (id: $0.id, key: ($0.order, -graph.dependents(of: $0.id).count)) }
        return keyed.sorted { $0.key != $1.key ? $0.key < $1.key : $0.id < $1.id }.prefix(limit).map(\.id)
    }

    /// Practised strongly in one modality, weakly (or never) in another the learner has used.
    public func modalityGaps(at date: Date, among modalities: [Modality], kinds: Set<ComponentKind>? = nil,
                             strong: Double = 0.85, weak: Double = 0.6, limit: Int = 10)
        -> [(component: ComponentID, strong: Modality, weak: Modality, gap: Double)] {
        var out: [(ComponentID, Modality, Modality, Double)] = []
        for (c, ms) in modalitiesOf where kinds?.contains(c.kind) ?? true {
            let rs = ms.compactMap { m in retrievability(c, m, at: date).map { (m, $0) } }
            guard let best = rs.max(by: { $0.1 < $1.1 }), best.1 >= strong else { continue }
            let worst = modalities.filter { $0 != best.0 }.map { m in (m, retrievability(c, m, at: date) ?? 0) }
                .filter { ms.contains($0.0) || $0.1 == 0 }
                .min(by: { $0.1 < $1.1 })
            if let worst, worst.1 <= weak, ms.contains(worst.0) { out.append((c, best.0, worst.0, best.1 - worst.1)) }
        }
        return out.sorted { $0.3 != $1.3 ? $0.3 > $1.3 : $0.0 < $1.0 }.prefix(limit).map { $0 }
    }

    // MARK: Over time

    /// Expected number of `components` retrievable at each date, one series per modality group:
    /// Σ over components of the best R within the group (0 before first practice).
    public func growth(of components: [ComponentID], groups: [Set<Modality>], at dates: [Date]) -> [[Double]] {
        groups.map { group in
            dates.map { d in
                components.reduce(0) { sum, c in
                    sum + (group.compactMap { traces[MemoryKey(c, $0)]?.retrievability(at: d) }.max() ?? 0)
                }
            }
        }
    }

    /// Daily dates from the first event to `end`.
    public func days(until end: Date, calendar: Calendar = .current) -> [Date] {
        guard let first = events.map(\.t).min() else { return [] }
        var d = calendar.startOfDay(for: first)
        var out: [Date] = []
        while d <= end {
            out.append(min(end, calendar.date(byAdding: .day, value: 1, to: d)!.addingTimeInterval(-1)))
            d = calendar.date(byAdding: .day, value: 1, to: d)!
        }
        return out
    }

    // MARK: Coverage

    /// Fraction of tokens known: a token is a list of candidate components (e.g. its form and
    /// its lexeme); it counts as known when any candidate's strength reaches `threshold`.
    public func coverage(_ tokens: [[ComponentID]], at date: Date, threshold: Double = 0.5) -> Double {
        guard !tokens.isEmpty else { return 0 }
        let known = tokens.filter { $0.contains { (strength($0, at: date) ?? 0) >= threshold } }.count
        return Double(known) / Double(tokens.count)
    }
}
