import Foundation

/// The standard Russian ЙЦУКЕН layout (Russian PC / Apple), as rows of lowercase keys, and the
/// logic that says which key an answer needs next.
enum RussianKeyboard {
    /// ё sits on the key left of 1 (Apple: `]`); it is shown at the end of the top row.
    static let rows: [[Character]] = [
        Array("йцукенгшщзхъ"),
        Array("фывапролджэ"),
        Array("ячсмитьбю"),
    ]
    static let homeRow: [Character] = Array("фывапролджэ")
    static let allLetters: [Character] = Array("йцукенгшщзхъфывапролджэячсмитьбюё")

    /// The key a typed answer needs next.
    enum Next: Equatable {
        case letter(Character)
        case space
        /// The typed text has left the target: the way back is a backspace.
        case backspace
        case done
    }

    /// Compares what has been typed with the target letter by letter (case, stress and ё/е
    /// aside) and names the next key. Ending in a space while the target has none is `.backspace`.
    static func next(target: String, typed: String) -> Next {
        let want = Array(RussianText.unstressed(target).lowercased().replacingOccurrences(of: "ё", with: "е"))
        let have = Array(typed.lowercased().replacingOccurrences(of: "ё", with: "е"))
        guard !want.isEmpty else { return .done }
        if have.count > want.count { return .backspace }
        for (i, h) in have.enumerated() where h != want[i] { return .backspace }
        guard have.count < want.count else { return .done }
        let ch = want[have.count]
        if ch == " " { return .space }
        return .letter(ch)
    }

    /// Where a letter sits: (row, column), nil for ё and anything not on the map.
    static func position(of letter: Character) -> (row: Int, column: Int)? {
        let lower = Character(String(letter).lowercased())
        for (r, row) in rows.enumerated() {
            if let c = row.firstIndex(of: lower) { return (r, c) }
        }
        return nil
    }

    /// Which hand and finger a key belongs to, as a touch-typing chart has it (0 = left little
    /// finger … 7 = right little finger), by column with the row's stagger.
    static func finger(row: Int, column: Int) -> Int {
        let shifted = column + (row == 2 ? 1 : 0)
        switch shifted {
        case 0: return 0
        case 1: return 1
        case 2: return 2
        case 3, 4: return 3
        case 5, 6: return 4
        case 7: return 5
        case 8: return 6
        default: return 7
        }
    }
}
