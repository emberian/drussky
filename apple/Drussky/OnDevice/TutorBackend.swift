import Foundation
import Network
import Observation

// Two tutors answer in Drussky: Claude over the network (Services/Tutor.swift) and, where the
// device has one, Apple's on-device model (OnDeviceSchemas.swift). `TutorRouter` picks per task;
// every answer says who gave it. The policy table below is what the benchmark in
// research/ONDEVICE_TUTOR.md supports, nothing more.

/// Who answered.
enum TutorProvenance: String, Codable, Sendable, Hashable {
    case claude, onDevice

    /// The small tag shown beside an answer.
    var label: String {
        switch self {
        case .claude: "Claude"
        case .onDevice: "on device"
        }
    }
}

/// The tutor jobs that have a cheap, frequent, offline shape.
enum TutorTask: String, CaseIterable, Sendable {
    /// One word or phrase in a passage or library text → a 1–4 word English meaning.
    case gloss
    /// "What does X mean / why this ending", one or two sentences.
    case quickAnswer
    /// A diary line → red-pen edits (DiaryCorrection).
    case diaryCorrection
    /// Goal-text forms → dictionary forms (Goal.LemmaEntry).
    case lemmatize
}

/// What the router knows when it decides.
struct TutorConditions: Sendable, Equatable {
    var onDeviceAvailable: Bool
    var online: Bool
    var hasKey: Bool

    /// The device right now.
    @MainActor static var live: TutorConditions {
        TutorConditions(onDeviceAvailable: OnDeviceBackend.isAvailable, online: Connectivity.shared.online,
                        hasKey: Keychain.apiKey?.isEmpty == false)
    }
}

enum TutorRoute: Equatable, Sendable {
    case onDevice
    case claude
    /// Nobody can answer; the text says why, in the learner's terms.
    case unavailable(String)
}

enum TutorRouter {
    /// Tasks the on-device model answers well enough to be asked before Claude. None, as of the
    /// 2026-11-08 benchmark (research/ONDEVICE_TUTOR.md): it explained 0 of 20 endings correctly,
    /// invented 6 diary edits in 10 lines, and lemmatized 12 of 40 forms.
    static let onDeviceFirst: Set<TutorTask> = []

    /// Tasks the on-device model may answer when Claude is out of reach (offline, or no key).
    /// Glosses: 40/50 right on unknown passage words in 0.8 s, the misses on names and function
    /// words; a labelled meaning beats a dash, and Claude is one tap away when back online.
    /// A wrong correction or lemma is worse than none, so the other tasks stay unavailable.
    static let onDeviceFallback: Set<TutorTask> = [.gloss]

    static func route(_ task: TutorTask, _ c: TutorConditions) -> TutorRoute {
        if onDeviceFirst.contains(task), c.onDeviceAvailable { return .onDevice }
        if c.hasKey, c.online { return .claude }
        if onDeviceFallback.contains(task), c.onDeviceAvailable { return .onDevice }
        guard c.hasKey else { return .unavailable(Tutor.Failure.noKey.errorDescription ?? "") }
        return .unavailable("You're offline. The tutor needs the network for this.")
    }

    /// The backend for a route, or nil when unavailable.
    static func backend(for route: TutorRoute) -> (any TutorBackend)? {
        switch route {
        case .onDevice: OnDeviceBackend()
        case .claude: ClaudeBackend()
        case .unavailable: nil
        }
    }
}

/// A meaning for one word, with who said so.
struct TutorGloss: Sendable, Hashable {
    var en: String
    var lemma: String?
    var provenance: TutorProvenance
}

/// One tutor. Both implementations speak the app's contracts: [[ru|en]] markup for prose,
/// DiaryCorrection and Goal.LemmaEntry for structured answers.
protocol TutorBackend: Sendable {
    var provenance: TutorProvenance { get }
    func gloss(word: String, context: String) async throws -> TutorGloss
    /// Prose with [[ру́сский|english]] markup, as TutorSheet renders it.
    func quickAnswer(question: String, context: String) async throws -> String
    /// `briefing` is DiaryTutor.prompt's learner summary (letters, words, constructions).
    func correctDiary(text: String, prompt: String, briefing: String, at date: Date) async throws -> DiaryCorrection
    func lemmatize(_ forms: [String]) async throws -> [String: Goal.LemmaEntry]
}

/// Is there a network path right now. One monitor for the app, read on the main actor.
@MainActor @Observable
final class Connectivity {
    static let shared = Connectivity()
    private(set) var online = true
    private let monitor = NWPathMonitor()

    private init() {
        monitor.pathUpdateHandler = { path in
            let up = path.status == .satisfied
            Task { @MainActor in Connectivity.shared.online = up }
        }
        monitor.start(queue: DispatchQueue(label: "software.ember.drussky.connectivity"))
    }
}
