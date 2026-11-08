import XCTest
@testable import Drussky

/// Human narration of library texts: the word-timing files, the read-along page, and the
/// highlight arithmetic (Narration.swift).
@MainActor
final class NarrationTests: XCTestCase {
    private func narratedTexts() -> [Library.Text] { Library.load().texts.filter(\.isNarrated) }

    func testEveryNarratedTextHasSoundTimingsAndProvenance() throws {
        let audio = try XCTUnwrap(Narration.audioDir)
        let index = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: audio.appendingPathComponent("index-narration.json")))
        let sources = try JSONDecoder().decode([String: [String: AnyCodable]].self, from: Data(contentsOf: audio.appendingPathComponent("sources-narration.json")))
        let texts = narratedTexts()
        XCTAssertFalse(texts.isEmpty, "the library ships narrated texts")
        for t in texts {
            let timings = try XCTUnwrap(Narration.timings(try XCTUnwrap(t.words)), t.id)
            XCTAssertEqual(timings.problems(for: t.text), [], t.id)
            XCTAssertEqual(timings.file, t.narration, t.id)
            XCTAssertEqual(timings.text, t.id)
            XCTAssertEqual(index["narration:\(t.id)"], t.narration, "\(t.id) is in index-narration.json")
            let entry = try XCTUnwrap(sources[t.narration ?? ""], "\(t.id) has provenance")
            for field in ["license", "source_page", "reader", "cut", "orig_url", "pipeline"] {
                XCTAssertNotNil(entry[field], "\(t.id): sources entry has \(field)")
            }
            XCTAssertNotNil(Narration.url(t.narration ?? ""), "\(t.id): clip \(t.narration ?? "") is bundled (audio.py restore)")
            // Every word in the text is said within the clip at a plausible pace.
            XCTAssertGreaterThan(timings.words.count, 20, t.id)
            let pace = Double(t.text.count) / timings.duration
            XCTAssertTrue((6...20).contains(pace), "\(t.id): \(pace) characters per second")
        }
    }

    func testPassageWithNarrationRoundTrips() throws {
        let json = #"{"kind":"dictation","text":"Муравей сел на ветку и спасся.","narration":"narr-x.m4a","words":"narr-x.words.json","span":[26,31]}"#
        let ex = try JSONDecoder().decode(Exercise.self, from: Data(json.utf8))
        XCTAssertEqual(ex.narration, "narr-x.m4a")
        XCTAssertEqual(ex.span, [26, 31])
        let round = try JSONDecoder().decode(Exercise.self, from: JSONEncoder().encode(ex))
        XCTAssertEqual(round, ex)
        let plain = try JSONDecoder().decode(Exercise.self, from: Data(#"{"kind":"passage","text":"Лев спал."}"#.utf8))
        XCTAssertNil(plain.narration)
        XCTAssertNil(plain.span)
    }

    func testWordTimingsDecodeTheCompactArrayForm() throws {
        let json = #"{"version":1,"text":"t","file":"t.m4a","duration":3.5,"reader":"R","words":[["Лев",0.5,0.8],["спал",0.9,1.4]]}"#
        let t = try JSONDecoder().decode(WordTimings.self, from: Data(json.utf8))
        XCTAssertEqual(t.words, [.init("Лев", 0.5, 0.8), .init("спал", 0.9, 1.4)])
        XCTAssertEqual(t.problems(for: "Лев спал."), [])
        XCTAssertFalse(t.problems(for: "Лев не спал.").isEmpty)
        let back = try JSONDecoder().decode(WordTimings.self, from: JSONEncoder().encode(t))
        XCTAssertEqual(back, t)
    }

    func testTimedTokensFollowTheScriptsRule() {
        let text = "В 1816 году, в мае месяце, — случилось мне проезжать через ***скую губернию.\n«Что э́то?» — спроси́л он."
        let timed = Narration.timedTokens(text)
        XCTAssertEqual(timed.map(\.word), ["В", "году", "в", "мае", "месяце", "случилось", "мне", "проезжать", "через", "скую", "губернию", "Что", "это", "спросил", "он"])
        XCTAssertEqual(timed[0].tokenIndex, 0)
        XCTAssertEqual(timed[1].tokenIndex, 2, "1816 is a token without a timing")
        XCTAssertEqual(Narration.bare("э́то?»"), "это", "stress marks come off the bare word")
    }

    func testHighlightIndexAtPlaybackTime() {
        let words: [WordTimings.Word] = [.init("а", 0.5, 0.8), .init("б", 0.9, 1.2), .init("в", 1.6, 2.0)]
        XCTAssertNil(Narration.wordIndex(at: 0.3, in: words), "before the first word")
        XCTAssertEqual(Narration.wordIndex(at: 0.5, in: words), 0)
        XCTAssertEqual(Narration.wordIndex(at: 0.6, in: words), 0)
        XCTAssertEqual(Narration.wordIndex(at: 0.85, in: words), 0, "a word stays lit through the pause after it")
        XCTAssertEqual(Narration.wordIndex(at: 0.9, in: words), 1)
        XCTAssertEqual(Narration.wordIndex(at: 1.4, in: words), 1)
        XCTAssertEqual(Narration.wordIndex(at: 1.7, in: words), 2)
        XCTAssertEqual(Narration.wordIndex(at: 2.0, in: words), 2)
        XCTAssertNil(Narration.wordIndex(at: 2.5, in: words), "after the last word")
        XCTAssertNil(Narration.wordIndex(at: 1.0, in: []))
    }

    func testChunksEndAtSentencesAndLines() {
        let prose = "Муравей спустился к ручью: захотел напиться. Волна захлестнула его! Голубка несла ветку…\nПотом охотник расставил сеть"
        XCTAssertEqual(Narration.chunks(prose), [0...5, 6...8, 9...11, 12...15])
        XCTAssertEqual(Narration.chunkText(prose, 6...8), "Волна захлестнула его!")
        XCTAssertEqual(Narration.chunkText(prose, 12...15), "Потом охотник расставил сеть")
        let verse = "Мороз и солнце; день чудесный!\nЕще ты дремлешь, друг прелестный —\nПора, красавица, проснись:"
        XCTAssertEqual(Narration.chunks(verse), [0...4, 5...9, 10...12], "a verse line is a chunk even without a full stop")
        XCTAssertEqual(Narration.chunkText(verse, 5...9), "Еще ты дремлешь, друг прелестный", "a trailing dash is not part of what the learner writes")
    }

    func testMappingAStretchOfTheTextOntoTimings() {
        let full = "Лев спал. Мышь пробежала ему по телу. Он проснулся и поймал ее."
        let timings = WordTimings(version: 1, text: "t", file: "t.m4a", duration: 10, reader: nil,
                                  words: Narration.timedTokens(full).enumerated().map { .init($1.word, Double($0), Double($0) + 0.5) })
        XCTAssertEqual(Narration.map(full, onto: timings), Array(0..<12).map(Optional.some))
        // The tutor's passage is the middle sentence, with a stress mark added.
        XCTAssertEqual(Narration.map("Мышь пробежа́ла ему по телу.", onto: timings), [2, 3, 4, 5, 6])
        // A token without a letter has no timing; a changed word is left untimed, the rest still lands.
        XCTAssertEqual(Narration.map("Он — проснулся и схватил ее.", onto: timings), [7, nil, 8, 9, nil, 11])
        XCTAssertNil(Narration.map("Совсем другой текст про кота.", onto: timings))
    }

    func testIntervalOfAChunkStaysInsideItsNeighbours() {
        let timings = WordTimings(version: 1, text: "t", file: "t.m4a", duration: 10, reader: nil,
                                  words: [.init("а", 1.0, 1.4), .init("б", 1.5, 1.9), .init("в", 2.1, 2.5), .init("г", 4.0, 4.4)])
        let i = Narration.interval(1...2, in: timings)
        XCTAssertEqual(i.start, 1.4, accuracy: 0.001, "no earlier than the previous word's end")
        XCTAssertEqual(i.end, 2.85, accuracy: 0.001, "a short tail, well before the next word")
        let whole = Narration.interval(0...3, in: timings)
        XCTAssertEqual(whole.start, 0.75, accuracy: 0.001)
        XCTAssertEqual(whole.end, 4.75, accuracy: 0.001)
    }

    func testReadAlongPageIsBuiltFromTheTimings() throws {
        let text = try XCTUnwrap(narratedTexts().first, "a narrated text in the bundle")
        let page = Narration.page(for: text)
        XCTAssertEqual(page.exercises.first?.kind, .passage)
        XCTAssertEqual(page.exercises.first?.narration, text.narration)
        let chunks = page.exercises.dropFirst()
        XCTAssertEqual(chunks.count, 4)
        XCTAssertEqual(chunks.map(\.kind), [.dictation, .readAloud, .dictation, .readAloud])
        let timings = try XCTUnwrap(Narration.timings(try XCTUnwrap(text.words)))
        for ex in chunks {
            let span = try XCTUnwrap(ex.span)
            XCTAssertEqual(ex.text, Narration.chunkText(text.text, span[0]...span[1]))
            XCTAssertTrue((3...9).contains(span[1] - span[0] + 1))
            let i = try XCTUnwrap(Narration.interval(of: ex))
            XCTAssertLessThan(i.start, i.end)
            XCTAssertLessThanOrEqual(i.end, timings.duration + 0.01)
            XCTAssertTrue(text.text.contains(ex.text ?? "?"), "the chunk is the text as written")
        }
    }
}

/// Just enough to read sources-narration.json's mixed values in a test.
private struct AnyCodable: Decodable {
    init(from decoder: Decoder) throws { _ = try decoder.singleValueContainer() }
}
