import Foundation
import SwiftUI

/// The listening strand (`Resources/listening.json`, COURSE_SCHEMA.md "Listening"): twelve sets
/// of short heard situations in seven kinds, each set meant to follow one course page, and six
/// reserved sets of five unheard messages for the weekly sample. Decoded as the authoring
/// contract writes it (zero-based indices; a field a kind does not use is absent, so every
/// kind-specific field is Optional); unknown fields are ignored.
struct Listening: Codable, Sendable {
    var version: Int
    var sets: [ListeningSet]
    var weekly_samples: [ListeningSet]

    static let shared = load()

    static func load(bundle: Bundle = .main) -> Listening {
        guard let url = bundle.url(forResource: "listening", withExtension: "json"), let data = try? Data(contentsOf: url),
              let l = try? JSONDecoder().decode(Listening.self, from: data) else {
            return Listening(version: 1, sets: [], weekly_samples: [])
        }
        return l
    }

    /// The practice set at 1-based `number`, in file order.
    func set(number: Int) -> ListeningSet? { sets[safe: number - 1] }
    func weeklySet(number: Int) -> ListeningSet? { weekly_samples[safe: number - 1] }

    /// Every exact string the strand can play: main turns, both intonation forms, scene labels,
    /// object choices and the crucial spans revealed after an answer (the audio inventory,
    /// research/artifacts/listening-spoken.txt, is written from the same rule by validate_listening.py).
    var spokenStrings: [String] {
        var seen = Set<String>(), out: [String] = []
        for s in (sets + weekly_samples).flatMap(\.items) { for t in s.spokenStrings where seen.insert(t).inserted { out.append(t) } }
        return out
    }
}

struct ListeningSet: Codable, Sendable, Hashable, Identifiable {
    var id: String
    var title: String
    var title_en: String
    var after_page: String
    var items: [ListeningItem]
}

struct ListeningItem: Codable, Sendable, Hashable, Identifiable {
    enum Kind: String, Codable, Sendable {
        case pickObject = "pick_object"
        case whoNeeds = "who_needs"
        case follow
        case questionOrStatement = "question_or_statement"
        case message
        case correction
        case disagreement

        /// Two speakers, played in turn with two voices.
        var isExchange: Bool { self == .whoNeeds || self == .disagreement }
    }

    var id: String
    var kind: Kind
    /// Translation of what is played (A/B-labelled lines for an exchange; the played form for
    /// intonation), hidden until answered.
    var en: String
    var after_page: String
    /// `any` (a familiar course voice will do) or `unfamiliar` (a voice not heard on course pages).
    var voice_hint: String
    /// Lowercase unstressed surface form → meaning in context; may be empty.
    var glosses: [String: String]
    /// The exact stressed stretch of the heard turn that decides the answer, and why.
    var crucial_span: String
    var why: String

    // pick_object, follow, message, correction
    var ru: String? = nil
    // every kind but message
    var question: String? = nil
    var options: [String]? = nil
    var answer: Int? = nil
    // who_needs, disagreement
    var lines: [Line]? = nil
    // follow
    var scene: [SceneEntity]? = nil
    var outcomes: [Outcome]? = nil
    // question_or_statement
    var forms: [Form]? = nil
    var played: Int? = nil
    // message
    var questions: [Question]? = nil

    struct Line: Codable, Sendable, Hashable {
        var speaker: String
        var ru: String
        var en: String
    }

    struct SceneEntity: Codable, Sendable, Hashable, Identifiable {
        var id: String
        var ru: String
        var en: String
        /// `object` (movable) or `place` (a destination).
        var role: String
    }

    /// One object's resulting location; `place` nil (an explicit null) means it stays where it is.
    struct Outcome: Codable, Sendable, Hashable {
        var object: String
        var place: String?
    }

    struct Form: Codable, Sendable, Hashable {
        var ru: String
        var en: String
    }

    /// A question about a message: a practice question has `options`/`answer`; a weekly one has
    /// `response_mode` (`english_text`), `answer_en` and `accept_en` instead.
    struct Question: Codable, Sendable, Hashable {
        var kind: String
        var prompt: String
        var options: [String]? = nil
        var answer: Int? = nil
        var crucial_span: String
        var why: String
        var response_mode: String? = nil
        var answer_en: String? = nil
        var accept_en: [String]? = nil
    }

    /// What is played before the answer: the turn, the lines in order, or the chosen form.
    var heard: [String] {
        switch kind {
        case .whoNeeds, .disagreement: (lines ?? []).map(\.ru)
        case .questionOrStatement: [forms?[safe: played ?? 0]?.ru].compactMap { $0 }
        default: [ru].compactMap { $0 }
        }
    }

    /// The heard text as one string, lines on their own lines (the transcript shown afterwards).
    var transcript: String { heard.joined(separator: "\n") }

    var spokenStrings: [String] {
        var out = heard
        out += (forms ?? []).map(\.ru)
        out += (scene ?? []).map(\.ru)
        if kind == .pickObject { out += options ?? [] }
        out.append(crucial_span)
        out += (questions ?? []).map(\.crucial_span)
        return out
    }

    /// How many choices the learner makes: one, or one per message question.
    var decisionCount: Int { kind == .message ? (questions ?? []).count : 1 }

    /// The right choice for decision `i`.
    func answer(_ i: Int) -> Int? { kind == .message ? questions?[safe: i]?.answer : answer }
    func options(_ i: Int) -> [String] { (kind == .message ? questions?[safe: i]?.options : options) ?? [] }
    /// The span and reason to show for decision `i` (a message's own question's; the item's otherwise).
    func reveal(_ i: Int) -> (span: String, why: String) {
        if kind == .message, let q = questions?[safe: i] { return (q.crucial_span, q.why) }
        return (crucial_span, why)
    }

    /// Which voice says a turn. `any` keeps the course voice (xenia); `unfamiliar` moves to aidar.
    /// An exchange always has two voices so A and B are audibly two people: xenia/aidar, or
    /// aidar/baya when the item asks for unfamiliar ones. Mirrored by scripts/make_listening_audio.py.
    func voice(forLine i: Int = 0) -> String {
        let unfamiliar = voice_hint == "unfamiliar"
        guard kind.isExchange else { return unfamiliar ? "aidar" : "xenia" }
        let speaker = lines?[safe: i]?.speaker ?? "A"
        return speaker == "A" ? (unfamiliar ? "aidar" : "xenia") : (unfamiliar ? "baya" : "aidar")
    }
}

// MARK: - Pages

extension Listening {
    /// A listening set as a page: one `listening` exercise per item, in an order that is fixed for
    /// the set but is not the authored one (the first set alternates statement and question;
    /// that alternation must not be a cue).
    static func page(for set: ListeningSet, number: Int) -> Shelves.Page {
        let exercises = ordered(set.items).map { item -> Exercise in
            var e = Exercise(kind: .listening)
            e.listening = item
            e.glosses = item.glosses.isEmpty ? nil : item.glosses
            return e
        }
        // One line: the English title. The how-to is said once, on the first set, never again.
        let howTo = number == 1 ? "Each card is something said once: nothing plays until you tap Play, and the words appear after you answer." : ""
        return Shelves.Page(title: set.title,
                            intro: Shelves.intro(set.title_en, howTo),
                            unitTitle: place(number),
                            exercises: exercises,
                            sourceID: sourceID(set))
    }

    static func place(_ number: Int) -> String { "На слух · \(number)" }
    static func sourceID(_ set: ListeningSet) -> String { "listening:\(set.id)" }

    /// A stable order from the item ids (FNV-1a; `hashValue` changes per launch). If the
    /// asking-or-telling cards still come out strictly alternating statement and question, the
    /// last two of them trade places: the pattern must not answer the card.
    static func ordered(_ items: [ListeningItem]) -> [ListeningItem] {
        func fnv(_ s: String) -> UInt64 {
            var h: UInt64 = 0xcbf29ce484222325
            for b in s.utf8 { h ^= UInt64(b); h = h &* 0x100000001b3 }
            return h
        }
        var out = items.enumerated().sorted { (fnv($0.element.id), $0.offset) < (fnv($1.element.id), $1.offset) }.map(\.element)
        let trials = out.indices.filter { out[$0].kind == .questionOrStatement }
        let played = trials.map { out[$0].played ?? 0 }
        let alternates = played.count >= 4 && zip(played, played.dropFirst()).allSatisfy { $0 != $1 }
        if alternates, let a = trials.dropLast().last, let b = trials.last { out.swapAt(a, b) }
        return out
    }

    /// The transcript with its stressed vowels marked and the crucial span underlined in red pen.
    static func highlighted(_ text: String, span: String, showStress: Bool) -> AttributedString {
        guard let r = text.range(of: span) else { return Theme.stressed(text, show: showStress) }
        var out = Theme.stressed(String(text[..<r.lowerBound]), show: showStress)
        var mid = Theme.stressed(String(text[r]), show: showStress)
        mid.underlineStyle = nil  // the stress marks' thin underline gives way to the span's
        mid.swiftUI.underlineStyle = Text.LineStyle(pattern: .solid, color: Theme.redPen)
        out += mid
        out += Theme.stressed(String(text[r.upperBound...]), show: showStress)
        return out
    }
}
