import Foundation
import PencilKit

extension QA {
    /// `-qa-diary`: three diary pages with written entries and canned corrections (no tutor key
    /// needed), the newest one open on screen with its red pen; earlier ones feed "My Russian".
    static func diaryPages(_ book: Book) {
        let samples: [(prompt: String, wrote: String, correction: DiaryCorrection?, daysAgo: Double)] = [
            ("Where is everything? Say where three things are.", "Кот тут. Мама дома. Книга там.",
             DiaryCorrection(corrected: "Кот тут. Ма́ма до́ма. Кни́га там.", edits: [],
                             praise: "Three true sentences, every ending in place.", teach: [], at: Date()), 5),
            ("What do you love? Two or three things.", "Я люблю кофе и книга. Мой сестра тоже.",
             DiaryCorrection(corrected: "Я люблю́ ко́фе и кни́ги. Моя́ сестра́ то́же.",
                             edits: [DiaryEdit(span: "книга", replacement: "кни́ги", category: .ending,
                                               why: "After люблю́ the thing loved is the object; for books in general, the plural кни́ги.", en: "books", pattern: "plural"),
                                     DiaryEdit(span: "Мой сестра", replacement: "Моя́ сестра́", category: .agreement,
                                               why: "Сестра́ is feminine, so *my* is моя́.", en: "my sister", pattern: "gender")],
                             praise: "Люблю́ and то́же are exactly right, and the sentence reads naturally.",
                             teach: [DiaryTeach(title: "Мой follows the noun's gender", note: "Мой for он-words, моя́ for она́-words, моё for оно́-words: [[моя́ сестра́|my sister]], [[мой брат|my brother]].", pattern: "gender")],
                             at: Date()), 2),
            ("What are you reading or thinking about today?", "Я читаю книга о космос. Это интересно.",
             DiaryCorrection(corrected: "Я чита́ю кни́гу о ко́смосе. Э́то интере́сно.",
                             edits: [DiaryEdit(span: "книга", replacement: "кни́гу", category: .ending,
                                               why: "The thing you read is the object: кни́га becomes кни́гу.", en: "book (object)", pattern: "acc-u"),
                                     DiaryEdit(span: "космос", replacement: "ко́смосе", category: .ending,
                                               why: "After о (about) the noun takes -е: о ко́смосе.", en: "space (about space)", pattern: "prep-e")],
                             praise: "Я чита́ю is perfect, and интере́сно is a lovely word to have already.",
                             teach: [DiaryTeach(title: "The object takes -у", note: "What you read, see or love is the object, and -а becomes -у: [[я чита́ю кни́гу|I am reading a book]].", pattern: "acc-u")],
                             at: Date()), 0),
        ]
        let base = Date()
        for s in samples {
            book.now = { base.addingTimeInterval(-s.daysAgo * 86_400) }
            var ex = Exercise(kind: .diary)
            ex.prompt = s.prompt
            book.insertWrittenPage(title: "Дневни́к · Diary", intro: "A few words of your own Russian. Write what you can; your tutor marks it in red.",
                                   unitTitle: "My Russian", exercises: [ex], newWords: [])
            guard let page = book.page else { continue }
            try? diaryInk(s.wrote).dataRepresentation().write(to: book.inkURL(page: page, exercise: 0))
            let ref = ExerciseRef(pageID: page.id, index: 0)
            book.submitDiary(ref, entry: DiaryEntry(prompt: s.prompt, text: s.wrote, recognized: nil, submitted: book.now()))
            if var c = s.correction {
                c.at = book.now()
                book.finishDiary(ref, correction: c)
            }
        }
        book.now = Date.init
        book.saveNow()
    }

    /// Hand-sized cursive, one sentence per ruled line, in the diary strip's design space.
    static func diaryInk(_ text: String) -> PKDrawing {
        let line = StripMetrics(height: StripMetrics.designHeight)
        var out = PKDrawing()
        let sentences = text.split(separator: ". ", omittingEmptySubsequences: true).map { s in
            s.hasSuffix(".") || s.hasSuffix("!") || s.hasSuffix("?") ? String(s) : String(s) + "."
        }
        for (k, sentence) in sentences.enumerated() {
            let d = SyntheticInk.drawing(for: sentence, height: StripMetrics.designHeight, wobble: Double(k) * 1.3, cursive: true)
            // Shrink toward the baseline (a hand writes smaller than a model), then drop a line.
            let t = CGAffineTransform(translationX: 0, y: line.baselineY).scaledBy(x: 0.72, y: 0.72)
                .translatedBy(x: 0, y: -line.baselineY).translatedBy(x: 0, y: CGFloat(k) * StripMetrics.designHeight / 0.72)
            out.append(d.transformed(using: t))
        }
        return out
    }
}
