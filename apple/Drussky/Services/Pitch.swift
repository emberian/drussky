import Accelerate
import AVFoundation

/// Fundamental frequency (F0) over time with YIN (de Cheveigné & Kawahara 2002), 40 display
/// bins a second (`track`) or 100 analysis frames a second (`analyse`, as `Acoustics` uses).
enum Pitch {
    static let sampleRate = 16_000.0
    static let framesPerSecond = 40.0
    static let minHz = 65.0
    static let maxHz = 500.0

    /// Analysis runs at 100 frames/s (fine enough for short voiced stretches), then each
    /// display bin takes the mean of its voiced frames.
    static let analysisRate = 100.0

    /// Hz per display bin (nil where unvoiced or too quiet), hop 1/`framesPerSecond` s.
    static func track(_ x: [Float], sampleRate sr: Double = sampleRate) -> [Float?] {
        let bin = max(1, Int(sr / framesPerSecond))
        let hop = max(1, Int(sr / analysisRate))
        let fine = clean(analyse(x, sampleRate: sr, hop: hop))
        return (0..<(x.count / bin)).map { b in
            let lo = (b * bin + hop / 2) / hop, hi = min(fine.count, ((b + 1) * bin + hop / 2) / hop)
            let v = lo < hi ? fine[lo..<hi].compactMap { $0 } : []
            return v.isEmpty ? nil : v.reduce(0, +) / Float(v.count)
        }
    }

    /// YIN per frame at `hop`: Hz, or nil where unvoiced or too quiet.
    static func analyse(_ x: [Float], sampleRate sr: Double, hop: Int, threshold: Float = 0.2) -> [Float?] {
        let maxLag = Int(sr / minHz), minLag = max(2, Int(sr / maxHz))
        let window = maxLag + 16
        let span = window + maxLag
        guard x.count >= hop else { return [] }
        let frames = x.count / hop
        let padded = [Float](repeating: 0, count: window / 2) + x + [Float](repeating: 0, count: span)

        // Loudness per frame decides which frames are worth asking about.
        var rms = [Float](repeating: 0, count: frames)
        for i in 0..<frames {
            padded.withUnsafeBufferPointer { p in
                vDSP_rmsqv(p.baseAddress! + window / 2 + i * hop, 1, &rms[i], vDSP_Length(hop))
            }
        }
        let loudest = rms.max() ?? 0
        guard loudest > 1e-4 else { return Array(repeating: nil, count: frames) }

        var out = [Float?](repeating: nil, count: frames)
        var corr = [Float](repeating: 0, count: maxLag + 1)
        var squares = [Float](repeating: 0, count: span)
        var prefix = [Float](repeating: 0, count: span + 1)
        var d = [Float](repeating: 0, count: maxLag + 1)
        for i in 0..<frames where rms[i] >= loudest * 0.06 {
            let start = i * hop
            padded.withUnsafeBufferPointer { p in
                let s = p.baseAddress! + start
                // C(τ) = Σ x[j] x[j+τ] for τ in 0...maxLag, as one correlation.
                vDSP_conv(s, 1, s, 1, &corr, 1, vDSP_Length(maxLag + 1), vDSP_Length(window))
                vDSP_vsq(s, 1, &squares, 1, vDSP_Length(span))
            }
            for j in 0..<span { prefix[j + 1] = prefix[j] + squares[j] }
            let e0 = prefix[window]
            // Difference d(τ) = E(0) + E(τ) − 2C(τ); cumulative-mean normalised.
            var running: Float = 0
            d[0] = 1
            for tau in 1...maxLag {
                let et = prefix[tau + window] - prefix[tau]
                let diff = max(0, e0 + et - 2 * corr[tau])
                running += diff
                d[tau] = running > 0 ? diff * Float(tau) / running : 1
            }
            var tau = minLag
            var found = -1
            while tau <= maxLag {
                if d[tau] < threshold {
                    while tau + 1 <= maxLag && d[tau + 1] < d[tau] { tau += 1 }
                    found = tau
                    break
                }
                tau += 1
            }
            if found < 0, let best = d[minLag...maxLag].indices.min(by: { d[$0] < d[$1] }), d[best] < 0.3 { found = best }
            guard found > 0 else { continue }
            // Parabolic interpolation around the dip.
            var t = Float(found)
            if found > 1 && found < maxLag {
                let a = d[found - 1], b = d[found], c = d[found + 1]
                let den = a - 2 * b + c
                if abs(den) > 1e-9 { t += 0.5 * (a - c) / den }
            }
            out[i] = Float(sr) / t
        }
        return out
    }

    /// Removes octave slips and specks: frames far from the median are folded back by an
    /// octave or dropped, a 5-frame median smooths each voiced run, runs under 4 frames go.
    static func clean(_ raw: [Float?]) -> [Float?] {
        let voiced = raw.compactMap { $0 }.sorted()
        guard !voiced.isEmpty else { return raw }
        let median = voiced[voiced.count / 2]
        var f = raw.map { v -> Float? in
            guard var v else { return nil }
            if v > median * 1.7 { v /= 2 } else if v < median * 0.6 { v *= 2 }
            return v > median * 1.7 || v < median * 0.6 ? nil : v
        }
        var smoothed = f
        for i in f.indices where f[i] != nil {
            let near = (max(0, i - 2)...min(f.count - 1, i + 2)).compactMap { f[$0] }.sorted()
            smoothed[i] = near[near.count / 2]
        }
        f = smoothed
        var i = 0
        while i < f.count {
            guard f[i] != nil else { i += 1; continue }
            var j = i
            while j < f.count && f[j] != nil { j += 1 }
            if j - i < 4 { for k in i..<j { f[k] = nil } }
            i = j
        }
        return f
    }

    /// Mono Float samples at `sampleRate`, from any audio file AVFoundation reads.
    static func samples(_ url: URL) -> [Float]? {
        guard let file = try? AVAudioFile(forReading: url),
              let input = AVAudioPCMBuffer(pcmFormat: file.processingFormat, frameCapacity: AVAudioFrameCount(file.length)),
              (try? file.read(into: input)) != nil,
              let format = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: sampleRate, channels: 1, interleaved: false),
              let converter = AVAudioConverter(from: file.processingFormat, to: format) else { return nil }
        let capacity = AVAudioFrameCount(Double(input.frameLength) * sampleRate / file.processingFormat.sampleRate) + 1024
        guard let output = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: capacity) else { return nil }
        var fed = false
        var error: NSError?
        converter.convert(to: output, error: &error) { _, status in
            if fed { status.pointee = .endOfStream; return nil }
            fed = true
            status.pointee = .haveData
            return input
        }
        guard error == nil, let data = output.floatChannelData?[0] else { return nil }
        return Array(UnsafeBufferPointer(start: data, count: Int(output.frameLength)))
    }
}

/// A speaker's melody: semitones relative to their own median F0, so a low and a high voice
/// draw on the same scale. Trimmed to where loudness is above 8% of the peak.
struct Contour: Sendable, Equatable {
    var semitones: [Float?]
    var medianHz: Float

    static func compute(_ url: URL) async -> Contour? {
        await Task.detached(priority: .userInitiated) { () -> Contour? in
            guard let x = Pitch.samples(url) else { return nil }
            return from(samples: x)
        }.value
    }

    static func from(samples x: [Float], sampleRate: Double = Pitch.sampleRate) -> Contour? {
        let hz = Pitch.track(x, sampleRate: sampleRate)
        let hop = max(1, Int(sampleRate / Pitch.framesPerSecond))
        var bins = [Float](repeating: 0, count: hz.count)
        x.withUnsafeBufferPointer { p in
            for i in bins.indices { vDSP_rmsqv(p.baseAddress! + i * hop, 1, &bins[i], vDSP_Length(hop)) }
        }
        guard let peak = bins.max(), peak > 0,
              let first = bins.firstIndex(where: { $0 / peak > 0.08 }),
              let last = bins.lastIndex(where: { $0 / peak > 0.08 }) else { return nil }
        let kept = Array(hz[first...last])
        let voiced = kept.compactMap { $0 }.sorted()
        guard voiced.count >= 3 else { return nil }
        let median = voiced[voiced.count / 2]
        return Contour(semitones: kept.map { $0.map { 12 * log2($0 / median) } }, medianHz: median)
    }

    /// The biggest climb from a low point to a later high, in semitones. A Russian yes/no
    /// question (ИК-3) jumps up on the stressed syllable of the asked-about word, usually 5+;
    /// a statement (ИК-1) stays within a few semitones and falls.
    var rise: Float {
        var low = Float.infinity, best: Float = 0
        for v in semitones.compactMap({ $0 }) {
            low = min(low, v)
            best = max(best, v - low)
        }
        return best
    }

    /// Mean semitones over the last quarter of the voiced frames minus the mean of the rest:
    /// positive when the melody ends up (a question), negative when it ends down.
    var finalMovement: Float {
        let v = semitones.compactMap { $0 }
        guard v.count >= 8 else { return 0 }
        let cut = v.count * 3 / 4
        let head = v[..<cut], tail = v[cut...]
        return tail.reduce(0, +) / Float(tail.count) - head.reduce(0, +) / Float(head.count)
    }
}
