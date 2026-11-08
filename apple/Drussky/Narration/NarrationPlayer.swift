import AVFoundation
import Foundation
import Observation

/// Plays a narration clip with a live playhead: the read-along view highlights the word at `time`,
/// a chunk plays from one word to another, and speed drops to 0.75× without changing pitch
/// (AVAudioPlayer's rate uses the time-pitch unit). One player for the app, like `Voice`: any new
/// playback replaces the current one.
@MainActor
@Observable
final class NarrationPlayer: NSObject, AVAudioPlayerDelegate {
    static let shared = NarrationPlayer()

    /// The clip that is loaded (an Audio/ file name) and whether it is sounding.
    private(set) var file: String?
    private(set) var isPlaying = false
    /// Playhead in seconds of the clip, refreshed ~30× a second while playing.
    private(set) var time: Double = 0
    /// 1.0 or 0.75: the reader at natural speed or slowed, same pitch.
    var rate: Float = 1.0 {
        didSet { player?.rate = rate }
    }
    /// Which exercise asked for the current playback (so two buttons on a page do not both show "playing").
    private(set) var owner: String?

    private var player: AVAudioPlayer?
    private var stopAt: Double = .infinity
    private var ticker: Task<Void, Never>?

    var duration: Double { player?.duration ?? 0 }

    /// Loads `file` (an Audio/ clip) if it is not the current one. False when the clip is missing.
    @discardableResult
    func load(_ file: String) -> Bool {
        if self.file == file, player != nil { return true }
        stop()
        guard let url = Narration.url(file), let p = try? AVAudioPlayer(contentsOf: url) else { return false }
        p.enableRate = true
        p.rate = rate
        p.delegate = self
        p.prepareToPlay()
        player = p
        self.file = file
        time = 0
        return true
    }

    /// Plays `file` from `start` and stops at `end` (the whole clip when omitted).
    func play(_ file: String, from start: Double = 0, to end: Double = .infinity, owner: String? = nil) {
        guard load(file), let player else { return }
        self.owner = owner
        stopAt = end
        player.currentTime = max(0, min(start, max(0, player.duration - 0.05)))
        time = player.currentTime
        player.play()
        isPlaying = true
        tick()
    }

    /// Plays an exercise's narration: its word span, or the whole clip.
    func play(exercise ex: Exercise, owner: String? = nil) {
        guard let i = Narration.interval(of: ex) else { return }
        play(i.file, from: i.start, to: i.end, owner: owner)
    }

    func pause() {
        player?.pause()
        isPlaying = false
        ticker?.cancel()
        if let player { time = player.currentTime }
    }

    /// Continues from the playhead, to the end of the clip.
    func resume() {
        guard let player else { return }
        stopAt = .infinity
        player.play()
        isPlaying = true
        tick()
    }

    func seek(to t: Double) {
        guard let player else { return }
        player.currentTime = max(0, min(t, player.duration))
        time = player.currentTime
    }

    func stop() {
        player?.stop()
        player = nil
        file = nil
        owner = nil
        isPlaying = false
        ticker?.cancel()
        time = 0
    }

    private func tick() {
        ticker?.cancel()
        ticker = Task { [weak self] in
            while let self, !Task.isCancelled, self.isPlaying {
                if let player = self.player {
                    self.time = player.currentTime
                    if self.time >= self.stopAt { self.pause(); break }
                }
                try? await Task.sleep(for: .milliseconds(33))
            }
        }
    }

    nonisolated func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
        // Only Sendable values cross to the main actor: the player's identity and its duration.
        let finished = ObjectIdentifier(player)
        let duration = player.duration
        Task { @MainActor in
            guard let current = self.player, ObjectIdentifier(current) == finished else { return }
            self.isPlaying = false
            self.ticker?.cancel()
            self.time = duration
        }
    }
}
