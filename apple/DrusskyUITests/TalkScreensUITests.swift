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

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
