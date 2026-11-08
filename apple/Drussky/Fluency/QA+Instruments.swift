import Foundation
import LearningCore

/// QA flags for the learning instruments (after -qa-reset …):
///   -qa-judge     ink on the first open recall exercise of the current page, with Check already
///                 tapped: the self-judgement panel (Judgement/)
///   -qa-hooks     canned hooks on a few course words (open one with -qa-contents -qa-node lex:кот)
///   -qa-fluency   three weeks of synthetic fluency samples: reading, writing, listening (Fluency/)
extension QA {
    static func applyInstruments(to book: Book) {
        if has("-qa-judge"), let page = book.page {
            if let i = page.exercises.indices.first(where: { [.dictation, .translate, .fill].contains(page.exercises[$0].kind) && !page.results[$0].status.isComplete }),
               let target = page.exercises[i].target {
                let drawing = SyntheticInk.drawing(for: RussianText.unstressed(target), height: 132, wobble: 1)
                try? drawing.dataRepresentation().write(to: book.inkURL(page: page, exercise: i))
            }
        }
        if has("-qa-hooks") {
            let canned: [(String, String, String?)] = [
                ("кот", "COT — a cat curled up asleep on a cot", "🐈"),
                ("дом", "DOME — a house with a dome for a roof", "🏠"),
                ("нет", "NYET — the net stretched across the door says no", "🙅"),
                ("молоко", "ma-la-KO — the cow (ko!) gives milk", "🥛"),
                ("вода", "va-DA — DArk water", "💧"),
                ("хлеб", "KHLEB — a loaf you have to clear your throat for", "🍞"),
            ]
            for (ru, text, emoji) in canned where book.word(ru) != nil {
                book.setHook(MemoryHook(text: text, emoji: emoji, source: .tutor, created: book.now().addingTimeInterval(-3 * 86_400), model: Tutor.model), for: ru)
            }
        }
        if has("-qa-fluency") { syntheticFluency(book) }
    }

    /// Reading a little faster week by week, writing a word more smoothly, listening at two speeds.
    private static func syntheticFluency(_ book: Book) {
        let now = book.now()
        let passage = "Робот проснулся ночью. На станции было тихо."
        for day in stride(from: 21, through: 0, by: -3) {
            let t = now.addingTimeInterval(-Double(day) * 86_400 - 3600)
            let wpm = 58 + Double(21 - day) * 1.4 + (day % 2 == 0 ? 3 : -2)
            book.fluency.record(wpm, metric: Fluency.readWPM, component: Fluency.passageID(passage), at: t)
            book.fluency.record(wpm, metric: Fluency.readWPM, component: Fluency.reading, at: t)
        }
        let word = book.knowledge.domain.utteranceID("кот") ?? ComponentID(.lexeme, "кот")
        for day in stride(from: 20, through: 0, by: -2) {
            let t = now.addingTimeInterval(-Double(day) * 86_400 - 7200)
            let sps = 1.3 + Double(20 - day) * 0.045 + (day % 4 == 0 ? 0.1 : -0.05)
            let legible = min(1, 0.55 + Double(20 - day) * 0.02)
            for c in [word, Fluency.writing] {
                book.fluency.record(sps, metric: Fluency.strokesPerSecond, component: c, at: t)
                book.fluency.record(legible, metric: Fluency.legibility, component: c, at: t)
            }
        }
        for i in 0..<12 {
            let t = now.addingTimeInterval(-Double(i) * 86_400 - 1800)
            book.fluency.record(i % 3 == 0 ? 0 : 1, metric: Fluency.listening, component: Fluency.listeningSkill, note: "1", at: t)
            book.fluency.record(i % 6 == 5 ? 0 : 1, metric: Fluency.listening, component: Fluency.listeningSkill, note: "0.75", at: t)
        }
    }
}
