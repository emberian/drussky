import Foundation
import LearningCore

extension Modality {
    /// Speaking one's own Russian in conversation, understood by the tutor and marked afterwards.
    static let speak: Modality = "own-speech"
}

/// Conversations become knowledge: each turn the tutor understood is `good` on the words said
/// (speaking modality); each red-pen edit is `again` on the right form with a speech confusion;
/// a taught pattern rides on the first edit. Ids are deterministic, so a live record and a
/// backfill can never double-count.
extension KnowledgeTracker {
    /// A turn the tutor answered: the learner produced these words and was understood.
    func talkUnderstood(page: BookPage, index: Int, round: Int, turn: Int, heard: String, at t: Date) {
        let item = "\(page.id)#\(index)"
        let event = Self.turnEvent(domain: domain, item: item, round: round, turn: turn, heard: heard, at: t, source: "live")
        guard let event, !model.events.contains(where: { $0.id == event.id }) else { return }
        record([event])
    }

    static func turnEvent(domain: RussianDomain, item: String, round: Int, turn: Int, heard: String, at t: Date, source: String) -> LearningEvent? {
        let components = domain.productionComponents(heard)
        guard !components.isEmpty else { return nil }
        return LearningEvent(id: "talk:\(item)#\(round):t\(turn)", t: t, type: .outcome, source: source, item: item,
                             activity: "talk", modality: .speak, components: components, grade: .good, response: heard)
    }

    /// Events a finished conversation yields: turn credits not yet logged, the pen's edits, and
    /// the round's one main outcome (so a backfill knows the round is accounted for).
    func talkEvidence(page: BookPage, index: Int, before: ExerciseResult, after: ExerciseResult, at t: Date,
                      backfill: Bool = false) -> [LearningEvent] {
        guard page.exercises.indices.contains(index), page.exercises[index].kind == .talk else { return [] }
        guard !before.status.isComplete, after.status.isComplete else { return [] }
        let item = "\(page.id)#\(index)"
        let round = after.roundNumber
        let source = backfill ? "backfill" : "live"
        let mainID = Self.outcomeID(item: item, round: round)
        let logged = Set(model.events.map(\.id))
        guard after.status != .skipped, let talk = after.talk, !talk.learnerLines.isEmpty else {
            return after.status == .skipped ? [] : [LearningEvent(id: mainID, t: t, type: .outcome, source: source, item: item,
                                                                  activity: "talk", modality: .speak, components: [])]
        }
        let at = talk.marked ?? talk.ended ?? t
        var events: [LearningEvent] = []
        for (k, line) in talk.learnerLines.enumerated() where !line.ru.isEmpty {
            if let e = Self.turnEvent(domain: domain, item: item, round: round, turn: k + 1, heard: line.ru, at: line.at, source: source),
               !logged.contains(e.id) { events.append(e) }
            guard let correction = line.correction else { continue }
            let unclaimed = (k == 0 ? (talk.teach ?? []) : []).compactMap(\.pattern)
                .filter { p in !p.isEmpty && !talk.learnerLines.contains { $0.correction?.edits.contains { $0.pattern == p } ?? false } }
            for (i, edit) in correction.edits.enumerated() {
                let id = "talk:\(item)#\(round):t\(k + 1):e\(i)"
                guard !logged.contains(id) else { continue }
                let patterns = [edit.pattern].compactMap { $0 } + (i == 0 ? unclaimed : [])
                var seen = Set<ComponentID>()
                let components = (domain.productionComponents(edit.replacement) + patterns.map { ComponentID(.construction, $0) })
                    .filter { seen.insert($0).inserted }
                let confusions = domain.speechConfusions(said: edit.span, right: edit.replacement)
                events.append(LearningEvent(id: id, t: at, type: .outcome, source: source, item: item, activity: "talk",
                                            modality: .speak, components: components, grade: .again,
                                            expected: edit.replacement, response: edit.span,
                                            confusions: confusions.isEmpty ? nil : confusions))
            }
        }
        if !logged.contains(mainID) {
            let edits = talk.learnerLines.reduce(0) { $0 + ($1.correction?.edits.count ?? 0) }
            events.append(LearningEvent(id: mainID, t: at, type: .outcome, source: source, item: item, activity: "talk",
                                        modality: .speak, components: [], grade: talk.isMarked ? (edits == 0 ? .good : .hard) : nil,
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
