import Foundation

/// CTC log posteriors from the recognizer: `frames` × `vocab`, row-major, blank = 0.
struct Posteriors: Sendable {
    var logp: [Float]
    var frames: Int
    var vocab: Int
    /// Seconds per output frame, and the time of frame 0's centre within the analysed audio.
    var frameSeconds: Double = 0.02
    var offset: Double = 0.03

    subscript(t: Int, k: Int) -> Float { logp[t * vocab + k] }
    func time(_ frame: Int) -> Double { offset + Double(frame) * frameSeconds }
}

enum CTC {
    static let negInf = -Float.infinity

    @inline(__always) static func lse(_ a: Float, _ b: Float) -> Float {
        if a == negInf { return b }
        if b == negInf { return a }
        let m = max(a, b)
        return m + log(exp(a - m) + exp(b - m))
    }

    /// log P(labels | x) by the CTC forward algorithm over frames lo..<hi.
    static func logLikelihood(_ p: Posteriors, _ labels: [Int], frames: Range<Int>? = nil) -> Float {
        let range = frames ?? 0..<p.frames
        let L = labels.count
        if L == 0 {
            var s: Float = 0
            for t in range { s += p[t, 0] }
            return s
        }
        let S = 2 * L + 1
        var ext = [Int](repeating: 0, count: S)
        for i in 0..<L { ext[2 * i + 1] = labels[i] }
        var alpha = [Float](repeating: negInf, count: S)
        var next = alpha
        guard let t0 = range.first else { return negInf }
        alpha[0] = p[t0, 0]
        alpha[1] = p[t0, ext[1]]
        for t in range.dropFirst() {
            for s in 0..<S {
                var a = alpha[s]
                if s >= 1 { a = lse(a, alpha[s - 1]) }
                if s >= 2 && ext[s] != 0 && ext[s] != ext[s - 2] { a = lse(a, alpha[s - 2]) }
                next[s] = a == negInf ? negInf : a + p[t, ext[s]]
            }
            swap(&alpha, &next)
        }
        return lse(alpha[S - 1], alpha[S - 2])
    }

    /// Best path (Viterbi) through `labels`: for each label, the frames it was emitted on.
    static func align(_ p: Posteriors, _ labels: [Int]) -> [ClosedRange<Int>?] {
        let L = labels.count, T = p.frames
        guard L > 0, T > 0 else { return Array(repeating: nil, count: L) }
        let S = 2 * L + 1
        var ext = [Int](repeating: 0, count: S)
        for i in 0..<L { ext[2 * i + 1] = labels[i] }
        var score = [Float](repeating: negInf, count: S)
        var back = [[Int8]](repeating: [Int8](repeating: 0, count: S), count: T)
        score[0] = p[0, 0]
        score[1] = p[0, ext[1]]
        var next = score
        for t in 1..<T {
            for s in 0..<S {
                var best = score[s], from: Int8 = 0
                if s >= 1, score[s - 1] > best { best = score[s - 1]; from = 1 }
                if s >= 2, ext[s] != 0, ext[s] != ext[s - 2], score[s - 2] > best { best = score[s - 2]; from = 2 }
                next[s] = best == negInf ? negInf : best + p[t, ext[s]]
                back[t][s] = from
            }
            swap(&score, &next)
        }
        var s = score[S - 1] >= score[S - 2] ? S - 1 : S - 2
        var spans = [ClosedRange<Int>?](repeating: nil, count: L)
        for t in stride(from: T - 1, through: 0, by: -1) {
            if s % 2 == 1 {
                let i = s / 2
                spans[i] = spans[i].map { t...$0.upperBound } ?? t...t
            }
            if t > 0 { s -= Int(back[t][s]) }
        }
        return spans
    }

    /// Greedy decode: (token, first frame, last frame, peak posterior) per emitted token.
    static func greedy(_ p: Posteriors) -> [(token: Int, frames: ClosedRange<Int>, peak: Float)] {
        var out: [(Int, ClosedRange<Int>, Float)] = []
        var prev = 0
        for t in 0..<p.frames {
            var best = 0
            var bv = p[t, 0]
            for k in 1..<p.vocab where p[t, k] > bv { bv = p[t, k]; best = k }
            if best != 0 {
                if best == prev, let last = out.last {
                    out[out.count - 1] = (best, last.1.lowerBound...t, max(last.2, exp(bv)))
                } else {
                    out.append((best, t...t, exp(bv)))
                }
            }
            prev = best
        }
        return out.map { (token: $0.0, frames: $0.1, peak: $0.2) }
    }
}
