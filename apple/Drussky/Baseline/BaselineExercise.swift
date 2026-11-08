import AVFoundation
import PencilKit
import SwiftUI

struct BaselineExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let task: BaselineTask
    @StateObject private var pad: PadController
    @State private var draft: BaselineResponse
    @State private var capture: TakeCapture?
    @State private var recording = false
    @State private var visible = false
    @State private var saving = false
    @State private var error: String?

    init(slot: ExerciseSlot, task: BaselineTask) {
        self.slot = slot
        self.task = task
        _pad = StateObject(wrappedValue: PadController(fileURL: slot.inkURL, writer: slot.book.store.ink))
        _draft = State(initialValue: slot.result.baseline ?? BaselineResponse(started: slot.book.now()))
    }

    private var responseLines: Int { task.kind == .listening ? 2 : 4 }
    private var sealed: Bool { slot.result.baseline?.sealed != nil }
    private var audioURL: URL? {
        if let text = task.text {  // a reserved message, in the voice the sample names (Model/Listening.swift)
            return task.speaker.flatMap { voice.recordingURL(for: text, voice: $0) } ?? voice.recordingURL(for: text)
        }
        return task.audio.flatMap { Bundle.main.url(forResource: "Audio", withExtension: nil)?.appendingPathComponent($0) }
            .flatMap { FileManager.default.fileExists(atPath: $0.path) ? $0 : nil }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(task.kind.rawValue.capitalized).font(Theme.label(13)).foregroundStyle(Theme.accent)
            Text(task.prompt).scaledFont(18, weight: .medium).fixedSize(horizontal: false, vertical: true)
            if let passage = task.passage {
                Text(Theme.stressed(passage, show: book.settings.showStress)).russianFont(23)
                    .lineSpacing(7).fixedSize(horizontal: false, vertical: true)
            }
            if task.kind == .listening {
                if let audioURL {
                    Button {
                        if !sealed {
                            if draft.plays == 0 { draft.started = book.now() }
                            draft.plays += 1
                        }
                        voice.playFile(audioURL, key: task.id)
                    } label: { Label("Listen to the message", systemImage: "speaker.wave.2") }
                } else {
                    Text("This message’s audio is unavailable. Leave it unanswered for now.").foregroundStyle(Theme.muted)
                }
            }
            ForEach(Array(task.questions.enumerated()), id: \.offset) { i, question in
                Text("\(i + 1). \(question)").scaledFont(16)
            }
            if task.kind == .speaking { microphone }
            if task.kind != .speaking {
                DiaryStrip(controller: pad, lines: responseLines, lineHeight: 76,
                           fingerDrawing: book.settings.fingerDrawing, locked: sealed)
                    .frame(height: CGFloat(responseLines) * 76)
                if sealed {
                    if let text = slot.result.baseline?.text, !text.isEmpty { Text(text).russianFont(21) }
                } else {
                    TextField("Type your first answer, or write above", text: $draft.text, axis: .vertical)
                        .lineLimit(2...7).autocorrectionDisabled().textInputAutocapitalization(.never)
                        .font(Theme.russian(21)).padding(10).background(Theme.card)
                }
            }
            if sealed { savedResponse }
            else {
                Toggle("I used help", isOn: $draft.helped)
                if task.kind == .reading || task.kind == .listening {
                    Toggle("I had seen or heard this material before", isOn: $draft.familiar)
                }
                if task.kind == .listening { Toggle("I recognized this speaker’s voice", isOn: $draft.familiarVoice) }
                if task.kind == .writing {
                    TextField("This week: roughly how much reading, listening and conversation?", text: Binding(
                        get: { draft.exposure ?? "" }, set: { draft.exposure = $0 }), axis: .vertical)
                        .lineLimit(2...4).padding(10).background(Theme.card)
                }
                Button(saving ? "Keeping your sample…" : "Keep my first response") { Task { await seal() } }
                    .buttonStyle(.borderedProminent)
                    .disabled(saving || recording || !hasResponse || (task.kind == .listening && draft.plays == 0))
                    .accessibilityIdentifier("seal-baseline-\(slot.index)")
            }
            if let error { Text(error).scaledFont(14).foregroundStyle(Theme.muted) }
        }
        .onChange(of: draft) { _, value in
            if !sealed { slot.record { $0.baseline = value } }
        }
        .onAppear { visible = true }
        .onDisappear { visible = false; capture?.stopNow() }
    }

    private var hasResponse: Bool {
        task.kind == .speaking ? slot.result.recording != nil : !draft.text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || !pad.isEmpty
    }

    private var savedResponse: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let response = slot.result.baseline {
                Text(response.helped ? "First response kept · with help." : "First response kept · no help reported.")
                    .scaledFont(14).foregroundStyle(Theme.muted)
                if response.familiar { Text("Material was familiar; keep this as practice, not an unseen sample.").scaledFont(14) }
                if task.kind == .listening {
                    Text("\(response.plays) listen(s) · \(task.speaker ?? "speaker") · \(response.familiarVoice ? "familiar voice" : "voice not recognized")")
                        .scaledFont(14).foregroundStyle(Theme.muted)
                }
                if let exposure = response.exposure, !exposure.isEmpty { Text(exposure).scaledFont(14) }
            }
            Text("Compare with these notes").font(Theme.label(13)).foregroundStyle(Theme.muted)
            ForEach(task.reference, id: \.self) { Text(Theme.stressed($0, show: book.settings.showStress)).scaledFont(17) }
            TextField("What came through? Give an example, or note a difficulty.", text: Binding(
                get: { slot.result.baseline?.reflection ?? "" },
                set: { value in slot.record { $0.baseline?.reflection = value } }), axis: .vertical)
                .lineLimit(2...5).padding(10).background(Theme.card)
        }
    }

    @ViewBuilder private var microphone: some View {
        if let file = slot.result.recording {
            Button { voice.playFile(book.voiceURL(file: file), key: file) } label: {
                Label("Listen to my first take", systemImage: "play.circle")
            }
        } else {
            Button(recording ? "Finish my take" : "Record my response") {
                if recording { capture?.stopNow() } else { Task { await recordVoice() } }
            }.buttonStyle(.bordered).disabled(saving)
            Text("Speak freely for up to a minute. Your first take stays here.").scaledFont(14).foregroundStyle(Theme.muted)
        }
    }

    private func seal() async {
        guard !sealed, hasResponse else { return }
        saving = true
        await pad.submitCurrent(fresh: true)
        let failures = await book.store.ink.flush()
        guard failures.isEmpty else { error = failures.joined(separator: "\n"); saving = false; return }
        if book.sealBaseline(slot.ref, response: draft) { book.requestAdvance(page: slot.ref.pageID) }
        saving = false
    }

    private func recordVoice() async {
        guard !recording, !sealed, slot.result.recording == nil else { return }
        recording = true
        if slot.result.baseline == nil { draft.started = book.now() }
        guard await AVAudioApplication.requestRecordPermission() else {
            error = "Allow microphone access to keep a spoken sample."; recording = false; return
        }
        guard visible else { recording = false; return }
        voice.stop()
        Voice.configureSession()
        var config = VoiceActivity.Config()
        config.maxSpeech = 60
        config.trailingSilence = 5
        let cap = TakeCapture(config: config)
        do {
            let stream = try cap.start()
            capture = cap
            cap.cueThenListen()
            var ending: VoiceActivity.Ending?
            for await event in stream {
                if case .ended(let value) = event { ending = value; break }
            }
            cap.shutdown()
            capture = nil
            guard let ending, ending.speech != nil else {
                error = "No speech was captured. Try again."; recording = false; return
            }
            let file = "\(slot.ref.pageID)-\(slot.index)-baseline-\(UUID().uuidString).m4a"
            let url = book.voiceURL(file: file)
            _ = try await Task.detached { try cap.write(ending, to: url) }.value
            slot.record { $0.recording = file }
            book.saveNow()
        } catch {
            capture?.shutdown(); capture = nil
            self.error = "The take could not be kept: \(error.localizedDescription)"
        }
        recording = false
    }
}
