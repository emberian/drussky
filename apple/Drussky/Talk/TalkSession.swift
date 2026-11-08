import Foundation
import Observation

/// Drives one conversation on one exercise: asks for scenes, streams each tutor turn and speaks
/// it sentence by sentence as it arrives, hands the microphone to the learner (its own
/// `SpeakingTurn`, no model, no target), and when the dialogue ends has the transcript marked.
/// Every line is written into the result as it happens; a page turn cancels whatever is live and
/// the page resumes where it left off.
@MainActor
@Observable
final class TalkSession {
    enum Status: Equatable {
        case idle
        /// Offline or no key: one line says so.
        case unavailable(String)
        /// Something said earlier, nothing live: continue or end.
        case paused
        case proposing
        case choosing
        /// Waiting for the tutor's first word.
        case thinking
        case speaking
        case yourTurn
        /// The take held no voice (or no Russian): again, type it, or end.
        case notHeard
        case marking
        case done
        case failed(String)
    }

    let ref: ExerciseRef
    let book: Book
    /// The tutor's voice (the system voice unless something better is plugged in).
    var voice: any TutorVoice
    /// Phone-level analysis for the afterwards tips, when the app has it.
    var analyzer: (any PronunciationAnalyzer)?
    /// Talk's own microphone turn: nothing to play first, nothing to compare with.
    let mic = SpeakingTurn()
    /// The page's player, which the turn needs even with nothing to play.
    let pageVoice: Voice

    private(set) var conversation: TalkConversation
    private(set) var dialogue: TalkDialogue
    private(set) var status: Status = .idle
    /// The tutor's line as it is being said (grows sentence by sentence).
    private(set) var streaming = ""
    private(set) var firstTokenAt: Date?
    private(set) var firstSpeechAt: Date?
    private var turnStarted: Date?
    var typedDraft = ""

    private var task: Task<Void, Never>?
    private var speaker: Task<Void, Never>?
    private var sentences: AsyncStream<String>.Continuation?
    /// Checked by every stage after an await, so a cancelled run never writes anything.
    private var run = 0

    var key: String { "\(ref.pageID)-\(ref.index)-talk" }
    var isLive: Bool { [.proposing, .thinking, .speaking, .yourTurn, .marking].contains(status) }

    init(ref: ExerciseRef, book: Book, pageVoice: Voice, voice: (any TutorVoice)? = nil, analyzer: (any PronunciationAnalyzer)? = nil) {
        self.ref = ref
        self.book = book
        self.pageVoice = pageVoice
        self.voice = voice ?? SystemTutorVoice()
        self.analyzer = analyzer
        let saved = book.result(ref)?.talk ?? TalkConversation(turns: Int.random(in: TalkDialogue.turnRange))
        conversation = saved
        dialogue = TalkDialogue(resuming: saved)
    }

    // MARK: Starting and resuming

    /// What the page does on appearing: a fresh conversation asks for scenes; one ended but not
    /// marked gets marked; one left in the middle waits for the learner.
    func appear(autoStart: Bool) {
        guard task == nil else { return }
        if let r = book.result(ref), r.status.isComplete { status = .done; return }
        if conversation.isEnded {
            if let why = unavailableReason { status = .failed(why) } else { mark() }
            return
        }
        if conversation.lines.isEmpty, conversation.scene == nil {
            if let scenes = conversation.proposed, !scenes.isEmpty { status = .choosing }
            else if let why = unavailableReason { status = .unavailable(why) }
            else if autoStart { propose() }
            return
        }
        status = .paused
    }

    var unavailableReason: String? {
        if !Reachability.shared.isOnline { return TalkTutor.Failure.offline.localizedDescription }
        if (Keychain.apiKey ?? "").isEmpty { return "Talk needs your tutor: add your Anthropic API key in Settings." }
        return nil
    }

    func propose() {
        begin(.proposing) { [self] mine in
            do {
                let scenes = try await TalkTutor.proposeScenes(book: book)
                guard run == mine else { return }
                conversation.proposed = scenes
                save()
                status = .choosing
            } catch { fail(error, mine) }
        }
    }

    func choose(_ scene: TalkScene) {
        guard status == .choosing || status == .idle else { return }
        conversation.scene = scene
        conversation.started = book.now()
        dialogue.choose()
        save()
        tutorTurn()
    }

    /// Picks up a conversation left in the middle.
    func resume() {
        switch status {
        case .paused, .failed, .unavailable: break
        default: return
        }
        switch dialogue.phase {
        case .tutorTurn: tutorTurn()
        case .learnerTurn: yourTurn()
        case .choosingScene: status = conversation.proposed == nil ? .idle : .choosing; if status == .idle { propose() }
        case .ended: mark()
        case .cancelled: break
        }
    }

    // MARK: The tutor's turn

    func tutorTurn() {
        begin(.thinking) { [self] mine in
            streaming = ""
            firstTokenAt = nil
            firstSpeechAt = nil
            turnStarted = book.now()
            let (stream, continuation) = AsyncStream.makeStream(of: String.self)
            sentences = continuation
            let voice = self.voice
            speaker = Task { @MainActor in
                for await s in stream {
                    guard !Task.isCancelled else { break }
                    await voice.speak(s)
                }
            }
            let dialogueNow = dialogue
            do {
                let reply = try await TalkTutor.turn(conversation, dialogue: dialogueNow, book: book, onFirstToken: { [self] in
                    guard run == mine else { return }
                    firstTokenAt = Date()
                    status = .speaking
                }, onSentence: { [self] s in
                    guard run == mine else { return }
                    if firstSpeechAt == nil { firstSpeechAt = Date() }
                    streaming += (streaming.isEmpty ? "" : " ") + s
                    continuation.yield(s)
                })
                continuation.finish()
                guard run == mine else { return }
                let completeAt = Date()
                await speaker?.value
                guard run == mine else { return }
                var line = TalkLine(role: .tutor, ru: reply.ru, en: reply.en.isEmpty ? nil : reply.en,
                                    newWords: reply.newWords.isEmpty ? nil : reply.newWords, at: book.now())
                if let t0 = turnStarted {
                    func ms(_ d: Date?) -> Int? { d.map { Int($0.timeIntervalSince(t0) * 1000) } }
                    line.latency = TalkLatency(firstToken: ms(firstTokenAt), firstSpeech: ms(firstSpeechAt), complete: ms(completeAt))
                }
                // The learner's previous line was understood: the tutor answered it.
                if let last = conversation.lines.last, last.role == .learner, !last.ru.isEmpty, let page = book.page(id: ref.pageID) {
                    book.knowledge.talkUnderstood(page: page, index: ref.index, round: book.result(ref)?.roundNumber ?? 0,
                                                  turn: conversation.learnerTurns, heard: last.ru, at: book.now())
                }
                conversation.lines.append(line)
                dialogue.tutorSpoke(goodbye: reply.ended)
                streaming = ""
                save()
                if dialogue.phase == .ended { end() } else { yourTurn() }
            } catch {
                continuation.finish()
                speaker?.cancel()
                voice.stop()
                fail(error, mine)
            }
        }
    }

    /// Says a tutor line again (tap the speaker on it).
    func replay(_ line: TalkLine) {
        guard !isLive || status == .yourTurn else { return }
        mic.cancel()
        speaker?.cancel()
        voice.stop()
        speaker = Task {
            guard !Task.isCancelled else { return }
            await voice.speak(line.ru)
        }
    }

    // MARK: The learner's turn

    /// Leave time to read, replay or compose a reply. Only a microphone tap starts capture.
    func yourTurn() {
        guard !dialogue.isOver else { return }
        task = nil
        mic.cancel()
        status = .yourTurn
    }

    /// The last tutor line is the recognizer's hint (the reply usually reuses its words).
    private func startCapture() {
        guard status == .yourTurn else { return }
        speaker?.cancel()
        voice.stop()
        let mine = run
        let hint = conversation.tutorLines.last?.ru ?? "Да."
        let request = SpeakingTurn.Request(key: key, target: hint, takeURL: book.newTakeURL(ref), model: nil, repeats: 0)
        mic.begin(request, voice: pageVoice) { [self] take in
            guard run == mine else { return }
            let heard = (take.heard ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            let file = FileManager.default.fileExists(atPath: take.file.path) ? take.file.lastPathComponent : nil
            if RussianText.letters(in: heard).isEmpty {
                pendingTake = file
                status = .notHeard
            } else {
                learnerSaid(heard, take: file, typed: false)
            }
        }
    }

    /// A take that held no recognisable Russian, kept if the learner then types what they said.
    private var pendingTake: String?

    /// Start when ready, or stop a take early. An interrupted/failed take can be retried.
    func tap() {
        guard status == .yourTurn else { return }
        if !mic.tap(key) { startCapture() }
    }

    /// Nothing came through: try the turn again.
    func again() {
        guard status == .notHeard || status == .paused else { return }
        pendingTake = nil
        yourTurn()
        startCapture()
    }

    /// The learner types the line instead (the recognizer gave up, or there is no microphone).
    func submitTyped() {
        let text = typedDraft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !RussianText.letters(in: text).isEmpty, [.yourTurn, .notHeard, .paused].contains(status) else { return }
        mic.cancel()
        typedDraft = ""
        learnerSaid(text, take: pendingTake, typed: true)
        pendingTake = nil
    }

    private func learnerSaid(_ heard: String, take: String?, typed: Bool) {
        conversation.lines.append(TalkLine(role: .learner, ru: heard, take: take, typed: typed ? true : nil, at: book.now()))
        dialogue.learnerSpoke()
        save()
        tutorTurn()
    }

    /// Enough for today: the tutor says goodbye, then the pen comes out.
    func endNow() {
        guard !dialogue.isOver else { return }
        mic.cancel()
        voice.stop()
        task?.cancel()
        speaker?.cancel()
        run += 1
        dialogue.endEarly()
        pendingTake = nil
        if conversation.lines.isEmpty { end(); return }
        tutorTurn()
    }

    // MARK: Afterwards

    private func end() {
        conversation.ended = book.now()
        save()
        guard !conversation.learnerLines.isEmpty else { book.skipTalk(ref); status = .done; return }
        mark()
    }

    /// The red pen over the transcript, then one pronunciation tip per take at most.
    func mark() {
        begin(.marking) { [self] mine in
            do {
                let m = try await TalkTutor.mark(conversation, book: book)
                guard run == mine else { return }
                var k = 0
                for i in conversation.lines.indices where conversation.lines[i].role == .learner {
                    k += 1
                    if let c = m.lines[k] { conversation.lines[i].correction = c }
                }
                conversation.praise = m.praise.isEmpty ? nil : m.praise
                conversation.teach = m.teach.isEmpty ? nil : m.teach
                conversation.model = TalkTutor.markModel
                conversation.marked = book.now()
                save()
                await tips(mine)
                guard run == mine else { return }
                book.finishTalk(ref, conversation)
                status = .done
            } catch { fail(error, mine) }
        }
    }

    /// Runs the pronunciation analyzer over each take against the corrected line; the first tip,
    /// kindly worded, goes on the line. Only phone-level analysis is trusted here: a word-level
    /// diff between what was heard and the corrected text would report grammar as pronunciation.
    private func tips(_ mine: Int) async {
        guard let analyzer else { return }
        for i in conversation.lines.indices {
            let line = conversation.lines[i]
            guard line.role == .learner, let file = line.take, line.typed != true else { continue }
            let target = line.corrected
            guard !RussianText.letters(in: target).isEmpty else { continue }
            let url = book.voiceURL(file: file)
            guard FileManager.default.fileExists(atPath: url.path) else { continue }
            guard let report = try? await analyzer.analyze(take: url, target: target, model: nil) else { continue }
            guard run == mine else { return }
            if let tip = Self.kindTip(from: report) { conversation.lines[i].tip = tip }
        }
        save()
    }

    /// One tip, phrased as a suggestion, or none when the take was fine.
    static func kindTip(from report: PronunciationReport) -> String? {
        guard report.accuracy < 0.97, let tip = report.tips.first else { return nil }
        let m = tip.message.trimmingCharacters(in: .whitespaces)
        return m.isEmpty ? nil : "One thing to try: " + m.prefix(1).lowercased() + m.dropFirst()
    }

    /// Ended but unmarked (the network went, the learner left): keep it as it is.
    func keepUnmarked() {
        cancel()
        book.keepTalkUnmarked(ref)
        status = .done
    }

    func skip() {
        cancel()
        book.skipTalk(ref)
        status = .done
    }

    // MARK: Plumbing

    /// Starts a stage: `status` changes at once (the page shows it before any await), the
    /// previous stage is cancelled, and `body` gets the run token to check after each await.
    private func begin(_ initial: Status, _ body: @escaping @MainActor (Int) async -> Void) {
        task?.cancel()
        speaker?.cancel()
        run += 1
        let mine = run
        status = initial
        task = Task { @MainActor in
            await body(mine)
            if run == mine { task = nil }
        }
    }

    private func fail(_ error: Error, _ mine: Int) {
        guard run == mine, !(error is CancellationError) else { return }
        streaming = ""
        status = .failed(error.localizedDescription)
    }

    private func save() { book.saveTalk(ref, conversation) }

    /// A page turn, or leaving: nothing live survives; the lines said so far are already saved.
    func cancel() {
        run += 1
        task?.cancel()
        task = nil
        speaker?.cancel()
        speaker = nil
        sentences?.finish()
        sentences = nil
        voice.stop()
        mic.cancel()
        streaming = ""
        if isLive || status == .notHeard { status = .paused }
    }
}
