import Foundation
import LearningCore

/// Fluency, in Drussky's terms: how fast the learner reads a passage, how smoothly and legibly
/// they write a word, how much of a narrated sentence they catch at the reader's pace and at
/// three-quarter speed. LearningCore's `MetricStore` keeps the series; this names them and does
/// the arithmetic.
enum Fluency {
    // Metrics
    /// Words a minute reading a passage (finger-follow pass), on the passage and on `reading`.
    static let readWPM = "read-wpm"
    /// Words a minute saying a sentence aloud (from the take's speech span), on the sentence and on `speaking`.
    static let sayWPM = "say-wpm"
    /// Strokes a second writing a word from memory, on the word and on `writing`.
    static let strokesPerSecond = "write-sps"
    /// How surely the recognizer read the word (0–1), on the word and on `writing`.
    static let legibility = "write-legibility"
    /// 1 or 0: a narrated sentence written right from hearing; the note is the playback rate ("1", "0.75").
    static let listening = "listen"

    // Components the skill-wide series hang on.
    static let reading = ComponentID("skill", "reading")
    static let writing = ComponentID("skill", "writing")
    static let speaking = ComponentID("skill", "speaking")
    static let listeningSkill = ComponentID("skill", "listening")

    static let week: TimeInterval = 7 * 86_400

    /// A passage's component: its first four words (plus a deterministic FNV-1a hex suffix when
    /// longer than four words), so the same text measured twice is one series while two passages
    /// sharing an opening stay distinct.
    static func passageID(_ text: String) -> ComponentID {
        let normalized = RussianText.key(text)
        let tokens = normalized.split(separator: " ")
        guard !tokens.isEmpty else { return ComponentID("passage", "—") }
        let head = tokens.prefix(4).joined(separator: " ")
        guard tokens.count > 4 else { return ComponentID("passage", head) }
        var hash: UInt32 = 2_166_136_261
        for byte in normalized.utf8 {
            hash ^= UInt32(byte)
            hash = hash &* 16_777_619
        }
        return ComponentID("passage", "\(head)#\(String(format: "%08x", hash))")
    }

    /// Words in a Russian text: whitespace tokens that carry a letter.
    static func wordCount(_ text: String) -> Int {
        text.split(whereSeparator: \.isWhitespace).filter { $0.contains(where: \.isLetter) }.count
    }

    /// Words a minute from the onsets of the units said (syllables or words) and, when known, the
    /// moment speech ended. Without an end, the last unit is given the mean gap between onsets.
    /// Nil when there is nothing to measure (fewer than two onsets, no span).
    static func wpm(words: Int, onsets: [Double], end: Double? = nil) -> Double? {
        let sorted = onsets.sorted()
        guard words > 0, let first = sorted.first, let last = sorted.last else { return nil }
        var seconds: Double
        if let end, end > first {
            seconds = end - first
        } else {
            guard sorted.count >= 2 else { return nil }
            let span = last - first
            seconds = span + span / Double(sorted.count - 1)
        }
        guard seconds > 0.2 else { return nil }
        return Double(words) / seconds * 60
    }

    /// Words a minute from a timed pass over `words` words lasting `seconds`.
    static func wpm(words: Int, seconds: Double) -> Double? {
        guard words > 0, seconds > 0.5 else { return nil }
        return Double(words) / seconds * 60
    }

    /// How legibly the word was written, from what the recognizer made of it: its own confidence
    /// when it read the answer; a little when it read something close; none when it read nothing.
    static func legibility(matched: Bool, confidence: Double?, reading: String?, target: String) -> Double {
        if matched { return max(0.5, min(1, confidence ?? 0.75)) }
        guard let reading, !reading.isEmpty else { return 0 }
        let a = Array(RussianText.comparable(target).filter { !$0.isWhitespace })
        let b = Array(RussianText.comparable(reading).filter { !$0.isWhitespace })
        guard !a.isEmpty else { return 0 }
        let d = RussianDomain.substitutions(a, b).distance
        return max(0, 0.5 - 0.5 * Double(d) / Double(a.count))
    }

    // MARK: Words for the cover

    static func percent(_ x: Double) -> String { "\(Int((x * 100).rounded()))%" }
    static func round(_ x: Double) -> String { String(format: x < 10 ? "%.1f" : "%.0f", x) }

    /// One honest line about fluency this week, or nothing. Reading first, then a word whose
    /// writing changed most, then writing overall, then listening at the two speeds.
    static func coverLine(_ store: MetricStore, now: Date, label: (ComponentID) -> String) -> String? {
        if let c = store.change(readWPM, of: reading, window: week, at: now), let rel = c.relative {
            let wpm = "\(round(c.recent)) words a minute"
            if rel >= 0.08 { return "You read about \(percent(rel)) faster than last week — \(wpm)." }
            if rel <= -0.08 { return "You read a little slower than last week — \(wpm), against \(round(c.earlier)). No hurry." }
            return "You read about as fast as last week: \(wpm)."
        }
        if let (id, c) = biggestWritingChange(store, now: now), let rel = c.relative {
            let word = label(id)
            if rel >= 0.1 { return "You wrote \(word) about \(percent(rel)) faster than last week — \(round(c.recent)) strokes a second." }
            if rel <= -0.1 { return "You wrote \(word) more slowly than last week — \(round(c.recent)) strokes a second, against \(round(c.earlier)). Careful is fine." }
        }
        if let c = store.change(strokesPerSecond, of: writing, window: week, at: now), let rel = c.relative {
            if abs(rel) >= 0.08 { return "Your writing runs \(rel > 0 ? "smoother" : "slower") than last week: \(round(c.recent)) strokes a second, from \(round(c.earlier))." }
            return "Your writing runs about as it did last week: \(round(c.recent)) strokes a second."
        }
        if let l = listeningSummary(store, now: now) { return l }
        return nil
    }

    /// The word whose writing speed moved most between this week and last, with the change.
    static func biggestWritingChange(_ store: MetricStore, now: Date) -> (ComponentID, MetricStore.Change)? {
        var best: (ComponentID, MetricStore.Change)?
        for id in store.components(with: strokesPerSecond) where id.kind != "skill" {
            guard let c = store.change(strokesPerSecond, of: id, window: week, at: now), let rel = c.relative else { continue }
            if best.map({ abs(rel) > abs($0.1.relative ?? 0) }) ?? true { best = (id, c) }
        }
        return best
    }

    /// Comprehension of narrated sentences at the reader's pace and at ¾, over two weeks, when
    /// each has at least three.
    static func listeningSummary(_ store: MetricStore, now: Date) -> String? {
        guard let full = store.mean(listening, of: listeningSkill, window: 2 * week, at: now, note: "1"), full.count >= 3,
              let slow = store.mean(listening, of: listeningSkill, window: 2 * week, at: now, note: "0.75"), slow.count >= 3 else { return nil }
        return "From the reader's voice you wrote \(percent(full.value)) of the sentences right at full speed and \(percent(slow.value)) at ¾ speed."
    }

    /// The rate a listening sample was taken at, as its note.
    static func rateNote(_ rate: Float) -> String { rate < 0.9 ? "0.75" : "1" }
}
