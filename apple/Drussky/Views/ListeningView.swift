import AVFoundation
import SwiftUI

/// One heard situation (Model/Listening.swift): the English task, Play (nothing plays until the
/// learner taps), Replay and a slower pace; the choices for its kind; and, once answered, the
/// stressed transcript with the deciding span underlined in red, why, and the translation.
/// Nothing moves or scrolls when the answer lands.
struct ListeningExercise: View {
    @Environment(Book.self) private var book
    @Environment(Voice.self) private var voice
    @Environment(\.horizontalSizeClass) private var sizeClass
    let slot: ExerciseSlot
    let isActive: Bool

    @State private var slower = false
    @State private var plays = 0
    /// Which line of an exchange is being said (the badge lights), nil between playbacks.
    @State private var speakingLine: Int?
    /// When the last playback ended (response latency runs from here to the tap).
    @State private var heardAt: Date?
    /// A message's picks so far, committed together once every question has one.
    @State private var pending: [Int: Int] = [:]
    @State private var playback: Task<Void, Never>?

    private var ex: Exercise { slot.exercise }
    private var item: ListeningItem? { ex.listening }
    private var done: Bool { slot.result.status.isComplete }
    private var compact: Bool { sizeClass == .compact }

    var body: some View {
        if let item {
            VStack(alignment: .leading, spacing: 12) {
                prompt(item)
                controls(item)
                if item.kind.isExchange { badges(item) }
                if item.kind == .follow { scene(item) }
                choices(item)
                if done { reveal(item) }
            }
            .frame(maxWidth: 680, alignment: .leading)
            .onDisappear { playback?.cancel(); playback = nil }
        }
    }

    // MARK: Prompt and playback

    private func prompt(_ item: ListeningItem) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack(spacing: 8) {
                Text(kindLabel(item.kind)).font(Theme.label(12)).tracking(1.2).foregroundStyle(Theme.accent)
                if item.voice_hint == "unfamiliar" { Text("· a voice you may not know").font(Theme.label(12)).foregroundStyle(Theme.muted) }
            }
            if item.kind != .message, let q = item.question {
                Text(q).scaledFont(18, weight: .medium).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
            } else if item.kind == .message {
                Text("A voice message. Listen, then answer the questions below.").scaledFont(18, weight: .medium).foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private func controls(_ item: ListeningItem) -> some View {
        FlowRow(spacing: 10) {
            PillButton(title: plays == 0 ? "Play" : "Replay", systemImage: plays == 0 ? "play.fill" : "arrow.counterclockwise", prominent: plays == 0) {
                play(item)
            }
            .accessibilityIdentifier("listening-play-\(slot.index)")
            Button { slower.toggle() } label: {
                Label("Slower", systemImage: "tortoise").font(Theme.label(14))
                    .padding(.horizontal, 14).padding(.vertical, 9).frame(minHeight: 44)
                    .foregroundStyle(slower ? Color.white : Theme.accent)
                    .background(Capsule().fill(slower ? Theme.accent : Theme.accent.opacity(0.10)))
            }
            .buttonStyle(.plain)
            .accessibilityValue(slower ? "on" : "off")
            if plays > 0 && !done {
                Text(plays == 1 ? "Heard once." : "Heard \(plays) times.").font(Theme.label(13)).foregroundStyle(Theme.muted)
            }
        }
    }

    /// Plays the item once: each line in its own voice in order, or the one turn. A second tap
    /// while it plays starts it over; a page turn stops it (`Voice.stop`).
    private func play(_ item: ListeningItem) {
        playback?.cancel()
        plays += 1
        heardAt = nil
        let rate: Float = slower ? 0.75 : 1
        let voice = self.voice
        playback = Task { @MainActor in
            for (i, text) in item.heard.enumerated() {
                speakingLine = item.kind.isExchange ? i : nil
                let finished: Bool
                if let url = voice.recordingURL(for: text, voice: item.voice(forLine: i)) ?? voice.recordingURL(for: text) {
                    finished = await voice.playFileToEnd(url, key: RussianText.audioKey(text), rate: rate)
                } else {
                    await voice.playToEnd(text)  // the system voice: no clip of this text in the bundle
                    finished = !Task.isCancelled
                }
                guard finished, !Task.isCancelled else { speakingLine = nil; playback = nil; return }
                if i + 1 < item.heard.count { try? await Task.sleep(for: .milliseconds(450)) }
            }
            speakingLine = nil
            heardAt = Date()
            playback = nil
        }
    }

    /// One span or form, in the item's own voice (the slower pace applies here too).
    private func playOne(_ text: String, voice name: String) {
        playback?.cancel(); playback = nil
        speakingLine = nil
        if let url = voice.recordingURL(for: text, voice: name) ?? voice.recordingURL(for: text) {
            voice.playFile(url, key: RussianText.audioKey(text), rate: slower ? 0.75 : 1)
        } else {
            voice.play(text)
        }
    }

    /// Both intonation forms, one after the other, in the same voice.
    private func playBothForms(_ item: ListeningItem) {
        guard let forms = item.forms, forms.count == 2 else { return }
        playback?.cancel()
        let voice = self.voice, name = item.voice(), rate: Float = slower ? 0.75 : 1
        playback = Task { @MainActor in
            for f in forms {
                if let url = voice.recordingURL(for: f.ru, voice: name) ?? voice.recordingURL(for: f.ru) {
                    guard await voice.playFileToEnd(url, key: RussianText.audioKey(f.ru), rate: rate) else { break }
                } else {
                    await voice.playToEnd(f.ru)
                }
                try? await Task.sleep(for: .milliseconds(500))
                if Task.isCancelled { break }
            }
            playback = nil
        }
    }

    // MARK: Speakers and scene

    /// A and B, the one speaking lit; their words never shown before the answer.
    private func badges(_ item: ListeningItem) -> some View {
        HStack(spacing: 10) {
            ForEach(Array((item.lines ?? []).enumerated()), id: \.offset) { i, line in
                let lit = speakingLine == i
                Text(line.speaker)
                    .font(.system(size: 15, weight: .bold, design: .rounded))
                    .frame(width: 34, height: 34)
                    .foregroundStyle(lit ? Color.white : Theme.accent)
                    .background(Circle().fill(lit ? Theme.accent : Theme.accent.opacity(0.12)))
                    .accessibilityLabel("Speaker \(line.speaker)\(lit ? ", speaking" : "")")
            }
            Text("two people, in turn").font(Theme.label(13)).foregroundStyle(Theme.muted)
        }
    }

    /// The things that can move, in a tray, and the places they can go to, as word chips.
    private func scene(_ item: ListeningItem) -> some View {
        let objects = (item.scene ?? []).filter { $0.role == "object" }
        let places = (item.scene ?? []).filter { $0.role == "place" }
        let outcome = done ? item.answer.flatMap { item.outcomes?[safe: $0] } : nil
        return VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top, spacing: 14) {
                Text("TRAY").font(Theme.label(11)).tracking(1.4).foregroundStyle(Theme.muted).frame(width: 64, alignment: .leading).padding(.top, 10)
                FlowRow(spacing: 8) {
                    ForEach(objects) { chip($0, marked: outcome?.object == $0.id, moved: outcome?.object == $0.id && outcome?.place != nil) }
                }
            }
            HStack(alignment: .top, spacing: 14) {
                Text("PLACES").font(Theme.label(11)).tracking(1.4).foregroundStyle(Theme.muted).frame(width: 64, alignment: .leading).padding(.top, 10)
                FlowRow(spacing: 8) {
                    ForEach(places) { chip($0, marked: outcome?.place == $0.id, moved: false) }
                }
            }
        }
        .padding(12)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
    }

    private func chip(_ e: ListeningItem.SceneEntity, marked: Bool, moved: Bool) -> some View {
        Button { playOne(e.ru, voice: "xenia") } label: {
            VStack(spacing: 1) {
                Text(Theme.stressed(e.ru, show: book.settings.showStress)).russianFont(22, weight: .medium).foregroundStyle(marked ? Theme.good : Theme.text)
                Text(e.en).scaledFont(12).foregroundStyle(Theme.muted)
            }
            .padding(.horizontal, 12).padding(.vertical, 6)
            .background(RoundedRectangle(cornerRadius: 10).fill(Theme.paper))
            .overlay(RoundedRectangle(cornerRadius: 10).stroke(marked ? Theme.good : Theme.rule.opacity(0.5), lineWidth: marked ? 2 : 1))
            .overlay(alignment: .topTrailing) {
                if moved { Image(systemName: "arrow.turn.right.down").font(.system(size: 11, weight: .bold)).foregroundStyle(Theme.good).offset(x: 4, y: -6) }
            }
        }
        .buttonStyle(.plain)
        .accessibilityLabel("\(RussianText.unstressed(e.ru)), \(e.en)\(marked ? ", in the answer" : "")")
    }

    // MARK: Choices

    @ViewBuilder private func choices(_ item: ListeningItem) -> some View {
        let picks = ListeningItem.picks(slot.result)
        ForEach(0..<max(1, item.decisionCount), id: \.self) { i in
            VStack(alignment: .leading, spacing: 8) {
                if item.kind == .message, let q = item.questions?[safe: i] {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text("\(i + 1).").font(Theme.label(14)).foregroundStyle(Theme.muted)
                        Text(q.prompt).scaledFont(17, weight: .medium).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
                    }
                }
                ChoiceGrid(options: item.options(i), chosen: done ? picks[safe: i] ?? nil : pending[i], correct: done ? item.answer(i) : nil,
                           russian: item.kind == .pickObject, showStress: book.settings.showStress, locked: done, neutral: item.kind == .message) { c in
                    choose(c, decision: i, item: item)
                }
            }
        }
        if !done && plays == 0 {
            Text("Play it first; then choose.").font(Theme.label(13)).foregroundStyle(Theme.muted)
        }
    }

    private func choose(_ c: Int, decision i: Int, item: ListeningItem) {
        guard !done else { return }
        let took = heardAt.map { Int(Date().timeIntervalSince($0) * 1000) }
        if item.kind == .message {
            pending[i] = c
            guard pending.count >= item.decisionCount else { return }  // all answers first, so one reveal exposes no other
            let picks = (0..<item.decisionCount).map { pending[$0] ?? -1 }
            let right = picks.enumerated().allSatisfy { $0.element == item.answer($0.offset) }
            slot.finish { r in
                r.recognized = ListeningItem.encode(picks: picks)
                r.attempts += 1
                r.status = right ? .correct : .missed
                r.responseMs = took
            }
            return
        }
        let right = c == item.answer(0)
        slot.finish { r in
            r.chosen = c
            r.attempts += 1
            r.status = right ? .correct : .missed
            r.responseMs = took
        }
    }

    // MARK: After the answer

    /// The words, at last: the transcript (lines with their badges for an exchange) with the
    /// deciding span underlined in red, why, the translation, and the glosses.
    private func reveal(_ item: ListeningItem) -> some View {
        let picks = ListeningItem.picks(slot.result)
        let missed = (0..<item.decisionCount).filter { picks[safe: $0] ?? nil != item.answer($0) }
        let spans = (missed.isEmpty ? [item.reveal(0)] : missed.map(item.reveal)).map(\.span)
        return VStack(alignment: .leading, spacing: 10) {
            Divider().overlay(Theme.rule.opacity(0.5))
            if item.kind.isExchange {
                ForEach(Array((item.lines ?? []).enumerated()), id: \.offset) { _, line in
                    HStack(alignment: .firstTextBaseline, spacing: 10) {
                        Text(line.speaker).font(.system(size: 13, weight: .bold, design: .rounded)).foregroundStyle(Theme.accent).frame(width: 22)
                        VStack(alignment: .leading, spacing: 2) {
                            transcriptLine(line.ru, spans: spans)
                            Text(line.en).scaledFont(14).foregroundStyle(Theme.muted)
                        }
                        HearButton(text: line.ru, size: 30)
                    }
                }
            } else {
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    transcriptLine(item.transcript, spans: spans)
                    if item.kind != .questionOrStatement { HearButton(text: item.transcript, size: 30) }
                }
                Text(item.en).scaledFont(15).italic().foregroundStyle(Theme.text.opacity(0.85)).fixedSize(horizontal: false, vertical: true)
            }
            ForEach(missed.isEmpty ? [0] : missed, id: \.self) { i in
                let r = item.reveal(i)
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    if !missed.isEmpty { Image(systemName: "pencil.tip").font(.system(size: 13, weight: .bold)).foregroundStyle(Theme.redPen) }
                    (Text(Theme.stressed(r.span, show: book.settings.showStress)).font(Theme.russian(17, weight: .semibold))
                     + Text(" — ").font(.system(size: 15)) + Text(Theme.stressed(r.why, show: book.settings.showStress)).font(.system(size: 15)))
                        .foregroundStyle(missed.isEmpty ? Theme.muted : Theme.text)
                        .fixedSize(horizontal: false, vertical: true)
                    if !missed.isEmpty, item.kind != .questionOrStatement {
                        Button { playOne(r.span, voice: item.voice()) } label: {
                            Image(systemName: "speaker.wave.2").font(.system(size: 13, weight: .semibold)).frame(width: 30, height: 30)
                                .background(Circle().fill(Theme.accent.opacity(0.1)))
                        }
                        .buttonStyle(.plain).foregroundStyle(Theme.accent)
                        .accessibilityLabel("Hear the span")
                    }
                }
            }
            if item.kind == .questionOrStatement, let forms = item.forms, forms.count == 2 {
                HStack(spacing: 12) {
                    ForEach(Array(forms.enumerated()), id: \.offset) { i, f in
                        Button { playOne(f.ru, voice: item.voice()) } label: {
                            HStack(spacing: 6) {
                                Image(systemName: "speaker.wave.2").font(.system(size: 12, weight: .semibold))
                                Text(Theme.stressed(f.ru, show: book.settings.showStress)).font(Theme.russian(17))
                                Text(i == 0 ? "IK-1" : "IK-3").font(Theme.label(11)).foregroundStyle(Theme.muted)
                            }
                            .padding(.horizontal, 12).padding(.vertical, 7)
                            .background(Capsule().fill(Theme.accent.opacity(0.10)))
                        }
                        .buttonStyle(.plain).foregroundStyle(Theme.accent)
                    }
                    Button { playBothForms(item) } label: { Label("Both", systemImage: "arrow.left.arrow.right").font(Theme.label(13)) }
                        .buttonStyle(.bordered).tint(Theme.accent)
                }
            }
            if !item.glosses.isEmpty {
                FlowRow(spacing: 12) {
                    ForEach(item.glosses.sorted { $0.key < $1.key }, id: \.key) { k, v in
                        (Text(k).font(Theme.russian(15, weight: .medium)) + Text(" \(v)").font(.system(size: 13)))
                            .foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
                    }
                }
            }
            HStack(spacing: 14) {
                Text(slot.result.status == .correct ? (slot.result.attempts <= 1 ? "Right first time." : "Got it.")
                     : item.kind == .message ? "Not all of it this time." : "Not this one.")
                    .font(Theme.label(14)).foregroundStyle(slot.result.status == .correct ? Theme.good : Theme.redPen)
                if slot.result.status == .missed {
                    PillButton(title: "Try it again", systemImage: "arrow.counterclockwise") { pending = [:]; plays = 0; book.reopen(slot.ref) }
                        .accessibilityHint("Practice only; it isn't graded")
                }
                if plays > 1 || slower { Text("\(plays) listen\(plays == 1 ? "" : "s")\(slower ? " · slower" : "")").font(Theme.label(12)).foregroundStyle(Theme.muted) }
            }
        }
        .transition(.opacity)
    }

    private func transcriptLine(_ text: String, spans: [String]) -> some View {
        let span = spans.first { text.contains($0) }
        return Text(span.map { Listening.highlighted(text, span: $0, showStress: book.settings.showStress) } ?? Theme.stressed(text, show: book.settings.showStress))
            .russianFont(compact ? 22 : 24, weight: .medium).foregroundStyle(Theme.text)
            .fixedSize(horizontal: false, vertical: true)
    }

    private func kindLabel(_ kind: ListeningItem.Kind) -> String {
        switch kind {
        case .pickObject: "WHICH THING"
        case .whoNeeds: "WHO NEEDS WHAT"
        case .follow: "FOLLOW THE INSTRUCTION"
        case .questionOrStatement: "ASKING OR TELLING"
        case .message: "VOICE MESSAGE"
        case .correction: "A CORRECTION"
        case .disagreement: "WHO WANTS WHAT"
        }
    }
}
