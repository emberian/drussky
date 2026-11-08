import PencilKit
import SwiftUI

/// Дневник: a prompt and ruled lines for the learner's own Russian. The recognizer proposes a
/// transcript they confirm or fix, the tutor corrects it, and the red pen goes on the page
/// while their ink stays exactly as written.
struct DiaryExercise: View {
    @Environment(Book.self) private var book
    @Environment(\.horizontalSizeClass) private var sizeClass
    let slot: ExerciseSlot
    let isActive: Bool

    @StateObject private var pad: PadController
    @State private var reading = false
    @State private var draft: String?
    @State private var recognized: String?
    @State private var typing = false
    @State private var typed = ""
    @State private var sending = false
    @State private var error: String?
    @FocusState private var focused: Bool

    init(slot: ExerciseSlot, isActive: Bool) {
        self.slot = slot
        self.isActive = isActive
        _pad = StateObject(wrappedValue: PadController(fileURL: slot.inkURL, writer: slot.book.store.ink))
    }

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var compact: Bool { sizeClass == .compact }
    private var lines: Int { compact ? 2 : 3 }
    private var lineHeight: CGFloat { compact ? 76 : 92 }
    private var prompt: String { ex.prompt ?? "" }
    /// Written in Russian throughout (no Latin letters), so it is set like Russian text.
    private var promptIsRussian: Bool { !RussianText.letters(in: prompt).isEmpty && !prompt.contains { $0.isLetter && $0.isASCII } }
    private var entry: DiaryEntry? { result.diary }
    private var hasInk: Bool { !pad.isEmpty }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            header
            if entry?.typed != true || hasInk {
                DiaryStrip(controller: pad, lines: lines, lineHeight: lineHeight, fingerDrawing: book.settings.fingerDrawing,
                           locked: result.status.isComplete || entry != nil)
                    .frame(height: CGFloat(lines) * lineHeight)
                    .accessibilityIdentifier("diary-\(slot.index)")
            }
            if let entry {
                if let correction = entry.correction {
                    RedPen(entry: entry, correction: correction)
                } else if result.status.isComplete {
                    writtenLine(entry.text, note: "Kept as written, uncorrected.")
                } else {
                    writtenLine(entry.text, note: nil)
                    sendingOrRetry(entry)
                }
            } else if !result.status.isComplete {
                composer
            } else if result.status == .skipped {
                Label("Not today — nothing written here.", systemImage: "minus.circle").scaledFont(14).foregroundStyle(Theme.muted)
            }
        }
    }

    // MARK: Prompt

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            VStack(alignment: .leading, spacing: 4) {
                Text("Дневни́к").font(Theme.cursive(compact ? 24 : 28)).foregroundStyle(Theme.ink)
                Instruction(text: "Write a few words of your own Russian.")
            }
            // Never spoken (no recording exists for a prompt); Russian in it shows its stress.
            if promptIsRussian {
                Text(Theme.stressed(prompt, show: book.settings.showStress)).russianFont(compact ? 24 : 28, weight: .medium)
                    .foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
            } else {
                Text(Theme.stressed(prompt, show: book.settings.showStress))
                    .scaledFont(compact ? 20 : 24, weight: .semibold, design: .serif).foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    // MARK: Composing

    @ViewBuilder private var composer: some View {
        if typing {
            VStack(alignment: .leading, spacing: 8) {
                TextField("Напиши́те…", text: $typed, axis: .vertical)
                    .lineLimit(2...5)
                    .font(Theme.russian(24))
                    .focused($focused)
                    .autocorrectionDisabled()
                    .padding(12)
                    .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
                    .accessibilityLabel("Diary text")
                FlowRow(spacing: 10) {
                    PillButton(title: sending ? "Sending…" : "Send to my tutor", systemImage: "paperplane", prominent: true) {
                        send(text: typed, recognized: nil, typed: true)
                    }
                    .disabled(sending || RussianText.letters(in: typed).isEmpty)
                    PillButton(title: "Keep as written", systemImage: "checkmark") {
                        send(text: typed, recognized: nil, typed: true, correct: false)
                    }
                    .disabled(sending || RussianText.letters(in: typed).isEmpty)
                    PillButton(title: "Use the Pencil", systemImage: "pencil") { typing = false }
                    skipButton
                }
                if let error { Text(error).scaledFont(14).foregroundStyle(Theme.redPen) }
            }
        } else if let draft {
            proposal(draft)
        } else {
            FlowRow(spacing: 10) {
                if hasInk {
                    PillButton(title: reading ? "Reading…" : "Read my writing", systemImage: "text.viewfinder", prominent: true) { Task { await readInk() } }
                        .disabled(reading)
                    PillButton(title: "Undo", systemImage: "arrow.uturn.backward") { pad.undo() }
                    PillButton(title: "Clear", systemImage: "eraser") { pad.clear() }
                } else {
                    PillButton(title: "Type instead", systemImage: "keyboard") { typing = true; focused = true }
                }
                if isActive || hasInk { skipButton }
            }
            .frame(minHeight: 44)
        }
    }

    /// The recognizer's reading, editable: what they meant to write is what the tutor sees.
    private func proposal(_ text: String) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(text.isEmpty ? "I can't make that out yet. Fix it below, or write it again." : "That reads to me as:")
                .scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
            TextField("Что вы написа́ли?", text: Binding(get: { draft ?? "" }, set: { draft = $0 }), axis: .vertical)
                .lineLimit(1...4)
                .font(Theme.russian(24))
                .autocorrectionDisabled()
                .padding(12)
                .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
                .accessibilityLabel("What I read")
            Text("Fix anything I misread, then send it.").scaledFont(13).foregroundStyle(Theme.muted)
            FlowRow(spacing: 10) {
                PillButton(title: sending ? "Sending…" : "Send to my tutor", systemImage: "paperplane", prominent: true) {
                    send(text: draft ?? "", recognized: recognized, typed: false)
                }
                .disabled(sending || RussianText.letters(in: draft ?? "").isEmpty)
                PillButton(title: "Keep as written", systemImage: "checkmark") {
                    send(text: draft ?? "", recognized: recognized, typed: false, correct: false)
                }
                .disabled(sending || RussianText.letters(in: draft ?? "").isEmpty)
                PillButton(title: "Read it again", systemImage: "arrow.counterclockwise") { Task { await readInk() } }
                    .disabled(reading)
                PillButton(title: "Keep writing", systemImage: "pencil") { draft = nil }
            }
            if let error { Text(error).scaledFont(14).foregroundStyle(Theme.redPen) }
        }
    }

    private var skipButton: some View {
        PillButton(title: "Not today", systemImage: "arrow.down") { slot.finish { $0.status = .skipped } }
            .accessibilityHint("Leaves the diary line empty, ungraded")
    }

    private func writtenLine(_ text: String, note: String?) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            VStack(alignment: .leading, spacing: 4) {
                Text("You wrote:").scaledFont(14, weight: .medium).foregroundStyle(Theme.muted)
                Text(text).russianFont(22, weight: .medium).foregroundStyle(Theme.text)
            }
            if let note { Text(note).scaledFont(13).foregroundStyle(Theme.muted) }
        }
    }

    /// Sent, but no correction yet (the app closed, the network failed): try again or keep it.
    private func sendingOrRetry(_ entry: DiaryEntry) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            if sending {
                HStack(spacing: 10) { ProgressView(); Text("Your tutor is reading…").scaledFont(15).foregroundStyle(Theme.muted) }
            } else {
                if let error { Text(error).scaledFont(14).foregroundStyle(Theme.redPen) }
                HStack(spacing: 10) {
                    PillButton(title: "Send to my tutor", systemImage: "paperplane", prominent: true) { Task { await correct(entry) } }
                    PillButton(title: "Keep as written", systemImage: "checkmark") { book.keepDiaryUncorrected(slot.ref) }
                }
            }
        }
    }

    // MARK: Actions

    private func readInk() async {
        guard !reading, !pad.isEmpty else { return }
        reading = true
        let drawing = pad.drawing, revision = pad.revision
        let vocabulary = book.targets.keys.filter { $0.hasPrefix("word:") }.map { String($0.dropFirst(5)) }
        let text = await InkReader.transcribe(drawing, vocabulary: vocabulary) ?? ""
        reading = false
        guard pad.revision == revision, entry == nil else { return }
        recognized = text
        draft = text
    }

    private func send(text: String, recognized: String?, typed: Bool, correct: Bool = true) {
        let clean = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !clean.isEmpty else { return }
        error = nil
        let entry = DiaryEntry(prompt: prompt, text: clean, recognized: recognized == clean ? nil : recognized,
                               typed: typed ? true : nil, submitted: book.now())
        book.submitDiary(slot.ref, entry: entry)
        if correct { Task { await self.correct(entry) } }
        else { book.keepDiaryUncorrected(slot.ref) }
    }

    private func correct(_ entry: DiaryEntry) async {
        sending = true
        error = nil
        defer { sending = false }
        let ref = slot.ref, book = slot.book, round = result.roundNumber
        do {
            let correction = try await DiaryTutor.correct(text: entry.text, prompt: entry.prompt, book: book)
            guard let current = book.result(ref), !current.status.isComplete, current.roundNumber == round else { return }
            book.finishDiary(ref, correction: correction)
            draft = nil
        } catch {
            self.error = error.localizedDescription
        }
    }
}

// MARK: - The red pen

/// A teacher's marks on a line of homework: the learner's words with the wrong spans underlined
/// and the right form written above in red, the corrected line, a word of praise, and the
/// pattern behind the biggest slip. Tap a mark to see why and hear the right form.
struct RedPen: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let entry: DiaryEntry
    let correction: DiaryCorrection
    var compact = false
    @State private var selected: Int?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            marked
            if correction.verified == false {
                Label("The tutor's marks don't match this line as written, so it stays ungraded. Read it as a suggestion.", systemImage: "questionmark.circle")
                    .scaledFont(13).foregroundStyle(Theme.muted)
            }
            if !correction.edits.isEmpty {
                HStack(alignment: .firstTextBaseline, spacing: 12) {
                    Text(Theme.stressed(correction.corrected, show: book.settings.showStress))
                        .font(Theme.cursive(compact ? 28 : 36)).foregroundStyle(Theme.redPen)
                        .rotationEffect(.degrees(-1.5))
                        .fixedSize(horizontal: false, vertical: true)
                    HearButton(text: correction.corrected, size: 34)
                }
                .accessibilityLabel("Corrected: \(RussianText.unstressed(correction.corrected))")
            }
            if let i = selected, correction.edits.indices.contains(i) { why(correction.edits[i]) }
            let unplaced = correction.unplaced(in: entry.text)
            if !unplaced.isEmpty {
                VStack(alignment: .leading, spacing: 6) {
                    ForEach(unplaced, id: \.self) { i in
                        Button { select(i) } label: { editRow(correction.edits[i]) }.buttonStyle(.plain)
                    }
                }
            }
            if !correction.praise.isEmpty {
                Text(correction.praise).font(Theme.cursive(compact ? 20 : 24)).foregroundStyle(Theme.redPen)
                    .fixedSize(horizontal: false, vertical: true)
            }
            ForEach(Array(correction.teach.enumerated()), id: \.offset) { _, t in teachCard(t) }
        }
        .environment(\.openURL, OpenURLAction { url in
            guard url.scheme == "drussky-say",
                  let t = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems?.first(where: { $0.name == "t" })?.value
            else { return .systemAction }
            voice.play(t)
            return .handled
        })
        .animation(.easeOut(duration: 0.2), value: selected)
    }

    /// The learner's text, word by word; a marked span is one chip with its replacement above.
    private var marked: some View {
        let segments = correction.segments(of: entry.text)
        return FlowRow(spacing: 8) {
            ForEach(Array(segments.enumerated()), id: \.offset) { _, seg in
                if let i = seg.edit {
                    Button { select(i) } label: { mark(seg.text, edit: correction.edits[i], chosen: selected == i) }
                        .buttonStyle(.plain)
                        .accessibilityLabel("\(seg.text), corrected to \(RussianText.unstressed(correction.edits[i].replacement))")
                } else {
                    ForEach(Array(seg.text.split(separator: " ").enumerated()), id: \.offset) { _, w in
                        Text(String(w)).russianFont(compact ? 22 : 26).foregroundStyle(Theme.text)
                            .padding(.top, compact ? 20 : 26)
                    }
                }
            }
        }
        .accessibilityElement(children: .contain)
    }

    private func mark(_ span: String, edit: DiaryEdit, chosen: Bool) -> some View {
        VStack(alignment: .center, spacing: -4) {
            Text(Theme.stressed(edit.replacement.isEmpty ? "—" : edit.replacement, show: book.settings.showStress))
                .font(Theme.cursive(compact ? 20 : 26)).foregroundStyle(Theme.redPen)
                .rotationEffect(.degrees(-2))
                .fixedSize()
            Text(span).russianFont(compact ? 22 : 26).foregroundStyle(Theme.text)
                .overlay(alignment: .bottom) {
                    WavyUnderline().stroke(Theme.redPen, lineWidth: 1.6).frame(height: 5).offset(y: 3)
                }
        }
        .padding(.horizontal, 4)
        .background(RoundedRectangle(cornerRadius: 8).fill(chosen ? Theme.redPen.opacity(0.08) : .clear))
    }

    private func editRow(_ e: DiaryEdit) -> some View {
        HStack(spacing: 8) {
            Text(e.span).russianFont(20).foregroundStyle(Theme.text).strikethrough(true, color: Theme.redPen)
            Image(systemName: "arrow.right").font(.system(size: 13, weight: .bold)).foregroundStyle(Theme.redPen)
            Text(Theme.stressed(e.replacement.isEmpty ? "—" : e.replacement, show: book.settings.showStress)).font(Theme.cursive(24)).foregroundStyle(Theme.redPen)
        }
    }

    private func why(_ e: DiaryEdit) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 10) {
                Text(e.category.title.uppercased()).font(Theme.label(11)).tracking(1.2).foregroundStyle(Theme.redPen)
                    .padding(.horizontal, 8).padding(.vertical, 3)
                    .background(Capsule().fill(Theme.redPen.opacity(0.1)))
                if !e.replacement.isEmpty {
                    Text(Theme.stressed(e.replacement, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(Theme.text)
                    HearButton(text: e.replacement, size: 30)
                    if let en = e.en, !en.isEmpty { Text(en).scaledFont(15).foregroundStyle(Theme.muted) }
                }
            }
            if !e.why.isEmpty {
                Text(TutorSheet.markdown(Tutor.displayMarkdown(e.why))).scaledFont(15).foregroundStyle(Theme.text)
                    .tint(Theme.accent).fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(12)
        .frame(maxWidth: 640, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
        .overlay(alignment: .leading) { Rectangle().fill(Theme.redPen.opacity(0.6)).frame(width: 3).padding(.vertical, 10) }
        .transition(.opacity)
    }

    private func teachCard(_ t: DiaryTeach) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(t.title).scaledFont(16, weight: .semibold).foregroundStyle(Theme.text)
            if !t.note.isEmpty {
                Text(TutorSheet.markdown(Tutor.displayMarkdown(t.note))).scaledFont(15).foregroundStyle(Theme.text.opacity(0.9))
                    .tint(Theme.accent).fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(12)
        .frame(maxWidth: 640, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
        .overlay(alignment: .leading) { Rectangle().fill(Theme.accent.opacity(0.5)).frame(width: 3).padding(.vertical, 10) }
    }

    private func select(_ i: Int) {
        selected = selected == i ? nil : i
        if selected != nil, !correction.edits[i].replacement.isEmpty { voice.play(correction.edits[i].replacement) }
    }
}

/// A teacher's wavy red underline.
struct WavyUnderline: Shape {
    func path(in rect: CGRect) -> Path {
        var p = Path()
        let amp = rect.height / 2, step: CGFloat = 6
        p.move(to: CGPoint(x: rect.minX, y: rect.midY))
        var x = rect.minX
        var up = true
        while x < rect.maxX {
            let nx = min(x + step, rect.maxX)
            p.addQuadCurve(to: CGPoint(x: nx, y: rect.midY), control: CGPoint(x: (x + nx) / 2, y: rect.midY + (up ? -amp : amp)))
            x = nx
            up.toggle()
        }
        return p
    }
}
