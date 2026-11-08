import Foundation
import LearningCore
import Observation

/// Drussky's knowledge model: the Russian domain, the evidence log in the notebook folder
/// (`events.jsonl`) and the model folded from it. Book feeds it every graded result.
@MainActor
@Observable
final class KnowledgeTracker {
    private(set) var domain: RussianDomain
    private(set) var model: KnowledgeModel
    private let log: EventLog
    private let course: Course
    private let lemmas: Lemmas
    private let pairs: SoundPairs
    private let morphology: Morphology
    private var lastActivity: [String: Date] = [:]
    private var keptCount: Int
    /// Events the log could not take yet (disk full, file busy); retried before the next append.
    private var unwritten: [LearningEvent] = []
    /// Set while some evidence is only in memory; cleared once it reaches events.jsonl.
    private(set) var logProblem: String?
    var now: () -> Date

    /// The memory groups drawn as growth series.
    static let growthGroups: [(title: String, modalities: Set<Modality>)] = [
        ("hear → write", [.hearWrite]), ("English → write", [.meaningWrite]), ("see → say", [.say]),
        ("recognize", [.hearPick, .readPick, .readCursive]),
    ]

    init(course: Course, root: URL, customWords: [Course.Word], now: @escaping () -> Date) {
        self.course = course
        self.lemmas = Lemmas.load()
        self.pairs = SoundPairs.load()
        self.morphology = Morphology.load()
        self.now = now
        keptCount = customWords.count
        domain = RussianDomain(course: course, lemmas: lemmas, pairs: pairs, customWords: customWords, morphology: morphology)
        log = EventLog(url: root.appendingPathComponent("events.jsonl"))
        model = KnowledgeModel(events: log.load())
    }

    var graph: KnowledgeGraph { domain.graph }
    var logURL: URL { log.url }

    /// Rebuilds the domain when the learner keeps new words, so they appear as components.
    func refresh(customWords: [Course.Word]) {
        guard customWords.count != keptCount else { return }
        keptCount = customWords.count
        domain = RussianDomain(course: course, lemmas: lemmas, pairs: pairs, customWords: customWords, morphology: morphology)
    }

    // MARK: Evidence

    func observe(page: BookPage, index: Int, before: ExerciseResult, after: ExerciseResult) {
        let t = now()
        let latency = lastActivity[page.id].map { t.timeIntervalSince($0) }.flatMap { $0 >= 0 && $0 < 600 ? $0 : nil }
        if page.exercises[safe: index]?.kind == .diary { record(diaryEvidence(page: page, index: index, before: before, after: after, at: t)); return }
        if page.exercises[safe: index]?.kind == .talk { record(talkEvidence(page: page, index: index, before: before, after: after, at: t)); return }
        guard let event = domain.evidence(for: RussianAttempt(page: page, index: index, before: before, after: after,
                                                             at: t, latency: latency)) else { return }
        lastActivity[page.id] = t
        record([event])
    }

    /// An analysed take: the sounds it got wrong, keyed by contrast (`RussianDomain.speechEvidence`).
    func heard(page: BookPage, index: Int, take: String, report: PronunciationReport, attempt: Int) {
        guard let event = domain.speechEvidence(page: page, index: index, take: take, report: report, attempt: attempt, at: now()),
              !model.events.contains(where: { $0.id == event.id }) else { return }
        record([event])
    }

    func askedTutor(page: BookPage, exercise index: Int?) {
        guard let index, page.exercises.indices.contains(index) else { return }
        let ex = page.exercises[index]
        record([LearningEvent(t: now(), type: .help, item: "\(page.id)#\(index)", activity: ex.kind.rawValue,
                              modality: RussianDomain.modality(for: ex.kind), components: domain.components(for: ex),
                              help: ["tutor"])])
    }

    /// Set by `close()`: the folder the log lives in is about to belong to another notebook.
    private(set) var isClosed = false

    /// Stops the log for good (before a restore replaces the folder); evidence that arrives late
    /// is dropped rather than written into the restored notebook.
    func close() { isClosed = true }

    func record(_ events: [LearningEvent]) {
        guard !isClosed else { return }
        append(events)
        for e in events { model.apply(e) }
    }

    private func append(_ events: [LearningEvent]) {
        guard !isClosed else { return }
        let batch = unwritten + events
        if log.append(batch) {
            unwritten = []
            logProblem = nil
        } else {
            unwritten = batch
            logProblem = "\(batch.count) answer(s) not yet written to events.jsonl"
        }
    }

    /// Tries again to write evidence the log refused earlier.
    func retryPendingEvents() {
        guard !unwritten.isEmpty else { return }
        append([])
    }

    /// The id of the outcome a completed result's round yields, live or reconstructed.
    nonisolated static func outcomeID(item: String, round: Int) -> String { "out:\(item)#\(round)" }

    /// Reconstructs the evidence a saved result should have left but the log lacks: the outcome of
    /// a finished round (notebooks from before the log existed, a completion whose append failed
    /// after earlier attempts were logged, a crash between saving the result and logging it), any
    /// of a diary line's per-edit outcomes a write stopped short of, and the speech evidence of a
    /// take whose report was saved but whose event was not (`takeReport` reads the sidecar beside
    /// a take and says when it was made). Every reconstructed event has the deterministic id the
    /// live one would have had, so each missing id is appended on its own and nothing doubles.
    /// Attempts never stand in for an outcome. Logs from before deterministic ids are read by
    /// counting their (random-id) outcomes: one per finished round.
    @discardableResult
    func backfill(pages: [BookPage], takeReport: (String) -> (report: PronunciationReport, at: Date)? = { _ in nil }) -> Int {
        guard !isClosed else { return 0 }
        let logged = model.items
        let ids = Set(model.events.map(\.id))
        var legacyOutcomes: [String: Int] = [:]
        for e in model.events where e.type == .outcome && !e.id.hasPrefix("out:") && !e.id.hasPrefix("diary:") && !e.id.hasPrefix("talk:") {
            legacyOutcomes[e.item, default: 0] += 1
        }
        var events: [LearningEvent] = []
        for page in pages {
            for (i, r) in page.results.enumerated() {
                let item = "\(page.id)#\(i)"
                if r.status.isComplete, legacyOutcomes[item, default: 0] <= r.roundNumber {
                    let at = r.updated ?? page.finished ?? page.created
                    var before = ExerciseResult()
                    before.attempts = 0
                    let expected: [LearningEvent]
                    if page.exercises[i].kind == .diary { expected = diaryEvidence(page: page, index: i, before: before, after: r, at: at, backfill: true) }
                    else if page.exercises[i].kind == .talk { expected = talkEvidence(page: page, index: i, before: before, after: r, at: at, backfill: true) }
                    else { expected = [domain.evidence(for: RussianAttempt(page: page, index: i, before: before, after: r, at: at, backfill: true))].compactMap { $0 } }
                    events += expected.filter { !ids.contains($0.id) }
                }
                // Takes analysed and kept, whose sounds never reached the log.
                if page.exercises[i].kind == .readAloud {
                    for (n, take) in ((r.takes ?? []) + [r.recording].compactMap { $0 }).enumerated() where !ids.contains("speech:\(take)") {
                        guard let (report, at) = takeReport(take),
                              let e = domain.speechEvidence(page: page, index: i, take: take, report: report, attempt: n + 1, at: at, source: "backfill") else { continue }
                        events.append(e)
                    }
                }
            }
            for note in page.notes ?? [] {
                guard let i = note.exercise, page.exercises.indices.contains(i), !ids.contains("bf-help:\(note.id)"),
                      !logged.contains("\(page.id)#\(i)") else { continue }
                let ex = page.exercises[i]
                events.append(LearningEvent(id: "bf-help:\(note.id)", t: note.date, type: .help, source: "backfill",
                                            item: "\(page.id)#\(i)", activity: ex.kind.rawValue,
                                            modality: RussianDomain.modality(for: ex.kind),
                                            components: domain.components(for: ex), help: ["tutor"]))
            }
        }
        guard !events.isEmpty else { return 0 }
        append(events)
        model = KnowledgeModel(events: model.events + events, fsrs: model.fsrs)
        return events.count
    }

    // MARK: Questions

    func strength(_ id: ComponentID) -> Double? { model.strength(id, at: now()) }

    /// Running-word coverage of a text by words currently retrievable (R ≥ 0.5 in some modality).
    func readability(of text: String) -> Double { model.coverage(domain.tokens(in: text), at: now()) }

    /// Words (lexemes and phrases) with any evidence.
    var practisedWords: [ComponentID] {
        graph.components.keys.filter { ($0.kind == .lexeme || $0.kind == .phrase) && model.hasEvidence($0) }.sorted()
    }

    func growth() -> (dates: [Date], series: [[Double]]) {
        let end = now()
        let dates = model.days(until: end)
        let words = graph.components.keys.filter { $0.kind == .lexeme || $0.kind == .phrase }
        return (dates, model.growth(of: Array(words), groups: Self.growthGroups.map(\.modalities), at: dates))
    }

    func label(_ id: ComponentID) -> String {
        if let c = graph[id] { return id.kind == .letter ? String(c.label.prefix(1)) : c.label }
        return id.key
    }

    /// A compact account of the model for the tutor and the page writer.
    func summary(limit: Int = 8) -> String {
        let t = now()
        guard !model.events.isEmpty else { return "Knowledge model: no evidence yet." }
        func r(_ x: Double) -> String { String(format: "%.2f", x) }
        var lines: [String] = []
        let weak = model.weakest(at: t, kinds: [.lexeme, .form, .phrase, .letter, .construction], limit: limit)
            .map { "\(label($0.0.component)) (\($0.0.modality.title), recall \(r($0.1)))" }
        if !weak.isEmpty { lines.append("Weakest memories now: " + weak.joined(separator: "; ") + ".") }
        let pairs = model.contrastPairs(at: t, limit: 6).map { "\(label($0.0)) ↔ \(label($0.1))" }
        if !pairs.isEmpty { lines.append("Confuses (practise side by side): " + pairs.joined(separator: ", ") + ".") }
        let gaps = model.modalityGaps(at: t, among: domain.modalities, kinds: [.lexeme, .form, .phrase], limit: 6)
            .map { "\(label($0.component)) (strong \($0.strong.title), weak \($0.weak.title))" }
        if !gaps.isEmpty { lines.append("Modality gaps: " + gaps.joined(separator: "; ") + ".") }
        let patterns = graph.components(of: .construction).compactMap { c in model.strength(c.id, at: t).map { (c.label, $0) } }
            .sorted { $0.1 < $1.1 }.prefix(4).map { "\($0.0) \(r($0.1))" }
        if !patterns.isEmpty { lines.append("Grammar patterns, weakest first: " + patterns.joined(separator: ", ") + ".") }
        let next = model.frontier(graph, at: t, kinds: [.construction, .phrase, .lexeme], limit: 6).map(label)
        if !next.isEmpty { lines.append("Ready to learn next (prerequisites strong): " + next.joined(separator: ", ") + ".") }
        return "Knowledge model (FSRS recall per skill and modality):\n" + lines.joined(separator: "\n")
    }

    /// Refolds the log with other FSRS parameters (learned from retention probes, Book+Day).
    func rebuild(fsrs: FSRS) { model = KnowledgeModel(events: model.events, fsrs: fsrs) }

    func exportGraph(calibration: Calibration? = nil) -> URL? {
        var doc = GraphExport(domain: domain.name, modalities: domain.modalities, graph: graph, model: model, at: now())
        doc.calibration = calibration
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("Drussky knowledge graph.json")
        do { try doc.json().write(to: url, options: .atomic); return url } catch { return nil }
    }
}
