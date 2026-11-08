import AVFoundation
import Observation
import Speech

/// Records the learner saying something and transcribes it with Russian speech recognition
/// (on-device when the device supports it).
///
/// Every start claims a new session before anything is awaited (permission prompts included),
/// so a second tap, a page turn or `cancel()` during the wait supersedes it and a late answer
/// is dropped. Each take is recorded to its own new file: nothing earlier is touched until the
/// caller commits the take. Recognition runs as a retained task that a timeout or cancel stops.
@MainActor
@Observable
final class SpeechCheck {
    enum Phase: Equatable { case idle, requesting, recording, listening, finished }
    enum Start: Equatable {
        case recording
        /// No microphone permission: nothing can be recorded.
        case micDenied
        /// Superseded by another start or a cancel while permission was being asked.
        case superseded
        case failed
    }
    /// One finished take: the new file, and what the recognizer heard (nil when it heard nothing
    /// or wasn't available).
    struct Take {
        var file: URL
        var heard: String?
        var timedOut: Bool
        var recognitionAllowed: Bool
    }

    private(set) var phase: Phase = .idle
    private(set) var level: Float = 0
    private(set) var activeKey: String?
    private var recorder: AVAudioRecorder?
    private var meter: Timer?
    private var session = 0
    private var recognition: Task<String?, Never>?
    private var recognitionAllowed = false

    var isRecording: Bool { phase == .recording }
    var isBusy: Bool { phase == .requesting || phase == .listening }
    /// True when the last check gave up waiting because the recognizer was still loading.
    private(set) var lastTimedOut = false
    /// Speech recognition is not allowed; recordings are kept and compared by ear.
    private(set) var recognitionDenied = false
    private static var warmed = false
    private static var warmTask: Task<Void, Never>?

    /// The first on-device Russian transcription loads a model and can take minutes; later ones
    /// take a fraction of a second. Run one silently on a bundled clip as soon as we're allowed.
    static func warmUp(with url: URL?) {
        guard !warmed, let url, SFSpeechRecognizer.authorizationStatus() == .authorized else { return }
        warmed = true
        warmTask = Task.detached(priority: .utility) { _ = await transcribe(url, hint: "") }
    }

    /// Starts recording a new take into `url` (a file that doesn't exist yet).
    func start(to url: URL, key: String) async -> Start {
        guard !isBusy else { return .superseded }
        cancel()
        session += 1
        let mine = session
        phase = .requesting
        activeKey = key
        let (mic, speech) = await Self.permissions()
        guard session == mine else { return .superseded }
        recognitionAllowed = speech
        recognitionDenied = !speech
        guard mic else { reset(); return .micDenied }
        stopMeter()
        Voice.configureSession()
        let settings: [String: Any] = [
            AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: 44_100,
            AVNumberOfChannelsKey: 1, AVEncoderAudioQualityKey: AVAudioQuality.high.rawValue,
        ]
        guard let r = try? AVAudioRecorder(url: url, settings: settings) else { reset(); return .failed }
        r.isMeteringEnabled = true
        guard r.record() else { try? FileManager.default.removeItem(at: url); reset(); return .failed }
        recorder = r
        phase = .recording
        meter = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self, let r = self.recorder else { return }
                r.updateMeters()
                self.level = max(0, min(1, (r.averagePower(forChannel: 0) + 50) / 50))
            }
        }
        return .recording
    }

    /// Stops recording and listens. nil when the take was cancelled or superseded meanwhile.
    func finish(target: String) async -> Take? {
        guard let r = recorder, phase == .recording else { return nil }
        let url = r.url
        r.stop()
        recorder = nil
        stopMeter()
        phase = .listening
        let mine = session
        var heard: String?
        var timedOut = false
        if recognitionAllowed {
            Self.warmTask?.cancel()
            let hint = RussianText.unstressed(target)
            let job = Task.detached(priority: .userInitiated) { await Self.transcribe(url, hint: hint) }
            recognition = job
            // Never keep the learner waiting on a recognizer that is still loading: ten seconds,
            // then the job itself is cancelled, not just abandoned.
            let timeout = Task { try? await Task.sleep(for: .seconds(10)); job.cancel() }
            heard = await job.value
            timedOut = timeout.isCancelled == false && job.isCancelled
            timeout.cancel()
            if recognition == job { recognition = nil }
        }
        guard session == mine else { return nil }
        lastTimedOut = timedOut
        phase = .finished
        activeKey = nil
        return Take(file: url, heard: heard, timedOut: timedOut, recognitionAllowed: recognitionAllowed)
    }

    /// Stops whatever is happening. A take still being recorded is discarded (its new file is
    /// deleted; nothing committed earlier is touched).
    func cancel() {
        session += 1
        if let r = recorder {
            r.stop()
            r.deleteRecording()
        }
        recorder = nil
        recognition?.cancel()
        recognition = nil
        reset()
    }

    private func reset() {
        stopMeter()
        phase = .idle
        activeKey = nil
    }

    private func stopMeter() { meter?.invalidate(); meter = nil; level = 0 }

    /// Microphone and speech recognition, asked separately: recording works without recognition.
    /// Nonisolated on purpose: the system answers permission requests on a background queue,
    /// and a main-actor closure there traps at runtime (Swift 6 isolation check).
    nonisolated static func permissions() async -> (mic: Bool, speech: Bool) {
        let mic = await AVAudioApplication.requestRecordPermission()
        let speech = await withCheckedContinuation { (c: CheckedContinuation<Bool, Never>) in
            SFSpeechRecognizer.requestAuthorization { @Sendable status in c.resume(returning: status == .authorized) }
        }
        return (mic, speech)
    }

    /// The running recognition task and its continuation, so cancellation from any thread both
    /// stops the recognizer and answers the waiting caller exactly once.
    private final class Handle: @unchecked Sendable {
        private let lock = NSLock()
        private var task: SFSpeechRecognitionTask?
        private var continuation: CheckedContinuation<String?, Never>?
        private var done = false

        func begin(_ c: CheckedContinuation<String?, Never>) -> Bool {
            lock.lock(); defer { lock.unlock() }
            if done { c.resume(returning: nil); return false }
            continuation = c
            return true
        }
        func set(_ t: SFSpeechRecognitionTask) {
            lock.lock(); defer { lock.unlock() }
            if done { t.cancel() } else { task = t }
        }
        func finish(_ value: String?) {
            lock.lock()
            guard !done else { lock.unlock(); return }
            done = true
            let c = continuation
            continuation = nil
            task = nil
            lock.unlock()
            c?.resume(returning: value)
        }
        func cancel() {
            lock.lock()
            let t = task
            lock.unlock()
            t?.cancel()
            finish(nil)
        }
    }

    /// Nonisolated, with a Sendable result handler: the recognizer may call back on any queue
    /// (on-device recognition does not always honour `queue`), and a main-actor closure there
    /// traps at runtime. Exactly one answer; cancelling the calling task cancels the
    /// recognition task and answers nil.
    nonisolated static func transcribe(_ url: URL, hint: String) async -> String? {
        guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "ru-RU")), recognizer.isAvailable else { return nil }
        let request = SFSpeechURLRecognitionRequest(url: url)
        request.contextualStrings = [hint]
        request.shouldReportPartialResults = false
        if recognizer.supportsOnDeviceRecognition { request.requiresOnDeviceRecognition = true }
        nonisolated(unsafe) let keepAlive = recognizer
        let handle = Handle()
        return await withTaskCancellationHandler {
            await withCheckedContinuation { (continuation: CheckedContinuation<String?, Never>) in
                guard handle.begin(continuation) else { return }
                let task = recognizer.recognitionTask(with: request) { @Sendable result, error in
                    _ = keepAlive
                    if let result, result.isFinal {
                        // Prefer an alternative that matches the target, else the best guess.
                        let all = result.transcriptions.map(\.formattedString)
                        handle.finish(all.first { RussianText.matches($0, hint) } ?? all.first)
                    } else if error != nil {
                        handle.finish(nil)
                    }
                }
                handle.set(task)
            }
        } onCancel: {
            handle.cancel()
        }
    }
}
