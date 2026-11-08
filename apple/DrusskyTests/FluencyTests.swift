import LearningCore
import PencilKit
import XCTest
@testable import Drussky

/// Fluency instruments: words a minute from onsets, strokes a second from ink, the series on
/// disk, the line on the cover.
@MainActor
final class FluencyTests: XCTestCase {
    func course() throws -> Course {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        return try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
    }

    func tempStore() -> BookStore {
        BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
    }

    let t0 = Date(timeIntervalSince1970: 1_800_000_000)
    func day(_ n: Double) -> Date { t0.addingTimeInterval(n * 86_400) }

    func testWordsPerMinuteFromOnsets() {
        // Six words, syllable onsets every 0.25 s over 3 s of speech, speech ending at 3.0: 120 wpm.
        let onsets = stride(from: 0.0, through: 2.75, by: 0.25).map { $0 }
        XCTAssertEqual(Fluency.wpm(words: 6, onsets: onsets, end: 3.0)!, 120, accuracy: 1e-9)
        // Without the end, the last unit gets the mean gap: the same 3 s.
        XCTAssertEqual(Fluency.wpm(words: 6, onsets: onsets)!, 120, accuracy: 1e-9)
        // Onsets need not be sorted; speech start offset does not matter.
        XCTAssertEqual(Fluency.wpm(words: 6, onsets: onsets.reversed().map { $0 + 1.5 }, end: 4.5)!, 120, accuracy: 1e-9)
        // One onset and no end: nothing to measure. No words: nothing.
        XCTAssertNil(Fluency.wpm(words: 3, onsets: [0.4]))
        XCTAssertNil(Fluency.wpm(words: 0, onsets: onsets, end: 3))
        XCTAssertNil(Fluency.wpm(words: 3, onsets: []))
        // An end before the first onset is ignored in favour of the onsets.
        XCTAssertEqual(Fluency.wpm(words: 6, onsets: onsets, end: -1)!, 120, accuracy: 1e-9)
        // A finger pass: 40 words in 30 s.
        XCTAssertEqual(Fluency.wpm(words: 40, seconds: 30)!, 80, accuracy: 1e-9)
        XCTAssertNil(Fluency.wpm(words: 40, seconds: 0.1), "a tap is not a reading")
        XCTAssertEqual(Fluency.wordCount("Робот просну́лся но́чью. — На станции 3 было тихо…"), 7)
        XCTAssertEqual(Fluency.passageID("Ро́бот проснулся ночью. На станции было тихо."),
                       Fluency.passageID("робот проснулся ночью на станции было тихо"))
        XCTAssertNotEqual(Fluency.passageID("Ро́бот проснулся ночью. На станции было тихо."),
                          Fluency.passageID("Ро́бот проснулся ночью. На улице шёл снег."),
                          "passages sharing their first four words must not collide")
        XCTAssertNotEqual(Fluency.passageID("Робот проснулся"), Fluency.passageID("Кот проснулся"))
    }

    func testStrokesPerSecondFromInkTiming() {
        let ink = PKInk(.pen, color: .black)
        func stroke(at start: Date, lasting: Double, x: CGFloat) -> PKStroke {
            let points = (0...4).map { i in
                PKStrokePoint(location: CGPoint(x: x + CGFloat(i) * 3, y: 50 + CGFloat(i)), timeOffset: lasting * Double(i) / 4,
                              size: CGSize(width: 3, height: 3), opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
            }
            return PKStroke(ink: ink, path: PKStrokePath(controlPoints: points, creationDate: start))
        }
        // Four strokes over two seconds, pauses included: 2 strokes a second.
        let d = PKDrawing(strokes: [stroke(at: t0, lasting: 0.3, x: 10), stroke(at: t0.addingTimeInterval(0.5), lasting: 0.3, x: 30),
                                    stroke(at: t0.addingTimeInterval(1.2), lasting: 0.3, x: 50), stroke(at: t0.addingTimeInterval(1.7), lasting: 0.3, x: 70)])
        XCTAssertEqual(WritingFluency.strokesPerSecond(d)!, 2, accuracy: 1e-6)
        // One stroke of half a second: its own duration counts.
        XCTAssertEqual(WritingFluency.strokesPerSecond(PKDrawing(strokes: [stroke(at: t0, lasting: 0.5, x: 10)]))!, 2, accuracy: 1e-6)
        XCTAssertNil(WritingFluency.strokesPerSecond(PKDrawing()))
        XCTAssertNil(WritingFluency.strokesPerSecond(PKDrawing(strokes: [stroke(at: t0, lasting: 0.05, x: 10)])), "no time in it")
    }

    func testLegibility() {
        XCTAssertEqual(Fluency.legibility(matched: true, confidence: 0.9, reading: "кот", target: "кот"), 0.9)
        XCTAssertEqual(Fluency.legibility(matched: true, confidence: nil, reading: "кот", target: "кот"), 0.75)
        XCTAssertEqual(Fluency.legibility(matched: true, confidence: 0.1, reading: "кот", target: "кот"), 0.5, "a match is at least half legible")
        XCTAssertEqual(Fluency.legibility(matched: false, confidence: 0.9, reading: "кат", target: "ко́т"), 0.5 - 0.5 / 3, accuracy: 1e-9)
        XCTAssertEqual(Fluency.legibility(matched: false, confidence: nil, reading: "", target: "кот"), 0)
        XCTAssertEqual(Fluency.legibility(matched: false, confidence: nil, reading: "xyzw", target: "кот"), 0, "nothing of it survived")
    }

    func testTrackerKeepsSeriesOnDiskAndNotesMoments() throws {
        let store = tempStore()
        let book = Book(course: try course(), store: store)
        let word = ComponentID(.lexeme, "кот")
        var ex = Exercise(kind: .translate)
        ex.answerText = "кот"
        let ink = PKInk(.pen, color: .black)
        let points = (0...4).map { i in
            PKStrokePoint(location: CGPoint(x: 10 + CGFloat(i) * 3, y: 50), timeOffset: Double(i) * 0.2,
                          size: CGSize(width: 3, height: 3), opacity: 1, force: 1, azimuth: 0, altitude: .pi / 2)
        }
        let drawing = PKDrawing(strokes: [PKStroke(ink: ink, path: PKStrokePath(controlPoints: points, creationDate: t0))])
        book.fluency.noteWriting(exercise: ex, drawing: drawing, matched: true, reading: "кот", confidence: 0.8, component: word)
        XCTAssertEqual(book.fluency.store.latest(Fluency.strokesPerSecond, of: word)!.value, 1 / 0.8, accuracy: 1e-5)  // PencilKit keeps Float offsets
        XCTAssertEqual(book.fluency.store.latest(Fluency.legibility, of: Fluency.writing)?.value, 0.8)
        // Copying is not measured.
        var copy = Exercise(kind: .copy); copy.text = "кот"
        book.fluency.noteWriting(exercise: copy, drawing: drawing, matched: true, reading: "кот", confidence: 1, component: word)
        XCTAssertEqual(book.fluency.store.samples(Fluency.strokesPerSecond, of: word).count, 1)

        // A take: five words said over 2.5 s of speech.
        let report = PronunciationReport(target: "я читаю книгу о роботах", transcript: nil, expected: [], heard: [], sounds: [],
                                         syllables: stride(from: 0.5, through: 2.5, by: 0.25).map { SyllableJudgement(text: "", isStressed: false, start: $0, end: $0 + 0.2, prominence: nil) },
                                         stressedSyllable: nil, accuracy: 1, tips: [], speechStart: 0.5, speechEnd: 3.0)
        book.fluency.noteTake(target: "я читаю книгу о роботах", report: report, component: nil)
        XCTAssertEqual(book.fluency.store.latest(Fluency.sayWPM, of: Fluency.speaking)!.value, 120, accuracy: 1e-9)
        book.fluency.noteTake(target: "кот", report: report, component: word)
        XCTAssertNil(book.fluency.store.latest(Fluency.sayWPM, of: word), "one word is not a pace")

        // A finger pass.
        XCTAssertEqual(book.fluency.noteReadingPass(text: "Робот проснулся ночью. На станции было тихо.", seconds: 6)!, 70, accuracy: 1e-9)
        XCTAssertEqual(book.fluency.lastReads(of: "Робот проснулся ночью. На станции было тихо.").last, 70)

        // Listening: Book.record notes a narrated dictation finishing, with the reader's rate.
        var narrated = Exercise(kind: .dictation)
        narrated.text = "На станции было тихо."; narrated.narration = "narr-x.m4a"
        var r0 = ExerciseResult(), r1 = ExerciseResult(); r1.status = .correct
        book.fluency.observe(narrated, before: r0, after: r1, rate: 0.75)
        r1.status = .missed
        book.fluency.observe(narrated, before: r0, after: r1, rate: 1.0)
        r0.status = .correct
        book.fluency.observe(narrated, before: r0, after: r1, rate: 1.0)  // already finished: nothing
        var plain = Exercise(kind: .dictation); plain.text = "кот"
        book.fluency.observe(plain, before: ExerciseResult(), after: r1, rate: 1.0)  // not narrated: nothing
        XCTAssertEqual(book.fluency.store.samples(Fluency.listening, of: Fluency.listeningSkill, note: "0.75").map(\.value), [1])
        XCTAssertEqual(book.fluency.store.samples(Fluency.listening, of: Fluency.listeningSkill, note: "1").map(\.value), [0])

        // Everything reads back from metrics.jsonl.
        XCTAssertTrue(FileManager.default.fileExists(atPath: store.root.appendingPathComponent("metrics.jsonl").path))
        let again = Book(course: try course(), store: store)
        XCTAssertEqual(again.fluency.store.records.count, book.fluency.store.records.count)
        XCTAssertEqual(again.fluency.store.records.map(\.value), book.fluency.store.records.map(\.value))
        XCTAssertEqual(again.fluency.store.records.map(\.component), book.fluency.store.records.map(\.component))
        XCTAssertEqual(again.fluency.store.samples(Fluency.listening, of: Fluency.listeningSkill, note: "0.75").map(\.value), [1])
        XCTAssertEqual(again.fluency.store.latest(Fluency.readWPM, of: Fluency.reading)?.value, 70)
    }

    func testCoverLineIsHonestAboutDirectionAndSilence() {
        var s = MetricStore()
        let now = day(0)
        let label: (ComponentID) -> String = { $0.key }
        XCTAssertNil(Fluency.coverLine(s, now: now, label: label), "nothing measured, nothing said")
        for (d, v) in [(-12.0, 60.0), (-9, 64), (-5, 76), (-2, 80)] { s.append(v, metric: Fluency.readWPM, component: Fluency.reading, at: day(d)) }
        var line = Fluency.coverLine(s, now: now, label: label)!
        XCTAssertTrue(line.hasPrefix("You read about 26% faster than last week"), line)
        XCTAssertTrue(line.contains("78 words a minute"), line)
        s = MetricStore()
        for (d, v) in [(-12.0, 80.0), (-9, 80), (-5, 70), (-2, 66)] { s.append(v, metric: Fluency.readWPM, component: Fluency.reading, at: day(d)) }
        line = Fluency.coverLine(s, now: now, label: label)!
        XCTAssertTrue(line.contains("a little slower"), line)
        XCTAssertTrue(line.contains("No hurry"), line)
        s = MetricStore()
        for (d, v) in [(-12.0, 80.0), (-9, 82), (-5, 81), (-2, 83)] { s.append(v, metric: Fluency.readWPM, component: Fluency.reading, at: day(d)) }
        XCTAssertTrue(Fluency.coverLine(s, now: now, label: label)!.contains("about as fast"))
        // Without reading, the word whose writing moved most.
        s = MetricStore()
        let word = ComponentID(.lexeme, "твёрдый")
        for (d, v) in [(-12.0, 1.0), (-9, 1.0), (-5, 1.2), (-2, 1.2)] { s.append(v, metric: Fluency.strokesPerSecond, component: word, at: day(d)) }
        for (d, v) in [(-12.0, 1.0), (-9, 1.0), (-5, 1.02), (-2, 1.02)] { s.append(v, metric: Fluency.strokesPerSecond, component: Fluency.writing, at: day(d)) }
        line = Fluency.coverLine(s, now: now, label: label)!
        XCTAssertTrue(line.hasPrefix("You wrote твёрдый about 20% faster than last week"), line)
        // Listening last: both speeds need three each.
        s = MetricStore()
        for i in 0..<3 { s.append(1, metric: Fluency.listening, component: Fluency.listeningSkill, at: day(-Double(i)), note: "0.75") }
        XCTAssertNil(Fluency.coverLine(s, now: now, label: label))
        for i in 0..<4 { s.append(i == 0 ? 1 : 0, metric: Fluency.listening, component: Fluency.listeningSkill, at: day(-Double(i)), note: "1") }
        line = Fluency.coverLine(s, now: now, label: label)!
        XCTAssertTrue(line.contains("25% of the sentences right at full speed and 100% at ¾ speed"), line)
    }
}
