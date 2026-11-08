import Foundation
import LearningCore
import Observation
import PencilKit

/// The notebook's fluency series (`metrics.jsonl`, append-only like events.jsonl) and the
/// moments Drussky measures: a word written from memory, a sentence said, a narrated sentence
/// written from hearing, a passage read with a finger.
@MainActor
@Observable
final class FluencyTracker {
    private(set) var store: MetricStore
    private let log: MetricLog
    private var unwritten: [MetricRecord] = []
    var now: () -> Date = Date.init

    init(root: URL) {
        log = MetricLog(url: root.appendingPathComponent("metrics.jsonl"))
        store = MetricStore(records: log.load())
    }

    var logURL: URL { log.url }

    func record(_ value: Double, metric: String, component: ComponentID, note: String? = nil, at t: Date? = nil) {
        let r = store.append(value, metric: metric, component: component, at: t ?? now(), note: note)
        let batch = unwritten + [r]
        unwritten = log.append(batch) ? [] : batch
    }

    // MARK: Moments

    /// A word (or short phrase) written from memory and finished: how fast the strokes came and how
    /// legibly they read. Trace and copy are practice, not measured.
    func noteWriting(exercise ex: Exercise, drawing: PKDrawing, matched: Bool, reading: String?, confidence: Double?,
                     component: ComponentID?) {
        guard [.dictation, .translate, .fill].contains(ex.kind), let target = ex.target else { return }
        let targets: [ComponentID] = [component, Fluency.writing].compactMap { $0 }
        if let sps = WritingFluency.strokesPerSecond(drawing) {
            for c in targets { record(sps, metric: Fluency.strokesPerSecond, component: c) }
        }
        let legible = Fluency.legibility(matched: matched, confidence: confidence, reading: reading, target: target)
        for c in targets { record(legible, metric: Fluency.legibility, component: c) }
    }

    /// A read-aloud take of three words or more: words a minute over its speech span.
    func noteTake(target: String, report: PronunciationReport, component: ComponentID?) {
        let words = Fluency.wordCount(target)
        guard words >= 3 else { return }
        let onsets = report.syllables.compactMap(\.start)
        let wpm = Fluency.wpm(words: words, onsets: onsets.isEmpty ? [report.speechStart ?? 0] : onsets, end: report.speechEnd)
            ?? report.speechStart.flatMap { s in report.speechEnd.flatMap { Fluency.wpm(words: words, seconds: $0 - s) } }
        guard let wpm else { return }
        for c in [component, Fluency.speaking].compactMap({ $0 }) { record(wpm, metric: Fluency.sayWPM, component: c) }
    }

    /// A passage read with a finger following the lines, `seconds` from first touch to the last lift.
    @discardableResult
    func noteReadingPass(text: String, seconds: Double) -> Double? {
        guard let wpm = Fluency.wpm(words: Fluency.wordCount(text), seconds: seconds) else { return nil }
        record(wpm, metric: Fluency.readWPM, component: Fluency.passageID(text))
        record(wpm, metric: Fluency.readWPM, component: Fluency.reading)
        return wpm
    }

    /// Book.record: a narrated sentence written from hearing was finished, right or not, at the
    /// rate the reader was playing.
    func observe(_ ex: Exercise, before: ExerciseResult, after: ExerciseResult, rate: Float) {
        guard ex.kind == .dictation, ex.narration != nil, !before.status.isComplete, after.status.isComplete,
              after.status != .skipped else { return }
        record(after.status == .correct ? 1 : 0, metric: Fluency.listening, component: Fluency.listeningSkill, note: Fluency.rateNote(rate))
    }

    /// The last reading-pass speed on a passage, and the one before it.
    func lastReads(of text: String) -> (last: Double?, previous: Double?) {
        let s = store.samples(Fluency.readWPM, of: Fluency.passageID(text))
        return (s.last?.value, s.count >= 2 ? s[s.count - 2].value : nil)
    }
}

/// Timing inside a PencilKit drawing.
enum WritingFluency {
    /// Strokes a second from the first stroke's start to the last stroke's end, pauses included
    /// (hesitation is part of fluency). Nil for an empty drawing or one with no time in it.
    static func strokesPerSecond(_ drawing: PKDrawing) -> Double? {
        let strokes = drawing.strokes
        guard let first = strokes.first else { return nil }
        var start = first.path.creationDate, end = start
        for s in strokes {
            let begin = s.path.creationDate
            let finish = begin.addingTimeInterval(s.path.last?.timeOffset ?? 0)
            start = min(start, begin)
            end = max(end, finish)
        }
        let seconds = end.timeIntervalSince(start)
        guard seconds >= 0.2 else { return nil }
        return Double(strokes.count) / seconds
    }
}
