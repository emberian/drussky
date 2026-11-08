import SwiftUI

/// A narrated passage: the human reading plays and the word being said lights up; tap a word to
/// hear from there, slow the reader to 0.75×, or listen first with the text hidden and revealed as
/// it is read. Shown by `PassageExercise` in place of its plain text card when the exercise carries
/// `narration` + `words`.
struct ReadAlongView: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    let exercise: Exercise
    let owner: String
    /// The word the learner tapped (gloss shown by the passage view).
    @Binding var selected: String?
    /// The «Структу́ра» layer over the same words (Structure/), when the text has one.
    var structure: PassageStructure? = nil
    var layer: StructureLayer? = nil
    @State private var listenFirst = false
    @State private var revealedUpTo = -1
    private let player = NarrationPlayer.shared

    private var text: String { exercise.text ?? "" }
    private var timings: WordTimings? { exercise.words.flatMap(Narration.timings) }
    private var mapping: [Int?]? { timings.flatMap { Narration.map(text, onto: $0) } }
    private var mine: Bool { player.file == exercise.narration && player.owner == owner }
    private var structureOn: Bool { structure != nil && layer != nil && book.settings.showStructure == true }
    private var current: Int? {
        guard mine, let timings else { return nil }
        return Narration.wordIndex(at: player.time, in: timings.words)
    }
    /// Lines (dialogue turns, verse) of tokens, each with the index of its first token in the whole text.
    private var lines: [(base: Int, tokens: [String])] {
        var out: [(Int, [String])] = []
        var base = 0
        for line in text.split(separator: "\n", omittingEmptySubsequences: true) {
            let toks = Narration.tokens(String(line)).map(String.init)
            out.append((base, toks))
            base += toks.count
        }
        return out
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(spacing: 12) {
                Button { togglePlay() } label: {
                    Image(systemName: mine && player.isPlaying ? "pause.fill" : "play.fill")
                        .font(.system(size: 20, weight: .semibold))
                        .frame(width: 48, height: 48)
                        .background(Circle().fill(Theme.accent))
                        .foregroundStyle(.white)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(mine && player.isPlaying ? "Pause the reading" : "Play the reading")
                .accessibilityIdentifier("narration-play")
                Instruction(text: listenFirst ? "Listen first. The words appear as they are read."
                                              : "Follow the reader. Tap any word to hear from there.")
                Spacer(minLength: 0)
                Button { player.rate = player.rate == 1 ? 0.75 : 1 } label: {
                    Text(player.rate == 1 ? "1×" : "¾×")
                        .font(Theme.label(14)).monospacedDigit()
                        .padding(.horizontal, 10).padding(.vertical, 7)
                        .background(Capsule().fill(Theme.accent.opacity(player.rate == 1 ? 0.10 : 0.22)))
                        .foregroundStyle(Theme.accent)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(player.rate == 1 ? "Slow the reader down" : "Natural speed")
                Button {
                    withAnimation { listenFirst.toggle(); revealedUpTo = -1 }
                } label: {
                    Image(systemName: listenFirst ? "eye.slash" : "eye")
                        .font(.system(size: 16, weight: .semibold))
                        .frame(width: 36, height: 34)
                        .background(Capsule().fill(Theme.accent.opacity(listenFirst ? 0.22 : 0.10)))
                        .foregroundStyle(Theme.accent)
                }
                .buttonStyle(.plain)
                .accessibilityLabel(listenFirst ? "Show the text" : "Listen first, text hidden")
                if structure != nil, layer != nil { StructureToggle() }
            }
            VStack(alignment: .leading, spacing: 6) {
                let map = mapping
                ForEach(Array(lines.enumerated()), id: \.offset) { li, line in
                    FlowRow(spacing: 8) {
                        ForEach(Array(line.tokens.enumerated()), id: \.offset) { j, tok in
                            wordView(tok, timing: map?[line.base + j], line: li, index: j)
                        }
                    }
                }
            }
            .padding(18)
            .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
            .onChange(of: current) { _, now in
                if let now, now > revealedUpTo { revealedUpTo = now }
            }
            if let timings, let reader = timings.reader {
                Text("Read by \(reader) · \(Int(timings.duration.rounded())) s").font(Theme.label(12)).foregroundStyle(Theme.muted)
            }
        }
        .onAppear(perform: qaSeek)
        .onDisappear { if mine { player.pause() } }
    }

    @ViewBuilder private func wordView(_ token: String, timing: Int?, line: Int, index: Int) -> some View {
        let word = PassageExercise.bare(token)
        let isNow = timing != nil && timing == current
        let hidden = listenFirst && (timing.map { $0 > revealedUpTo } ?? (revealedUpTo < 0))
        let tok = structure?.token(line: line, index: index)
        let mark: StructureMark = structureOn && !hidden
            ? layer!.mark(for: tok, previous: structure!.token(line: line, index: index - 1), next: structure!.token(line: line, index: index + 1),
                          in: structure!, roles: Structure.shared.roles) : .none
        Button { tap(word, timing: timing, token: tok) } label: {
            Text(Theme.stressed(token, show: book.settings.showStress))
                .russianFont(28, weight: isNow ? .semibold : .regular)
                .foregroundStyle(isNow ? Theme.accent : Theme.text)
                .opacity(hidden ? 0 : 1)
                .padding(.vertical, 2).padding(.horizontal, 3)
                .background(RoundedRectangle(cornerRadius: 6)
                    .fill(isNow ? Theme.accent.opacity(0.18) : selected == word && !hidden ? Theme.accent.opacity(0.10) : hidden ? Theme.muted.opacity(0.12) : .clear))
                .structureMarks(mark, on: structureOn && !hidden)
        }
        .buttonStyle(.plain)
        .simultaneousGesture(LongPressGesture(minimumDuration: 0.45).onEnded { _ in
            guard structureOn, !hidden, let tok, let si = tok.sentence, let s = structure?.sentence(si) else { return }
            layer?.longPressed(s, at: si, words: tok.words)
        })
        .accessibilityLabel(hidden ? "hidden word" : RussianText.unstressed(word))
        .accessibilityIdentifier(isNow ? "narration-current-word" : "narration-word")
    }

    private func togglePlay() {
        if mine && player.isPlaying { player.pause(); return }
        voice.stop()
        if mine, player.time > 0, player.time < player.duration - 0.1 { player.resume() }
        else if let file = exercise.narration { player.play(file, owner: owner) }
    }

    private func tap(_ word: String, timing: Int?, token: PassageStructure.Token? = nil) {
        if structureOn, layer?.consumeSuppressedTap() == true { return }  // the tap that ends a long press
        selected = word
        if structureOn, let token, let si = token.sentence, let s = structure?.sentence(si) { layer?.tapped(s, at: si, words: token.words) }
        guard let timing, let timings, let file = exercise.narration else { voice.play(word); return }
        voice.stop()
        if listenFirst, timing > revealedUpTo { return }  // not yet read: no peeking by tapping
        player.play(file, from: max(0, timings.words[timing].start - 0.05), owner: owner)
    }

    /// `-qa-narration-at MS` shows the page paused at that moment of the reading (screenshots).
    private func qaSeek() {
        guard let ms = QA.value("-qa-narration-at"), let file = exercise.narration else { return }
        player.load(file)
        player.play(file, from: Double(ms) / 1000, owner: owner)
        player.pause()
    }
}

/// A round speaker for an exercise built from a narration chunk (chunk dictation, shadowing): plays
/// the reader saying exactly that stretch, in place of `HearButton`'s clip or synthetic voice.
struct NarrationClipButton: View {
    @Environment(Voice.self) private var voice
    let exercise: Exercise
    let owner: String
    var size: CGFloat = 44
    var label: String? = nil
    private let player = NarrationPlayer.shared

    var body: some View {
        let playing = player.isPlaying && player.owner == owner
        Button {
            if playing { player.pause() } else { voice.stop(); player.play(exercise: exercise, owner: owner) }
        } label: {
            HStack(spacing: 8) {
                Image(systemName: playing ? "speaker.wave.3.fill" : "speaker.wave.2.fill")
                    .symbolEffect(.variableColor.iterative, isActive: playing)
                    .font(.system(size: size * 0.42, weight: .semibold))
                    .frame(width: size, height: size)
                    .background(Circle().fill(Theme.accent.opacity(0.12)))
                if let label { Text(label).font(Theme.label(14)) }
            }
            .foregroundStyle(Theme.accent)
            .overlay(alignment: .topTrailing) {
                Image(systemName: "headphones")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundStyle(.white)
                    .frame(width: 15, height: 15)
                    .background(Circle().fill(Theme.accent))
                    .offset(x: 3, y: -3)
                    .accessibilityHidden(true)
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label ?? "Hear the reader")
        .onDisappear { if player.owner == owner, player.isPlaying { player.pause() } }
    }
}

/// "Listen and read along" for a narrated library text: makes the read-along page locally (no
/// tutor needed) and opens it.
struct ReadAlongButton: View {
    @Environment(Book.self) private var book
    let text: Library.Text
    var onDone: () -> Void

    var body: some View {
        Button {
            let page = Narration.page(for: text)
            book.insertWrittenPage(title: page.title, intro: page.intro, unitTitle: "Library · \(text.level) · read aloud",
                                   exercises: page.exercises, newWords: [])
            onDone()
        } label: {
            Label("Listen and read along", systemImage: "headphones")
                .font(.system(size: 18, weight: .semibold, design: .rounded))
                .frame(maxWidth: .infinity).padding(.vertical, 14)
                .background(Capsule().fill(Theme.accent))
                .foregroundStyle(Color.white)
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("read-along")
    }
}
