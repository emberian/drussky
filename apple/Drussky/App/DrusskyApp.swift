import SwiftUI

@main
struct DrusskyApp: App {
    @State private var app = AppState()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            Group {
                switch app.phase {
                case .ready(let book, let voice, let speech):
                    BookView()
                        .environment(book)
                        .environment(voice)
                        .environment(speech)
                        .id(ObjectIdentifier(book))
                        .onChange(of: scenePhase) { _, phase in if phase != .active { app.flushForBackground() } }
                case .recovery(let problem):
                    NotebookRecoveryView(problem: problem)
                case .failed(let message):
                    ContentUnavailableView("Drussky couldn't open its course", systemImage: "book.closed",
                                           description: Text(message))
                }
            }
            .environment(app)
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
