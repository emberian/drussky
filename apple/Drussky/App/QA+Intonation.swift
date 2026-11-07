import Foundation

extension QA {
    /// `-qa-intonation`: a read-aloud page whose "take" is a bundled statement (Я го́лоден.) read
    /// against a question model (Вы лю́бите му́зыку?), so the pitch plot can be looked at
    /// without a microphone.
    static func intonationPage(_ book: Book) {
        var ex = Exercise(kind: .readAloud)
        ex.text = "Вы лю́бите му́зыку?"
        book.insertWrittenPage(title: "Intonation", intro: nil, unitTitle: "QA", exercises: [ex], newWords: [])
        guard let page = book.page,
              let take = Bundle.main.url(forResource: "intonation-ya-goloden", withExtension: "m4a", subdirectory: "Audio") else { return }
        let dest = book.recordingURL(page: page, exercise: 0)
        try? FileManager.default.removeItem(at: dest)
        try? FileManager.default.copyItem(at: take, to: dest)
        book.record(page: book.currentPage, exercise: 0) { r in
            r.attempts = 1
            r.recording = dest.lastPathComponent
            r.recognized = "вы любите музыку"
            r.status = .correct
        }
    }
}
