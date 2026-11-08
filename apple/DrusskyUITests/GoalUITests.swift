import XCTest

/// "Your book": the distance panel on the inside cover, a page written toward it, and the
/// first-page moment. Screenshots are attached; the tutor page is written live when
/// DRUSSKY_TUTOR_KEY is in the environment, otherwise the offline page stands in.
@MainActor
final class GoalUITests: XCTestCase {
    private let goal = "tolstoy-lev-i-mysh"

    func testDistancePanel() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "14", "-qa-goal", goal, "-qa-contents"]
        if let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] { app.launchEnvironment["DRUSSKY_TUTOR_KEY"] = key }
        app.launch()
        let panel = app.descendants(matching: .any)["goal-panel"].firstMatch
        XCTAssertTrue(panel.waitForExistence(timeout: 20), "the distance panel is on the inside cover")
        // Meanings arrive from the tutor when there is a key; give them a moment.
        if ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] != nil { sleep(25) } else { sleep(2) }
        attach(app, "goal-distance")
    }

    func testTowardMyBookPage() throws {
        let app = XCUIApplication()
        let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"]
        app.launchArguments = ["-qa-reset", "-qa-complete", "14", "-qa-goal", goal] + (key == nil ? ["-qa-goal-pull"] : [])
        if let key { app.launchEnvironment["DRUSSKY_TUTOR_KEY"] = key }
        app.launch()
        if key != nil {
            let add = app.buttons["Add a page"].firstMatch
            XCTAssertTrue(add.waitForExistence(timeout: 20))
            add.tap()
            let toward = app.buttons["Toward my book"].firstMatch
            XCTAssertTrue(toward.waitForExistence(timeout: 5))
            toward.tap()
            let write = app.buttons["Write a page toward my book"].firstMatch
            XCTAssertTrue(write.waitForExistence(timeout: 10))
            write.tap()
            // The sheet closes when the page is in the book.
            XCTAssertTrue(write.waitForNonExistence(timeout: 150), "the tutor's page arrived")
            sleep(2)
        } else {
            sleep(3)
        }
        XCTAssertTrue(app.staticTexts["TOWARD YOUR BOOK"].firstMatch.waitForExistence(timeout: 10), "the page toward the book is open")
        attach(app, "goal-page")
    }

    func testFirstPageMoment() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "14", "-qa-goal", goal, "-qa-goal-first"]
        if let key = ProcessInfo.processInfo.environment["DRUSSKY_TUTOR_KEY"] { app.launchEnvironment["DRUSSKY_TUTOR_KEY"] = key }
        app.launch()
        let stamp = app.descendants(matching: .any)["Stamp: first page of your book"].firstMatch
        XCTAssertTrue(stamp.waitForExistence(timeout: 20))
        sleep(2)
        attach(app, "goal-first-page")
        let read = app.buttons["I've read it"].firstMatch
        if read.waitForExistence(timeout: 5) {
            read.tap()
            sleep(2)
            attach(app, "goal-first-page-read")
        }
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
