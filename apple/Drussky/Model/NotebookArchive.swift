import zlib
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
            try validateFolder(archive)
            try fm.copyItem(at: archive, to: scratch.appendingPathComponent("notebook"))
        } else {
            try ZipReader(url: archive).extract(to: scratch)
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
            for (index, line) in text.components(separatedBy: "\n").enumerated() {
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !trimmed.isEmpty else { continue }
                do {
                    let data = Data(trimmed.utf8)
                    if file == "events.jsonl" {
                        let event = try EventLog.decoder().decode(LearningEvent.self, from: data)
                        guard event.v == LearningEvent.schemaVersion else { throw NotebookError(message: "unsupported event version \(event.v)") }
                    } else {
                        let metric = try EventLog.decoder().decode(MetricRecord.self, from: data)
                        guard metric.v == 1 else { throw NotebookError(message: "unsupported metric version \(metric.v)") }
                    }
                } catch {
                    throw NotebookError(message: "\(file), line \(index + 1): \(error.localizedDescription)")
                }
            }
        }
        // A derived cache is never part of the imported source of truth.
        let snapshot = folder.appendingPathComponent("knowledge-snapshot.json")
        if fm.fileExists(atPath: snapshot.path) { try fm.removeItem(at: snapshot) }

        let events = EventLog(url: folder.appendingPathComponent("events.jsonl")).load().count
        return Candidate(folder: folder, pages: state.pages.count,
                         completed: state.pages.reduce(0) { $0 + $1.completedCount }, events: events,
                         inkFiles: ink.filter { $0.pathExtension == "drawing" }.count, recordings: named,
                         missingRecordings: missing)
    }

    /// Folder imports obey the same bounds as ZIP imports and may not escape through links.
    private static func validateFolder(_ root: URL) throws {
        guard try root.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink != true else {
            throw NotebookError(message: "Choose the notebook folder itself, not a symbolic link.")
        }
        let keys: [URLResourceKey] = [.isSymbolicLinkKey, .isRegularFileKey, .isDirectoryKey, .fileSizeKey]
        guard let walker = FileManager.default.enumerator(at: root, includingPropertiesForKeys: keys) else {
            throw NotebookError(message: "The notebook folder couldn't be read.")
        }
        var total = 0
        for case let url as URL in walker {
            let info = try url.resourceValues(forKeys: Set(keys))
            guard info.isSymbolicLink != true, info.isDirectory == true || info.isRegularFile == true else {
                throw NotebookError(message: "The notebook contains a link or unsupported file.")
            }
            if info.isRegularFile == true {
                let size = info.fileSize ?? Int.max
                guard size <= ZipReader.maxEntryBytes else { throw NotebookError(message: "A notebook file exceeds the 128 MB import limit.") }
                total += size
                guard total <= ZipReader.maxExpandedBytes else { throw NotebookError(message: "The notebook exceeds the 1 GB import limit.") }
            }
        }
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
    // Limits cover both input and expanded output; checked before creating any extracted files.
    static let maxArchiveBytes = 256 * 1024 * 1024
    static let maxEntryBytes = 128 * 1024 * 1024
    static let maxExpandedBytes = 1024 * 1024 * 1024
    let data: Data

    init(data: Data) { self.data = data }

    init(url: URL) throws {
        let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? Int.max
        guard size <= Self.maxArchiveBytes else { throw NotebookError(message: "The archive exceeds the 256 MB import limit.") }
        data = try Data(contentsOf: url, options: .mappedIfSafe)
    }

    struct Entry {
        var name: String
        var method: UInt16
        var compressed: Int
        var size: Int
        var offset: Int
        var crc: UInt32
    }

    private func u16(_ at: Int) throws -> UInt16 {
        guard at >= 0, at + 2 <= data.count else { throw NotebookError(message: "the zip file is truncated") }
        return UInt16(data[at]) | (UInt16(data[at + 1]) << 8)
    }
    private func u32(_ at: Int) throws -> UInt32 {
        UInt32(try u16(at)) | (UInt32(try u16(at + 2)) << 16)
    }

    func entries() throws -> [Entry] {
        guard data.count <= Self.maxArchiveBytes else { throw NotebookError(message: "The archive exceeds the 256 MB import limit.") }
        var eocd = -1
        var i = data.count - 22
        while i >= max(0, data.count - 65_557) {
            if try u32(i) == 0x0605_4b50, i + 22 + Int(try u16(i + 20)) == data.count { eocd = i; break }
            i -= 1
        }
        guard eocd >= 0 else { throw NotebookError(message: "this is not a zip file") }
        let count = Int(try u16(eocd + 10))
        var p = Int(try u32(eocd + 16))
        guard try u16(eocd + 4) == 0, try u16(eocd + 6) == 0, Int(try u16(eocd + 8)) == count,
              count != 0xFFFF, p != 0xFFFF_FFFF else { throw NotebookError(message: "split and zip64 archives aren't supported") }
        var out: [Entry] = []
        var expanded = 0
        var names = Set<String>()
        for _ in 0..<count {
            guard try u32(p) == 0x0201_4b50 else { throw NotebookError(message: "the zip's directory is damaged") }
            let method = try u16(p + 10)
            let compressed = Int(try u32(p + 20)), size = Int(try u32(p + 24))
            let nameLen = Int(try u16(p + 28)), extraLen = Int(try u16(p + 30)), commentLen = Int(try u16(p + 32))
            let offset = Int(try u32(p + 42))
            guard p + 46 + nameLen + extraLen + commentLen <= eocd else { throw NotebookError(message: "the zip file is truncated") }
            let name = String(decoding: data[(p + 46)..<(p + 46 + nameLen)], as: UTF8.self)
            let parts = name.split(separator: "/", omittingEmptySubsequences: true)
            guard !parts.isEmpty, !parts.contains(".."), !parts.contains("."), !name.hasPrefix("/"),
                  !name.contains("\0"), !name.contains("\\"), names.insert(parts.joined(separator: "/")).inserted else {
                throw NotebookError(message: "the zip contains an unsafe or duplicate path")
            }
            let mode = (try u32(p + 38) >> 16) & 0xF000
            guard mode == 0 || mode == 0x8000 || mode == 0x4000,
                  try u16(p + 8) & 1 == 0, method == 0 || method == 8 else {
                throw NotebookError(message: "the zip contains an unsupported entry")
            }
            expanded += size
            guard size <= Self.maxEntryBytes, expanded <= Self.maxExpandedBytes else {
                throw NotebookError(message: "The archive exceeds the import limit (128 MB per file, 1 GB expanded).")
            }
            out.append(Entry(name: name, method: method, compressed: compressed, size: size, offset: offset, crc: try u32(p + 16)))
            p += 46 + nameLen + extraLen + commentLen
        }
        return out
    }

    /// Inflate through a fixed 64 KiB buffer. Advertised sizes never determine an allocation.
    private func extract(_ e: Entry, to url: URL) throws {
        guard try u32(e.offset) == 0x0403_4b50, try u16(e.offset + 8) == e.method,
              try u16(e.offset + 6) & 1 == 0 else { throw NotebookError(message: "the zip entry \(e.name) is damaged") }
        let start = e.offset + 30 + Int(try u16(e.offset + 26)) + Int(try u16(e.offset + 28))
        guard start + e.compressed <= data.count else { throw NotebookError(message: "the zip file is truncated") }
        guard FileManager.default.createFile(atPath: url.path, contents: nil) else { throw CocoaError(.fileWriteUnknown) }
        let handle = try FileHandle(forWritingTo: url)
        defer { try? handle.close() }
        var written = 0
        var checksum: uLong = 0
        func write(_ chunk: Data) throws {
            written += chunk.count
            guard written <= e.size else { throw NotebookError(message: "the zip entry expands beyond its declared size") }
            checksum = chunk.withUnsafeBytes { crc32(checksum, $0.bindMemory(to: Bytef.self).baseAddress, uInt(chunk.count)) }
            try handle.write(contentsOf: chunk)
        }
        if e.method == 0 {
            var position = start
            while position < start + e.compressed {
                let end = min(position + 65_536, start + e.compressed)
                try write(data.subdata(in: position..<end))
                position = end
            }
        } else {
            var stream = z_stream()
            guard inflateInit2_(&stream, -MAX_WBITS, ZLIB_VERSION, Int32(MemoryLayout<z_stream>.size)) == Z_OK else {
                throw NotebookError(message: "couldn't open the zip inflater")
            }
            defer { inflateEnd(&stream) }
            var buffer = [UInt8](repeating: 0, count: 65_536)
            try data.withUnsafeBytes { raw in
                stream.next_in = UnsafeMutablePointer(mutating: raw.bindMemory(to: Bytef.self).baseAddress!.advanced(by: start))
                stream.avail_in = uInt(e.compressed)
                while true {
                    let status = buffer.withUnsafeMutableBytes { dst -> Int32 in
                        stream.next_out = dst.bindMemory(to: Bytef.self).baseAddress!
                        stream.avail_out = 65_536
                        return inflate(&stream, Z_NO_FLUSH)
                    }
                    let n = 65_536 - Int(stream.avail_out)
                    try write(Data(buffer.prefix(n)))
                    if status == Z_STREAM_END {
                        guard stream.avail_in == 0 else { throw NotebookError(message: "the zip entry contains trailing compressed data") }
                        break
                    }
                    guard status == Z_OK, n > 0 else { throw NotebookError(message: "the zip entry \(e.name) didn't inflate") }
                }
            }
        }
        guard written == e.size, UInt32(checksum) == e.crc else { throw NotebookError(message: "the zip entry \(e.name) failed its size or checksum check") }
    }

    func extract(to dir: URL) throws {
        let entries = try entries()
        let fm = FileManager.default
        for e in entries {
            let url = dir.appendingPathComponent(e.name)
            if e.name.hasSuffix("/") {
                guard e.size == 0 else { throw NotebookError(message: "the zip directory has file data") }
                try fm.createDirectory(at: url, withIntermediateDirectories: true)
            } else {
                try fm.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
                try extract(e, to: url)
            }
        }
    }
}
