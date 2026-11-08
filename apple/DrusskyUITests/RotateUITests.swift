import XCTest

/// Turns the simulator for screenshot runs driven from the shell: Xcode 27 ships no Simulator.app
/// menu to rotate from and `simctl` has no orientation command, but the device keeps the
/// orientation a UI test leaves it in. Run one of these, then `simctl launch` and `simctl io
/// screenshot` as usual.
@MainActor
final class RotateUITests: XCTestCase {
    func testLandscapeLeft() {
        XCUIDevice.shared.orientation = .landscapeLeft
        sleep(1)
    }

    func testPortrait() {
        XCUIDevice.shared.orientation = .portrait
        sleep(1)
    }
}
