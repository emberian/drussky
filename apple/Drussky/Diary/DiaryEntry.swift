import Foundation

/// One diary line: what the learner wrote in their own Russian, and the tutor's correction.
/// Stored inside the exercise's result (`ExerciseResult.diary`), so it lives with the page and
/// its ink forever; every field the correction adds is optional, so older notebooks decode.
struct DiaryEntry: Codable, Sendable, Hashable {
    /// The prompt as it was shown (kept here so the entry reads on its own in "My Russian").
    var prompt: String
    /// What they meant to write: the recognizer's transcript, as confirmed or fixed by them.
    var text: String
    /// What the recognizer first proposed, when it differs from `text` (nil when typed).
    var recognized: String? = nil
    /// Typed on the keyboard instead of written with the Pencil.
    var typed: Bool? = nil
    var submitted: Date
    var correction: DiaryCorrection? = nil

    /// Plain-language line for the tutor's context.
    static func describe(_ ex: Exercise, _ r: ExerciseResult) -> String {
        var s = "Diary: writing freely about “\(ex.prompt ?? "")”."
        if let d = r.diary {
            s += " They wrote: «\(d.text)»."
            if let c = d.correction {
                s += c.edits.isEmpty ? " It was correct." : " Corrected to «\(c.corrected)» (\(c.edits.count) edit(s): \(c.edits.map { "\($0.span)→\($0.replacement)" }.joined(separator: ", ")))."
            }
        }
        return s
    }
}

/// The tutor's red pen: the corrected text, each change with its reason, one warm sentence,
/// and the pattern behind the biggest slip.
struct DiaryCorrection: Codable, Sendable, Hashable {
    var corrected: String
    var edits: [DiaryEdit]
    var praise: String
    var teach: [DiaryTeach]
    var model: String? = nil
    var at: Date
    /// Whether the marks account for the corrected line (`isConsistent`), decided when the
    /// correction lands. false: kept for the learner to read, but it grades nothing. nil: from
    /// before the check existed (read as consistent).
    var verified: Bool? = nil

    /// Structural consistency alone does not establish a backend's competence to grade.
    var canGrade: Bool { verified != false && model != OnDeviceBackend.modelName }

    /// Whether the marks account for the corrected line: every span is found in `text`, and the
    /// text with each span replaced reads as `corrected` (stress, case, punctuation and ё/е
    /// aside). With no edits, `corrected` must be the text itself. A reply with changed text but
    /// missing or malformed edits fails this, and must not award the replacement's words as
    /// produced right (DiaryBook.finishDiary, DiaryKnowledge).
    func isConsistent(with text: String) -> Bool {
        func norm(_ s: String) -> String { RussianText.key(s).replacingOccurrences(of: "ё", with: "е") }
        guard unplaced(in: text).isEmpty else { return false }
        let applied = segments(of: text).map { seg in seg.edit.map { " " + edits[$0].replacement + " " } ?? seg.text }.joined()
        return norm(applied) == norm(corrected)
    }

    /// A span of the learner's text (as they wrote it) that the teacher marks, with the edit.
    struct Segment: Hashable {
        var text: String
        var edit: Int?
    }

    /// Splits `text` into runs: untouched text and the spans of each edit (by index into
    /// `edits`), matched case- and stress-insensitively, longest spans first, no overlaps.
    /// Edits whose span can't be found still render in the list below the line.
    func segments(of text: String) -> [Segment] {
        let haystack = Array(RussianText.unstressed(text).lowercased())
        let chars = Array(text)
        guard haystack.count == chars.count else { return [Segment(text: text)] }
        var owner = [Int?](repeating: nil, count: chars.count)
        let order = edits.indices.sorted { edits[$0].span.count > edits[$1].span.count }
        for i in order {
            let needle = Array(RussianText.unstressed(edits[i].span).lowercased())
            guard !needle.isEmpty, needle.count <= haystack.count else { continue }
            var start = 0
            while start + needle.count <= haystack.count {
                if Array(haystack[start..<(start + needle.count)]) == needle,
                   owner[start..<(start + needle.count)].allSatisfy({ $0 == nil }) {
                    for k in start..<(start + needle.count) { owner[k] = i }
                    break
                }
                start += 1
            }
        }
        var out: [Segment] = []
        var k = 0
        let punctuation = Set(".,!?;:…»)")
        while k < chars.count {
            let o = owner[k]
            var end = k
            while end < chars.count, owner[end] == o { end += 1 }
            // Punctuation right after a marked span stays with it, as a teacher's underline would.
            if o != nil { while end < chars.count, owner[end] == nil, punctuation.contains(chars[end]) { end += 1 } }
            out.append(Segment(text: String(chars[k..<end]), edit: o))
            k = end
        }
        return out
    }

    /// Edits whose span was not found in `text` (shown as a list rather than marked inline).
    func unplaced(in text: String) -> [Int] {
        let placed = Set(segments(of: text).compactMap(\.edit))
        return edits.indices.filter { !placed.contains($0) }
    }
}

struct DiaryEdit: Codable, Sendable, Hashable {
    enum Category: String, Codable, Sendable, CaseIterable {
        case spelling, ending, agreement
        case wordChoice = "word-choice"
        case wordOrder = "word-order"
        case stress, punctuation, other

        var title: String {
            switch self {
            case .spelling: "spelling"
            case .ending: "ending"
            case .agreement: "agreement"
            case .wordChoice: "word choice"
            case .wordOrder: "word order"
            case .stress: "stress"
            case .punctuation: "punctuation"
            case .other: "other"
            }
        }

        /// A category this build doesn't know (a later app's notebook) reads as `other`, so one
        /// edit can never make the notebook undecodable.
        init(from decoder: Decoder) throws {
            self = Category(loose: try decoder.singleValueContainer().decode(String.self))
        }

        /// Tolerant of the tutor's wording ("word choice", "Word-Order", "case ending").
        init(loose raw: String) {
            let k = raw.lowercased().replacingOccurrences(of: "_", with: "-").replacingOccurrences(of: " ", with: "-")
            if let c = Category(rawValue: k) { self = c; return }
            if k.contains("spell") || k.contains("letter") { self = .spelling }
            else if k.contains("agree") || k.contains("gender") { self = .agreement }
            else if k.contains("end") || k.contains("case") || k.contains("conjug") || k.contains("declen") { self = .ending }
            else if k.contains("choice") || k.contains("vocab") || k.contains("word") && !k.contains("order") { self = .wordChoice }
            else if k.contains("order") { self = .wordOrder }
            else if k.contains("stress") || k.contains("accent") { self = .stress }
            else if k.contains("punct") { self = .punctuation }
            else { self = .other }
        }
    }

    /// The learner's words, exactly as in their text.
    var span: String
    /// What it should be, stress-marked; empty when the span should simply go.
    var replacement: String
    var category: Category
    /// One line in English on why.
    var why: String
    /// The replacement's meaning in English, so it can join the learner's words.
    var en: String? = nil
    /// A grammar pattern id (lemmas.json construction) this edit exemplifies, when one does.
    var pattern: String? = nil
}

/// The pattern behind the biggest error, taught in two sentences.
struct DiaryTeach: Codable, Sendable, Hashable {
    var title: String
    var note: String
    /// Construction id from lemmas.json when one fits, else a short slug.
    var pattern: String? = nil
}
