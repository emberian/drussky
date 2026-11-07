import Charts
import LearningCore
import SwiftUI

/// Expected number of words you'd recall on each day, one line per way of using them.
/// Each word contributes its FSRS retrievability that day (0 before first practice), so the
/// line rises as you learn and sags where memories fade between reviews.
struct GrowthChart: View {
    let tracker: KnowledgeTracker
    @State private var picked: Date?

    /// Categorical slots 1–4 of the validated dataviz palette, light and dark steps.
    static let colors: [Color] = [
        Color(Theme.dynamic(0x2A78D6, 0x3987E5)), Color(Theme.dynamic(0xEB6834, 0xD95926)),
        Color(Theme.dynamic(0x1BAF7A, 0x199E70)), Color(Theme.dynamic(0xEDA100, 0xC98500)),
    ]

    struct Point: Identifiable {
        var series: Int
        var date: Date
        var value: Double
        var id: String { "\(series)-\(date.timeIntervalSince1970)" }
    }

    var body: some View {
        let (dates, series) = tracker.growth()
        let groups = KnowledgeTracker.growthGroups
        let points = series.enumerated().flatMap { s, values in
            zip(dates, values).map { Point(series: s, date: $0, value: $1) }
        }
        let latest = series.map { $0.last ?? 0 }
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline) {
                Text("Words you'd recall today").font(Theme.label(13)).foregroundStyle(Theme.muted)
                Spacer()
                legend(groups.map(\.title), latest)
            }
            if dates.count >= 2 {
                chart(points, groups: groups.map(\.title), dates: dates, series: series)
                    .frame(height: 180)
            } else {
                Text("The lines start after your second day of practice.")
                    .font(.system(size: 14)).foregroundStyle(Theme.muted)
                    .frame(maxWidth: .infinity, minHeight: 60, alignment: .leading)
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
    }

    private func legend(_ titles: [String], _ latest: [Double]) -> some View {
        HStack(spacing: 14) {
            ForEach(titles.indices, id: \.self) { i in
                HStack(spacing: 5) {
                    Capsule().fill(Self.colors[i]).frame(width: 14, height: 3)
                    Text(titles[i]).font(.system(size: 12)).foregroundStyle(Theme.text)
                    Text("\(Int(latest[i].rounded()))").font(.system(size: 12, weight: .semibold, design: .rounded))
                        .monospacedDigit().foregroundStyle(Theme.muted)
                }
            }
        }
    }

    private func chart(_ points: [Point], groups: [String], dates: [Date], series: [[Double]]) -> some View {
        Chart {
            ForEach(points) { p in
                LineMark(x: .value("Day", p.date), y: .value("Words", p.value), series: .value("Modality", groups[p.series]))
                    .foregroundStyle(Self.colors[p.series])
                    .lineStyle(StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
                    .interpolationMethod(.monotone)
            }
            if let picked, let i = dates.firstIndex(where: { Calendar.current.isDate($0, inSameDayAs: picked) }) {
                RuleMark(x: .value("Day", dates[i]))
                    .foregroundStyle(Theme.muted.opacity(0.4))
                    .annotation(position: .top, alignment: .leading, spacing: 4, overflowResolution: .init(x: .fit, y: .disabled)) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(dates[i], format: .dateTime.month().day()).font(Theme.label(11)).foregroundStyle(Theme.muted)
                            ForEach(groups.indices, id: \.self) { s in
                                HStack(spacing: 4) {
                                    Circle().fill(Self.colors[s]).frame(width: 6, height: 6)
                                    Text("\(groups[s]) \(Int(series[s][i].rounded()))").font(.system(size: 11)).foregroundStyle(Theme.text)
                                }
                            }
                        }
                        .padding(6)
                        .background(RoundedRectangle(cornerRadius: 6).fill(Theme.paper))
                    }
            }
        }
        .chartLegend(.hidden)
        .chartXSelection(value: $picked)
        .chartXAxis {
            AxisMarks(values: .automatic(desiredCount: 5)) { _ in
                AxisGridLine().foregroundStyle(Theme.rule.opacity(0.15))
                AxisValueLabel(format: .dateTime.month(.abbreviated).day()).foregroundStyle(Theme.muted)
            }
        }
        .chartYAxis {
            AxisMarks(position: .leading, values: .automatic(desiredCount: 4)) { _ in
                AxisGridLine().foregroundStyle(Theme.rule.opacity(0.15))
                AxisValueLabel().foregroundStyle(Theme.muted)
            }
        }
    }
}
