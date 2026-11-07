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

/// One-time provisioning: a key dropped into Documents/tutor-key.txt (e.g. with `devicectl
/// device copy to`) moves into the Keychain and the file is deleted.
@MainActor
func importTutorKey() {
    let url = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("tutor-key.txt")
    guard let key = try? String(contentsOf: url, encoding: .utf8).trimmingCharacters(in: .whitespacesAndNewlines) else { return }
    if !key.isEmpty { Keychain.apiKey = key }
    try? FileManager.default.removeItem(at: url)
}

@MainActor
enum AppModel {
    case ready(Book, Voice, SpeechCheck)
    case failed(String)

    static func load() -> AppModel {
        Theme.registerFonts()
        Voice.configureSession()
        importTutorKey()
        guard let url = Bundle.main.url(forResource: "course", withExtension: "json") else {
            return .failed("course.json is missing from the app bundle.")
        }
        do {
            let course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
            let store = BookStore.standard()
            QA.prepare(store: store)
            let book = Book(course: course, store: store)
            QA.apply(to: book)
            let voice = Voice()
            SpeechCheck.warmUp(with: voice.recordingURL(for: "спаси́бо"))
            return .ready(book, voice, SpeechCheck())
        } catch {
            return .failed("\(error)")
        }
    }
}
