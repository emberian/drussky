import Foundation
import LearningCore

/// A conservative vocabulary estimate, not a comprehension score. Copying, recognition,
/// inferred transfer and a lemma alone cannot demonstrate production of an inflected form.
enum ReadingEvidence {
    static let modalities: Set<Modality> = [.hearWrite, .meaningWrite, .say]
    static let recallThreshold = 0.85
    static let supportedThreshold = 0.95
    static let comfortableThreshold = 0.98

    static func knows(_ id: ComponentID, model: KnowledgeModel, at date: Date) -> Bool {
        guard id.kind == .lexeme || id.kind == .form || id.kind == .phrase else { return false }
        let directEvents = model.history(of: id).filter { event in
            if event.type == .help { return true }
            guard let expected = event.expected else { return false }
            let key = RussianText.key(expected)
            return id.kind == .phrase ? key == id.key : key.split(separator: " ").contains(Substring(id.key))
        }
        let direct = KnowledgeModel(events: directEvents, fsrs: model.fsrs)
        return modalities.contains { modality in
            guard let trace = direct.trace(id, modality), let last = trace.review(at: date),
                  last.grade == .good || last.grade == .easy else { return false }
            return trace.strength(at: date) >= recallThreshold
        }
    }

    static func knows(_ unit: Target.Unit, model: KnowledgeModel, at date: Date) -> Bool {
        unit.candidates.contains { knows($0, model: model, at: date) }
    }

    static func coverage(_ text: String, domain: RussianDomain, model: KnowledgeModel, at date: Date) -> Double {
        let tokens = domain.tokens(in: text)
        guard !tokens.isEmpty else { return 0 }
        let known = Set(Set(tokens.flatMap { $0 }).filter { knows($0, model: model, at: date) })
        return Double(tokens.filter { $0.contains(where: known.contains) }.count) / Double(tokens.count)
    }

    static func label(for fraction: Double) -> String {
        if fraction >= comfortableThreshold { return "98%+ practiced forms" }
        if fraction >= supportedThreshold { return "95%+ · try with support" }
        return "study with support"
    }
}
