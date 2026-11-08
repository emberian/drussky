import Foundation

/// A page toward the goal: what to put on it (the brief handed to PageWriter), the check that
/// nothing on it was invented, and an offline page built from the text alone.
enum GoalPull {
    /// What the tutor is given for a goal page.
    struct Brief: Sendable {
        var goal: Goal
        /// The passage's sentences (indices into the analysis), if a readable stretch exists.
        var sentences: Range<Int>?
        var passage: String?
        var words: [(rung: GoalDistance.Rung, sentence: String)]
        var patterns: [GoalDistance.Pattern]
        /// Normalized goal text, for the invented-sentence check.
        var normalizedText: String

        init(goal: Goal, distance: GoalDistance, patterns: [GoalDistance.Pattern]) {
            self.goal = goal
            let window = distance.passageWindow(avoiding: Set(goal.usedSentences ?? []))
            sentences = window
            passage = window.map { distance.analysis.sentenceText($0) }
            words = distance.wordsToTeach(in: window, limit: 6)
            self.patterns = patterns.filter { ($0.strength ?? 0) < 0.6 }.prefix(2).map { $0 }
            normalizedText = GoalPull.normalize(goal.text)
        }
    }

    /// Comparison form for "is this in the text": lowercase, no stress, no punctuation, one space.
    static func normalize(_ s: String) -> String { RussianText.key(s).replacingOccurrences(of: "ё", with: "е") }

    /// True when every Russian sentence or phrase of `s` is a contiguous stretch of the goal text.
    static func inText(_ s: String?, _ text: String) -> Bool {
        guard let s else { return true }
        let n = normalize(s)
        guard !n.isEmpty else { return true }
        return (" " + text + " ").contains(" " + n + " ")
    }

    /// Drops an exercise whose Russian isn't in the goal text. Explanations and glosses are
    /// free; options that aren't the answer may be single words (distractors).
    static func verify(_ ex: Exercise, in brief: Brief) -> Exercise? {
        let text = brief.normalizedText
        switch ex.kind {
        case .passage:
            guard let t = ex.text else { return nil }
            let lines = t.split(separator: "\n").map(String.init)
            let sentences = lines.flatMap(GoalAnalysis.splitSentences)
            return sentences.allSatisfy { inText($0, text) } ? ex : nil
        case .explain:
            var e = ex
            e.examples = ex.examples?.filter { inText($0.ru, text) }
            return e
        case .trace, .copy, .dictation, .readAloud, .readMeaning:
            return inText(ex.text, text) ? ex : nil
        case .translate:
            return inText(ex.answerText, text) ? ex : nil
        case .fill:
            let whole = [ex.before, ex.answerText, ex.after].compactMap { $0 }.joined(separator: " ")
            return inText(whole, text) ? ex : nil
        case .listenPick:
            guard inText(ex.say, text) else { return nil }
            var e = ex
            e.options = ex.options?.filter { $0 == ex.say || !RussianText.key($0).contains(" ") || inText($0, text) }
            return (e.options?.count ?? 0) >= 2 ? e : nil
        case .diary:
            return ex  // an English prompt to write about the text; no Russian to verify
        case .readHandwriting:
            return inText(ex.text, text) ? ex : nil
        case .meet, .soundPair, .buildWord, .familyGuess, .talk, .paradigmFill, .transform, .cueSort, .constructionChoice, .listening, .structure:
            return nil  // generated locally, never from a goal text
        }
    }

    // MARK: Prompt

    static func prompt(_ brief: Brief, state: String) -> String {
        let words = brief.words.map { w -> String in
            let gloss = w.rung.gloss.map { " (\($0))" } ?? ""
            return "«\(w.rung.example)»\(gloss) — in: \(w.sentence)"
        }.joined(separator: "\n")
        let patterns = brief.patterns.map { "\($0.construction.name): \($0.construction.note)" }.joined(separator: "; ")
        let passage = brief.passage.map {
            """
            This stretch is now ≥80% known to them; it is the page's passage:
            ---
            \($0)
            ---
            """
        } ?? "No stretch of the text is readable yet: write NO passage; teach the words below through the phrases they occur in."
        return """
        \(state)

        The learner's own goal is to read \(brief.goal.byline) for real. This page is a step toward it, built ONLY \
        from the text's own sentences.

        \(passage)

        The most valuable words still unknown, each with the sentence of the text it occurs in:
        \(words)
        \(patterns.isEmpty ? "" : "Grammar patterns the text leans on that they are still weak in: \(patterns).")

        Write the page: \(brief.passage == nil ? "" : "first ONE passage exercise with the stretch above exactly as written (add stress marks, gloss every word not in their known words, give an English translation). Then ")\
        5–7 graded exercises on the words above, taken from the text itself: read_meaning on a sentence or clause \
        that contains the word (options in English), fill with the word blanked out of its own sentence or clause \
        (hint = English of that sentence), dictation of a short clause (≤ 6 words) containing it, translate where the \
        English has one obvious Russian answer that is a phrase of the text. One explain (≤ 3 sentences) on a pattern \
        above is welcome, with examples quoted from the text. Every Russian string in every exercise must be a \
        contiguous stretch of the text as written (inflections untouched); never invent or alter a sentence — glosses, \
        hints and explanations are the only place for your own words. Letters they haven't met may appear in reading \
        exercises (passage, read_meaning, listen_pick, read_aloud) but not in writing ones. List the words taught in \
        new_words with their meaning in this text. Title the page after the text.
        """
    }

    // MARK: Offline page

    /// A page toward the goal with no tutor: the passage (if any), then the words through their own
    /// sentences — the gap in its own sentence when a gloss is known, hearing and writing the word,
    /// and choosing its meaning when there are enough glosses to choose among.
    static func localPage(_ brief: Brief, met letters: Set<String>, allMet: Bool) -> (title: String, intro: String?, exercises: [Exercise]) {
        var out: [Exercise] = []
        if let passage = brief.passage {
            var ex = Exercise(kind: .passage)
            ex.text = passage
            var glosses: [String: String] = [:]
            for w in brief.words { if let g = w.rung.gloss { glosses[RussianText.key(w.rung.example)] = g } }
            ex.glosses = glosses.isEmpty ? nil : glosses
            out.append(ex)
        }
        func writable(_ s: String) -> Bool { allMet || RussianText.letters(in: s).isSubset(of: letters) }
        let glossed = brief.words.compactMap { w in w.rung.gloss.map { (w, $0) } }
        for (w, gloss) in glossed.prefix(4) {
            let sentence = w.sentence
            let token = w.rung.example
            if let r = sentence.range(of: token), writable(token) {
                var fill = Exercise(kind: .fill)
                fill.before = String(sentence[..<r.lowerBound])
                fill.after = String(sentence[r.upperBound...])
                fill.answerText = token
                fill.hint = "the word for “\(gloss)”"
                out.append(fill)
            }
            if glossed.count >= 3 {
                var pick = Exercise(kind: .readMeaning)
                pick.text = token
                let others = glossed.map(\.1).filter { $0 != gloss }.shuffled().prefix(2)
                let options = ([gloss] + others).shuffled()
                pick.options = options
                pick.answerIndex = options.firstIndex(of: gloss)
                out.append(pick)
            }
        }
        for w in brief.words.prefix(3) where writable(w.rung.example) {
            var d = Exercise(kind: .dictation)
            d.text = w.rung.example
            out.append(d)
        }
        let intro = brief.passage == nil
            ? "Nothing in your book reads easily yet, so here are its words, one by one, in their own sentences."
            : "A few sentences of your book, read as written — then its words, in their own sentences."
        return (brief.goal.title, intro, out)
    }
}
