import Foundation

/// Actions that finish after an await, committed by stable id and checked against the round
/// they began in, so late work can't land on the wrong exercise or a newer attempt.
extension Book {
    enum TakeOutcome: Equatable {
        /// The exercise is gone or moved to another round: only the recording was kept.
        case keptOnly
        case recorded
        case finished
    }

    /// Commits a read-aloud take recorded into `file` (already in voice/). The previous recording
    /// moves to `takes`; nothing is deleted.
    @discardableResult
    func commitTake(_ ref: ExerciseRef, round: Int, file: String, heard: String?, target: String,
                    recognitionAllowed: Bool, timedOut: Bool) -> TakeOutcome {
        guard let current = result(ref) else { return .keptOnly }
        guard current.roundNumber == round else {
            record(ref) { r in r.takes = (r.takes ?? []) + [file] }
            return .keptOnly
        }
        let ok = heard.map { RussianText.matches($0, target) } ?? false
        let wasComplete = current.status.isComplete
        record(ref) { r in
            r.attempts += 1
            if let old = r.recording { r.takes = (r.takes ?? []) + [old] }
            r.recording = file
            r.recognized = heard
            guard !r.status.isComplete else { return }
            if ok { r.status = r.attempts == 1 && (r.failedChecks ?? 0) == 0 ? .correct : .done }
            else if recognitionAllowed && !timedOut { r.failedChecks = (r.failedChecks ?? 0) + 1 }
        }
        if !wasComplete, result(ref)?.status.isComplete == true {
            requestAdvance(page: ref.pageID)
            return .finished
        }
        return .recorded
    }
}
