import Accelerate
import Foundation

/// Hands-free endpointing for one take: measures the room, starts the take when a voice starts,
/// ends it after a stretch of trailing silence, and says exactly which samples to keep.
///
/// Works in 10 ms frames on mono samples at any rate. A frame is *active* when its energy is
/// well above the measured floor; speech is confirmed only once active frames also turn out
/// *voiced* (periodic, by normalised autocorrelation of the pre-emphasised signal), so a door,
/// a cough of breath or a fan switching on never starts a take, while a leading с or ш (loud but
/// unvoiced) is still kept because the start is backdated to the first active frame.
/// Pure value: feed it samples in any chunking and it gives the same answer.
struct VoiceActivity: Sendable {
    struct Config: Sendable {
        /// Room measurement before listening (the floor is a low percentile of these frames).
        var calibration = 0.25
        /// A take ends after this much continuous quiet once speech has begun.
        var trailingSilence = 0.7
        /// Longest speech kept (from onset).
        var maxSpeech = 6.0
        /// Give up when no voice starts within this long after listening begins.
        var waitLimit = 8.0
        /// Onset: this far above the floor (dB); speech stays on while above floor + `hold`.
        var onsetAbove: Float = 10
        var hold: Float = 6
        /// Absolute floor for thresholds (dBFS): a perfectly silent input still needs real sound.
        var minimumOnset: Float = -62
        /// Voiced frames needed to confirm a voice, within `confirmWithin` of the first active frame.
        var voicedToConfirm = 3
        var confirmWithin = 0.45
        /// Normalised autocorrelation peak above which a frame counts as voiced.
        var voicing: Float = 0.5
        /// Kept around the speech: a little room before and after, so nothing sounds clipped.
        var padBefore = 0.12
        var padAfter = 0.18
    }

    enum Reason: String, Sendable, Codable { case silence, maxLength, tapped, noSpeech }

    /// How a take ended and which samples hold it. `speech` is where the voice is; `keep` adds
    /// the padding. Both are sample ranges into everything fed since the detector was made.
    struct Ending: Sendable, Equatable {
        var reason: Reason
        var speech: Range<Int>?
        var keep: Range<Int>?
    }

    enum Event: Sendable, Equatable {
        /// The room is measured; listening for a voice. Floor in dBFS.
        case listening(floor: Float)
        /// A voice began, at this sample (backdated to the first active frame).
        case onset(Int)
        case ended(Ending)
    }

    let config: Config
    let sampleRate: Double
    private let hop: Int
    private let decimation: Int
    // Rolling decimated, pre-emphasised signal for the voicing test (last `window` samples).
    private let window: Int
    private let minLag: Int
    private let maxLag: Int
    private var history: [Float]
    private var lastRaw: Float = 0
    private var pending: [Float] = []

    private enum State: Sendable, Equatable { case calibrating, waiting, candidate, speech, done }
    private var state: State = .calibrating
    private(set) var frameEnergy: [Float] = []
    private(set) var floor: Float = -90
    private var calibrationFrames: Int
    private var candidateStart = 0
    private var candidateVoiced = 0
    private var candidateQuiet = 0
    private var onsetFrame = 0
    private var lastActive = 0
    private var listeningFrom = 0

    /// Samples fed so far.
    private(set) var count = 0
    /// Latest frame's energy relative to the floor, 0…1 over 40 dB (for a level meter).
    private(set) var level: Float = 0
    var hasVoice: Bool { state == .speech || (state == .done && onsetSeen) }
    private var onsetSeen = false

    init(sampleRate: Double, config: Config = Config()) {
        self.config = config
        self.sampleRate = sampleRate
        hop = max(1, Int((sampleRate / 100).rounded()))
        decimation = max(1, Int(sampleRate / 16_000))
        let low = sampleRate / Double(max(1, Int(sampleRate / 16_000)))
        window = Int(low * 0.03)
        minLag = max(2, Int(low / 400))
        maxLag = Int(low / 70)
        history = []
        calibrationFrames = max(5, Int(config.calibration * 100))
    }

    private var frameSeconds: Double { Double(hop) / sampleRate }
    private func frames(_ seconds: Double) -> Int { Int((seconds / frameSeconds).rounded()) }

    /// Feeds samples; returns what happened, in order.
    mutating func feed(_ samples: UnsafeBufferPointer<Float>) -> [Event] {
        guard state != .done else { return [] }
        var events: [Event] = []
        var i = 0
        while i < samples.count {
            let need = hop - pending.count
            let take = min(need, samples.count - i)
            pending.append(contentsOf: samples[i..<(i + take)])
            i += take
            if pending.count == hop {
                if let e = frame(pending) { events.append(contentsOf: e) }
                pending.removeAll(keepingCapacity: true)
                if state == .done { break }
            }
        }
        count += samples.count
        return events
    }

    mutating func feed(_ samples: [Float]) -> [Event] { samples.withUnsafeBufferPointer { feed($0) } }

    /// The learner tapped to stop: end now with whatever voice there is.
    mutating func finishNow() -> Ending {
        guard state != .done else { return Ending(reason: .tapped, speech: nil, keep: nil) }
        let n = frameEnergy.count
        switch state {
        case .speech:
            state = .done
            return ending(.tapped, from: onsetFrame, to: lastActive + 1)
        case .candidate:
            state = .done
            return ending(.tapped, from: candidateStart, to: min(n, lastActive + 1))
        default:
            state = .done
            // Nothing confirmed: keep anything loud enough to be worth a listen.
            let t = threshold(hold: true)
            guard let a = frameEnergy.indices.first(where: { $0 >= listeningFrom && frameEnergy[$0] > t }),
                  let b = frameEnergy.indices.last(where: { frameEnergy[$0] > t }) else {
                return Ending(reason: .noSpeech, speech: nil, keep: nil)
            }
            return ending(.tapped, from: a, to: b + 1)
        }
    }

    // MARK: Frames

    private func threshold(hold: Bool) -> Float {
        hold ? max(floor + config.hold, config.minimumOnset - 4) : max(floor + config.onsetAbove, config.minimumOnset)
    }

    private mutating func frame(_ x: [Float]) -> [Event]? {
        var meanSquare: Float = 0
        vDSP_measqv(x, 1, &meanSquare, vDSP_Length(x.count))
        let db = 10 * log10(meanSquare + 1e-12)
        let index = frameEnergy.count
        frameEnergy.append(db)
        pushHistory(x)
        level = max(0, min(1, (db - floor) / 40))

        switch state {
        case .calibrating:
            guard frameEnergy.count >= calibrationFrames else { return nil }
            let sorted = frameEnergy.sorted()
            floor = max(-90, min(-30, sorted[sorted.count / 5]))
            state = .waiting
            listeningFrom = index + 1
            return [.listening(floor: floor)]
        case .waiting:
            if index - listeningFrom > frames(config.waitLimit) {
                state = .done
                return [.ended(Ending(reason: .noSpeech, speech: nil, keep: nil))]
            }
            if db > threshold(hold: false) {
                state = .candidate
                candidateStart = index
                candidateVoiced = isVoiced() ? 1 : 0
                candidateQuiet = 0
                lastActive = index
            } else {
                track(db)
            }
            return nil
        case .candidate:
            if db > threshold(hold: true) {
                lastActive = index
                candidateQuiet = 0
                if isVoiced() { candidateVoiced += 1 }
            } else {
                candidateQuiet += 1
            }
            if candidateVoiced >= config.voicedToConfirm {
                state = .speech
                onsetSeen = true
                onsetFrame = backdate(candidateStart)
                return [.onset(onsetFrame * hop)]
            }
            // A burst with no voice in it (a knock, a breath, noise): forget it.
            if candidateQuiet > frames(0.12) || index - candidateStart > frames(config.confirmWithin) {
                state = .waiting
            }
            return nil
        case .speech:
            if db > threshold(hold: true) { lastActive = index }
            if index - lastActive >= frames(config.trailingSilence) {
                state = .done
                return [.ended(ending(.silence, from: onsetFrame, to: lastActive + 1))]
            }
            if index - onsetFrame >= frames(config.maxSpeech) {
                state = .done
                return [.ended(ending(.maxLength, from: onsetFrame, to: index + 1))]
            }
            return nil
        case .done:
            return nil
        }
    }

    /// The floor follows the room while nobody speaks: down quickly, up slowly.
    private mutating func track(_ db: Float) {
        if db < floor { floor = 0.7 * floor + 0.3 * db } else { floor += 0.02 * (db - floor) }
    }

    /// Steps back from the first loud frame over the quieter lead-in (an с, a soft п burst).
    private func backdate(_ start: Int) -> Int {
        let soft = floor + 3
        var i = start
        let limit = max(listeningFrom, start - frames(0.15))
        while i - 1 >= limit && frameEnergy[i - 1] > soft { i -= 1 }
        return i
    }

    /// Speech frames [a, b) refined at the tail over its fading end, plus padding, as samples.
    private func ending(_ reason: Reason, from a: Int, to b: Int) -> Ending {
        var end = b
        let soft = floor + 3
        let limit = min(frameEnergy.count, b + frames(0.2))
        while end < limit && frameEnergy[end] > soft { end += 1 }
        let total = frameEnergy.count * hop + pending.count
        let speech = (a * hop)..<min(total, end * hop)
        let keep = max(0, speech.lowerBound - Int(config.padBefore * sampleRate))..<min(total, speech.upperBound + Int(config.padAfter * sampleRate))
        return Ending(reason: reason, speech: speech, keep: keep)
    }

    // MARK: Voicing

    private mutating func pushHistory(_ x: [Float]) {
        var low: [Float] = []
        low.reserveCapacity(x.count / decimation + 1)
        var j = 0
        while j + decimation <= x.count {
            var s: Float = 0
            for k in 0..<decimation { s += x[j + k] }
            let v = s / Float(decimation)
            low.append(v - 0.95 * lastRaw)  // pre-emphasis whitens room rumble
            lastRaw = v
            j += decimation
        }
        history.append(contentsOf: low)
        let keep = window + maxLag
        if history.count > keep { history.removeFirst(history.count - keep) }
    }

    /// Normalised autocorrelation peak over 70–400 Hz in the last 30 ms.
    private func isVoiced() -> Bool {
        guard history.count >= window + maxLag else { return false }
        return Self.periodicity(history, window: window, minLag: minLag, maxLag: maxLag) > config.voicing
    }

    static func periodicity(_ h: [Float], window: Int, minLag: Int, maxLag: Int) -> Float {
        let base = h.count - window - maxLag
        guard base >= 0 else { return 0 }
        var best: Float = 0
        h.withUnsafeBufferPointer { p in
            let s = p.baseAddress! + base
            var e0: Float = 0
            vDSP_svesq(s, 1, &e0, vDSP_Length(window))
            guard e0 > 1e-10 else { return }
            var corr = [Float](repeating: 0, count: maxLag + 1)
            vDSP_conv(s, 1, s, 1, &corr, 1, vDSP_Length(maxLag + 1), vDSP_Length(window))
            for tau in minLag...maxLag {
                var et: Float = 0
                vDSP_svesq(s + tau, 1, &et, vDSP_Length(window))
                let r = corr[tau] / sqrt(e0 * et + 1e-12)
                if r > best { best = r }
            }
        }
        return best
    }
}
