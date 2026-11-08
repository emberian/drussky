import XCTest

/// The phone: nothing on a page may run past the screen's edge. Run on an iPhone simulator;
/// the element tree (with frames) is printed for reading when something does.
@MainActor
final class PhoneLayoutUITests: XCTestCase {
    private func overflowing(_ app: XCUIApplication) -> [String] {
        let screen = app.windows.firstMatch.frame
        var out: [String] = []
        for e in app.descendants(matching: .any).allElementsBoundByIndex.prefix(400) {
            let f = e.frame
            guard f.width > 0, f.width < screen.width * 3 else { continue }
            if f.minX < screen.minX - 0.5 || f.maxX > screen.maxX + 0.5 {
                out.append("\(e.elementType.rawValue) '\(e.label.prefix(40))' \(f)")
            }
        }
        return out
    }

    func testCoursePageFitsThePhone() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "2", "-qa-partial", "2", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.buttons["Add a page"].firstMatch.waitForExistence(timeout: 20))
        sleep(2)
        let screen = app.windows.firstMatch.frame
        print("SCREEN \(screen)")
        print(app.debugDescription)
        let bad = overflowing(app)
        XCTAssertTrue(bad.isEmpty, "past the screen edge:\n" + bad.joined(separator: "\n"))
        let shot = XCTAttachment(screenshot: app.screenshot()); shot.name = "phone-course"; shot.lifetime = .keepAlways; add(shot)
    }

    func testPocketPageFitsThePhone() throws {
        let app = XCUIApplication()
        app.launchArguments = ["-qa-reset", "-qa-complete", "6", "-qa-pocket", "-qa-top"]
        app.launch()
        XCTAssertTrue(app.buttons["Add a page"].firstMatch.waitForExistence(timeout: 20))
        sleep(2)
        let bad = overflowing(app)
        XCTAssertTrue(bad.isEmpty, "past the screen edge:\n" + bad.joined(separator: "\n"))
        let shot = XCTAttachment(screenshot: app.screenshot()); shot.name = "phone-pocket"; shot.lifetime = .keepAlways; add(shot)
    }
}
