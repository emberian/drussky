import Foundation
import LearningCore

/// Listening (Model/Listening.swift) as evidence: the learner shows the meaning of something heard
/// with an English choice, a noun, or a scene result, so the outcome lands under `hear-pick` on the
/// words and constructions of the heard turn — never on production. A miss blames the crucial span
/// only. Asking or telling (question_or_statement) is one skill of its own, `sound:intonation`.
extension RussianDomain {
    /// The yes/no question contour against the statement's (IK-3 against IK-1).
    static let intonationComponent = ComponentID(.sound, "intonation")

    static let intonation = Component(id: intonationComponent, label: "Asking or telling",
                                      gloss: "The same words as a statement (IK-1) or a yes/no question (IK-3)", unit: "Listening")

    /// The components a listening item exercises: the words of what was heard (with their
    /// constructions, as a dictation of the same text would be tagged). Given a result with a
    /// wrong decision in it, those of the deciding span(s) only: a miss blames the span, not the
    /// turn. An intonation trial is evidence of the intonation skill alone.
    func listeningComponents(for ex: Exercise, result: ExerciseResult?) -> [ComponentID] {
        guard let item = ex.listening else { return [] }
        if item.kind == .questionOrStatement { return [Self.intonationComponent] }
        var texts = [item.heard.joined(separator: " ")]
        if let result {
            let missed = ListeningItem.picks(result).enumerated().filter { $0.element != item.answer($0.offset) }.map { item.reveal($0.offset).span }
            if !missed.isEmpty { texts = missed }
        }
        var out: [ComponentID] = []
        for text in texts {
            var probe = Exercise(kind: .dictation)
            probe.text = text
            out += components(for: probe).filter { $0.kind != .letter }  // a heard word is not evidence of its letters
        }
        var seen = Set<ComponentID>()
        return out.filter { seen.insert($0).inserted }
    }

    /// pick_object: the noun picked for the noun asked, both course words.
    func listeningConfusions(_ ex: Exercise, response: String) -> [Confusion] {
        guard let item = ex.listening, item.kind == .pickObject, let answer = item.answer, let right = item.options?[safe: answer],
              RussianText.key(right) != RussianText.key(response),
              let a = utteranceID(right), let b = utteranceID(response), a != b else { return [] }
        return [Confusion(target: a, produced: b, channel: .choice)]
    }
}

extension ListeningItem {
    /// How a result stores the learner's choices: `chosen` for one decision; for a message, its
    /// questions' picks in order in `recognized` ("1,2"), committed together once all are made.
    static func picks(_ r: ExerciseResult) -> [Int?] {
        if let rec = r.recognized, rec.contains(",") || r.chosen == nil && !rec.isEmpty {
            return rec.split(separator: ",", omittingEmptySubsequences: false).map { Int($0) }
        }
        return [r.chosen]
    }

    static func encode(picks: [Int]) -> String { picks.map(String.init).joined(separator: ",") }

    /// The learner's answer as text, for the log and the tutor.
    static func response(_ ex: Exercise, _ r: ExerciseResult) -> String? {
        guard let item = ex.listening else { return nil }
        let chosen = picks(r).enumerated().compactMap { i, c in c.flatMap { item.options(i)[safe: $0] } }
        return chosen.isEmpty ? nil : chosen.joined(separator: " / ")
    }

    /// For the tutor: what was heard, what was asked, and the choices.
    static func describe(_ ex: Exercise, _ r: ExerciseResult) -> String {
        guard let item = ex.listening else { return "A listening card." }
        let heard = item.kind.isExchange ? (item.lines ?? []).map { "\($0.speaker): «\($0.ru)»" }.joined(separator: " ") : "«\(item.transcript)»"
        switch item.kind {
        case .message:
            let qs = (item.questions ?? []).map { "“\($0.prompt)” (\(($0.options ?? []).joined(separator: " / ")))" }.joined(separator: "; ")
            return "Hearing a voice message \(heard) and answering \(qs)."
        case .questionOrStatement:
            return "Hearing \(heard) and deciding whether it was a statement or a question."
        default:
            return "Hearing \(heard) and answering “\(item.question ?? "")” among: \((item.options ?? []).joined(separator: " / "))."
        }
    }
}
