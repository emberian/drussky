import XCTest

/// Margin notes and a reading page made from the learner's own text, through the real UI.
@MainActor
final class NewPagesUITests: XCTestCase {
    func testMarginNotesAndOwnTextPage() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-finger", "-qa-complete", "6"]
        if let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] { app.launchEnvironment["DRUSSKY_TUTOR_KEY"] = key }
        app.launch()

        let margin = app.buttons["Write in the margins"].firstMatch
        XCTAssertTrue(margin.waitForExistence(timeout: 10))
        margin.tap()
        let w = app.windows.firstMatch
        func line(_ a: CGVector, _ b: CGVector) { w.coordinate(withNormalizedOffset: a).press(forDuration: 0.05, thenDragTo: w.coordinate(withNormalizedOffset: b)) }
        line(CGVector(dx: 0.80, dy: 0.30), CGVector(dx: 0.92, dy: 0.26))
        line(CGVector(dx: 0.80, dy: 0.33), CGVector(dx: 0.90, dy: 0.34))
        line(CGVector(dx: 0.82, dy: 0.36), CGVector(dx: 0.86, dy: 0.42))
        app.buttons["Stop writing in the margins"].firstMatch.tap()
        attach(app, "margin")

        app.buttons["Add a page"].firstMatch.tap()
        app.buttons["Read my own text…"].firstMatch.tap()
        let editor = app.textViews.firstMatch
        XCTAssertTrue(editor.waitForExistence(timeout: 5))
        editor.tap()
        editor.typeText("Робот проснулся ночью. На станции было тихо. Он посмотрел в окно и увидел новую звезду.")
        app.buttons["Make it a page"].firstMatch.tap()
        let read = app.buttons["I've read it"].firstMatch
        XCTAssertTrue(read.waitForExistence(timeout: 150), "the tutor's reading page opened")
        sleep(2)
        attach(app, "own-text-page")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
