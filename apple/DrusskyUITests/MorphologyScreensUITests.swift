import XCTest

/// Screens of the roots and word-families work: the generated family page, the cognates page,
/// the Word families section of the inside cover, and one family opened.
@MainActor
final class MorphologyScreensUITests: XCTestCase {
    func testFamilyAndCognatePages() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "70", "-qa-families", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.staticTexts["Word families"].firstMatch.waitForExistence(timeout: 20))
        sleep(2)
        attach(app, "families-page")
        app.terminate()

        app.launchArguments = ["-qa-reset", "-qa-complete", "70", "-qa-cognates", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.staticTexts["Cognates"].firstMatch.waitForExistence(timeout: 20))
        sleep(2)
        attach(app, "cognates-page")
    }

    func testWordFamiliesOnTheInsideCover() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "70", "-qa-contents"]
        app.launch()
        let title = app.staticTexts["Word families"].firstMatch
        XCTAssertTrue(title.waitForExistence(timeout: 20))
        var tries = 0
        while !title.isHittable && tries < 12 { app.swipeUp(); tries += 1 }
        sleep(1)
        attach(app, "families-map")
        app.terminate()

        app.launchArguments = ["-qa-reset", "-qa-complete", "70", "-qa-contents", "-qa-family", "чит"]
        app.launch()
        XCTAssertTrue(app.navigationBars.firstMatch.waitForExistence(timeout: 20))
        sleep(3)
        attach(app, "family-chit")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
