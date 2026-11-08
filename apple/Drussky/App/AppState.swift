import Observation
import SwiftUI
import UIKit

/// What the app is showing: the notebook, a recovery screen for a notebook that exists but
/// can't be opened (never replaced silently), or a missing course.
@MainActor
@Observable
final class AppState {
    enum Phase {
        case ready(Book, Voice, SpeakingTurn)
        case recovery(NotebookProblem)
        case failed(String)
    }

    private(set) var phase: Phase = .failed("Loading…")
    /// A one-off message for the learner (after a restore).
    var notice: String?
    private var course: Course?
    private var voice: Voice?
    private var speech: SpeakingTurn?
    private let root: URL
    private var launched = false

    init(root: URL? = nil) {
        Theme.registerFonts()
        Voice.configureSession()
        importTutorKey()
        self.root = root ?? BookStore.standardRoot
        load()
    }

    var book: Book? { if case .ready(let b, _, _) = phase { b } else { nil } }

    /// Opens the notebook folder (again).
    func load() {
        do {
            if course == nil {
                guard let url = Bundle.main.url(forResource: "course", withExtension: "json") else {
                    phase = .failed("course.json is missing from the app bundle.")
                    return
                }
                course = try JSONDecoder().decode(Course.self, from: Data(contentsOf: url))
            }
            guard let course else { return }
            let store = BookStore(root: root)
            let first = !launched
            launched = true
            if first { QA.prepare(store: store) }
            switch Book.open(course: course, store: store) {
            case .failure(let problem):
                phase = .recovery(problem)
            case .success(let book):
                if first { QA.apply(to: book) }
                let voice = self.voice ?? Voice()
                let speech = self.speech ?? SpeakingTurn()
                self.voice = voice
                self.speech = speech
                SpeechCheck.warmUp(with: voice.recordingURL(for: "спаси́бо"))
                phase = .ready(book, voice, speech)
            }
        } catch {
            phase = .failed("\(error)")
        }
    }

    /// Starts a fresh notebook only after the unreadable one has been moved aside intact.
    func startFresh() throws {
        try BookStore(root: root).setAside()
        load()
    }

    /// Restores an exported notebook: validated in isolation first, then swapped in with the
    /// current notebook kept beside it. Returns a summary for the learner.
    func restore(from archive: URL) async throws -> String {
        let scoped = archive.startAccessingSecurityScopedResource()
        defer { if scoped { archive.stopAccessingSecurityScopedResource() } }
        let scratch = FileManager.default.temporaryDirectory.appendingPathComponent("restore-\(UUID().uuidString)")
        defer { try? FileManager.default.removeItem(at: scratch) }
        let candidate = try NotebookArchive.candidate(from: archive, scratch: scratch)
        if let book { await book.close() }
        speech?.cancel()
        do {
            let kept = try NotebookArchive.install(candidate, at: root)
            load()
            let where_ = kept.map { " Your previous notebook is kept as \($0.lastPathComponent)." } ?? ""
            return "Restored \(candidate.pages) pages, \(candidate.inkFiles) handwriting files, \(candidate.recordings) recordings and \(candidate.events) answers.\(where_)"
        } catch {
            load()
            throw error
        }
    }

    /// Writes everything pending when the app leaves the foreground, holding a background task
    /// until it's done.
    func flushForBackground() {
        guard let book else { return }
        let app = UIApplication.shared
        let token = BackgroundToken()
        token.id = app.beginBackgroundTask(withName: "Save notebook") { token.end() }
        Task {
            await book.flush()
            token.end()
        }
    }

    private final class BackgroundToken: @unchecked Sendable {
        var id: UIBackgroundTaskIdentifier = .invalid
        func end() {
            let id = self.id
            self.id = .invalid
            guard id != .invalid else { return }
            DispatchQueue.main.async { UIApplication.shared.endBackgroundTask(id) }
        }
    }
}
