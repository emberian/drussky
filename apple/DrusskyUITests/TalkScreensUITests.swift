import XCTest

/// Screens of Talk: the scene choice, the middle of a conversation, the marked transcript.
/// No key or microphone: `-qa-talk` poses the page from a canned conversation.
@MainActor
final class TalkScreensUITests: XCTestCase {
    func testTalkScreens() throws {
        let app = XCUIApplication()
        // A permission dialog left by an earlier test process would cover the page.
        let allow = XCUIApplication(bundleIdentifier: "com.apple.springboard").buttons["Allow"].firstMatch
        if allow.waitForExistence(timeout: 2) { allow.tap() }
        for state in ["scene", "mid", "marked"] {
            app.launchArguments = ["-qa-reset", "-qa-complete", "12", "-qa-talk", state, "-qa-top"]
            app.launch()
            XCTAssertTrue(app.staticTexts["Разгово́р"].firstMatch.waitForExistence(timeout: 20), state)
            if state == "mid" {
                let cont = app.buttons["Continue talking"].firstMatch
                XCTAssertTrue(cont.waitForExistence(timeout: 5))
            }
            sleep(2)
            attach(app, "talk-\(state)")
            app.terminate()
        }
    }

    /// Ember's first Talk with no key, and offline: one honest line, and the page can be left.
    func testTalkUnavailableIsOneLineAndLeavable() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "3", "-qa-talk", "fresh", "-qa-offline"]
        app.launch()
        XCTAssertTrue(app.staticTexts["Разгово́р"].firstMatch.waitForExistence(timeout: 20))
        XCTAssertTrue(app.staticTexts["Talk needs the internet — the tutor is on the other end of it."].firstMatch.waitForExistence(timeout: 5))
        attach(app, "talk-offline")
        let notToday = app.buttons["Not today"].firstMatch
        XCTAssertTrue(notToday.exists)
        notToday.tap()
        XCTAssertTrue(app.staticTexts["Not today — nothing said here."].firstMatch.waitForExistence(timeout: 5), "skipped, the page finishes")
        XCTAssertTrue(app.buttons["Turn the page"].firstMatch.waitForExistence(timeout: 5))
        app.terminate()

        app.launchArguments = ["-qa-reset", "-qa-complete", "3", "-qa-talk", "fresh", "-qa-nokey"]
        app.launch()
        XCTAssertTrue(app.staticTexts["Разгово́р"].firstMatch.waitForExistence(timeout: 20))
        XCTAssertTrue(app.staticTexts["Talk needs your tutor: add your Anthropic API key in Settings."].firstMatch.waitForExistence(timeout: 5))
        attach(app, "talk-nokey")
        XCTAssertTrue(app.buttons["Skip to the next page"].firstMatch.exists, "the page is leavable without touching Talk")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
