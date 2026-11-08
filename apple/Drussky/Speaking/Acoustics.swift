import Accelerate
import Foundation

/// Frame-level features of a 16 kHz signal at 100 frames/s (frame i is centred on i/100 s):
/// loudness, loudness in the vowel band, and cepstral coefficients for alignment.
struct Acoustics: Sendable {
    static let sampleRate = 16_000.0
    static let rate = 100.0
    static let hop = 160
    static let window = 400
    static let fftSize = 512
    static let melBands = 26
    static let coefficients = 12

    var energy: [Float]      // dBFS per frame
    var vowelBand: [Float]   // dB, 300–3000 Hz
    var mfcc: [[Float]]      // c1…c12, mean-normalised over the utterance
    var count: Int { energy.count }

    static func time(_ frame: Int) -> Double { Double(frame) / rate }
    static func frame(_ time: Double) -> Int { Int((time * rate).rounded()) }

    init(energy: [Float], vowelBand: [Float], mfcc: [[Float]]) {
        self.energy = energy
        self.vowelBand = vowelBand
        self.mfcc = mfcc
    }

    init(samples x: [Float]) {
        let n = x.count
        let frames = n / Self.hop + 1
        let half = Self.window / 2
        let padded = [Float](repeating: 0, count: half) + x + [Float](repeating: 0, count: half + Self.fftSize)
        var hann = [Float](repeating: 0, count: Self.window)
        vDSP_hann_window(&hann, vDSP_Length(Self.window), Int32(vDSP_HANN_NORM))
        let log2n = vDSP_Length(9)
        guard let setup = vDSP_create_fftsetup(log2n, FFTRadix(kFFTRadix2)) else {
            self.init(energy: [], vowelBand: [], mfcc: [])
            return
        }
        defer { vDSP_destroy_fftsetup(setup) }
        let bins = Self.fftSize / 2
        let filters = Self.melFilters
        let dct = Self.dctMatrix
        let lowBin = Int(300.0 / Self.sampleRate * Double(Self.fftSize)), highBin = Int(3000.0 / Self.sampleRate * Double(Self.fftSize))

        var energy = [Float](repeating: -120, count: frames)
        var band = [Float](repeating: -120, count: frames)
        var mfcc = [[Float]](repeating: [Float](repeating: 0, count: Self.coefficients), count: frames)
        var frame = [Float](repeating: 0, count: Self.fftSize)
        var real = [Float](repeating: 0, count: bins), imag = [Float](repeating: 0, count: bins)
        var power = [Float](repeating: 0, count: bins + 1)
        var mel = [Float](repeating: 0, count: Self.melBands)
        for i in 0..<frames {
            let start = i * Self.hop
            padded.withUnsafeBufferPointer { p in
                var ms: Float = 0
                vDSP_measqv(p.baseAddress! + start, 1, &ms, vDSP_Length(Self.window))
                energy[i] = 10 * log10(ms + 1e-12)
                vDSP_vmul(p.baseAddress! + start, 1, hann, 1, &frame, 1, vDSP_Length(Self.window))
            }
            for k in Self.window..<Self.fftSize { frame[k] = 0 }
            real.withUnsafeMutableBufferPointer { rp in
                imag.withUnsafeMutableBufferPointer { ip in
                    var split = DSPSplitComplex(realp: rp.baseAddress!, imagp: ip.baseAddress!)
                    frame.withUnsafeBufferPointer { fp in
                        fp.baseAddress!.withMemoryRebound(to: DSPComplex.self, capacity: bins) {
                            vDSP_ctoz($0, 2, &split, 1, vDSP_Length(bins))
                        }
                    }
                    vDSP_fft_zrip(setup, &split, 1, log2n, FFTDirection(FFT_FORWARD))
                    power[0] = rp[0] * rp[0] / 4
                    power[bins] = ip[0] * ip[0] / 4
                    for k in 1..<bins { power[k] = (rp[k] * rp[k] + ip[k] * ip[k]) / 4 }
                }
            }
            var b: Float = 0
            for k in lowBin...highBin { b += power[k] }
            band[i] = 10 * log10(b / Float(Self.fftSize) + 1e-12)
            for m in 0..<Self.melBands {
                var s: Float = 0
                for (k, w) in filters[m] { s += power[k] * w }
                mel[m] = log(s + 1e-10)
            }
            for c in 0..<Self.coefficients {
                var s: Float = 0
                for m in 0..<Self.melBands { s += dct[c][m] * mel[m] }
                mfcc[i][c] = s
            }
        }
        // Mean normalisation removes the microphone and room (cepstral mean subtraction).
        if frames > 0 {
            for c in 0..<Self.coefficients {
                let mean = mfcc.reduce(0) { $0 + $1[c] } / Float(frames)
                for i in 0..<frames { mfcc[i][c] -= mean }
            }
        }
        self.init(energy: energy, vowelBand: band, mfcc: mfcc)
    }

    /// Triangular mel filters as sparse (bin, weight) lists, 60–7600 Hz.
    static let melFilters: [[(Int, Float)]] = {
        func mel(_ f: Double) -> Double { 2595 * log10(1 + f / 700) }
        func hz(_ m: Double) -> Double { 700 * (pow(10, m / 2595) - 1) }
        let lo = mel(60), hi = mel(7600)
        let edges = (0...(melBands + 1)).map { hz(lo + (hi - lo) * Double($0) / Double(melBands + 1)) }
        let binHz = sampleRate / Double(fftSize)
        return (0..<melBands).map { m in
            let a = edges[m], c = edges[m + 1], b = edges[m + 2]
            return (Int(a / binHz)...min(fftSize / 2, Int(b / binHz) + 1)).compactMap { k -> (Int, Float)? in
                let f = Double(k) * binHz
                let w = f < c ? (f - a) / (c - a) : (b - f) / (b - c)
                return w > 0 ? (k, Float(w)) : nil
            }
        }
    }()

    /// DCT-II rows 1…12 (c0, overall loudness, is left out).
    static let dctMatrix: [[Float]] = (1...coefficients).map { c in
        (0..<melBands).map { m in Float(cos(Double.pi * Double(c) * (Double(m) + 0.5) / Double(melBands))) }
    }

    /// Frames holding the utterance: loud relative to its own peak and to its quietest stretch.
    func speechBounds() -> Range<Int>? {
        guard let peak = energy.max(), peak > -80 else { return nil }
        let sorted = energy.sorted()
        let quiet = sorted[sorted.count / 10]
        let threshold = max(peak - 35, quiet + 8)
        guard let a = energy.firstIndex(where: { $0 > threshold }), let b = energy.lastIndex(where: { $0 > threshold }), b > a else { return nil }
        return a..<(b + 1)
    }
}

/// Dynamic time warping between two feature sequences (Sakoe–Chiba band, symmetric steps with
/// the diagonal weighted twice), ends anchored.
enum DTW {
    struct Alignment: Sendable {
        var path: [(Int, Int)]
        /// Mean local distance along the path.
        var cost: Float

        /// The frame of `b` matching frame `i` of `a` (the mean of its partners on the path).
        func map(_ i: Int) -> Double {
            guard let first = path.first, let last = path.last else { return Double(i) }
            if i <= first.0 { return Double(first.1) }
            if i >= last.0 { return Double(last.1) }
            var lo = 0, hi = path.count - 1
            while lo < hi { let mid = (lo + hi) / 2; if path[mid].0 < i { lo = mid + 1 } else { hi = mid } }
            var sum = 0, n = 0, k = lo
            while k < path.count && path[k].0 == i { sum += path[k].1; n += 1; k += 1 }
            return n > 0 ? Double(sum) / Double(n) : Double(path[lo].1)
        }
    }

    static func align(_ a: [[Float]], _ b: [[Float]], band: Double = 0.5) -> Alignment? {
        let n = a.count, m = b.count
        guard n > 1, m > 1 else { return nil }
        let width = max(abs(n - m) + 8, Int(band * Double(max(n, m))))
        let inf = Float.greatestFiniteMagnitude / 4
        var cost = [Float](repeating: inf, count: n * m)
        func d(_ i: Int, _ j: Int) -> Float {
            var s: Float = 0
            a[i].withUnsafeBufferPointer { p in b[j].withUnsafeBufferPointer { q in
                vDSP_distancesq(p.baseAddress!, 1, q.baseAddress!, 1, &s, vDSP_Length(p.count))
            } }
            return s.squareRoot()
        }
        func inBand(_ i: Int, _ j: Int) -> Bool { abs(j - Int(Double(i) * Double(m - 1) / Double(n - 1))) <= width }
        cost[0] = d(0, 0)
        for i in 0..<n {
            for j in 0..<m where (i > 0 || j > 0) && inBand(i, j) {
                let local = d(i, j)
                var best = inf
                if i > 0 && j > 0 { best = cost[(i - 1) * m + j - 1] + 2 * local }
                if i > 0 { best = min(best, cost[(i - 1) * m + j] + local) }
                if j > 0 { best = min(best, cost[i * m + j - 1] + local) }
                cost[i * m + j] = best
            }
        }
        guard cost[n * m - 1] < inf else { return nil }
        var path: [(Int, Int)] = [(n - 1, m - 1)]
        var i = n - 1, j = m - 1
        while i > 0 || j > 0 {
            if i == 0 { j -= 1 } else if j == 0 { i -= 1 } else {
                let diag = cost[(i - 1) * m + j - 1], up = cost[(i - 1) * m + j], left = cost[i * m + j - 1]
                if diag <= up && diag <= left { i -= 1; j -= 1 } else if up <= left { i -= 1 } else { j -= 1 }
            }
            path.append((i, j))
        }
        path.reverse()
        return Alignment(path: path, cost: cost[n * m - 1] / Float(n + m))
    }
}
