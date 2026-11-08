import Foundation

/// FSRS-5 (Free Spaced Repetition Scheduler; Ye et al., open-spaced-repetition/fsrs4anki wiki,
/// "The Algorithm", FSRS-5 section), with its published default parameters. Time is in days.
///
///   R(t, S)    = (1 + F·t/S)^C,  C = −0.5, F = 19/81             (so R(S, S) = 0.9)
///   S₀(G)      = w[G−1]
///   D₀(G)      = w4 − e^(w5·(G−1)) + 1
///   D′(D, G)   = w7·D₀(4) + (1−w7)·(D + ΔD·(10−D)/9),  ΔD = −w6·(G−3)    (clamped to [1, 10])
///   S′ᵣ (G>1)  = S·(1 + e^w8·(11−D)·S^(−w9)·(e^(w10·(1−R)) − 1)·h·b),  h = w15 if Hard, b = w16 if Easy
///   S′f (G=1)  = min(w11·D^(−w12)·((S+1)^w13 − 1)·e^(w14·(1−R)),  S / e^(w17·w18))
///   same day   S′ = S·e^(w17·(G − 3 + w18))
///   interval   I(r) = S/F·(r^(1/C) − 1)        (I(0.9) = S)
public struct FSRS: Codable, Sendable, Hashable {
    public var w: [Double]
    public static let decay = -0.5
    public static let factor = 19.0 / 81.0

    public static let defaultWeights: [Double] = [
        0.40255, 1.18385, 3.173, 15.69105, 7.1949, 0.5345, 1.4604, 0.0046, 1.54575, 0.1192,
        1.01925, 1.9395, 0.11, 0.29605, 2.2698, 0.2315, 2.9898, 0.51655, 0.6621,
    ]

    public init(w: [Double] = FSRS.defaultWeights) {
        precondition(w.count == 19, "FSRS-5 has 19 parameters")
        self.w = w
    }

    public init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let w = try c.decode([Double].self, forKey: .w)
        guard w.count == 19 else {
            throw DecodingError.dataCorruptedError(forKey: .w, in: c, debugDescription: "FSRS-5 has 19 parameters")
        }
        self.w = w
    }

    public struct State: Codable, Sendable, Hashable {
        public var stability: Double
        public var difficulty: Double
        public init(stability: Double, difficulty: Double) {
            self.stability = stability
            self.difficulty = difficulty
        }
    }

    public static func retrievability(elapsedDays t: Double, stability s: Double) -> Double {
        pow(1 + factor * max(0, t) / max(s, 0.01), decay)
    }

    public static func interval(stability s: Double, desiredRetention r: Double = 0.9) -> Double {
        s / factor * (pow(r, 1 / decay) - 1)
    }

    public func initialStability(_ g: Grade) -> Double { max(0.01, w[g.rawValue - 1]) }

    public func initialDifficulty(_ g: Grade) -> Double {
        clampD(w[4] - exp(w[5] * Double(g.rawValue - 1)) + 1)
    }

    public func nextDifficulty(_ d: Double, _ g: Grade) -> Double {
        let delta = -w[6] * Double(g.rawValue - 3)
        let damped = d + delta * (10 - d) / 9
        return clampD(w[7] * initialDifficulty(.easy) + (1 - w[7]) * damped)
    }

    public func recallStability(_ s: Double, _ d: Double, _ r: Double, _ g: Grade) -> Double {
        let hard = g == .hard ? w[15] : 1
        let easy = g == .easy ? w[16] : 1
        return s * (1 + exp(w[8]) * (11 - d) * pow(s, -w[9]) * (exp(w[10] * (1 - r)) - 1) * hard * easy)
    }

    public func forgetStability(_ s: Double, _ d: Double, _ r: Double) -> Double {
        let long = w[11] * pow(d, -w[12]) * (pow(s + 1, w[13]) - 1) * exp(w[14] * (1 - r))
        return min(long, s / exp(w[17] * w[18]))
    }

    public func shortTermStability(_ s: Double, _ g: Grade) -> Double {
        s * exp(w[17] * (Double(g.rawValue) - 3 + w[18]))
    }

    /// The state after a review `elapsedDays` after the previous one (nil state = first review).
    public func next(_ state: State?, elapsedDays t: Double, grade g: Grade) -> State {
        guard let state else { return State(stability: initialStability(g), difficulty: initialDifficulty(g)) }
        let s: Double
        if t < 1 {
            s = shortTermStability(state.stability, g)
        } else {
            let r = Self.retrievability(elapsedDays: t, stability: state.stability)
            s = g == .again ? forgetStability(state.stability, state.difficulty, r)
                            : recallStability(state.stability, state.difficulty, r, g)
        }
        return State(stability: min(36_500, max(0.01, s)), difficulty: nextDifficulty(state.difficulty, g))
    }

    private func clampD(_ d: Double) -> Double { min(10, max(1, d)) }
}
