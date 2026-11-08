import XCTest
@testable import Drussky

/// credits.json (apple/scripts/build_credits.py) is what «Исто́чники · Sources» shows. It must
/// decode, and it must cover every bundled clip exactly once, so a regenerated audio manifest
/// cannot drift away from the credits without this test going red.
@MainActor
final class CreditsTests: XCTestCase {
    private func audioDir() throws -> URL { try XCTUnwrap(Bundle.main.url(forResource: "Audio", withExtension: nil)) }

    func testCreditsDecodeWithEveryGroup() throws {
        let credits = try Credits.load()
        XCTAssertEqual(credits.title, "Исто́чники · Sources")
        for kind in ["human_audio", "narration", "synthetic_audio", "tatoeba_text", "fonts", "reference_material", "models_and_algorithms"] {
            XCTAssertFalse(credits.group(kind).isEmpty, "credits.json has \(kind) entries")
        }
        for group in credits.groups {
            for e in group.entries {
                XCTAssertFalse(e.attribution.isEmpty, "\(e.id) has an attribution sentence")
                XCTAssertFalse(e.credit_status.isEmpty, "\(e.id) says whether its credit is complete")
            }
        }
        for font in credits.group("fonts") {
            XCTAssertTrue(font.license_text?.contains("SIL OPEN FONT LICENSE Version 1.1") ?? false, "\(font.id) carries the full OFL")
            XCTAssertNotNil(UIFont(name: String(font.id.dropFirst("font-".count)), size: 12), "\(font.id) names a registered font")
        }
        // The Milena take was macOS speech, which Apple's licence does not allow us to share.
        XCTAssertTrue(credits.group("synthetic_audio").allSatisfy { $0.id != "apple-system-voice" }, "no macOS system-voice clip is bundled")
        // Human credits that claim to be complete name a licence and a source page.
        for e in credits.group("human_audio") + credits.group("narration") where e.isReady {
            XCTAssertNotNil(e.source_url, "\(e.id) (\(e.title)) has a source page")
            XCTAssertNotEqual(e.license, "unknown", "\(e.id) (\(e.title)) has a recognised licence")
            XCTAssertFalse(e.author?.hasPrefix("not stated") ?? true, "\(e.id) (\(e.title)) names its contributor")
        }
    }

    /// Mirrors build_credits.py's own assertion against the built bundle: every .m4a in Audio/ is
    /// owned by exactly one audio credit, and every clip an index refers to is bundled.
    func testEveryBundledClipHasExactlyOneCredit() throws {
        let credits = try Credits.load()
        let audio = try audioDir()
        let bundled = Set(try FileManager.default.contentsOfDirectory(atPath: audio.path).filter { $0.hasSuffix(".m4a") })
        XCTAssertGreaterThan(bundled.count, 2000, "the audio folder is bundled")
        var referenced = Set<String>()
        for name in ["index.json", "index-pairs.json", "index-narration.json"] {
            let index = try JSONDecoder().decode([String: String].self, from: Data(contentsOf: audio.appendingPathComponent(name)))
            referenced.formUnion(index.values)
        }
        for name in ["variants.json", "variants-pairs.json"] {
            let variants = try JSONDecoder().decode([String: [String]].self, from: Data(contentsOf: audio.appendingPathComponent(name)))
            referenced.formUnion(variants.values.joined())
        }
        XCTAssertEqual(referenced.subtracting(bundled).sorted(), [], "clips an index names but the bundle lacks")
        let owners = credits.clipOwners
        XCTAssertEqual(bundled.subtracting(owners.keys).sorted(), [], "bundled clips without a credit: regenerate credits.json")
        XCTAssertEqual(Set(owners.keys).subtracting(bundled).sorted(), [], "credited clips that are not bundled: regenerate credits.json")
        XCTAssertEqual(owners.filter { $0.value.count != 1 }.keys.sorted(), [], "clips credited more than once")
    }

    /// The Tatoeba credits follow the library's own record of who wrote each sentence.
    func testTatoebaCreditsFollowTheLibrary() throws {
        let credits = try Credits.load()
        let url = try XCTUnwrap(Bundle.main.url(forResource: "library", withExtension: "json"))
        let library = try JSONDecoder().decode(Library.self, from: Data(contentsOf: url))
        let sentences = try XCTUnwrap(library.sentences)
        let entries = Dictionary(uniqueKeysWithValues: credits.group("tatoeba_text").map { ($0.source_url ?? "", $0) })
        XCTAssertEqual(entries.count, sentences.count, "one Tatoeba credit per library sentence")
        var credited = 0
        for s in sentences {
            let e = try XCTUnwrap(entries[s.source ?? ""], "credit for \(s.ru)")
            XCTAssertEqual(e.title, s.ru)
            XCTAssertEqual(e.audio_asset, s.audio)
            if let author = s.ru_author { XCTAssertEqual(e.ru_contributors, [author], s.ru); credited += 1 }
            if let author = s.en_author { XCTAssertEqual(e.en_contributors, [author], s.ru) }
            if let en = s.en_source { XCTAssertEqual(e.translation_source_url, en, s.ru) }
            XCTAssertEqual(e.isReady, s.ru_author != nil && s.en_author != nil && s.en_source != nil, "\(s.ru): ready exactly when both contributors and the translation are recorded")
        }
        XCTAssertGreaterThan(credited, 0, "Tatoeba contributors have been fetched into library.json")
    }
}
