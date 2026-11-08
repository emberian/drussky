import AVFoundation
import Foundation
import Network
import Observation

/// Speaks the tutor's lines. The system voice is the first implementation; a narrator engine
/// slots in behind the same three calls.
@MainActor
protocol TutorVoice: AnyObject {
    /// What the learner is hearing, for Settings ("Milena · Premium").
    var name: String { get }
    /// Speaks `text` (stress-marked Russian) and returns when it has been said, stopped or
    /// replaced. Successive calls queue: the next sentence follows the one before.
    func speak(_ text: String) async
    func stop()
}

/// Which installed voice speaks Russian: the highest quality (Premium > Enhanced > default),
/// ru-RU before other Russian variants, then by name so the choice is stable.
enum RussianVoicePick {
    struct Candidate: Equatable {
        var language: String
        var quality: Int
        var name: String
    }

    static func best(_ voices: [Candidate]) -> Candidate? {
        voices.filter { $0.language.lowercased().hasPrefix("ru") }.max { a, b in
            if a.quality != b.quality { return a.quality < b.quality }
            let aRU = a.language.lowercased() == "ru-ru", bRU = b.language.lowercased() == "ru-ru"
            if aRU != bRU { return !aRU }
            return a.name > b.name
        }
    }

    static func best(_ voices: [AVSpeechSynthesisVoice]) -> AVSpeechSynthesisVoice? {
        let candidates = voices.map { Candidate(language: $0.language, quality: $0.quality.rawValue, name: $0.name) }
        guard let pick = best(candidates) else { return nil }
        return voices.first { $0.name == pick.name && $0.language == pick.language && $0.quality.rawValue == pick.quality }
    }

    static func qualityName(_ q: AVSpeechSynthesisVoiceQuality) -> String {
        switch q {
        case .premium: "Premium"
        case .enhanced: "Enhanced"
        default: "Standard"
        }
    }

    /// How to get a better one: shown in Settings.
    static let howToImprove = "A richer Russian voice can be downloaded on this device: Settings → Accessibility → Spoken Content → Voices → Russian, then choose a voice marked Enhanced or Premium. Drussky picks the best one installed."
}

/// The system voice, through its own synthesizer so it never fights the page's own speaker
/// buttons. Delegate callbacks arrive on the main thread.
@MainActor
final class SystemTutorVoice: NSObject, TutorVoice, AVSpeechSynthesizerDelegate {
    private let synth = AVSpeechSynthesizer()
    private let voice: AVSpeechSynthesisVoice?
    private var waiting: [ObjectIdentifier: CheckedContinuation<Void, Never>] = [:]
    /// A little slower than the default: the learner is a beginner.
    var rate: Float = AVSpeechUtteranceDefaultSpeechRate * 0.86

    override init() {
        voice = Voice.bestRussianVoice
        super.init()
        synth.delegate = self
    }

    var name: String {
        guard let voice else { return "No Russian voice installed" }
        return "\(voice.name) · \(RussianVoicePick.qualityName(voice.quality))"
    }

    func speak(_ text: String) async {
        let plain = RussianText.unstressed(text).trimmingCharacters(in: .whitespacesAndNewlines)
        guard !plain.isEmpty else { return }
        let u = AVSpeechUtterance(string: plain)
        u.voice = voice
        u.rate = rate
        u.postUtteranceDelay = 0.08
        await withCheckedContinuation { (c: CheckedContinuation<Void, Never>) in
            waiting[ObjectIdentifier(u)] = c
            synth.speak(u)
        }
    }

    func stop() {
        synth.stopSpeaking(at: .immediate)
        let all = waiting.values
        waiting = [:]
        for c in all { c.resume() }
    }

    private func finished(_ u: AVSpeechUtterance) {
        waiting.removeValue(forKey: ObjectIdentifier(u))?.resume()
    }

    nonisolated func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didFinish utterance: AVSpeechUtterance) {
        nonisolated(unsafe) let u = utterance
        Task { @MainActor in self.finished(u) }
    }

    nonisolated func speechSynthesizer(_ synthesizer: AVSpeechSynthesizer, didCancel utterance: AVSpeechUtterance) {
        nonisolated(unsafe) let u = utterance
        Task { @MainActor in self.finished(u) }
    }
}

/// A voice that says nothing and takes no time: tests and QA poses.
@MainActor
final class SilentTutorVoice: TutorVoice {
    private(set) var spoken: [String] = []
    var name: String { "Silent" }
    func speak(_ text: String) async { spoken.append(text) }
    func stop() {}
}

/// Whether the tutor can be reached at all: Talk is unavailable offline, with one line saying so.
@MainActor
@Observable
final class Reachability {
    static let shared = Reachability()
    private(set) var isOnline = true
    private let monitor = NWPathMonitor()

    private init() {
        monitor.pathUpdateHandler = { [weak self] path in
            let online = path.status == .satisfied
            Task { @MainActor in self?.isOnline = online }
        }
        monitor.start(queue: DispatchQueue(label: "software.ember.drussky.reachability"))
    }
}
