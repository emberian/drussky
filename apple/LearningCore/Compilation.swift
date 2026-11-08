import Foundation

/// Known is not the same as compiled. A memory is *known* when FSRS expects it to be recalled;
/// it is *compiled* when the right answer also comes fast, without the pause of working it out,
/// so it costs the learner nothing to hold while something new is stacked on it. Correctness
/// alone cannot tell the two apart; response latency can. Every threshold lives here.
public struct CompilationPolicy: Sendable, Equatable {
    /// Weight of the newest right answer's latency in the running average. 0.3 lets the last
    /// three or four answers dominate, so a word that has just become quick is seen as quick
    /// within a session, while one slow answer does not undo a week of fast ones.
    public var alpha: Double
    public static let defaultAlpha = 0.3
    /// Share of the last `window` reviews that must be right first time. 0.9 over a window of 5
    /// means every one of them: compiled knowledge does not stumble.
    public var accuracy: Double
    public var window: Int
    /// Right answers with a measured latency before the average means anything: three, so a
    /// single lucky tap never compiles a word.
    public var minSamples: Int
    /// Milliseconds under which a right answer counts as fast, per modality (the domain sets them:
    /// choosing by ear is quicker than writing a word); `defaultFloorMs` for one not listed.
    public var floorsMs: [Modality: Double]
    public var defaultFloorMs: Double

    public init(alpha: Double = CompilationPolicy.defaultAlpha, accuracy: Double = 0.9, window: Int = 5, minSamples: Int = 3,
                floorsMs: [Modality: Double] = [:], defaultFloorMs: Double = 4_000) {
        self.alpha = alpha
        self.accuracy = accuracy
        self.window = max(1, window)
        self.minSamples = max(1, minSamples)
        self.floorsMs = floorsMs
        self.defaultFloorMs = defaultFloorMs
    }

    public func floor(for m: Modality) -> Double { floorsMs[m] ?? defaultFloorMs }
}

extension MemoryTrace {
    /// Whether this memory is compiled in its modality: enough right answers with a measured
    /// latency, their average under the modality's floor, and the recent reviews right.
    public func isCompiled(_ policy: CompilationPolicy, modality: Modality) -> Bool {
        guard latencySamples >= policy.minSamples, let ema = latencyEMA, ema < policy.floor(for: modality),
              let acc = recentAccuracy(window: policy.window) else { return false }
        return acc >= policy.accuracy
    }

    /// Whether latency has been measured often enough to say anything about compilation.
    public func hasLatencyEvidence(_ policy: CompilationPolicy) -> Bool { latencySamples >= policy.minSamples }
}

extension KnowledgeModel {
    public func isCompiled(_ c: ComponentID, _ m: Modality, policy: CompilationPolicy) -> Bool {
        trace(c, m)?.isCompiled(policy, modality: m) ?? false
    }

    /// The modalities in which `c` is compiled.
    public func compiledModalities(of c: ComponentID, policy: CompilationPolicy) -> Set<Modality> {
        Set(traces(of: c).filter { $0.value.isCompiled(policy, modality: $0.key) }.keys)
    }

    /// Known, measured, and not yet quick: a component whose recall is strong (≥ `ready`) and whose
    /// latency has been measured enough to judge, in no modality compiled. A component never
    /// timed is not flagged: no measurement, no claim.
    public func isKnownButUncompiled(_ c: ComponentID, at date: Date, ready: Double, policy: CompilationPolicy) -> Bool {
        guard (strength(c, at: date) ?? 0) >= ready else { return false }
        let timed = traces(of: c).filter { $0.value.hasLatencyEvidence(policy) }
        guard !timed.isEmpty else { return false }
        return !timed.contains { $0.value.isCompiled(policy, modality: $0.key) }
    }

    /// The prerequisites of `pageComponents` (outside the page itself) that are known but not yet
    /// compiled. New material stacked on these costs working memory it need not; a heavy day is
    /// the wrong time to stack it. Weakest latency (slowest) first.
    public func uncompiledPrerequisites(of pageComponents: [ComponentID], in graph: KnowledgeGraph, at date: Date,
                                        ready: Double = FrontierPolicy().ready, policy: CompilationPolicy) -> [ComponentID] {
        let page = Set(pageComponents)
        var pre = Set<ComponentID>()
        for c in page { for p in graph.prerequisites(of: c) where !page.contains(p) { pre.insert(p) } }
        let slow = pre.filter { isKnownButUncompiled($0, at: date, ready: ready, policy: policy) }
        func latency(_ c: ComponentID) -> Double { traces(of: c).values.compactMap(\.latencyEMA).max() ?? 0 }
        return slow.sorted { latency($0) != latency($1) ? latency($0) > latency($1) : $0 < $1 }
    }
}
