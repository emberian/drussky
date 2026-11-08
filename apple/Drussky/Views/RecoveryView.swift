import SwiftUI
import UniformTypeIdentifiers

/// Shown instead of the book when book.json exists but can't be opened. Nothing is written
/// until the learner chooses: try again, restore an export, or start fresh with the old
/// notebook moved aside intact.
struct NotebookRecoveryView: View {
    @Environment(AppState.self) private var app
    let problem: NotebookProblem
    @State private var confirmFresh = false
    @State private var error: String?

    var body: some View {
        ZStack {
            Theme.paper.ignoresSafeArea()
            VStack(alignment: .leading, spacing: 18) {
                Label("Your notebook didn't open", systemImage: "book.closed")
                    .font(.title2.weight(.bold)).foregroundStyle(Theme.text)
                Text(problem.message)
                    .font(.body).foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
                if let backup = problem.backup {
                    Text("A copy was kept as \(backup.lastPathComponent).")
                        .font(.callout).foregroundStyle(Theme.muted)
                }
                VStack(alignment: .leading, spacing: 12) {
                    Button { app.load() } label: { Label("Try again", systemImage: "arrow.clockwise") }
                        .buttonStyle(.borderedProminent)
                    RestoreButton(error: $error)
                    Button(role: .destructive) { confirmFresh = true } label: {
                        Label("Start a new notebook", systemImage: "square.and.pencil")
                    }
                    .buttonStyle(.bordered)
                }
                if let error {
                    Text(error).font(.callout).foregroundStyle(Theme.redPen).fixedSize(horizontal: false, vertical: true)
                }
            }
            .padding(32)
            .frame(maxWidth: 620, alignment: .leading)
        }
        .confirmationDialog("Start a new notebook?", isPresented: $confirmFresh, titleVisibility: .visible) {
            Button("Move the old one aside and start fresh", role: .destructive) {
                do { try app.startFresh() } catch { self.error = "Couldn't move it aside, so nothing was changed: \(error.localizedDescription)" }
            }
        } message: {
            Text("The notebook that didn't open is renamed, not deleted; it stays in the app's folder and in exports.")
        }
    }
}

/// Picks an exported notebook (.zip), checks it, and swaps it in with the current one kept.
struct RestoreButton: View {
    @Environment(AppState.self) private var app
    @Binding var error: String?
    @State private var picking = false
    @State private var chosen: URL?
    @State private var working = false

    var body: some View {
        Button { picking = true } label: {
            Label(working ? "Restoring…" : "Restore from an export…", systemImage: "arrow.down.doc")
        }
        .buttonStyle(.bordered)
        .disabled(working)
        .fileImporter(isPresented: $picking, allowedContentTypes: [.zip, .folder]) { result in
            switch result {
            case .success(let url): chosen = url
            case .failure(let e): error = e.localizedDescription
            }
        }
        .confirmationDialog("Restore this notebook?", isPresented: Binding(get: { chosen != nil }, set: { if !$0 { chosen = nil } }),
                            titleVisibility: .visible) {
            Button("Restore") {
                guard let url = chosen else { return }
                chosen = nil
                working = true
                Task {
                    do { app.notice = try await app.restore(from: url); error = nil } catch { self.error = "Not restored: \(error.localizedDescription)" }
                    working = false
                }
            }
        } message: {
            Text("It is checked first. Your current notebook is kept beside it, not deleted.")
        }
    }
}
