import AVFoundation
import Observation
import Speech

/// Records the learner saying something and transcribes it with Russian speech recognition
/// (on-device when the device supports it).
@MainActor
@Observable
final class SpeechCheck {
    enum Phase: Equatable { case idle, recording, listening, finished }
    private(set) var phase: Phase = .idle
    private(set) var level: Float = 0
    private(set) var activeKey: String?
    private var recorder: AVAudioRecorder?
    private var meter: Timer?
    private var session = 0

    var isRecording: Bool { phase == .recording }
    /// True when the last check gave up waiting because the recognizer was still loading.
    private(set) var lastTimedOut = false
    private static var warmed = false

    /// The first on-device Russian transcription loads a model and can take minutes; later ones
    /// take a fraction of a second. Run one silently on a bundled clip as soon as we're allowed.
    static func warmUp(with url: URL?) {
        guard !warmed, let url, SFSpeechRecognizer.authorizationStatus() == .authorized else { return }
        warmed = true
        Task.detached(priority: .utility) { _ = await transcribe(url, hint: "") }
    }

    func start(to url: URL, key: String) async -> Bool {
        guard phase != .listening, await Self.permissions() else { return false }
        session += 1
        stopMeter()
        Voice.configureSession()
        let settings: [String: Any] = [
            AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: 44_100,
            AVNumberOfChannelsKey: 1, AVEncoderAudioQualityKey: AVAudioQuality.high.rawValue,
        ]
        try? FileManager.default.removeItem(at: url)
        guard let r = try? AVAudioRecorder(url: url, settings: settings) else { return false }
        r.isMeteringEnabled = true
        guard r.record() else { return false }
        recorder = r
        activeKey = key
        phase = .recording
        meter = Timer.scheduledTimer(withTimeInterval: 0.05, repeats: true) { [weak self] _ in
            Task { @MainActor in
                guard let self, let r = self.recorder else { return }
                r.updateMeters()
                self.level = max(0, min(1, (r.averagePower(forChannel: 0) + 50) / 50))
            }
        }
        return true
    }

    /// Stops recording and returns what was heard (nil when nothing was recognized).
    func finish(target: String) async -> String? {
        guard let r = recorder else { return nil }
        let url = r.url
        r.stop()
        recorder = nil
        stopMeter()
        phase = .listening
        let mine = session
        let hint = RussianText.unstressed(target)
        // Never keep the learner waiting on a recognizer that is still loading.
        // A race that resumes exactly once: the transcription, or ten seconds.
        let heard: String?? = await withCheckedContinuation { (c: CheckedContinuation<String??, Never>) in
            let once = Once()
            Task { let r = await Self.transcribe(url, hint: hint); if once.claim() { c.resume(returning: .some(r)) } }
            Task { try? await Task.sleep(for: .seconds(10)); if once.claim() { c.resume(returning: .none) } }
        }
        lastTimedOut = heard == nil
        if session == mine {
            phase = .finished
            activeKey = nil
        }
        return heard ?? nil
    }

    func cancel() {
        session += 1
        recorder?.stop()
        recorder = nil
        stopMeter()
        phase = .idle
        activeKey = nil
    }

    private func stopMeter() { meter?.invalidate(); meter = nil; level = 0 }

    /// Nonisolated on purpose: the system answers permission requests on a background queue,
    /// and a main-actor closure there traps at runtime (Swift 6 isolation check).
    nonisolated static func permissions() async -> Bool {
        let mic = await AVAudioApplication.requestRecordPermission()
        let speech = await withCheckedContinuation { (c: CheckedContinuation<Bool, Never>) in
            SFSpeechRecognizer.requestAuthorization { @Sendable status in c.resume(returning: status == .authorized) }
        }
        return mic && speech
    }

    /// Nonisolated, with a Sendable result handler: the recognizer may call back on any queue
    /// (on-device recognition does not always honour `queue`), and a main-actor closure there
    /// traps at runtime. Exactly one resume, guarded by `Once`.
    nonisolated static func transcribe(_ url: URL, hint: String) async -> String? {
        guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "ru-RU")), recognizer.isAvailable else { return nil }
        let request = SFSpeechURLRecognitionRequest(url: url)
        request.contextualStrings = [hint]
        request.shouldReportPartialResults = false
        if recognizer.supportsOnDeviceRecognition { request.requiresOnDeviceRecognition = true }
        nonisolated(unsafe) let keepAlive = recognizer
        return await withCheckedContinuation { (continuation: CheckedContinuation<String?, Never>) in
            let once = Once()
            recognizer.recognitionTask(with: request) { @Sendable result, error in
                _ = keepAlive
                if let result, result.isFinal {
                    // Prefer an alternative that matches the target, else the best guess.
                    let all = result.transcriptions.map(\.formattedString)
                    let best = all.first { RussianText.matches($0, hint) } ?? all.first
                    if once.claim() { continuation.resume(returning: best) }
                } else if error != nil {
                    if once.claim() { continuation.resume(returning: nil) }
                }
            }
        }
    }
}
