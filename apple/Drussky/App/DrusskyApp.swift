import SwiftUI

@main
struct DrusskyApp: App {
    @State private var model = AppModel.load()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            Group {
                switch model {
                case .ready(let book, let voice, let speech):
                    BookView()
                        .environment(book)
                        .environment(voice)
                        .environment(speech)
                        .onChange(of: scenePhase) { _, phase in if phase != .active { book.saveNow() } }
                case .failed(let message):
                    ContentUnavailableView("Drussky couldn't open its course", systemImage: "book.closed",
                                           description: Text(message))
                }
            }
            .tint(Theme.accent)
        }
    }
}

@MainActor
enum AppModel {
    case ready(Book, Voice, SpeechCheck)
    case failed(String)

    static func load() -> AppModel {
        Theme.registerFonts()
        Voice.configureSession()
        guard let url = Bundle.main.url(forResource: "course", withExtension: "json") else {
            return .failed("course.json is missing from the app bundle.")
        }
        do {
            let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
            let store = BookStore.standard()
            QA.prepare(store: store)
            let book = Book(course: course, store: store)
            QA.apply(to: book)
            return .ready(book, Voice(), SpeechCheck())
        } catch {
            return .failed("\(error)")
        }
    }
}
