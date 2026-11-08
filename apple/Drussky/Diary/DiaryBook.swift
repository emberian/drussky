import Foundation

/// One diary line as it appears in "My Russian": where it was written and what came of it.
struct DiaryRecord: Identifiable, Hashable {
    var ref: ExerciseRef
    var pageNumber: Int
    var entry: DiaryEntry
    var id: String { ref.item }
}

extension Book {
    /// Every diary line ever written, newest first.
    var diaryEntries: [DiaryRecord] {
        var out: [DiaryRecord] = []
        for page in pages {
            for (i, r) in page.results.enumerated() where page.exercises[i].kind == .diary {
                if let d = r.diary { out.append(DiaryRecord(ref: ExerciseRef(pageID: page.id, index: i), pageNumber: page.number, entry: d)) }
            }
        }
        return out.sorted { $0.entry.submitted > $1.entry.submitted }
    }

    /// Records what was written (before the tutor has seen it), as an attempt.
    func submitDiary(_ ref: ExerciseRef, entry: DiaryEntry) {
        record(ref) { r in
            r.diary = entry
            r.attempts += 1
        }
    }

    /// The correction lands: the entry finishes (right when nothing needed changing), and every
    /// corrected form joins the learner's words so reviews bring it back (hear→write, en→write).
    /// A correction whose marks don't account for its corrected line (`isConsistent`) is kept to
    /// read but finishes the line ungraded: it adds no words and awards nothing.
    func finishDiary(_ ref: ExerciseRef, correction: DiaryCorrection) {
        guard let current = result(ref), let entry = current.diary else { return }
        var correction = correction
        let verified = correction.isConsistent(with: entry.text)
        correction.verified = verified
        if verified {
            for e in correction.edits where !RussianText.key(e.replacement).isEmpty && !RussianText.letters(in: e.replacement).isEmpty {
                keep(e.replacement, en: e.en ?? "")
            }
        }
        record(ref) { r in
            r.diary?.correction = correction
            r.status = verified && correction.edits.isEmpty ? .correct : .done
        }
    }

    /// Finishes a line nobody corrected (no key, or the learner chose to keep it as written).
    func keepDiaryUncorrected(_ ref: ExerciseRef) {
        guard result(ref)?.diary != nil else { return }
        record(ref) { $0.status = .done }
    }

    /// ＋ → Write in my diary: a page with one diary line, its prompt chosen from what the
    /// learner has met, placed right after the current page.
    func insertDiaryPage() {
        var ex = Exercise(kind: .diary)
        ex.prompt = DiaryPrompts.pick(for: self)
        insertWrittenPage(title: "Дневни́к", intro: "Write a few words in Russian. Send them to your tutor for corrections, or keep them as written.",
                          unitTitle: "My Russian", exercises: [ex], newWords: [])
    }

    /// Red-pen categories over the last `days`, most frequent first.
    func diaryCategoryCounts(days: Int = 7) -> [(DiaryEdit.Category, Int)] {
        let since = now().addingTimeInterval(-Double(days) * 86_400)
        var counts: [DiaryEdit.Category: Int] = [:]
        for d in diaryEntries where d.entry.submitted >= since {
            for e in d.entry.correction?.edits ?? [] { counts[e.category, default: 0] += 1 }
        }
        return counts.sorted { $0.value > $1.value || ($0.value == $1.value && $0.key.rawValue < $1.key.rawValue) }.map { ($0.key, $0.value) }
    }

    /// "My Russian" as a Markdown file in the temporary folder: every entry with its correction.
    func exportDiary() -> URL? {
        let f = DateFormatter()
        f.dateStyle = .long
        f.timeStyle = .none
        var lines = ["# My Russian", "", "Everything written in the Drussky diary, newest first.", ""]
        for d in diaryEntries {
            lines.append("## \(f.string(from: d.entry.submitted)) · page \(d.pageNumber)")
            lines.append("")
            lines.append("_\(d.entry.prompt)_")
            lines.append("")
            lines.append("> \(d.entry.text)")
            if let c = d.entry.correction {
                lines.append("")
                lines.append(c.edits.isEmpty ? "Correct as written." : "Corrected: **\(c.corrected)**")
                for e in c.edits {
                    lines.append("- \(e.span) → \(e.replacement.isEmpty ? "(remove)" : e.replacement) — \(e.category.title)\(e.why.isEmpty ? "" : ": \(e.why)")")
                }
                if !c.praise.isEmpty { lines.append(""); lines.append(c.praise) }
                for t in c.teach { lines.append(""); lines.append("**\(t.title)** \(RussianText.plainMarkup(t.note))") }
            }
            lines.append("")
        }
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("My Russian.md")
        do { try lines.joined(separator: "\n").write(to: url, atomically: true, encoding: .utf8); return url } catch { return nil }
    }
}

extension RussianText {
    /// [[ру́сский|english]] → ру́сский (english), for plain-text export.
    static func plainMarkup(_ s: String) -> String {
        s.replacing(/\[\[([^|\]]+)\|([^\]]*)\]\]/) { m in String(m.1) + (m.2.isEmpty ? "" : " (\(m.2))") }
    }
}

/// Prompts for the ＋ menu's diary page, unlocked by the units the learner has reached.
/// Each uses only topics whose words the course has taught by then.
enum DiaryPrompts {
    struct Entry: Codable, Sendable, Hashable {
        var unit: String
        var prompt: String
    }

    private struct File: Codable {
        var version: Int
        var prompts: [Entry]
    }

    static let entries: [Entry] = {
        guard let url = Bundle.main.url(forResource: "diary-prompts", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let file = try? JSONDecoder().decode(File.self, from: data) else {
            return []
        }
        return file.prompts
    }()

    static let bank: [(unit: String, prompt: String)] = entries.map { ($0.unit, $0.prompt) }

    /// A prompt for a unit the learner has reached, least recently used first.
    @MainActor
    static func pick(for book: Book) -> String {
        let reached = Set(book.pages.compactMap(\.sourceID).compactMap { id in book.course.pageSequence.first { $0.page.id == id }?.unit.id })
        let used = book.diaryEntries.map(\.entry.prompt) + book.pages.flatMap { $0.exercises.compactMap { $0.kind == .diary ? $0.prompt : nil } }
        let open = bank.filter { reached.contains($0.unit) }.map(\.prompt)
        let pool = open.isEmpty ? [bank[0].prompt, bank[1].prompt] : open
        let fresh = pool.filter { !used.contains($0) }
        return (fresh.isEmpty ? pool : fresh).randomElement() ?? bank[0].prompt
    }
}
