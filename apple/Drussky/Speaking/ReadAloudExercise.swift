import SwiftUI
import UIKit

// MARK: - Read aloud

/// Say it: one tap plays the model, gives a "your turn" cue and takes the voice hands-free; the
/// take is trimmed, kept on the page, and compared with the model sound by sound and syllable
/// by syllable.
struct ReadAloudExercise: View {
    @Environment(Voice.self) private var voice
    @Environment(SpeakingTurn.self) private var turn
    @Environment(Book.self) private var book
    let slot: ExerciseSlot
    let isActive: Bool
    @State private var report: PronunciationReport?
    @State private var noPairs = false
    @State private var starting = false
    /// Shadowing: the human reader's sentence cut from the narration, used as the model.
    @State private var narrationChunk: URL?
    private var modelURL: URL? { narrationChunk ?? voice.recordingURL(for: target) }

    private var key: String { "\(slot.page.id)-\(slot.index)" }
    private var target: String { slot.exercise.text ?? "" }
    private var recordingExists: Bool { slot.result.recording != nil && FileManager.default.fileExists(atPath: slot.recordingURL.path) }
    private var mine: Bool { turn.activeKey == key }
    private var analysing: Bool { mine && turn.phase == .analysing }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: slot.result.status.isComplete ? "Say it again whenever you like." : "Listen, then say it.")
                .task(id: slot.exercise.narration) {
                    if slot.exercise.narration != nil { narrationChunk = await Narration.chunkURL(for: slot.exercise) }
                }
            HStack(alignment: .firstTextBaseline, spacing: 14) {
                Text(SpokenText.marked(target, report: recordingExists ? report : nil, showStress: book.settings.showStress))
                    .russianFont(40, weight: .medium)
                    .foregroundStyle(Theme.text)
                    .accessibilityLabel(target)
                if slot.exercise.narration != nil {
                    NarrationClipButton(exercise: slot.exercise, owner: key, size: 40, label: "Reader")
                } else {
                    HearButton(text: target, size: 40)
                }
            }
            MeaningTag(text: target)
            HStack(alignment: .center, spacing: 16) {
                TurnButton(phase: mine ? turn.phase : .idle, level: mine ? turn.level : 0) { tap() }
                TurnStatus(phase: mine ? turn.phase : .idle, problem: turn.problem(for: key), hasTake: recordingExists,
                           repeats: book.settings.modelRepeats) {
                    _ = turn.tap(key)
                }
            }
            if turn.problem(for: key) == .micDenied {
                HStack(spacing: 10) {
                    PillButton(title: "Open Settings", systemImage: "gear") {
                        if let url = URL(string: UIApplication.openSettingsURLString) { UIApplication.shared.open(url) }
                    }
                    if !slot.result.status.isComplete { skipButton }
                }
            }
            if recordingExists, let report {
                SpeakingReportView(report: report, target: target, modelURL: modelURL,
                                   takeURL: slot.recordingURL, version: slot.result.attempts,
                                   practise: practise)
                if noPairs {
                    Text("Those pairs open up after a few more letters.").scaledFont(13).foregroundStyle(Theme.muted)
                }
            }
            if let heard = slot.result.recognized, slot.result.attempts > 0 {
                heardLine(heard)
            } else if slot.result.attempts > 0 && !slot.result.status.isComplete && !analysing {
                heardLine("")
            } else if !slot.result.status.isComplete && isActive && turn.problem(for: key) != .micDenied {
                // Always a way past it, graded or not: no microphone, a quiet room, not today.
                skipButton
            }
            if slot.result.status == .skipped {
                Label("Skipped for now — not graded.", systemImage: "minus.circle").scaledFont(14).foregroundStyle(Theme.muted)
            }
        }
        .task(id: "\(slot.result.recording ?? "")#\(slot.result.attempts)") { await loadReport() }
    }

    private var skipButton: some View {
        PillButton(title: "Skip for now", systemImage: "arrow.down") { turn.cancel(); slot.finish { $0.status = .skipped } }
            .accessibilityHint("Leaves this exercise ungraded")
    }

    @ViewBuilder private func heardLine(_ heard: String) -> some View {
        let ok = RussianText.matches(heard, target)
        VStack(alignment: .leading, spacing: 8) {
            if ok {
                Label("Heard «\(heard)».", systemImage: "checkmark").scaledFont(15, weight: .medium).foregroundStyle(Theme.good)
            } else {
                Text(heard.isEmpty
                     ? (turn.recognitionDenied ? "Speech recognition is off, so nothing checks this one. Compare yours with the model by ear."
                        : turn.lastTimedOut ? "The Russian listener is still getting ready. Compare yours with the model by ear this time." : "I didn't catch Russian there.")
                     : "I heard «\(heard)».")
                    .scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                if !slot.result.status.isComplete {
                    FlowRow(spacing: 10) {
                        PillButton(title: "Sounded right to me", systemImage: "hand.thumbsup") {
                            slot.finish { $0.status = .done; $0.selfAssessed = true }
                        }
                        skipButton
                    }
                }
            }
        }
    }

    private func tap() {
        if turn.tap(key) || starting { return }
        starting = true
        Task { await start(); starting = false }
    }

    /// Shadowing a narrated sentence waits for the reader's chunk: the model the learner hears
    /// is the one the take is compared with.
    private func start() async {
        if slot.exercise.narration != nil, narrationChunk == nil { narrationChunk = await Narration.chunkURL(for: slot.exercise) }
        let ref = slot.ref, book = slot.book, round = slot.result.roundNumber, target = self.target
        let request = SpeakingTurn.Request(key: key, target: target, takeURL: book.newTakeURL(ref),
                                           model: modelURL, repeats: book.settings.modelRepeats)
        turn.begin(request, voice: voice) { take in
            // Resolved by id and round: the page may have turned or the exercise been reopened.
            book.commitTake(ref, round: round, file: take.file.lastPathComponent, heard: take.heard, target: target,
                            recognitionAllowed: take.recognitionAllowed, timedOut: take.timedOut, onsetMs: take.onsetMs)
            if let r = take.report, let page = book.page(id: ref.pageID) {
                book.knowledge.heard(page: page, index: ref.index, take: take.file.lastPathComponent, report: r,
                                     attempt: book.result(ref)?.attempts ?? 0)
                book.fluency.noteTake(target: target, report: r, component: book.knowledge.domain.utteranceID(target))  // Fluency/
            }
        }
    }

    private func loadReport() async {
        guard recordingExists else { report = nil; return }
        let url = slot.recordingURL
        report = await turn.analyseEarlier(url, target: target, model: modelURL, heard: slot.result.recognized)
    }

    private func practise(_ contrast: String) {
        turn.cancel()
        noPairs = !book.insertSoundPairsPage(contrast: contrast, voices: { voice.pairVoices($0, exactStress: $1).count })
    }
}

// MARK: - The one button

/// Microphone when idle; while the model plays, a speaker; on "your turn" a ring goes out once;
/// while listening, a ring that breathes with the voice. Tap: start, skip the model, or stop.
struct TurnButton: View {
    let phase: SpeakingTurn.Phase
    let level: Float
    let action: () -> Void
    @State private var cueRing = false

    private var listening: Bool { phase == .waiting || phase == .hearing || phase == .cue }

    var body: some View {
        Button(action: action) {
            ZStack {
                if listening {
                    Circle().stroke(Theme.redPen.opacity(phase == .hearing ? 0.4 : 0.22), lineWidth: 5)
                        .frame(width: 64 + CGFloat(level) * 30, height: 64 + CGFloat(level) * 30)
                        .animation(.easeOut(duration: 0.08), value: level)
                }
                Circle()
                    .stroke(Theme.redPen.opacity(cueRing ? 0 : 0.6), lineWidth: cueRing ? 1 : 4)
                    .frame(width: cueRing ? 110 : 64, height: cueRing ? 110 : 64)
                Circle().fill(fill).frame(width: 64, height: 64)
                icon
            }
            .frame(width: 96, height: 96)
        }
        .buttonStyle(.plain)
        .disabled(phase == .analysing || phase == .preparing)
        .accessibilityLabel(label)
        .sensoryFeedback(.impact(weight: .light), trigger: phase) { _, new in new == .cue }
        .onChange(of: phase) { _, p in
            guard p == .cue else { return }
            cueRing = false
            withAnimation(.easeOut(duration: 0.7)) { cueRing = true }
        }
        .onAppear { cueRing = true }
    }

    private var fill: Color {
        switch phase {
        case .cue, .waiting, .hearing: Theme.redPen
        case .model: Theme.accent.opacity(0.75)
        default: Theme.accent
        }
    }

    @ViewBuilder private var icon: some View {
        switch phase {
        case .model:
            Image(systemName: "speaker.wave.2.fill").foregroundStyle(.white).font(.system(size: 22))
                .symbolEffect(.variableColor.iterative, isActive: true)
        case .cue, .waiting, .hearing:
            Image(systemName: "stop.fill").foregroundStyle(.white).font(.system(size: 20))
        case .analysing, .preparing:
            ProgressView().tint(.white)
        case .idle:
            Image(systemName: "mic.fill").foregroundStyle(.white).font(.system(size: 24))
        }
    }

    private var label: String {
        switch phase {
        case .model: "Skip the model"
        case .cue, .waiting, .hearing: "Stop"
        case .analysing, .preparing: "Checking"
        case .idle: "Listen, then say it"
        }
    }
}

/// One line beside the button: what is happening and what to do.
struct TurnStatus: View {
    let phase: SpeakingTurn.Phase
    let problem: SpeakingTurn.Problem?
    let hasTake: Bool
    let repeats: Int
    let skip: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            switch phase {
            case .model(let k, let n):
                Text(n > 1 ? "Listen (\(k) of \(n))" : "Listen").scaledFont(17, weight: .semibold).foregroundStyle(Theme.accent)
                Button("Skip to my turn", action: skip).font(Theme.label(13)).foregroundStyle(Theme.muted)
            case .cue, .waiting:
                Text("Your turn").scaledFont(17, weight: .semibold).foregroundStyle(Theme.redPen)
                Text("Say it — I'll stop when you do.").scaledFont(13).foregroundStyle(Theme.muted)
            case .hearing:
                Text("Listening…").scaledFont(17, weight: .semibold).foregroundStyle(Theme.redPen)
                Text("Tap to stop early.").scaledFont(13).foregroundStyle(Theme.muted)
            case .analysing:
                Text("Listening back…").scaledFont(17, weight: .semibold).foregroundStyle(Theme.muted)
            case .preparing:
                Text(" ").scaledFont(17)
            case .idle:
                idle
            }
        }
        .animation(.easeOut(duration: 0.15), value: phase)
    }

    @ViewBuilder private var idle: some View {
        switch problem {
        case .micDenied?:
            Label("Saying it needs the microphone: Settings → Drussky → Microphone.", systemImage: "mic.slash")
                .scaledFont(14).foregroundStyle(Theme.redPen)
        case .interrupted?:
            Text("Interrupted — tap to go again.").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
        case .nothingHeard?:
            Text("I didn't hear you — tap and say it a little louder.").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
        case .failed?:
            Text("The microphone didn't start — tap to try again.").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
        case nil:
            Text(hasTake ? "Again" : (repeats > 0 ? "Tap: hear it, then say it" : "Tap, then say it"))
                .scaledFont(15, weight: .medium).foregroundStyle(Theme.muted)
        }
    }
}

// MARK: - The target, marked

enum SpokenText {
    static let close = Color(Theme.dynamic(0xB26A00, 0xF2B45A))

    /// The target with the stressed vowel in blue (as everywhere) and each sound's verdict on
    /// its letters: came out as another or not at all in red, close in amber, right in green.
    /// A word-level report only marks the letters the recognizer heard as others, in amber with a
    /// dotted line (a hypothesis, not a verdict): it cannot vouch for the rest either.
    static func marked(_ text: String, report: PronunciationReport?, showStress: Bool) -> AttributedString {
        var verdicts: [Int: PhoneJudgement.Verdict] = [:]
        if let report {
            let rank: [PhoneJudgement.Verdict: Int] = [.good: 0, .close: 1, .extra: 1, .substituted: 2, .missing: 2]
            for s in report.sounds {
                guard let r = s.letterRange else { continue }
                for i in r {
                    if let current = verdicts[i], rank[current, default: 0] > rank[s.verdict, default: 0] { continue }
                    verdicts[i] = s.verdict
                }
            }
        }
        let wordLevel = report?.isWordLevel ?? true
        var out = AttributedString()
        for (i, ch) in text.enumerated() {
            let scalars = ch.unicodeScalars.filter { $0 != "\u{0301}" }
            guard !scalars.isEmpty else { continue }
            var piece = AttributedString(String(String.UnicodeScalarView(scalars)))
            let stressed = scalars.count != ch.unicodeScalars.count
            if showStress && stressed {
                piece.foregroundColor = Theme.accent
                piece.underlineStyle = .single
            }
            switch verdicts[i] {
            case .substituted? where wordLevel, .missing? where wordLevel:
                piece.foregroundColor = close
                piece.underlineStyle = Text.LineStyle(pattern: .dot, color: close)
            case .substituted?:
                piece.foregroundColor = Theme.redPen
                piece.underlineStyle = Text.LineStyle(pattern: .solid, color: Theme.redPen)
            case .missing?:
                piece.foregroundColor = Theme.redPen.opacity(0.55)
                piece.underlineStyle = Text.LineStyle(pattern: .dot, color: Theme.redPen)
            case .close?:
                piece.foregroundColor = close
            case .good? where !wordLevel:
                piece.foregroundColor = stressed && showStress ? Theme.accent : Theme.good
            default:
                break
            }
            out += piece
        }
        return out
    }
}
