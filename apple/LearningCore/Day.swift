import Foundation

/// How a day of study is shaped: the first page of a new day, the return after a long absence,
/// how many reviews a day may ask for, and how the overflow waits.
public struct DayPolicy: Sendable, Equatable {
    /// Reviews placed on pages per calendar day; what is due beyond it waits, most urgent first.
    public var reviewCap: Int
    /// Of the day's allowance, how many the morning page itself carries (the rest weave into later pages).
    public var morningReviews: Int
    /// Easiest due items that open the morning page.
    public var warmUp: Int
    /// An absence at least this long opens with a welcome-back page instead of a morning page.
    public var welcomeBackAfter: TimeInterval
    /// How many due items the welcome-back page re-tests (a stratified sample, never everything).
    public var welcomeBackSample: Int
    /// Due items woven into an ordinary course page.
    public var weave: Int
    /// From this many due, the next page is a review page rather than a course page.
    public var reviewPageThreshold: Int

    public init(reviewCap: Int = 25, morningReviews: Int = 12, warmUp: Int = 3, welcomeBackAfter: TimeInterval = 7 * 86_400,
                welcomeBackSample: Int = 8, weave: Int = 3, reviewPageThreshold: Int = 10) {
        self.reviewCap = max(0, reviewCap)
        self.morningReviews = max(0, morningReviews)
        self.warmUp = max(0, warmUp)
        self.welcomeBackAfter = welcomeBackAfter
        self.welcomeBackSample = max(1, welcomeBackSample)
        self.weave = max(0, weave)
        self.reviewPageThreshold = max(1, reviewPageThreshold)
    }
}

/// What kind of page a new day opens with.
public enum DayShape: Sendable, Equatable {
    /// Not the first page of a day.
    case ordinary
    /// The first page on a new calendar day.
    case morning
    /// The first page after `days` away.
    case welcomeBack(days: Int)

    /// From the last thing the learner did and the time now. No history: an ordinary first page.
    public static func shape(lastActivity: Date?, now: Date, calendar: Calendar = .current, policy: DayPolicy = DayPolicy()) -> DayShape {
        guard let lastActivity, lastActivity <= now else { return .ordinary }
        guard calendar.startOfDay(for: lastActivity) < calendar.startOfDay(for: now) else { return .ordinary }
        let away = now.timeIntervalSince(lastActivity)
        if away >= policy.welcomeBackAfter {
            return .welcomeBack(days: calendar.dateComponents([.day], from: calendar.startOfDay(for: lastActivity),
                                                              to: calendar.startOfDay(for: now)).day ?? Int(away / 86_400))
        }
        return .morning
    }
}

/// The reviews a composed page carries, and the ones it leaves for later.
public struct DayComposition: Sendable, Equatable {
    /// The easiest due items (highest recall, no lapse), to open on.
    public var warmUp: [DueReview]
    /// The day's reviews, most urgent first.
    public var reviews: [DueReview]
    /// Due but deferred: most urgent first, so a later page or day takes them in order.
    public var deferred: [DueReview]

    public var placed: [DueReview] { warmUp + reviews }

    /// Most urgent first: relearning items, then the most forgotten (1 − R), then the most
    /// confused — `DueReview.priority`, which is already that sum.
    public static func overflowOrder(_ due: [DueReview]) -> [DueReview] {
        due.sorted { $0.priority != $1.priority ? $0.priority > $1.priority : $0.id < $1.id }
    }

    /// The morning page: `warmUp` easiest items, then the most urgent up to `allowance` (what is
    /// left of the day's cap) and the page's own limit. Everything else is deferred, most urgent first.
    public static func morning(due: [DueReview], allowance: Int, policy: DayPolicy = DayPolicy()) -> DayComposition {
        let room = max(0, min(allowance, policy.morningReviews))
        let ordered = overflowOrder(due)
        // Easiest: not relearning, lowest priority (highest recall). Only when there is room past them.
        let easiest = ordered.filter { !$0.relearning }.suffix(policy.warmUp).reversed()
        let warm = room > policy.warmUp && ordered.count > policy.warmUp ? Array(easiest) : []
        let warmIDs = Set(warm.map(\.id))
        let rest = ordered.filter { !warmIDs.contains($0.id) }
        let take = max(0, room - warm.count)
        return DayComposition(warmUp: warm, reviews: Array(rest.prefix(take)), deferred: Array(rest.dropFirst(take)))
    }

    /// After a long absence: a stratified sample across the urgency range (every k-th item of
    /// the ordered due list), so the page measures where memory stands instead of re-testing all of it.
    public static func welcomeBack(due: [DueReview], policy: DayPolicy = DayPolicy()) -> DayComposition {
        let ordered = overflowOrder(due)
        guard ordered.count > policy.welcomeBackSample else {
            return DayComposition(warmUp: [], reviews: ordered, deferred: [])
        }
        var sample: [DueReview] = []
        let n = policy.welcomeBackSample
        for i in 0..<n {
            // Spread the picks over the list, the first pick being the most urgent.
            let index = i * (ordered.count - 1) / max(1, n - 1)
            sample.append(ordered[index])
        }
        var seen = Set<String>()
        let picked = sample.filter { seen.insert($0.id).inserted }
        let pickedIDs = Set(picked.map(\.id))
        return DayComposition(warmUp: [], reviews: picked, deferred: ordered.filter { !pickedIDs.contains($0.id) })
    }

    /// An ordinary page during the day: up to `policy.weave` of the most urgent, within what is left of the cap.
    public static func weave(due: [DueReview], allowance: Int, policy: DayPolicy = DayPolicy()) -> DayComposition {
        let ordered = overflowOrder(due)
        let take = max(0, min(allowance, policy.weave))
        return DayComposition(warmUp: [], reviews: Array(ordered.prefix(take)), deferred: Array(ordered.dropFirst(take)))
    }
}

/// When to leave the course's order: pull forward to what the learner is ready for, or step back
/// to what the next page leans on.
public struct FrontierPolicy: Sendable, Equatable {
    /// A prerequisite is strong at this strength (best recall over modalities).
    public var ready: Double
    /// A prerequisite with evidence is weak below this strength (or while relearning).
    public var weak: Double
    /// This many weak prerequisites earn a remediation page before the course page.
    public var minWeak: Int
    /// At most this many items on a remediation page.
    public var maxRemediation: Int

    public init(ready: Double = 0.8, weak: Double = 0.6, minWeak: Int = 3, maxRemediation: Int = 6) {
        self.ready = ready
        self.weak = weak
        self.minWeak = max(1, minWeak)
        self.maxRemediation = max(1, maxRemediation)
    }
}

public enum FrontierDecision: Sendable, Equatable {
    /// Follow the course as it stands (reviews first when many are due).
    case proceed
    /// Many reviews are due, but the next course page introduces things whose prerequisites are
    /// all strong: take the course page (with reviews woven in) rather than a page of review.
    case preferCourse
    /// Several of the next page's prerequisites are weak: bring these back first, weakest first.
    case remediate([ComponentID])
}

extension KnowledgeModel {
    /// The prerequisites of `pageComponents` (outside the page itself) that the learner has met
    /// and is losing: strength below `policy.weak`, or relearning in some modality. Weakest first.
    public func weakPrerequisites(of pageComponents: [ComponentID], in graph: KnowledgeGraph, at date: Date,
                                  policy: FrontierPolicy = FrontierPolicy()) -> [ComponentID] {
        let page = Set(pageComponents)
        var pre = Set<ComponentID>()
        for c in page { for p in graph.prerequisites(of: c) where !page.contains(p) { pre.insert(p) } }
        let weak = pre.compactMap { p -> (ComponentID, Double)? in
            let traces = traces(of: p)
            guard !traces.isEmpty else { return nil }   // never met: not a lapse, the course will teach it
            let s = strength(p, at: date) ?? 0
            let relearning = traces.values.contains { $0.isRelearning }
            return s < policy.weak || relearning ? (p, relearning ? s - 1 : s) : nil
        }
        return weak.sorted { $0.1 != $1.1 ? $0.1 < $1.1 : $0.0 < $1.0 }.map(\.0)
    }

    /// The components of `pageComponents` not yet learned (strength < 0.5) whose prerequisites
    /// are all strong: the page's own frontier.
    public func readyNewComponents(of pageComponents: [ComponentID], in graph: KnowledgeGraph, at date: Date,
                                   policy: FrontierPolicy = FrontierPolicy()) -> [ComponentID] {
        pageComponents.filter { c in
            guard (strength(c, at: date) ?? 0) < 0.5 else { return false }
            let pre = graph.prerequisites(of: c)
            return !pre.isEmpty && pre.allSatisfy { (strength($0, at: date) ?? 0) >= policy.ready }
        }
    }

    /// Whether to follow the course, pull it forward, or step back, given the components the next
    /// course page introduces and how many reviews are due.
    public func frontierDecision(pageComponents: [ComponentID], in graph: KnowledgeGraph, dueCount: Int, at date: Date,
                                 policy: FrontierPolicy = FrontierPolicy(), day: DayPolicy = DayPolicy()) -> FrontierDecision {
        guard !pageComponents.isEmpty else { return .proceed }
        let weak = weakPrerequisites(of: pageComponents, in: graph, at: date, policy: policy)
        if weak.count >= policy.minWeak { return .remediate(Array(weak.prefix(policy.maxRemediation))) }
        if dueCount >= day.reviewPageThreshold {
            let fresh = pageComponents.filter { (strength($0, at: date) ?? 0) < 0.5 && !graph.prerequisites(of: $0).isEmpty }
            let ready = readyNewComponents(of: pageComponents, in: graph, at: date, policy: policy)
            if !fresh.isEmpty, ready.count == fresh.count { return .preferCourse }
        }
        return .proceed
    }
}
