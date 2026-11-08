import Foundation
import LearningCore

/// Corrections become knowledge: each red-pen edit is evidence in the production modality, a
/// confusion between what was written and the right form, and (when the tutor named one) a
/// grammar pattern exercised; the words written right are evidence that they can be produced.
extension KnowledgeTracker {
    /// Events a diary result yields when it finishes. Ids are deterministic (one main outcome
    /// per round, one per edit), so a backfill can never double what a live record logged.
    func diaryEvidence(page: BookPage, index: Int, before: ExerciseResult, after: ExerciseResult, at t: Date,
                       backfill: Bool = false) -> [LearningEvent] {
        guard page.exercises.indices.contains(index), page.exercises[index].kind == .diary else { return [] }
        guard !before.status.isComplete, after.status.isComplete else { return [] }
        let item = "\(page.id)#\(index)"
        let round = after.roundNumber
        let source = backfill ? "backfill" : "live"
        let mainID = Self.outcomeID(item: item, round: round)
        guard after.status != .skipped, let entry = after.diary else {
            // Finished without writing (skipped, or a page completed for them): no recall moved.
            return after.status == .skipped ? [] : [LearningEvent(id: mainID, t: t, type: .outcome, source: source, item: item,
                                                                  activity: "diary", modality: .produce, components: [])]
        }
        let at = entry.correction?.at ?? entry.submitted
        guard let correction = entry.correction else {
            // Kept without a correction: it was written, but nobody checked it.
            return [LearningEvent(id: mainID, t: at, type: .outcome, source: source, item: item, activity: "diary",
                                  modality: .produce, components: domain.productionComponents(entry.text), response: entry.text)]
        }
        var events: [LearningEvent] = []
        // A taught pattern no edit names itself rides on the first edit (the biggest slip).
        let unclaimed = correction.teach.compactMap(\.pattern).filter { p in !p.isEmpty && !correction.edits.contains { $0.pattern == p } }
        for (i, edit) in correction.edits.enumerated() {
            let patterns = [edit.pattern].compactMap { $0 } + (i == 0 ? unclaimed : [])
            var seen = Set<ComponentID>()
            let components = (domain.productionComponents(edit.replacement) + patterns.map { ComponentID(.construction, $0) })
                .filter { seen.insert($0).inserted }
            let confusions = domain.diaryConfusions(wrote: edit.span, right: edit.replacement)
            events.append(LearningEvent(id: "diary:\(item)#\(round):\(i)", t: at, type: .outcome, source: source, item: item,
                                        activity: "diary", modality: .produce, components: components, grade: .again,
                                        expected: edit.replacement, response: edit.span,
                                        confusions: confusions.isEmpty ? nil : confusions))
        }
        // Words produced right: everything in the corrected text that no edit touched.
        let fixed = Set(correction.edits.flatMap { RussianText.key($0.replacement).split(separator: " ").map(String.init) })
        let right = domain.productionComponents(correction.corrected, excluding: fixed)
        events.append(LearningEvent(id: mainID, t: at, type: .outcome, source: source, item: item, activity: "diary",
                                    modality: .produce, components: right, grade: right.isEmpty ? nil : .good,
                                    expected: correction.corrected, response: entry.text))
        return events
    }
}

extension RussianDomain {
    /// The word components a free text exercises: known words by their form or lemma, other
    /// words as lexemes of their own (the diary is where new words first appear).
    func productionComponents(_ text: String, excluding: Set<String> = []) -> [ComponentID] {
        var out: [ComponentID] = []
        for t in RussianText.key(text).split(separator: " ").map(String.init) where !excluding.contains(t) {
            guard !RussianText.letters(in: t).isEmpty else { continue }
            if words.contains(t) {
                let id = wordID(t)
                out.append(id)
                if id.kind == .form, let l = lemmas.lemma(t) { out.append(ComponentID(.lexeme, l)) }
            } else if let l = lemma(of: t) {
                out.append(ComponentID(.form, t))
                out.append(ComponentID(.lexeme, l))
            } else {
                out.append(ComponentID(.lexeme, t))
            }
        }
        var seen = Set<ComponentID>()
        return out.filter { seen.insert($0).inserted }
    }

    /// The component for something the learner wrote: a known word's node, else a form node
    /// named after the writing itself (so the confusion edge still says what was produced).
    func producedID(_ s: String) -> ComponentID? {
        let key = RussianText.key(s)
        guard !key.isEmpty else { return nil }
        if let id = utteranceID(key) { return id }
        return ComponentID(key.contains(" ") ? .phrase : .form, key)
    }

    /// What was wanted → what was written. Word for word when the span and replacement have the
    /// same number of words, else the whole span against the whole replacement.
    func diaryConfusions(wrote span: String, right replacement: String) -> [Confusion] {
        let a = RussianText.key(replacement).split(separator: " ").map(String.init)
        let b = RussianText.key(span).split(separator: " ").map(String.init)
        guard !a.isEmpty, !b.isEmpty else { return [] }
        if a.count == b.count {
            return zip(a, b).compactMap { x, y in
                guard x != y, let t = productionComponents(x).first, let p = producedID(y), t != p else { return nil }
                return Confusion(target: t, produced: p, channel: .writing)
            }
        }
        guard let t = productionComponents(replacement).first ?? producedID(replacement), let p = producedID(span), t != p else { return [] }
        return [Confusion(target: t, produced: p, channel: .writing)]
    }
}
