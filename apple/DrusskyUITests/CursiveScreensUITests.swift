import XCTest

/// Screens of the cursive work: the proof sheet of all 66 models and 30 words, a meet page with
/// the cursive setting on (stroke-model ghost, "Watch it written"), and a read-handwriting page.
@MainActor
final class CursiveScreensUITests: XCTestCase {
    func testProofSheet() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-cursive-proof"]
        app.launch()
        XCTAssertTrue(app.descendants(matching: .any)["cursive-proof"].firstMatch.waitForExistence(timeout: 20))
        sleep(2)
        attach(app, "cursive-proof")
        app.swipeUp(); app.swipeUp(); app.swipeUp()
        sleep(1)
        attach(app, "cursive-proof-words")
    }

    func testCursiveMeetPage() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-cursive", "-qa-top"]
        app.launch()
        let watch = app.buttons["watch-strokes"].firstMatch
        XCTAssertTrue(watch.waitForExistence(timeout: 20))
        XCTAssertEqual(watch.label, "Watch it written")
        sleep(1)
        attach(app, "cursive-meet")
        watch.tap()
        sleep(2)
        attach(app, "cursive-meet-demo")
    }

    func testReadHandwritingPage() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "14", "-qa-handwriting", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.staticTexts["Read handwriting"].firstMatch.waitForExistence(timeout: 20))
        XCTAssertTrue(app.descendants(matching: .any)["handwriting-1"].firstMatch.waitForExistence(timeout: 10))
        sleep(2)
        attach(app, "read-handwriting")
    }

    func testCursiveUnitPage() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-until", "u31-p2", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.staticTexts["The zigzag letters: и ш л м"].firstMatch.waitForExistence(timeout: 30))
        sleep(2)
        attach(app, "cursive-unit-page")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
