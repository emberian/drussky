import LearningCore
import SwiftUI

/// After Check, in "judge my writing: myself": the model answer written out beside the learner's
/// ink (under it, never over it), and two calm buttons. The reader's opinion, when it has one,
/// pre-selects the likely verdict and says so in one small line.
struct SelfJudgementPanel: View {
    @Environment(Book.self) private var book
    let target: String
    let reader: SelfJudgement.ReaderOpinion?
    /// The reader is still looking.
    let reading: Bool
    let onVerdict: (SelfJudgement.Verdict) -> Void

    var body: some View {
        let likely: SelfJudgement.Verdict? = reader?.matched == true ? .right : (reader?.contradicts == true ? .notQuite : nil)
        ViewThatFits(in: .horizontal) {
            HStack(alignment: .center, spacing: 28) {
                model
                verdicts(likely)
            }
            VStack(alignment: .leading, spacing: 12) {
                model
                verdicts(likely)
            }
        }
        .padding(.vertical, 6)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("self-judgement")
    }

    private var model: some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Text("Model").font(Theme.label(12)).foregroundStyle(Theme.muted)
            if book.settings.cursiveModels {
                Text(RussianText.unstressed(target)).font(Theme.cursive(40)).foregroundStyle(Theme.text)
            } else {
                Text(Theme.stressed(target, show: book.settings.showStress)).russianFont(34, weight: .medium).foregroundStyle(Theme.text)
            }
            HearButton(text: target, size: 34)
        }
        .accessibilityLabel("Model answer \(RussianText.unstressed(target))")
    }

    private func verdicts(_ likely: SelfJudgement.Verdict?) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 10) {
                PillButton(title: "Right", systemImage: "checkmark", prominent: likely == .right, tint: Theme.good) { onVerdict(.right) }
                    .accessibilityIdentifier("verdict-right")
                PillButton(title: "Not quite", systemImage: "arrow.counterclockwise", prominent: likely == .notQuite, tint: Theme.redPen) { onVerdict(.notQuite) }
                    .accessibilityIdentifier("verdict-not-quite")
            }
            if let hint = SelfJudgement.hint(reader, reading: reading) {
                Text(hint).scaledFont(13).foregroundStyle(Theme.muted)
                    .transition(.opacity)
            }
        }
        .animation(.easeOut(duration: 0.2), value: reader)
    }
}

/// On a word's history: the last time the learner marked it right while the reader read it as
/// something else. An invitation, not a correction.
struct SecondLookNote: View {
    @Environment(Book.self) private var book
    let id: ComponentID

    var body: some View {
        if let e = book.knowledge.readerDisagreement(id) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Image(systemName: "eyeglasses").foregroundStyle(Theme.accent)
                Text("On \(e.t.formatted(.dateTime.month().day())) you marked it right while the reader saw «\(e.response ?? "")». Worth another look, nothing more.")
                    .scaledFont(14).foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
            }
            .padding(12)
            .background(RoundedRectangle(cornerRadius: 12).fill(Theme.accent.opacity(0.06)))
        }
    }
}

/// Settings: who judges recall writing.
struct JudgeSettingsSection: View {
    @Environment(Book.self) private var book

    var body: some View {
        let judge = Binding(get: { book.settings.writingJudge }, set: { book.settings.writingJudge = $0 })
        Section {
            Picker("Judge my writing", selection: judge) {
                ForEach(WritingJudge.allCases, id: \.self) { Text($0.title).tag($0) }
            }
        } footer: {
            Text("Myself: after you write from memory, Check shows the model answer beside your line and you say Right or Not quite; the reader looks too, hints, and its reading is kept as evidence. The reader: it confirms a right answer by itself, and I wrote it right overrides it. Tracing and copying are always ticked by the reader.")
        }
    }
}
