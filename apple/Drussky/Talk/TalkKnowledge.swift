import Foundation
import LearningCore

extension Modality {
    /// Speaking one's own Russian in conversation, understood by the tutor and marked afterwards.
    static let speak: Modality = "own-speech"
}

/// Conversation assessment uses the final, structurally checked corrections. Receiving a
/// partner reply is not a correctness judgment. Typed turns keep the writing modality.
extension KnowledgeTracker {
    /// Events a finished conversation yields: turn credits not yet logged, the pen's edits, and
    /// the round's one main outcome (so a backfill knows the round is accounted for).
    func talkEvidence(page: BookPage, index: Int, before: ExerciseResult, after: ExerciseResult, at t: Date,
                      backfill: Bool = false, ignoringLogged: Bool = false) -> [LearningEvent] {
        guard page.exercises.indices.contains(index), page.exercises[index].kind == .talk else { return [] }
        guard !before.status.isComplete, after.status.isComplete else { return [] }
        let item = "\(page.id)#\(index)"
        let round = after.roundNumber
        let source = backfill ? "backfill" : "live"
        let mainID = Self.outcomeID(item: item, round: round)
        let logged: Set<String> = ignoringLogged ? [] : Set(model.events.map(\.id))
        guard after.status != .skipped, let talk = after.talk, !talk.learnerLines.isEmpty else {
            return after.status == .skipped ? [] : [LearningEvent(id: mainID, t: t, type: .outcome, source: source, item: item,
                                                                  activity: "talk", modality: .speak, components: [])]
        }
        let at = talk.marked ?? talk.ended ?? t
        var events: [LearningEvent] = []
        for (k, line) in talk.learnerLines.enumerated() where !line.ru.isEmpty {
            let modality: Modality = line.typed == true ? .produce : .speak
            guard let correction = line.correction, talk.isMarked,
                  correction.canGrade, correction.isConsistent(with: line.ru) else { continue }
            let untouched = correction.segments(of: line.ru).filter { $0.edit == nil }.map(\.text).joined(separator: " ")
            let right = domain.productionComponents(untouched)
            let creditID = "talk:\(item)#\(round):t\(k + 1)"
            if !logged.contains(creditID) {
                events.append(LearningEvent(id: creditID, t: at, type: .outcome, source: source, item: item,
                                            activity: "talk", modality: modality, components: right,
                                            grade: right.isEmpty ? nil : .good, response: line.ru))
            }
            let unclaimed = (k == 0 ? (talk.teach ?? []) : []).compactMap(\.pattern)
                .filter { p in !p.isEmpty && !talk.learnerLines.contains { $0.correction?.edits.contains { $0.pattern == p } ?? false } }
            for (i, edit) in correction.edits.enumerated() {
                let id = "talk:\(item)#\(round):t\(k + 1):e\(i)"
                guard !logged.contains(id) else { continue }
                let patterns = [edit.pattern].compactMap { $0 } + (i == 0 ? unclaimed : [])
                var seen = Set<ComponentID>()
                let components = (domain.productionComponents(edit.replacement) + patterns.map { ComponentID(.construction, $0) })
                    .filter { seen.insert($0).inserted }
                let confusions = line.typed == true
                    ? domain.diaryConfusions(wrote: edit.span, right: edit.replacement)
                    : domain.speechConfusions(said: edit.span, right: edit.replacement)
                events.append(LearningEvent(id: id, t: at, type: .outcome, source: source, item: item, activity: "talk",
                                            modality: modality, components: components, grade: .again,
                                            expected: edit.replacement, response: edit.span,
                                            confusions: confusions.isEmpty ? nil : confusions))
            }
        }
        if !logged.contains(mainID) {
            let assessed = talk.isMarked && talk.learnerLines.allSatisfy { line in
                guard let c = line.correction else { return false }
                return c.canGrade && c.isConsistent(with: line.ru)
            }
            let edits = talk.learnerLines.reduce(0) { $0 + ($1.correction?.edits.count ?? 0) }
            events.append(LearningEvent(id: mainID, t: at, type: .outcome, source: source, item: item, activity: "talk",
                                        modality: .speak, components: [], grade: assessed ? (edits == 0 ? .good : .hard) : nil,
                                        response: talk.learnerLines.map(\.ru).joined(separator: " / ")))
        }
        return events
    }
}

extension RussianDomain {
    /// What was wanted → what was said, on the speech channel (the diary's rule, another channel).
    func speechConfusions(said span: String, right replacement: String) -> [Confusion] {
        diaryConfusions(wrote: span, right: replacement).map { Confusion(target: $0.target, produced: $0.produced, channel: .speech) }
    }
}
