import AVFoundation
import SwiftUI

/// A minimal-pair trial (or the labelled card that opens a contrast): hear one word, pick it,
/// then hear both. Each trial uses a different speaker where several recorded the word.
struct SoundPairExercise: View {
    @Environment(Voice.self) private var voice
    @Environment(Book.self) private var book
    let slot: ExerciseSlot
    let isActive: Bool
    @State private var playback: Task<Void, Never>?

    private var ex: Exercise { slot.exercise }
    private var options: [String] { ex.options ?? [] }
    private var exact: Bool { Set(options.map(RussianText.key)).count == 1 && options.count > 1 }
    private var contrastName: String { SoundPairExercise.contrasts[ex.hint ?? ""] ?? "" }

    var body: some View {
        if ex.isSoundPairTrial { trial } else { teaching }
    }

    // MARK: Teaching card

    private var teaching: some View {
        let examples = ex.examples ?? []
        return VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Text(contrastName).font(.system(size: 22, weight: .semibold, design: .serif)).foregroundStyle(Theme.text)
                Text("tap each word; hear them side by side").scaledFont(14).foregroundStyle(Theme.muted)
            }
            if let note = ex.text {
                Text(note).scaledFont(16).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
            }
            Grid(alignment: .leading, horizontalSpacing: 14, verticalSpacing: 6) {
                ForEach(Array(stride(from: 0, to: examples.count - 1, by: 2)), id: \.self) { i in
                    GridRow {
                        wordCell(examples[i], exactStress: RussianText.key(examples[i].ru) == RussianText.key(examples[i + 1].ru))
                        Text("·").foregroundStyle(Theme.muted)
                        wordCell(examples[i + 1], exactStress: RussianText.key(examples[i].ru) == RussianText.key(examples[i + 1].ru))
                        Button { playBoth(examples[i].ru, examples[i + 1].ru, exactStress: RussianText.key(examples[i].ru) == RussianText.key(examples[i + 1].ru)) } label: {
                            Image(systemName: "arrow.left.arrow.right").font(.system(size: 14, weight: .semibold))
                                .frame(width: 34, height: 34).background(Circle().fill(Theme.accent.opacity(0.1)))
                        }
                        .buttonStyle(.plain).foregroundStyle(Theme.accent)
                        .accessibilityLabel("Play both")
                    }
                }
            }
            if !slot.result.status.isComplete {
                PillButton(title: "Ready", systemImage: "checkmark", prominent: true) { slot.finish { $0.status = .done } }
            }
        }
    }

    private func wordCell(_ w: Course.Example, exactStress: Bool) -> some View {
        Button { play(w.ru, exactStress: exactStress, voice: nil) } label: {
            HStack(alignment: .firstTextBaseline, spacing: 8) {
                Text(Theme.stressed(w.ru, show: true)).russianFont(26, weight: .medium).foregroundStyle(Theme.text)
                Text(w.en).scaledFont(14).foregroundStyle(Theme.muted).lineLimit(1)
            }
        }
        .buttonStyle(.plain)
    }

    // MARK: Trial

    private var trial: some View {
        let answer = ex.answerIndex ?? 0
        let done = slot.result.status.isComplete
        return VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 14) {
                Button { play(ex.say ?? "", exactStress: exact, voice: ex.times) } label: {
                    Image(systemName: "speaker.wave.2.fill").font(.system(size: 22, weight: .semibold))
                        .frame(width: 52, height: 52).background(Circle().fill(Theme.accent.opacity(0.12)))
                }
                .buttonStyle(.plain).foregroundStyle(Theme.accent)
                .accessibilityLabel("Play again")
                VStack(alignment: .leading, spacing: 2) {
                    Instruction(text: "Which word was it?")
                    Text(contrastName).font(Theme.label(12)).foregroundStyle(Theme.muted)
                }
            }
            HStack(spacing: 12) {
                ForEach(Array(options.enumerated()), id: \.offset) { i, word in
                    option(i, word, answer: answer, done: done)
                }
            }
            if done {
                feedback(answer: answer)
            }
        }
        .onChange(of: book.advanceSignal) { _, _ in
            if isActive, slot.isAdvanceTarget, !done { play(ex.say ?? "", exactStress: exact, voice: ex.times) }
        }
        .onDisappear { playback?.cancel() }
    }

    private func option(_ i: Int, _ word: String, answer: Int, done: Bool) -> some View {
        let chosen = slot.result.chosen == i
        let right = i == answer
        let color: Color = done ? (right ? Theme.good : (chosen ? Theme.redPen : Theme.text)) : Theme.text
        return Button { choose(i) } label: {
            VStack(spacing: 2) {
                Text(Theme.stressed(word, show: exact || book.settings.showStress))
                    .russianFont(30, weight: .medium)
                Text(ex.glosses?[RussianText.pairKey(word)] ?? "").scaledFont(14).foregroundStyle(Theme.muted)
            }
            .foregroundStyle(color)
            .padding(.horizontal, 22).padding(.vertical, 10)
            .frame(minWidth: 130)
            .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
            .overlay(RoundedRectangle(cornerRadius: 14)
                .stroke(done && (right || chosen) ? color : Theme.rule.opacity(0.5), lineWidth: done && (right || chosen) ? 2.5 : 1))
        }
        .buttonStyle(.plain)
        .disabled(done)
    }

    private func feedback(answer: Int) -> some View {
        let said = options[safe: answer] ?? ""
        let other = options[safe: 1 - answer] ?? ""
        let right = slot.result.status == .correct
        let gloss = { (w: String) in ex.glosses?[RussianText.pairKey(w)] ?? "" }
        return HStack(spacing: 10) {
            Text(right ? "Yes — " : "It was ").font(.system(size: 15, weight: .medium)).foregroundStyle(right ? Theme.good : Theme.redPen)
            + Text(Theme.stressed(said, show: true)).font(Theme.russian(19, weight: .medium))
            + Text(", \(gloss(said)). Compare ").font(.system(size: 15))
            + Text(Theme.stressed(other, show: true)).font(Theme.russian(19, weight: .medium))
            + Text(", \(gloss(other)).").font(.system(size: 15))
            Spacer(minLength: 0)
            Button { playBoth(said, other, exactStress: exact, first: ex.times) } label: {
                Label("Both", systemImage: "arrow.left.arrow.right").font(Theme.label(13))
            }
            .buttonStyle(.bordered).tint(Theme.accent)
            Button { play(said, exactStress: exact, voice: nil) } label: {
                Label("Another voice", systemImage: "person.2.wave.2").font(Theme.label(13))
            }
            .buttonStyle(.bordered).tint(Theme.accent)
            Button { playback?.cancel(); voice.stop(); book.requestAdvance(page: slot.ref.pageID) } label: {
                Label("Next", systemImage: "arrow.down").font(Theme.label(13))
            }
            .buttonStyle(.bordered).tint(Theme.accent)
        }
        .foregroundStyle(Theme.text)
    }

    private func choose(_ i: Int) {
        let right = i == ex.answerIndex
        slot.record { r in
            r.chosen = i
            r.attempts += 1
            r.status = right ? .correct : .missed
        }
        // Hear what it was, then the other member, in that speaker's voice where possible; the
        // page moves on only when that comparison has finished (or the learner taps Next).
        let answer = ex.answerIndex ?? 0
        let book = slot.book, page = slot.ref.pageID
        playBoth(options[safe: answer] ?? "", options[safe: 1 - answer] ?? "", exactStress: exact, first: ex.times) {
            book.requestAdvance(page: page)
        }
    }

    // MARK: Playback

    @State private var turn = 0

    private func clips(_ word: String, exactStress: Bool) -> [URL] { voice.pairVoices(word, exactStress: exactStress) }

    /// `voice` picks a speaker; nil plays the next one in turn.
    private func play(_ word: String, exactStress: Bool, voice speaker: Int?) {
        playback?.cancel()
        let urls = clips(word, exactStress: exactStress)
        guard !urls.isEmpty else { voice.play(word); return }
        let n: Int
        if let speaker { n = speaker } else { turn += 1; n = (ex.times ?? 0) + turn }
        voice.playFile(urls[n % urls.count], key: RussianText.pairKey(word))
    }

    /// One word, then the other, as one sequence: anything else played meanwhile (the next
    /// trial, a page turn) ends it, and `then` runs only if it played to the end.
    private func playBoth(_ a: String, _ b: String, exactStress: Bool, first: Int? = nil, then: (@MainActor () -> Void)? = nil) {
        playback?.cancel()
        let ua = clips(a, exactStress: exactStress), ub = clips(b, exactStress: exactStress)
        guard let x = ua[safe: (first ?? 0) % max(1, ua.count)], let y = ub[safe: (first ?? 0) % max(1, ub.count)] else { then?(); return }
        let voice = self.voice
        let token = voice.playFile(x, key: RussianText.pairKey(a))
        playback = Task { @MainActor in
            try? await Task.sleep(for: .seconds(Self.duration(x) + 0.45))
            guard !Task.isCancelled, voice.continueSequence(y, key: RussianText.pairKey(b), token: token) else { return }
            try? await Task.sleep(for: .seconds(Self.duration(y) + 0.35))
            guard !Task.isCancelled, voice.sequence == token else { return }
            then?()
        }
    }

    static func duration(_ url: URL) -> Double {
        guard let f = try? AVAudioFile(forReading: url) else { return 0.8 }
        return Double(f.length) / f.processingFormat.sampleRate
    }

    /// Contrast names for labels, from pairs.json.
    static let contrasts: [String: String] = Dictionary(uniqueKeysWithValues: SoundPairs.load().contrasts.map { ($0.id, $0.name) })
}
