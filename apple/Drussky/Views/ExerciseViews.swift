import PencilKit
import SwiftUI

/// Everything an exercise view needs to read and record its own state. It holds the exercise's
/// stable id, not a position, so work that finishes after an await (recognition, a check) lands
/// on the exercise it began on even if pages were inserted meanwhile.
@MainActor
struct ExerciseSlot {
    let book: Book
    let ref: ExerciseRef

    init(book: Book, pageIndex: Int, index: Int) {
        self.book = book
        ref = ExerciseRef(pageID: book.pages[pageIndex].id, index: index)
    }

    init(book: Book, ref: ExerciseRef) {
        self.book = book
        self.ref = ref
    }

    var index: Int { ref.index }
    var pageIndex: Int { book.index(ofPage: ref.pageID) ?? 0 }
    var page: BookPage { book.page(id: ref.pageID) ?? book.pages[pageIndex] }
    var exercise: Exercise { page.exercises[index] }
    var result: ExerciseResult { page.results[index] }

    func record(_ update: (inout ExerciseResult) -> Void) { book.record(ref, update) }
    /// Finish in place. Answering an exercise is not a request to move the paper or play the next one.
    func finish(_ update: (inout ExerciseResult) -> Void) {
        record(update)
    }
    /// Whether an advance signal is for this exercise's page.
    var isAdvanceTarget: Bool { book.advancePageID == ref.pageID }
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
            VStack(spacing: 6) {
                if slot.exercise.baseline != nil && slot.result.status.isComplete {
                    Image(systemName: "bookmark").foregroundStyle(Theme.muted).frame(width: 34, height: 34)
                        .accessibilityLabel("Sample kept")
                } else {
                    MarginMark(number: slot.index + 1, status: slot.result.status, isActive: isActive, isReview: slot.exercise.reviewKey != nil)
                }
                if slot.exercise.baseline == nil && (slot.exercise.kind != .explain || isActive) {
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
            VStack(alignment: .leading, spacing: 8) {
                content
            }
            .padding(.leading, 18)
            .padding(.trailing, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 9)
        .opacity(isActive || slot.result.status.isComplete ? 1 : 0.72)
        .sensoryFeedback(trigger: slot.result.status) { old, new in
            guard !old.isComplete else { return nil }
            switch new {
            case .correct: return .success
            case .done: return .impact(weight: .light)
            case .missed: return .warning
            case .open, .skipped: return nil
            }
        }
        .animation(.easeOut(duration: 0.25), value: isActive)
    }

    @ViewBuilder private var content: some View {
        if let task = slot.exercise.baseline {
            BaselineExercise(slot: slot, task: task).id(slot.ref)
        } else {
        switch slot.exercise.kind {
        case .meet: MeetExercise(slot: slot, isActive: isActive)
        case .explain: ExplainExercise(slot: slot)
        case .trace, .copy: WritingExercise(slot: slot, isActive: isActive)
        case .dictation, .translate, .fill: WritingExercise(slot: slot, isActive: isActive)
        case .listenPick: ListenPickExercise(slot: slot, isActive: isActive)
        case .readAloud: ReadAloudExercise(slot: slot, isActive: isActive)
        case .readMeaning: ReadMeaningExercise(slot: slot, isActive: isActive)
        case .constructionChoice: ConstructionChoiceExercise(slot: slot)
        case .passage: PassageExercise(slot: slot)
        case .soundPair: SoundPairExercise(slot: slot, isActive: isActive)
        case .diary: DiaryExercise(slot: slot, isActive: isActive)
        case .buildWord: BuildWordExercise(slot: slot, isActive: isActive)
        case .familyGuess: FamilyGuessExercise(slot: slot, isActive: isActive)
        case .readHandwriting: ReadHandwritingExercise(slot: slot, isActive: isActive)
        case .talk: TalkExercise(slot: slot, isActive: isActive)
        case .structure: StructureExercise(slot: slot, isActive: isActive)
        case .transform: WritingExercise(slot: slot, isActive: isActive)  // Grammar/: a form from a cue, on the strip
        case .paradigmFill: ParadigmFillExercise(slot: slot, isActive: isActive)
        case .cueSort: CueSortExercise(slot: slot, isActive: isActive)
        case .listening: ListeningExercise(slot: slot, isActive: isActive)  // Views/ListeningView.swift
        }
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
            case .skipped:
                Image(systemName: "minus")
                    .font(.system(size: 20, weight: .bold))
                    .foregroundStyle(Theme.muted)
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
        .accessibilityValue(Self.spoken(status))
    }

    static func spoken(_ status: ExerciseResult.Status) -> String {
        switch status {
        case .open: "not done yet"
        case .done: "done"
        case .correct: "right"
        case .missed: "missed, answer shown"
        case .skipped: "skipped"
        }
    }
}

/// One-line instruction in the tutor's voice.
struct Instruction: View {
    var text: String
    var body: some View {
        Text(text)
            .scaledFont(15, weight: .medium)
            .foregroundStyle(Theme.muted)
    }
}

/// A Russian phrase shown large, tappable to hear.
struct RussianLine: View {
    @Environment(Book.self) private var book
    var text: String
    var size: CGFloat = 34
    /// Show the meaning below the word: first contact with a word is an introduction.
    var gloss: Bool = false
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .center, spacing: 14) {
                Text(Theme.stressed(text, show: book.settings.showStress))
                    .russianFont(size, weight: .medium)
                    .foregroundStyle(Theme.text)
                HearButton(text: text, size: 40)
            }
            if gloss { MeaningTag(text: text) }
        }
    }
}

/// The meaning of a course word (or a kept word), wherever it is presented.
struct MeaningTag: View {
    @Environment(Book.self) private var book
    var text: String
    var body: some View {
        if let w = book.word(text) ?? book.word(RussianText.unstressed(text)) {
            Text(w.en).font(.system(size: 17, weight: .medium)).foregroundStyle(Theme.muted)
            .accessibilityLabel("means \(w.en)")
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
    @State private var demo = StrokeDemo()

    var body: some View {
        if let letter = slot.exercise.letter.flatMap({ book.course.letter($0) }) {
            VStack(alignment: .leading, spacing: 6) {
                let compact = sizeClass == .compact
                let k: CGFloat = compact ? 0.55 : 0.75
                let letters = HStack(alignment: .center, spacing: 18 * k) {
                    HStack(alignment: .lastTextBaseline, spacing: 8) {
                        Text(letter.upper).font(Theme.russian(96 * k, weight: .regular))
                        Text(letter.lower).font(Theme.russian(72 * k, weight: .regular))
                    }
                    .fixedSize()
                    .foregroundStyle(Theme.text)
                    Text("\(letter.upper)\(letter.lower)")
                        .font(Theme.cursive(54 * k))
                        .fixedSize()
                        .foregroundStyle(Theme.ink)
                        .accessibilityLabel("handwritten \(letter.upper)")
                    HearButton(text: letter.say, size: 52 * k)
                    if !compact { WatchStrokesButton(demo: demo, cursive: cursive) }  // on a phone it sits under the letters: the row must fit 390 pt
                }
                let about = VStack(alignment: .leading, spacing: 4) {
                    if compact { WatchStrokesButton(demo: demo) }
                    (Text(letter.sound).font(.system(size: 18, weight: .semibold))
                     + Text("  ") + Text(Theme.stressed(letter.tip, show: book.settings.showStress)).font(.system(size: 17)))
                        .foregroundStyle(Theme.text)
                        .fixedSize(horizontal: false, vertical: true)
                    if let example = letter.example, RussianText.letters(in: example).isSubset(of: knownLetters(adding: letter.upper)) {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("as in").scaledFont(15).foregroundStyle(Theme.muted)
                            RussianLine(text: example, size: 22, gloss: true)
                        }
                    }
                }
                if compact {
                    letters
                    about
                } else {
                    HStack(alignment: .center, spacing: 28) {
                        letters
                        about.frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                WritingExercise(slot: slot, isActive: isActive, ghostOverride: "\(letter.upper) \(letter.lower)",
                                instructionOverride: "Trace it, then write it a few more times on your own.",
                                coach: LetterCoach(letter: letter, demo: demo, cursive: cursive))
            }
            .onChange(of: book.advanceSignal) { _, _ in
                if isActive && slot.isAdvanceTarget { voice.play(letter.say) }
            }
        }
    }

    /// The cursive setting, or a page that asks for cursive on its own (Cursive/).
    private var cursive: Bool { book.settings.cursiveModels || slot.exercise.cursive == true }

    private func knownLetters(adding upper: String) -> Set<String> {
        Set(book.targets.keys.filter { $0.hasPrefix("letter:") }.map { String($0.dropFirst(7)) }).union([upper])
    }
}

// MARK: - Explain

struct ExplainExercise: View {
    @Environment(Book.self) private var book
    let slot: ExerciseSlot

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(Theme.stressed(slot.exercise.text ?? "", show: book.settings.showStress))
                .scaledFont(18)
                .lineSpacing(4)
                .foregroundStyle(Theme.text)
                .fixedSize(horizontal: false, vertical: true)
            if let examples = slot.exercise.examples, !examples.isEmpty {
                FlowRow(spacing: 18) {
                    ForEach(examples, id: \.self) { ex in
                        VStack(alignment: .leading, spacing: 4) {
                            RussianLine(text: ex.ru, size: 24)
                                // A word takes its own width in the flow row, never "м…"; a sentence wider
                                // than the page (a dialogue line on a phone) wraps instead of being clipped.
                                .fixedSize(horizontal: false, vertical: true)
                            Text(ex.en).scaledFont(15).foregroundStyle(Theme.muted)
                        }
                    }
                }
            }
        }
        .padding(14)
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
    var coach: LetterCoach? = nil

    @StateObject private var pad: PadController
    @State private var checking = false
    /// A check asked for while one was running (true if any of those asks was manual).
    @State private var pendingCheck: Bool?
    @State private var lastReading: String?
    @State private var idleTask: Task<Void, Never>?
    @State private var width: CGFloat = 640
    /// Judging myself (Judgement/): Check was tapped; the reader's silent opinion of the ink.
    @State private var checked: Bool
    @State private var opinion: SelfJudgement.ReaderOpinion?
    @State private var readingSilently = false

    init(slot: ExerciseSlot, isActive: Bool, ghostOverride: String? = nil, instructionOverride: String? = nil, coach: LetterCoach? = nil) {
        self.coach = coach
        self.slot = slot
        self.isActive = isActive
        self.ghostOverride = ghostOverride
        self.instructionOverride = instructionOverride
        _pad = StateObject(wrappedValue: PadController(fileURL: slot.inkURL, writer: slot.book.store.ink))
        _checked = State(initialValue: QA.has("-qa-judge") && [.dictation, .translate, .fill].contains(slot.exercise.kind))
    }

    private var ex: Exercise { slot.exercise }
    private var result: ExerciseResult { slot.result }
    private var target: String { ex.target ?? "" }
    private var isRecall: Bool { [.dictation, .translate, .fill, .transform, .readHandwriting].contains(ex.kind) }
    /// The cursive setting, or an exercise that asks for cursive on its own (Cursive/).
    private var cursive: Bool { book.settings.cursiveModels || ex.cursive == true }
    /// Recall judged by the learner after Check, the reader hinting (Judgement/SelfJudgement.swift).
    private var selfJudged: Bool { isRecall && book.settings.writingJudge == .myself }
    private var judging: Bool { selfJudged && checked && !pad.isEmpty && !result.status.isComplete }

    /// A cursive copy shows the hand once on the line (the print word is above), then room to copy.
    private var cursiveCopy: Bool { ex.kind == .copy && ex.cursive == true }

    private var ghost: String? {
        if let ghostOverride { return ghostOverride }
        return ex.kind == .trace || cursiveCopy ? target : nil
    }

    private var ghostTimes: Int {
        if ghostOverride != nil || cursiveCopy { return 1 }  // meet and cursive copy: one model, then free paper
        let len = RussianText.unstressed(ghost ?? "").count
        return len <= 3 ? 3 : len <= 6 ? 2 : 1
    }

    private var stripHeight: CGFloat {
        let sizing = ghost ?? target
        let times = ghost != nil ? ghostTimes + (cursiveCopy ? max(ex.times ?? 1, 1) : 0) : (ex.kind == .copy ? max(ex.times ?? 1, 1) : 1)
        let preferred: CGFloat = horizontalCompact ? 96 : 112
        return StripMetrics.fitting(sizing, times: times, width: width, cursive: cursive, preferred: preferred)
    }

    @Environment(\.horizontalSizeClass) private var sizeClass
    private var horizontalCompact: Bool { sizeClass == .compact }

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            prompt
            if isRecall { HookLine(word: target) }  // Hooks/: a fading mnemonic before the recall
            ZStack(alignment: .topTrailing) {
                WritingStrip(controller: pad, height: stripHeight, ghost: ghost, ghostTimes: ghostTimes,
                             cursive: cursive, fingerDrawing: book.settings.fingerDrawing, locked: judging)
                    .frame(height: stripHeight)
                    .accessibilityIdentifier("strip-\(slot.index)")
                    .background(GeometryReader { g in Color.clear.onAppear { width = g.size.width }
                        .onChange(of: g.size.width) { _, w in width = w } })
                    .overlay { if let coach, let ghost { StrokeDemoOverlay(demo: coach.demo, placed: coach.placed(ghost: ghost), height: stripHeight) } }
                if result.revealed || result.status == .missed {
                    Correction(text: target)
                        .offset(y: -30)
                        .allowsHitTesting(false)
                }
            }
            .overlay(alignment: .bottomTrailing) {
                if result.status.isComplete {
                    Button { Task { await writeAgain() } } label: {
                        Label("Write it again", systemImage: "arrow.counterclockwise")
                            .font(Theme.label(12)).foregroundStyle(Theme.muted.opacity(0.8))
                    }
                    .buttonStyle(.plain)
                    .padding(.bottom, 2)
                }
            }
            if judging {
                SelfJudgementPanel(target: target, reader: opinion?.revision == pad.revision ? opinion : nil, reading: readingSilently) { verdict in
                    idleTask?.cancel()
                    let reader = opinion?.revision == pad.revision ? opinion : nil
                    noteWritingFluency(matched: reader?.matched ?? false, reading: reader?.reading, confidence: nil)
                    SelfJudgement.record(verdict, on: slot, reader: reader, responseMs: pad.inkDurationMs)
                    if verdict == .notQuite, let s = ex.spoken { voice.play(s) }
                }
            }
            if KeyboardPresence.shared.present { TypedAnswer(slot: slot, isActive: isActive) }  // Devices/Typing: a hardware keyboard may answer
            if (result.inkAttempts ?? 0) > 0 {
                EarlierAttempts(urls: (1...(result.inkAttempts ?? 0)).map { slot.book.inkAttemptURL(slot.ref, attempt: $0) },
                                height: stripHeight * 0.4)
            }
            // Formation hints compare with the stroke models of the hand shown: print or cursive.
            if let coach {  // tucked into the strip's empty bottom margin, clear of "Write it again"
                FormationHintLine(pad: pad, letter: coach.letter, cursive: cursive, lift: stripHeight * 0.08,
                                  clearTrailing: result.status.isComplete ? 130 : 0)
            }
            controls
        }
        .onAppear { pad.onChange = { _ in scheduleIdleCheck() } }
        .onChange(of: pad.isEmpty) { _, empty in if checked && !empty { readSilently() } }  // ink already on the strip when Check stands
        .onChange(of: book.advanceSignal) { _, _ in
            guard isActive, slot.isAdvanceTarget, ex.kind == .dictation else { return }
            if ex.narration != nil { voice.stop(); NarrationPlayer.shared.play(exercise: ex, owner: "\(slot.page.id)-\(slot.index)") }
            else if let s = ex.spoken { voice.play(s) }
        }
    }

    // What the learner is asked for.
    @ViewBuilder private var prompt: some View {
        switch ex.kind {
        case .meet:
            Instruction(text: instructionOverride ?? "Trace it.")
        case .trace:
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 14) {
                    Instruction(text: "Trace, saying it as you write.")
                    HearButton(text: target, size: 36)
                }
                MeaningTag(text: target)
            }
        case .copy:
            VStack(alignment: .leading, spacing: 6) {
                Instruction(text: (ex.times ?? 1) > 1 ? "Copy it \(ex.times!) times." : "Copy it.")
                RussianLine(text: target, size: 36, gloss: true)
            }
        case .dictation:
            HStack(spacing: 14) {
                if ex.narration != nil {  // a sentence of a narrated text, at the reader's pace
                    NarrationClipButton(exercise: ex, owner: "\(slot.page.id)-\(slot.index)", size: 52)
                } else {
                    HearButton(text: ex.spoken ?? target, size: 52)
                }
                Instruction(text: ex.narration != nil ? "Write what the reader says." : "Write what you hear.")
            }
        case .translate:
            VStack(alignment: .leading, spacing: 4) {
                Instruction(text: "Write it in Russian.")
                HStack(spacing: 12) {
                    Text(ex.en ?? "").scaledFont(28, weight: .semibold, design: .serif).foregroundStyle(Theme.text)
                }
            }
        case .fill:
            VStack(alignment: .leading, spacing: 6) {
                Instruction(text: "Write the missing word.")
                (Text(Theme.stressed(ex.before ?? "", show: book.settings.showStress)) + Text(" ____ ").foregroundStyle(Theme.accent)
                    + Text(Theme.stressed(ex.after ?? "", show: book.settings.showStress)))
                    .russianFont(30, weight: .medium)
                    .foregroundStyle(Theme.text)
                Text(ex.hint ?? "").scaledFont(16).italic().foregroundStyle(Theme.muted)
            }
        case .transform:
            TransformPrompt(exercise: ex)
        default:
            EmptyView()
        }
    }

    @ViewBuilder private var controls: some View {
        FlowRow(spacing: 10) {
            if !result.status.isComplete && !pad.isEmpty && !judging {
                if selfJudged {
                    PillButton(title: "Check", systemImage: "checkmark", prominent: true) { checked = true; readSilently() }
                        .accessibilityIdentifier("check-\(slot.index)")
                } else if isRecall {
                    PillButton(title: checking ? "Reading…" : "Check", systemImage: "checkmark", prominent: true) { check(manual: true) }
                        .disabled(checking)
                } else {
                    PillButton(title: "Done", systemImage: "checkmark", prominent: true) { finishPractice() }
                }
                PillButton(title: "Undo", systemImage: "arrow.uturn.backward") { pad.undo() }
                PillButton(title: "Clear", systemImage: "eraser") { pad.clear(); lastReading = nil }
            }
        }
        .frame(minHeight: 44)
        .frame(maxWidth: .infinity, alignment: .leading)
        .animation(.easeOut(duration: 0.2), value: pad.isEmpty)
        if let feedback = feedbackLine {
            feedback
        }    }

    private var feedbackLine: AnyView? {
        if let line = SelfJudgement.feedback(result) {
            return AnyView(Text(line).font(Theme.label(14)).foregroundStyle(Theme.good))
        }
        if result.status == .correct, isRecall {
            return AnyView(Text(result.isRightFirstTime ? "Right first time." : result.assisted == true ? "Written again after seeing it."
                                : result.selfAssessed == true ? "Marked right by you." : "Got it.")
                .font(Theme.label(14)).foregroundStyle(Theme.good))
        }
        if result.status == .missed {
            return AnyView(VStack(alignment: .leading, spacing: 8) {
                HStack(spacing: 10) {
                    Text("Copy the red version once, beside yours. It will come back soon.")
                        .scaledFont(15).foregroundStyle(Theme.redPen)
                    if let s = ex.spoken { HearButton(text: s, size: 32) }
                }
                GrammarRowHint(exercise: ex)  // Grammar/: the paradigm row of a missed form, its cell marked
            })
        }
        if let reading = lastReading, !result.status.isComplete {
            return AnyView(VStack(alignment: .leading, spacing: 8) {
                Text(reading.isEmpty ? "I can't make that out yet." : "That reads to me as «\(reading)».")
                    .scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                FlowRow(spacing: 10) {
                    PillButton(title: "Try again", systemImage: "arrow.counterclockwise") { pad.clear(); lastReading = nil }
                    PillButton(title: "Show me", systemImage: "eye", tint: Theme.redPen) {
                        slot.record { $0.revealed = true; $0.status = .missed }
                        if let s = ex.spoken { voice.play(s) }
                    }
                    PillButton(title: "I wrote it right", systemImage: "hand.thumbsup") {
                        slot.finish { $0.status = .correct; $0.selfAssessed = true; $0.recognized = reading }
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
            if selfJudged { readSilently() } else { check(manual: false) }
        }
    }

    /// Judging myself: the reader looks at the ink and keeps its opinion for the panel's hint and
    /// the evidence log; it never changes the result. Re-reads when the ink changed meanwhile.
    private func readSilently() {
        guard selfJudged, !result.status.isComplete, !pad.isEmpty, !readingSilently else { return }
        let drawing = pad.drawing, revision = pad.revision, target = self.target, subjectDrop = ex.kind == .translate
        guard opinion?.revision != revision else { return }
        readingSilently = true
        Task {
            let r = await InkReader.read(drawing, target: target, subjectDrop: subjectDrop)
            readingSilently = false
            opinion = SelfJudgement.ReaderOpinion(reading: r?.best, matched: r?.matches, revision: revision)
            lastConfidence = r?.confidence
            if pad.revision != revision { readSilently() }
        }
    }

    @State private var lastConfidence: Double?

    /// Fluency/: strokes a second and legibility of a word written from memory.
    private func noteWritingFluency(matched: Bool, reading: String?, confidence: Double?) {
        slot.book.fluency.noteWriting(exercise: ex, drawing: pad.drawing, matched: matched, reading: reading, confidence: confidence ?? lastConfidence,
                                      component: slot.book.knowledge.domain.utteranceID(target))
    }

    /// Reads the ink. Silent checks only ever confirm success; a manual check also reports misses.
    /// A reading only counts for the ink it was made from: if the strip changed while Vision was
    /// reading (more writing, Clear, Undo), the reading is dropped and the latest ink is read.
    private func check(manual: Bool) {
        guard !result.status.isComplete, !pad.isEmpty else { return }
        if checking { pendingCheck = (pendingCheck ?? false) || manual; return }
        checking = true
        let drawing = pad.drawing, revision = pad.revision
        let took = pad.inkDurationMs  // first stroke to the last before this check
        let target = self.target
        let practice = !isRecall
        let subjectDrop = ex.kind == .translate
        let ref = slot.ref, book = slot.book, round = result.roundNumber
        Task {
            let reading = await InkReader.read(drawing, target: target, subjectDrop: subjectDrop)
            checking = false
            let again = pendingCheck
            pendingCheck = nil
            guard let current = book.result(ref), !current.status.isComplete, current.roundNumber == round else { return }
            guard pad.revision == revision else {
                check(manual: manual || again == true)
                return
            }
            let ok = reading.map { practice ? ($0.matches || RussianText.isRepetition($0.best, of: target)) : $0.matches } ?? false
            let slot = ExerciseSlot(book: book, ref: ref)
            if ok {
                if !practice { noteWritingFluency(matched: true, reading: reading?.best, confidence: reading?.confidence) }
                let apply: ((inout ExerciseResult) -> Void) -> Void = manual ? slot.finish : slot.record
                apply { r in
                    r.attempts += manual ? 1 : 0
                    r.status = practice ? .done : .correct
                    r.recognized = reading?.best
                    r.responseMs = took
                }
                lastReading = nil
            } else if manual {
                slot.record { r in
                    r.attempts += 1
                    r.failedChecks = (r.failedChecks ?? 0) + 1
                    r.recognized = reading?.best
                    r.responseMs = took
                }
                lastReading = reading?.best ?? ""
            } else if again == true {
                check(manual: true)
            }
        }
    }

    /// Starts another round on a fresh strip. The ink so far is kept as an earlier attempt;
    /// if it can't be kept, nothing is cleared.
    private func writeAgain() async {
        idleTask?.cancel()
        let book = slot.book, ref = slot.ref
        let n = (result.inkAttempts ?? 0) + 1
        let dest = book.inkAttemptURL(ref, attempt: n)
        do {
            try await pad.archive(to: dest)
        } catch {
            book.report("ink: couldn't keep the earlier writing (\((error as NSError).localizedDescription)), so it was left in place")
            return
        }
        lastReading = nil
        book.reopen(ref, keptInk: FileManager.default.fileExists(atPath: dest.path))
    }

    private func finishPractice() {
        idleTask?.cancel()
        slot.finish { $0.status = .done }
    }
}

/// The teacher's correction, written in red above the line.
struct Correction: View {
    @Environment(Book.self) private var book
    let text: String
    var body: some View {
        Text(Theme.stressed(text, show: book.settings.showStress))
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
    /// When the clip last finished (response latency runs from here to the tap).
    @State private var clipEndedAt: Date?

    var body: some View {
        let ex = slot.exercise
        let options = ex.options ?? []
        let clipKey = RussianText.audioKey(ex.say ?? "")
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 14) {
                HearButton(text: ex.say ?? "", size: 52)
                Instruction(text: "Which one did you hear?")
            }
            ChoiceGrid(options: options, chosen: slot.result.chosen, correct: options.firstIndex(of: ex.say ?? ""),
                       russian: true, showStress: book.settings.showStress, locked: slot.result.status.isComplete) { i in
                choose(i, options: options)
            }
            if slot.result.status.isComplete, let say = ex.say, slot.result.status != .missed {
                HStack(spacing: 8) {
                    Text(Theme.stressed(say, show: book.settings.showStress)).russianFont(22, weight: .medium)
                    MeaningTag(text: say)
                }
            }
            if slot.result.status == .missed, let say = ex.say {
                ChoiceCorrection(slot: slot) {
                    HStack(spacing: 10) {
                        Text("It was").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                        Text(Theme.stressed(say, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(Theme.good)
                        HearButton(text: say, size: 34)
                    }
                }
            }
        }
        .onChange(of: book.advanceSignal) { _, _ in
            if isActive, slot.isAdvanceTarget, let s = ex.say { voice.play(s) }
        }
        .onChange(of: voice.speakingKey) { old, new in
            // The clip of this exercise just ended: the clock for the tap starts now.
            if old == clipKey, new == nil { clipEndedAt = Date() }
        }
    }

    private func choose(_ i: Int, options: [String]) {
        let say = slot.exercise.say
        let right = options[i] == say
        let missedNow = !right && slot.result.attempts + 1 >= 2
        let took = clipEndedAt.map { Int(Date().timeIntervalSince($0) * 1000) }
        clipEndedAt = nil
        // A wrong pick plays what was picked; the last miss plays what it really was.
        if missedNow, let say { voice.play(say) } else if !right, let s = options[safe: i] { voice.play(s) }
        slot.finish { r in
            r.chosen = i
            r.attempts += 1
            r.status = right ? (r.attempts == 1 ? .correct : .done) : (r.attempts >= 2 ? .missed : .open)
            r.responseMs = took
        }
    }
}

struct ReadMeaningExercise: View {
    let slot: ExerciseSlot
    let isActive: Bool
    /// When the exercise came up (response latency runs from here to the tap).
    @State private var shownAt: Date?

    var body: some View {
        let ex = slot.exercise
        // A Russian line asks for its meaning; an English question (a shelf reading's) asks about the text just read.
        let question = RussianText.letters(in: ex.text ?? "").isEmpty
        VStack(alignment: .leading, spacing: 12) {
            if question {
                Instruction(text: "From the text")
                Text(Theme.stressed(ex.text ?? "", show: slot.book.settings.showStress)).scaledFont(20, weight: .medium).foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true).frame(maxWidth: 640, alignment: .leading)
            } else {
                Instruction(text: "What does it mean?")
                RussianLine(text: ex.text ?? "", size: 34)
            }
            ChoiceGrid(options: ex.options ?? [], chosen: slot.result.chosen, correct: ex.answerIndex,
                       russian: false, showStress: true, locked: slot.result.status.isComplete) { i in
                let right = i == ex.answerIndex
                let took = shownAt.map { Int(Date().timeIntervalSince($0) * 1000) }
                shownAt = Date()  // a second pick is timed from the first
                slot.finish { r in
                    r.chosen = i
                    r.attempts += 1
                    r.status = right ? (r.attempts == 1 ? .correct : .done) : (r.attempts >= 2 ? .missed : .open)
                    r.responseMs = took
                }
            }
            if slot.result.status == .missed, let answer = ex.answerIndex.flatMap({ ex.options?[safe: $0] }) {
                ChoiceCorrection(slot: slot) {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 8) {
                            Text(question ? "The text says" : "It means").scaledFont(15, weight: .medium).foregroundStyle(Theme.text)
                            Text(answer).scaledFont(17, weight: .semibold).foregroundStyle(Theme.good)
                        }
                        if question, let why = ex.hint {
                            Text(Theme.stressed(why, show: slot.book.settings.showStress)).scaledFont(14).italic().foregroundStyle(Theme.muted)
                                .fixedSize(horizontal: false, vertical: true)
                        }
                    }
                }
            }
        }
        .onChange(of: isActive, initial: true) { _, active in if active, shownAt == nil { shownAt = Date() } }
    }
}

/// After a missed choice: the answer, said and shown, and an optional (ungraded) try again.
struct ChoiceCorrection<Content: View>: View {
    let slot: ExerciseSlot
    @ViewBuilder let content: Content

    var body: some View {
        HStack(spacing: 14) {
            content
            PillButton(title: "Try it again", systemImage: "arrow.counterclockwise") { slot.book.reopen(slot.ref) }
                .accessibilityHint("Practice only; it isn't graded")
        }
    }
}

/// Choices that say, by shape as well as colour, which one was picked and which was right.
/// Once the exercise is over the right answer is always marked, picked or not.
struct ChoiceGrid: View {
    let options: [String]
    let chosen: Int?
    let correct: Int?
    let russian: Bool
    let showStress: Bool
    var locked = false
    /// A pick not judged yet (a message's answers are judged together at the end): the chosen one
    /// is shown as chosen, with no mark and no colour of right or wrong.
    var neutral = false
    let onChoose: (Int) -> Void

    var body: some View {
        let finished = locked || (chosen != nil && chosen == correct)
        let judged = finished || !neutral
        FlowRow(spacing: 12) {
            ForEach(Array(options.enumerated()), id: \.offset) { i, option in
                let isChosen = chosen == i
                let isRight = correct == i
                let revealed = finished && isRight && !isChosen
                let shown = russian ? Theme.stressed(option, show: showStress) : AttributedString(option)
                let color: Color = isChosen ? (judged ? (isRight ? Theme.good : Theme.redPen) : Theme.accent) : (revealed ? Theme.good : Theme.text)
                Button { onChoose(i) } label: {
                    HStack(spacing: 8) {
                        if (isChosen && judged) || revealed {
                            Image(systemName: isRight ? "checkmark" : "xmark").font(.system(size: 15, weight: .bold))
                        }
                        Group {
                            if russian { Text(shown).russianFont(30, weight: .medium) } else { Text(shown).scaledFont(18, weight: .medium) }
                        }
                    }
                    .padding(.horizontal, russian ? 26 : 18).padding(.vertical, russian ? 12 : 14)
                    .frame(minWidth: russian ? 84 : 0)
                    .foregroundStyle(color)
                    .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
                    .overlay(RoundedRectangle(cornerRadius: 14)
                        .stroke(isChosen || revealed ? color : Theme.rule.opacity(0.5),
                                style: StrokeStyle(lineWidth: isChosen || revealed ? 2.5 : 1, dash: revealed ? [6, 4] : [])))
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("choice-\(i)")
                .allowsHitTesting(!finished)  // not .disabled: the marked answers stay full strength
                .sensoryFeedback(judged ? (isRight ? .success : .error) : .selection, trigger: isChosen)
                .accessibilityValue(isChosen ? (judged ? (isRight ? "your choice, right" : "your choice, wrong") : "your choice") : revealed ? "the right answer" : "")
            }
        }
    }
}

// Read aloud: Speaking/ReadAloudExercise.swift

// MARK: - Layout helpers

struct FlowRow: Layout {
    var spacing: CGFloat = 10

    /// Each child is offered the row's width, never more: a child that would wrap (a scene card
    /// with a long title) reports the height it will really take, so rows below do not overlap it.
    private func fit(_ s: LayoutSubview, in width: CGFloat) -> CGSize {
        s.sizeThatFits(ProposedViewSize(width: width.isFinite ? width : nil, height: nil))
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let maxW = proposal.width ?? .infinity
        var x: CGFloat = 0, y: CGFloat = 0, rowH: CGFloat = 0, widest: CGFloat = 0
        for s in subviews {
            let size = fit(s, in: maxW)
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
            let size = fit(s, in: bounds.width)
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

// MARK: - Passage

/// A short text to read: tap any word for its meaning and sound, hear the whole, and reveal a
/// translation only if you want it.
struct PassageExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let slot: ExerciseSlot
    @State private var selected: String?
    @State private var showTranslation = false
    @State private var pass = ReadingPass()  // Fluency/: a finger-follow reading pass, words a minute
    /// The «Структу́ра» layer (Structure/): the text's sentences found in library-structure.json, and what is open.
    @State private var passage: PassageStructure?
    @State private var layer = StructureLayer()

    private var text: String { slot.exercise.text ?? "" }
    private var structureOn: Bool { passage != nil && book.settings.showStructure == true }

    /// Lines (dialogue turns, verse) of words, punctuation kept attached for display.
    private var lines: [[String]] {
        text.split(separator: "\n", omittingEmptySubsequences: true)
            .map { $0.split(separator: " ").map(String.init) }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if slot.exercise.narration != nil, slot.exercise.words != nil {
                // A human reading: the words light up as they are said (Narration/ReadAlongView.swift).
                ReadAlongView(exercise: slot.exercise, owner: "\(slot.page.id)-\(slot.index)", selected: $selected,
                              structure: passage, layer: layer)
                panel
            } else {
            // One row where it fits (iPad); the instruction on its own line above the controls on a phone.
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 14) {
                    Instruction(text: "Read it. Tap any word you want to check.").fixedSize()
                    HearButton(text: text, size: 40, label: "Whole text")
                    Spacer(minLength: 0)
                    if passage != nil { StructureToggle() }
                }
                VStack(alignment: .leading, spacing: 8) {
                    Instruction(text: "Read it. Tap any word you want to check.")
                    HStack(spacing: 14) {
                        HearButton(text: text, size: 40, label: "Whole text")
                        Spacer(minLength: 0)
                        if passage != nil { StructureToggle() }
                    }
                }
            }
            VStack(alignment: .leading, spacing: 6) {
              ForEach(Array(lines.enumerated()), id: \.offset) { li, tokens in
              FlowRow(spacing: 8) {
                ForEach(Array(tokens.enumerated()), id: \.offset) { ti, token in
                    let word = Self.bare(token)
                    let tok = passage?.token(line: li, index: ti)
                    let mark = structureOn && passage != nil
                        ? layer.mark(for: tok, previous: passage?.token(line: li, index: ti - 1), next: passage?.token(line: li, index: ti + 1),
                                     in: passage!, roles: Structure.shared.roles) : .none
                    let shown = Text(Theme.stressed(token, show: book.settings.showStress))
                        .russianFont(28)
                        .foregroundStyle(Theme.text)
                        .padding(.vertical, 2).padding(.horizontal, 3)
                    if word.isEmpty {
                        shown  // a dash or a lone quotation mark: nothing to look up
                    } else {
                        Button { select(word, token: tok) } label: {
                            shown.background(RoundedRectangle(cornerRadius: 6)
                                .fill(selected == word ? Theme.accent.opacity(0.15) : .clear))
                                .structureMarks(mark, on: structureOn)
                        }
                        .buttonStyle(.plain)
                        .simultaneousGesture(LongPressGesture(minimumDuration: 0.45).onEnded { _ in
                            guard structureOn, let tok, let si = tok.sentence, let s = passage?.sentence(si) else { return }
                            layer.longPressed(s, at: si, words: tok.words)
                        })
                    }
                }
              }
              }
            }
            .padding(18)
            .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
            .overlay { ReadingPassOverlay(pass: pass, text: text) }
            panel
            ReadingPassBar(pass: pass, text: text)
            if let frames = slot.exercise.frames, !frames.isEmpty { PassageFrames(frames: frames) }
            if let notice = slot.exercise.notice, !notice.isEmpty, slot.result.status.isComplete { PassageNotice(lines: notice) }
            }
            if let selected {
                HStack(spacing: 12) {
                    Text(selected).russianFont(24, weight: .medium).foregroundStyle(Theme.text)
                    // A word still unknown, on a root already known, is shown taken apart first; one with no
                    // gloss on the page is asked of the tutor (OnDevice/TutorGlossView.swift).
                    GlossLine(word: selected, context: text, known: gloss(for: selected))
                    HearButton(text: selected, size: 34)
                }
                .transition(.opacity)
            }
            if let en = slot.exercise.en {
                Button(showTranslation ? "Hide the translation" : "Show the translation") {
                    withAnimation { showTranslation.toggle() }
                }
                .font(Theme.label(14)).foregroundStyle(Theme.accent)
                if showTranslation {
                    Text(en).scaledFont(17).italic().foregroundStyle(Theme.text.opacity(0.85))
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            if !slot.result.status.isComplete {
                PillButton(title: "I've read it", systemImage: "checkmark", prominent: true) {
                    slot.finish { $0.status = .done }
                }
            }
        }
        .animation(.easeOut(duration: 0.2), value: selected)
        .animation(.easeOut(duration: 0.2), value: layer.panel)
        .onAppear { if passage == nil { passage = PassageStructure(text: text) } }
        .onChange(of: book.settings.showStructure) { _, on in if on != true { layer.reset() } }
    }

    /// What a tap or long press opened in the structure layer, right under the text.
    @ViewBuilder private var panel: some View {
        if structureOn, let passage, layer.panel != nil {
            StructurePanel(layer: layer, passage: passage)
        }
    }

    private func select(_ word: String, token: PassageStructure.Token? = nil) {
        if structureOn, layer.consumeSuppressedTap() { return }  // the tap that ends a long press
        selected = word
        voice.play(word)
        if structureOn, let passage, let token, let si = token.sentence, let s = passage.sentence(si) { layer.tapped(s, at: si, words: token.words, in: passage) }
    }

    /// The page's own gloss first, then the lexicon, then a phrase of the lexicon the word is taught
    /// in, then (for notation) how the page says it is read.
    private func gloss(for word: String) -> String? {
        let key = RussianText.key(word)
        if let g = slot.exercise.glosses?.first(where: { RussianText.key($0.key) == key })?.value { return g }
        if let w = book.word(word)?.en { return w }
        if let p = book.phraseGloss(for: word) { return p }
        if let f = slot.exercise.frames?.first(where: { $0.symbol == word }) { return "\(f.ru) — \(f.en)" }
        return nil
    }

    /// The token without the punctuation around it: «Что», э́то?», „так“, ’вот’ and (A) all read clean.
    static func bare(_ token: String) -> String {
        token.trimmingCharacters(in: .punctuationCharacters)
    }
}

/// What to hear in a dialogue just read: one to three lines, the stressed Russian first and the
/// English after it, as the file wrote them. Shown only once the passage is marked read.
struct PassageNotice: View {
    @Environment(Book.self) private var book
    let lines: [String]

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("WHAT TO HEAR").font(Theme.label(11)).tracking(1.4).foregroundStyle(Theme.muted)
            ForEach(Array(lines.enumerated()), id: \.offset) { _, line in
                Text(Theme.stressed(line, show: book.settings.showStress)).scaledFont(15).foregroundStyle(Theme.text.opacity(0.85))
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(.leading, 6)
        .frame(maxWidth: 640, alignment: .leading)
        .transition(.opacity)
    }
}

/// How a passage's phrases and notation are read: a quiet list under the text, each line
/// hearable (the synthetic voice until a clip is recorded), with the grammar behind it.
struct PassageFrames: View {
    @Environment(Book.self) private var book
    let frames: [Exercise.Frame]

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("HOW IT IS READ").font(Theme.label(11)).tracking(1.4).foregroundStyle(Theme.muted)
            ForEach(Array(frames.enumerated()), id: \.offset) { _, f in
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    if let s = f.symbol {
                        Text(s).font(.system(size: 20, design: .serif)).foregroundStyle(Theme.text).frame(minWidth: 26, alignment: .leading)
                    }
                    VStack(alignment: .leading, spacing: 2) {
                        HStack(spacing: 8) {
                            Text(Theme.stressed(f.ru, show: book.settings.showStress)).russianFont(21).foregroundStyle(Theme.text)
                            HearButton(text: f.ru, size: 26)
                        }
                        Text(f.en).scaledFont(14).foregroundStyle(Theme.muted)
                        if let n = f.note { Text(n).scaledFont(13).italic().foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true) }
                    }
                }
            }
        }
        .padding(.leading, 6)
        .frame(maxWidth: 640, alignment: .leading)
    }
}
