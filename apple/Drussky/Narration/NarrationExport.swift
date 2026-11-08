import AVFoundation
import Foundation

extension Narration {
    /// The stretch of a narration an exercise uses, as its own file (for the rhythm strip, which
    /// compares the learner's take with a model clip). Cut once into Caches/narration-chunks/.
    static func chunkURL(for ex: Exercise) async -> URL? {
        guard let i = interval(of: ex), let src = url(i.file) else { return nil }
        if i.start == 0 && i.end == .infinity { return src }
        let dir = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0].appendingPathComponent("narration-chunks", isDirectory: true)
        let name = "\((i.file as NSString).deletingPathExtension)-\(Int(i.start * 1000))-\(Int(i.end * 1000)).m4a"
        let out = dir.appendingPathComponent(name)
        if FileManager.default.fileExists(atPath: out.path) { return out }
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let asset = AVURLAsset(url: src)
        guard let session = AVAssetExportSession(asset: asset, presetName: AVAssetExportPresetAppleM4A) else { return nil }
        let start = CMTime(seconds: i.start, preferredTimescale: 1000)
        let end = CMTime(seconds: i.end, preferredTimescale: 1000)
        session.timeRange = CMTimeRange(start: start, end: end)
        do {
            try await session.export(to: out, as: .m4a)
            return out
        } catch {
            return nil
        }
    }
}
