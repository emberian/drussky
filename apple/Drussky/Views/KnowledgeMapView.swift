import LearningCore
import SwiftUI
import UIKit

/// "What you know", on the inside cover: the knowledge map, growth over time, and what's next.
struct KnowledgeSection: View {
    @Environment(Book.self) private var book
    @State private var selected: ComponentID?

    var body: some View {
        let k = book.knowledge
        VStack(alignment: .leading, spacing: 14) {
            Text("What you know").font(.system(size: 22, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
            Text("Letters, words and patterns glow by how surely you'd recall them today. Red threads join things you've mixed up. Tap any for its history.")
                .font(.system(size: 15)).foregroundStyle(Theme.muted)
            KnowledgeMap(tracker: k, selected: $selected)
            nextUp(k)
            WordFamiliesSection()
            GrammarSection()
            GrowthChart(tracker: k)
        }
        .onAppear {
            // QA: -qa-node lex:кот opens that node's history.
            let args = ProcessInfo.processInfo.arguments
            if let i = args.firstIndex(of: "-qa-node"), i + 1 < args.count { selected = ComponentID(args[i + 1]) }
        }
        .sheet(item: $selected) { id in
            NavigationStack { ComponentHistoryView(id: id) }
                .environment(book)
                .presentationSizing(.page)
        }
    }

    @ViewBuilder private func nextUp(_ k: KnowledgeTracker) -> some View {
        let t = k.now()
        let next = k.model.frontier(k.graph, at: t, kinds: [.construction, .phrase, .lexeme], limit: 8)
        let pairs = k.model.contrastPairs(at: t, limit: 6)
        if !next.isEmpty || !pairs.isEmpty {
            HStack(alignment: .top, spacing: 28) {
                if !next.isEmpty {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Ready next").font(Theme.label(13)).foregroundStyle(Theme.muted)
                        FlowRow(spacing: 8) {
                            ForEach(next, id: \.self) { id in
                                Button { selected = id } label: {
                                    Text(k.label(id)).font(Theme.russian(16)).foregroundStyle(Theme.text)
                                        .padding(.horizontal, 10).padding(.vertical, 4)
                                        .overlay(Capsule().strokeBorder(Theme.accent.opacity(0.5), style: StrokeStyle(lineWidth: 1, dash: [4, 3])))
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                if !pairs.isEmpty {
                    VStack(alignment: .leading, spacing: 6) {
                        Text("Often mixed up").font(Theme.label(13)).foregroundStyle(Theme.muted)
                        ForEach(Array(pairs.enumerated()), id: \.offset) { _, p in
                            Button { selected = p.0 } label: {
                                HStack(spacing: 6) {
                                    Text(k.label(p.0)).font(Theme.russian(17))
                                    Text("↔").foregroundStyle(Theme.redPen)
                                    Text(k.label(p.1)).font(Theme.russian(17))
                                    Text("×\(Int(p.2.rounded(.up)))").font(Theme.label(12)).foregroundStyle(Theme.muted)
                                }
                                .foregroundStyle(Theme.text)
                            }
                            .buttonStyle(.plain)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
            }
        }
    }
}

/// Node positions for the map: three bands (letters, words, patterns), laid out for a width.
@MainActor
struct MapLayout {
    struct Node {
        var id: ComponentID
        var label: String
        var rect: CGRect
        var strength: Double?
        var round: Bool
    }
    struct Band { var title: String; var y: CGFloat }

    var nodes: [Node] = []
    var bands: [Band] = []
    var height: CGFloat = 0
    var index: [ComponentID: Int] = [:]

    static let wordFont = UIFont.systemFont(ofSize: 16, weight: .regular).withDesign(.serif)
    static let patternFont = UIFont.systemFont(ofSize: 13, weight: .medium)

    init(tracker k: KnowledgeTracker, width: CGFloat) {
        let t = k.now()
        let model = k.model, graph = k.graph
        var y: CGFloat = 0
        func band(_ title: String) { bands.append(Band(title: title, y: y)); y += 22 }
        func add(_ id: ComponentID, _ label: String, _ rect: CGRect, round: Bool) {
            index[id] = nodes.count
            nodes.append(Node(id: id, label: label, rect: rect, strength: model.strength(id, at: t), round: round))
        }

        band("Letters")
        y += 14  // headroom for threads arcing over the first row
        let letters = "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ".compactMap { graph[ComponentID(.letter, String($0))] }
        let cols = max(1, min(17, Int(width / 50)))
        let cell = min(48, (width - CGFloat(cols - 1) * 6) / CGFloat(cols))
        for (i, c) in letters.enumerated() {
            let r = CGRect(x: CGFloat(i % cols) * (cell + 6), y: y + CGFloat(i / cols) * (cell + 6), width: cell, height: cell)
            add(c.id, String(c.label.prefix(1)), r, round: true)
        }
        y += CGFloat((letters.count + cols - 1) / cols) * (cell + 6) + 14

        // Words: everything practised, plus forms caught up in a confusion.
        let confused = Set(model.confusionEdges(at: t).flatMap { [$0.from, $0.to] })
        let words = graph.components.values.filter { c in
            switch c.id.kind {
            case .lexeme, .phrase: model.hasEvidence(c.id)
            case .form: confused.contains(c.id)
            default: false
            }
        }.sorted { ($0.order, $0.id) < ($1.order, $1.id) }
        band(words.isEmpty ? "Words — none yet" : "Words · \(words.count)")
        flow(words.map { ($0.id, $0.id.kind == .phrase ? $0.label : $0.label) }, font: Self.wordFont, height: 28,
             width: width, y: &y, add: add)
        y += 14

        band("Patterns & sounds")
        let patterns = graph.components(of: .construction).filter { $0.unit != "Grammar" } + graph.components(of: .sound)  // table cells are drawn by GrammarSection
        flow(patterns.map { ($0.id, $0.label) }, font: Self.patternFont, height: 26, width: width, y: &y, add: add)
        height = y + 4
    }

    private func flow(_ items: [(ComponentID, String)], font: UIFont, height h: CGFloat, width: CGFloat,
                      y: inout CGFloat, add: (ComponentID, String, CGRect, Bool) -> Void) {
        var x: CGFloat = 0
        for (id, label) in items {
            let w = ceil((label as NSString).size(withAttributes: [.font: font]).width) + 18
            if x > 0 && x + w > width { x = 0; y += h + 6 }
            add(id, label, CGRect(x: x, y: y, width: min(w, width), height: h), false)
            x += w + 6
        }
        if !items.isEmpty { y += h }
    }

    func node(at p: CGPoint) -> ComponentID? {
        nodes.first { $0.rect.insetBy(dx: -3, dy: -3).contains(p) }?.id
    }

    /// Where an edge endpoint is drawn: the node itself, or (for an undrawn form) its lexeme.
    func anchor(_ id: ComponentID, graph: KnowledgeGraph) -> Node? {
        if let i = index[id] { return nodes[i] }
        if let lex = graph.edges(from: id, .formOf).first?.to, let i = index[lex] { return nodes[i] }
        return nil
    }
}

extension UIFont {
    func withDesign(_ design: UIFontDescriptor.SystemDesign) -> UIFont {
        fontDescriptor.withDesign(design).map { UIFont(descriptor: $0, size: pointSize) } ?? self
    }
}

struct KnowledgeMap: View {
    let tracker: KnowledgeTracker
    @Binding var selected: ComponentID?
    @State private var width: CGFloat = 800

    var body: some View {
        let layout = MapLayout(tracker: tracker, width: width)
        let t = tracker.now()
        let edges = tracker.model.confusionEdges(at: t)
        Canvas { ctx, _ in
            draw(layout: layout, edges: edges, in: &ctx)
        }
        .frame(height: layout.height)
        .frame(maxWidth: .infinity)
        .onGeometryChange(for: CGFloat.self) { $0.size.width } action: { width = $0 }
        .contentShape(Rectangle())
        .onTapGesture(coordinateSpace: .local) { p in
            if let id = layout.node(at: p) { selected = id }
        }
        // The canvas has no elements of its own: VoiceOver gets the same nodes as a list of
        // buttons, each with its recall, opening the same history.
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Knowledge map: \(layout.nodes.filter { ($0.strength ?? 0) >= 0.5 }.count) things you can recall now")
        .accessibilityChildren {
            ForEach(layout.nodes, id: \.id) { node in
                Button(node.label) { selected = node.id }
                    .accessibilityValue(node.strength.map { "recall \(Int(($0 * 100).rounded())) percent" } ?? "not practised")
            }
        }
    }

    private func draw(layout: MapLayout, edges: [LearningCore.Edge], in ctx: inout GraphicsContext) {
        for band in layout.bands {
            ctx.draw(Text(band.title.uppercased()).font(.system(size: 11, weight: .semibold, design: .rounded)).foregroundStyle(Theme.muted),
                     at: CGPoint(x: 0, y: band.y + 7), anchor: .leading)
        }
        // Glow under strong nodes.
        ctx.drawLayer { glow in
            glow.addFilter(.blur(radius: 6))
            for n in layout.nodes {
                guard let s = n.strength, s >= 0.75 else { continue }
                let path = shape(n).path(in: n.rect.insetBy(dx: -2, dy: -2))
                glow.fill(path, with: .color(Theme.good.opacity(0.22 * (s - 0.6) / 0.4)))
            }
        }
        // Confusion threads.
        for e in edges {
            guard let a = layout.anchor(e.from, graph: tracker.graph), let b = layout.anchor(e.to, graph: tracker.graph),
                  a.id != b.id else { continue }
            let p = CGPoint(x: a.rect.midX, y: a.rect.minY + 2), q = CGPoint(x: b.rect.midX, y: b.rect.minY + 2)
            let mid = CGPoint(x: (p.x + q.x) / 2, y: (p.y + q.y) / 2)
            let d = hypot(q.x - p.x, q.y - p.y)
            // Bow upward from the tops of its ends; nodes are drawn over the thread.
            let bend = min(46, 10 + d * 0.22)
            var normal = d > 0 ? CGPoint(x: -(q.y - p.y) / d, y: (q.x - p.x) / d) : .zero
            if normal.y > 0 { normal = CGPoint(x: -normal.x, y: -normal.y) }
            var path = Path()
            path.move(to: p)
            path.addQuadCurve(to: q, control: CGPoint(x: mid.x + normal.x * bend, y: mid.y + normal.y * bend))
            let w = 1 + 1.5 * min(e.weight, 3) / 3
            ctx.stroke(path, with: .color(Theme.redPen.opacity(0.45 + 0.3 * min(e.weight, 3) / 3)), lineWidth: w)
        }
        let selectedEdges = selected.map { id in tracker.graph.edges(from: id) + tracker.graph.edges(to: id) } ?? []
        for e in selectedEdges where e.kind != .confusedWith {
            guard let a = layout.anchor(e.from, graph: tracker.graph), let b = layout.anchor(e.to, graph: tracker.graph) else { continue }
            var path = Path()
            path.move(to: CGPoint(x: a.rect.midX, y: a.rect.midY))
            path.addLine(to: CGPoint(x: b.rect.midX, y: b.rect.midY))
            ctx.stroke(path, with: .color(Theme.accent.opacity(0.35)), style: StrokeStyle(lineWidth: 1, dash: [3, 3]))
        }
        for n in layout.nodes {
            let path = shape(n).path(in: n.rect)
            if let s = n.strength {
                ctx.fill(path, with: .color(Theme.card))
                ctx.fill(path, with: .color(Theme.accent.opacity(0.08 + 0.42 * s)))
                ctx.stroke(path, with: .color(Theme.accent.opacity(0.25 + 0.5 * s)), lineWidth: n.id == selected ? 2 : 1)
            } else {
                ctx.stroke(path, with: .color(Theme.muted.opacity(0.28)), lineWidth: 1)
            }
            let color: Color = n.strength == nil ? Theme.muted.opacity(0.55) : Theme.text
            let font: Font = switch n.id.kind {
            case .letter: Theme.russian(min(24, n.rect.height * 0.5), weight: .medium)
            case .construction, .sound: .system(size: 13, weight: .medium)
            default: Theme.russian(16)
            }
            ctx.draw(Text(n.label).font(font).foregroundStyle(color), at: CGPoint(x: n.rect.midX, y: n.rect.midY))
        }
    }

    private func shape(_ n: MapLayout.Node) -> AnyShape {
        n.round ? AnyShape(Circle()) : AnyShape(RoundedRectangle(cornerRadius: n.rect.height / 2))
    }
}

struct ReadabilityBadge: View {
    let fraction: Double
    var body: some View {
        VStack(alignment: .trailing, spacing: 0) {
            Text("\(Int((fraction * 100).rounded()))%").font(.system(size: 17, weight: .semibold, design: .rounded))
                .monospacedDigit().foregroundStyle(Theme.text)
            Text("you know").font(.system(size: 11)).foregroundStyle(Theme.muted)
        }
        .accessibilityElement(children: .combine)
    }
}
