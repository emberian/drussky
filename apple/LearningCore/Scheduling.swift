import Foundation

/// What to review and when, from the FSRS traces: the one scheduler. The domain names the
/// things a learner should be able to recall (a word, a symbol) as candidates, each with the
/// components a review of it exercises and the modalities it can be reviewed in.
public struct ReviewPolicy: Sendable {
    /// Review when predicted recall falls below this.
    public var retention: Double
    /// After a lapse ("again"), the item comes back this soon, whatever its stability says.
    public var relearningStep: TimeInterval
    /// Never practised in any review modality: first recall this long after it was introduced.
    public var firstRecallDelay: TimeInterval

    public init(retention: Double = 0.9, relearningStep: TimeInterval = 10 * 60, firstRecallDelay: TimeInterval = 0.6 * 86_400) {
        self.retention = retention
        self.relearningStep = relearningStep
        self.firstRecallDelay = firstRecallDelay
    }
}

public struct ReviewCandidate: Sendable, Hashable {
    /// The domain's id for the thing to recall.
    public var id: String
    /// The components a review of it exercises (and whose traces say when it is due).
    public var components: [ComponentID]
    /// Modalities it can be reviewed in.
    public var modalities: Set<Modality>
    /// When the learner first met it (first recall is scheduled from here).
    public var introduced: Date
    /// Overrides the policy's first-recall delay.
    public var firstRecallDelay: TimeInterval?

    public init(id: String, components: [ComponentID], modalities: Set<Modality>, introduced: Date,
                firstRecallDelay: TimeInterval? = nil) {
        self.id = id
        self.components = components
        self.modalities = modalities
        self.introduced = introduced
        self.firstRecallDelay = firstRecallDelay
    }
}

public struct DueReview: Sendable, Hashable {
    public var id: String
    /// The modality to review in; nil for a first recall (the domain picks one).
    public var modality: Modality?
    /// Higher first: relearning, then lowest recall, then confusion pressure.
    public var priority: Double
    public var relearning: Bool

    public init(id: String, modality: Modality?, priority: Double, relearning: Bool) {
        self.id = id
        self.modality = modality
        self.priority = priority
        self.relearning = relearning
    }
}

extension MemoryTrace {
    /// The last review was a lapse: the trace is relearning.
    public var isRelearning: Bool { reviews.count > 1 && reviews.last?.grade == .again }

    /// When this trace next wants a review. A lapse brings it back after the relearning step;
    /// otherwise when predicted recall reaches `retention`.
    public func nextReview(_ policy: ReviewPolicy) -> Date? {
        guard let last, let s = state?.stability else { return nil }
        if reviews.last?.grade == .again { return last.addingTimeInterval(policy.relearningStep) }
        return last.addingTimeInterval(FSRS.interval(stability: s, desiredRetention: policy.retention) * 86_400)
    }
}

extension KnowledgeModel {
    /// The candidates due now, most urgent first, at most one modality each. A candidate is due
    /// in a modality when the weakest of its components' traces there wants a review; one never
    /// reviewed in any of its modalities is due for a first recall after its delay. Moving on
    /// without answering changes nothing here: only outcomes move a trace.
    public func dueReviews(_ candidates: [ReviewCandidate], at date: Date, policy: ReviewPolicy = ReviewPolicy()) -> [DueReview] {
        var out: [DueReview] = []
        for c in candidates where !c.components.isEmpty {
            var best: DueReview?
            var practised = false
            for m in c.modalities.sorted() {
                let ts = c.components.compactMap { trace($0, m) }
                guard !ts.isEmpty else { continue }
                practised = true
                let dueOnes = ts.filter { ($0.nextReview(policy) ?? .distantFuture) <= date }
                guard !dueOnes.isEmpty else { continue }
                let r = dueOnes.map { $0.retrievability(at: date) }.min() ?? 1
                let relearning = dueOnes.contains { $0.reviews.last?.grade == .again }
                let pressure = 0.15 * min(3, c.components.map { confusionWeight(of: $0, at: date) }.max() ?? 0)
                let p = (relearning ? 1 : 0) + (1 - r) + pressure
                if best == nil || p > best!.priority { best = DueReview(id: c.id, modality: m, priority: p, relearning: relearning) }
            }
            if let best { out.append(best); continue }
            if !practised, c.introduced.addingTimeInterval(c.firstRecallDelay ?? policy.firstRecallDelay) <= date {
                out.append(DueReview(id: c.id, modality: nil, priority: 0.5, relearning: false))
            }
        }
        return out.sorted { $0.priority != $1.priority ? $0.priority > $1.priority : $0.id < $1.id }
    }

    /// Secure now: every component has, in one of `modalities`, a trace with present recall at
    /// least `recall`, stability at least `stability` days, and no unrecovered lapse.
    /// (Present memory, not lifetime counts: an old lapse since recovered doesn't count.)
    public func isSecure(_ components: [ComponentID], in modalities: Set<Modality>, at date: Date,
                         recall: Double = 0.85, stability: Double = 2) -> Bool {
        guard !components.isEmpty else { return false }
        return components.allSatisfy { c in
            modalities.contains { m in
                guard let t = trace(c, m), let s = t.state, t.reviews.last?.grade != .again else { return false }
                return s.stability >= stability && t.retrievability(at: date) >= recall
            }
        }
    }
}
