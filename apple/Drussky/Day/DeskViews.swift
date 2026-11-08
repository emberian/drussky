import Charts
import LearningCore
import SwiftUI

/// One calm line at the foot of a page the desk shortened.
struct DeskLine: View {
    @Environment(Book.self) private var book
    let pageIndex: Int

    var body: some View {
        if let line = book.deskLine(for: pageIndex) {
            Label(line, systemImage: "moon.zzz")
                .scaledFont(15)
                .foregroundStyle(Theme.muted)
                .fixedSize(horizontal: false, vertical: true)
                .frame(maxWidth: 640, alignment: .leading)
                .accessibilityLabel(line)
        }
    }
}

/// Inside the cover: today's one line, and how well the book's predictions have held up.
struct DeskNotes: View {
    @Environment(Book.self) private var book

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            if let line = book.coverLine {
                Text(line).scaledFont(16).foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
            }
            FluencyNotes()  // Fluency/
            RetentionChart(book: book)
        }
    }
}

/// "How well the book knows you": predicted recall against what happened on the probes, one
/// reliability curve per way of using a word. On the diagonal, the book predicts you exactly;
/// above it you remember more than it thinks; below, less.
struct RetentionChart: View {
    let book: Book

    /// Observations folded onto the growth groups (four series, four validated colours).
    static func grouped(_ observations: [Calibration.Observation]) -> Calibration {
        let groups = KnowledgeTracker.growthGroups
        let folded = observations.compactMap { o -> Calibration.Observation? in
            guard let g = groups.first(where: { $0.modalities.contains(o.modality) }) else { return nil }
            return Calibration.Observation(modality: Modality(rawValue: g.title), predicted: o.predicted, observed: o.observed)
        }
        return Calibration(observations: folded, binCount: 4)
    }

    struct Point: Identifiable {
        var series: Int
        var title: String
        var predicted: Double
        var observed: Double
        var count: Int
        var id: String { "\(series)-\(predicted)" }
    }

    var body: some View {
        let observations = book.day.probes.observations
        let calibration = Self.grouped(observations)
        let titles = KnowledgeTracker.growthGroups.map(\.title)
        let points = calibration.series.flatMap { s in
            s.bins.map { Point(series: titles.firstIndex(of: s.modality?.rawValue ?? "") ?? 0, title: s.modality?.rawValue ?? "",
                               predicted: $0.predicted, observed: $0.observed, count: $0.count) }
        }
        VStack(alignment: .leading, spacing: 8) {
            Text("Review predictions and observed answers").font(Theme.label(13)).foregroundStyle(Theme.muted)
            if let all = calibration.overall, all.count >= 4 {
                Text(summary(all)).scaledFont(15).foregroundStyle(Theme.text)
                    .fixedSize(horizontal: false, vertical: true)
                HStack(alignment: .top, spacing: 18) {
                    chart(points).frame(width: 210, height: 170).padding(.trailing, 10)
                    legend(calibration, titles: titles)
                }
            } else {
                Text(observations.isEmpty
                     ? "Now and then the book slips in a few things it believes you know, one, seven and thirty days on, and checks itself against what you actually recall. The chart appears once a handful have come back."
                     : "\(observations.count) probe\(observations.count == 1 ? "" : "s") answered so far — a few more and the chart appears.")
                    .scaledFont(14).foregroundStyle(Theme.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(16)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
        .accessibilityElement(children: .combine)
        .accessibilityLabel(calibration.overall.map(summary) ?? "No retention probes answered yet")
    }

    private func pct(_ x: Double) -> String { "\(Int((x * 100).rounded()))%" }

    private func summary(_ all: Calibration.Series) -> String {
        let gap = all.overconfidence
        let verdict = abs(gap) <= 0.05 ? "about right." : gap > 0 ? "a little sure of itself." : "a little cautious."
        var scale = ""
        if let k = book.day.stabilityScale, k != 1 {
            scale = " Its intervals have moved to \(Int((k * 100).rounded()))% of the standard, so reviews come a little \(k < 1 ? "sooner" : "later")."
        }
        return "On \(all.count) probes it predicted \(pct(all.predicted)) recall and you recalled \(pct(all.observed)) — \(verdict)\(scale)"
    }

    private func chart(_ points: [Point]) -> some View {
        Chart {
            // The diagonal: prediction equals outcome.
            LineMark(x: .value("Predicted", 0.0), y: .value("Recalled", 0.0), series: .value("Series", "diagonal"))
                .foregroundStyle(Theme.muted.opacity(0.5)).lineStyle(StrokeStyle(lineWidth: 1, dash: [3, 3]))
            LineMark(x: .value("Predicted", 1.0), y: .value("Recalled", 1.0), series: .value("Series", "diagonal"))
                .foregroundStyle(Theme.muted.opacity(0.5)).lineStyle(StrokeStyle(lineWidth: 1, dash: [3, 3]))
            ForEach(points) { p in
                LineMark(x: .value("Predicted", p.predicted), y: .value("Recalled", p.observed), series: .value("Series", p.title))
                    .foregroundStyle(GrowthChart.colors[p.series % GrowthChart.colors.count])
                    .lineStyle(StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
                PointMark(x: .value("Predicted", p.predicted), y: .value("Recalled", p.observed))
                    .foregroundStyle(GrowthChart.colors[p.series % GrowthChart.colors.count])
                    .symbolSize(CGFloat(36 + min(p.count, 20) * 4))
            }
        }
        .chartLegend(.hidden)
        .chartXScale(domain: 0...1)
        .chartYScale(domain: 0...1)
        .chartXAxis {
            AxisMarks(values: [0, 0.5, 1]) { v in
                AxisGridLine().foregroundStyle(Theme.rule.opacity(0.15))
                AxisValueLabel { if let d = v.as(Double.self) { Text(pct(d)).foregroundStyle(Theme.muted) } }
            }
        }
        .chartYAxis {
            AxisMarks(position: .leading, values: [0, 0.5, 1]) { v in
                AxisGridLine().foregroundStyle(Theme.rule.opacity(0.15))
                AxisValueLabel { if let d = v.as(Double.self) { Text(pct(d)).foregroundStyle(Theme.muted) } }
            }
        }
        .chartXAxisLabel("predicted", alignment: .center).chartYAxisLabel("recalled")
        .chartPlotStyle { $0.padding(.trailing, 16) }
        .accessibilityHidden(true)
    }

    private func legend(_ calibration: Calibration, titles: [String]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            ForEach(calibration.series.indices, id: \.self) { i in
                let s = calibration.series[i]
                let slot = titles.firstIndex(of: s.modality?.rawValue ?? "") ?? 0
                HStack(alignment: .firstTextBaseline, spacing: 6) {
                    Circle().fill(GrowthChart.colors[slot % GrowthChart.colors.count]).frame(width: 8, height: 8)
                    VStack(alignment: .leading, spacing: 1) {
                        Text(s.modality?.rawValue ?? "").font(.system(size: 13)).foregroundStyle(Theme.text)
                        Text("\(s.count) probes · predicted \(pct(s.predicted)), recalled \(pct(s.observed))")
                            .font(.system(size: 12)).monospacedDigit().foregroundStyle(Theme.muted)
                    }
                }
            }
            Text("Above the dotted line you remember more than predicted.").font(.system(size: 12)).foregroundStyle(Theme.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

/// Settings for the day's shape.
struct DaySettingsSection: View {
    @Environment(Book.self) private var book

    var body: some View {
        @Bindable var book = book
        let retention = Binding(get: { book.settings.desiredRetention ?? 0.9 }, set: { book.settings.desiredRetention = $0 })
        let cap = Binding(get: { book.settings.dailyReviewCap ?? 25 }, set: { book.settings.dailyReviewCap = $0 })
        let probes = Binding(get: { book.settings.probes ?? true }, set: { book.settings.probes = $0 })
        Section {
            Picker("Aim to remember", selection: retention) {
                ForEach([0.8, 0.85, 0.9, 0.95], id: \.self) { r in Text("\(Int((r * 100).rounded()))%").tag(r) }
            }
            Stepper("Reviews per day: \(cap.wrappedValue)", value: cap, in: 5...60, step: 5)
            Toggle("Measure retention with probes", isOn: probes)
        } header: {
            Text("Your day")
        } footer: {
            Text("A review comes due when the book expects your recall to have fallen to the aim; 80% means fewer, later reviews, 95% many more. The first page of a day opens with a warm-up and the day's reviews up to the cap; what is left waits, most urgent first. Probes slip a few items you know onto pages at 1, 7 and 30 days to check review predictions — see Practice evidence.")
        }
    }
}
