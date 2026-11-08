import Foundation
import LearningCore
import SwiftUI

extension Modality { static let constructionChoice: Modality = "construction-choice" }

enum ConstructionChoice {
    static let key = "construction:noun-a-case"
    static let component = ComponentID(.construction, "noun-a-case")

    struct Item: Codable, Sendable, Hashable {
        var text: String
        var options: [String]
        var answer: Int
        var en: String? = nil
        var why: String
        var word: String
        var reserve: Bool = false

        var exercise: Exercise {
            var ex = Exercise(kind: .constructionChoice)
            ex.text = text
            ex.options = options
            ex.answerIndex = answer
            ex.en = en
            ex.hint = why
            ex.skills = [ConstructionChoice.component.key]
            ex.word = word
            return ex
        }
    }

    private struct File: Codable {
        var version: Int
        var intro: String
        var examples: String? = nil
        var diary_prompt: String? = nil
        var items: [Item]
    }

    private static let loaded: File = {
        guard let url = Bundle.main.url(forResource: "constructions", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let file = try? JSONDecoder().decode(File.self, from: data) else {
            return File(version: 1, intro: "", items: [])
        }
        return file
    }()

    static let intro: String = loaded.intro
    static let examples: String = loaded.examples ?? ""
    static let diaryPrompt: String = loaded.diary_prompt ?? ""
    static let items: [Item] = loaded.items

    /// The decision, not sentence copying, is the target. Each sentence is offered at most once.
    static let bank: [Exercise] = loaded.items.map(\.exercise)

    private static let reservedWords: Set<String> = Set(loaded.items.filter(\.reserve).map(\.word))
    private static let reservedTexts: Set<String> = Set(loaded.items.filter(\.reserve).map(\.text))

    static func reservedForProbe(_ ex: Exercise) -> Bool {
        if let word = ex.word, reservedWords.contains(word) { return true }
        if let text = ex.text, reservedTexts.contains(text) { return true }
        return false
    }
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
        intro.text = ConstructionChoice.intro
        // These explanations are text-only (no new spoken strings are requested), so the three
        // sentences are lines of one note: each Russian line with its English under it, never mixed.
        var examples = Exercise(kind: .explain)
        examples.text = ConstructionChoice.examples
        let pool = force && freshConstructionChoices.count < 3 ? ConstructionChoice.bank : freshConstructionChoices
        let choices = Array(pool.filter { !ConstructionChoice.reservedForProbe($0) }.prefix(3))
        var diary = Exercise(kind: .diary)
        diary.prompt = ConstructionChoice.diaryPrompt
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
            Text(Theme.stressed(ex.text ?? "", show: true)).russianFont(28).foregroundStyle(Theme.text)
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
                Text(Theme.stressed(ex.word ?? "", show: true)).russianFont(23)
                Text(ex.hint ?? "").scaledFont(16).foregroundStyle(Theme.muted)
            }
        }
    }
}
