import XCTest

/// Real touch input on the PencilKit paper: strokes → ink → exercise state → persistence.
final class WritingUITests: XCTestCase {
    func testTouchWritingCompletesAndPersists() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset"]
        app.launch()

        let strip = app.descendants(matching: .any)["strip-0"].firstMatch
        XCTAssertTrue(strip.waitForExistence(timeout: 15), "first writing strip")
        func stroke(_ a: CGVector, _ b: CGVector) {
            strip.coordinate(withNormalizedOffset: a).press(forDuration: 0.05, thenDragTo: strip.coordinate(withNormalizedOffset: b))
        }
        // А: two slants and a bar, beside the traced model.
        stroke(CGVector(dx: 0.30, dy: 0.70), CGVector(dx: 0.34, dy: 0.17))
        stroke(CGVector(dx: 0.34, dy: 0.17), CGVector(dx: 0.38, dy: 0.70))
        stroke(CGVector(dx: 0.315, dy: 0.50), CGVector(dx: 0.365, dy: 0.50))
        attach(app, "ink")

        let again = app.buttons["Write it again"].firstMatch
        if !again.waitForExistence(timeout: 4) {
            let done = app.buttons["Done"].firstMatch
            XCTAssertTrue(done.waitForExistence(timeout: 3), "Done appears once there is ink")
            done.tap()
        }
        XCTAssertTrue(again.waitForExistence(timeout: 5), "exercise 1 completed by touch")
        attach(app, "done")

        // Undo affects only this strip and survives as a real control.
        app.terminate()
        app.launchArguments = []
        app.launch()
        XCTAssertTrue(app.buttons["Write it again"].firstMatch.waitForExistence(timeout: 15), "completion persisted across relaunch")
        attach(app, "relaunched")
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
