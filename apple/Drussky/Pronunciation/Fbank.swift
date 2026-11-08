import Accelerate
import Foundation

/// Kaldi-style log mel filterbank exactly as lhotse computes it for ZIPA's training data:
/// 25 ms frames every 10 ms (frames = (n + 80) / 160, mirrored edges), DC removed,
/// pre-emphasis 0.97, Povey window, 512-point power spectrum, 80 triangular mel bins from
/// 20 Hz to 7600 Hz, natural log floored at float epsilon. Input is 16 kHz audio in [-1, 1].
struct Fbank: Sendable {
    static let sampleRate = 16_000
    static let frameLength = 400
    static let frameShift = 160
    static let fftLength = 512
    static let bins = 80

    let window: [Float]
    /// bins × (fftLength/2) weights, row-major; the Nyquist bin has no weight.
    let mel: [Float]

    static let shared = Fbank()

    init() {
        let L = Self.frameLength
        window = (0..<L).map { i in Float(pow(0.5 - 0.5 * cos(2 * Double.pi * Double(i) / Double(L - 1)), 0.85)) }
        let half = Self.fftLength / 2
        func lin2mel(_ f: Double) -> Double { 1127 * log(1 + f / 700) }
        let lo = lin2mel(20), hi = lin2mel(Double(Self.sampleRate) / 2 - 400)
        let delta = (hi - lo) / Double(Self.bins + 1)
        let width = Double(Self.sampleRate) / Double(Self.fftLength)
        var m = [Float](repeating: 0, count: Self.bins * half)
        for b in 0..<Self.bins {
            let left = lo + Double(b) * delta, center = left + delta, right = center + delta
            for k in 0..<half {
                let x = lin2mel(width * Double(k))
                let w = max(0, min((x - left) / (center - left), (right - x) / (right - center)))
                m[b * half + k] = Float(w)
            }
        }
        mel = m
    }

    static func frameCount(samples n: Int) -> Int { (n + frameShift / 2) / frameShift }

    /// frames × 80, row-major.
    func compute(_ x: [Float]) -> [Float] {
        let n = x.count
        let frames = Self.frameCount(samples: n)
        guard frames > 0, n > Self.frameLength else { return [] }
        let L = Self.frameLength, shift = Self.frameShift, N = Self.fftLength, half = N / 2
        let padLeft = (L - shift) / 2
        let padded: [Float] = {
            let newCount = (frames - 1) * shift + L
            let padRight = newCount - n - padLeft
            var p = [Float](x[0..<padLeft].reversed())
            p.reserveCapacity(newCount)
            p += x
            if padRight > 0 { p += x[(n - padRight)..<n].reversed() }
            else if padRight < 0 { p.removeLast(-padRight) }
            return p
        }()
        guard let dft = vDSP_DFT_zrop_CreateSetup(nil, vDSP_Length(N), .FORWARD) else { return [] }
        defer { vDSP_DFT_DestroySetup(dft) }
        var out = [Float](repeating: 0, count: frames * Self.bins)
        var frame = [Float](repeating: 0, count: N)
        var evens = [Float](repeating: 0, count: half), odds = [Float](repeating: 0, count: half)
        var re = [Float](repeating: 0, count: half), im = [Float](repeating: 0, count: half)
        var power = [Float](repeating: 0, count: half)
        var energies = [Float](repeating: 0, count: Self.bins)
        let eps = Float.ulpOfOne
        for f in 0..<frames {
            let start = f * shift
            for i in 0..<L { frame[i] = padded[start + i] }
            var mean: Float = 0
            vDSP_meanv(frame, 1, &mean, vDSP_Length(L))
            for i in 0..<L { frame[i] -= mean }
            for i in stride(from: L - 1, through: 1, by: -1) { frame[i] -= 0.97 * frame[i - 1] }
            frame[0] -= 0.97 * frame[0]
            for i in 0..<L { frame[i] *= window[i] }
            for i in L..<N { frame[i] = 0 }
            for i in 0..<half { evens[i] = frame[2 * i]; odds[i] = frame[2 * i + 1] }
            vDSP_DFT_Execute(dft, evens, odds, &re, &im)
            // zrop packs DC in re[0] and Nyquist in im[0], and returns twice the true values.
            power[0] = re[0] * re[0] * 0.25
            for k in 1..<half { power[k] = (re[k] * re[k] + im[k] * im[k]) * 0.25 }
            mel.withUnsafeBufferPointer { m in
                vDSP_mmul(m.baseAddress!, 1, power, 1, &energies, 1, vDSP_Length(Self.bins), 1, vDSP_Length(half))
            }
            for b in 0..<Self.bins { out[f * Self.bins + b] = log(max(energies[b], eps)) }
        }
        return out
    }
}
