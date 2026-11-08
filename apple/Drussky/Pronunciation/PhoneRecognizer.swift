import CoreML
import Foundation

/// The on-device phone recognizer: ZIPA small CR-CTC (64 M parameters, multilingual IPA,
/// Zhu et al. 2025) converted to Core ML by apple/scripts/models.py. Not in git; when the
/// model folder is absent `PhoneRecognizer.bundled` is nil and the analyzer falls back to
/// word-level comparison.
actor PhoneRecognizer {
    struct Meta: Decodable, Sendable {
        var tokens: [String]
        var window_frames: Int
        var blank: Int
    }

    static let modelName = "ZipaPhones"
    let modelURL: URL
    let meta: Meta
    /// Token id per Unicode scalar the recognizer writes.
    nonisolated let tokenIDs: [Unicode.Scalar: Int]
    private var model: MLModel?

    init(directory: URL) throws {
        modelURL = directory.appendingPathComponent("\(Self.modelName).mlmodelc")
        let data = try Data(contentsOf: directory.appendingPathComponent("\(Self.modelName).json"))
        meta = try JSONDecoder().decode(Meta.self, from: data)
        var ids: [Unicode.Scalar: Int] = [:]
        for (i, t) in meta.tokens.enumerated() where t.unicodeScalars.count == 1 { ids[t.unicodeScalars.first!] = i }
        tokenIDs = ids
        guard FileManager.default.fileExists(atPath: modelURL.path) else { throw CocoaError(.fileNoSuchFile) }
    }

    /// The model bundled with the app (Resources/Models), if `models.py fetch` put it there.
    static let bundled: PhoneRecognizer? = {
        for base in [Bundle.main.resourceURL, Bundle(for: BundleToken.self).resourceURL].compactMap({ $0 }) {
            let dir = base.appendingPathComponent("Models")
            if let r = try? PhoneRecognizer(directory: dir) { return r }
        }
        return nil
    }()
    private final class BundleToken {}

    /// Token ids for a canonical IPA spelling (nil if the recognizer cannot write it).
    nonisolated func tokens(_ ipa: String) -> [Int]? {
        var out: [Int] = []
        for s in ipa.unicodeScalars {
            let k: Unicode.Scalar = s == "ɡ" ? "g" : s
            guard let id = tokenIDs[k] else { return nil }
            out.append(id)
        }
        return out
    }

    nonisolated func token(_ id: Int) -> String { id < meta.tokens.count ? meta.tokens[id] : "" }

    func load() throws -> MLModel {
        if let model { return model }
        let config = MLModelConfiguration()
        // CPU only. The Neural Engine's float16 overflows inside this model (27% of decoded
        // symbols change; ZIPA's own fp16 export needed fixes too), and the GPU path once
        // aborted the process compiling its MPS graph. On the CPU it agrees with the float32
        // ONNX reference to within 1% of symbols and runs a 10 s window in ~50 ms on an M2 Max.
        config.computeUnits = .cpuOnly
        let m = try MLModel(contentsOf: modelURL, configuration: config)
        model = m
        return m
    }

    /// Posteriors for 16 kHz mono audio, in windows of `window_frames` fbank frames.
    func posteriors(_ samples: [Float]) throws -> Posteriors {
        let m = try load()
        let feats = Fbank.shared.compute(samples)
        let bins = Fbank.bins
        let total = feats.count / bins
        let W = meta.window_frames
        var logp: [Float] = []
        var frames = 0
        var vocab = meta.tokens.count
        var start = 0
        while start < total {
            let n = min(W, total - start)
            if n < 9 { break }
            let x = try MLMultiArray(shape: [1, NSNumber(value: W), NSNumber(value: bins)], dataType: .float32)
            let xp = x.dataPointer.bindMemory(to: Float.self, capacity: W * bins)
            feats.withUnsafeBufferPointer { f in
                xp.update(from: f.baseAddress! + start * bins, count: n * bins)
            }
            (xp + n * bins).update(repeating: 0, count: (W - n) * bins)
            let len = try MLMultiArray(shape: [1], dataType: .int32)
            len[0] = NSNumber(value: n)
            let out = try m.prediction(from: MLDictionaryFeatureProvider(dictionary: ["features": x, "length": len]))
            guard let lp = out.featureValue(for: "log_probs")?.multiArrayValue else { throw CocoaError(.coderValueNotFound) }
            vocab = lp.shape[2].intValue
            let valid = min((n - 7) / 2, lp.shape[1].intValue)
            let strides = lp.strides.map(\.intValue)
            if lp.dataType == .float32 {
                let p = lp.dataPointer.bindMemory(to: Float.self, capacity: lp.count)
                for t in 0..<valid { for k in 0..<vocab { logp.append(p[t * strides[1] + k * strides[2]]) } }
            } else {
                for t in 0..<valid { for k in 0..<vocab { logp.append(lp[[0, NSNumber(value: t), NSNumber(value: k)]].floatValue) } }
            }
            frames += valid
            // Windows after the first start where the previous one's output frames end.
            start += valid * 2
        }
        return Posteriors(logp: logp, frames: frames, vocab: vocab, frameSeconds: 0.02, offset: 0.035)
    }
}
