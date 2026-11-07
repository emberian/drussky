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
    /// Other speakers' recordings of the same word (variants.json); each tap plays the next voice.
    private var variants: [String: [String]] = [:]
    /// Personal-study recordings from PrivatePack/Audio (never in a release build).
    private var privateIndex: [String: String] = [:]
    private let privateDir = Bundle.main.url(forResource: "PrivatePack", withExtension: nil)?.appendingPathComponent("Audio")
    private var turn: [String: Int] = [:]
    /// Playback runs on its own queue: an audio-route stall (Bluetooth, a busy audio server)
    /// must never freeze the page. `player` is only touched on `audioQueue`.
    private let audioQueue = DispatchQueue(label: "software.ember.drussky.voice", qos: .userInitiated)
    private let deck = Deck()

    /// The current player; only touched on `audioQueue`.
    private final class Deck: @unchecked Sendable { var player: AVAudioPlayer? }
    private let synth = AVSpeechSynthesizer()
    private let audioDir: URL?

    override init() {
        audioDir = Bundle.main.url(forResource: "Audio", withExtension: nil)
        super.init()
        synth.delegate = self
        // index.json, then every index-*.json beside it (index-pairs.json …); earlier files win.
        for url in Self.audioTables(audioDir, prefix: "index") {
            if let data = try? Data(contentsOf: url), let map = try? JSONDecoder().decode([String: String].self, from: data) {
                index.merge(map) { first, _ in first }
            }
        }
        if let url = privateDir?.appendingPathComponent("index.json"), let data = try? Data(contentsOf: url),
           let map = try? JSONDecoder().decode([String: String].self, from: data) {
            privateIndex = map
        }
        for url in Self.audioTables(audioDir, prefix: "variants") {
            if let data = try? Data(contentsOf: url), let map = try? JSONDecoder().decode([String: [String]].self, from: data) {
                for (k, files) in map { variants[k, default: []] += files.filter { !(variants[k] ?? []).contains($0) } }
            }
        }
    }

    /// `<prefix>.json` first, then `<prefix>-*.json` in name order.
    private static func audioTables(_ dir: URL?, prefix: String) -> [URL] {
        guard let dir else { return [] }
        let names = (try? FileManager.default.contentsOfDirectory(atPath: dir.path)) ?? []
        let extra = names.filter { $0.hasPrefix(prefix + "-") && $0.hasSuffix(".json") }.sorted()
        return ([prefix + ".json"] + extra).map { dir.appendingPathComponent($0) }
    }

    /// Every speaker's recording of a pair word. With `exactStress` only clips filed under the
    /// stress-marked form count (за́мок ≠ замо́к); otherwise the plain word's clips join in.
    func pairVoices(_ form: String, exactStress: Bool) -> [URL] {
        var keys = [RussianText.pairKey(form)]
        if !exactStress { keys.append(RussianText.key(form)) }
        var files: [String] = []
        for k in keys {
            for f in (index[k].map { [$0] } ?? []) + (variants[k] ?? []) where !files.contains(f) { files.append(f) }
        }
        return files.compactMap { audioDir?.appendingPathComponent($0) }
    }

    static func configureSession() {
        let session = AVAudioSession.sharedInstance()
        try? session.setCategory(.playAndRecord, mode: .default,
                                 options: [.defaultToSpeaker, .allowBluetoothA2DP, .allowBluetoothHFP])
        try? session.setActive(true)
    }

    func hasRecording(_ text: String) -> Bool { index[RussianText.key(text)] != nil || privateIndex[RussianText.key(text)] != nil }
    func speakerCount(_ text: String) -> Int { index[RussianText.key(text)] == nil ? 0 : 1 + (variants[RussianText.key(text)]?.count ?? 0) }

    func recordingURL(for text: String) -> URL? {
        index[RussianText.key(text)].flatMap { audioDir?.appendingPathComponent($0) }
    }

    func play(_ text: String) {
        let key = RussianText.key(text)
        stop()
        speakingKey = key
        if let primary = index[key] {
            let voices = [primary] + (variants[key] ?? [])
            let n = turn[key, default: 0]
            turn[key] = n + 1
            if let url = audioDir?.appendingPathComponent(voices[n % voices.count]) {
                start(url)
                return
            }
        }
        if let file = privateIndex[key], let url = privateDir?.appendingPathComponent(file) {
            start(url)
            return
        }
        let utterance = AVSpeechUtterance(string: RussianText.unstressed(text))
        utterance.voice = Self.bestRussianVoice
        utterance.rate = AVSpeechUtteranceDefaultSpeechRate * 0.85
        synth.speak(utterance)
    }

    func playFile(_ url: URL, key: String) {
        stop()
        speakingKey = key
        start(url)
    }

    private func start(_ url: URL) {
        audioQueue.async { [weak self] in
            guard let self else { return }
            self.deck.player?.stop()
            guard let p = try? AVAudioPlayer(contentsOf: url) else {
                Task { @MainActor in self.speakingKey = nil }
                return
            }
            p.delegate = self
            self.deck.player = p
            p.play()
        }
    }

    func stop() {
        let deck = self.deck
        audioQueue.async {
            deck.player?.stop()
            deck.player = nil
        }
        if synth.isSpeaking { synth.stopSpeaking(at: .immediate) }
        speakingKey = nil
    }

    static let bestRussianVoice: AVSpeechSynthesisVoice? = {
        let voices = AVSpeechSynthesisVoice.speechVoices().filter { $0.language.hasPrefix("ru") }
        return voices.max { $0.quality.rawValue < $1.quality.rawValue } ?? AVSpeechSynthesisVoice(language: "ru-RU")
    }()

    nonisolated func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        let finished = ObjectIdentifier(player)
        audioQueue.async { [weak self] in
            // Only the current player's end clears the indicator (a replaced one also "finishes").
            guard let self, let current = self.deck.player, ObjectIdentifier(current) == finished else { return }
            Task { @MainActor in self.speakingKey = nil }
        }
    }

    nonisolated func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        Task { @MainActor in self.speakingKey = nil }
    }
}
