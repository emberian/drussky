import AVFoundation
import SwiftUI

/// Loudness over time, silence trimmed: enough to compare the rhythm and stress of two takes.
enum Envelope {
    static let binsPerSecond = 40.0

    static func compute(_ url: URL) async -> [Float]? {
        await Task.detached(priority: .userInitiated) { () -> [Float]? in
            guard let file = try? AVAudioFile(forReading: url),
                  let buffer = AVAudioPCMBuffer(pcmFormat: file.processingFormat, frameCapacity: AVAudioFrameCount(file.length)),
                  (try? file.read(into: buffer)) != nil, let data = buffer.floatChannelData?[0] else { return nil }
            let n = Int(buffer.frameLength)
            let hop = max(1, Int(file.processingFormat.sampleRate / binsPerSecond))
            var bins: [Float] = []
            var i = 0
            while i < n {
                let end = min(n, i + hop)
                var sum: Float = 0
                for j in i..<end { sum += data[j] * data[j] }
                bins.append(sqrt(sum / Float(end - i)))
                i = end
            }
            guard let peak = bins.max(), peak > 0 else { return nil }
            let norm = bins.map { $0 / peak }
            guard let first = norm.firstIndex(where: { $0 > 0.08 }), let last = norm.lastIndex(where: { $0 > 0.08 }) else { return nil }
            return Array(norm[first...last])
        }.value
    }
}

/// Model above, the learner's take below, on the same time scale.
struct RhythmView: View {
    let modelURL: URL?
    let mineURL: URL
    let version: Int
    @State private var model: [Float]?
    @State private var mine: [Float]?

    var body: some View {
        Group {
            if let model, let mine {
                let longest = max(model.count, mine.count, 1)
                VStack(alignment: .leading, spacing: 6) {
                    row(model, of: longest, label: "Model", color: Theme.accent.opacity(0.75))
                    row(mine, of: longest, label: "You", color: Theme.ink)
                    Text(hint(model: model, mine: mine))
                        .font(.system(size: 13)).foregroundStyle(Theme.muted)
                }
                .padding(12)
                .frame(maxWidth: 560, alignment: .leading)
                .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
            }
        }
        .task(id: version) {
            if let modelURL { model = await Envelope.compute(modelURL) }
            mine = await Envelope.compute(mineURL)
        }
    }

    private func row(_ env: [Float], of longest: Int, label: String, color: Color) -> some View {
        HStack(spacing: 10) {
            Text(label).font(Theme.label(11)).foregroundStyle(Theme.muted).frame(width: 44, alignment: .leading)
            GeometryReader { g in
                Canvas { ctx, size in
                    let w = size.width / CGFloat(longest)
                    let mid = size.height / 2
                    var path = Path()
                    for (i, v) in env.enumerated() {
                        let h = max(1, CGFloat(v) * mid)
                        path.addRoundedRect(in: CGRect(x: CGFloat(i) * w, y: mid - h, width: max(1, w * 0.7), height: h * 2),
                                            cornerSize: CGSize(width: 1, height: 1))
                    }
                    ctx.fill(path, with: .color(color))
                }
                .frame(width: g.size.width, height: g.size.height)
            }
            .frame(height: 34)
        }
    }

    private func hint(model: [Float], mine: [Float]) -> String {
        let ratio = Double(mine.count) / Double(max(model.count, 1))
        if ratio > 1.45 { return "Yours is slower — fine for now; aim for the model's shape." }
        if ratio < 0.7 { return "Yours is quicker than the model; give the stressed vowel its length." }
        return "Similar length. Look for the loudest, longest part — that's the stressed syllable."
    }
}
