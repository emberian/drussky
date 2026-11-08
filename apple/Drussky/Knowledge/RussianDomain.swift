import Foundation
import LearningCore

extension ComponentKind {
    /// A Cyrillic letter (print and cursive are one letter): `letter:Ж`, the same key as `memories`.
    static let letter: ComponentKind = "letter"
    /// A sound contrast from pairs.json: `sound:y-i`.
    static let sound: ComponentKind = "sound"
    /// A lemma: `lex:читать`.
    static let lexeme: ComponentKind = "lex"
    /// An inflected form that differs from its lemma: `form:читаю`.
    static let form: ComponentKind = "form"
    /// A multi-word lexicon entry: `phrase:у меня есть`.
    static let phrase: ComponentKind = "phrase"
    /// A grammar pattern from lemmas.json: `gram:past`.
    static let construction: ComponentKind = "gram"
}

extension Modality {
    /// Writing over or beside a model.
    static let copy: Modality = "see-write"
    static let hearWrite: Modality = "hear-write"
    /// Writing the Russian for an English prompt (translate, fill).
    static let meaningWrite: Modality = "meaning-write"
    static let say: Modality = "see-say"
    static let hearPick: Modality = "hear-pick"
    static let readPick: Modality = "read-pick"
    /// Writing one's own Russian (the diary), corrected by the tutor.
    static let produce: Modality = "own-writing"

    var title: String {
        switch self {
        case .copy: "copy"
        case .produce: "own writing"
        case .hearWrite: "hear → write"
        case .meaningWrite: "English → write"
        case .say: "see → say"
        case .hearPick: "hear → pick"
        case .readPick: "read → pick"
        case .build: "English → build"
        case .type: "typed"
        default: rawValue
        }
    }
}

extension Channel {
    static let writing: Channel = "writing"
    static let speech: Channel = "speech"
    static let choice: Channel = "choice"
    static let sound: Channel = "sound"
    static let meaning: Channel = "meaning"
}

/// Curated inflection data (Resources/lemmas.json): form → lemma + construction tags, and the
/// constructions themselves with their prerequisites.
struct Lemmas: Codable, Sendable {
    struct Construction: Codable, Sendable, Hashable {
        var id: String
        var name: String
        var note: String
        var requires: [String]?
    }
    var version: Int
    var constructions: [Construction]
    /// form → [lemma, tag, tag…]
    var forms: [String: [String]]

    func lemma(_ form: String) -> String? { forms[form]?.first }
    func tags(_ form: String) -> [String] { Array((forms[form] ?? []).dropFirst()) }

    static func load(bundle: Bundle = .main) -> Lemmas {
        guard let url = bundle.url(forResource: "lemmas", withExtension: "json"), let data = try? Data(contentsOf: url),
              let l = try? JSONDecoder().decode(Lemmas.self, from: data) else {
            return Lemmas(version: 1, constructions: [], forms: [:])
        }
        return l
    }
}

/// One graded thing on a workbook page: the exercise and its result before and after.
struct RussianAttempt {
    var page: BookPage
    var index: Int
    var before: ExerciseResult
    var after: ExerciseResult
    var at: Date
    var latency: Double? = nil
    var backfill = false
}

/// Russian, as Drussky teaches it, in LearningCore's terms.
struct RussianDomain: KnowledgeDomain {
    let name = "drussky-ru"
    let modalities: [Modality] = [.copy, .hearWrite, .meaningWrite, .say, .hearPick, .readPick, .produce, .build, .type]
    let graph: KnowledgeGraph
    let lemmas: Lemmas
    /// Roots, affixes and segmentations (Morphology/).
    let morphology: Morphology
    /// Every single word the graph knows as a word (lexicon tokens, lemma-map forms, pair words, kept words, morphology lemmas).
    let words: Set<String>
    /// Lexicon key → its component (phrase or word).
    private let entryIDs: [String: ComponentID]
    /// Lowercased English gloss → lexicon keys with that gloss.
    private let byGloss: [String: [String]]
    private let letterSays: [String: String]

    init(course: Course, lemmas: Lemmas, pairs: SoundPairs, customWords: [Course.Word] = [], morphology: Morphology = .empty) {
        self.lemmas = lemmas
        self.morphology = morphology
        let lexicon = course.lexicon + customWords
        var words = Set<String>()
        for w in lexicon { for t in RussianText.key(w.ru).split(separator: " ") { words.insert(String(t)) } }
        for (form, entry) in lemmas.forms { words.insert(form); if let l = entry.first { words.insert(l) } }
        for p in pairs.pairs { for w in p.words { words.insert(RussianText.key(w.ru)) } }
        for key in morphology.lemmas.keys where !key.contains(" ") { words.insert(key) }
        self.words = words
        var gloss: [String: [String]] = [:]
        for w in lexicon where !w.en.isEmpty { gloss[w.en.lowercased(), default: []].append(RussianText.key(w.ru)) }
        byGloss = gloss
        letterSays = Dictionary(course.letters.map { (RussianText.key($0.say), $0.upper) }, uniquingKeysWith: { a, _ in a })

        // First appearance of every token and letter in the course, for ordering and units.
        var firstSeen: [String: (Int, String)] = [:]
        var letterSeen: [String: (Int, String)] = [:]
        var n = 0
        for (unit, page) in course.pageSequence {
            for ex in page.exercises {
                n += 1
                if ex.kind == .meet, let l = ex.letter, letterSeen[l] == nil { letterSeen[l] = (n, unit.title) }
                let strings = [ex.target, ex.text, ex.answerText, ex.before, ex.after].compactMap { $0 } + (ex.options ?? [])
                for s in strings { for t in RussianText.key(s).split(separator: " ") where firstSeen[String(t)] == nil {
                    firstSeen[String(t)] = (n, unit.title)
                } }
            }
        }

        var comps: [Component] = []
        var edges: [Edge] = []
        func letterIDs(_ s: String) -> [ComponentID] { RussianText.letters(in: s).sorted().map { ComponentID(.letter, $0) } }
        for l in course.letters {
            comps.append(Component(id: ComponentID(.letter, l.upper), label: "\(l.upper)\(l.lower)", gloss: l.sound,
                                   order: letterSeen[l.upper]?.0 ?? .max, unit: letterSeen[l.upper]?.1))
        }
        for c in lemmas.constructions {
            let id = ComponentID(.construction, c.id)
            comps.append(Component(id: id, label: c.name, gloss: c.note))
            for r in c.requires ?? [] { edges.append(Edge(ComponentID(.construction, r), id, .prerequisite)) }
        }
        for c in pairs.contrasts {
            comps.append(Component(id: ComponentID(.sound, c.id), label: c.name, gloss: c.note, unit: "Sound pairs"))
        }
        let glossFor: [String: String] = Dictionary(lexicon.map { (RussianText.key($0.ru), $0.en) }, uniquingKeysWith: { a, _ in a })
        // Words: lexeme nodes, form nodes for inflections, edges to letters and constructions.
        func addWord(_ token: String, gloss: String?, order: Int, unit: String?) -> ComponentID {
            let lemma = lemmas.lemma(token) ?? token
            let lexID = ComponentID(.lexeme, lemma)
            let seen = firstSeen[lemma] ?? firstSeen[token]
            comps.append(Component(id: lexID, label: lemma, gloss: glossFor[lemma] ?? (lemma == token ? gloss : nil),
                                   order: seen?.0 ?? order, unit: seen?.1 ?? unit))
            edges += letterIDs(lemma).map { Edge(lexID, $0, .uses) }
            var id = lexID
            if lemma != token {
                id = ComponentID(.form, token)
                comps.append(Component(id: id, label: token, gloss: glossFor[token] ?? gloss,
                                       order: firstSeen[token]?.0 ?? order, unit: firstSeen[token]?.1 ?? unit))
                edges.append(Edge(id, lexID, .formOf))
                edges += letterIDs(token).map { Edge(id, $0, .uses) }
            }
            for tag in lemmas.tags(token) { edges.append(Edge(id, ComponentID(.construction, tag), .exemplifies)) }
            return id
        }
        var entries: [String: ComponentID] = [:]
        for (i, w) in lexicon.enumerated() {
            let key = RussianText.key(w.ru)
            let tokens = key.split(separator: " ").map(String.init)
            let order = 1_000_000 + i
            let unit = i >= course.lexicon.count ? "Kept" : nil
            if tokens.count == 1 {
                entries[key] = addWord(key, gloss: w.en, order: order, unit: unit)
            } else if !tokens.isEmpty {
                let id = ComponentID(.phrase, key)
                let seen = tokens.compactMap { firstSeen[$0] }.max { $0.0 < $1.0 }
                comps.append(Component(id: id, label: RussianText.unstressed(w.ru), gloss: w.en,
                                       order: seen?.0 ?? order, unit: seen?.1 ?? unit))
                for t in tokens { edges.append(Edge(addWord(t, gloss: nil, order: order, unit: unit), id, .partOf)) }
                for tag in Self.contextTags(tokens, lemmas: lemmas) { edges.append(Edge(id, ComponentID(.construction, tag), .exemplifies)) }
                entries[key] = id
            }
        }
        for form in lemmas.forms.keys where entries[form] == nil { _ = addWord(form, gloss: nil, order: 2_000_000, unit: nil) }
        for p in pairs.pairs {
            for w in p.words {
                let id = addWord(RussianText.key(w.ru), gloss: w.en, order: 3_000_000, unit: "Sound pairs")
                if p.same != true { edges.append(Edge(id, ComponentID(.sound, p.contrast), .exemplifies)) }
            }
        }
        entryIDs = entries
        // Roots, affixes and word families: a word's component is its lexeme (or the lexicon phrase it is).
        let (mc, me) = Self.morphologyGraph(morphology) { entries[$0] ?? ComponentID(.lexeme, $0) }
        graph = KnowledgeGraph(components: comps + mc, edges: edges + me)
    }

    // MARK: Words

    /// The component a single word exercises: its form node when inflected, else its lexeme.
    func wordID(_ token: String) -> ComponentID {
        if let lemma = lemmas.lemma(token), lemma != token { return ComponentID(.form, token) }
        return ComponentID(.lexeme, token)
    }

    /// The best single id for a Russian string: a lexicon phrase, a word, or a letter by its sound.
    func utteranceID(_ s: String, letters: Bool = false) -> ComponentID? {
        let key = RussianText.key(s)
        guard !key.isEmpty else { return nil }
        if letters, let l = letterSays[key] { return ComponentID(.letter, l) }
        if let id = entryIDs[key] { return id }
        if !key.contains(" "), words.contains(key) { return wordID(key) }
        return nil
    }

    /// Lemma of a word as written, with a small suffix-stripping fallback for texts outside the course.
    func lemma(of token: String) -> String? {
        if let l = lemmas.lemma(token) { return l }
        if words.contains(token) { return token }
        if let e = morphology.entry(word: token) { return e.key }
        let endings = ["ами", "ями", "ого", "его", "ому", "ему", "ой", "ей", "ом", "ем", "ам", "ям", "ах", "ях",
                       "ую", "юю", "ые", "ие", "ый", "ий", "у", "ю", "е", "а", "я", "ы", "и", "о"]
        for e in endings where token.hasSuffix(e) && token.count > e.count + 2 {
            let stem = String(token.dropLast(e.count))
            for back in ["", "а", "я", "о", "е", "ь", "ый", "ий", "ой"] where words.contains(stem + back) { return stem + back }
        }
        return nil
    }

    /// Construction tags from word order: в/на + where to, у меня есть, не + verb.
    static func contextTags(_ tokens: [String], lemmas: Lemmas) -> Set<String> {
        var tags = Set<String>()
        for (i, t) in tokens.enumerated() {
            let next = i + 1 < tokens.count ? tokens[i + 1] : nil
            if t == "не", next != nil { tags.insert("negation") }
            if t == "у", let n = next, ["меня", "тебя", "него", "неё", "нас", "вас", "них"].contains(n) { tags.insert("have") }
            if ["в", "на"].contains(t), let n = next {
                let ntags = lemmas.tags(n)
                if ntags.contains("acc-u") { tags.insert("acc-dir") }
                else if ntags.isEmpty, lemmas.lemma(n) == nil, ["город", "парк", "дом", "космос"].contains(n) { tags.insert("acc-dir") }
            }
        }
        return tags
    }

    /// Construction tags for a word the lemma map doesn't know, from its ending.
    static func suffixTags(_ t: String) -> [String] {
        guard t.count >= 5 else { return [] }
        let rules: [(String, String)] = [("ешь", "pres-2sg"), ("ёшь", "pres-2sg"), ("ишь", "pres-2sg"),
                                         ("ете", "pres-2pl"), ("ёте", "pres-2pl"), ("ите", "pres-2pl"),
                                         ("ают", "pres-3pl"), ("яют", "pres-3pl"), ("ала", "past"), ("ила", "past"),
                                         ("ела", "past"), ("али", "past"), ("или", "past"), ("ать", "inf"), ("ить", "inf"),
                                         ("еть", "inf")]
        return rules.filter { t.hasSuffix($0.0) }.map(\.1).prefix(1).map { $0 }
    }

    // MARK: Tagging an exercise

    /// Which components an exercise exercises.
    func components(for ex: Exercise) -> [ComponentID] {
        if ex.kind == .meet, let l = ex.letter { return [ComponentID(.letter, l)] }
        if let key = ex.reviewKey, key.hasPrefix("letter:"), let id = ComponentID(key) { return [id] }
        var out: [ComponentID] = []
        if ex.kind == .soundPair, let contrast = ex.hint { out.append(ComponentID(.sound, contrast)) }
        let target: String? = switch ex.kind {
        case .readMeaning: ex.text
        case .soundPair: ex.say
        case .familyGuess: ex.word  // the new family member, not the sentence it sits in
        default: ex.target
        }
        guard let target else { return out }
        let key = RussianText.key(target)
        let tokens = key.split(separator: " ").map(String.init)
        if tokens.count > 1, let phrase = entryIDs[key], phrase.kind == .phrase { out.append(phrase) }
        var tags = Self.contextTags(tokens, lemmas: lemmas)
        for t in tokens {
            if words.contains(t) {
                let id = wordID(t)
                out.append(id)
                if id.kind == .form, let l = lemmas.lemma(t) { out.append(ComponentID(.lexeme, l)) }
                tags.formUnion(lemmas.tags(t))
            } else if let l = lemma(of: t) {
                out.append(ComponentID(.lexeme, l))
                tags.formUnion(Self.suffixTags(t))
            } else {
                // A syllable or a word outside the course: it exercises its letters.
                out += RussianText.letters(in: t).sorted().map { ComponentID(.letter, $0) }
            }
        }
        if let skills = ex.skills {
            tags = Set(skills.map { $0.hasPrefix("gram:") ? String($0.dropFirst(5)) : $0 })
        }
        // A review of a word the course can't analyse still has a memory of its own, so its
        // outcome lands where the scheduler looks.
        if let key = ex.reviewKey, key.hasPrefix("word:"), key.count > 5,
           !out.contains(where: { [.lexeme, .form, .phrase].contains($0.kind) }) {
            out.insert(ComponentID(.lexeme, String(key.dropFirst(5))), at: 0)
        }
        out += tags.sorted().map { ComponentID(.construction, $0) }
        // Roots: every word is evidence for the root it is built on; a family guess names its
        // root outright; building a word also exercises the affix tiles it is built from.
        out += rootComponents(for: out)
        if ex.kind == .familyGuess, let r = ex.root { out.append(ComponentID(.root, r)) }
        if ex.kind == .buildWord, let parts = ex.parts { out += affixComponents(tiles: parts) }
        var seen = Set<ComponentID>()
        return out.filter { seen.insert($0).inserted }
    }

    /// The components whose traces schedule a review target ("letter:Ж", "word:читаю"): the ones
    /// a review exercise of it is tagged with, word-level only (a phrase if it is one; a form
    /// rather than its lemma).
    func reviewComponents(forTarget key: String) -> [ComponentID] {
        let parts = key.split(separator: ":", maxSplits: 1).map(String.init)
        guard parts.count == 2, !parts[1].isEmpty else { return [] }
        if parts[0] == "letter" { return [ComponentID(.letter, parts[1])] }
        var ex = Exercise(kind: .dictation)
        ex.text = parts[1]
        ex.reviewKey = key
        let all = components(for: ex)
        if let phrase = all.first(where: { $0.kind == .phrase }) { return [phrase] }
        let lemmasOfForms = Set(all.filter { $0.kind == .form }.compactMap { lemmas.lemma($0.key) })
        return all.filter { $0.kind == .form || ($0.kind == .lexeme && !lemmasOfForms.contains($0.key)) }
    }

    static func modality(for kind: ExerciseKind) -> Modality? {
        switch kind {
        case .meet, .trace, .copy: .copy
        case .dictation: .hearWrite
        case .translate, .fill: .meaningWrite
        case .readAloud: .say
        case .listenPick, .soundPair: .hearPick
        case .readMeaning, .familyGuess: .readPick
        case .buildWord: .build
        case .explain, .passage: nil
        case .diary: nil  // its evidence is built from the correction (DiaryKnowledge.swift)
        }
    }

    /// The FSRS grade an outcome earns, or nil when it isn't evidence of recall:
    ///   shown the answer and missed                     again
    ///   chose wrong, then right (choices)               again (the second pick was helped by the first)
    ///   right after the answer was shown, then reopened nil (corrected or copied, not retrieved)
    ///   right with no failed check and one attempt      good ("right first time")
    ///   right after a failed check or extra attempt     hard
    ///   finished by the learner's own judgement         hard (not verified)
    ///   skipped                                         nil
    ///   copying and tracing done                        good (in the copy modality only)
    static func grade(_ ex: Exercise, _ r: ExerciseResult) -> Grade? {
        if r.status == .skipped { return nil }
        if r.revealed || r.status == .missed { return .again }
        switch r.status {
        case .open, .missed, .skipped: return r.status == .missed ? .again : nil
        case .correct:
            if r.assisted == true { return nil }
            if r.selfAssessed == true { return .hard }
            return r.isRightFirstTime ? .good : .hard
        case .done:
            switch ex.kind {
            case .meet, .trace, .copy: return .good
            case .listenPick, .readMeaning, .soundPair, .familyGuess, .buildWord: return r.assisted == true ? nil : .again
            case .explain, .passage: return nil
            default: return r.assisted == true ? nil : .hard  // finished, but not verified right first time
            }
        }
    }

    /// What the learner gave, as text.
    static func response(_ ex: Exercise, _ r: ExerciseResult) -> String? {
        if let c = r.chosen, let options = ex.options, options.indices.contains(c) { return options[c] }
        return r.recognized
    }

    // MARK: Confusions

    func confusions(_ ex: Exercise, response: String) -> [Confusion] {
        switch ex.kind {
        case .listenPick, .soundPair:
            let letters = ex.reviewKey?.hasPrefix("letter:") ?? false
            guard let say = ex.say, RussianText.key(say) != RussianText.key(response),
                  let a = utteranceID(say, letters: letters), let b = utteranceID(response, letters: letters), a != b else { return [] }
            return [Confusion(target: a, produced: b, channel: ex.kind == .soundPair ? .sound : .choice)]
        case .readMeaning:
            guard let text = ex.text, let a = utteranceID(text),
                  let keys = byGloss[response.lowercased()], keys.count == 1, let b = utteranceID(keys[0]), a != b else { return [] }
            return [Confusion(target: a, produced: b, channel: .meaning)]
        case .readAloud:
            guard let target = ex.text else { return [] }
            return wordSubstitutions(target, response, channel: .speech)
        case .dictation, .translate, .fill, .copy, .trace:
            guard let target = ex.target, !RussianText.answerMatches(response, target, subjectDrop: ex.kind == .translate) else { return [] }
            let words = wordSubstitutions(target, response, channel: .writing)
            return words.isEmpty ? letterSubstitutions(target, response) : words
        default:
            return []
        }
    }

    /// Same number of words, some replaced by other words the course knows.
    private func wordSubstitutions(_ target: String, _ response: String, channel: Channel) -> [Confusion] {
        let a = RussianText.comparable(target).split(separator: " ").map(String.init)
        let b = RussianText.comparable(response).split(separator: " ").map(String.init)
        guard a.count == b.count, a != b else { return [] }
        var out: [Confusion] = []
        for (x, y) in zip(a, b) where x != y && words.contains(x) && words.contains(y) {
            out.append(Confusion(target: wordID(x), produced: wordID(y), channel: channel))
        }
        return out
    }

    /// Letter-for-letter substitutions when the reading is close to the target (edit distance
    /// at most a third of its length). Recognizer misreadings land here too; the channel says so.
    private func letterSubstitutions(_ target: String, _ response: String) -> [Confusion] {
        let a = Array(RussianText.comparable(target).filter { !$0.isWhitespace })
        let b = Array(RussianText.comparable(response).filter { !$0.isWhitespace })
        guard a.count >= 1, !b.isEmpty, a != b else { return [] }
        let subs = Self.substitutions(a, b)
        guard subs.distance <= max(1, a.count / 3) else { return [] }
        return subs.pairs.compactMap { x, y in
            let lx = RussianText.letters(in: String(x)), ly = RussianText.letters(in: String(y))
            guard let p = lx.first, let q = ly.first, p != q else { return nil }
            return Confusion(target: ComponentID(.letter, p), produced: ComponentID(.letter, q), channel: .writing)
        }
    }

    /// Levenshtein alignment: the distance and the substituted (target, produced) pairs.
    static func substitutions(_ a: [Character], _ b: [Character]) -> (distance: Int, pairs: [(Character, Character)]) {
        let n = a.count, m = b.count
        var d = Array(repeating: Array(repeating: 0, count: m + 1), count: n + 1)
        for i in 0...n { d[i][0] = i }
        for j in 0...m { d[0][j] = j }
        if n > 0 && m > 0 {
            for i in 1...n { for j in 1...m {
                d[i][j] = min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + (a[i - 1] == b[j - 1] ? 0 : 1))
            } }
        }
        var pairs: [(Character, Character)] = []
        var i = n, j = m
        while i > 0 && j > 0 {
            if d[i][j] == d[i - 1][j - 1] + (a[i - 1] == b[j - 1] ? 0 : 1) {
                if a[i - 1] != b[j - 1] { pairs.append((a[i - 1], b[j - 1])) }
                i -= 1; j -= 1
            } else if d[i][j] == d[i - 1][j] + 1 { i -= 1 } else { j -= 1 }
        }
        return (d[n][m], pairs.reversed())
    }

    // MARK: KnowledgeDomain

    func evidence(for a: RussianAttempt) -> LearningEvent? {
        guard a.page.exercises.indices.contains(a.index) else { return nil }
        let ex = a.page.exercises[a.index]
        guard let modality = a.after.typed == true ? Modality.type : Self.modality(for: ex.kind), !(ex.kind == .soundPair && ex.say == nil) else { return nil }
        let finished = !a.before.status.isComplete && a.after.status.isComplete
        let tried = a.after.attempts > a.before.attempts
        guard finished || tried else { return nil }
        let response = Self.response(ex, a.after)
        // A response carries confusions only the first time it is logged: on the call that made it.
        // A right answer (including "I wrote it right" over a misreading) and finished practice say nothing about confusion.
        let fresh = tried || a.backfill
        let wrongish = a.after.status != .correct && a.after.status != .skipped && (a.after.status != .done || ex.kind == .readAloud)
        let confusions = fresh && wrongish ? response.map { confusions(ex, response: $0) } ?? [] : []
        let item = "\(a.page.id)#\(a.index)"
        // Outcomes have one id per round, so a reconstructed one and a live one can't both count.
        let id = finished ? KnowledgeTracker.outcomeID(item: item, round: a.after.roundNumber) : UUID().uuidString
        let grade = finished ? Self.grade(ex, a.after) : nil
        // A root is credited by every word built on it, never blamed: missing one word is not
        // forgetting the root (weak, one-way evidence).
        var touched = components(for: ex)
        if grade == nil || grade == .again { touched.removeAll { $0.kind == .root } }
        return LearningEvent(
            id: id, t: a.at, type: finished ? .outcome : .attempt,
            source: a.backfill ? "backfill" : "live", item: item, activity: ex.kind.rawValue, modality: modality,
            components: touched, grade: grade,
            correct: finished ? nil : false, attempt: a.after.attempts, expected: [.readMeaning, .familyGuess].contains(ex.kind)
                ? ex.answerIndex.flatMap { ex.options?[safe: $0] } : (ex.kind == .soundPair ? ex.say : ex.target),
            response: response, latency: a.latency,
            help: a.after.revealed ? ["revealed"] : (a.after.assisted == true ? ["assisted"] : nil),
            confusions: confusions.isEmpty ? nil : confusions, review: ex.reviewKey != nil ? true : nil)
    }

    func normalize(_ answer: String) -> String { RussianText.comparable(answer) }

    func tokens(in text: String) -> [[ComponentID]] {
        RussianText.key(text).split(separator: " ").map(String.init)
            .filter { !RussianText.letters(in: $0).isEmpty }
            .map { t in
                guard let l = lemma(of: t) else { return [ComponentID(.lexeme, t)] }
                return l == t ? [ComponentID(.lexeme, l)] : [ComponentID(.form, t), ComponentID(.lexeme, l)]
            }
    }
}
