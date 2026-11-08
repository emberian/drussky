import XCTest

/// Screens a person sees but the unit tests can't: landscape, the tutor sheet with a live answer.
@MainActor
final class ScreensUITests: XCTestCase {
    func testLandscapeAndTutor() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "2", "-qa-partial", "4"]
        if let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] { app.launchEnvironment["DRUSSKY_TUTOR_KEY"] = key }
        app.launch()
        XCUIDevice.shared.orientation = .landscapeLeft
        sleep(2)
        attach(app, "landscape")
        XCUIDevice.shared.orientation = .portrait
        sleep(1)

        let ask = app.buttons["Ask the tutor about exercise 5"].firstMatch
        XCTAssertTrue(ask.waitForExistence(timeout: 10))
        ask.tap()
        let suggestion = app.buttons["Why is it this way?"].firstMatch
        XCTAssertTrue(suggestion.waitForExistence(timeout: 5))
        suggestion.tap()
        sleep(12)
        attach(app, "tutor")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
