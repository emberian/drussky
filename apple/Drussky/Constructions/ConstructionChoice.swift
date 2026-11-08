import Foundation
import LearningCore
import SwiftUI

extension Modality { static let constructionChoice: Modality = "construction-choice" }

enum ConstructionChoice {
    static let key = "construction:noun-a-case"
    static let component = ComponentID(.construction, "noun-a-case")
    static func reservedForProbe(_ ex: Exercise) -> Bool {
        ex.text == "Где твоя́ ___?" || ex.text == "Он ви́дит ___."
    }

    /// The decision, not sentence copying, is the target. Each sentence is offered at most once.
    static let bank: [Exercise] = {
        var out: [Exercise] = []
        let nouns = [("book", ["кни́га", "кни́гу", "кни́ги"]),
                     ("newspaper", ["газе́та", "газе́ту", "газе́ты"]),
                     ("room", ["ко́мната", "ко́мнату", "ко́мнаты"])]
        for (noun, forms) in nouns {
            let frames: [(String, Int, String)] = [
                ("Э́та ___ моя́.", 0, "The noun is the subject: use the naming form, -а."),
                ("Где твоя́ ___?", 0, "The noun is the subject: use the naming form, -а."),
                ("Я ищу́ ___.", 1, "The thing being sought is the direct object: -а becomes -у."),
                ("Он ви́дит ___.", 1, "The thing being seen is the direct object: -а becomes -у."),
                ("У меня́ нет ___.", 2, "After нет, use the genitive: -и after г, otherwise -ы here."),
                ("Здесь нет ___.", 2, "After нет, use the genitive: -и after г, otherwise -ы here.")
            ]
            for (frame, answer, reason) in frames {
                var ex = Exercise(kind: .constructionChoice)
                ex.text = frame
                ex.options = forms
                ex.answerIndex = answer
                // The plural accusative looks like the singular genitive. State the intended
                // singular meaning so both книгу and книги cannot answer the same prompt.
                if frame == "Я ищу́ ___." { ex.en = "I am looking for the \(noun) (one)." }
                if frame == "Он ви́дит ___." { ex.en = "He sees the \(noun) (one)." }
                ex.hint = reason
                ex.skills = [component.key]
                // A stable identity for the sentence, independent of the options' display order.
                ex.word = frame.replacingOccurrences(of: "___", with: forms[answer])
                out.append(ex)
            }
        }
        return out
    }()
}

@MainActor
extension Book {
    var canOpenConstructionPractice: Bool {
        !pages.contains { $0.sourceID == ConstructionChoice.key }
            && knowledge.model.hasEvidence(ComponentID(.construction, "acc-u"))
            && knowledge.model.hasEvidence(ComponentID(.construction, "genitive"))
    }

    var freshConstructionChoices: [Exercise] {
        let used = Set(pages.flatMap { $0.exercises.filter { $0.kind == .constructionChoice }.compactMap(\.word) })
        return ConstructionChoice.bank.filter { ex in
            guard let sentence = ex.word, !used.contains(sentence), let noun = ex.options?.first else { return false }
            return targets["word:\(RussianText.key(noun))"] != nil
        }
    }

    /// `force` (QA) opens it whatever the notebook has met, from the whole bank.
    @discardableResult
    func openConstructionPractice(force: Bool = false) -> Bool {
        guard force || (canOpenConstructionPractice && freshConstructionChoices.count >= 3) else { return false }
        var intro = Exercise(kind: .explain)
        intro.text = "First follow one noun through three roles. Then choose an ending in new sentences: who or what, the direct object, or what is absent."
        // These explanations are text-only: no new spoken strings are requested.
        var examples = Exercise(kind: .explain)
        examples.text = "Кни́га здесь. — The book is here. Я чита́ю кни́гу. — I am reading the book. Здесь нет кни́ги. — There is no book here."
        let pool = force && freshConstructionChoices.count < 3 ? ConstructionChoice.bank : freshConstructionChoices
        let choices = Array(pool.filter { !ConstructionChoice.reservedForProbe($0) }.prefix(3))
        var diary = Exercise(kind: .diary)
        diary.prompt = "Something you have, something you need, and something you cannot find. Use your own words."
        insertWrittenPage(title: "Which ending fits?", intro: "Follow the noun’s job in the sentence.", unitTitle: "Construction practice",
                          exercises: [intro, examples] + choices + [diary], newWords: [], sourceID: ConstructionChoice.key)
        return true
    }
}

struct ConstructionChoiceExercise: View {
    let slot: ExerciseSlot
    var body: some View {
        let ex = slot.exercise
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: "Choose the form for one thing in this sentence.")
            Text(ex.text ?? "").russianFont(28).foregroundStyle(Theme.text)
            if let meaning = ex.en { Text(meaning).scaledFont(16).foregroundStyle(Theme.muted) }
            ChoiceGrid(options: ex.options ?? [], chosen: slot.result.chosen, correct: ex.answerIndex,
                       russian: true, showStress: true, locked: slot.result.status.isComplete) { chosen in
                guard !slot.result.status.isComplete else { return }
                slot.finish { result in
                    result.chosen = chosen
                    result.attempts += 1
                    result.status = chosen == ex.answerIndex ? .correct : .missed
                }
            }
            if slot.result.status.isComplete {
                Text(ex.word ?? "").russianFont(23)
                Text(ex.hint ?? "").scaledFont(16).foregroundStyle(Theme.muted)
            }
        }
    }
}
