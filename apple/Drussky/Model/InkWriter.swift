import Foundation

/// Every ink change gets a process-wide increasing revision, so a write can always be told
/// apart from an older one, even across strip views recreated for the same file.
@MainActor
enum InkRevision {
    private static var counter = 0
    static func next() -> Int { counter += 1; return counter }
}

/// The one writer of a notebook's ink files. Writes are serialized, coalesced per file and
/// revisioned: a stale drawing (or a stale "empty", which deletes) can never replace newer
/// ink. Dirty drawings stay pending until written; `flush()` drains them (background, export).
actor InkWriter {
    private struct Pending { var revision: Int; var data: Data? }
    private var pending: [URL: Pending] = [:]
    private var written: [URL: Int] = [:]
    private var timer: Task<Void, Never>?
    private var closed = false
    private let delay: Duration
    /// Called (off the main actor) with a description whenever a write fails; the ink stays pending.
    private var onFailure: (@Sendable (String) -> Void)?

    init(delay: Duration = .milliseconds(250)) { self.delay = delay }

    func setFailureHandler(_ handler: @escaping @Sendable (String) -> Void) { onFailure = handler }

    /// Queues `data` (nil: the strip is empty, delete the file) as revision `revision` of `url`.
    func submit(_ url: URL, revision: Int, data: Data?) {
        guard !closed else { return }
        if let w = written[url], w >= revision { return }
        if let p = pending[url], p.revision >= revision { return }
        pending[url] = Pending(revision: revision, data: data)
        guard timer == nil else { return }
        let delay = self.delay
        timer = Task { [weak self] in
            try? await Task.sleep(for: delay)
            await self?.timerFired()
        }
    }

    private func timerFired() {
        timer = nil
        _ = writeAll()
    }

    /// Writes everything pending now. Returns the failures (their ink stays pending).
    @discardableResult
    func flush() -> [String] {
        timer?.cancel()
        timer = nil
        return writeAll()
    }

    var hasPending: Bool { !pending.isEmpty }

    /// Writes what is pending and refuses anything later (the folder is about to be replaced).
    func close() -> [String] {
        let failures = flush()
        if failures.isEmpty { closed = true }
        return failures
    }

    /// A notebook close failed in another writer; the live notebook still owns this folder.
    func resume() { closed = false }

    /// Moves `url`'s ink (after writing anything pending for it) to `destination`, so a new
    /// attempt can start on a clean strip while the old one is kept.
    func archive(_ url: URL, to destination: URL) throws {
        guard !closed else { throw NotebookError(message: "This notebook is closed.") }
        if pending[url] != nil {
            let failures = write(url)
            if let f = failures { throw NotebookError(message: f) }
        }
        let fm = FileManager.default
        guard fm.fileExists(atPath: url.path) else { return }
        try fm.moveItem(at: url, to: destination)
    }

    private func writeAll() -> [String] {
        pending.keys.sorted { $0.path < $1.path }.compactMap { write($0) }
    }

    private func write(_ url: URL) -> String? {
        guard let p = pending[url] else { return nil }
        do {
            if let data = p.data {
                try data.write(to: url, options: .atomic)
            } else if FileManager.default.fileExists(atPath: url.path) {
                try FileManager.default.removeItem(at: url)
            }
            pending[url] = nil
            written[url] = p.revision
            return nil
        } catch {
            let message = "ink \(url.lastPathComponent): \((error as NSError).localizedDescription)"
            onFailure?(message)
            return message
        }
    }
}
