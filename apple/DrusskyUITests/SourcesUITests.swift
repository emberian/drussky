import XCTest

/// «Исто́чники · Sources» (Views/CreditsView.swift): the top of the page, a contributor opened,
/// the Tatoeba sentences opened, and the fonts and open items further down. Screenshots go to
/// $DRUSSKY_SHOTS when set (and are attached either way).
@MainActor
final class SourcesUITests: XCTestCase {
    func testSourcesPage() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-sources"]
        app.launch()
        XCTAssertTrue(app.staticTexts["Recorded by people"].firstMatch.waitForExistence(timeout: 20))
        sleep(1)
        shoot(app, "sources-top")
        // A contributor's clips.
        let first = app.buttons.containing(.staticText, identifier: "fjay69").firstMatch
        if first.waitForExistence(timeout: 5) {
            first.tap()
            sleep(1)
            shoot(app, "sources-contributor")
            first.tap()
        }
        // The sentences, with their authors and translators.
        var tries = 0
        let sentences = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'All ' AND label ENDSWITH ' sentences'")).firstMatch
        while !sentences.isHittable && tries < 20 { app.swipeUp(); tries += 1 }
        XCTAssertTrue(sentences.isHittable, "the sentences disclosure is reachable")
        sentences.tap()
        sleep(1)
        shoot(app, "sources-sentences")
        sentences.tap()
        // Fonts, reference material, open items.
        tries = 0
        let fonts = app.staticTexts["Fonts"].firstMatch
        while !fonts.isHittable && tries < 20 { app.swipeUp(); tries += 1 }
        sleep(1)
        shoot(app, "sources-fonts")
        tries = 0
        let open = app.staticTexts["Not yet resolved"].firstMatch
        while !open.isHittable && tries < 20 { app.swipeUp(); tries += 1 }
        sleep(1)
        shoot(app, "sources-end")
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
