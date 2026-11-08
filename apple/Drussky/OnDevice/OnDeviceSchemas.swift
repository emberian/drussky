#if canImport(FoundationModels)
import Foundation
import FoundationModels

// The on-device tutor's structured outputs and prompts. This file depends on FoundationModels
// only (no app types), so the benchmark CLI in research/ondevice/ compiles it as is: what is
// measured there is what ships here. OnDeviceBackend.swift maps these onto the app's contracts.

@available(iOS 26.0, macOS 26.0, *)
enum OnDevicePrompts {
    /// Shared with every session: who the learner is, how to write Russian.
    static let persona = TutorIdentity.onDevicePersona

    static let gloss = persona + """

    Task: give the meaning of ONE Russian word as it is used in the sentence shown. The gloss is 1–4 English \
    words for the meaning in this context (for a noun after a preposition, the bare meaning is enough). \
    The lemma is the word's dictionary form in Cyrillic: nominative singular for nouns and adjectives, \
    infinitive for verbs.
    """

    static let quick = persona + """

    Task: answer the learner's quick question about the Russian in front of them in ONE or TWO sentences. \
    If they ask why a word has a certain ending, name the case or verb form and the reason for it \
    (preposition, role in the sentence, agreement). Mark every Russian word or phrase you write as \
    [[ру́сский|english]] so the learner can tap it. No preamble.
    """

    static let diary = persona + """

    Task: the learner wrote a few words of their OWN Russian in their diary. Correct it like a kind, exact \
    teacher marking homework in red pen. Keep their meaning and their words wherever they are right; fix only \
    what is wrong. Every edit's span must be a verbatim word or phrase from their text; edits must not overlap. \
    If the text is already correct, return no edits and no teach items. At most two teach items.
    """

    static let lemmas = persona + """

    Task: you receive Russian word forms, one per line, lowercase, without stress marks. For each form give \
    its dictionary form (nominative singular for nouns and adjectives, masculine for adjectives; infinitive \
    for verbs including participles and gerunds) and a 1–4 word English gloss. Return one row per input form, \
    in the same order, with the form copied exactly.
    """

    static func glossPrompt(word: String, context: String) -> String {
        "Sentence: \(context)\nWord: \(word)"
    }

    static func quickPrompt(question: String, context: String) -> String {
        "Where the learner is:\n\(context)\n\nQuestion: \(question)"
    }

    static func diaryPrompt(text: String, prompt: String, briefing: String) -> String {
        """
        \(briefing)
        The diary prompt was: “\(prompt)”
        They wrote:
        ---
        \(text)
        ---
        """
    }

    static func lemmasPrompt(_ forms: [String]) -> String { forms.joined(separator: "\n") }
}

@available(iOS 26.0, macOS 26.0, *)
@Generable(description: "The meaning of one Russian word in its sentence.")
struct OnDeviceGloss: Sendable {
    @Guide(description: "1–4 English words: what the word means in this sentence")
    var gloss: String
    @Guide(description: "The word's dictionary form, in Cyrillic")
    var lemma: String
}

@available(iOS 26.0, macOS 26.0, *)
@Generable(description: "A red-pen correction of a learner's short Russian text.")
struct OnDeviceDiaryCorrection: Sendable {
    @Generable(description: "One marked slip.")
    struct Edit: Sendable {
        @Guide(description: "The learner's words exactly as written, a word or a few words")
        var span: String
        @Guide(description: "The right form; empty to delete")
        var replacement: String
        var category: Category
        @Guide(description: "One line in English: what differs and why")
        var why: String
        @Guide(description: "English meaning of the replacement")
        var en: String
    }

    @Generable(description: "What kind of slip it is.")
    enum Category: String, Sendable {
        case spelling, ending, agreement, wordChoice, wordOrder, stress, punctuation, other
    }

    @Generable(description: "The pattern behind the biggest slip.")
    struct Teach: Sendable {
        @Guide(description: "The pattern, in 3–6 words")
        var title: String
        @Guide(description: "At most two sentences teaching it, with one Russian example")
        var note: String
    }

    @Guide(description: "The whole text, corrected, keeping every word that was already right")
    var corrected: String
    @Guide(description: "The slips, in text order; empty when the text is correct", .maximumCount(8))
    var edits: [Edit]
    @Guide(description: "One specific, warm sentence about what they got right")
    var praise: String
    @Guide(description: "0–2 items", .maximumCount(2))
    var teach: [Teach]
}

@available(iOS 26.0, macOS 26.0, *)
@Generable(description: "Dictionary forms for a list of Russian word forms.")
struct OnDeviceLemmaTable: Sendable {
    @Generable(description: "One input form with its dictionary form.")
    struct Row: Sendable {
        @Guide(description: "The input form, copied exactly")
        var form: String
        @Guide(description: "Dictionary form in Cyrillic (ё written where it belongs)")
        var lemma: String
        @Guide(description: "1–4 English words")
        var gloss: String
    }
    var rows: [Row]
}

/// One place that talks to the model, so the app and the benchmark ask identically.
@available(iOS 26.0, macOS 26.0, *)
enum OnDeviceModel {
    static var availability: SystemLanguageModel.Availability { SystemLanguageModel.default.availability }
    static var isAvailable: Bool { if case .available = availability { return true } else { return false } }

    /// Deterministic answers: a tutor should say the same thing twice.
    static let options = GenerationOptions(samplingMode: .greedy, maximumResponseTokens: 600)

    static func gloss(word: String, context: String) async throws -> OnDeviceGloss {
        let session = LanguageModelSession(instructions: OnDevicePrompts.gloss)
        return try await session.respond(to: OnDevicePrompts.glossPrompt(word: word, context: context),
                                         generating: OnDeviceGloss.self, options: options).content
    }

    static func quickAnswer(question: String, context: String) async throws -> String {
        let session = LanguageModelSession(instructions: OnDevicePrompts.quick)
        return try await session.respond(to: OnDevicePrompts.quickPrompt(question: question, context: context),
                                         options: options).content
    }

    static func correctDiary(text: String, prompt: String, briefing: String) async throws -> OnDeviceDiaryCorrection {
        let session = LanguageModelSession(instructions: OnDevicePrompts.diary)
        return try await session.respond(to: OnDevicePrompts.diaryPrompt(text: text, prompt: prompt, briefing: briefing),
                                         generating: OnDeviceDiaryCorrection.self, options: options).content
    }

    static func lemmatize(_ forms: [String]) async throws -> OnDeviceLemmaTable {
        let session = LanguageModelSession(instructions: OnDevicePrompts.lemmas)
        return try await session.respond(to: OnDevicePrompts.lemmasPrompt(forms),
                                         generating: OnDeviceLemmaTable.self, options: options).content
    }
}
#endif
