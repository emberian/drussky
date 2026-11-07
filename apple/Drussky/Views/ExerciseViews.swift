import PencilKit
import SwiftUI

/// Everything an exercise view needs to read and record its own state.
@MainActor
struct ExerciseSlot {
    let book: Book
    let pageIndex: Int
    let index: Int

    var page: BookPage { book.pages[pageIndex] }
    var exercise: Exercise { page.exercises[index] }
    var result: ExerciseResult { page.results[index] }

    func record(_ update: (inout ExerciseResult) -> Void) { book.record(page: pageIndex, exercise: index, update) }
    var inkURL: URL { book.inkURL(page: page, exercise: index) }
    var recordingURL: URL { book.recordingURL(page: page, exercise: index) }
}

/// Routes an exercise to its view and draws the margin number / teacher's mark.
struct ExerciseRow: View {
    let slot: ExerciseSlot
    let isActive: Bool
    let marginWidth: CGFloat
    var onAsk: () -> Void = {}

    var body: some View {
        HStack(alignment: .top, spacing: 0) {
            VStack(spacing: 14) {
                MarginMark(number: slot.index + 1, status: slot.result.status, isActive: isActive, isReview: slot.exercise.reviewKey != nil)
                if slot.exercise.kind != .explain || isActive {
                    Button(action: onAsk) {
                        Image(systemName: "questionmark.bubble")
                            .font(.system(size: 17))
                            .foregroundStyle(Theme.muted.opacity(isActive ? 0.9 : 0.45))
                            .frame(width: 40, height: 40)
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Ask the tutor about exercise \(slot.index + 1)")
                }
            }
            .frame(width: marginWidth)
            .padding(.top, 4)
            VStack(alignment: .leading, spacing: 12) {
                content
            }
            .padding(.leading, 18)
            .padding(.trailing, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 18)
        .opacity(isActive || slot.result.status.isComplete ? 1 : 0.72)
        .sensoryFeedback(trigger: slot.result.status) { old, new in
            guard !old.isComplete else { return nil }
            switch new {
            case .correct: return .success
            case .done: return .impact(weight: .light)
            case .missed: return .warning
            case .open: return nil
            }
        }
        .animation(.easeOut(duration: 0.25), value: isActive)
    }

    @ViewBuilder private var content: some View {
        switch slot.exercise.kind {
        case .meet: MeetExercise(slot: slot, isActive: isActive)
        case .explain: ExplainExercise(slot: slot)
        case .trace, .copy: WritingExercise(slot: slot, isActive: isActive)
        case .dictation, .translate, .fill: WritingExercise(slot: slot, isActive: isActive)
        case .listenPick: ListenPickExercise(slot: slot, isActive: isActive)
        case .readAloud: ReadAloudExercise(slot: slot, isActive: isActive)
        case .readMeaning: ReadMeaningExercise(slot: slot, isActive: isActive)
        }
    }
}

struct MarginMark: View {
    let number: Int
    let status: ExerciseResult.Status
    let isActive: Bool
    let isReview: Bool

    var body: some View {
        ZStack {
            switch status {
            case .correct, .done:
                Image(systemName: "checkmark")
                    .font(.system(size: 22, weight: .heavy))
                    .foregroundStyle(Theme.redPen)
                    .rotationEffect(.degrees(-8))
                    .transition(.scale.combined(with: .opacity))
            case .missed:
                Text("\(number)")
                    .font(.system(size: 17, weight: .bold, design: .rounded))
                    .foregroundStyle(Theme.redPen)
                    .overlay(Circle().stroke(Theme.redPen, lineWidth: 1.6).frame(width: 30, height: 30))
            case .open:
                Text("\(number)")
                    .font(.system(size: 17, weight: .bold, design: .rounded))
                    .foregroundStyle(isActive ? Theme.accent : Theme.muted)
            }
        }
        .frame(width: 34, height: 34)
        .overlay(alignment: .bottom) {
            if isReview {
                Image(systemName: "arrow.counterclockwise")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundStyle(Theme.muted)
                    .offset(y: 12)
            }
        }
        .animation(.spring(response: 0.35, dampingFraction: 0.6), value: status)
        .accessibilityLabel("Exercise \(number)")
    }
}

/// One-line instruction in the tutor's voice.
struct Instruction: View {
    var text: String
    var body: some View {
        Text(text)
            .font(.system(size: 15, weight: .medium))
            .foregroundStyle(Theme.muted)
    }
}

/// A Russian phrase shown large, tappable to hear.
struct RussianLine: View {
    @Environment(Book.self) private var book
    var text: String
    var size: CGFloat = 34
    var body: some View {
        HStack(alignment: .center, spacing: 14) {
            Text(Theme.stressed(text, show: book.settings.showStress))
                .font(Theme.russian(size, weight: .medium))
                .foregroundStyle(Theme.text)
            HearButton(text: text, size: 40)
        }
    }
}

// MARK: - Meet a letter

struct MeetExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    @Environment(\.horizontalSizeClass) private var sizeClass
    let slot: ExerciseSlot
    let isActive: Bool

    var body: some View {
        if let letter = slot.exercise.letter.flatMap({ book.course.letter($0) }) {
            VStack(alignment: .leading, spacing: 14) {
                let k: CGFloat = sizeClass == .compact ? 0.62 : 1
                HStack(alignment: .center, spacing: 28 * k) {
                    HStack(alignment: .lastTextBaseline, spacing: 10) {
                        Text(letter.upper).font(Theme.russian(96 * k, weight: .regular))
                        Text(letter.lower).font(Theme.russian(72 * k, weight: .regular))
                    }
                    .fixedSize()
                    .foregroundStyle(Theme.text)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("\(letter.upper)\(letter.lower)")
                            .font(Theme.cursive(54 * k))
                            .fixedSize()
                            .foregroundStyle(Theme.ink)
                        Text("handwritten")
                            .font(Theme.label(11))
                            .foregroundStyle(Theme.muted)
                    }
                    Spacer(minLength: 0)
                    HearButton(text: letter.say, size: 60 * k)
                }
                VStack(alignment: .leading, spacing: 6) {
                    Text(letter.sound)
                        .font(.system(size: 19, weight: .semibold))
                        .foregroundStyle(Theme.text)
                    Text(Theme.stressed(letter.tip, show: book.settings.showStress))
                        .font(.system(size: 17))
                        .foregroundStyle(Theme.text.opacity(0.85))
                    if let example = letter.example, RussianText.letters(in: example).isSubset(of: knownLetters(adding: letter.upper)) {
                        HStack(spacing: 8) {
                            Text("as in").font(.system(size: 15)).foregroundStyle(Theme.muted)
                            RussianLine(text: example, size: 24)
                        }
                    }
                }
                WritingExercise(slot: slot, isActive: isActive, ghostOverride: "\(letter.upper) \(letter.lower)",
                                instructionOverride: "Trace it, then write it a few more times on your own.")
            }
            .onChange(of: isActive) { _, active in if active { voice.play(letter.say) } }
        }
    }

    private func knownLetters(adding upper: String) -> Set<String> {
        Set(book.memories.keys.filter { $0.hasPrefix("letter:") }.map { String($0.dropFirst(7)) }).union([upper])
    }
}

// MARK: - Explain

struct ExplainExercise: View {
    @Environment(Book.self) private var book
    let slot: ExerciseSlot

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(Theme.stressed(slot.exercise.text ?? "", show: book.settings.showStress))
                .font(.system(size: 18))
                .lineSpacing(4)
                .foregroundStyle(Theme.text)
                .fixedSize(horizontal: false, vertical: true)
            if let examples = slot.exercise.examples, !examples.isEmpty {
                VStack(alignment: .leading, spacing: 10) {
                    ForEach(examples, id: \.self) { ex in
                        HStack(spacing: 14) {
                            RussianLine(text: ex.ru, size: 26)
                            Text(ex.en).font(.system(size: 16)).foregroundStyle(Theme.muted)
                        }
                    }
                }
                .padding(.leading, 4)
            }
        }
        .padding(18)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        .overlay(alignment: .leading) {
            Rectangle().fill(Theme.accent.opacity(0.5)).frame(width: 3).padding(.vertical, 12)
        }
        .onAppear {
            if !slot.result.status.isComplete { slot.record { $0.status = .done } }
        }
    }
}

// MARK: - Writing (trace, copy, dictation, translate, fill, and the trace half of meet)

struct WritingExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    let isActive: Bool
    var ghostOverride: String? = nil
    var instructionOverride: String? = nil

    @StateObject private var pad: PadController
    @State private var checking = false
    @State private var lastReading: String?
    @State private var idleTask: Task<Void, Never>?
    @State private var width: CGFloat = 640

    init(slot: ExerciseSlot, isActive: Bool, ghostOverride: String? = nil, instructionOverride: String? = nil) {
        self.slot = slot
        self.isActive = isActive
        self.ghostOverride = ghostOverride
        self.instructionOverride = instructionOverride
        _pad = StateObject(wrappedValue: PadController(fileURL: slot.inkURL))
    }

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var target: String { ex.target ?? "" }
    private var isRecall: Bool { [.dictation, .translate, .fill].contains(ex.kind) }

    private var ghost: String? {
        if let ghostOverride { return ghostOverride }
        return ex.kind == .trace ? target : nil
    }

    private var ghostTimes: Int {
        if ghostOverride != nil { return 1 }  // meet: one model, then free paper
        let len = RussianText.unstressed(ghost ?? "").count
        return len <= 3 ? 3 : len <= 6 ? 2 : 1
    }

    private var stripHeight: CGFloat {
        let sizing = ghost ?? target
        let times = ghost != nil ? ghostTimes : (ex.kind == .copy ? max(ex.times ?? 1, 1) : 1)
        let preferred: CGFloat = horizontalCompact ? 104 : 132
        return StripMetrics.fitting(sizing, times: times, width: width, cursive: book.settings.cursiveModels, preferred: preferred)
    }

    @Environment(\.horizontalSizeClass) private var sizeClass
    private var horizontalCompact: Bool { sizeClass == .compact }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            prompt
            ZStack(alignment: .topTrailing) {
                WritingStrip(controller: pad, height: stripHeight, ghost: ghost, ghostTimes: ghostTimes,
                             cursive: book.settings.cursiveModels, fingerDrawing: book.settings.fingerDrawing)
                    .frame(height: stripHeight)
                    .accessibilityIdentifier("strip-\(slot.index)")
                    .background(GeometryReader { g in Color.clear.onAppear { width = g.size.width }
                        .onChange(of: g.size.width) { _, w in width = w } })
                if result.revealed || result.status == .missed {
                    Correction(text: target)
                        .offset(y: -30)
                        .allowsHitTesting(false)
                }
            }
            controls
        }
        .onAppear { pad.onChange = { _ in scheduleIdleCheck() } }
        .onChange(of: isActive) { _, active in
            if active, ex.kind == .dictation, let s = ex.spoken { voice.play(s) }
        }
    }

    // What the learner is asked for.
    @ViewBuilder private var prompt: some View {
        switch ex.kind {
        case .meet:
            Instruction(text: instructionOverride ?? "Trace it.")
        case .trace:
            HStack(spacing: 14) {
                Instruction(text: "Trace, saying it as you write.")
                HearButton(text: target, size: 36)
            }
        case .copy:
            VStack(alignment: .leading, spacing: 6) {
                Instruction(text: (ex.times ?? 1) > 1 ? "Copy it \(ex.times!) times." : "Copy it.")
                RussianLine(text: target, size: 36)
            }
        case .dictation:
            HStack(spacing: 14) {
                HearButton(text: ex.spoken ?? target, size: 52)
                Instruction(text: "Write what you hear.")
            }
        case .translate:
            VStack(alignment: .leading, spacing: 4) {
                Instruction(text: "Write it in Russian.")
                HStack(spacing: 12) {
                    if let emoji = book.course.word(target)?.emoji { Text(emoji).font(.system(size: 40)) }
                    Text(ex.en ?? "").font(.system(size: 28, weight: .semibold, design: .serif)).foregroundStyle(Theme.text)
                }
            }
        case .fill:
            VStack(alignment: .leading, spacing: 6) {
                Instruction(text: "Write the missing word.")
                (Text(Theme.stressed(ex.before ?? "", show: book.settings.showStress)) + Text(" ____ ").foregroundStyle(Theme.accent)
                    + Text(Theme.stressed(ex.after ?? "", show: book.settings.showStress)))
                    .font(Theme.russian(30, weight: .medium))
                    .foregroundStyle(Theme.text)
                Text(ex.hint ?? "").font(.system(size: 16)).italic().foregroundStyle(Theme.muted)
            }
        default:
            EmptyView()
        }
    }

    @ViewBuilder private var controls: some View {
        HStack(spacing: 10) {
            if !result.status.isComplete && !pad.isEmpty {
                if isRecall {
                    PillButton(title: checking ? "Reading…" : "Check", systemImage: "checkmark", prominent: true) { check(manual: true) }
                        .disabled(checking)
                } else {
                    PillButton(title: "Done", systemImage: "checkmark", prominent: true) { finishPractice() }
                }
                PillButton(title: "Undo", systemImage: "arrow.uturn.backward") { pad.undo() }
                PillButton(title: "Clear", systemImage: "eraser") { pad.clear(); lastReading = nil }
            }
            Spacer(minLength: 0)
            if result.status.isComplete {
                Button("Write it again") { pad.clear(); lastReading = nil; slot.book.reopen(page: slot.pageIndex, exercise: slot.index) }
                    .font(Theme.label(13)).foregroundStyle(Theme.muted)
            }
        }
        .frame(minHeight: 38)
        .animation(.easeOut(duration: 0.2), value: pad.isEmpty)
        if let feedback = feedbackLine {
            feedback
        }
    }

    private var feedbackLine: AnyView? {
        if result.status == .correct, isRecall {
            return AnyView(Text(result.attempts > 1 ? "Got it." : "Right first time.")
                .font(Theme.label(14)).foregroundStyle(Theme.good))
        }
        if result.status == .missed {
            return AnyView(HStack(spacing: 10) {
                Text("Copy the red version once, beside yours. It will come back soon.")
                    .font(.system(size: 15)).foregroundStyle(Theme.redPen)
                if let s = ex.spoken { HearButton(text: s, size: 32) }
            })
        }
        if let reading = lastReading, !result.status.isComplete {
            return AnyView(VStack(alignment: .leading, spacing: 8) {
                Text(reading.isEmpty ? "I can't make that out yet." : "That reads to me as «\(reading)».")
                    .font(.system(size: 15, weight: .medium)).foregroundStyle(Theme.text)
                HStack(spacing: 10) {
                    PillButton(title: "Try again", systemImage: "arrow.counterclockwise") { pad.clear(); lastReading = nil }
                    PillButton(title: "Show me", systemImage: "eye", tint: Theme.redPen) {
                        slot.record { $0.revealed = true; $0.status = .missed }
                        if let s = ex.spoken { voice.play(s) }
                    }
                    PillButton(title: "I wrote it right", systemImage: "hand.thumbsup") {
                        slot.record { $0.status = .correct; $0.recognized = reading }
                    }
                }
            })
        }
        return nil
    }

    private func scheduleIdleCheck() {
        idleTask?.cancel()
        guard !result.status.isComplete, !pad.isEmpty else { return }
        idleTask = Task {
            try? await Task.sleep(for: .milliseconds(isRecall ? 1400 : 1100))
            guard !Task.isCancelled else { return }
            check(manual: false)
        }
    }

    /// Reads the ink. Silent checks only ever confirm success; a manual check also reports misses.
    private func check(manual: Bool) {
        guard !checking, !result.status.isComplete else { return }
        checking = true
        let drawing = pad.drawing
        let target = self.target
        let practice = !isRecall
        Task {
            let reading = await InkReader.read(drawing, target: target)
            checking = false
            guard !result.status.isComplete else { return }
            let ok = reading.map { practice ? ($0.matches || RussianText.isRepetition($0.best, of: target)) : $0.matches } ?? false
            if ok {
                slot.record { r in
                    r.attempts += manual ? 1 : 0
                    r.status = practice ? .done : .correct
                    r.recognized = reading?.best
                }
                lastReading = nil
            } else if manual {
                slot.record { $0.attempts += 1; $0.recognized = reading?.best }
                lastReading = reading?.best ?? ""
            }
        }
    }

    private func finishPractice() {
        idleTask?.cancel()
        slot.record { $0.status = .done }
    }
}

/// The teacher's correction, written in red above the line.
struct Correction: View {
    let text: String
    var body: some View {
        Text(RussianText.unstressed(text))
            .font(Theme.cursive(38))
            .foregroundStyle(Theme.redPen)
            .rotationEffect(.degrees(-2))
            .padding(.trailing, 24)
    }
}

// MARK: - Listen and pick

struct ListenPickExercise: View {
    @Environment(Voice.self) private var voice
    @Environment(Book.self) private var book
    let slot: ExerciseSlot
    let isActive: Bool

    var body: some View {
        let ex = slot.exercise
        let options = ex.options ?? []
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 14) {
                HearButton(text: ex.say ?? "", size: 52)
                Instruction(text: "Which one did you hear?")
            }
            ChoiceGrid(options: options, chosen: slot.result.chosen, correct: options.firstIndex(of: ex.say ?? ""),
                       russian: true, showStress: book.settings.showStress, locked: slot.result.status.isComplete) { i in
                choose(i, options: options)
            }
        }
        .onChange(of: isActive) { _, active in if active, let s = ex.say { voice.play(s) } }
    }

    private func choose(_ i: Int, options: [String]) {
        let right = options[i] == slot.exercise.say
        if !right, let s = options[safe: i] { voice.play(s) }
        slot.record { r in
            r.chosen = i
            r.attempts += 1
            r.status = right ? (r.attempts == 1 ? .correct : .done) : (r.attempts >= 2 ? .missed : .open)
        }
    }
}

struct ReadMeaningExercise: View {
    let slot: ExerciseSlot
    let isActive: Bool

    var body: some View {
        let ex = slot.exercise
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: "What does it mean?")
            RussianLine(text: ex.text ?? "", size: 34)
            ChoiceGrid(options: ex.options ?? [], chosen: slot.result.chosen, correct: ex.answerIndex,
                       russian: false, showStress: true, locked: slot.result.status.isComplete) { i in
                let right = i == ex.answerIndex
                slot.record { r in
                    r.chosen = i
                    r.attempts += 1
                    r.status = right ? (r.attempts == 1 ? .correct : .done) : (r.attempts >= 2 ? .missed : .open)
                }
            }
        }
    }
}

struct ChoiceGrid: View {
    let options: [String]
    let chosen: Int?
    let correct: Int?
    let russian: Bool
    let showStress: Bool
    var locked = false
    let onChoose: (Int) -> Void

    var body: some View {
        let finished = locked || (chosen != nil && chosen == correct)
        FlowRow(spacing: 12) {
            ForEach(Array(options.enumerated()), id: \.offset) { i, option in
                let isChosen = chosen == i
                let isRight = correct == i
                let shown = russian ? Theme.stressed(option, show: showStress) : AttributedString(option)
                Button { onChoose(i) } label: {
                    Text(shown)
                        .font(russian ? Theme.russian(30, weight: .medium) : .system(size: 18, weight: .medium))
                        .padding(.horizontal, russian ? 26 : 18).padding(.vertical, russian ? 12 : 14)
                        .frame(minWidth: russian ? 84 : 0)
                        .foregroundStyle(isChosen ? (isRight ? Theme.good : Theme.redPen) : Theme.text)
                        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                        .overlay(RoundedRectangle(cornerRadius: 14)
                            .stroke(isChosen ? (isRight ? Theme.good : Theme.redPen) : Theme.rule.opacity(0.5),
                                    lineWidth: isChosen ? 2.5 : 1))
                }
                .buttonStyle(.plain)
                .disabled(finished)
                .sensoryFeedback(isRight ? .success : .error, trigger: isChosen)
            }
        }
    }
}

// MARK: - Read aloud

struct ReadAloudExercise: View {
    @Environment(Voice.self) private var voice
    @Environment(SpeechCheck.self) private var speech
    @Environment(Book.self) private var book
    let slot: ExerciseSlot
    let isActive: Bool
    @State private var denied = false

    private var key: String { "\(slot.page.id)-\(slot.index)" }
    private var target: String { slot.exercise.text ?? "" }
    private var recordingExists: Bool { slot.result.recording != nil && FileManager.default.fileExists(atPath: slot.recordingURL.path) }

    var body: some View {
        let mine = speech.activeKey == key
        VStack(alignment: .leading, spacing: 12) {
            Instruction(text: slot.result.status.isComplete ? "Read it aloud." : "Read it aloud, then listen to check.")
            Text(Theme.stressed(target, show: book.settings.showStress))
                .font(Theme.russian(40, weight: .medium))
                .foregroundStyle(Theme.text)
            HStack(spacing: 14) {
                Button { Task { await toggle() } } label: {
                    ZStack {
                        Circle().fill(mine && speech.isRecording ? Theme.redPen : Theme.accent)
                            .frame(width: 60, height: 60)
                        if mine && speech.isRecording {
                            Circle().stroke(Theme.redPen.opacity(0.35), lineWidth: 6)
                                .frame(width: 60 + CGFloat(speech.level) * 26, height: 60 + CGFloat(speech.level) * 26)
                            Image(systemName: "stop.fill").foregroundStyle(.white).font(.system(size: 22))
                        } else if mine && speech.phase == .listening {
                            ProgressView().tint(.white)
                        } else {
                            Image(systemName: "mic.fill").foregroundStyle(.white).font(.system(size: 24))
                        }
                    }
                    .frame(width: 80, height: 80)
                }
                .buttonStyle(.plain)
                .disabled(speech.phase == .listening)
                .accessibilityLabel(mine && speech.isRecording ? "Stop recording" : "Record")
                if recordingExists {
                    Button { voice.playFile(slot.recordingURL, key: "mine-\(key)") } label: {
                        Label("You", systemImage: "person.wave.2.fill").font(Theme.label(14))
                    }
                    .buttonStyle(.bordered).tint(Theme.accent)
                }
                HearButton(text: target, size: 44, label: "Model")
            }
            if recordingExists {
                RhythmView(modelURL: voice.recordingURL(for: target), mineURL: slot.recordingURL, version: slot.result.attempts)
            }
            if denied {
                Text("Drussky needs the microphone and speech recognition, in Settings → Drussky.")
                    .font(.system(size: 14)).foregroundStyle(Theme.redPen)
            }
            if let heard = slot.result.recognized, slot.result.attempts > 0 {
                heardLine(heard)
            } else if slot.result.attempts > 0 && !slot.result.status.isComplete && speech.phase != .listening {
                heardLine("")
            }
        }
    }

    @ViewBuilder private func heardLine(_ heard: String) -> some View {
        let ok = RussianText.matches(heard, target)
        VStack(alignment: .leading, spacing: 8) {
            if ok {
                Text("Heard «\(heard)». ").font(.system(size: 15, weight: .medium)).foregroundStyle(Theme.good)
            } else {
                Text(heard.isEmpty
                     ? (speech.lastTimedOut ? "The Russian listener is still getting ready. Compare yours with the model by ear this time." : "I didn't catch Russian there.")
                     : "I heard «\(heard)».")
                    .font(.system(size: 15, weight: .medium)).foregroundStyle(Theme.text)
                if !slot.result.status.isComplete {
                    HStack(spacing: 10) {
                        PillButton(title: "Sounded right", systemImage: "hand.thumbsup") { slot.record { $0.status = .done } }
                        PillButton(title: "Move on", systemImage: "arrow.down") { slot.record { $0.status = .done } }
                    }
                    Text("Compare yours with the model, then go again if you like.")
                        .font(.system(size: 13)).foregroundStyle(Theme.muted)
                }
            }
        }
    }

    private func toggle() async {
        if speech.activeKey == key && speech.isRecording {
            let heard = await speech.finish(target: target)
            let ok = heard.map { RussianText.matches($0, target) } ?? false
            slot.record { r in
                r.attempts += 1
                r.recording = slot.recordingURL.lastPathComponent
                r.recognized = heard
                if ok && !r.status.isComplete { r.status = r.attempts == 1 ? .correct : .done }
            }
        } else {
            voice.stop()
            if speech.isRecording { speech.cancel() }
            denied = !(await speech.start(to: slot.recordingURL, key: key))
            // Permission was just granted (or already was): load the recognizer while they speak.
            SpeechCheck.warmUp(with: voice.recordingURL(for: target))
        }
    }
}

// MARK: - Layout helpers

struct FlowRow: Layout {
    var spacing: CGFloat = 10

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxW = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowH: CGFloat = 0, widest: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x > 0 && x + size.width > maxW { x = 0; y += rowH + spacing; rowH = 0 }
            x += size.width + spacing
            rowH = max(rowH, size.height)
            widest = max(widest, x - spacing)
        }
        return CGSize(width: min(widest, maxW), height: y + rowH)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        var x = bounds.minX, y = bounds.minY, rowH: CGFloat = 0
        for s in subviews {
            let size = s.sizeThatFits(.unspecified)
            if x > bounds.minX && x + size.width > bounds.maxX { x = bounds.minX; y += rowH + spacing; rowH = 0 }
            s.place(at: CGPoint(x: x, y: y), proposal: ProposedViewSize(size))
            x += size.width + spacing
            rowH = max(rowH, size.height)
        }
    }
}

extension Array {
    subscript(safe i: Int) -> Element? { indices.contains(i) ? self[i] : nil }
}
