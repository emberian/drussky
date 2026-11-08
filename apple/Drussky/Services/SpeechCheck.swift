import AVFoundation
import Speech

/// Russian speech recognition (on-device when the device supports it) and the permissions a
/// spoken take needs. Recording itself is `TakeCapture`; the turn around it is `SpeakingTurn`.
enum SpeechCheck {
    @MainActor private static var warmed = false
    @MainActor private static var warmTask: Task<Void, Never>?

    /// The first on-device Russian transcription loads a model and can take minutes; later ones
    /// take a fraction of a second. Run one silently on a bundled clip as soon as we're allowed.
    @MainActor static func warmUp(with url: URL?) {
        guard !warmed, let url, SFSpeechRecognizer.authorizationStatus() == .authorized else { return }
        warmed = true
        warmTask = Task.detached(priority: .utility) { _ = await transcribe(url, hint: "") }
    }

    /// A real check is about to run: the silent warm-up gives way.
    @MainActor static func cancelWarmUp() { warmTask?.cancel() }

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
