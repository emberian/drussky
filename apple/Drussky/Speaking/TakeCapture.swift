import AVFoundation

/// What the microphone reports during one take, in order.
enum CaptureEvent: Sendable, Equatable {
    /// The room is measured; waiting for the voice.
    case listening
    /// 0…1 above the room's floor, about 30 times a second.
    case level(Float)
    /// The voice began.
    case voice
    case ended(VoiceActivity.Ending)
}

/// The microphone side of a take: an AVAudioEngine with an input tap feeding `VoiceActivity`,
/// a player node for the "your turn" cue, and the samples of the take in memory until they are
/// written, trimmed, as the m4a that stays on the page.
///
/// Nonisolated and lock-protected on purpose: the tap and the cue's completion run on audio
/// threads, and a main-actor closure called there traps at runtime (the crash fixed in
/// SpeechCheck). Every closure handed to AVFoundation is formed here, outside any actor.
final class TakeCapture: @unchecked Sendable {
    enum Failure: Error { case noInput, engine(Error) }

    private let lock = NSLock()
    private let engine = AVAudioEngine()
    private let cue = AVAudioPlayerNode()
    private var detector: VoiceActivity?
    private var samples: [Float] = []
    private var continuation: AsyncStream<CaptureEvent>.Continuation?
    private var lastLevelAt = 0
    private var config: VoiceActivity.Config
    private(set) var sampleRate: Double = 0
    private var running = false

    init(config: VoiceActivity.Config = VoiceActivity.Config()) {
        self.config = config
    }

    /// Starts the engine and returns the event stream. Nothing is analysed until `cueThenListen`.
    func start() throws -> AsyncStream<CaptureEvent> {
        let (stream, continuation) = AsyncStream.makeStream(of: CaptureEvent.self, bufferingPolicy: .unbounded)
        let input = engine.inputNode
        let format = input.outputFormat(forBus: 0)
        guard format.sampleRate > 0, format.channelCount > 0 else { throw Failure.noInput }
        lock.withLock {
            self.continuation = continuation
            sampleRate = format.sampleRate
            samples = []
            detector = nil
        }
        engine.attach(cue)
        engine.connect(cue, to: engine.mainMixerNode, format: Self.cueFormat)
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            self?.receive(buffer)
        }
        engine.prepare()
        do { try engine.start() } catch {
            input.removeTap(onBus: 0)
            throw Failure.engine(error)
        }
        running = true
        return stream
    }

    /// Plays the soft "your turn" cue, then measures the room and listens. The cue itself never
    /// reaches the detector: it is armed only once the cue has played out.
    func cueThenListen() {
        guard let buffer = Self.cueBuffer else { arm(); return }
        cue.scheduleBuffer(buffer, at: nil, options: [], completionCallbackType: .dataPlayedBack) { [weak self] _ in
            guard let self else { return }
            DispatchQueue.global(qos: .userInitiated).asyncAfter(deadline: .now() + 0.05) { self.arm() }
        }
        cue.play()
    }

    private func arm() {
        lock.withLock {
            guard running else { return }
            detector = VoiceActivity(sampleRate: sampleRate, config: config)
            lastLevelAt = 0
            samples = []
            samples.reserveCapacity(Int(sampleRate * (config.maxSpeech + config.waitLimit + 1)))
        }
    }

    /// Called on the audio thread for every input buffer.
    private func receive(_ buffer: AVAudioPCMBuffer) {
        guard let data = buffer.floatChannelData?[0] else { return }
        let n = Int(buffer.frameLength)
        var out: [CaptureEvent] = []
        lock.withLock {
            guard var d = detector else { return }
            let chunk = UnsafeBufferPointer(start: data, count: n)
            samples.append(contentsOf: chunk)
            for e in d.feed(chunk) {
                switch e {
                case .listening: out.append(.listening)
                case .onset: out.append(.voice)
                case .ended(let ending): out.append(.ended(ending))
                }
            }
            if d.count - lastLevelAt >= Int(sampleRate / 30) || d.count < lastLevelAt {
                lastLevelAt = d.count
                out.append(.level(d.level))
            }
            detector = d
        }
        let c = lock.withLock { continuation }
        for e in out { c?.yield(e) }
    }

    /// The learner tapped: end the take now with whatever voice there is.
    func stopNow() {
        let (ending, c): (VoiceActivity.Ending?, AsyncStream<CaptureEvent>.Continuation?) = lock.withLock {
            guard var d = detector else { return (nil, continuation) }
            let e = d.finishNow()
            detector = d
            return (e, continuation)
        }
        c?.yield(.ended(ending ?? VoiceActivity.Ending(reason: .noSpeech, speech: nil, keep: nil)))
    }

    /// Stops the engine and closes the stream. Safe to call more than once, from any thread
    /// except the audio thread itself.
    func shutdown() {
        let c: AsyncStream<CaptureEvent>.Continuation? = lock.withLock {
            running = false
            detector = nil
            defer { continuation = nil }
            return continuation
        }
        engine.inputNode.removeTap(onBus: 0)
        cue.stop()
        engine.stop()
        c?.finish()
    }

    /// Writes the kept samples of `ending` as AAC into `url` (a file that doesn't exist yet), with
    /// 8 ms fades so the cut never clicks. Returns where the speech lies within the written file.
    func write(_ ending: VoiceActivity.Ending, to url: URL) throws -> (speechStart: Double, speechEnd: Double, duration: Double) {
        let (all, rate) = lock.withLock { (samples, sampleRate) }
        return try Self.write(all, sampleRate: rate, ending: ending, to: url)
    }

    static func write(_ all: [Float], sampleRate rate: Double, ending: VoiceActivity.Ending, to url: URL) throws
        -> (speechStart: Double, speechEnd: Double, duration: Double) {
        guard let keep = ending.keep, let speech = ending.speech else { throw Failure.noInput }
        let lo = max(0, min(all.count, keep.lowerBound)), hi = max(lo, min(all.count, keep.upperBound))
        var clip = Array(all[lo..<hi])
        let fade = min(clip.count / 2, Int(rate * 0.008))
        for i in 0..<fade {
            let g = Float(i) / Float(max(1, fade))
            clip[i] *= g
            clip[clip.count - 1 - i] *= g
        }
        guard let format = AVAudioFormat(commonFormat: .pcmFormatFloat32, sampleRate: rate, channels: 1, interleaved: false),
              let buffer = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(max(1, clip.count))) else {
            throw Failure.noInput
        }
        buffer.frameLength = AVAudioFrameCount(clip.count)
        clip.withUnsafeBufferPointer { src in
            buffer.floatChannelData![0].update(from: src.baseAddress!, count: clip.count)
        }
        let settings: [String: Any] = [
            AVFormatIDKey: kAudioFormatMPEG4AAC, AVSampleRateKey: rate,
            AVNumberOfChannelsKey: 1, AVEncoderAudioQualityKey: AVAudioQuality.high.rawValue,
        ]
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        do {
            let file = try AVAudioFile(forWriting: url, settings: settings, commonFormat: .pcmFormatFloat32, interleaved: false)
            try file.write(from: buffer)
        } catch {
            try? FileManager.default.removeItem(at: url)
            throw error
        }
        return (Double(speech.lowerBound - lo) / rate, Double(speech.upperBound - lo) / rate, Double(clip.count) / rate)
    }

    // MARK: The cue

    static let cueFormat = AVAudioFormat(standardFormatWithSampleRate: 44_100, channels: 1)!

    /// A soft two-partial "tink": quick attack, 160 ms decay, quiet.
    nonisolated(unsafe) static let cueBuffer: AVAudioPCMBuffer? = {
        let rate = cueFormat.sampleRate
        let n = Int(rate * 0.18)
        guard let b = AVAudioPCMBuffer(pcmFormat: cueFormat, frameCapacity: AVAudioFrameCount(n)) else { return nil }
        b.frameLength = AVAudioFrameCount(n)
        let out = b.floatChannelData![0]
        for i in 0..<n {
            let t = Double(i) / rate
            let attack = min(1, t / 0.006)
            let env = attack * exp(-t / 0.05)
            out[i] = Float(env * (0.16 * sin(2 * .pi * 880 * t) + 0.05 * sin(2 * .pi * 1_320 * t)))
        }
        return b
    }()
}
