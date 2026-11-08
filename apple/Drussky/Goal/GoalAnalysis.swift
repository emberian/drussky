import Foundation
import LearningCore

/// A goal text taken apart for the knowledge model: sentences, tokens with their lemmas, the
/// LearningCore target they make, and the grammar patterns they use. Built once per goal text
/// and lemma table; the distance is then a cheap fold over the model.
struct GoalAnalysis: Sendable {
    struct Token: Sendable, Hashable {
        /// As written (stress marks removed, case kept).
        var text: String
        /// RussianText.key form.
        var key: String
        var lemma: String
        /// The lemma came from the rule-based lemmatizer, not the course or the tutor.
        var guessed: Bool
        var sentence: Int
    }

    struct Sentence: Sendable, Hashable {
        var text: String
        var tokens: Range<Int>
        var paragraph: Int
    }

    let goal: Goal
    let sentences: [Sentence]
    let tokens: [Token]
    let target: Target
    /// Construction id → occurrences in the text.
    let patterns: [String: Int]
    /// Lemma → English, from the tutor table or the course.
    let glosses: [String: String]

    init(goal: Goal, domain: RussianDomain) {
        self.goal = goal
        let lemmas = domain.lemmas
        var vocabulary = Set(lemmas.forms.values.compactMap(\.first)).union(domain.words)
        for e in (goal.lemmaCache ?? [:]).values { vocabulary.insert(RussianText.key(e.lemma)) }
        let lemmatizer = RussianLemmatizer(vocabulary: vocabulary)

        var sentences: [Sentence] = []
        var tokens: [Token] = []
        var units: [Target.Unit] = []
        var segments: [Range<Int>] = []
        var patterns: [String: Int] = [:]
        var glosses: [String: String] = [:]
        for (p, paragraph) in goal.text.split(separator: "\n", omittingEmptySubsequences: true).enumerated() {
            for s in Self.splitSentences(String(paragraph)) {
                let si = sentences.count
                let start = tokens.count
                var keys: [String] = []
                for word in Self.words(in: s) {
                    let key = RussianText.key(word)
                    guard !key.isEmpty, !key.contains(" ") else { continue }
                    if key.count == 1, RussianLemmatizer.closedClass[key] == nil { continue }
                    keys.append(key)
                    var lemma: String
                    var guessed = false
                    var gloss: String?
                    if let l = lemmas.lemma(key) { lemma = l }
                    else if let e = goal.lemmaCache?[key] { lemma = RussianText.key(e.lemma); gloss = e.en }
                    else if domain.words.contains(key) { lemma = key }
                    else { lemma = lemmatizer.lemma(key); guessed = true }
                    if lemma.isEmpty { lemma = key }
                    if let gloss, !gloss.isEmpty, glosses[lemma] == nil { glosses[lemma] = gloss }
                    else if glosses[lemma] == nil, let g = domain.graph[ComponentID(.lexeme, lemma)]?.gloss { glosses[lemma] = g }
                    tokens.append(Token(text: RussianText.unstressed(word), key: key, lemma: lemma, guessed: guessed, sentence: si))
                    let lexID = ComponentID(.lexeme, lemma)
                    var candidates = [lexID]
                    if lemma != key { candidates += [ComponentID(.form, key), ComponentID(.lexeme, key)] }
                    let bonus = si < Goal.firstPageSentences ? Goal.firstPageBonus : 0
                    units.append(Target.Unit(key: lexID, candidates: candidates, weight: 1 + bonus))
                    for tag in lemmas.tags(key) { patterns[tag, default: 0] += 1 }
                    if lemmas.lemma(key) == nil { for tag in RussianDomain.suffixTags(key) { patterns[tag, default: 0] += 1 } }
                }
                for tag in RussianDomain.contextTags(keys, lemmas: lemmas) { patterns[tag, default: 0] += 1 }
                guard tokens.count > start else { continue }
                sentences.append(Sentence(text: s, tokens: start..<tokens.count, paragraph: p))
                segments.append(start..<tokens.count)
            }
        }
        self.sentences = sentences
        self.tokens = tokens
        self.target = Target(units: units, segments: segments)
        self.patterns = patterns
        self.glosses = glosses
    }

    // MARK: Text

    /// Sentences of a paragraph: split after . ! ? … (and any closing quotes) before a space.
    static func splitSentences(_ paragraph: String) -> [String] {
        var out: [String] = []
        var current = ""
        let chars = Array(paragraph)
        var i = 0
        let terminators: Set<Character> = [".", "!", "?", "…"]
        let closers: Set<Character> = ["»", "\"", "”", "’", ")", "'"]
        while i < chars.count {
            let c = chars[i]
            current.append(c)
            if terminators.contains(c) {
                var j = i + 1
                while j < chars.count, terminators.contains(chars[j]) || closers.contains(chars[j]) { current.append(chars[j]); j += 1 }
                let atEnd = j >= chars.count
                let nextIsSpace = !atEnd && chars[j].isWhitespace
                // "т. е." style abbreviations: a one-letter word before the period stays in the sentence.
                let word = current.split(whereSeparator: \.isWhitespace).last.map { $0.filter { !terminators.contains($0) && !closers.contains($0) } } ?? ""
                if (atEnd || nextIsSpace) && word.count != 1 {
                    let s = current.trimmingCharacters(in: .whitespaces)
                    if !s.isEmpty { out.append(s) }
                    current = ""
                }
                i = j
                continue
            }
            i += 1
        }
        let rest = current.trimmingCharacters(in: .whitespaces)
        if !rest.isEmpty { out.append(rest) }
        return out
    }

    /// Cyrillic words of a sentence, in order (hyphenated compounds come apart).
    static func words(in sentence: String) -> [String] {
        var out: [String] = []
        var current = ""
        for ch in sentence {
            let isLetter = ch.unicodeScalars.first.map { (0x0400...0x04FF).contains($0.value) } ?? false
            if isLetter || ch == RussianText.stressMark { current.append(ch) }
            else if !current.isEmpty { out.append(current); current = "" }
        }
        if !current.isEmpty { out.append(current) }
        return out
    }

    /// Token key → the lemma this analysis gave it.
    var lemmaByKey: [String: String] {
        Dictionary(tokens.map { ($0.key, $0.lemma) }, uniquingKeysWith: { a, _ in a })
    }

    /// Tokens the tutor hasn't lemmatized: outside the course, not yet in the goal's table.
    var tokensNeedingLemmas: [String] {
        var seen = Set<String>()
        return tokens.filter { $0.guessed && seen.insert($0.key).inserted }.map(\.key)
    }

    func sentenceText(_ range: Range<Int>) -> String {
        sentences[range.clamped(to: 0..<sentences.count)].map(\.text).joined(separator: " ")
    }

    var firstPageSentences: Range<Int> { 0..<min(Goal.firstPageSentences, sentences.count) }

    // MARK: Distance

    func distance(model: KnowledgeModel, at date: Date) -> GoalDistance {
        GoalDistance(analysis: self, model: model, at: date)
    }
}

/// Where the learner stands relative to a goal, right now.
struct GoalDistance: Sendable {
    struct Rung: Sendable, Hashable, Identifiable {
        var id: String { lemma }
        var lemma: String
        /// As first written in the text.
        var example: String
        var gloss: String?
        var count: Int
        var value: Double
        var guessed: Bool
        var onFirstPage: Bool
    }
    struct Pattern: Sendable, Hashable, Identifiable {
        var id: String { construction.id }
        var construction: Lemmas.Construction
        var count: Int
        var strength: Double?
    }

    let analysis: GoalAnalysis
    let core: Distance
    /// Known share per sentence.
    var sentenceCoverage: [Double] { core.segmentCoverage }
    let ladder: [Rung]
    let readableSentences: Int
    /// Lowest known share among the first-page sentences, and the distinct unknown lemmas there.
    let firstPageCoverage: Double
    let firstPageUnknown: Int
    let firstPageReady: Bool
    /// Every lemma of the text the learner doesn't know yet (the ladder is the top of this).
    let unknownLemmas: Set<String>

    init(analysis: GoalAnalysis, model: KnowledgeModel, at date: Date) {
        self.analysis = analysis
        let d = model.distance(to: analysis.target, at: date, threshold: Goal.knownThreshold, ladder: 20)
        core = d
        var unknownAll = Set<String>()
        for (i, u) in analysis.target.units.enumerated() where !model.knows(u, at: date, threshold: Goal.knownThreshold) {
            unknownAll.insert(analysis.tokens[i].lemma)
        }
        unknownLemmas = unknownAll
        var firstToken: [String: GoalAnalysis.Token] = [:]
        for t in analysis.tokens where firstToken[t.lemma] == nil { firstToken[t.lemma] = t }
        ladder = d.ladder.map { r in
            let t = firstToken[r.key.key]
            return Rung(lemma: r.key.key, example: t?.text ?? r.key.key, gloss: analysis.glosses[r.key.key], count: r.count,
                        value: r.value, guessed: t?.guessed ?? false, onFirstPage: (t?.sentence ?? .max) < Goal.firstPageSentences)
        }
        readableSentences = d.segmentsReadable(at: Goal.readableThreshold)
        let first = analysis.firstPageSentences
        let firstCoverage = first.isEmpty ? 0 : first.map { d.segmentCoverage[$0] }.min() ?? 0
        firstPageCoverage = firstCoverage
        var unknown = Set<String>()
        for s in first {
            for i in analysis.sentences[s].tokens where !model.knows(analysis.target.units[i], at: date, threshold: Goal.knownThreshold) {
                unknown.insert(analysis.tokens[i].lemma)
            }
        }
        firstPageUnknown = unknown.count
        firstPageReady = !first.isEmpty && firstCoverage >= Goal.firstPageThreshold
    }

    var coverage: Double { core.coverage }
    var unknownWords: Int { core.unknownKeys }
    var totalWords: Int { core.keys }

    /// The patterns the text uses, most frequent first, with the learner's strength in each.
    func patterns(domain: RussianDomain, model: KnowledgeModel, at date: Date, limit: Int = 8) -> [Pattern] {
        analysis.patterns.compactMap { id, n -> Pattern? in
            guard let c = domain.lemmas.constructions.first(where: { $0.id == id }) else { return nil }
            return Pattern(construction: c, count: n, strength: model.strength(ComponentID(.construction, id), at: date))
        }
        .sorted { $0.count != $1.count ? $0.count > $1.count : $0.id < $1.id }
        .prefix(limit).map { $0 }
    }

    /// 2–4 consecutive sentences all readable now, worth the most (ladder value of the unknown
    /// words in them; sentences already used in a pulled page count less). nil when no stretch
    /// is readable yet.
    func passageWindow(avoiding used: Set<Int> = []) -> Range<Int>? {
        let n = analysis.sentences.count
        let value = Dictionary(ladder.map { ($0.lemma, $0.value) }, uniquingKeysWith: { a, _ in a })
        var best: (Range<Int>, Double)?
        for start in 0..<n {
            for len in Goal.passageSentences where start + len <= n {
                let r = start..<(start + len)
                guard r.allSatisfy({ sentenceCoverage[$0] >= Goal.readableThreshold }) else { break }
                var score = 0.1 * Double(len)
                for s in r {
                    for i in analysis.sentences[s].tokens { score += value[analysis.tokens[i].lemma] ?? 0 }
                    if used.contains(s) { score -= 3 }
                }
                if best == nil || score > best!.1 { best = (r, score) }
            }
        }
        return best?.0
    }

    /// The highest-value unknown words that occur in `sentences` (or anywhere), with the
    /// sentence each is first found in — what a pulled page should teach.
    func wordsToTeach(in sentences: Range<Int>?, limit: Int = 6) -> [(rung: Rung, sentence: String)] {
        var out: [(Rung, String)] = []
        for r in ladder where out.count < limit {
            let hits = analysis.tokens.filter { $0.lemma == r.lemma && (sentences?.contains($0.sentence) ?? true) }
            guard let t = hits.first else { continue }
            out.append((r, analysis.sentences[t.sentence].text))
        }
        if out.count < min(limit, 4), sentences != nil { return wordsToTeach(in: nil, limit: limit) }
        return out
    }
}
