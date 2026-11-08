import XCTest
#if canImport(FoundationModels)
import FoundationModels
#endif

/// The on-device gloss: with no Claude key, a word tapped on the first page of the learner's
/// book (its lemma table unfilled) gets a meaning from the device, tagged "on device".
/// Skips where the simulator has no model.
@MainActor
final class OnDeviceUITests: XCTestCase {
    func testOfflineGlossIsTaggedOnDevice() throws {
        #if canImport(FoundationModels)
        guard #available(iOS 26.0, *), case .available = SystemLanguageModel.default.availability else {
            throw XCTSkip("no on-device model in this simulator")
        }
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "14", "-qa-goal", "tolstoy-lev-i-mysh", "-qa-goal-first"]
        app.launch()
        let stamp = app.descendants(matching: .any)["Stamp: first page of your book"].firstMatch
        XCTAssertTrue(stamp.waitForExistence(timeout: 30), "the first page of the book is open")
        // A word of Tolstoy's that no page gloss or lexicon entry covers.
        let word = app.buttons.matching(NSPredicate(format: "label CONTAINS[c] %@", "пробежа")).firstMatch
        XCTAssertTrue(word.waitForExistence(timeout: 10), "a passage word to tap")
        word.tap()
        let tag = app.descendants(matching: .any)["provenance-on-device"].firstMatch
        XCTAssertTrue(tag.waitForExistence(timeout: 30), "the gloss arrives from the device and says so")
        sleep(1)
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = "ondevice-gloss"
        shot.lifetime = .keepAlways
        add(shot)
        #else
        throw XCTSkip("FoundationModels not in this SDK")
        #endif
    }
}
