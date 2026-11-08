import Foundation

/// The authored course, decoded from `course.json` (see apple/COURSE_SCHEMA.md).
struct Course: Codable, Sendable {
    var version: Int
    var letters: [Letter]
    var lexicon: [Word]
    var units: [Unit]

    struct Letter: Codable, Sendable, Hashable, Identifiable {
        var upper: String
        var lower: String
        var say: String
        var sound: String
        var family: String
        var tip: String
        var example: String?
        var id: String { upper }
    }

    struct Word: Codable, Sendable, Hashable {
        var ru: String
        var en: String
        var emoji: String?
        var note: String?
    }

    struct Unit: Codable, Sendable, Identifiable {
        var id: String
        var title: String
        var subtitle: String?
        var pages: [Page]
    }

    struct Page: Codable, Sendable, Identifiable {
        var id: String
        var title: String
        var intro: String?
        var exercises: [Exercise]
    }

    struct Example: Codable, Sendable, Hashable {
        var ru: String
        var en: String
    }

    func letter(_ upper: String) -> Letter? {
        letters.first { $0.upper == upper || $0.lower == upper }
    }

    func word(_ ru: String) -> Word? {
        let key = RussianText.key(ru)
        return lexicon.first { RussianText.key($0.ru) == key }
    }

    /// Flattened, ordered list of authored pages with their unit.
    var pageSequence: [(unit: Unit, page: Page)] {
        units.flatMap { unit in unit.pages.map { (unit, $0) } }
    }
}

enum ExerciseKind: String, Codable, Sendable {
    case meet, explain, trace, copy
    case listenPick = "listen_pick"
    case dictation
    case readAloud = "read_aloud"
    case translate, fill
    case readMeaning = "read_meaning"
    case passage
    /// Generated minimal-pair listening (SoundPairs.swift); never authored in course.json.
    case soundPair = "sound_pair"
    /// Free writing in the learner's own Russian, corrected by the tutor in red pen (Diary/).
    case diary
    /// Word building from morpheme tiles and guessing a new family member (Morphology/).
    case buildWord = "build_word"
    case familyGuess = "family_guess"
    /// Grammar as a system (Grammar/): a table with cells to fill, one form from a cue, phrases
    /// sorted by the case or pattern they show.
    case paradigmFill = "paradigm_fill"
    case transform
    case cueSort = "cue_sort"

    /// Exercises answered with the pencil on ruled paper.
    var isWriting: Bool {
        switch self {
        case .meet, .trace, .copy, .dictation, .translate, .fill, .transform: true
        default: false
        }
    }
}

struct Exercise: Codable, Sendable, Hashable {
    var kind: ExerciseKind
    var letter: String? = nil
    var text: String? = nil
    var examples: [Course.Example]? = nil
    var times: Int? = nil
    var say: String? = nil
    var options: [String]? = nil
    var en: String? = nil
    var before: String? = nil
    var after: String? = nil
    var hint: String? = nil
    // `answer` is a string for translate/fill and an index for read_meaning.
    var answerText: String? = nil
    var answerIndex: Int? = nil
    /// Set on exercises the app generated for spaced review.
    var reviewKey: String? = nil
    /// Passage glosses: word as written (lowercase, no stress) → meaning in context.
    var glosses: [String: String]? = nil
    /// Grammar patterns the exercise practises (lemmas.json construction ids); when absent the
    /// knowledge model tags it from its words.
    var skills: [String]? = nil
    /// Diary: what to write about, in English or Russian.
    var prompt: String? = nil

    /// A human reading of the text (an Audio/ clip) with its word timings file (Audio/*.words.json);
    /// `span` = [first, last] timing index when the exercise is one stretch of it (chunk dictation,
    /// shadowing). See Narration.swift.
    var narration: String? = nil
    var words: String? = nil
    var span: [Int]? = nil

    /// build_word: the morpheme tiles that spell `answer`, in order (`options` holds every tile offered);
    /// `alternatives` are other words the tiles may also spell that count as right.
    var parts: [String]? = nil
    var alternatives: [String]? = nil
    /// family_guess: the new family member inside `text`, and the root it shares with known words.
    var word: String? = nil
    var root: String? = nil
    /// paradigm_fill / transform: the paradigms.json table, the cell asked (`cue`), the cells to
    /// write (`cells`), and every cell shown with its form (`forms`; `options` present = tap tiles
    /// instead of writing). cue_sort: `items` sorted into `options`.
    var paradigm: String? = nil
    var cue: String? = nil
    var cells: [String]? = nil
    var forms: [String: String]? = nil
    var items: [SortItem]? = nil

    enum CodingKeys: String, CodingKey {
        case kind, letter, text, examples, times, say, options, en, before, after, hint, answer, reviewKey, glosses, skills, prompt
        case narration, words, span
        case parts, alternatives, word, root
        case paradigm, cue, cells, forms, items
    }

    init(kind: ExerciseKind) { self.kind = kind }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        kind = try c.decode(ExerciseKind.self, forKey: .kind)
        letter = try c.decodeIfPresent(String.self, forKey: .letter)
        text = try c.decodeIfPresent(String.self, forKey: .text)
        examples = try c.decodeIfPresent([Course.Example].self, forKey: .examples)
        times = try c.decodeIfPresent(Int.self, forKey: .times)
        say = try c.decodeIfPresent(String.self, forKey: .say)
        options = try c.decodeIfPresent([String].self, forKey: .options)
        en = try c.decodeIfPresent(String.self, forKey: .en)
        before = try c.decodeIfPresent(String.self, forKey: .before)
        after = try c.decodeIfPresent(String.self, forKey: .after)
        hint = try c.decodeIfPresent(String.self, forKey: .hint)
        reviewKey = try c.decodeIfPresent(String.self, forKey: .reviewKey)
        glosses = try c.decodeIfPresent([String: String].self, forKey: .glosses)
        skills = try c.decodeIfPresent([String].self, forKey: .skills)
        prompt = try c.decodeIfPresent(String.self, forKey: .prompt)
        narration = try c.decodeIfPresent(String.self, forKey: .narration)
        words = try c.decodeIfPresent(String.self, forKey: .words)
        span = try c.decodeIfPresent([Int].self, forKey: .span)
        parts = try c.decodeIfPresent([String].self, forKey: .parts)
        alternatives = try c.decodeIfPresent([String].self, forKey: .alternatives)
        word = try c.decodeIfPresent(String.self, forKey: .word)
        root = try c.decodeIfPresent(String.self, forKey: .root)
        paradigm = try c.decodeIfPresent(String.self, forKey: .paradigm)
        cue = try c.decodeIfPresent(String.self, forKey: .cue)
        cells = try c.decodeIfPresent([String].self, forKey: .cells)
        forms = try c.decodeIfPresent([String: String].self, forKey: .forms)
        items = try c.decodeIfPresent([SortItem].self, forKey: .items)
        if let i = try? c.decodeIfPresent(Int.self, forKey: .answer) {
            answerIndex = i
        } else {
            answerText = try c.decodeIfPresent(String.self, forKey: .answer)
        }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(kind, forKey: .kind)
        try c.encodeIfPresent(letter, forKey: .letter)
        try c.encodeIfPresent(text, forKey: .text)
        try c.encodeIfPresent(examples, forKey: .examples)
        try c.encodeIfPresent(times, forKey: .times)
        try c.encodeIfPresent(say, forKey: .say)
        try c.encodeIfPresent(options, forKey: .options)
        try c.encodeIfPresent(en, forKey: .en)
        try c.encodeIfPresent(before, forKey: .before)
        try c.encodeIfPresent(after, forKey: .after)
        try c.encodeIfPresent(hint, forKey: .hint)
        try c.encodeIfPresent(reviewKey, forKey: .reviewKey)
        try c.encodeIfPresent(glosses, forKey: .glosses)
        try c.encodeIfPresent(skills, forKey: .skills)
        try c.encodeIfPresent(prompt, forKey: .prompt)
        try c.encodeIfPresent(narration, forKey: .narration)
        try c.encodeIfPresent(words, forKey: .words)
        try c.encodeIfPresent(span, forKey: .span)
        try c.encodeIfPresent(parts, forKey: .parts)
        try c.encodeIfPresent(alternatives, forKey: .alternatives)
        try c.encodeIfPresent(word, forKey: .word)
        try c.encodeIfPresent(root, forKey: .root)
        try c.encodeIfPresent(paradigm, forKey: .paradigm)
        try c.encodeIfPresent(cue, forKey: .cue)
        try c.encodeIfPresent(cells, forKey: .cells)
        try c.encodeIfPresent(forms, forKey: .forms)
        try c.encodeIfPresent(items, forKey: .items)
        if let answerIndex { try c.encode(answerIndex, forKey: .answer) }
        else { try c.encodeIfPresent(answerText, forKey: .answer) }
    }

    /// The Russian the learner is expected to produce, for writing/speaking exercises.
    var target: String? {
        switch kind {
        case .meet: letter
        case .trace, .copy, .dictation, .readAloud: text
        case .translate, .fill, .buildWord, .transform: answerText
        case .listenPick: say
        default: nil
        }
    }

    /// The Russian the app should speak for this exercise.
    var spoken: String? {
        switch kind {
        case .listenPick: say
        case .dictation: say ?? text
        case .readAloud, .trace, .copy, .readMeaning, .passage, .familyGuess: text
        case .fill: [before, answerText, after].compactMap { $0 }.joined()
        case .translate, .buildWord, .transform: answerText
        default: nil
        }
    }
}
