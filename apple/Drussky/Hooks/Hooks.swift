import Foundation
import LearningCore

/// A keyword mnemonic for one word: an English sound-alike tied to the meaning in one vivid
/// line (мы́ло "soap": "MEAL-o — wash your hands before the meal"), with an emoji when one helps.
/// Dual coding: the sound, the image, the meaning, in one hook.
struct MemoryHook: Codable, Sendable, Hashable {
    enum Source: String, Codable, Sendable {
        /// The tutor's line, accepted as it came.
        case tutor
        /// The tutor's line, reworded by the learner.
        case edited
        /// The learner's own.
        case mine

        init(from decoder: Decoder) throws {
            self = Source(rawValue: try decoder.singleValueContainer().decode(String.self)) ?? .tutor
        }
    }

    var text: String
    var emoji: String? = nil
    var source: Source = .tutor
    var created: Date
    var model: String? = nil

    init(text: String, emoji: String? = nil, source: Source = .tutor, created: Date, model: String? = nil) {
        self.text = text
        self.emoji = emoji
        self.source = source
        self.created = created
        self.model = model
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        text = try c.decode(String.self, forKey: .text)
        emoji = try c.decodeIfPresent(String.self, forKey: .emoji)
        source = try c.decodeIfPresent(Source.self, forKey: .source) ?? .tutor
        created = try c.decodeIfPresent(Date.self, forKey: .created) ?? .distantPast
        model = try c.decodeIfPresent(String.self, forKey: .model)
    }
}

/// Every hook the learner has, by word key (`RussianText.key`), and when hooks were last written
/// in a batch for the hardest words. Saved inside book.json (optional field `hooks`).
struct HookShelf: Codable, Sendable, Equatable {
    var hooks: [String: MemoryHook] = [:]
    var batchedAt: Date? = nil

    init() {}

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        hooks = try c.decodeIfPresent([String: MemoryHook].self, forKey: .hooks) ?? [:]
        batchedAt = try c.decodeIfPresent(Date.self, forKey: .batchedAt)
    }

    var isEmpty: Bool { hooks.isEmpty && batchedAt == nil }
}

/// How visibly a hook is shown before a recall: faint from the start, fainter with every
/// successful recall since it was made, gone after a handful. The hook is scaffolding; the
/// memory has to carry itself (desirable difficulty).
enum HookFading {
    static let steps: [Double] = [0.8, 0.65, 0.5, 0.35, 0.2]

    /// Opacity for a hook after `recalls` successful recalls since it was made; 0 hides it.
    static func opacity(recalls: Int) -> Double {
        recalls < 0 ? steps[0] : (recalls < steps.count ? steps[recalls] : 0)
    }
}

extension Book {
    func hook(for ru: String) -> MemoryHook? { hooks.hooks[RussianText.key(ru)] }

    /// Sets or removes (nil) the hook of a word.
    func setHook(_ hook: MemoryHook?, for ru: String) {
        let key = RussianText.key(ru)
        guard !key.isEmpty else { return }
        if let hook { hooks.hooks[key] = hook } else { hooks.hooks.removeValue(forKey: key) }
    }

    /// Successful recalls (hear → write, English → write) of a word since its hook was made.
    func recallsSinceHook(for ru: String) -> Int {
        guard let hook = hook(for: ru), let id = knowledge.domain.utteranceID(ru) else { return 0 }
        return knowledge.model.history(of: id).filter {
            $0.type == .outcome && $0.t > hook.created && ($0.grade?.isSuccess ?? false)
                && [Modality.hearWrite, .meaningWrite].contains($0.modality)
        }.count
    }

    /// How faintly to show a word's hook before a recall (0: not at all).
    func hookOpacity(for ru: String) -> Double {
        hook(for: ru) == nil ? 0 : HookFading.opacity(recalls: recallsSinceHook(for: ru))
    }

    /// The words missed most (reviews graded *again*, summed over their traces), lexicon or kept,
    /// without a hook yet: the ones a hook would help first.
    func wordsWantingHooks(limit: Int = 60) -> [Course.Word] {
        var scored: [(Course.Word, Int)] = []
        for id in knowledge.practisedWords {
            guard hooks.hooks[id.key] == nil, let w = word(id.key) else { continue }
            let misses = knowledge.model.traces(of: id).values.reduce(0) { $0 + $1.reviews.filter { $0.grade == .again }.count }
            if misses > 0 { scored.append((w, misses)) }
        }
        return scored.sorted { $0.1 != $1.1 ? $0.1 > $1.1 : $0.0.ru < $1.0.ru }.prefix(limit).map(\.0)
    }

    /// Every hook as Markdown, for export.
    func hooksMarkdown() -> String {
        var lines = ["# My hooks", ""]
        for (key, h) in hooks.hooks.sorted(by: { $0.key < $1.key }) {
            let w = word(key)
            let head = [w?.ru ?? key, w.map { "— \($0.en)" }].compactMap { $0 }.joined(separator: " ")
            lines.append("- **\(head)**: \([h.emoji, h.text].compactMap { $0 }.joined(separator: " "))" + (h.source == .tutor ? "" : " _(\(h.source == .mine ? "mine" : "edited"))_"))
        }
        return lines.joined(separator: "\n") + "\n"
    }

    func exportHooks() -> URL? {
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("My hooks.md")
        do { try hooksMarkdown().write(to: url, atomically: true, encoding: .utf8); return url } catch { return nil }
    }
}
