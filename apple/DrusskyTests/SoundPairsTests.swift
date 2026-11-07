import XCTest
@testable import Drussky

/// Pitch tracking (YIN) on synthesized tones and bundled speech, and the sound-pairs pages.
@MainActor
final class SoundPairsTests: XCTestCase {

    // MARK: Pitch

    /// A voice-like tone: fundamental plus decaying harmonics, frequency given per sample.
    private func tone(seconds: Double, hz: (Double) -> Double, sr: Double = Pitch.sampleRate) -> [Float] {
        var phase = 0.0
        return (0..<Int(seconds * sr)).map { n in
            phase += 2 * .pi * hz(Double(n) / sr) / sr
            return Float(0.5 * sin(phase) + 0.25 * sin(2 * phase) + 0.12 * sin(3 * phase) + 0.06 * sin(4 * phase))
        }
    }

    private func median(_ v: [Float]) -> Float { let s = v.sorted(); return s[s.count / 2] }

    func testPitchOfSteadyTones() {
        for hz in [90.0, 140.0, 220.0, 330.0] {
            let track = Pitch.track(tone(seconds: 0.8, hz: { _ in hz }))
            let voiced = track.compactMap { $0 }
            XCTAssertGreaterThan(Double(voiced.count), Double(track.count) * 0.9, "\(hz) Hz mostly voiced")
            XCTAssertEqual(Double(median(voiced)), hz, accuracy: hz * 0.01, "\(hz) Hz")
            XCTAssertLessThan(abs(Double(voiced.max()!) - hz), hz * 0.03, "no octave slips at \(hz) Hz")
        }
    }

    func testPitchFollowsAGlide() {
        // 150 → 300 Hz, exponential: one octave in 1.2 s, i.e. a straight line in semitones.
        let seconds = 1.2
        let glide = { (t: Double) in 150 * pow(2, t / seconds) }
        let track = Pitch.track(tone(seconds: seconds, hz: glide))
        var errors: [Double] = []
        for (i, f) in track.enumerated() {
            guard let f else { continue }
            let truth = glide(Double(i) / Pitch.framesPerSecond)
            errors.append(abs(12 * log2(Double(f) / truth)))
        }
        XCTAssertGreaterThan(errors.count, track.count * 8 / 10)
        let within = errors.filter { $0 < 0.5 }.count
        XCTAssertGreaterThan(Double(within), Double(errors.count) * 0.9, "90% of frames within half a semitone")
        let contour = Contour.from(samples: tone(seconds: seconds, hz: glide))!
        XCTAssertGreaterThan(contour.finalMovement, 3, "a rising glide ends high")
        XCTAssertEqual(Double(contour.rise), 12, accuracy: 1.5, "one octave")
    }

    func testSilenceAndNoiseAreUnvoiced() {
        XCTAssertTrue(Pitch.track([Float](repeating: 0, count: 16_000)).allSatisfy { $0 == nil })
        var rng = SystemRandomNumberGenerator()
        let noise = (0..<16_000).map { _ in Float.random(in: -0.3...0.3, using: &rng) }
        let voiced = Pitch.track(noise).compactMap { $0 }.count
        XCTAssertLessThan(voiced, 4, "white noise has no pitch")
    }

    /// One speaker (Lingua Libre): «Вы лю́бите му́зыку?» jumps up on лю́ (ИК-3), «Я го́лоден.»
    /// stays level and falls (ИК-1). Both bundled as read-aloud models.
    func testQuestionJumpsUpStatementFalls() async throws {
        func contour(_ name: String) async throws -> Contour {
            let url = try XCTUnwrap(Bundle.main.url(forResource: name, withExtension: "m4a", subdirectory: "Audio"))
            let c = await Contour.compute(url)
            return try XCTUnwrap(c, name)
        }
        let question = try await contour("intonation-vy-lyubite-muzyku")
        let statement = try await contour("intonation-ya-goloden")
        for c in [question, statement] {
            XCTAssertGreaterThan(c.semitones.compactMap { $0 }.count, 12, "voiced bins")
            XCTAssertTrue((80...160).contains(c.medianHz), "a low male voice, not an octave slip: \(c.medianHz) Hz")
        }
        XCTAssertGreaterThan(question.rise, 6, "question rise \(question.rise)")
        XCTAssertLessThan(statement.rise, 3, "statement rise \(statement.rise)")
        XCTAssertLessThan(statement.finalMovement, -2, "statement falls \(statement.finalMovement)")
        // Their models are what read-aloud plays for these sentences.
        let voice = Voice()
        XCTAssertEqual(voice.recordingURL(for: "Вы лю́бите му́зыку?")?.lastPathComponent, "intonation-vy-lyubite-muzyku.m4a")
    }

    // MARK: Sound pairs

    private func book() throws -> Book {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        return Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
    }

    func testPairsFileIsWellFormedAndHasAudio() {
        let data = SoundPairs.load()
        let voice = Voice()
        XCTAssertGreaterThanOrEqual(data.contrasts.count, 6)
        let ids = Set(data.contrasts.map(\.id))
        var missing: [String] = []
        var multi = 0
        for p in data.pairs {
            XCTAssertTrue(ids.contains(p.contrast), p.contrast)
            XCTAssertEqual(p.words.count, 2)
            XCTAssertNotEqual(p.words[0].ru, p.words[1].ru)
            for w in p.words {
                XCTAssertFalse(w.en.isEmpty, w.ru)
                let vowels = RussianText.unstressed(w.ru).filter { "аеёиоуыэюя".contains($0) }.count
                if vowels > 1 && !w.ru.contains("ё") { XCTAssertTrue(w.ru.unicodeScalars.contains("\u{0301}"), "stress mark on \(w.ru)") }
                let n = voice.pairVoices(w.ru, exactStress: p.exactStress).count
                if n == 0 { missing.append(w.ru) }
                if n > 1 { multi += 1 }
            }
        }
        XCTAssertEqual(missing, [], "pair words without a recording")
        XCTAssertGreaterThan(multi, data.pairs.count, "most words have several voices")
        // Stress pairs never share a clip.
        for p in data.pairs where p.exactStress {
            let a = Set(voice.pairVoices(p.words[0].ru, exactStress: true)), b = Set(voice.pairVoices(p.words[1].ru, exactStress: true))
            XCTAssertTrue(a.isDisjoint(with: b), p.words[0].ru)
        }
    }

    func testPairsPageUsesOnlyKnownLettersAndFavoursWeakContrasts() throws {
        let book = try book()
        let data = SoundPairs.load()
        let voices: (String, Bool) -> Int = { _, _ in 2 }
        XCTAssertTrue(book.soundPairsExercises(from: data, voices: voices).filter(\.isSoundPairTrial).isEmpty,
                      "nothing before any letters are met")
        for _ in 0..<40 {
            QA.complete(book, page: book.currentPage, upTo: Int.max)
            book.turnForward()
        }
        let met = Set(book.course.letters.filter { book.letterKnown($0.upper) }.map(\.upper))
        let page = book.soundPairsExercises(from: data, voices: voices)
        let trials = page.filter(\.isSoundPairTrial)
        XCTAssertEqual(trials.count, 10)
        for e in page {
            for w in (e.options ?? []) + (e.examples ?? []).map(\.ru) {
                XCTAssertTrue(RussianText.letters(in: w).isSubset(of: met), w)
            }
        }
        for t in trials {
            XCTAssertEqual(t.options?[t.answerIndex ?? -1], t.say)
            XCTAssertEqual(t.glosses?.count, 2)
        }

        // Miss one contrast repeatedly, ace the others: the next page leans on the missed one.
        let contrasts = Set(trials.compactMap(\.hint))
        XCTAssertGreaterThan(contrasts.count, 1)
        let weak = contrasts.sorted()[0]
        book.insertWrittenPage(title: "Sound pairs", intro: nil, unitTitle: "Listening", exercises: page, newWords: [])
        for (i, e) in page.enumerated() where e.isSoundPairTrial {
            book.record(page: book.currentPage, exercise: i) { r in
                r.status = e.hint == weak ? .missed : .correct
            }
        }
        XCTAssertLessThan(book.contrastStats[weak]!.accuracy, 0.5)
        let next = book.soundPairsExercises(from: data, voices: voices).filter(\.isSoundPairTrial)
        let share = Double(next.filter { $0.hint == weak }.count) / Double(next.count)
        XCTAssertGreaterThan(share, 0.4, "weak contrast gets the most trials")
    }

    func testSoundStatsPersistAndOldNotebooksStillLoad() throws {
        let root = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        let store = BookStore(root: root)
        let book = Book(course: course, store: store)
        var trial = Exercise(kind: .soundPair)
        trial.hint = "y-i"; trial.options = ["был", "бил"]; trial.say = "был"; trial.answerIndex = 0
        book.insertWrittenPage(title: "Sound pairs", intro: nil, unitTitle: "Listening", exercises: [trial, trial], newWords: [])
        book.record(page: book.currentPage, exercise: 0) { $0.status = .correct }
        book.record(page: book.currentPage, exercise: 1) { $0.status = .missed }
        book.reopen(page: book.currentPage, exercise: 1)
        book.record(page: book.currentPage, exercise: 1) { $0.status = .correct }
        XCTAssertEqual(book.contrastStats["y-i"], ContrastStat(right: 1, wrong: 1, last: book.contrastStats["y-i"]?.last),
                       "only first completions count")
        book.saveNow()
        let again = Book(course: course, store: store)
        XCTAssertEqual(again.contrastStats["y-i"]?.trials, 2)
        XCTAssertEqual(again.pages[again.currentPage].exercises[0].kind, .soundPair)

        // A notebook written before sound pairs existed has no soundStats key.
        var json = try JSONSerialization.jsonObject(with: Data(contentsOf: store.stateURL)) as! [String: Any]
        json.removeValue(forKey: "soundStats")
        json["pages"] = (json["pages"] as! [[String: Any]]).filter { ($0["exercises"] as! [[String: Any]]).allSatisfy { $0["kind"] as? String != "sound_pair" } }
        json["currentPage"] = 0
        try JSONSerialization.data(withJSONObject: json).write(to: store.stateURL)
        let old = Book(course: course, store: store)
        XCTAssertTrue(old.contrastStats.isEmpty)
        XCTAssertFalse(old.pages.isEmpty)
        let files = try FileManager.default.contentsOfDirectory(atPath: root.path)
        XCTAssertFalse(files.contains { $0.hasPrefix("book-unreadable") }, "old notebook decoded")
    }
}
