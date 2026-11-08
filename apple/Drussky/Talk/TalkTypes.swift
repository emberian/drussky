import Foundation

/// A spoken conversation with the tutor (Talk/). Everything here is stored inside the exercise's
/// result (`ExerciseResult.talk`), so the whole dialogue lives with the page and its recordings;
/// every field is optional on decoding so older notebooks open.
struct TalkConversation: Codable, Sendable, Hashable {
    /// The three scenes the tutor proposed, kept so the page reads the same when reopened.
    var proposed: [TalkScene]? = nil
    var scene: TalkScene? = nil
    var lines: [TalkLine] = []
    /// How many turns the learner gets before the tutor says goodbye (6–10).
    var turns: Int = 8
    var started: Date? = nil
    var ended: Date? = nil
    /// When the red pen went over it.
    var marked: Date? = nil
    /// One warm sentence about the whole conversation.
    var praise: String? = nil
    /// The pattern behind the biggest slip of the conversation.
    var teach: [DiaryTeach]? = nil
    var model: String? = nil

    var learnerLines: [TalkLine] { lines.filter { $0.role == .learner } }
    var tutorLines: [TalkLine] { lines.filter { $0.role == .tutor } }
    var learnerTurns: Int { learnerLines.count }
    var isEnded: Bool { ended != nil }
    var isMarked: Bool { marked != nil }

    /// Plain-language line for the tutor's context on the page.
    static func describe(_ ex: Exercise, _ r: ExerciseResult) -> String {
        guard let t = r.talk else { return "Talk: a spoken conversation with the tutor, not started yet." }
        var s = "Talk: a spoken conversation" + (t.scene.map { " (\($0.title))" } ?? "") + " of \(t.learnerTurns) turn(s)."
        if let last = t.learnerLines.last, !last.ru.isEmpty { s += " Their last line, as heard: «\(last.ru)»." }
        if t.isMarked {
            let edits = t.learnerLines.reduce(0) { $0 + ($1.correction?.edits.count ?? 0) }
            s += edits == 0 ? " Marked: nothing to correct." : " Marked: \(edits) correction(s)."
        }
        return s
    }
}

/// Where the conversation happens: the tutor proposes three from the learner's words.
struct TalkScene: Codable, Sendable, Hashable, Identifiable {
    /// English title ("At breakfast").
    var title: String
    /// The same in Russian, stress-marked («За за́втраком»).
    var ru: String
    var id: String { title }
}

/// One line of the dialogue: the tutor's (spoken by the app) or the learner's (as heard).
struct TalkLine: Codable, Sendable, Hashable, Identifiable {
    enum Role: String, Codable, Sendable { case tutor, learner }
    var id: String = UUID().uuidString
    var role: Role
    /// Tutor: stress-marked Russian. Learner: what speech recognition heard ("" when nothing).
    var ru: String
    /// Tutor: the English whisper. Learner: nil.
    var en: String? = nil
    /// Tutor: the ≤2 new words this turn introduced.
    var newWords: [Course.Word]? = nil
    /// Learner: the recording's file name in voice/.
    var take: String? = nil
    /// Learner: typed instead of spoken.
    var typed: Bool? = nil
    /// Learner: the red pen, after the conversation.
    var correction: DiaryCorrection? = nil
    /// Learner: one pronunciation tip for this take, if any.
    var tip: String? = nil
    /// Tutor: how long the reply took (see `TalkLatency`).
    var latency: TalkLatency? = nil
    var at: Date

    /// The learner's line as the pen left it (or as heard).
    var corrected: String { correction?.corrected ?? ru }
}

/// Milliseconds from sending the turn to the first streamed token, to the first spoken
/// sentence, and to the end of the reply.
struct TalkLatency: Codable, Sendable, Hashable {
    var firstToken: Int?
    var firstSpeech: Int?
    var complete: Int?
}

/// The turn order of a conversation, with nothing async in it: who speaks next, when it ends,
/// and that a cancelled dialogue stays cancelled. `TalkSession` drives one of these.
struct TalkDialogue: Equatable, Sendable {
    enum Phase: Equatable, Sendable {
        case choosingScene
        case tutorTurn
        case learnerTurn
        case ended
        case cancelled
    }

    static let turnRange = 6...10

    private(set) var phase: Phase = .choosingScene
    private(set) var learnerTurns = 0
    let totalTurns: Int

    init(totalTurns: Int = 8) {
        self.totalTurns = min(max(totalTurns, Self.turnRange.lowerBound), Self.turnRange.upperBound)
    }

    /// Resumes a saved conversation where it left off.
    init(resuming c: TalkConversation) {
        self.init(totalTurns: c.turns)
        learnerTurns = c.learnerTurns
        if c.isEnded { phase = .ended }
        else if c.scene == nil { phase = .choosingScene }
        else if c.lines.last?.role == .tutor { phase = .learnerTurn }
        else { phase = .tutorTurn }
    }

    /// The tutor's next line must be the goodbye.
    var isClosing: Bool { learnerTurns >= totalTurns }
    var turnsLeft: Int { max(0, totalTurns - learnerTurns) }
    var isOver: Bool { phase == .ended || phase == .cancelled }

    mutating func choose() {
        guard phase == .choosingScene else { return }
        phase = .tutorTurn
    }

    /// The tutor spoke; `goodbye` when the reply ended the conversation (or it had to).
    mutating func tutorSpoke(goodbye: Bool) {
        guard phase == .tutorTurn else { return }
        phase = goodbye || isClosing ? .ended : .learnerTurn
    }

    mutating func learnerSpoke() {
        guard phase == .learnerTurn else { return }
        learnerTurns += 1
        phase = .tutorTurn
    }

    /// The learner ends it early: the tutor gets one last turn to say goodbye.
    mutating func endEarly() {
        guard !isOver else { return }
        phase = .tutorTurn
        learnerTurns = totalTurns
    }

    mutating func cancel() {
        guard phase != .ended else { return }
        phase = .cancelled
    }
}
