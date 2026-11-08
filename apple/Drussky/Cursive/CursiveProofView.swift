import SwiftUI

/// The cursive models laid out as a proof sheet: every letter form on its ruled line, then
/// words written through, with the joins in red. Opened by `-qa-cursive-proof`.
struct CursiveProofView: View {
    static let words = ["мама", "молоко", "шишка", "лимон", "тишина", "дом", "будет", "вода", "где", "жук", "зима", "кот",
                        "окно", "папа", "ракета", "сестра", "утро", "кофе", "хлеб", "цирк", "чашка", "щи", "объект", "сыр",
                        "мать", "это", "юг", "я тут", "Анна и Том", "Это Москва."]
    static let alphabet = "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ"

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                Text("Пропись · the cursive models").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                if let m = CursiveModels.shared {
                    let rows = Self.alphabet.map { String($0) }.chunked(6) + Self.alphabet.lowercased().map { String($0) }.chunked(6)
                    ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                        ProofLine(texts: row, models: m, cell: 100)
                    }
                    ForEach(Array(Self.words.chunked(3).enumerated()), id: \.offset) { _, row in
                        ProofLine(texts: row, models: m, cell: 300)
                    }
                } else {
                    Text("No cursive models in the bundle.").foregroundStyle(Theme.redPen)
                }
            }
            .padding(20)
        }
        .background(Theme.paper)
        .accessibilityIdentifier("cursive-proof")
    }
}

private struct ProofLine: View {
    let texts: [String]
    let models: CursiveModels
    let cell: CGFloat
    private let height: CGFloat = 96

    var body: some View {
        Canvas { ctx, size in
            let k = height / StripMetrics.designHeight
            for (y, alpha) in [(models.lines.cap, 0.4), (models.lines.xHeight, 0.5), (models.lines.baseline, 0.9), (models.lines.descender, 0.3)] {
                var p = Path()
                p.move(to: CGPoint(x: 0, y: y * k)); p.addLine(to: CGPoint(x: size.width, y: y * k))
                ctx.stroke(p, with: .color(Theme.rule.opacity(alpha)), style: StrokeStyle(lineWidth: y == models.lines.baseline ? 1.2 : 0.7, dash: y == models.lines.xHeight ? [4, 4] : []))
            }
            var x: CGFloat = 8
            for text in texts {
                for item in CursiveLayout.place(text, in: models, x0: 0).items {
                    switch item {
                    case .join(let pts):
                        var p = Path()
                        p.addLines(pts.map { CGPoint(x: x + $0.x * k, y: $0.y * k) })
                        ctx.stroke(p, with: .color(Theme.redPen.opacity(0.8)), style: StrokeStyle(lineWidth: 2 * k, lineCap: .round, lineJoin: .round))
                    case .letter(let l, let lx):
                        for s in l.model.strokes {
                            var p = Path()
                            p.addLines(s.points.map { CGPoint(x: x + (lx + $0.x) * k, y: $0.y * k) })
                            ctx.stroke(p, with: .color(Theme.ink), style: StrokeStyle(lineWidth: 3 * k, lineCap: .round, lineJoin: .round))
                            if let first = s.points.first {
                                let r = 2.4 * k
                                ctx.fill(Path(ellipseIn: CGRect(x: x + (lx + first.x) * k - r, y: first.y * k - r, width: 2 * r, height: 2 * r)), with: .color(Theme.redPen))
                            }
                        }
                    }
                }
                x += cell * k
            }
        }
        .frame(height: height)
    }
}

extension Array {
    func chunked(_ n: Int) -> [[Element]] {
        stride(from: 0, to: count, by: n).map { Array(self[$0..<Swift.min($0 + n, count)]) }
    }
}
