import XCTest

/// The read-along page as a person sees it: the reader playing with the current word lit, the
/// listen-first mode, the sentence exercises under it, and the library's headphone marks.
/// Screenshots go to $TEST_RUNNER_DRUSSKY_SHOTS when set (and are attached either way).
@MainActor
final class NarrationUITests: XCTestCase {
    func testReadAlongPageAndLibrary() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-narration", "tolstoy-muravei-i-golubka"]
        app.launch()

        let play = app.buttons["narration-play"].firstMatch
        XCTAssertTrue(play.waitForExistence(timeout: 15), "the read-along page opens with its play button")
        play.tap()
        sleep(12)
        XCTAssertTrue(app.buttons["narration-current-word"].firstMatch.exists, "a word is lit while the reader speaks")
        shoot(app, "read-along-playing")
        play.tap()  // pause

        app.buttons["Listen first, text hidden"].firstMatch.tap()
        play.tap()
        sleep(5)
        shoot(app, "read-along-listen-first")
        play.tap()

        app.swipeUp()
        sleep(1)
        shoot(app, "read-along-chunks")

        app.buttons["Add a page"].firstMatch.tap()  // the ＋ menu holds the library
        let library = app.buttons["Library"].firstMatch
        XCTAssertTrue(library.waitForExistence(timeout: 5))
        library.tap()
        sleep(2)
        shoot(app, "library-headphones")
        // The page behind the sheet has the same title; the library row is the cell.
        let row = app.cells.containing(.staticText, identifier: "Муравей и голубка").firstMatch
        if row.waitForExistence(timeout: 5) {
            row.tap()
            XCTAssertTrue(app.buttons["read-along"].firstMatch.waitForExistence(timeout: 5), "a narrated text offers Listen and read along")
            shoot(app, "library-text-read-along")
        }
    }

    private func shoot(_ app: XCUIApplication, _ name: String) {
        let shot = app.screenshot()
        let attachment = XCTAttachment(screenshot: shot)
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
        if let dir = ProcessInfo.processInfo.environment["DRUSSKY_SHOTS"] {
            try? FileManager.default.createDirectory(atPath: dir, withIntermediateDirectories: true)
            try? shot.pngRepresentation.write(to: URL(fileURLWithPath: dir).appendingPathComponent("\(name).png"))
        }
    }
}
