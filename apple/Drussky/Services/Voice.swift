import AVFoundation
import Foundation
import Observation

/// Plays Russian: a bundled recording when one exists for the text, otherwise the best
/// installed Russian system voice.
@MainActor
@Observable
final class Voice: NSObject, AVAudioPlayerDelegate, AVSpeechSynthesizerDelegate {
    private(set) var speakingKey: String?
    private var index: [String: String] = [:]
    private var player: AVAudioPlayer?
    private let synth = AVSpeechSynthesizer()
    private let audioDir: URL?

    override init() {
        audioDir = Bundle.main.url(forResource: "Audio", withExtension: nil)
        super.init()
        synth.delegate = self
        if let url = audioDir?.appendingPathComponent("index.json"),
           let data = try? Data(contentsOf: url),
           let map = try? JSONDecoder().decode([String: String].self, from: data) {
            index = map
        }
    }

    static func configureSession() {
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playAndRecord, mode: .default,
                                 options: [.defaultToSpeaker, .allowBluetoothA2DP, .allowBluetoothHFP])
        try? session.setActive(true)
    }

    func hasRecording(_ text: String) -> Bool { index[RussianText.key(text)] != nil }

    func recordingURL(for text: String) -> URL? {
        index[RussianText.key(text)].flatMap { audioDir?.appendingPathComponent($0) }
    }

    func play(_ text: String) {
        let key = RussianText.key(text)
        stop()
        speakingKey = key
        if let file = index[key], let url = audioDir?.appendingPathComponent(file),
           let p = try? AVAudioPlayer(contentsOf: url) {
            p.delegate = self
            player = p
            p.play()
            return
        }
        let utterance = AVSpeechUtterance(string: RussianText.unstressed(text))
        utterance.voice = Self.bestRussianVoice
        utterance.rate = AVSpeechUtteranceDefaultSpeechRate * 0.85
        synth.speak(utterance)
    }

    func playFile(_ url: URL, key: String) {
        stop()
        guard let p = try? AVAudioPlayer(contentsOf: url) else { return }
        speakingKey = key
        p.delegate = self
        player = p
        p.play()
    }

    func stop() {
        player?.stop()
        player = nil
        if synth.isSpeaking { synth.stopSpeaking(at: .immediate) }
        speakingKey = nil
    }

    static let bestRussianVoice: AVSpeechSynthesisVoice? = {
        let voices = AVSpeechSynthesisVoice.speechVoices().filter { $0.language.hasPrefix("ru") }
        return voices.max { $0.quality.rawValue < $1.quality.rawValue } ?? AVSpeechSynthesisVoice(language: "ru-RU")
    }()

    nonisolated func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        Task { @MainActor in self.speakingKey = nil }
    }

    nonisolated func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        Task { @MainActor in self.speakingKey = nil }
    }
}
