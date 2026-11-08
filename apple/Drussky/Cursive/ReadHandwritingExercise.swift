import SwiftUI

/// Someone's handwriting to read: a word or short line in another hand, and either the word
/// printed on the ruled line (read by the recognizer) or picked among look-alikes.
struct ReadHandwritingExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool

    private var ex: Exercise { slot.exercise }
    private var text: String { ex.text ?? "" }

    var body: some View {
        let options = ex.options
        let sample = HandwritingSample.make(text: text, seed: ex.seed.map { UInt64(bitPattern: Int64($0)) } ?? HandwritingSample.seed(for: text, index: slot.index))
        VStack(alignment: .leading, spacing: 10) {
            Instruction(text: options != nil ? "Someone wrote this. Which word is it?" : "Someone wrote this. Print it on the line.")
            HandwritingSampleView(sample: sample)
                .accessibilityIdentifier("handwriting-\(slot.index)")
            if let options {
                choices(options)
            } else {
                WritingExercise(slot: slot, isActive: isActive)
            }
            if slot.result.status.isComplete, slot.result.status != .missed {
                HStack(spacing: 10) {
                    Text("It says").scaledFont(15, weight: .medium).foregroundStyle(Theme.muted)
                    Text(Theme.stressed(text, show: book.settings.showStress)).russianFont(24, weight: .medium).foregroundStyle(Theme.text)
                    HearButton(text: text, size: 32)
                    MeaningTag(text: text)
                }
            }
        }
    }

    @ViewBuilder private func choices(_ options: [String]) -> some View {
        let correct = options.firstIndex { RussianText.key($0) == RussianText.key(text) }
        ChoiceGrid(options: options, chosen: slot.result.chosen, correct: correct,
                   russian: true, showStress: false, locked: slot.result.status.isComplete) { i in
            let right = i == correct
            slot.finish { r in
                r.chosen = i
                r.attempts += 1
                r.status = right ? (r.attempts == 1 ? .correct : .done) : (r.attempts >= 2 ? .missed : .open)
            }
            if right { voice.play(text) }
        }
        if slot.result.status == .missed {
            ChoiceCorrection(slot: slot) {
                HStack(spacing: 10) {
                    Text("It says").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                    Text(Theme.stressed(text, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(Theme.good)
                    HearButton(text: text, size: 34)
                }
            }
        }
    }
}
