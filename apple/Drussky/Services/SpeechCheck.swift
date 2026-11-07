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

    var isRecording: Bool { phase == .recording }

    func start(to url: URL, key: String) async -> Bool {
        guard await Self.permissions() else { return false }
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
        let heard = await Self.transcribe(url, hint: RussianText.unstressed(target))
        phase = .finished
        activeKey = nil
        return heard
    }

    func cancel() {
        recorder?.stop()
        recorder = nil
        stopMeter()
        phase = .idle
        activeKey = nil
    }

    private func stopMeter() { meter?.invalidate(); meter = nil; level = 0 }

    static func permissions() async -> Bool {
        let mic = await AVAudioApplication.requestRecordPermission()
        let speech = await withCheckedContinuation { c in
            SFSpeechRecognizer.requestAuthorization { c.resume(returning: $0 == .authorized) }
        }
        return mic && speech
    }

    static func transcribe(_ url: URL, hint: String) async -> String? {
        guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "ru-RU")), recognizer.isAvailable else { return nil }
        let request = SFSpeechURLRecognitionRequest(url: url)
        request.contextualStrings = [hint]
        request.shouldReportPartialResults = false
        if recognizer.supportsOnDeviceRecognition { request.requiresOnDeviceRecognition = true }
        return await withCheckedContinuation { continuation in
            var resumed = false
            recognizer.recognitionTask(with: request) { result, error in
                guard !resumed else { return }
                if let result, result.isFinal {
                    resumed = true
                    // Prefer an alternative that matches the target, else the best guess.
                    let all = result.transcriptions.map(\.formattedString)
                    continuation.resume(returning: all.first { RussianText.matches($0, hint) } ?? all.first)
                } else if error != nil {
                    resumed = true
                    continuation.resume(returning: nil)
                }
            }
        }
    }
}
