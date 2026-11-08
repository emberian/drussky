import XCTest

/// The listening strand on screen (Views/ListeningView.swift): a set as a page before anything is
/// played, a card answered (the transcript, span and why appear in place), the voice-message card
/// with two questions, an exchange with its speaker badges, a scene to follow, and the weekly
/// sample's messages. Nothing on a phone runs past the screen's edge.
@MainActor
final class ListeningScreensUITests: XCTestCase {
    private func launch(_ args: [String]) -> XCUIApplication {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset"] + args
        app.launch()
        XCTAssertTrue(app.buttons["Add a page"].firstMatch.waitForExistence(timeout: 20))
        return app
    }

    /// Set 2 («Дай, пожа́луйста»): which-thing cards with Russian choices, one asking-or-telling, two to follow.
    func testSetTwoPlaysAndAnswers() throws {
        let app = launch(["-qa-listening", "2", "-qa-top"])
        sleep(2)
        attach(app, "listening-2-fresh")
        let play = app.buttons["listening-play-0"].firstMatch
        XCTAssertTrue(play.waitForExistence(timeout: 5), "the first card's Play")
        play.tap()
        sleep(3)
        // The choices are there before and after playing; pick the first card's first one (right or wrong, the words appear).
        let choice = app.buttons["choice-0"].firstMatch
        XCTAssertTrue(choice.waitForExistence(timeout: 3), "choices under the first card")
        choice.tap()
        sleep(1)
        attach(app, "listening-2-answered")
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label CONTAINS 'Right first time' OR label CONTAINS 'Not this one'")).firstMatch.waitForExistence(timeout: 3),
                      "the card says how it went, in place")
        XCTAssertTrue(app.buttons["Slower"].firstMatch.exists)
        XCTAssertFalse(app.buttons["Hear 0"].exists)
        check(app)
    }

    /// Set 7: voice messages with two questions, a correction, who-needs with badges.
    func testSetSevenMessageAndExchange() throws {
        let app = launch(["-qa-listening", "7", "-qa-top"])
        sleep(2)
        attach(app, "listening-7-fresh")
        // A message card: its two questions sit under one Play; both must be answered before anything is marked.
        let message = app.staticTexts["A voice message. Listen, then answer the questions below."].firstMatch
        XCTAssertTrue(message.waitForExistence(timeout: 5))
        XCTAssertTrue(app.staticTexts["VOICE MESSAGE"].firstMatch.exists)
        XCTAssertTrue(app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH 'Speaker A'")).firstMatch.exists
                      || app.otherElements.matching(NSPredicate(format: "label BEGINSWITH 'Speaker A'")).firstMatch.exists, "an exchange shows its speaker badges")
        check(app)
    }

    /// Set 3 holds a scene to follow and exchanges; the page is worked through by QA so every reveal is on screen.
    func testSetThreeRevealedAndTheWeeklySample() throws {
        let app = launch(["-qa-listening", "3", "-qa-partial", "8", "-qa-top"])
        sleep(2)
        attach(app, "listening-3-done-top")
        XCTAssertTrue(app.staticTexts["TRAY"].firstMatch.waitForExistence(timeout: 5), "the scene's tray")
        XCTAssertTrue(app.staticTexts["PLACES"].firstMatch.exists)
        app.swipeUp(); sleep(1)
        attach(app, "listening-3-done-mid")
        app.swipeUp(); sleep(1)
        attach(app, "listening-3-done-end")
        check(app)
        app.terminate()

        let week = launch(["-qa-listening-week", "1", "-qa-top"])
        sleep(2)
        XCTAssertTrue(week.buttons["Listen to the message"].firstMatch.waitForExistence(timeout: 5), "a reserved message, played by its text")
        attach(week, "listening-week-1")
        check(week)
    }

    private func check(_ app: XCUIApplication) {
        let screen = app.windows.firstMatch.frame
        var bad: [String] = []
        for e in app.descendants(matching: .any).allElementsBoundByIndex.prefix(400) {
            let f = e.frame
            guard f.width > 0, f.width < screen.width * 3 else { continue }
            if f.minX < screen.minX - 0.5 || f.maxX > screen.maxX + 0.5 { bad.append("\(e.elementType.rawValue) '\(e.label.prefix(40))' \(f)") }
        }
        XCTAssertTrue(bad.isEmpty, "past the screen edge:\n" + bad.joined(separator: "\n"))
    }

    private func attach(_ app: XCUIApplication, _ name: String) {
        let shot = XCTAttachment(screenshot: app.screenshot())
        shot.name = name
        shot.lifetime = .keepAlways
        add(shot)
    }
}
