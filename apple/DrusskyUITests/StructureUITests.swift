import XCTest

/// The structure layer as a person sees it: a library text read as written with «Структу́ра» on
/// (the school underlines, the clause brackets, the rings), a word held down (roles and root x-ray),
/// a pronoun tapped (its head pulses), and the referent question under the text.
/// Screenshots go to $TEST_RUNNER_DRUSSKY_SHOTS when set (and are attached either way).
@MainActor
final class StructureUITests: XCTestCase {
    func testStructureLayerAndQuestion() throws {
        let app = XCUIApplication()
        let id = ProcessInfo.processInfo.environment["DRUSSKY_STRUCTURE_TEXT"] ?? "tolstoy-lev-i-mysh"
        app.launchArguments = ["-qa-reset", "-qa-structure", id]
        app.launch()

        let toggle = app.buttons["structure-toggle"].firstMatch
        XCTAssertTrue(toggle.waitForExistence(timeout: 20), "the reading page offers the structure toggle")
        XCTAssertEqual(toggle.label, "Hide the structure", "-qa-structure opens with the layer on")
        sleep(2)
        shoot(app, "structure-on")

        // A word held down: its roles and root x-ray under the text.
        let words = app.buttons.matching(NSPredicate(format: "label MATCHES %@", "^[А-Яа-яЁё][а-яё]+$"))
        let second = words.element(boundBy: 1)
        XCTAssertTrue(second.waitForExistence(timeout: 5))
        second.press(forDuration: 0.8)
        let panel = app.descendants(matching: .any)["structure-panel"].firstMatch
        XCTAssertTrue(panel.waitForExistence(timeout: 4), "a long press opens the word's panel")
        if !panel.isHittable { app.swipeUp() }  // a phone: the panel sits under the text, below the fold
        sleep(1)
        shoot(app, "structure-xray")
        if !toggle.isHittable { app.swipeDown() }

        // Off: the text stays exactly where it was, bare.
        toggle.tap()
        sleep(1)
        shoot(app, "structure-off")
        toggle.tap()

        // The referent question under the passage.
        let probe = app.buttons["structure-probe"].firstMatch
        XCTAssertTrue(probe.waitForExistence(timeout: 5), "the page asks who is meant")
        var tries = 0
        while !probe.isHittable && tries < 8 { app.swipeUp(); tries += 1 }
        sleep(1)
        shoot(app, "structure-question")
        let head = app.buttons["structure-head"].firstMatch
        if head.exists && head.isHittable {
            head.tap()
            sleep(1)
            shoot(app, "structure-answered")
        }
    }

    /// The same layer over a human reading: the read-along page of a narrated text with «Структу́ра» on.
    func testReadAlongWithStructure() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-structure", "tolstoy-muravei-i-golubka", "-qa-narration", "tolstoy-muravei-i-golubka"]
        app.launch()
        XCTAssertTrue(app.buttons["narration-play"].firstMatch.waitForExistence(timeout: 20), "the read-along page opens")
        let toggle = app.buttons["structure-toggle"].firstMatch
        XCTAssertTrue(toggle.waitForExistence(timeout: 5), "a narrated text offers the toggle too")
        XCTAssertEqual(toggle.label, "Hide the structure")
        sleep(2)
        shoot(app, "structure-read-along")
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
