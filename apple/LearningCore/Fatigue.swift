import Foundation

/// How a session is going, read from the event log: simple signals, each explained in words.
public struct FatiguePolicy: Sendable, Equatable {
    /// A pause longer than this starts a new session.
    public var sessionGap: TimeInterval
    /// Outcomes in the session before the answer-based signals are trusted.
    public var minItems: Int
    /// The last this many outcomes are "recent".
    public var window: Int
    /// Score at or above which the learner is tired.
    public var tired: Double
    /// Score at or below which (with enough items and no misses) the learner is sharp.
    public var sharp: Double

    public init(sessionGap: TimeInterval = 45 * 60, minItems: Int = 6, window: Int = 8, tired: Double = 0.55, sharp: Double = 0.15) {
        self.sessionGap = sessionGap
        self.minItems = max(1, minItems)
        self.window = max(2, window)
        self.tired = tired
        self.sharp = sharp
    }
}

public struct FatigueEstimate: Sendable, Equatable {
    public enum Level: String, Sendable { case sharp, steady, tired }

    public struct Signals: Sendable, Equatable {
        /// Median latency of the recent outcomes over that of the session's first ones (1 = no change).
        public var latencyRatio: Double?
        /// Share of the recent outcomes graded `again`.
        public var missRate: Double?
        /// Tries (attempts + the finishing outcome) per finished item, recently.
        public var triesPerItem: Double?
        /// Since the page (or the session) started.
        public var minutes: Double
        /// Graded outcomes in the session.
        public var items: Int
    }

    /// 0 fresh … 1 spent.
    public var score: Double
    public var level: Level
    public var signals: Signals
    /// Plain words for each signal that contributed.
    public var reasons: [String]
}

public enum Fatigue {
    /// The session is the run of events ending at the last one before `now`, with no gap longer
    /// than `policy.sessionGap`. Time on the page runs from `pageStarted` when given.
    ///
    /// Score = 0.3·slower + 0.3·missing + 0.2·retrying + 0.2·long, each part in 0…1:
    ///   slower   (latencyRatio − 1)          (twice as slow → 1)
    ///   missing  (missRate − 0.15) / 0.35     (half missed → 1)
    ///   retrying (triesPerItem − 1.2) / 1.3   (2.5 tries per item → 1)
    ///   long     (minutes − 15) / 25          (40 minutes → 1)
    /// With fewer than `policy.minItems` outcomes only `long` counts.
    public static func estimate(events: [LearningEvent], now: Date, pageStarted: Date? = nil,
                                policy: FatiguePolicy = FatiguePolicy()) -> FatigueEstimate {
        let session = sessionEvents(events, now: now, policy: policy)
        let outcomes = completions(in: session)
        let start = pageStarted ?? session.first?.t ?? now
        let minutes = max(0, now.timeIntervalSince(start) / 60)
        var signals = FatigueEstimate.Signals(latencyRatio: nil, missRate: nil, triesPerItem: nil, minutes: minutes, items: outcomes.count)
        var reasons: [String] = []
        func unit(_ x: Double) -> Double { min(1, max(0, x)) }

        let long = unit((minutes - 15) / 25)
        if long > 0.4 { reasons.append("\(Int(minutes.rounded())) minutes on this page") }
        var score = 0.2 * long

        if outcomes.count >= policy.minItems {
            let recent = Array(outcomes.suffix(policy.window))
            let misses = recent.filter { $0.grade == .again }.count
            let missRate = Double(misses) / Double(recent.count)
            signals.missRate = missRate
            let missing = unit((missRate - 0.15) / 0.35)
            if misses >= 2 && missing > 0 { reasons.append("\(misses) of the last \(recent.count) missed") }

            let recentItems = Set(recent.map(\.item))
            let tries = tries(in: session, on: recentItems, completions: outcomes)
            let perItem = Double(tries) / Double(recentItems.count)
            signals.triesPerItem = perItem
            let retrying = unit((perItem - 1.2) / 1.3)
            if retrying > 0.3 { reasons.append("several tries per answer") }

            var slower = 0.0
            let latencies = outcomes.compactMap(\.latency)
            if latencies.count >= policy.minItems {
                let third = max(2, latencies.count / 3)
                let early = median(Array(latencies.prefix(third))), late = median(Array(latencies.suffix(third)))
                if early > 0 {
                    let ratio = late / early
                    signals.latencyRatio = ratio
                    slower = unit(ratio - 1)
                    if ratio >= 1.5 { reasons.append("answers take \(ratio >= 1.95 ? "twice" : "half again") as long as when you started") }
                }
            }
            score += 0.3 * slower + 0.3 * missing + 0.2 * retrying
        }
        score = unit(score)
        let level: FatigueEstimate.Level
        if score >= policy.tired {
            level = .tired
        } else if outcomes.count >= policy.minItems, score <= policy.sharp, (signals.missRate ?? 1) <= 0.13, (signals.latencyRatio ?? 1) <= 1.1 {
            level = .sharp
        } else {
            level = .steady
        }
        return FatigueEstimate(score: score, level: level, signals: signals, reasons: reasons)
    }

    /// The events of the session that ends at the last event before `now` (if that was less than
    /// a session gap ago), in time order; events at the same instant keep their logged order.
    public static func sessionEvents(_ events: [LearningEvent], now: Date, policy: FatiguePolicy = FatiguePolicy()) -> [LearningEvent] {
        let past = events.enumerated().filter { $0.element.t <= now }
            .sorted { ($0.element.t, $0.offset) < ($1.element.t, $1.offset) }.map(\.element)
        guard let last = past.last, now.timeIntervalSince(last.t) < policy.sessionGap else { return [] }
        var start = past.count - 1
        while start > 0, past[start].t.timeIntervalSince(past[start - 1].t) < policy.sessionGap { start -= 1 }
        return Array(past[start...])
    }

    /// The learner's answers: one per completion. A domain may log several outcomes for one answer
    /// at one instant (a corrected line of free writing yields one per correction and then its
    /// own); those describe one answer, and the last logged carries its grade. Ungraded
    /// completions are not answers.
    public static func completions(in session: [LearningEvent]) -> [LearningEvent] {
        var out: [LearningEvent] = []
        for e in session where e.type == .outcome {
            if let last = out.last, last.item == e.item, last.t == e.t { out[out.count - 1] = e } else { out.append(e) }
        }
        return out.filter { $0.grade != nil }
    }

    /// Tries on `items`: their attempts, one per attempt number when numbered (a domain may log
    /// the same try twice, as the exercise's attempt and as an annotation of it), plus their completions.
    static func tries(in session: [LearningEvent], on items: Set<String>, completions: [LearningEvent]) -> Int {
        var numbered = Set<String>()
        var count = completions.filter { items.contains($0.item) }.count
        for e in session where e.type == .attempt && items.contains(e.item) {
            if let n = e.attempt { if numbered.insert("\(e.item)#\(n)").inserted { count += 1 } } else { count += 1 }
        }
        return count
    }

    static func median(_ xs: [Double]) -> Double {
        guard !xs.isEmpty else { return 0 }
        let s = xs.sorted()
        return s.count % 2 == 1 ? s[s.count / 2] : (s[s.count / 2 - 1] + s[s.count / 2]) / 2
    }
}

/// When in the day each modality goes well, from the log: first-try success by hour band.
public struct HourlyPreference: Sendable {
    /// Four bands: night (0–6), morning (6–12), afternoon (12–18), evening (18–24).
    public static func band(ofHour hour: Int) -> Int { min(3, max(0, hour / 6)) }
    public static let bandNames = ["night", "morning", "afternoon", "evening"]

    struct Key: Hashable { var band: Int; var modality: Modality }
    private var success: [Key: Int] = [:]
    private var trials: [Key: Int] = [:]

    public init(events: [LearningEvent], calendar: Calendar = .current) {
        for e in events where e.type == .outcome {
            guard let g = e.grade, let m = e.modality else { continue }
            let key = Key(band: Self.band(ofHour: calendar.component(.hour, from: e.t)), modality: m)
            trials[key, default: 0] += 1
            if g >= .good { success[key, default: 0] += 1 }
        }
    }

    /// Smoothed first-try success rate (one imagined success and failure) and the trials behind it.
    public func accuracy(_ m: Modality, hour: Int) -> (rate: Double, trials: Int) {
        let key = Key(band: Self.band(ofHour: hour), modality: m)
        let n = trials[key] ?? 0
        return (Double((success[key] ?? 0) + 1) / Double(n + 2), n)
    }

    /// `modalities` ordered best-first for this hour. Modalities with fewer than `minTrials` in
    /// the band keep their given order, after the ones with evidence.
    public func order(_ modalities: [Modality], hour: Int, minTrials: Int = 8) -> [Modality] {
        let indexed = modalities.enumerated().map { i, m in (i, m, accuracy(m, hour: hour)) }
        let known = indexed.filter { $0.2.trials >= minTrials }.sorted { $0.2.rate != $1.2.rate ? $0.2.rate > $1.2.rate : $0.0 < $1.0 }
        let rest = indexed.filter { $0.2.trials < minTrials }
        return (known + rest).map { $0.1 }
    }

    /// The modality that goes best in this hour band, when one clearly does (≥ `minTrials` and a
    /// rate at least `margin` above the next).
    public func standout(_ modalities: [Modality], hour: Int, minTrials: Int = 8, margin: Double = 0.1) -> Modality? {
        let ranked = modalities.map { ($0, accuracy($0, hour: hour)) }.filter { $0.1.trials >= minTrials }
            .sorted { $0.1.rate > $1.1.rate }
        guard let best = ranked.first else { return nil }
        if ranked.count == 1 { return best.0 }
        return best.1.rate - ranked[1].1.rate >= margin ? best.0 : nil
    }
}
