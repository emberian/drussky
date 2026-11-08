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

    // Recordings are found by `RussianText.audioKey`, which keeps a rising question apart from
    // its statement. With no recording of the right intonation, the system voice reads the text
    // with its punctuation rather than a clip with the wrong contour.
    func hasRecording(_ text: String) -> Bool { index[RussianText.audioKey(text)] != nil || privateIndex[RussianText.audioKey(text)] != nil }
    func speakerCount(_ text: String) -> Int { index[RussianText.audioKey(text)] == nil ? 0 : 1 + (variants[RussianText.audioKey(text)]?.count ?? 0) }

    func recordingURL(for text: String) -> URL? {
        index[RussianText.audioKey(text)].flatMap { audioDir?.appendingPathComponent($0) }
    }

    func play(_ text: String) {
        let key = RussianText.audioKey(text)
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

    /// Starts `url` and returns the token of the sequence it begins (see `continueSequence`).
    @discardableResult
    func playFile(_ url: URL, key: String) -> Int {
        stop()
        speakingKey = key
        start(url)
        return sequence
    }

    /// Bumped by every new playback and by `stop()`. A multi-clip sequence (hear one word, then
    /// its pair) holds the token it began with and plays its next clip only while that is still
    /// current: anything played or stopped in between (the next trial, a page turn) ends it.
    private(set) var sequence = 0

    /// Plays the next clip of the sequence begun as `token`; false (and silent) if it has ended.
    @discardableResult
    func continueSequence(_ url: URL, key: String, token: Int) -> Bool {
        guard token == sequence else { return false }
        speakingKey = key
        start(url)
        return true
    }

    /// Plays `text` and returns once it has finished, been stopped, or been replaced.
    func playToEnd(_ text: String) async {
        play(text)
        await untilDone(sequence)
    }

    /// Plays `url` and returns once it has finished, been stopped, or been replaced; false if
    /// it was stopped or replaced (so a sequence knows to end).
    @discardableResult
    func playFileToEnd(_ url: URL, key: String) async -> Bool {
        let token = playFile(url, key: key)
        await untilDone(token)
        return sequence == token
    }

    private func untilDone(_ token: Int) async {
        while speakingKey != nil && sequence == token {
            await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in
                withObservationTracking { _ = self.speakingKey; _ = self.sequence } onChange: { c.resume() }
            }
        }
    }

    /// Plays `from…to` seconds of `url`: one syllable of the model or of a take.
    func playSegment(_ url: URL, key: String, from: Double, to: Double) {
        stop()
        speakingKey = key
        let token = sequence
        let length = max(0.06, to - from)
        audioQueue.async { [weak self] in
            guard let self else { return }
            self.deck.player?.stop()
            guard let p = try? AVAudioPlayer(contentsOf: url) else {
                Task { @MainActor in self.speakingKey = nil }
                return
            }
            p.delegate = self
            self.deck.player = p
            p.currentTime = max(0, from)
            p.play()
            let mine = ObjectIdentifier(p)
            self.audioQueue.asyncAfter(deadline: .now() + length) {
                guard let current = self.deck.player, ObjectIdentifier(current) == mine else { return }
                current.stop()
                Task { @MainActor in if self.sequence == token { self.speakingKey = nil } }
            }
        }
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
        sequence += 1
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
        return RussianVoicePick.best(voices) ?? AVSpeechSynthesisVoice(language: "ru-RU")
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
