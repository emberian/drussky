import XCTest
@testable import Drussky

/// The pronunciation engine: G2P, features, CTC, alignment, mistake naming, the word-level
/// fallback, and (when the phone model is bundled) end-to-end clips from the pairs.
final class PronunciationTests: XCTestCase {

    // MARK: G2P

    /// Standard Moscow transcriptions (ZIPA/espeak symbols). Reduction by distance from the
    /// stress, palatalisation, hard ж ш ц, soft ч щ, devoicing, assimilation, exceptions.
    static let g2pTable: [(String, String)] = [
        ("мыл", "m ɨ l"), ("мил", "mʲ i l"), ("мать", "m a tʲ"), ("брат", "b r a t"),
        ("дома́", "d ɐ m a"), ("до́ма", "d o m ə"), ("за́мок", "z a m ə k"), ("замо́к", "z ɐ m o k"),
        ("хорошо́", "x ə r ɐ ʂ o"), ("молоко́", "m ə l ɐ k o"), ("окно́", "ɐ k n o"),
        ("язы́к", "j ɪ z ɨ k"), ("часы́", "tɕ ɪ s ɨ"), ("пятно́", "pʲ ɪ t n o"), ("неде́ля", "nʲ ɪ dʲ e lʲ ə"),
        ("жена́", "ʐ ɨ n a"), ("жить", "ʐ ɨ tʲ"), ("шесть", "ʂ ɛ s tʲ"), ("цирк", "ts ɨ r k"),
        ("щи", "ɕː i"), ("счёт", "ɕː o t"), ("чай", "tɕ a j"),
        ("луг", "l u k"), ("код", "k o t"), ("хлеб", "x lʲ e p"), ("муж", "m u ʂ"),
        ("в па́рк", "f p a r k"), ("сде́лать", "z dʲ e l ə tʲ"), ("вокза́л", "v ɐ ɡ z a l"), ("ло́дка", "l o t k ə"),
        ("что", "ʂ t o"), ("его́", "j ɪ v o"), ("сего́дня", "sʲ ɪ v o d nʲ ə"), ("кра́сного", "k r a s n ə v ə"),
        ("учи́ться", "u tɕ i ts ə"), ("здра́вствуйте", "z d r a s t v u j tʲ ɪ"), ("по́здно", "p o z n ə"),
        ("со́лнце", "s o n ts ɨ"), ("мя́гкий", "mʲ a x kʲ i j"), ("два́дцать", "d v a ts ə tʲ"),
        ("объе́кт", "ɐ b j e k t"), ("солове́й", "s ə l ɐ vʲ e j"), ("сейча́с", "sʲ ɪ tɕ a s"),
    ]

    func testG2PTable() {
        for (word, want) in Self.g2pTable {
            XCTAssertEqual(RussianG2P.transcribe(word).sounds.map(\.ipa).joined(separator: " "), want, word)
        }
    }

    func testG2PStressSyllablesAndLetters() {
        let t = RussianG2P.transcribe("за́мок")
        XCTAssertEqual(t.syllables.map(\.text), ["за́", "мок"])
        XCTAssertEqual(t.syllables.map(\.stressed), [true, false])
        // о of мок is reduced, and remembers it was о (for the "full unstressed о" mistake).
        let o = t.sounds[3]
        XCTAssertEqual(o.ipa, "ə"); XCTAssertEqual(o.underlying, "o"); XCTAssertEqual(o.letters, 3..<4)
        // мать: the soft т covers т and ь.
        XCTAssertEqual(RussianG2P.transcribe("мать").sounds.last?.letters, 2..<4)
        // Phrases: words and a proclitic joined for voicing.
        let p = RussianG2P.transcribe("Я был в па́рке.")
        XCTAssertEqual(p.sounds.map(\.word).max(), 3)
        XCTAssertEqual(p.sounds.first { $0.word == 2 }?.ipa, "f")
    }

    // MARK: Features

    /// Reference values from lhotse's Fbank (ZIPA's training features) on the same signal.
    func testFbankMatchesLhotse() {
        let x: [Float] = (0..<8000).map { i in
            let t = Double(i) / 16000
            return Float((0.3 * sin(2 * .pi * 220 * t) + 0.1 * sin(2 * .pi * 1375 * t + 0.5) + 0.05 * sin(2 * .pi * 5100 * t))
                         * (0.5 + 0.5 * sin(2 * .pi * 3 * t)))
        }
        let f = Fbank.shared.compute(x)
        XCTAssertEqual(f.count, 50 * 80)
        let cols = [0, 10, 20, 30, 40, 50, 60, 70, 79]
        let ref: [Int: [Float]] = [
            0: [-5.7307, -1.9168, -6.4040, -3.3102, -2.9848, -5.0238, -10.3966, -2.1764, -6.9908],
            10: [-8.0682, -3.4939, -14.9859, -8.3425, -11.1240, -15.9424, -15.9424, -7.4118, -15.9424],
            49: [-5.4042, -1.7185, -3.9239, -2.5299, -3.5303, -5.1345, -4.7228, -2.4535, -8.2115],
        ]
        for (frame, values) in ref {
            for (c, v) in zip(cols, values) { XCTAssertEqual(f[frame * 80 + c], v, accuracy: 2e-3, "frame \(frame) bin \(c)") }
        }
    }

    // MARK: CTC and alignment

    /// Three tokens over 6 frames: blank, a, a, blank, b, blank.
    private func toyPosteriors() -> Posteriors {
        let rows: [[Float]] = [[0.9, 0.05, 0.05], [0.1, 0.8, 0.1], [0.1, 0.8, 0.1], [0.8, 0.1, 0.1], [0.1, 0.1, 0.8], [0.9, 0.05, 0.05]]
        return Posteriors(logp: rows.flatMap { $0.map(log) }, frames: 6, vocab: 3)
    }

    func testCTCPrefersTheSpokenLabels() {
        let p = toyPosteriors()
        let right = CTC.logLikelihood(p, [1, 2]), swapped = CTC.logLikelihood(p, [2, 1]), short = CTC.logLikelihood(p, [1])
        XCTAssertGreaterThan(right, swapped + 3)
        XCTAssertGreaterThan(right, short + 1)
        let spans = CTC.align(p, [1, 2])
        XCTAssertEqual(spans[0], 1...2); XCTAssertEqual(spans[1], 4...4)
        XCTAssertEqual(CTC.greedy(p).map(\.token), [1, 2])
    }

    func testFeatureWeightedAlignment() {
        // ы→и is a substitution in place, not a deletion plus an insertion.
        XCTAssertEqual(Align.run(["m", "ɨ", "l"], ["m", "i", "l"]), [.match(0, 0), .substitute(1, 1), .match(2, 2)])
        // An added vowel is an insertion.
        XCTAssertEqual(Align.run(["f", "p", "a"], ["f", "ɨ", "p", "a"]), [.match(0, 0), .insert(1), .match(1, 2), .match(2, 3)])
        XCTAssertLessThan(PhoneSet.distance("tʲ", "t"), PhoneSet.distance("t", "k"))
        XCTAssertLessThan(PhoneSet.distance("ɨ", "i"), PhoneSet.distance("ɨ", "a"))
    }

    // MARK: Mistakes and contrasts

    func testMistakesNameThePracticeContrasts() throws {
        func kinds(_ word: String, _ i: Int) -> Set<Mistake.Kind> { Set(PhoneSet.options(for: RussianG2P.transcribe(word).sounds[i]).errors.map(\.kind)) }
        XCTAssertTrue(kinds("мыл", 1).contains(.yi))
        XCTAssertTrue(kinds("мать", 2).contains(.hardened))
        XCTAssertTrue(kinds("мат", 2).contains(.softened))
        XCTAssertTrue(kinds("щи", 0).contains(.sh))
        XCTAssertTrue(kinds("шар", 0).contains(.shch))
        XCTAssertTrue(kinds("жар", 0).contains(.devoicing))
        XCTAssertTrue(kinds("код", 2).contains(.voicing))      // saying the д
        XCTAssertTrue(kinds("молоко́", 1).contains(.reduction)) // a full unstressed о
        // A correct realisation is never a mistake: ɐ may be written a.
        XCTAssertFalse(PhoneSet.options(for: RussianG2P.transcribe("дома́").sounds[1]).errors.contains { $0.ipa == "a" })
        // Every contrast a mistake names exists in pairs.json, so practice can follow.
        let url = try XCTUnwrap(Bundle.main.url(forResource: "pairs", withExtension: "json"))
        let pairs = try XCTUnwrap(JSONSerialization.jsonObject(with: Data(contentsOf: url)) as? [String: Any])
        let ids = Set((pairs["contrasts"] as? [[String: Any]] ?? []).compactMap { $0["id"] as? String })
        let kinds: [Mistake.Kind] = [.yi, .softened, .hardened, .shch, .sh, .devoicing, .voicing, .reduction, .fullVowel, .rhotic, .other]
        for k in kinds { if let c = Mistake(ipa: "", kind: k).contrast { XCTAssertTrue(ids.contains(c), c) } }
        XCTAssertTrue(ids.contains("stress"))
    }

    func testWordFallbackComparesSounds() {
        let target = "мыл"
        let r = WordFallback.report(target: target, expected: RussianG2P.transcribe(target), transcript: "мил", speech: (0, 16000))
        XCTAssertEqual(r.transcript, "мил")
        XCTAssertTrue(r.sounds.contains { $0.verdict == .substituted && $0.expected?.ipa == "ɨ" && $0.letterRange == 1..<2 })
        XCTAssertTrue(r.tips.isEmpty, "a recognized spelling cannot establish a sound error")
        XCTAssertTrue(r.sounds.allSatisfy { !$0.supportsCorrection })
        let ok = WordFallback.report(target: "до́ма", expected: RussianG2P.transcribe("до́ма"), transcript: "Дома", speech: (0, 16000))
        XCTAssertEqual(ok.accuracy, 1)
        XCTAssertTrue(ok.tips.isEmpty)
    }

    // MARK: End to end (needs `scripts/models.py fetch` and the restored pair audio)

    private func clip(_ word: String) throws -> URL {
        guard PhoneRecognizer.bundled != nil else { throw XCTSkip("phone model not bundled (python3 apple/scripts/models.py fetch)") }
        let idx = try XCTUnwrap(Bundle.main.url(forResource: "index-pairs", withExtension: "json", subdirectory: "Audio"))
        let course = try XCTUnwrap(Bundle.main.url(forResource: "index", withExtension: "json", subdirectory: "Audio"))
        var map = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: course))
        map.merge(try JSONDecoder().decode([String: String].self, from: Data(contentsOf: idx))) { _, b in b }
        let name = try XCTUnwrap(map[word], word)
        guard let url = Bundle.main.url(forResource: (name as NSString).deletingPathExtension, withExtension: "m4a", subdirectory: "Audio") else {
            throw XCTSkip("\(name) not restored (python3 apple/scripts/audio.py restore)")
        }
        return url
    }

    private func report(_ target: String, audio: String) async throws -> PronunciationReport {
        try await PhoneticAnalyzer(recognizer: PhoneRecognizer.bundled).analyze(take: try clip(audio), target: target, model: nil)
    }

    func testMilHeardAgainstMylFlagsYI() async throws {
        let r = try await report("мыл", audio: "мил")
        let y = try XCTUnwrap(r.sounds.first { $0.expected?.ipa == "ɨ" })
        XCTAssertEqual(y.verdict, .substituted)
        XCTAssertEqual(y.heard?.ipa, "i")
        XCTAssertEqual(y.letterRange, 1..<2)
        XCTAssertEqual(r.tips.first?.contrast, "y-i")
    }

    func testNativeTakesScoreWell() async throws {
        for w in ["мыл", "быть", "мать"] {
            let r = try await report(w, audio: w)
            XCTAssertGreaterThanOrEqual(r.accuracy, 0.99, w)
            XCTAssertFalse(r.sounds.contains { $0.verdict == .substituted }, w)
            XCTAssertTrue(r.tips.isEmpty, "\(w): \(r.tips.map(\.message))")
            XCTAssertNotNil(r.speechStart); XCTAssertEqual(r.syllables.count, 1)
        }
    }

    func testMatHeardAgainstMatFlagsHardT() async throws {
        let r = try await report("мать", audio: "мат")
        XCTAssertTrue(r.sounds.contains { $0.expected?.ipa == "tʲ" && $0.verdict == .substituted && $0.heard?.ipa == "t" })
        XCTAssertEqual(r.tips.first?.contrast, "hard-soft")
    }

    func testZamokStressShowsInTheVowels() async throws {
        // за́мок said as замо́к: the unstressed о came out full.
        let r = try await report("за́мок", audio: "замо́к")
        XCTAssertTrue(r.sounds.contains { $0.expected?.ipa == "ə" && $0.verdict == .substituted && $0.heard?.ipa == "o" })
        XCTAssertTrue(r.tips.contains { $0.contrast == "reduction" || $0.contrast == "stress" })
        let native = try await report("замо́к", audio: "замо́к")
        XCTAssertEqual(native.accuracy, 1)
        XCTAssertEqual(native.syllables.map(\.isStressed), [false, true])
    }
}
