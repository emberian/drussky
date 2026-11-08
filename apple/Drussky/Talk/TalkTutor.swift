import Foundation

/// The tutor's side of a conversation: three scenes to choose from, one spoken line per turn
/// (streamed, so the voice can start on the first sentence), and the red pen over the whole
/// transcript afterwards.
@MainActor
enum TalkTutor {
    /// Turns are short and must start speaking fast: Sonnet 5.5, measured as good as Opus on ten sample
    /// turns and a second sooner to the first word (README, "Talk"); the scenes call likewise. The
    /// marking is homework and may take its time: Opus.
    static let turnModel = PageWriter.rescaleModel
    static let scenesModel = PageWriter.rescaleModel
    static let markModel = Tutor.model

    enum Failure: LocalizedError {
        case unreadable, noScenes, offline
        var errorDescription: String? {
            switch self {
            case .unreadable: "The tutor's line didn't come back in a form I could read. Try again."
            case .noScenes: "The tutor couldn't think of a scene for your words yet. A few more pages and try again."
            case .offline: "Talk needs the internet — the tutor is on the other end of it."
            }
        }
    }

    // MARK: Where the learner is

    /// Words the learner has practised in the knowledge model or kept as review targets.
    static func knownWords(_ book: Book) -> [Course.Word] {
        var keys = Set(book.knowledge.practisedWords.map(\.key))
        for k in book.targets.keys where k.hasPrefix("word:") { keys.insert(String(k.dropFirst(5))) }
        return keys.sorted().compactMap { book.word($0) }
    }

    /// The learner's state, shared by every prompt: letters, words, what the model knows.
    static func state(_ book: Book) -> String {
        let met = book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper).joined(separator: " ")
        let known = knownWords(book)
            .prefix(200).map { "\($0.ru) = \($0.en)" }.joined(separator: "; ")
        let patterns = Lemmas.load().constructions.map { "\($0.id) (\($0.name))" }.joined(separator: ", ")
        return """
        Letters met: \(met.isEmpty ? "none" : met).
        Words they know: \(known.isEmpty ? "none yet" : known).
        Construction ids: \(patterns).
        \(book.knowledge.summary(limit: 5))
        """
    }

    // MARK: Scenes

    /// Every scene is an information gap: the tutor holds facts the learner lacks, the learner
    /// has something to find out or get done, so there is a reason to listen and to answer.
    static let scenesSystem = """
    You are the tutor inside Drussky, a Russian exercise book used by one adult English speaker, Ember: quick, \
    curious, a beginner in Russian who wants to actually speak it. You are about to have a short spoken \
    conversation with them in Russian. Propose THREE small everyday scenes they could hold a 6–10 turn \
    conversation in using ONLY the words they know (plus a word or two you would teach). Each scene is an \
    information gap: YOU (the partner) know something they don't (where the keys are, your schedule, which film \
    you prefer, what the shop has), and THEY have a communicative goal that needs your information or agreement. \
    Reply with ONE JSON object and nothing else:
    {"scenes":[{"title":"At breakfast","ru":"За за́втраком",
                "goal":"Find out what there is to drink and order one thing",
                "frames":[{"ru":"Что есть …?","en":"what is there …?"},{"ru":"Я хочу́ …","en":"I want …"}],
                "secret":"There is tea and juice but no coffee; the milk is finished"}, ...]}
    Titles in plain English, 2–5 words; `ru` the same in Russian with stress marks (U+0301) on words of 2+ \
    syllables, never on ё or one-syllable words. `goal` is one sentence in English the learner can act on. `frames` are 2–3 phrase \
    frames they may need, in Russian with a gap (…) and stress marks, built from THEIR words first, each with its \
    English. `secret` is what you know and they don't, in English, 1–2 sentences, concrete, answerable with their \
    words. Vary the three: a place, a situation, a topic (e.g. "about a book you read"). No preamble.
    """

    static func proposeScenes(book: Book) async throws -> [TalkScene] {
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: scenesState(book) + "\nPropose the three scenes.")],
                                            system: scenesSystem, maxTokens: 2000, effort: "low", model: scenesModel) {
            raw += chunk
        }
        let scenes = parseScenes(raw)
        guard !scenes.isEmpty else { throw Failure.noScenes }
        return scenes
    }

    /// Scenes need only the words (not the constructions or the knowledge summary): a smaller
    /// prompt, measurably faster to first token (README, "Talk").
    static func scenesState(_ book: Book) -> String {
        let known = knownWords(book)
            .prefix(200).map { "\($0.ru) = \($0.en)" }.joined(separator: "; ")
        return "Words they know: \(known.isEmpty ? "none yet" : known)."
    }

    static func parseScenes(_ reply: String) -> [TalkScene] {
        guard let json = PageWriter.extractJSON(reply), let data = json.data(using: .utf8),
              let any = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return [] }
        func str(_ v: Any?) -> String? {
            guard let s = (v as? String)?.trimmingCharacters(in: .whitespacesAndNewlines), !s.isEmpty else { return nil }
            return s
        }
        var out: [TalkScene] = []
        for item in (any["scenes"] as? [Any]) ?? [] {
            guard let s = item as? [String: Any], let title = str(s["title"]) else { continue }
            var scene = TalkScene(title: title, ru: str(s["ru"]) ?? "")
            scene.goal = str(s["goal"])
            scene.secret = str(s["secret"] ?? s["facts"])
            let frames = ((s["frames"] as? [Any]) ?? []).compactMap { f -> TalkFrame? in
                if let d = f as? [String: Any], let ru = str(d["ru"]) { return TalkFrame(ru: ru, en: str(d["en"]) ?? "") }
                if let line = f as? String, !RussianText.letters(in: line).isEmpty {
                    let bits = line.split(separator: "=", maxSplits: 1).map { $0.trimmingCharacters(in: .whitespaces) }
                    return TalkFrame(ru: bits[0], en: bits.count > 1 ? bits[1] : "")
                }
                return nil
            }
            scene.frames = frames.isEmpty ? nil : Array(frames.prefix(3))
            if !out.contains(where: { $0.title == title }) { out.append(scene) }
        }
        return Array(out.prefix(3))
    }

    // MARK: Turns

    /// The reply format is line-based, not JSON, so the Russian can be spoken sentence by
    /// sentence while the rest of the reply is still arriving.
    static func turnSystem(scene: TalkScene, state: String) -> String {
        """
        You are the tutor inside Drussky, a Russian exercise book, having a short SPOKEN conversation in Russian \
        with one adult English speaker, Ember: quick, curious, a beginner who wants to speak and understand real \
        Russian. The app reads your Russian aloud with a synthetic voice and listens to their answer through speech \
        recognition, so what you receive of their reply is what the recognizer heard, and may be garbled.

        Scene: \(scene.title) (\(scene.ru)).
        What you know and they don't (your private facts; never read them out unasked, answer from them when \
        asked, and stay consistent with them): \(scene.secret ?? "decide small concrete facts yourself and keep to them").
        Their goal (they were told it in English; you were not told to help them reach it): \(scene.goal ?? "to hold the conversation").
        You are a partner, not an oracle: respond to the meaning they actually expressed, with your facts. If a \
        reply is unclear or doesn't answer, ask them to clarify («Прости́те, что?», «Чай и́ли ко́фе?») rather than \
        guessing kindly. If they ask for something your facts rule out, say so and offer what there is. You are \
        not obliged to agree. Let them reach the goal only through what they manage to say.

        Rules for every turn:
        - RU: what you say — Russian only, 1–2 short natural sentences, at most 12 words in all, built from the words \
        they know plus AT MOST TWO new words (list those in NEW). Usually end with a question they can answer with \
        words they have. Stress marks (U+0301) on every word of 2+ syllables, never on ё or one-syllable words. \
        Address the learner consistently as вы (as their Russian tutor; never mix ты and вы).
        - If their reply is close to a sensible answer, respond to its meaning. If a word is wrong or unclear, \
        repair gently inside RU with «Вы хоте́ли сказа́ть …?» and the right form, then carry on. Never explain in \
        English inside RU. If nothing was heard, ask again more simply.
        - EN: a natural English translation of RU (the learner reveals it only when stuck).
        - NEW: the new words as "сло́во = word; друго́е = other", or omit the line when there are none.
        - You will be told the turn number. When told it is the LAST turn, say goodbye warmly in RU and put END \
        on its own line after everything.
        Reply format, exactly these lines and nothing else:
        RU: …
        EN: …
        NEW: …
        END

        Where the learner is now:
        \(state)
        """
    }

    /// The conversation so far as API turns; the latest instruction is the last user turn.
    static func turns(for c: TalkConversation, dialogue: TalkDialogue) -> [Tutor.Turn] {
        var out: [Tutor.Turn] = [.init(role: "user", text: "Turn 1 of \(dialogue.totalTurns + 1). Begin the conversation: greet them and open the scene.")]
        var k = 1
        for line in c.lines {
            switch line.role {
            case .tutor:
                out.append(.init(role: "assistant", text: replyText(line)))
            case .learner:
                k += 1
                let heard = line.ru.isEmpty ? "(nothing was heard)" : "They said (as heard): «\(line.ru)»"
                let last = k > dialogue.totalTurns || dialogue.isClosing
                out.append(.init(role: "user", text: heard + (last ? "\nThis is the LAST turn (\(k) of \(dialogue.totalTurns + 1)): say goodbye and end with END."
                                                                      : "\nTurn \(k) of \(dialogue.totalTurns + 1).")))
            }
        }
        // The API alternates roles; two learner lines in a row (a retry) fold into one.
        var merged: [Tutor.Turn] = []
        for t in out {
            if let last = merged.last, last.role == t.role { merged[merged.count - 1].text += "\n" + t.text } else { merged.append(t) }
        }
        if merged.last?.role == "assistant" {
            merged.append(.init(role: "user", text: dialogue.isClosing ? "This is the LAST turn: say goodbye and end with END." : "Go on."))
        }
        return merged
    }

    /// A tutor line as it was (or would have been) replied, so the history reads consistently.
    static func replyText(_ line: TalkLine) -> String {
        var s = "RU: \(line.ru)\nEN: \(line.en ?? "")"
        if let words = line.newWords, !words.isEmpty { s += "\nNEW: " + words.map { "\($0.ru) = \($0.en)" }.joined(separator: "; ") }
        return s
    }

    /// What a turn's reply says once complete.
    struct Reply: Equatable, Sendable {
        var ru = ""
        var en = ""
        var newWords: [Course.Word] = []
        var ended = false
    }

    /// Reads a streamed reply line by line and hands out each complete Russian sentence as soon
    /// as its end arrives, so speech starts before the reply is finished. Lenient: a reply with
    /// no `RU:` prefix is read as Russian up to the first `EN:`.
    struct ReplyParser: Sendable {
        private var buffer = ""
        private var ruSoFar = ""
        /// Sentences already handed out, by their end offset in `ruSoFar`.
        private var spokenUpTo = 0
        private(set) var reply = Reply()
        private var ruClosed = false

        /// Feeds a chunk; returns the newly completed Russian sentences.
        mutating func feed(_ chunk: String) -> [String] {
            buffer += chunk
            var out: [String] = []
            while let nl = buffer.firstIndex(of: "\n") {
                let line = String(buffer[..<nl])
                buffer = String(buffer[buffer.index(after: nl)...])
                out += take(line: line, complete: true)
            }
            // The RU line may still be arriving: speak its finished sentences already.
            if !ruClosed, isRussianLine(buffer) {
                out += sentences(fromRU: content(of: buffer), final: false)
            }
            return out
        }

        /// The stream ended: whatever is left is read, and the last sentence spoken.
        mutating func finish() -> [String] {
            var out: [String] = []
            if !buffer.isEmpty { out += take(line: buffer, complete: true); buffer = "" }
            if !ruClosed { out += sentences(fromRU: ruSoFar, final: true); ruClosed = true }
            reply.ru = ruSoFar.trimmingCharacters(in: .whitespacesAndNewlines)
            return out
        }

        private func isRussianLine(_ line: String) -> Bool {
            let t = line.trimmingCharacters(in: .whitespaces)
            if t.uppercased().hasPrefix("RU:") { return true }
            // No prefix yet: Russian text before anything else is the spoken line.
            return ruSoFar.isEmpty && !t.isEmpty && !t.uppercased().hasPrefix("EN") && !t.uppercased().hasPrefix("NEW")
                && !t.uppercased().hasPrefix("END") && !RussianText.letters(in: t).isEmpty
        }

        private func content(of line: String) -> String {
            let t = line.trimmingCharacters(in: .whitespaces)
            if let r = t.range(of: ":"), t[..<r.lowerBound].count <= 3 { return String(t[r.upperBound...]).trimmingCharacters(in: .whitespaces) }
            return t
        }

        private mutating func take(line raw: String, complete: Bool) -> [String] {
            let line = raw.trimmingCharacters(in: .whitespaces)
            guard !line.isEmpty else { return [] }
            let upper = line.uppercased()
            if upper == "END" || upper.hasPrefix("END:") || upper.hasPrefix("END ") {
                reply.ended = true
                if !ruClosed { ruClosed = true; return sentences(fromRU: ruSoFar, final: true) }
                return []
            }
            if upper.hasPrefix("EN:") {
                reply.en = content(of: line)
                if !ruClosed { ruClosed = true; return sentences(fromRU: ruSoFar, final: true) }
                return []
            }
            if upper.hasPrefix("NEW:") {
                reply.newWords = Self.words(content(of: line))
                if !ruClosed { ruClosed = true; return sentences(fromRU: ruSoFar, final: true) }
                return []
            }
            if isRussianLine(line) || (!ruClosed && ruSoFar.isEmpty) {
                // Prose around the line (a stray sentence) is ignored unless it is the Russian itself.
                guard !RussianText.letters(in: line).isEmpty else { return [] }
                let text = content(of: line)
                let out = sentences(fromRU: text, final: true)
                ruClosed = true
                return out
            }
            // A second Russian line after RU (the model wrapped): still part of what is said.
            if !ruClosed, !RussianText.letters(in: line).isEmpty {
                return sentences(fromRU: ruSoFar + " " + line, final: true)
            }
            return []
        }

        /// Updates `ruSoFar` to `text` and returns sentences completed since the last call.
        private mutating func sentences(fromRU text: String, final: Bool) -> [String] {
            ruSoFar = text
            let chars = Array(text)
            var out: [String] = []
            var start = spokenUpTo
            var i = start
            while i < chars.count {
                let c = chars[i]
                if ".!?…".contains(c) {
                    // Swallow closing quotes/brackets and repeated terminators right after.
                    var end = i + 1
                    while end < chars.count, "»)\"'.!?…".contains(chars[end]) { end += 1 }
                    // A terminator is only final once something follows it (or the stream ends),
                    // so «Что́ это?» isn't cut before its closing quote arrives.
                    if end < chars.count || final {
                        let s = String(chars[start..<end]).trimmingCharacters(in: .whitespaces)
                        if !s.isEmpty { out.append(s) }
                        start = end
                        i = end
                        continue
                    }
                }
                i += 1
            }
            if final, start < chars.count {
                let s = String(chars[start...]).trimmingCharacters(in: .whitespaces)
                if !s.isEmpty { out.append(s) }
                start = chars.count
            }
            spokenUpTo = start
            return out
        }

        /// "сло́во = word; друго́е = other" → words.
        static func words(_ s: String) -> [Course.Word] {
            s.split(whereSeparator: { $0 == ";" || $0 == "," }).compactMap { part in
                let bits = part.split(separator: "=", maxSplits: 1).map { $0.trimmingCharacters(in: .whitespaces) }
                guard let ru = bits.first, !RussianText.letters(in: ru).isEmpty else { return nil }
                let en = bits.count > 1 ? bits[1] : ""
                return Course.Word(ru: ru, en: en, emoji: nil, note: "from a conversation")
            }
        }
    }

    /// Streams one tutor turn. `onSentence` fires on the main actor for each completed Russian
    /// sentence as it arrives (the voice starts there); the complete reply is returned.
    static func turn(_ c: TalkConversation, dialogue: TalkDialogue, book: Book, onFirstToken: @escaping @MainActor () -> Void = {},
                     onSentence: @escaping @MainActor (String) -> Void) async throws -> Reply {
        guard let scene = c.scene else { throw Failure.unreadable }
        var parser = ReplyParser()
        var first = true
        var raw = ""
        for try await chunk in Tutor.stream(turns(for: c, dialogue: dialogue), system: turnSystem(scene: scene, state: state(book)),
                                            maxTokens: 4000, effort: "low", model: turnModel) {
            if first { first = false; onFirstToken() }
            raw += chunk
            for s in parser.feed(chunk) { onSentence(s) }
            try Task.checkCancellation()
        }
        for s in parser.finish() { onSentence(s) }
        try Task.checkCancellation()  // a cancelled stream ends empty rather than throwing
        var reply = parser.reply
        if RussianText.letters(in: reply.ru).isEmpty {
            // The reply came in some other shape: the first line with Russian in it is what is said.
            guard let line = raw.split(separator: "\n").map({ $0.trimmingCharacters(in: .whitespaces) })
                .first(where: { !RussianText.letters(in: $0).isEmpty && !$0.uppercased().hasPrefix("EN") }) else {
                #if DEBUG
                print("TALK unreadable reply:", raw)
                #endif
                throw Failure.unreadable
            }
            reply.ru = line.replacing(/^\**(RU|Ru|ru)\**\s*:\**\s*/, with: "")
            reply.ended = reply.ended || raw.uppercased().contains("\nEND")
            onSentence(reply.ru)
        }
        return reply
    }

    // MARK: The red pen

    static let markSystem = """
    You are the tutor inside Drussky, a Russian exercise book used by one adult English speaker, Ember: quick, \
    curious, a beginner in Russian. You have just had a short spoken conversation with them in Russian. Below is \
    the transcript: your lines, and their lines as SPEECH RECOGNITION heard them. Now mark their lines the way a \
    kind, exact teacher marks homework in red pen, keeping in mind that it was spoken: be lenient about anything \
    the recognizer could have misheard (a homophone, an unstressed vowel, a dropped soft sign); mark what they \
    clearly said wrong (a wrong ending, agreement, word choice, word order), accept natural spoken Russian word \
    order and omitted subject pronouns, and praise what was right.
    First, before any grammar: did they do what they set out to do? You are told their goal; judge it ONLY from \
    what they said and how the partner answered in the transcript, never from anything you imagine the partner \
    knew. Say what they managed to convey, and what the partner had to ask them to clarify, if anything.
    Reply with ONE JSON object and nothing else:
    {"goal": {"reached": true, "conveyed": "1–2 sentences in English: what they got across and whether the goal was met",
              "clarified": "what the partner had to ask them to repeat or clarify, or ''"},
     "lines": [{"n": 1, "corrected": "their line, corrected, with stress marks (U+0301) on words of 2+ syllables, never on ё or one-syllable words",
                "edits": [{"span": "their words exactly as heard", "replacement": "the right form, stress-marked ('' to delete)",
                           "category": "spelling|ending|agreement|word-choice|word-order|stress|punctuation|other",
                           "why": "one line in English saying what differs and why", "en": "English meaning of the replacement",
                           "pattern": "a construction id from the list when one explains it, else omit"}],
                "praise": "a few words about what was right in this line, or ''"}],
     "praise": "one specific, warm sentence about the conversation as a whole",
     "teach": [{"title": "the pattern behind the biggest slip, in 3-6 words",
                "note": "at most 2 sentences teaching it, with one example marked [[ру́сский|english]]",
                "pattern": "construction id when one fits, else a short slug"}]}
    Rules: one object in `lines` per numbered learner line, in order, none skipped (a line with nothing to fix has \
    `edits`: []). Every `span` must be a verbatim substring of that line as heard; edits never overlap. Keep their \
    meaning and their words wherever they are right; never restyle. 0–2 teach items for the whole conversation. \
    No preamble, no sign-off.
    """

    /// The transcript the marker reads: learner lines numbered.
    static func transcript(_ c: TalkConversation) -> String {
        var n = 0
        return c.lines.map { line in
            switch line.role {
            case .tutor: return "Tutor: \(line.ru)"
            case .learner:
                n += 1
                return "Learner \(n): \(line.ru.isEmpty ? "(nothing heard)" : line.ru)"
            }
        }.joined(separator: "\n")
    }

    struct Marking: Sendable {
        /// Corrections by learner-line number (1-based), only for lines that had text.
        var lines: [Int: DiaryCorrection]
        var praise: String
        var teach: [DiaryTeach]
        var outcome: TalkOutcome?
    }

    static func mark(_ c: TalkConversation, book: Book) async throws -> Marking {
        let prompt = """
        \(state(book))

        Scene: \(c.scene?.title ?? "").
        Their goal: \(c.scene?.goal ?? "to hold the conversation").
        Transcript:
        ---
        \(transcript(c))
        ---
        """
        var raw = ""
        for try await chunk in Tutor.stream([.init(role: "user", text: prompt)], system: markSystem, maxTokens: 6000,
                                            effort: "medium", model: markModel) {
            raw += chunk
            try Task.checkCancellation()
        }
        guard let m = parseMarking(raw, conversation: c, at: book.now()) else { throw Failure.unreadable }
        return m
    }

    /// Reads the marking JSON; each line's object goes through the diary's tolerant parser so the
    /// two red pens can never disagree about a field.
    static func parseMarking(_ reply: String, conversation c: TalkConversation, at date: Date) -> Marking? {
        guard let json = PageWriter.extractJSON(reply), let data = json.data(using: .utf8),
              let any = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        let heard = c.learnerLines.map(\.ru)
        var lines: [Int: DiaryCorrection] = [:]
        for (i, item) in ((any["lines"] as? [Any]) ?? []).enumerated() {
            guard let obj = item as? [String: Any] else { continue }
            let n = (obj["n"] as? NSNumber)?.intValue ?? (Int((obj["n"] as? String) ?? "") ?? i + 1)
            guard heard.indices.contains(n - 1), !heard[n - 1].isEmpty,
                  let d = try? JSONSerialization.data(withJSONObject: obj), let s = String(data: d, encoding: .utf8),
                  let correction = DiaryTutor.parse(s, fallbackText: heard[n - 1], at: date) else { continue }
            lines[n] = correction
        }
        let praise = ((any["praise"] as? String) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        var teach: [DiaryTeach] = []
        for item in (any["teach"] as? [Any]) ?? [] {
            guard let t = item as? [String: Any], let title = (t["title"] as? String), !title.isEmpty else { continue }
            let pattern = (t["pattern"] as? String).flatMap { $0.isEmpty ? nil : $0.replacingOccurrences(of: "gram:", with: "") }
            teach.append(DiaryTeach(title: title, note: (t["note"] as? String) ?? "", pattern: pattern))
        }
        var outcome: TalkOutcome?
        if let g = any["goal"] as? [String: Any] {
            let conveyed = ((g["conveyed"] as? String) ?? (g["summary"] as? String) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            let clarified = ((g["clarified"] as? String) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            let reached: Bool? = (g["reached"] as? Bool) ?? (g["reached"] as? NSNumber)?.boolValue
                ?? (g["reached"] as? String).map { ["true", "yes"].contains($0.lowercased()) }
            if !conveyed.isEmpty || reached != nil {
                outcome = TalkOutcome(reached: reached, conveyed: conveyed, clarified: clarified.isEmpty ? nil : clarified)
            }
        }
        return Marking(lines: lines, praise: praise, teach: Array(teach.prefix(2)), outcome: outcome)
    }
}
