import LearningCore
import SwiftUI

/// One node of the knowledge map: recall per modality, every attempt, what it's confused with,
/// and what it connects to.
struct ComponentHistoryView: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    let id: ComponentID

    var body: some View {
        let k = book.knowledge
        let t = k.now()
        let c = k.graph[id]
        let history = k.model.history(of: id)
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                header(c, k)
                modalities(k, t)
                if !history.isEmpty { timeline(history) }
                confusions(k, t)
                connections(k)
                if !history.isEmpty { attempts(history, k) }
            }
            .padding(24)
            .frame(maxWidth: 760, alignment: .leading)
            .frame(maxWidth: .infinity)
        }
        .background(GridPaper(marginX: nil))
        .navigationTitle(k.label(id))
        .navigationBarTitleDisplayMode(.inline)
        .navigationDestination(for: ComponentID.self) { ComponentHistoryView(id: $0) }
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
    }

    private var kindName: String {
        switch id.kind {
        case .letter: "letter"
        case .lexeme: "word"
        case .form: "word form"
        case .phrase: "phrase"
        case .construction: "grammar pattern"
        case .sound: "sound contrast"
        default: id.kind.rawValue
        }
    }

    private func header(_ c: Component?, _ k: KnowledgeTracker) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 14) {
            Text(c?.label ?? id.key).font(Theme.russian(id.kind == .letter ? 54 : 38, weight: .semibold)).foregroundStyle(Theme.text)
            if [.lexeme, .form, .phrase].contains(id.kind) { HearButton(text: c?.label ?? id.key, size: 36) }
            VStack(alignment: .leading, spacing: 2) {
                Text(kindName.uppercased()).font(Theme.label(11)).foregroundStyle(Theme.muted)
                if let g = c?.gloss { Text(g).font(.system(size: 16)).foregroundStyle(Theme.text) }
                if let u = c?.unit { Text(u).font(.system(size: 13)).foregroundStyle(Theme.muted) }
            }
        }
    }

    private func modalities(_ k: KnowledgeTracker, _ t: Date) -> some View {
        let traces = k.model.traces(of: id)
        let shown = k.domain.modalities.filter { m in traces[m] != nil || k.model.estimatedRetrievability(id, m, at: t) != nil }
        return VStack(alignment: .leading, spacing: 10) {
            sectionTitle("Recall today")
            if shown.isEmpty {
                Text("Not practised yet.").font(.system(size: 15)).foregroundStyle(Theme.muted)
            }
            ForEach(shown, id: \.self) { m in
                let est = k.model.estimatedRetrievability(id, m, at: t)
                let trace = traces[m]
                let title = Text(m.title).scaledFont(14, weight: .medium).foregroundStyle(Theme.text)
                let bar = RecallBar(value: est?.value ?? 0, estimated: est?.estimated ?? true).frame(height: 10)
                let percent = Text(est.map { "\(Int(($0.value * 100).rounded()))%" } ?? "–")
                    .scaledFont(14, weight: .semibold, design: .rounded).monospacedDigit()
                    .foregroundStyle(est?.estimated == true ? Theme.muted : Theme.text)
                let detail = Group {
                    if let trace, let s = trace.state {
                        Text("holds \(Self.days(s.stability)) · \(trace.reviews.count)× · \(trace.lapses) lapse\(trace.lapses == 1 ? "" : "s")")
                    } else {
                        Text("estimated from other ways")
                    }
                }
                .scaledFont(12).foregroundStyle(Theme.muted)
                // Columns when they fit; stacked when the sheet is narrow (iPhone, Split View) or
                // the text is large.
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 12) {
                        title.lineLimit(1).fixedSize().frame(minWidth: 130, alignment: .leading)
                        bar.frame(minWidth: 80)
                        percent.lineLimit(1).fixedSize().frame(minWidth: 44, alignment: .trailing)
                        detail.lineLimit(1).fixedSize().frame(minWidth: 190, alignment: .leading)
                    }
                    VStack(alignment: .leading, spacing: 4) {
                        HStack { title; Spacer(minLength: 8); percent }
                        bar
                        detail.fixedSize(horizontal: false, vertical: true)
                    }
                }
                .accessibilityElement(children: .combine)
            }
        }
    }

    static func days(_ s: Double) -> String {
        s < 1 ? "\(max(1, Int((s * 24).rounded())))h" : s < 60 ? "\(Int(s.rounded()))d" : "\(Int((s / 30).rounded()))mo"
    }

    /// Every outcome on a time axis: green right, amber hard, red missed; small grey ticks for tries.
    private func timeline(_ events: [LearningEvent]) -> some View {
        let first = events.first!.t, last = max(book.knowledge.now(), first.addingTimeInterval(86_400))
        return VStack(alignment: .leading, spacing: 6) {
            sectionTitle("Over time")
            Canvas { ctx, size in
                let span = last.timeIntervalSince(first)
                var base = Path()
                base.move(to: CGPoint(x: 0, y: size.height / 2))
                base.addLine(to: CGPoint(x: size.width, y: size.height / 2))
                ctx.stroke(base, with: .color(Theme.rule.opacity(0.3)), lineWidth: 1)
                for e in events {
                    let x = 6 + (size.width - 12) * e.t.timeIntervalSince(first) / span
                    switch e.type {
                    case .outcome:
                        let color: Color = switch e.grade {
                        case .again?: Theme.redPen
                        case .hard?: Color(Theme.dynamic(0xB7791F, 0xE0A84A))
                        case nil: Theme.muted  // finished, not graded (skipped, or rewritten after being shown)
                        default: Theme.good
                        }
                        ctx.fill(Path(ellipseIn: CGRect(x: x - 5, y: size.height / 2 - 5, width: 10, height: 10)), with: .color(color))
                    case .attempt:
                        var tick = Path()
                        tick.move(to: CGPoint(x: x, y: size.height / 2 - 8))
                        tick.addLine(to: CGPoint(x: x, y: size.height / 2 + 8))
                        ctx.stroke(tick, with: .color(Theme.muted.opacity(0.6)), lineWidth: 1.5)
                    case .help:
                        ctx.draw(Text("?").font(Theme.label(11)).foregroundStyle(Theme.accent), at: CGPoint(x: x, y: size.height / 2 - 12))
                    }
                }
            }
            .frame(height: 34)
            HStack {
                Text(first, format: .dateTime.month().day())
                Spacer()
                Text("today")
            }
            .font(.system(size: 11)).foregroundStyle(Theme.muted)
        }
    }

    @ViewBuilder private func confusions(_ k: KnowledgeTracker, _ t: Date) -> some View {
        let stats = k.model.confusions.values.filter { $0.target == id || $0.produced == id }
            .sorted { $0.weight(at: t) > $1.weight(at: t) }
        if !stats.isEmpty {
            VStack(alignment: .leading, spacing: 8) {
                sectionTitle("Mixed up with")
                ForEach(Array(stats.enumerated()), id: \.offset) { _, s in
                    let other = s.target == id ? s.produced : s.target
                    NavigationLink(value: other) {
                        HStack(spacing: 8) {
                            Text(s.target == id ? "gave" : "wanted").font(.system(size: 13)).foregroundStyle(Theme.muted)
                                .frame(width: 50, alignment: .leading)
                            Text(k.label(other)).font(Theme.russian(19, weight: .medium)).foregroundStyle(Theme.text)
                            Text(s.target == id ? "for this" : "but gave this").font(.system(size: 13)).foregroundStyle(Theme.muted)
                            Spacer()
                            Text("×\(s.count) · \(s.channels.map(\.rawValue).sorted().joined(separator: ", "))")
                                .font(.system(size: 12)).foregroundStyle(Theme.muted)
                        }
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    @ViewBuilder private func connections(_ k: KnowledgeTracker) -> some View {
        let g = k.graph
        let groups: [(String, [ComponentID])] = [
            ("Lemma", g.edges(from: id, .formOf).map(\.to)),
            ("Forms", g.edges(to: id, .formOf).map(\.from)),
            ("Part of", g.edges(from: id, .partOf).map(\.to)),
            ("Made of", g.edges(to: id, .partOf).map(\.from)),
            ("Patterns", g.edges(from: id, .exemplifies).map(\.to)),
            ("Examples", g.edges(to: id, .exemplifies).map(\.from).filter { k.model.hasEvidence($0) }),
            ("Needs first", g.edges(to: id, .prerequisite).map(\.from)),
            ("Leads to", g.edges(from: id, .prerequisite).map(\.to)),
            ("Letters", g.edges(from: id, .uses).map(\.to)),
            ("Words using it", id.kind == .letter ? g.edges(to: id, .uses).map(\.from).filter { k.model.hasEvidence($0) } : []),
        ].filter { !$0.1.isEmpty }
        if !groups.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
                sectionTitle("Connected to")
                ForEach(groups, id: \.0) { title, ids in
                    HStack(alignment: .firstTextBaseline, spacing: 10) {
                        Text(title).font(.system(size: 13)).foregroundStyle(Theme.muted).frame(width: 110, alignment: .leading)
                        FlowRow(spacing: 6) {
                            ForEach(ids.prefix(40), id: \.self) { other in
                                NavigationLink(value: other) {
                                    Text(k.label(other)).font(Theme.russian(16)).foregroundStyle(Theme.text)
                                        .padding(.horizontal, 9).padding(.vertical, 3)
                                        .background(Capsule().fill(Theme.accent.opacity(0.06 + 0.3 * (k.strength(other) ?? 0))))
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }
            }
        }
    }

    private func attempts(_ events: [LearningEvent], _ k: KnowledgeTracker) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            sectionTitle("Attempts")
            ForEach(events.suffix(30).reversed(), id: \.id) { e in
                HStack(alignment: .firstTextBaseline, spacing: 10) {
                    Text(e.t, format: .dateTime.month().day().hour().minute())
                        .font(.system(size: 12)).monospacedDigit().foregroundStyle(Theme.muted).frame(width: 110, alignment: .leading)
                    Text(outcome(e)).font(Theme.label(12)).foregroundStyle(color(e)).frame(width: 60, alignment: .leading)
                    Text([e.modality?.title, e.activity?.replacingOccurrences(of: "_", with: " ")].compactMap { $0 }.joined(separator: " · "))
                        .font(.system(size: 13)).foregroundStyle(Theme.text)
                    if let r = e.response, !r.isEmpty, e.type != .help {
                        Text("«\(r)»").font(Theme.russian(15)).foregroundStyle(e.grade == .good ? Theme.muted : Theme.redPen).lineLimit(1)
                    }
                    Spacer()
                    if e.source == "backfill" { Text("from your pages").font(.system(size: 11)).foregroundStyle(Theme.muted) }
                }
            }
        }
    }

    private func outcome(_ e: LearningEvent) -> String {
        switch e.type {
        case .help: "asked"
        case .attempt: "try"
        case .outcome:
            switch e.grade {
            case .again?: "missed"
            case .hard?: "hard"
            case .easy?: "easy"
            default: "right"
            }
        }
    }

    private func color(_ e: LearningEvent) -> Color {
        switch (e.type, e.grade) {
        case (.outcome, .again?), (.attempt, _): Theme.redPen
        case (.outcome, .hard?): Theme.muted
        case (.outcome, _): Theme.good
        default: Theme.accent
        }
    }

    private func sectionTitle(_ s: String) -> some View {
        Text(s).font(.system(size: 17, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
    }
}

struct RecallBar: View {
    let value: Double
    let estimated: Bool
    var body: some View {
        GeometryReader { geo in
            ZStack(alignment: .leading) {
                Capsule().fill(Theme.rule.opacity(0.12))
                if estimated {
                    Capsule().strokeBorder(Theme.accent.opacity(0.6), style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
                        .frame(width: max(10, geo.size.width * value))
                } else {
                    Capsule().fill(Theme.accent.opacity(0.35 + 0.55 * value)).frame(width: max(4, geo.size.width * value))
                }
            }
        }
    }
}
