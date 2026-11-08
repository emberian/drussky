import XCTest

/// The read-aloud turn's screens, posed by `-qa-speak` without a microphone: the model has
/// played and the page waits for the tap; the report after a take.
@MainActor
final class SpeakingScreensUITests: XCTestCase {
    func testWaitsForTheTapThenShowsTheReport() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "3", "-qa-speak", "ready", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.staticTexts["молоко́"].firstMatch.waitForExistence(timeout: 20))
        let button = app.buttons["Your turn: tap to say it"].firstMatch
        XCTAssertTrue(button.waitForExistence(timeout: 6), "after the model, nothing listens until the tap")
        XCTAssertTrue(app.staticTexts["Tap to say it."].firstMatch.exists)
        XCTAssertFalse(app.staticTexts["Say it — I'll stop when you do."].firstMatch.exists, "no capture before the tap")
        attach(app, "speak-ready")
        app.terminate()

        app.launchArguments = ["-qa-reset", "-qa-complete", "3", "-qa-speak", "report", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.staticTexts["молоко́"].firstMatch.waitForExistence(timeout: 20))
        XCTAssertTrue(app.buttons["Listen, then say it"].firstMatch.waitForExistence(timeout: 6))
        XCTAssertTrue(app.staticTexts["Heard «молоко»."].firstMatch.waitForExistence(timeout: 15), "the canned take reaches the report")
        attach(app, "speak-report")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
