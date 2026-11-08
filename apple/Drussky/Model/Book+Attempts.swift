import Foundation

/// Actions that finish after an await, committed by stable id and checked against the round
/// they began in, so late work can't land on the wrong exercise or a newer attempt.
extension Book {
    enum TakeOutcome: Equatable {
        /// The exercise is gone or moved to another round: only the recording was kept.
        case keptOnly
        case recorded
        case finished
        /// The notebook was closed (it is being replaced): nothing was recorded.
        case closed
    }

    /// Commits a read-aloud take recorded into `file` (already in voice/). The previous recording
    /// moves to `takes`; nothing is deleted. `heard` is what word recognition made of the take
    /// (nil: it heard no Russian, or never ran); only words heard and wrong are a failed check —
    /// a recognizer that heard nothing says nothing about the pronunciation.
    @discardableResult
    func commitTake(_ ref: ExerciseRef, round: Int, file: String, heard: String?, target: String,
                    recognitionAllowed: Bool, timedOut: Bool) -> TakeOutcome {
        guard !isClosed else { return .closed }
        guard let current = result(ref) else { return .keptOnly }
        guard current.roundNumber == round else {
            record(ref) { r in r.takes = (r.takes ?? []) + [file] }
            return .keptOnly
        }
        let ok = heard.map { RussianText.matches($0, target) } ?? false
        let heardWords = heard?.isEmpty == false
        let wasComplete = current.status.isComplete
        record(ref) { r in
            r.attempts += 1
            if let old = r.recording { r.takes = (r.takes ?? []) + [old] }
            r.recording = file
            r.recognized = heard
            guard !r.status.isComplete else { return }
            if ok { r.status = r.attempts == 1 && (r.failedChecks ?? 0) == 0 ? .correct : .done }
            else if heardWords && recognitionAllowed && !timedOut { r.failedChecks = (r.failedChecks ?? 0) + 1 }
        }
        // The take is already a file in voice/: its place in the result is saved now, not after the
        // debounce, so the next opening finds it (and can reconstruct its evidence from the report).
        saveNow()
        if !wasComplete, result(ref)?.status.isComplete == true {
            return .finished
        }
        return .recorded
    }
}
