import SwiftUI

/// Under a take: the model and the learner syllable by syllable (length as width, prominence as
/// depth of colour, the melody drawn inside), tap any syllable to hear just that part; A/B
/// playback; and at most two things to try, each with a page to practise it.
struct SpeakingReportView: View {
    @Environment(Voice.self) private var voice
    @Environment(Book.self) private var book
    let report: PronunciationReport
    let target: String
    let modelURL: URL?
    let takeURL: URL
    let version: Int
    let practise: (String) -> Void
    @State private var comparison: ProsodyComparison?

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            if let comparison, comparison.take.count == comparison.syllables.count {
                SyllableGrid(comparison: comparison, modelURL: modelURL, takeURL: takeURL, showStress: book.settings.showStress)
            }
            HStack(spacing: 8) {
                if let modelURL { chip("Model", "speaker.wave.2.fill", key: "ab-model-\(takeURL.lastPathComponent)") { voice.playFile(modelURL, key: $0) } }
                chip("You", "person.wave.2.fill", key: "ab-you-\(takeURL.lastPathComponent)") { voice.playFile(takeURL, key: $0) }
                if let modelURL {
                    chip("Model, then you", "arrow.left.arrow.right", key: "ab-both-\(takeURL.lastPathComponent)") { key in
                        Task {
                            guard await voice.playFileToEnd(modelURL, key: key) else { return }
                            try? await Task.sleep(for: .milliseconds(250))
                            voice.playFile(takeURL, key: key)
                        }
                    }
                }
            }
            if !report.tips.isEmpty {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(Array(report.tips.prefix(2).enumerated()), id: \.offset) { _, tip in tipRow(tip) }
                }
            } else if comparison?.model != nil, comparison?.stressMisses.isEmpty == true {
                Label("Stress and rhythm follow the model.", systemImage: "checkmark")
                    .scaledFont(14, weight: .medium).foregroundStyle(Theme.good)
            }
        }
        .padding(12)
        .frame(maxWidth: 640, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
        .task(id: "\(takeURL.lastPathComponent)#\(version)") { await load() }
    }

    private func tipRow(_ tip: PronunciationTip) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(tip.message).scaledFont(15).foregroundStyle(Theme.text).fixedSize(horizontal: false, vertical: true)
            if let c = tip.contrast.flatMap(SoundContrasts.id(for:)), let name = SoundContrasts.name(c) {
                Button { practise(c) } label: {
                    Label("Practise \(name.lowercased())", systemImage: "ear.badge.waveform").font(Theme.label(13))
                }
                .buttonStyle(.plain).foregroundStyle(Theme.accent)
            }
        }
    }

    private func chip(_ title: String, _ icon: String, key: String, play: @escaping (String) -> Void) -> some View {
        let playing = voice.speakingKey == key
        return Button { playing ? voice.stop() : play(key) } label: {
            Label(title, systemImage: playing ? "stop.fill" : icon).font(Theme.label(13))
                .padding(.horizontal, 12).padding(.vertical, 7)
                .foregroundStyle(Theme.accent)
                .background(Capsule().fill(Theme.accent.opacity(playing ? 0.2 : 0.1)))
        }
        .buttonStyle(.plain)
    }

    private func load() async {
        let target = self.target, takeURL = self.takeURL, modelURL = self.modelURL
        // The analyzer's own syllable times for the take, when it gave them for every syllable.
        let spans: [ClosedRange<Double>]? = {
            let s = report.syllables.compactMap { j -> ClosedRange<Double>? in
                guard let a = j.start, let b = j.end, b > a else { return nil }
                return a...b
            }
            return s.count == report.syllables.count && !s.isEmpty ? s : nil
        }()
        comparison = await Task.detached(priority: .userInitiated) { () -> ProsodyComparison? in
            guard let x = Pitch.samples(takeURL) else { return nil }
            return ProsodyComparison.compute(target: target, take: x, model: modelURL.flatMap(Pitch.samples), takeSpans: spans)
        }.value
    }
}

/// Syllables in columns: the text, the model's syllable, the learner's. Width is length,
/// colour depth is prominence, the line inside is pitch. The written stress is in blue; a
/// learner's syllable carrying the stress that belongs elsewhere is ringed in red.
struct SyllableGrid: View {
    @Environment(Voice.self) private var voice
    let comparison: ProsodyComparison
    let modelURL: URL?
    let takeURL: URL
    let showStress: Bool

    private static let perSecond: CGFloat = 230
    private func width(_ s: SpokenSyllable?) -> CGFloat { max(30, min(150, CGFloat((s?.end ?? 0) - (s?.start ?? 0)) * Self.perSecond)) }

    var body: some View {
        let misplaced = Set(comparison.stressMisses.map(\.heard))
        HStack(alignment: .top, spacing: 8) {
            VStack(alignment: .leading, spacing: 4) {
                Text(" ").russianFont(19).hidden()
                if comparison.model != nil {
                    Text("Model").font(Theme.label(11)).foregroundStyle(Theme.muted).frame(height: 34)
                }
                Text("You").font(Theme.label(11)).foregroundStyle(Theme.muted).frame(height: 34)
            }
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(alignment: .top, spacing: 3) {
                    ForEach(comparison.syllables.indices, id: \.self) { i in
                        let syl = comparison.syllables[i]
                        let gap = i > 0 && comparison.syllables[i - 1].word != syl.word
                        let w = max(width(comparison.model?[i]), width(comparison.take[i]))
                        VStack(alignment: .leading, spacing: 4) {
                            Text(Theme.stressed(syl.text.trimmingCharacters(in: .whitespaces), show: showStress && syl.isStressed))
                                .russianFont(19).foregroundStyle(Theme.text).lineLimit(1).fixedSize()
                            if let model = comparison.model, let modelURL {
                                box(model[i], url: modelURL, key: "syl-m-\(takeURL.lastPathComponent)-\(i)", color: Theme.accent,
                                    stressed: syl.isStressed, misplaced: false)
                            }
                            box(comparison.take[i], url: takeURL, key: "syl-y-\(takeURL.lastPathComponent)-\(i)", color: Theme.ink,
                                stressed: syl.isStressed, misplaced: misplaced.contains(i))
                        }
                        .frame(width: w, alignment: .leading)
                        .padding(.leading, gap ? 10 : 0)
                    }
                }
            }
        }
        .accessibilityElement(children: .contain)
    }

    private func box(_ s: SpokenSyllable, url: URL, key: String, color: Color, stressed: Bool, misplaced: Bool) -> some View {
        let playing = voice.speakingKey == key
        return Button {
            voice.playSegment(url, key: key, from: max(0, s.start - 0.03), to: s.end + 0.04)
        } label: {
            Canvas { ctx, size in
                let mid = size.height / 2
                let scale = (size.height / 2 - 3) / 9
                var path = Path()
                var open = false
                for (k, v) in s.trace.enumerated() {
                    guard let v else { open = false; continue }
                    let p = CGPoint(x: 4 + (size.width - 8) * CGFloat(k) / 7, y: mid - CGFloat(max(-9, min(9, v))) * scale)
                    if open { path.addLine(to: p) } else { path.move(to: p); open = true }
                }
                ctx.stroke(path, with: .color(color.opacity(0.9)), style: StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
            }
            .frame(width: width(s), height: 34)
            .background(RoundedRectangle(cornerRadius: 7).fill(color.opacity(0.08 + 0.42 * s.prominence)))
            .overlay(RoundedRectangle(cornerRadius: 7)
                .stroke(misplaced ? Theme.redPen : (playing ? Theme.accent : .clear), lineWidth: misplaced || playing ? 2 : 0))
        }
        .buttonStyle(.plain)
        .accessibilityLabel(stressed ? "stressed syllable" : "syllable")
        .accessibilityHint("Plays this syllable")
    }
}
