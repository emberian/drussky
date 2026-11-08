import Foundation
import LearningCore

extension Modality {
    /// Reading a word in someone's handwriting (read_handwriting): a modality of its own, since a
    /// word one can hear, say and print may still be unreadable in a slanted, joined hand.
    static let readCursive: Modality = "read-cursive"
}

extension Channel {
    /// A confusion between letter shapes in handwriting (ш read as и, д as б).
    static let shape: Channel = "shape"
}

extension RussianDomain {
    /// A handwritten word taken for another known word.
    func cursiveConfusions(_ ex: Exercise, response: String) -> [Confusion] {
        guard let text = ex.text, RussianText.key(text) != RussianText.key(response),
              let a = utteranceID(text), let b = utteranceID(response), a != b else { return [] }
        return [Confusion(target: a, produced: b, channel: .shape)]
    }
}
