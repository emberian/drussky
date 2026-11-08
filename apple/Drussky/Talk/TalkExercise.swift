import SwiftUI

/// Разгово́р: the conversation on the page. A scene to pick, then the dialogue as it happens
/// (the tutor's lines spoken and written, yours as heard), the one microphone button for your
/// turn, and afterwards the whole transcript with the red pen and a pronunciation tip or two.
struct TalkExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var pageVoice
    @Environment(SpeakingTurn.self) private var speech
    @Environment(\.horizontalSizeClass) private var sizeClass
    let slot: ExerciseSlot
    let isActive: Bool

    @State private var session: TalkSession?
    @State private var revealed: Set<String> = []
    @State private var typing = false
    @FocusState private var focused: Bool

    private var compact: Bool { sizeClass == .compact }
    private var conversation: TalkConversation? { session?.conversation ?? slot.result.talk }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            header
            if let session {
                content(session)
            }
        }
        .onAppear {
            if session == nil {
                let s = TalkSession(ref: slot.ref, book: slot.book, pageVoice: pageVoice,
                                    voice: QA.active ? SilentTutorVoice() : SystemTutorVoice(), analyzer: speech.analyzer)
                session = s
                s.appear(autoStart: isActive && !QA.active)
            }
        }
        .onDisappear { session?.cancel() }
    }

    // MARK: Header

    private var header: some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Text("Разгово́р").font(Theme.cursive(compact ? 24 : 28)).foregroundStyle(Theme.ink)
            if let scene = conversation?.scene {
                Text(Theme.stressed(scene.ru.isEmpty ? scene.title : scene.ru, show: book.settings.showStress))
                    .russianFont(compact ? 18 : 22, weight: .medium).foregroundStyle(Theme.text)
                if !scene.ru.isEmpty { Text(scene.title).scaledFont(14).foregroundStyle(Theme.muted) }
            } else {
                Instruction(text: "Talk with your tutor, in Russian, out loud.")
            }
        }
    }

    // MARK: Body by state

    @ViewBuilder private func content(_ s: TalkSession) -> some View {
        switch s.status {
        case .unavailable(let why):
            Label(why, systemImage: "wifi.slash").scaledFont(15).foregroundStyle(Theme.muted)
            if !slot.result.status.isComplete { HStack { skipButton(s) } }
        case .idle:
            HStack(spacing: 10) {
                PillButton(title: "Start", systemImage: "bubble.left.and.bubble.right", prominent: true) { s.propose() }
                skipButton(s)
            }
        case .proposing:
            waiting("Your tutor is thinking of a scene…")
        case .choosing:
            scenes(s)
        case .paused, .thinking, .speaking, .yourTurn, .notHeard, .marking, .done, .failed:
            transcript(s)
            liveControls(s)
        }
    }

    private func waiting(_ text: String) -> some View {
        HStack(spacing: 10) {
            ProgressView().tint(Theme.accent)
            Text(text).scaledFont(15).foregroundStyle(Theme.muted)
        }
        .frame(minHeight: 34)
    }

    private func skipButton(_ s: TalkSession) -> some View {
        PillButton(title: "Not today", systemImage: "arrow.down") { s.skip() }
            .accessibilityHint("Leaves the conversation for another day, ungraded")
    }

    // MARK: Scenes

    private func scenes(_ s: TalkSession) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Where shall we talk?").scaledFont(16, weight: .semibold).foregroundStyle(Theme.text)
            FlowRow(spacing: 10) {
                ForEach(s.conversation.proposed ?? []) { scene in
                    Button { s.choose(scene) } label: {
                        VStack(alignment: .leading, spacing: 3) {
                            Text(Theme.stressed(scene.ru.isEmpty ? scene.title : scene.ru, show: book.settings.showStress))
                                .russianFont(22, weight: .medium).foregroundStyle(Theme.text)
                            if !scene.ru.isEmpty { Text(scene.title).scaledFont(14).foregroundStyle(Theme.muted) }
                        }
                        .padding(.horizontal, 16).padding(.vertical, 12)
                        .frame(minWidth: 160, alignment: .leading)
                        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                        .overlay(RoundedRectangle(cornerRadius: 14).stroke(Theme.accent.opacity(0.35)))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Scene: \(scene.title)")
                }
            }
            HStack(spacing: 10) {
                PillButton(title: "Other scenes", systemImage: "arrow.counterclockwise") { s.propose() }
                skipButton(s)
            }
        }
    }

    // MARK: Transcript

    private func transcript(_ s: TalkSession) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            ForEach(Array(s.conversation.lines.enumerated()), id: \.element.id) { _, line in
                switch line.role {
                case .tutor: tutorLine(line, s)
                case .learner: learnerLine(line, s)
                }
            }
            if s.status == .speaking || s.status == .thinking {
                speakingLine(s)
            }
        }
        .frame(maxWidth: 680, alignment: .leading)
        .animation(.easeOut(duration: 0.2), value: s.conversation.lines.count)
    }

    /// The tutor's line: Russian as written, the speaker to hear it again, and the whisper
    /// (tap to reveal the English and the new words).
    private func tutorLine(_ line: TalkLine, _ s: TalkSession) -> some View {
        let shown = revealed.contains(line.id)
        return VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Button { s.replay(line) } label: {
                    Image(systemName: "speaker.wave.2.fill").font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(Theme.accent).frame(width: 30, height: 30)
                        .background(Circle().fill(Theme.accent.opacity(0.12)))
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Hear it again")
                Text(Theme.stressed(line.ru, show: book.settings.showStress)).russianFont(compact ? 22 : 26, weight: .medium)
                    .foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
                Button { toggle(line.id) } label: {
                    Image(systemName: shown ? "eye.fill" : "eye").font(.system(size: 14)).foregroundStyle(Theme.muted)
                        .frame(width: 30, height: 30)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(shown ? "Hide the English" : "Whisper the English")
            }
            if shown {
                VStack(alignment: .leading, spacing: 6) {
                    if let en = line.en, !en.isEmpty {
                        Text(en).scaledFont(15).italic().foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
                    }
                    if let words = line.newWords, !words.isEmpty {
                        FlowRow(spacing: 8) {
                            ForEach(words, id: \.ru) { w in PhraseChip(phrase: Tutor.Phrase(ru: w.ru, en: w.en)) }
                        }
                    }
                }
                .padding(.leading, 40)
                .transition(.opacity)
            }
        }
    }

    /// The line being said right now, sentence by sentence.
    private func speakingLine(_ s: TalkSession) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 10) {
            Image(systemName: s.status == .thinking ? "ellipsis" : "speaker.wave.3.fill")
                .font(.system(size: 14, weight: .semibold)).foregroundStyle(Theme.accent)
                .symbolEffect(.variableColor.iterative, isActive: true)
                .frame(width: 30, height: 30)
                .background(Circle().fill(Theme.accent.opacity(0.12)))
            if s.status == .thinking {
                Text("Your tutor is thinking…").scaledFont(15).foregroundStyle(Theme.muted)
            } else {
                Text(Theme.stressed(s.streaming, show: book.settings.showStress)).russianFont(compact ? 22 : 26, weight: .medium)
                    .foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
            }
        }
        .accessibilityLabel(s.status == .thinking ? "Your tutor is thinking" : "Your tutor is speaking")
    }

    /// Your line as heard, with the red pen once the conversation is marked, and the tip.
    private func learnerLine(_ line: TalkLine, _ s: TalkSession) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Text(line.typed == true ? "You typed:" : "You (as heard):").scaledFont(13, weight: .medium).foregroundStyle(Theme.muted)
                if line.correction == nil {
                    Text(line.ru.isEmpty ? "—" : line.ru).font(Theme.cursive(compact ? 26 : 30)).foregroundStyle(Theme.ink)
                        .fixedSize(horizontal: false, vertical: true)
                }
                if let file = line.take {
                    TakeButton(url: book.voiceURL(file: file), key: "\(line.id)-take")
                }
            }
            if let c = line.correction {
                // The red pen over the line as heard: marks above the words, the corrected line in red.
                RedPen(entry: DiaryEntry(prompt: s.conversation.scene?.title ?? "", text: line.ru, submitted: line.at),
                       correction: c, compact: true)
            }
            if let tip = line.tip, !tip.isEmpty {
                Label(tip, systemImage: "waveform").scaledFont(14).foregroundStyle(Theme.text.opacity(0.85))
                    .padding(.horizontal, 10).padding(.vertical, 6)
                    .background(RoundedRectangle(cornerRadius: 10).fill(Theme.card))
            }
        }
        .padding(.leading, 40)
    }

    private func toggle(_ id: String) {
        if revealed.contains(id) { revealed.remove(id) } else { revealed.insert(id) }
    }

    // MARK: Controls

    @ViewBuilder private func liveControls(_ s: TalkSession) -> some View {
        switch s.status {
        case .yourTurn:
            HStack(alignment: .center, spacing: 16) {
                TurnButton(phase: s.mic.phase, level: s.mic.level) { s.tap() }
                TurnStatus(phase: s.mic.phase, problem: s.mic.problem(for: s.key), hasTake: false, repeats: 0) { s.tap() }
            }
            HStack(spacing: 10) {
                PillButton(title: "Type instead", systemImage: "keyboard") { s.mic.cancel(); typing = true; focused = true }
                PillButton(title: "End the conversation", systemImage: "hand.wave") { s.endNow() }
            }
            if typing { typedField(s) }
        case .notHeard:
            Text(s.mic.problem(for: s.key) == .micDenied ? "Talking needs the microphone: Settings → Drussky → Microphone. You can type your lines instead."
                 : "I didn't catch Russian there.").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
            HStack(spacing: 10) {
                PillButton(title: "Say it again", systemImage: "mic", prominent: true) { s.again() }
                PillButton(title: "Type it", systemImage: "keyboard") { typing = true; focused = true }
                PillButton(title: "End the conversation", systemImage: "hand.wave") { s.endNow() }
            }
            if typing { typedField(s) }
        case .paused:
            HStack(spacing: 10) {
                PillButton(title: "Continue talking", systemImage: "bubble.left.and.bubble.right", prominent: true) { s.resume() }
                PillButton(title: "End the conversation", systemImage: "hand.wave") { s.endNow() }
            }
        case .marking:
            waiting("Your tutor is marking it…")
        case .done:
            afterwards(s)
        case .failed(let message):
            Text(message).scaledFont(14).foregroundStyle(Theme.redPen).fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 10) {
                if s.conversation.isEnded {
                    PillButton(title: "Mark it again", systemImage: "arrow.counterclockwise", prominent: true) { s.mark() }
                    PillButton(title: "Keep it unmarked", systemImage: "checkmark") { s.keepUnmarked() }
                } else {
                    PillButton(title: "Try again", systemImage: "arrow.counterclockwise", prominent: true) { s.resume() }
                    PillButton(title: "End the conversation", systemImage: "hand.wave") { s.endNow() }
                }
            }
        default:
            EmptyView()
        }
    }

    private func typedField(_ s: TalkSession) -> some View {
        @Bindable var s = s
        return HStack(spacing: 10) {
            TextField("Что вы ска́жете?", text: $s.typedDraft, axis: .vertical)
                .lineLimit(1...3)
                .font(Theme.russian(22))
                .focused($focused)
                .autocorrectionDisabled()
                .padding(10)
                .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
                .onSubmit { s.submitTyped(); typing = false }
                .accessibilityLabel("Your line")
            PillButton(title: "Say", systemImage: "paperplane", prominent: true) { s.submitTyped(); typing = false }
                .disabled(RussianText.letters(in: s.typedDraft).isEmpty)
        }
    }

    /// After the pen: a word of praise for the whole, the pattern behind the biggest slip.
    @ViewBuilder private func afterwards(_ s: TalkSession) -> some View {
        let c = s.conversation
        if let praise = c.praise, !praise.isEmpty {
            Text(praise).font(Theme.cursive(compact ? 20 : 24)).foregroundStyle(Theme.redPen).fixedSize(horizontal: false, vertical: true)
        }
        ForEach(Array((c.teach ?? []).enumerated()), id: \.offset) { _, t in
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
        if c.isEnded, !c.isMarked, slot.result.status == .done {
            Text("Kept as spoken, unmarked.").scaledFont(13).foregroundStyle(Theme.muted)
        }
        if slot.result.status == .skipped {
            Label("Not today — nothing said here.", systemImage: "minus.circle").scaledFont(14).foregroundStyle(Theme.muted)
        }
    }
}

/// Plays one of your takes back.
struct TakeButton: View {
    @Environment(Voice.self) private var voice
    let url: URL
    let key: String

    var body: some View {
        let playing = voice.speakingKey == key
        Button { if playing { voice.stop() } else { voice.playFile(url, key: key) } } label: {
            Image(systemName: playing ? "stop.fill" : "play.fill").font(.system(size: 12, weight: .bold))
                .foregroundStyle(Theme.ink).frame(width: 28, height: 28)
                .background(Circle().fill(Theme.ink.opacity(0.1)))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(playing ? "Stop" : "Hear your take")
    }
}

/// Settings: which voice reads the tutor's lines, and how to get a better one.
struct TalkVoiceSection: View {
    var body: some View {
        Section {
            LabeledContent("Tutor's voice", value: Voice.bestRussianVoice.map { "\($0.name) · \(RussianVoicePick.qualityName($0.quality))" } ?? "none installed")
        } header: {
            Text("Talk")
        } footer: {
            Text(RussianVoicePick.howToImprove)
        }
    }
}

/// "My Russian": every conversation held, newest first, with its marks.
struct TalkEntriesView: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        let records = book.talkRecords
        if !records.isEmpty {
            VStack(alignment: .leading, spacing: 14) {
                Text("Conversations").font(.system(size: 20, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                ForEach(records) { r in
                    VStack(alignment: .leading, spacing: 8) {
                        HStack(alignment: .firstTextBaseline) {
                            Text(PageView.russianDate(r.conversation.started ?? Date())).font(Theme.cursive(24)).foregroundStyle(Theme.ink)
                            Text(r.conversation.scene?.title ?? "").font(.system(size: 14)).italic().foregroundStyle(Theme.muted)
                            Spacer()
                            Button {
                                if let i = book.index(ofPage: r.ref.pageID) { book.goTo(i) }
                                dismiss()
                            } label: { Text("стр. \(r.pageNumber)").font(Theme.label(12)).foregroundStyle(Theme.accent) }
                            .buttonStyle(.plain)
                            .accessibilityLabel("Go to page \(r.pageNumber)")
                        }
                        ForEach(r.conversation.lines) { line in
                            HStack(alignment: .firstTextBaseline, spacing: 8) {
                                Text(line.role == .tutor ? "Tutor" : "You").font(Theme.label(11)).foregroundStyle(Theme.muted).frame(width: 40, alignment: .trailing)
                                if line.role == .learner, let c = line.correction {
                                    RedPen(entry: DiaryEntry(prompt: "", text: line.ru, submitted: line.at), correction: c, compact: true)
                                } else {
                                    Text(RussianText.unstressed(line.ru)).russianFont(18).foregroundStyle(Theme.text)
                                }
                            }
                        }
                        if let praise = r.conversation.praise, !praise.isEmpty {
                            Text(praise).font(Theme.cursive(20)).foregroundStyle(Theme.redPen)
                        }
                    }
                    .padding(16)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                }
            }
        }
    }
}
