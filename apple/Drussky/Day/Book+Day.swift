import Foundation
import LearningCore

/// What the book keeps about the shape of its days: probes and their outcomes, the parameters
/// they taught it, the pages the desk shortened. Saved inside book.json (optional field `day`).
struct DayState: Codable, Sendable, Equatable {
    var probes = ProbeLedger()
    /// FSRS stability scale learned from probes (ParameterNudge); nil: the published defaults.
    var stabilityScale: Double? = nil
    var lastNudge: Date? = nil
    /// Start of the day the last morning or welcome-back page was composed, and what it was.
    var lastOpening: Date? = nil
    var lastOpeningKind: String? = nil
    /// Due reviews left waiting when that page was composed.
    var deferredAtOpening: Int? = nil
    /// Pages the desk shortened (page id → the line shown on the page), and when it last did.
    var shortened: [String: String]? = nil
    var lastShortened: Date? = nil

    /// In-session only.
    var fatigue: FatigueEstimate? = nil
    var sharpAt: Date? = nil

    enum CodingKeys: String, CodingKey {
        case probes, stabilityScale, lastNudge, lastOpening, lastOpeningKind, deferredAtOpening, shortened, lastShortened
    }

    init() {}
}

/// The day's shape, in Drussky's terms: when a page is composed for a new day or a return, how
/// many reviews a day asks for, when the course is left for what is ready or what is weak, which
/// exercises are retention probes, and what the desk does when the learner tires.
extension Book {
    static let morningUnit = "Новый день · New day"
    static let welcomeBackUnit = "С возвращением · Welcome back"
    static let remediationUnit = "Подготовка · Getting ready"

    var dayPolicy: DayPolicy { DayPolicy(reviewCap: settings.dailyReviewCap ?? 25) }
    var probePolicy: ProbePolicy { ProbePolicy(enabled: settings.probes ?? true) }

    /// The last thing the learner did: the newest page or answer.
    var lastActivity: Date? {
        var latest = pages.last?.created
        for page in pages.suffix(3) {
            for d in [page.finished].compactMap({ $0 }) + page.results.compactMap(\.updated) where latest.map({ d > $0 }) ?? true { latest = d }
        }
        if let e = knowledge.model.events.last?.t, latest.map({ e > $0 }) ?? true { latest = e }
        return latest
    }

    private var startOfToday: Date { Calendar.current.startOfDay(for: now()) }
    private func isToday(_ d: Date?) -> Bool { d.map { Calendar.current.isDate($0, inSameDayAs: now()) } ?? false }

    // MARK: Composed pages

    /// Appends the page a new day opens with (morning or welcome-back), or a remediation page the
    /// next course page needs. False: nothing special, the ordinary flow decides.
    func appendComposedPage() -> Bool {
        guard !pages.isEmpty else { return false }
        switch DayShape.shape(lastActivity: lastActivity, now: now(), policy: dayPolicy) {
        case .welcomeBack(let days): return appendWelcomeBack(days: days)
        case .morning: return appendMorningPage()
        case .ordinary: return appendRemediationIfNeeded()
        }
    }

    /// Reviews still allowed today: the daily cap less the reviews already placed on today's
    /// pages (probes aren't reviews and don't count).
    func reviewAllowance() -> Int {
        let probeItems = Set(day.probes.probes.compactMap(\.item))
        let today = startOfToday
        var placed = 0
        for page in pages.reversed() where page.created >= today {
            placed += page.exercises.indices.filter { page.exercises[$0].reviewKey != nil && !probeItems.contains("\(page.id)#\($0)") }.count
        }
        return max(0, dayPolicy.reviewCap - placed)
    }

    /// The first page of a new day: a short warm-up (the easiest due items), the day's reviews up
    /// to the cap, ordered by what goes well at this hour, the next course page, and a diary line.
    private func appendMorningPage() -> Bool {
        let next = nextCoursePage
        let taughtHere = Set(next?.page.exercises.compactMap { memoryKey(for: $0) } ?? [])
        let due = dueReviews().filter { !taughtHere.contains($0.id) }
        let comp = DayComposition.morning(due: due, allowance: reviewAllowance(), policy: dayPolicy)
        guard next != nil || !comp.placed.isEmpty else { return false }
        let hour = Calendar.current.component(.hour, from: now())
        let preferred = HourlyPreference(events: knowledge.model.events).order(knowledge.domain.modalities, hour: hour)
        func rank(_ r: DueReview) -> Int { r.modality.flatMap { preferred.firstIndex(of: $0) } ?? preferred.count }
        let reviews = comp.reviews.enumerated().sorted { rank($0.element) != rank($1.element) ? rank($0.element) < rank($1.element) : $0.offset < $1.offset }.map(\.element)
        var exercises: [Exercise] = []
        for (i, r) in (comp.warmUp + reviews).enumerated() {
            exercises.append(reviewExercise(for: r.id, modality: r.modality, variant: pages.count + i))
        }
        let back = comp.placed.count
        var intro = back == 0 ? nil : (back == 1 ? "A new day. One thing to bring back first, then on." : "A new day. \(back) things to bring back first, then on.")
        var sourceID: String?
        var unitTitle = Self.morningUnit, title = "Morning page"
        if let (unit, source) = next {
            courseCursor += 1
            exercises += source.exercises.map(faded)
            sourceID = source.id
            unitTitle = unit.title
            title = source.title
            intro = [intro, source.intro].compactMap { $0 }.joined(separator: "\n\n")
        }
        exercises += diaryLine()
        appendPage(sourceID: sourceID, unitTitle: unitTitle, title: title, intro: intro, isReview: sourceID == nil, exercises: exercises)
        noteOpening("morning", deferred: comp.deferred.count)
        return true
    }

    /// After a long absence: a stratified sample of what is due, to see where memory stands,
    /// rather than everything at once.
    private func appendWelcomeBack(days: Int) -> Bool {
        let due = dueReviews()
        guard !due.isEmpty else { return appendMorningPage() }
        let comp = DayComposition.welcomeBack(due: due, policy: dayPolicy)
        let exercises = comp.reviews.enumerated().map { reviewExercise(for: $1.id, modality: $1.modality, variant: pages.count + $0) }
        let waiting = comp.deferred.count
        let intro = "\(days) days away. A few of the things you knew, to see where they stand"
            + (waiting > 0 ? "; the rest come back over the next days, not all at once." : ".")
            + " Missing one costs nothing — it just comes back sooner."
        appendPage(sourceID: nil, unitTitle: Self.welcomeBackUnit, title: "Good to see you", intro: intro, isReview: true,
                   exercises: exercises + diaryLine())
        noteOpening("welcome-back", deferred: waiting)
        return true
    }

    /// Several things the next course page leans on are weak: a short page bringing them back
    /// (built locally from the review exercises) before the course goes on.
    private func appendRemediationIfNeeded() -> Bool {
        guard let (_, source) = nextCoursePage, pages.last?.unitTitle != Self.remediationUnit else { return false }
        guard case .remediate(let weak) = frontierDecision(for: source) else { return false }
        let keys = weak.compactMap(targetKey)
        guard keys.count >= FrontierPolicy().minWeak else { return false }
        let exercises = keys.enumerated().map { reviewExercise(for: $1, modality: nil, variant: pages.count + $0) }
        appendPage(sourceID: nil, unitTitle: Self.remediationUnit, title: "Before the next page",
                   intro: "The next page leans on these. Bring them back first; it will go easier.", isReview: true, exercises: exercises)
        return true
    }

    /// Many reviews are due, but the next page's new material is ready (its prerequisites strong):
    /// take the course page, with reviews woven in, instead of a page of review.
    func frontierPrefersCourse() -> Bool {
        guard let (_, source) = nextCoursePage else { return false }
        return frontierDecision(for: source) == .preferCourse
    }

    private func frontierDecision(for source: Course.Page) -> FrontierDecision {
        var seen = Set<ComponentID>()
        let components = source.exercises.flatMap { knowledge.domain.components(for: $0) }.filter { seen.insert($0).inserted }
        return knowledge.model.frontierDecision(pageComponents: components, in: knowledge.graph, dueCount: dueReviews().count,
                                                at: now(), day: dayPolicy)
    }

    /// The review target a component is recalled through, if the learner has met it.
    private func targetKey(_ id: ComponentID) -> String? {
        let key: String
        switch id.kind {
        case .letter: key = "letter:\(id.key)"
        case .lexeme, .form, .phrase: key = "word:\(id.key)"
        default: return nil
        }
        return targets[key] != nil ? key : nil
    }

    /// A free line at the end of a composed page, when the course knows the `diary` exercise kind.
    private func diaryLine() -> [Exercise] {
        guard let kind = ExerciseKind(rawValue: "diary") else { return [] }
        return [Exercise(kind: kind)]
    }

    private func noteOpening(_ kind: String, deferred: Int) {
        day.lastOpening = startOfToday
        day.lastOpeningKind = kind
        day.deferredAtOpening = deferred
    }

    /// While the learner is sharp (Fatigue, within the last 20 minutes): one word from the
    /// frontier — ready, not yet met — copied once, so it joins their words.
    func extraNewItem(excluding taught: Set<String>) -> [Exercise] {
        guard let at = day.sharpAt, now().timeIntervalSince(at) < 20 * 60, now() >= at else { return [] }
        for id in knowledge.model.frontier(knowledge.graph, at: now(), kinds: [.lexeme], limit: 12) {
            let key = "word:\(id.key)"
            guard targets[key] == nil, !taught.contains(key), let w = word(id.key) else { continue }
            var e = Exercise(kind: .copy)
            e.text = w.ru
            e.times = 1
            return [e]
        }
        return []
    }

    // MARK: Probes

    /// Probes whose time has come, as ordinary review exercises for the end of the page being
    /// made (at most three; one whose item the page already asks waits for the next page); each
    /// is marked placed on its item. Draws a new probe set first when one is due.
    func probeExercises(pageID: String, among exercises: [Exercise]) -> [Exercise] {
        guard probePolicy.enabled else { return [] }
        drawProbesIfDue()
        var onPage = Set(exercises.compactMap(\.reviewKey))
        var out: [Exercise] = []
        for p in day.probes.pending(at: now()) where out.count < 3 {
            guard targets[p.candidate] != nil, reviewModalities(for: p.candidate).contains(p.modality) else {
                day.probes.probes.removeAll { $0.id == p.id }   // no longer something to recall this way
                continue
            }
            guard onPage.insert(p.candidate).inserted else { continue }
            out.append(reviewExercise(for: p.candidate, modality: p.modality, variant: exercises.count + out.count))
            day.probes.place(p.id, on: "\(pageID)#\(exercises.count + out.count - 1)")
        }
        return out
    }

    /// Whether the exercise at `index` on `page` is a retention probe rather than a review.
    func isProbe(page: BookPage, index: Int) -> Bool {
        let item = "\(page.id)#\(index)"
        return day.probes.probes.contains { $0.item == item }
    }

    private func drawProbesIfDue() {
        guard day.probes.isDrawDue(at: now(), policy: probePolicy) else { return }
        let drawn = knowledge.model.drawProbes(reviewCandidates(), at: now(), policy: probePolicy,
                                               seed: UInt64(max(0, now().timeIntervalSince1970)))
        guard !drawn.isEmpty else { return }
        day.probes.add(drawn, at: now())
    }

    /// Predicted recall against observed outcomes so far (all probes answered).
    var calibration: Calibration { Calibration(observations: day.probes.observations) }

    func exportGraph() -> URL? {
        knowledge.exportGraph(calibration: day.probes.observations.isEmpty ? nil : calibration)
    }

    /// Re-folds memory with the stability scale the probes have taught (nothing when none).
    func applyLearnedParameters() {
        guard let k = day.stabilityScale, k != 1 else { return }
        knowledge.rebuild(fsrs: FSRS().scalingStability(by: k))
    }

    /// One bounded step at most once per probe period.
    private func nudgeParameters() {
        if let last = day.lastNudge, now().timeIntervalSince(last) < probePolicy.every { return }
        let current = day.stabilityScale ?? 1
        let k = ParameterNudge.scale(current: current, calibration: calibration, minObservations: probePolicy.minObservations)
        guard k != current else { return }
        day.stabilityScale = k
        day.lastNudge = now()
        applyLearnedParameters()
    }

    // MARK: The desk

    /// After every finished exercise: a probe's outcome is recorded against what was predicted,
    /// and the session is read for fatigue. Tired: this page shortens. Sharp: the next page may
    /// bring one more new word.
    func deskObserved(_ ref: ExerciseRef, exercise: Exercise, result: ExerciseResult) {
        guard result.status.isComplete else { return }
        if day.probes.probes.contains(where: { $0.item == ref.item && $0.answered == nil }) {
            let grade = RussianDomain.grade(exercise, result)
            var predicted: Double?
            if grade != nil, let key = exercise.reviewKey, let m = RussianDomain.modality(for: exercise.kind) {
                // Retrievability just before this very review, as the trace recorded it.
                predicted = knowledge.domain.reviewComponents(forTarget: key)
                    .compactMap { knowledge.model.trace($0, m)?.reviews.last?.recallBefore }.min()
            }
            day.probes.observe(item: ref.item, predicted: predicted, observed: grade.map(\.isSuccess), at: now())
            nudgeParameters()
        }
        guard let p = index(ofPage: ref.pageID) else { return }
        let pageID = pages[p].id
        let started = knowledge.model.events.first { $0.item.hasPrefix(pageID + "#") }?.t
        let estimate = Fatigue.estimate(events: knowledge.model.events, now: now(), pageStarted: started)
        day.fatigue = estimate
        switch estimate.level {
        case .tired where p == currentPage && day.shortened?[pageID] == nil: shorten(page: p, estimate)
        case .sharp: day.sharpAt = now()
        default: break
        }
    }

    /// The page shortens: of the reviews not yet begun, two stay as recognition (hear or read and
    /// pick) instead of recall, the rest are left for a fresher hour (skipped, never graded).
    private func shorten(page p: Int, _ estimate: FatigueEstimate) {
        var page = pages[p]
        let probeItems = Set(day.probes.probes.compactMap(\.item))
        let untouched = page.exercises.indices.filter { i in
            let r = page.results[i]
            return !r.status.isComplete && page.exercises[i].reviewKey != nil && r.attempts == 0 && (r.failedChecks ?? 0) == 0
                && r.recognized == nil && !probeItems.contains("\(page.id)#\(i)")
        }
        for i in untouched.prefix(2) {
            let ex = page.exercises[i]
            guard let key = ex.reviewKey, [.dictation, .translate, .fill, .readAloud].contains(ex.kind) else { continue }
            let can = reviewModalities(for: key)
            guard let m = [Modality.hearPick, .readPick].first(where: { can.contains($0) }) else { continue }
            page.exercises[i] = reviewExercise(for: key, modality: m, variant: i)
        }
        for i in untouched.dropFirst(2) {
            page.results[i].status = .skipped
            page.results[i].updated = now()
        }
        if page.finished == nil && page.results.allSatisfy({ $0.status.isComplete }) { page.finished = now() }
        replace(page: page, at: p)
        var line = "Shorter page today — you've been at it a while."
        if let why = estimate.reasons.first { line += " (\(why.prefix(1).capitalized + String(why.dropFirst())).)" }
        day.shortened = (day.shortened ?? [:]).merging([page.id: line]) { _, new in new }
        day.lastShortened = now()
    }

    /// One calm line for a page's footer, when the desk changed it.
    func deskLine(for pageIndex: Int) -> String? {
        guard pages.indices.contains(pageIndex) else { return nil }
        return day.shortened?[pages[pageIndex].id]
    }

    /// One line inside the cover about today, or nothing.
    var coverLine: String? {
        if isToday(day.lastShortened) { return "Shorter page today — you've been at it a while." }
        if isToday(day.lastOpening) {
            let waiting = day.deferredAtOpening ?? 0
            if day.lastOpeningKind == "welcome-back" {
                return waiting > 0 ? "Welcome back. Today re-tests a sample; \(waiting) more come back over the next days." : "Welcome back."
            }
            if waiting > 0 { return "\(waiting) reviews wait for tomorrow — today keeps to \(dayPolicy.reviewCap)." }
        }
        let hour = Calendar.current.component(.hour, from: now())
        let pref = HourlyPreference(events: knowledge.model.events)
        if let best = pref.standout(knowledge.domain.modalities.filter { $0 != .copy }, hour: hour) {
            return "\(HourlyPreference.bandNames[HourlyPreference.band(ofHour: hour)].capitalized)s suit your \(best.title) — it goes first on a new day's page."
        }
        return nil
    }
}
