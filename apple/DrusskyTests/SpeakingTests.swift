import AVFoundation
import LearningCore
import XCTest
@testable import Drussky

/// The hands-free take (endpointing, trimming), the syllable-aligned comparison with the model
/// (DTW), the word-level analyzer on bundled recordings, and the evidence a take leaves.
@MainActor
final class SpeakingTests: XCTestCase {
    let sr = 48_000.0

    // MARK: Signals

    private var rng = SystemRandomNumberGenerator()

    private func silence(_ s: Double) -> [Float] { [Float](repeating: 0, count: Int(s * sr)) }

    /// White noise at `db` dBFS (RMS).
    private func noise(_ s: Double, db: Float) -> [Float] {
        let a = pow(10, db / 20) * sqrt(3)
        return (0..<Int(s * sr)).map { _ in Float.random(in: -a...a, using: &rng) }
    }

    /// A voice-like tone: 150 Hz with harmonics, at about `db` dBFS.
    private func tone(_ s: Double, db: Float, hz: Double = 150) -> [Float] {
        let g = pow(10, db / 20) / 0.58
        return (0..<Int(s * sr)).map { n in
            let p = 2 * Double.pi * hz * Double(n) / sr
            return g * Float(0.6 * sin(p) + 0.35 * sin(2 * p) + 0.2 * sin(3 * p) + 0.1 * sin(4 * p))
        }
    }

    private func mix(_ a: [Float], _ b: [Float]) -> [Float] {
        (0..<max(a.count, b.count)).map { (a.indices.contains($0) ? a[$0] : 0) + (b.indices.contains($0) ? b[$0] : 0) }
    }

    private func run(_ x: [Float], config: VoiceActivity.Config = .init(), chunk: Int = 1024) -> (events: [VoiceActivity.Event], vad: VoiceActivity) {
        var vad = VoiceActivity(sampleRate: sr, config: config)
        var events: [VoiceActivity.Event] = []
        var i = 0
        while i < x.count {
            let n = min(chunk, x.count - i)
            events += x[i..<(i + n)].withUnsafeBufferPointer { vad.feed($0) }
            i += n
        }
        return (events, vad)
    }

    private func ending(_ events: [VoiceActivity.Event]) -> VoiceActivity.Ending? {
        for e in events { if case .ended(let end) = e { return end } }
        return nil
    }

    private func onset(_ events: [VoiceActivity.Event]) -> Int? {
        for e in events { if case .onset(let i) = e { return i } }
        return nil
    }

    // MARK: Endpointing

    func testSilenceNeverStartsATake() {
        var c = VoiceActivity.Config()
        c.waitLimit = 2
        let (events, _) = run(silence(3), config: c)
        XCTAssertNil(onset(events))
        XCTAssertEqual(ending(events)?.reason, .noSpeech)
        XCTAssertEqual(events.first, .listening(floor: -90))
    }

    func testNoiseNeverStartsATake() {
        var c = VoiceActivity.Config()
        c.waitLimit = 3.5
        // A steady fan the room measurement takes in.
        XCTAssertNil(onset(run(noise(4, db: -35), config: c).events))
        // A loud burst of noise (a door, a breath into the microphone) over a quiet room.
        let burst = mix(noise(4, db: -65), silence(1) + noise(0.6, db: -18))
        let (events, _) = run(burst, config: c)
        XCTAssertNil(onset(events), "unvoiced burst ignored")
        XCTAssertEqual(ending(events)?.reason, .noSpeech)
    }

    func testAToneIsTakenFromOnsetToTheSilenceAfterIt() throws {
        let x = mix(noise(3, db: -60), silence(0.8) + tone(0.6, db: -22))
        let (events, vad) = run(x)
        let start = try XCTUnwrap(onset(events))
        XCTAssertEqual(Double(start) / sr, 0.8, accuracy: 0.03)
        let end = try XCTUnwrap(ending(events))
        XCTAssertEqual(end.reason, .silence)
        let speech = try XCTUnwrap(end.speech), keep = try XCTUnwrap(end.keep)
        XCTAssertEqual(Double(speech.lowerBound) / sr, 0.8, accuracy: 0.03)
        XCTAssertEqual(Double(speech.upperBound) / sr, 1.4, accuracy: 0.04)
        XCTAssertEqual(Double(speech.lowerBound - keep.lowerBound) / sr, 0.12, accuracy: 0.01)
        XCTAssertEqual(Double(keep.upperBound - speech.upperBound) / sr, 0.18, accuracy: 0.01)
        // Ended ~0.7 s after the voice stopped, not at the end of the signal.
        XCTAssertLessThan(Double(vad.count) / sr, 2.3)
        XCTAssertGreaterThan(Double(vad.count) / sr, 2.0)
    }

    func testAShortPauseStaysInOneTake() throws {
        let x = mix(noise(4, db: -60), silence(0.5) + tone(0.4, db: -22) + silence(0.3) + tone(0.4, db: -24, hz: 190))
        let end = try XCTUnwrap(ending(run(x).events))
        let speech = try XCTUnwrap(end.speech)
        XCTAssertEqual(Double(speech.lowerBound) / sr, 0.5, accuracy: 0.03)
        XCTAssertEqual(Double(speech.upperBound) / sr, 1.6, accuracy: 0.04)
    }

    func testALeadingHissIsKeptWithTheVoice() throws {
        // с before a vowel: 90 ms of loud unvoiced noise, then the voice.
        let x = mix(noise(3, db: -60), silence(0.7) + noise(0.09, db: -28) + tone(0.5, db: -22))
        let start = try XCTUnwrap(onset(run(x).events))
        XCTAssertEqual(Double(start) / sr, 0.7, accuracy: 0.025)
    }

    func testTheTakeEndsAtTheLongestSpeech() throws {
        let end = try XCTUnwrap(ending(run(silence(0.4) + tone(9, db: -20)).events))
        XCTAssertEqual(end.reason, .maxLength)
        let speech = try XCTUnwrap(end.speech)
        XCTAssertEqual(Double(speech.count) / sr, 6, accuracy: 0.05)
    }

    func testChunkingDoesNotChangeTheResult() {
        let x = mix(noise(3, db: -60), silence(0.6) + tone(0.5, db: -24))
        let a = ending(run(x, chunk: 1024).events), b = ending(run(x, chunk: 4800).events), c = ending(run(x, chunk: 137).events)
        XCTAssertNotNil(a)
        XCTAssertEqual(a, b)
        XCTAssertEqual(a, c)
    }

    func testStoppingByHand() {
        var vad = VoiceActivity(sampleRate: sr)
        _ = vad.feed(silence(1))
        XCTAssertEqual(vad.finishNow().reason, .noSpeech)
        var talking = VoiceActivity(sampleRate: sr)
        _ = talking.feed(silence(0.5) + tone(0.5, db: -22))
        let end = talking.finishNow()
        XCTAssertEqual(end.reason, .tapped)
        XCTAssertEqual(Double(end.speech?.lowerBound ?? 0) / sr, 0.5, accuracy: 0.03)
    }

    // MARK: Trimming

    func testTheWrittenTakeIsTrimmedWithPadding() throws {
        let x = mix(noise(3, db: -60), silence(0.9) + tone(0.5, db: -22))
        let end = try XCTUnwrap(ending(run(x).events))
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("\(UUID().uuidString).m4a")
        defer { try? FileManager.default.removeItem(at: url) }
        let written = try TakeCapture.write(x, sampleRate: sr, ending: end, to: url)
        XCTAssertEqual(written.speechStart, 0.12, accuracy: 0.01)
        XCTAssertEqual(written.speechEnd - written.speechStart, 0.5, accuracy: 0.05)
        XCTAssertEqual(written.duration, 0.5 + 0.12 + 0.18, accuracy: 0.05)
        let file = try AVAudioFile(forReading: url)
        XCTAssertEqual(Double(file.length) / file.processingFormat.sampleRate, written.duration, accuracy: 0.03)
        // Silence trimmed: the file starts quiet, and the voice is where it says.
        let y = try XCTUnwrap(Pitch.samples(url))
        let a = Acoustics(samples: y)
        let bounds = try XCTUnwrap(a.speechBounds())
        XCTAssertEqual(Acoustics.time(bounds.lowerBound), written.speechStart, accuracy: 0.03)
    }

    // MARK: Alignment

    private func clip(_ name: String) throws -> [Float] {
        let url = try XCTUnwrap(Bundle.main.url(forResource: name, withExtension: "m4a", subdirectory: "Audio"), "\(name).m4a")
        return try XCTUnwrap(Pitch.samples(url))
    }

    private func clipURL(_ name: String) throws -> URL {
        try XCTUnwrap(Bundle.main.url(forResource: name, withExtension: "m4a", subdirectory: "Audio"), "\(name).m4a")
    }

    /// Overlap-add time warp: output time u reads the input at `source(u)` seconds.
    private func warp(_ x: [Float], to seconds: Double, source: (Double) -> Double) -> [Float] {
        let fs = Acoustics.sampleRate, win = 320, hop = 80
        let n = Int(seconds * fs)
        var out = [Float](repeating: 0, count: n + win), norm = [Float](repeating: 0, count: n + win)
        let w = (0..<win).map { Float(0.5 - 0.5 * cos(2 * Double.pi * Double($0) / Double(win))) }
        var k = 0
        while k * hop < n {
            let centre = Int(source(Double(k * hop) / fs) * fs)
            for j in 0..<win {
                let src = centre - win / 2 + j
                guard src >= 0 && src < x.count else { continue }
                out[k * hop + j] += x[src] * w[j]
                norm[k * hop + j] += w[j]
            }
            k += 1
        }
        return (0..<n).map { norm[$0] > 1e-3 ? out[$0] / norm[$0] : 0 }
    }

    func testAlignmentFollowsAnUnevenStretch() throws {
        for name in ["moloko-c2d396", "spasibo-00f66d"] {
            let x = try clip(name)
            let T = Double(x.count) / Acoustics.sampleRate
            // First half 1.4× slower, second half 0.8×: a learner who drags the start and rushes the end.
            let forward = { (t: Double) in t < T / 2 ? 1.4 * t : 0.7 * T + 0.8 * (t - T / 2) }
            let y = warp(x, to: forward(T)) { u in u < 0.7 * T ? u / 1.4 : T / 2 + (u - 0.7 * T) / 0.8 }
            let a = Acoustics(samples: x), b = Acoustics(samples: y)
            let al = try XCTUnwrap(DTW.align(a.mfcc, b.mfcc))
            let speech = try XCTUnwrap(a.speechBounds())
            let errors = speech.map { abs(al.map($0) - forward(Acoustics.time($0)) * Acoustics.rate) }
            let mean = errors.reduce(0, +) / Double(errors.count)
            XCTAssertLessThan(mean, 3, "\(name): mean error \(mean) frames")
            XCTAssertLessThan(errors.sorted()[errors.count * 9 / 10], 6, "\(name): 90th percentile")
        }
    }

    func testSyllablesCarryOntoAStretchedCopy() throws {
        let x = try clip("moloko-c2d396")
        let T = Double(x.count) / Acoustics.sampleRate
        let y = warp(x, to: T * 1.3) { $0 / 1.3 }
        let c = try XCTUnwrap(ProsodyComparison.compute(target: "молоко́", take: y, model: x))
        let model = try XCTUnwrap(c.model)
        XCTAssertEqual(c.take.count, 3)
        for i in 0..<3 {
            XCTAssertEqual(c.take[i].start, model[i].start * 1.3, accuracy: 0.05, "syllable \(i) start")
            XCTAssertEqual(c.take[i].end, model[i].end * 1.3, accuracy: 0.05, "syllable \(i) end")
        }
    }

    // MARK: Syllables

    func testSyllablesOfTheTarget() {
        func parts(_ s: String) -> [String] { Syllables.split(s).map { $0.text.trimmingCharacters(in: .whitespaces) } }
        XCTAssertEqual(parts("молоко́"), ["мо", "ло", "ко́"])
        XCTAssertEqual(Syllables.split("молоко́").map(\.isStressed), [false, false, true])
        XCTAssertEqual(parts("сестра́"), ["се", "стра́"])
        XCTAssertEqual(parts("Спаси́бо, ма́ма!"), ["Спа", "си́", "бо", "ма́", "ма"])
        XCTAssertEqual(parts("Я ду́маю."), ["Я", "ду́", "ма", "ю"])
        XCTAssertEqual(parts("большо́й"), ["боль", "шо́й"])
        XCTAssertEqual(Syllables.split("кто").map(\.isStressed), [true])
        let ma = Syllables.split("Спаси́бо, ма́ма!")[3]
        XCTAssertEqual(Array("Спаси́бо, ма́ма!")[ma.letters].map(String.init).joined(), "ма́")
    }

    // MARK: The word-level analyzer

    private func analyse(target: String, take: URL, model: URL?, heard: String?) async throws -> PronunciationReport {
        try await WordLevelAnalyzer(transcribe: { _, _ in heard }).analyze(take: take, target: target, model: model)
    }

    func testAnotherSpeakerSayingItWell() async throws {
        let model = try clipURL("moloko-c2d396")
        var stressedRight = 0
        for take in ["moloko-aleksey-pogrebnoj-al-f0a170", "moloko-doctorandusmanhattan-ee42ca", "moloko-1apollinariya1-21adf8"] {
            let r = try await analyse(target: "молоко́", take: try clipURL(take), model: model, heard: "молоко")
            XCTAssertTrue(r.isWordLevel)
            XCTAssertEqual(r.syllables.map(\.text), ["мо", "ло", "ко́"])
            XCTAssertEqual(r.accuracy, 1)
            XCTAssertTrue(r.sounds.allSatisfy { $0.verdict == .good })
            XCTAssertFalse(r.tips.contains { $0.contrast != nil && $0.contrast != "stress" }, take)
            let starts = r.syllables.compactMap(\.start), ends = r.syllables.compactMap(\.end)
            XCTAssertEqual(starts.count, 3)
            XCTAssertEqual(starts, starts.sorted())
            for i in 0..<3 { XCTAssertLessThan(starts[i], ends[i]) }
            if r.stressedSyllable == 2 { stressedRight += 1 }
        }
        XCTAssertGreaterThanOrEqual(stressedRight, 2, "the final stress of молоко́ found in most speakers")
    }

    func testALetterHeardAsAnother() async throws {
        let r = try await analyse(target: "был", take: try clipURL("bil-cdd458"), model: try clipURL("byl-8826ed"), heard: "бил")
        let sub = try XCTUnwrap(r.sounds.first { $0.verdict == .substituted })
        XCTAssertEqual(sub.letterRange, 1..<2)
        XCTAssertEqual(sub.expected?.ipa, "ɨ")
        XCTAssertEqual(sub.heard?.ipa, "i")
        let tip = try XCTUnwrap(r.tips.first)
        XCTAssertEqual(tip.contrast, "y-i")
        XCTAssertTrue(tip.message.hasPrefix("«ы» came out as «и»."), tip.message)
        XCTAssertEqual(r.accuracy, 2.0 / 3.0, accuracy: 1e-9)
    }

    func testStressOnTheWrongSyllable() async throws {
        let voices = Voice()
        let right = try XCTUnwrap(voices.pairVoices("замо́к", exactStress: true).first)
        let wrong = try XCTUnwrap(voices.pairVoices("за́мок", exactStress: true).first)
        let miss = try await analyse(target: "замо́к", take: wrong, model: right, heard: "замок")
        XCTAssertEqual(miss.stressedSyllable, 0)
        let tip = try XCTUnwrap(miss.tips.first { $0.contrast == "stress" }, "\(miss.tips)")
        XCTAssertTrue(tip.message.contains("«мо́к»"), tip.message)
        let hit = try await analyse(target: "за́мок", take: wrong, model: wrong, heard: "замок")
        XCTAssertEqual(hit.stressedSyllable, 0)
        XCTAssertFalse(hit.tips.contains { $0.contrast == "stress" })
    }

    func testNothingRecognisedStillComparesProsody() async throws {
        let r = try await analyse(target: "спаси́бо", take: try clipURL("spasibo-00f66d"), model: try clipURL("spasibo-00f66d"), heard: nil)
        XCTAssertTrue(r.sounds.isEmpty)
        XCTAssertEqual(r.syllables.count, 3)
        XCTAssertEqual(r.stressedSyllable, 1)
        XCTAssertTrue(r.tips.isEmpty)
    }

    func testContrastNames() {
        XCTAssertEqual(SoundContrasts.id(for: "ы/и"), "y-i")
        XCTAssertEqual(SoundContrasts.id(for: "Hard/soft"), "hard-soft")
        XCTAssertEqual(SoundContrasts.id(for: "stress"), "stress")
        XCTAssertNil(SoundContrasts.id(for: "vowel length"))
        XCTAssertEqual(SoundContrasts.id(expected: "у", heard: "ю"), "hard-soft")
    }

    // MARK: Evidence

    func testASubstitutionIsSpeechEvidenceOnItsContrast() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        let book = Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        var ex = Exercise(kind: .readAloud)
        ex.text = "был"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        let page = try XCTUnwrap(book.page)
        let report = WordLevelAnalyzer.report(target: "был", transcript: "бил", comparison: nil)
        book.knowledge.heard(page: page, index: 0, take: "take-1.m4a", report: report, attempt: 1)
        book.knowledge.heard(page: page, index: 0, take: "take-1.m4a", report: report, attempt: 1)
        let events = book.knowledge.model.events.filter { $0.id == "speech:take-1.m4a" }
        XCTAssertEqual(events.count, 1, "a take counts once")
        let e = try XCTUnwrap(events.first)
        XCTAssertEqual(e.type, .attempt)
        XCTAssertEqual(e.modality, .say)
        XCTAssertTrue(e.components.contains(ComponentID(.sound, "y-i")))
        XCTAssertEqual(e.confusions, [Confusion(target: ComponentID(.letter, "Ы"), produced: ComponentID(.letter, "И"), channel: .speech)])
        // A clean take adds nothing beyond the exercise's own outcome.
        let clean = WordLevelAnalyzer.report(target: "был", transcript: "был", comparison: nil)
        book.knowledge.heard(page: page, index: 0, take: "take-2.m4a", report: clean, attempt: 2)
        XCTAssertFalse(book.knowledge.model.events.contains { $0.id == "speech:take-2.m4a" })
    }

    // MARK: Finishing the exercise

    /// A phone-level report says how it sounded, never which words were said: with the phone
    /// model installed, word recognition still runs and still finishes the exercise.
    func testAPhoneLevelReportStillChecksTheWords() async throws {
        struct Phones: PronunciationAnalyzer {
            func analyze(take: URL, target: String, model: URL?) async throws -> PronunciationReport {
                PronunciationReport(target: target, transcript: nil, expected: [Phone(ipa: "b")], heard: [Phone(ipa: "b")],
                                    sounds: [PhoneJudgement(expected: Phone(ipa: "b"), heard: Phone(ipa: "b"), verdict: .good, letterRange: 0..<1)],
                                    syllables: [], stressedSyllable: nil, accuracy: 1, tips: [])
            }
        }
        let take = try clipURL("byl-8826ed")
        let (report, heard) = await SpeakingTurn.analyse(take: take, target: "был", model: nil, primary: Phones(), recognise: { _, _ in "был" })
        XCTAssertEqual(heard, "был")
        XCTAssertEqual(report?.transcript, "был", "the words heard travel with the report")
        XCTAssertFalse(report?.isWordLevel ?? true)
        // Without a phone analyzer, the words heard are recognised once and handed to the fallback.
        let counting = Counter()
        let (fallback, words) = await SpeakingTurn.analyse(take: take, target: "был", model: nil, primary: nil,
                                                           recognise: { _, _ in counting.bump(); return "бил" })
        XCTAssertEqual(counting.count, 1)
        XCTAssertEqual(words, "бил")
        XCTAssertEqual(fallback?.transcript, "бил")
        XCTAssertTrue(fallback?.isWordLevel ?? false)

        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        let book = Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        var ex = Exercise(kind: .readAloud); ex.text = "был"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        let ref = ExerciseRef(pageID: book.page!.id, index: 0)
        XCTAssertEqual(book.commitTake(ref, round: 0, file: "t1.m4a", heard: heard, target: "был", recognitionAllowed: true, timedOut: false), .finished)
        XCTAssertEqual(book.result(ref)?.status, .correct)
    }

    private final class Counter: @unchecked Sendable {
        private let lock = NSLock()
        private var n = 0
        var count: Int { lock.withLock { n } }
        func bump() { lock.withLock { n += 1 } }
    }

    /// The recognizer hearing nothing is its failure, not a failed check against the learner.
    func testNoTranscriptIsNotAFailedCheck() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        let book = Book(course: course, store: BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)))
        var ex = Exercise(kind: .readAloud); ex.text = "был"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        let ref = ExerciseRef(pageID: book.page!.id, index: 0)
        XCTAssertEqual(book.commitTake(ref, round: 0, file: "t1.m4a", heard: nil, target: "был", recognitionAllowed: true, timedOut: false), .recorded)
        XCTAssertEqual(book.result(ref)?.attempts, 1)
        XCTAssertNil(book.result(ref)?.failedChecks)
        XCTAssertEqual(book.commitTake(ref, round: 0, file: "t2.m4a", heard: "бил", target: "был", recognitionAllowed: true, timedOut: false), .recorded)
        XCTAssertEqual(book.result(ref)?.failedChecks, 1, "words heard and wrong are a failed check")
        XCTAssertEqual(book.commitTake(ref, round: 0, file: "t3.m4a", heard: "был", target: "был", recognitionAllowed: true, timedOut: false), .finished)
        XCTAssertEqual(book.result(ref)?.status, .done, "right after a failed check")
    }

    /// A take whose report was saved beside it but whose sound evidence never reached the log (the
    /// app died between the two writes) is reconstructed on the next opening, once.
    func testASavedReportWithoutItsEventIsBackfilled() throws {
        let url = try XCTUnwrap(Bundle.main.url(forResource: "course", withExtension: "json"))
        let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
        let store = BookStore(root: FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString))
        let book = Book(course: course, store: store)
        var ex = Exercise(kind: .readAloud); ex.text = "был"
        book.insertWrittenPage(title: "t", intro: nil, unitTitle: "t", exercises: [ex], newWords: [])
        let ref = ExerciseRef(pageID: book.page!.id, index: 0)
        let take = book.newTakeURL(ref)
        try Data("voice".utf8).write(to: take)
        let sub = PhoneJudgement(expected: Phone(ipa: "ɨ"), heard: Phone(ipa: "i"), verdict: .substituted, letterRange: 1..<2)
        let report = PronunciationReport(target: "был", transcript: "бил", expected: [], heard: [Phone(ipa: "b"), Phone(ipa: "i"), Phone(ipa: "l")],
                                         sounds: [sub], syllables: [], stressedSyllable: nil, accuracy: 0.6, tips: [])
        try JSONEncoder().encode(report).write(to: SpeakingTurn.reportURL(for: take))
        book.commitTake(ref, round: 0, file: take.lastPathComponent, heard: "бил", target: "был", recognitionAllowed: true, timedOut: false)
        // …and the app dies before `knowledge.heard`.
        let id = "speech:\(take.lastPathComponent)"
        XCTAssertFalse(book.knowledge.model.events.contains { $0.id == id })
        let again = Book(course: course, store: store)
        let events = again.knowledge.model.events.filter { $0.id == id }
        XCTAssertEqual(events.count, 1)
        XCTAssertEqual(events.first?.source, "backfill")
        XCTAssertEqual(events.first?.confusions, [Confusion(target: ComponentID(.letter, "Ы"), produced: ComponentID(.letter, "И"), channel: .speech)])
        XCTAssertEqual(Book(course: course, store: store).knowledge.model.events.filter { $0.id == id }.count, 1, "never doubled")
    }

    func testSettingsFromBeforeSpeakingOpen() throws {
        let old = #"{"fingerDrawing":true,"cursiveModels":false,"showStress":false}"#
        let s = try JSONDecoder().decode(Settings.self, from: Data(old.utf8))
        XCTAssertTrue(s.fingerDrawing)
        XCTAssertFalse(s.showStress)
        XCTAssertEqual(s.modelRepeats, 1)
    }
}
