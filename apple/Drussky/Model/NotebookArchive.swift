import Compression
import Foundation
import LearningCore
import PencilKit

/// Export and restore of the whole notebook folder (book.json, ink/, voice/, events.jsonl).
enum NotebookArchive {
    /// Zips `root` with the system's own archiver (the same zip the share sheet would make).
    static func export(root: URL, to dest: URL = FileManager.default.temporaryDirectory.appendingPathComponent("Drussky notebook.zip")) -> URL? {
        var output: URL?
        var error: NSError?
        NSFileCoordinator().coordinate(readingItemAt: root, options: .forUploading, error: &error) { zipped in
            try? FileManager.default.removeItem(at: dest)
            if (try? FileManager.default.copyItem(at: zipped, to: dest)) != nil { output = dest }
        }
        return output
    }

    /// What a candidate notebook contains, checked in isolation before it may replace anything.
    struct Candidate: Sendable {
        var folder: URL
        var pages: Int
        var completed: Int
        var events: Int
        var inkFiles: Int
        var recordings: Int
        var missingRecordings: [String]
    }

    /// Unpacks `archive` (a zip from Export, or a notebook folder) into `scratch` and validates it:
    /// book.json must open cleanly (the same loader and invariants as launch), every ink file
    /// must be a drawing, and every recording a result names must be present.
    static func candidate(from archive: URL, scratch: URL) throws -> Candidate {
        let fm = FileManager.default
        try? fm.removeItem(at: scratch)
        try fm.createDirectory(at: scratch, withIntermediateDirectories: true)
        var isDir: ObjCBool = false
        if fm.fileExists(atPath: archive.path, isDirectory: &isDir), isDir.boolValue {
            try fm.copyItem(at: archive, to: scratch.appendingPathComponent("notebook"))
        } else {
            try ZipReader(data: Data(contentsOf: archive)).extract(to: scratch)
        }
        guard let folder = findNotebook(in: scratch) else { throw NotebookError(message: "there is no book.json in it") }
        let store = BookStore(root: folder)
        guard case .loaded(let state, let repairs) = store.load() else {
            if case .unavailable(let p) = store.load() { throw NotebookError(message: p.message) }
            throw NotebookError(message: "its book.json is missing")
        }
        guard repairs.isEmpty else { throw NotebookError(message: "its book.json needs repairs: \(repairs.joined(separator: "; "))") }
        let ink = (try? fm.contentsOfDirectory(at: folder.appendingPathComponent("ink"), includingPropertiesForKeys: nil)) ?? []
        for url in ink where url.pathExtension == "drawing" {
            guard (try? PKDrawing(data: Data(contentsOf: url))) != nil else {
                throw NotebookError(message: "the handwriting file \(url.lastPathComponent) is damaged")
            }
        }
        var missing: [String] = []
        var named = 0
        for page in state.pages {
            for r in page.results {
                for file in (r.takes ?? []) + [r.recording].compactMap({ $0 }) {
                    named += 1
                    if !fm.fileExists(atPath: folder.appendingPathComponent("voice/\(file)").path) { missing.append(file) }
                }
            }
        }
        guard missing.isEmpty else {
            throw NotebookError(message: "\(missing.count) of its \(named) recordings are missing (\(missing.prefix(3).joined(separator: ", ")))")
        }
        for file in ["library.json", "goal.json", "typing.json"] {
            let url = folder.appendingPathComponent(file)
            guard fm.fileExists(atPath: url.path) else { continue }
            guard let data = try? Data(contentsOf: url),
                  (try? JSONSerialization.jsonObject(with: data)) != nil else {
                throw NotebookError(message: "its \(file) is damaged")
            }
        }
        for file in ["events.jsonl", "metrics.jsonl"] {
            let url = folder.appendingPathComponent(file)
            guard fm.fileExists(atPath: url.path) else { continue }
            guard let text = try? String(contentsOf: url, encoding: .utf8) else {
                throw NotebookError(message: "its \(file) is damaged")
            }
            for line in text.split(whereSeparator: \.isNewline) {
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                guard !trimmed.isEmpty else { continue }
                guard (try? JSONSerialization.jsonObject(with: Data(trimmed.utf8))) != nil else {
                    throw NotebookError(message: "its \(file) is damaged")
                }
            }
        }
        let events = EventLog(url: folder.appendingPathComponent("events.jsonl")).load().count
        return Candidate(folder: folder, pages: state.pages.count,
                         completed: state.pages.reduce(0) { $0 + $1.completedCount }, events: events,
                         inkFiles: ink.filter { $0.pathExtension == "drawing" }.count, recordings: named,
                         missingRecordings: missing)
    }

    /// Replaces the notebook at `root` with a validated candidate. The current notebook is moved
    /// (not copied, not deleted) to a uniquely named sibling folder and its location returned;
    /// if the swap fails halfway, it is moved back.
    @discardableResult
    static func install(_ candidate: Candidate, at root: URL) throws -> URL? {
        let fm = FileManager.default
        var kept: URL?
        if fm.fileExists(atPath: root.path) {
            let aside = root.deletingLastPathComponent()
                .appendingPathComponent("\(root.lastPathComponent)-before-restore-\(BookStore.stamp())", isDirectory: true)
            try fm.moveItem(at: root, to: aside)
            kept = aside
        }
        do {
            try fm.moveItem(at: candidate.folder, to: root)
        } catch {
            if let kept { try? fm.moveItem(at: kept, to: root) }
            throw error
        }
        return kept
    }

    /// The folder holding book.json: the scratch root itself or one level down (the system zip
    /// puts everything inside a folder named after the notebook).
    private static func findNotebook(in dir: URL) -> URL? {
        let fm = FileManager.default
        if fm.fileExists(atPath: dir.appendingPathComponent("book.json").path) { return dir }
        let children = (try? fm.contentsOfDirectory(at: dir, includingPropertiesForKeys: nil)) ?? []
        for child in children.sorted(by: { $0.path < $1.path }) where !child.lastPathComponent.hasPrefix("__MACOSX") {
            if fm.fileExists(atPath: child.appendingPathComponent("book.json").path) { return child }
        }
        return nil
    }
}

/// A small reader for the zip files the system archiver writes: stored and deflated entries,
/// sizes from the central directory (so data descriptors are fine). Refuses anything else.
struct ZipReader {
    let data: Data
    private let bytes: [UInt8]

    init(data: Data) {
        self.data = data
        bytes = [UInt8](data)
    }

    struct Entry { var name: String; var method: UInt16; var compressed: Int; var size: Int; var offset: Int }

    private func u16(_ at: Int) throws -> UInt16 {
        guard at >= 0, at + 2 <= bytes.count else { throw NotebookError(message: "the zip file is truncated") }
        let lo = UInt16(bytes[at]), hi = UInt16(bytes[at + 1])
        return lo | (hi << 8)
    }
    private func u32(_ at: Int) throws -> UInt32 {
        guard at >= 0, at + 4 <= bytes.count else { throw NotebookError(message: "the zip file is truncated") }
        let b0 = UInt32(bytes[at]), b1 = UInt32(bytes[at + 1]), b2 = UInt32(bytes[at + 2]), b3 = UInt32(bytes[at + 3])
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    func entries() throws -> [Entry] {
        // End of central directory: signature 0x06054b50 within the last 64 KiB.
        var eocd = -1
        var i = data.count - 22
        while i >= max(0, data.count - 65_557) {
            if try u32(i) == 0x0605_4b50 { eocd = i; break }
            i -= 1
        }
        guard eocd >= 0 else { throw NotebookError(message: "this is not a zip file") }
        let count = Int(try u16(eocd + 10))
        var p = Int(try u32(eocd + 16))
        guard count != 0xFFFF, p != 0xFFFF_FFFF else { throw NotebookError(message: "zip64 archives aren't supported") }
        var out: [Entry] = []
        for _ in 0..<count {
            guard try u32(p) == 0x0201_4b50 else { throw NotebookError(message: "the zip's directory is damaged") }
            let method = try u16(p + 10)
            let compressed = Int(try u32(p + 20)), size = Int(try u32(p + 24))
            let nameLen = Int(try u16(p + 28)), extraLen = Int(try u16(p + 30)), commentLen = Int(try u16(p + 32))
            let offset = Int(try u32(p + 42))
            guard p + 46 + nameLen <= data.count else { throw NotebookError(message: "the zip file is truncated") }
            let name = String(decoding: data[(p + 46)..<(p + 46 + nameLen)], as: UTF8.self)
            out.append(Entry(name: name, method: method, compressed: compressed, size: size, offset: offset))
            p += 46 + nameLen + extraLen + commentLen
        }
        return out
    }

    func contents(of e: Entry) throws -> Data {
        guard try u32(e.offset) == 0x0403_4b50 else { throw NotebookError(message: "the zip entry \(e.name) is damaged") }
        let start = e.offset + 30 + Int(try u16(e.offset + 26)) + Int(try u16(e.offset + 28))
        guard start + e.compressed <= data.count else { throw NotebookError(message: "the zip file is truncated") }
        let body = data[start..<(start + e.compressed)]
        switch e.method {
        case 0: return Data(body)
        case 8:
            guard e.size > 0 else { return Data() }
            var out = Data(count: e.size)
            let n = out.withUnsafeMutableBytes { dst in
                body.withUnsafeBytes { src in
                    compression_decode_buffer(dst.bindMemory(to: UInt8.self).baseAddress!, e.size,
                                              src.bindMemory(to: UInt8.self).baseAddress!, e.compressed, nil, COMPRESSION_ZLIB)
                }
            }
            guard n == e.size else { throw NotebookError(message: "the zip entry \(e.name) didn't inflate") }
            return out
        default:
            throw NotebookError(message: "the zip entry \(e.name) uses an unsupported compression")
        }
    }

    func extract(to dir: URL) throws {
        let fm = FileManager.default
        for e in try entries() {
            let parts = e.name.split(separator: "/").map(String.init)
            guard !parts.isEmpty, !parts.contains(".."), !e.name.hasPrefix("/") else { continue }
            let url = parts.reduce(dir) { $0.appendingPathComponent($1) }
            if e.name.hasSuffix("/") {
                try fm.createDirectory(at: url, withIntermediateDirectories: true)
                continue
            }
            try fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
            try contents(of: e).write(to: url)
        }
    }
}
