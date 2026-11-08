import Foundation
import LearningCore

/// Who decides whether a recall answer written with the Pencil was right.
enum WritingJudge: String, Codable, Sendable, CaseIterable {
    /// The learner: after writing, Check shows the model answer beside the ink and they say
    /// Right or Not quite. The recognizer reads silently, hints, and is recorded as evidence.
    case myself
    /// The recognizer: it confirms a right answer by itself; "I wrote it right" overrides it.
    case reader

    var title: String {
        switch self {
        case .myself: "Myself"
        case .reader: "The reader"
        }
    }
}

extension Settings {
    /// `judge` is stored as a string so a value from a later build still decodes (as the default).
    var writingJudge: WritingJudge {
        get { judge.flatMap(WritingJudge.init(rawValue:)) ?? .myself }
        set { judge = newValue.rawValue }
    }
}

/// Self-judgement first, recognizer second (LearningCore/README.md, "Judging one's own writing").
enum SelfJudgement {
    enum Verdict: String, Codable, Sendable {
        case right
        case notQuite = "not-quite"
    }

    /// What the recognizer made of the ink, tied to the ink revision it read.
    struct ReaderOpinion: Equatable, Sendable {
        /// Its best reading; nil when it had nothing to read, "" when it could make nothing out.
        var reading: String?
        /// Whether the reading matched the answer; nil when there was no reading.
        var matched: Bool?
        var revision: Int

        /// A reading that says something: non-empty text that did not match.
        var contradicts: Bool { matched == false && !(reading ?? "").isEmpty }
    }

    /// The evidence a self-judged result yields: the grade FSRS gets, how sure the record is of
    /// it, and whether the learner and the reader disagreed.
    struct Assessment: Equatable, Sendable {
        var grade: Grade
        var confidence: Double
        var disagreement: Bool
    }

    /// The verdict decides success; the reader only modulates it:
    ///   right, reader agrees            good first time, else hard   confidence 1
    ///   right, reader had no reading    good first time, else hard   confidence 0.7
    ///   right, reader read otherwise    hard (one notch of doubt)    confidence 0.4, disagreement
    ///   not quite, reader agreed        again                        confidence 0.6, disagreement
    ///   not quite, reader had none      again                        confidence 0.8
    ///   not quite, reader read otherwise again                       confidence 1
    static func assess(verdict: Verdict, readerAgreed: Bool?, firstTry: Bool) -> Assessment {
        switch (verdict, readerAgreed) {
        case (.right, true?): Assessment(grade: firstTry ? .good : .hard, confidence: 1, disagreement: false)
        case (.right, nil): Assessment(grade: firstTry ? .good : .hard, confidence: 0.7, disagreement: false)
        case (.right, false?): Assessment(grade: .hard, confidence: 0.4, disagreement: true)
        case (.notQuite, true?): Assessment(grade: .again, confidence: 0.6, disagreement: true)
        case (.notQuite, nil): Assessment(grade: .again, confidence: 0.8, disagreement: false)
        case (.notQuite, false?): Assessment(grade: .again, confidence: 1, disagreement: false)
        }
    }

    /// The assessment of a result the learner judged, or nil when the reader (or nobody) did.
    static func assessment(_ r: ExerciseResult) -> Assessment? {
        guard let raw = r.judged, let verdict = Verdict(rawValue: raw) else { return nil }
        let firstTry = r.attempts <= 1 && (r.failedChecks ?? 0) == 0 && r.assisted != true
        return assess(verdict: verdict, readerAgreed: r.readerAgreed, firstTry: firstTry)
    }

    /// Who judged a finished recall result, for the evidence log.
    static func judge(_ r: ExerciseResult) -> String? {
        if r.judged != nil || r.selfAssessed == true { return "self" }
        return r.status == .correct ? "reader" : nil
    }

    /// How sure the evidence is of a finished recall result's grade.
    static func confidence(_ r: ExerciseResult) -> Double? {
        if let a = assessment(r) { return a.confidence }
        if r.selfAssessed == true { return 0.4 }  // "I wrote it right" over a reading that disagreed
        return nil
    }

    /// Records the learner's verdict. Right finishes the exercise (the page may move on); Not
    /// quite marks it missed with the answer shown, so the red version can be copied once.
    @MainActor
    static func record(_ verdict: Verdict, on slot: ExerciseSlot, reader: ReaderOpinion?) {
        let update: (inout ExerciseResult) -> Void = { r in
            r.attempts += 1
            r.judged = verdict.rawValue
            r.readerAgreed = reader?.matched
            r.recognized = reader?.reading
            switch verdict {
            case .right: r.status = .correct
            case .notQuite: r.status = .missed; r.revealed = true
            }
        }
        if verdict == .right { slot.finish(update) } else { slot.record(update) }
    }

    /// The calm line under a finished, self-judged answer.
    static func feedback(_ r: ExerciseResult) -> String? {
        guard r.status == .correct, let a = assessment(r) else { return nil }
        if a.disagreement { return "Right, by your judgement. The reader saw «\(r.recognized ?? "")» — worth another look later." }
        if r.readerAgreed == true { return r.attempts <= 1 && (r.failedChecks ?? 0) == 0 ? "Right first time — the reader agrees." : "Right — the reader agrees." }
        return "Right, by your judgement."
    }

    /// The hint beside the verdict buttons: what the reader thinks, never a ruling.
    static func hint(_ o: ReaderOpinion?, reading: Bool) -> String? {
        guard let o else { return reading ? "Reading…" : nil }
        if o.matched == true { return "The reader agrees." }
        if o.contradicts { return "The reader saw «\(o.reading ?? "")». You decide." }
        return "The reader couldn't make it out — your call."
    }
}

extension KnowledgeTracker {
    /// The latest self-judged success on a component that the reader contradicted: something
    /// worth another look, not a verdict.
    func readerDisagreement(_ id: ComponentID) -> LearningEvent? {
        model.history(of: id).last { $0.type == .outcome && $0.judge == "self" && ($0.grade?.isSuccess ?? false) && ($0.confidence ?? 1) < 0.5 }
    }

    /// Words with a standing disagreement: the last outcome on them was a contested success.
    func worthAnotherLook(limit: Int = 8) -> [ComponentID] {
        var out: [(ComponentID, Date)] = []
        for id in practisedWords {
            guard let last = model.history(of: id).last(where: { $0.type == .outcome }), let d = readerDisagreement(id), d.id == last.id else { continue }
            out.append((id, d.t))
        }
        return out.sorted { $0.1 > $1.1 }.prefix(limit).map(\.0)
    }
}
