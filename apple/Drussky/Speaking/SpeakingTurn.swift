import AVFoundation
import Observation

/// One read-aloud turn: the model plays (skippable, as many times as the learner set) with the
/// microphone closed, then the page waits for their tap; on it a soft cue sounds and the
/// microphone takes the voice from its first sound to the silence after it. The take is
/// trimmed, written as the page's recording and analysed; the result goes back to the exercise
/// it began on. With no model to play, the tap that began the turn is that tap.
///
/// The app's one microphone: starting a turn anywhere ends any other. A page turn ends a turn
/// still listening (nothing is kept); one already being analysed finishes and lands on its own
/// exercise, which it addresses by id.
@MainActor
@Observable
final class SpeakingTurn {
    enum Phase: Equatable {
        case idle
        /// Asking for the microphone (first time only).
        case preparing
        /// Playing the model: this time, of how many.
        case model(Int, of: Int)
        /// The model has played; nothing is listening until the learner taps.
        case ready
        /// The "your turn" cue.
        case cue
        /// Listening, no voice yet.
        case waiting
        /// The voice is coming in.
        case hearing
        case analysing
    }
    enum Problem: Equatable { case micDenied, interrupted, nothingHeard, failed }

    struct Request {
        var key: String
        var target: String
        /// A new file for the take (nothing is written there unless a voice is captured).
        var takeURL: URL
        var model: URL?
        var repeats: Int
        /// Endpointing for this take; nil takes the target's own (`config(target:model:)`).
        var config: VoiceActivity.Config? = nil
    }
    struct Take {
        var file: URL
        var report: PronunciationReport?
        var heard: String?
        var recognitionAllowed: Bool
        var timedOut: Bool
        /// From the microphone opening after the cue to the voice's onset, in milliseconds: how
        /// long the words took to come. nil when the take was stopped by a tap before any voice.
        var onsetMs: Int?
    }

    private(set) var phase: Phase = .idle
    private(set) var activeKey: String?
    private(set) var level: Float = 0
    /// What went wrong on the last turn of an exercise, by key.
    private(set) var problem: (key: String, problem: Problem)?
    private(set) var recognitionDenied = false
    private(set) var lastTimedOut = false
    /// Phone-level analysis, when the app has it; the word-level analyzer stands in otherwise
    /// and whenever this one fails.
    var analyzer: (any PronunciationAnalyzer)?

    private var session = 0
    /// Bumped only by `invalidate()`: a turn begun in an earlier epoch may finish its analysis,
    /// but writes no take or report and calls nothing back. Page turns bump `session` alone.
    private var epoch = 0
    private var task: Task<Void, Never>?
    private var capture: TakeCapture?
    private weak var voice: Voice?
    private var skipModel = false
    private var tapped = false
    /// The turn waiting in `.ready` for its tap (`tap` resumes it; `cancel` releases it).
    private var readiness: CheckedContinuation<Void, Never>?
    /// Reports of takes analysed this run, by file name (each is also saved beside its take).
    private var reports: [String: PronunciationReport] = [:]

    init() {
        let center = NotificationCenter.default
        center.addObserver(forName: AVAudioSession.interruptionNotification, object: nil, queue: .main) { [weak self] note in
            let began = (note.userInfo?[AVAudioSessionInterruptionTypeKey] as? UInt) == AVAudioSession.InterruptionType.began.rawValue
            MainActor.assumeIsolated { if began { self?.interrupt() } }
        }
        center.addObserver(forName: AVAudioSession.routeChangeNotification, object: nil, queue: .main) { [weak self] note in
            let reason = (note.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt).flatMap(AVAudioSession.RouteChangeReason.init)
            let lost = reason == .oldDeviceUnavailable || reason == .newDeviceAvailable
            MainActor.assumeIsolated { if lost { self?.interrupt() } }
        }
        // The input changing under a take (another app, a different microphone) spoils it.
        center.addObserver(forName: .AVAudioEngineConfigurationChange, object: nil, queue: .main) { [weak self] _ in
            MainActor.assumeIsolated { if let self, self.phase == .waiting || self.phase == .hearing { self.interrupt() } }
        }
    }

    var isCapturing: Bool { [.cue, .waiting, .hearing].contains(phase) }
    var isBusy: Bool { phase != .idle }
    func isActive(_ key: String) -> Bool { activeKey == key && phase != .idle }
    func problem(for key: String) -> Problem? { problem?.key == key ? problem?.problem : nil }

    // MARK: Driving

    func begin(_ request: Request, voice: Voice, onTake: @escaping @MainActor (Take) -> Void) {
        cancel()
        session += 1
        let mine = session, era = epoch
        self.voice = voice
        activeKey = request.key
        problem = nil
        skipModel = false
        tapped = false
        phase = .preparing
        task = Task { await run(request, voice: voice, mine: mine, era: era, onTake: onTake) }
    }

    /// The one button: skips the model while it plays, ends the take while listening.
    /// false when nothing of `key` is in progress (so the caller starts a turn).
    func tap(_ key: String) -> Bool {
        guard activeKey == key, phase != .idle else { return false }
        switch phase {
        case .model:
            skipModel = true
            voice?.stop()
        case .ready:
            readiness?.resume()
            readiness = nil
        case .cue, .waiting, .hearing:
            tapped = true
            capture?.stopNow()
        case .idle, .preparing, .analysing:
            break
        }
        return true
    }

    /// Ends whatever is in progress before analysis; nothing is kept. Analysis already under
    /// way finishes and lands on its exercise.
    func cancelCapture() {
        guard phase != .analysing else { return }
        cancel()
    }

    func cancel() {
        guard phase != .idle || task != nil else { return }
        session += 1
        task?.cancel()
        task = nil
        if case .model = phase { voice?.stop() }
        readiness?.resume()
        readiness = nil
        capture?.shutdown()
        capture = nil
        phase = .idle
        level = 0
        activeKey = nil
    }

    /// Hard cancellation, for when the notebook itself is going away (a restore): nothing begun
    /// before this writes a take or a report into the folder, logs evidence or lands on an
    /// exercise, and the call returns only once the turn in progress has wound down, so no file
    /// is being written while the folder is swapped. A page turn is `cancel`, never this.
    func invalidate() async {
        epoch += 1
        let pending = task
        cancel()
        await pending?.value
    }

    private func interrupt() {
        guard let key = activeKey, phase != .analysing, phase != .idle, phase != .preparing, phase != .cue, phase != .ready else { return }
        cancel()
        problem = (key, .interrupted)
    }

    private func end(_ key: String, _ p: Problem?) {
        phase = .idle
        level = 0
        activeKey = nil
        capture = nil
        task = nil
        if let p { problem = (key, p) }
    }

    private func run(_ r: Request, voice: Voice, mine: Int, era: Int, onTake: @escaping @MainActor (Take) -> Void) async {
        let (mic, speech) = await SpeechCheck.permissions()
        guard session == mine else { return }
        recognitionDenied = !speech
        guard mic else { end(r.key, .micDenied); return }
        if speech { SpeechCheck.warmUp(with: r.model) }

        // Listen first: the model the take will be compared with (the reader's own sentence when
        // shadowing a narration), the text's clip or voice when there is none.
        for k in stride(from: 1, through: r.repeats, by: 1) {
            guard session == mine, !skipModel else { break }
            phase = .model(k, of: r.repeats)
            let watchdog = Task { try? await Task.sleep(for: .seconds(20)); if !Task.isCancelled { voice.stop() } }
            if let model = r.model { await voice.playFileToEnd(model, key: RussianText.audioKey(r.target)) } else { await voice.playToEnd(r.target) }
            watchdog.cancel()
            if k < r.repeats && !skipModel { try? await Task.sleep(for: .milliseconds(350)) }
        }
        guard session == mine else { return }

        // The learner is still listening: nothing opens until they say they are ready. A tap
        // that skipped the model is a skip, not a go. With no model, their tap began the turn.
        if r.repeats > 0 {
            voice.stop()
            phase = .ready
            await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in readiness = c }
            guard session == mine else { return }
        }

        // Your turn. Nothing else may be sounding into the microphone (a narration still reading).
        voice.stop()
        Voice.configureSession()
        let cap = TakeCapture(config: r.config ?? Self.config(target: r.target, model: r.model))
        let stream: AsyncStream<CaptureEvent>
        do { stream = try cap.start() } catch { end(r.key, .failed); return }
        capture = cap
        phase = .cue
        cap.cueThenListen()
        var ending: VoiceActivity.Ending?
        var listeningSince: Date?
        var onsetMs: Int?
        listening: for await e in stream {
            guard session == mine else { break }
            switch e {
            case .listening:
                if phase == .cue { phase = .waiting }
                listeningSince = listeningSince ?? Date()
            case .level(let l): level = l
            case .voice:
                phase = .hearing
                if onsetMs == nil, let since = listeningSince { onsetMs = Int(Date().timeIntervalSince(since) * 1000) }
            case .ended(let e): ending = e; break listening
            }
        }
        cap.shutdown()
        level = 0
        guard session == mine else { return }
        guard let ending, ending.speech != nil else { end(r.key, tapped ? nil : .nothingHeard); return }

        phase = .analysing
        let url = r.takeURL
        // From here on only the epoch matters: a page turn (`cancel`) lets the analysis land on
        // its exercise by id; a restore (`invalidate`) means the folder is no longer this notebook.
        guard epoch == era else { return }
        do {
            _ = try await Task.detached(priority: .userInitiated) { try cap.write(ending, to: url) }.value
        } catch { end(r.key, .failed); return }
        let flag = Flag()
        let recognise: @Sendable (URL, String) async -> String? = { u, hint in
            speech ? await SpeakingTurn.transcribe(u, hint: hint, timedOut: flag) : nil
        }
        let (report, heard) = await Self.analyse(take: url, target: r.target, model: r.model, primary: analyzer, recognise: recognise)
        guard epoch == era else { return }
        if let report { remember(report, for: url) }
        lastTimedOut = flag.isSet
        let superseded = session != mine
        if !superseded { end(r.key, nil) }
        onTake(Take(file: url, report: report, heard: heard, recognitionAllowed: speech, timedOut: flag.isSet, onsetMs: onsetMs))
    }

    /// How it sounded and what words it was, side by side: the phone-level analyzer (when the app
    /// has one) judges the sounds, and word recognition says whether the right words were said,
    /// which is what finishes the exercise. A phone-level report carries no transcript of its
    /// own, so the words are never read off it; the word-level analyzer (the fallback) is handed
    /// the words already heard rather than recognising twice. A report with no transcript gets
    /// the heard words, so the sidecar beside the take says both.
    nonisolated static func analyse(take url: URL, target: String, model: URL?, primary: (any PronunciationAnalyzer)?,
                                    recognise: @escaping @Sendable (URL, String) async -> String?) async -> (report: PronunciationReport?, heard: String?) {
        async let words = recognise(url, RussianText.unstressed(target))
        var report: PronunciationReport?
        if let primary { report = try? await primary.analyze(take: url, target: target, model: model) }
        let heard = await words
        if report == nil {
            report = try? await WordLevelAnalyzer(transcribe: { _, _ in heard }).analyze(take: url, target: target, model: model)
        }
        if report?.transcript == nil { report?.transcript = heard }
        return (report, heard)
    }

    /// Endpointing for this target: room for a long sentence, never less than six seconds.
    static func config(target: String, model: URL?) -> VoiceActivity.Config {
        var c = VoiceActivity.Config()
        let modelSeconds = model.flatMap { try? AVAudioFile(forReading: $0) }.map { Double($0.length) / $0.processingFormat.sampleRate }
            ?? Double(Syllables.split(target).count) * 0.3
        c.maxSpeech = min(15, max(6, modelSeconds * 2 + 1.5))
        return c
    }

    // MARK: Reports

    nonisolated static func reportURL(for take: URL) -> URL { take.deletingPathExtension().appendingPathExtension("json") }

    private func remember(_ report: PronunciationReport, for take: URL) {
        reports[take.lastPathComponent] = report
        if let data = try? JSONEncoder().encode(report) { try? data.write(to: Self.reportURL(for: take), options: .atomic) }
    }

    /// The report of a take, from this run or from beside the file.
    func report(for take: URL) -> PronunciationReport? {
        if let r = reports[take.lastPathComponent] { return r }
        guard let data = try? Data(contentsOf: Self.reportURL(for: take)),
              let r = try? JSONDecoder().decode(PronunciationReport.self, from: data) else { return nil }
        reports[take.lastPathComponent] = r
        return r
    }

    /// Analyses a take that has no report yet (recorded before reports existed), with what the
    /// recognizer heard then.
    func analyseEarlier(_ take: URL, target: String, model: URL?, heard: String?) async -> PronunciationReport? {
        if let r = report(for: take) { return r }
        let analyzer = WordLevelAnalyzer(transcribe: { _, _ in heard })
        guard let r = try? await analyzer.analyze(take: take, target: target, model: model) else { return nil }
        remember(r, for: take)
        return r
    }

    // MARK: Recognition

    final class Flag: @unchecked Sendable {
        private let lock = NSLock()
        private var value = false
        var isSet: Bool { lock.withLock { value } }
        func set() { lock.withLock { value = true } }
    }

    /// The recognizer, given ten seconds at most (the first on-device load can take minutes).
    nonisolated static func transcribe(_ url: URL, hint: String, timedOut: Flag) async -> String? {
        await SpeechCheck.cancelWarmUp()
        let job = Task.detached(priority: .userInitiated) { await SpeechCheck.transcribe(url, hint: hint) }
        let timer = Task.detached { try? await Task.sleep(for: .seconds(10)); if !Task.isCancelled { timedOut.set(); job.cancel() } }
        let heard = await job.value
        timer.cancel()
        return heard
    }

    // MARK: QA

    /// Holds the turn in a phase for screenshots (QA only).
    func pose(_ phase: Phase, key: String, level: Float = 0) {
        self.phase = phase
        activeKey = key
        self.level = level
    }
}
