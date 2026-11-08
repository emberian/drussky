import LearningCore
import SwiftUI

/// Examples and their intervals, rather than a retention gauge. These records link the weekly
/// sample to later construction decisions elsewhere in the same notebook.
struct BaselineRecallNotes: View {
    @Environment(Book.self) private var book

    var body: some View {
        let probes = book.day.probes.probes.filter { $0.id.hasPrefix("construction-transfer:") }
        if !probes.isEmpty {
            VStack(alignment: .leading, spacing: 10) {
                Text("Later, in new sentences").font(.system(size: 22, weight: .bold, design: .serif))
                ForEach(probes) { probe in
                    if let answered = probe.answered {
                        VStack(alignment: .leading, spacing: 4) {
                            Text(probe.context ?? "Construction choice").russianFont(21)
                            Text(account(probe, answered: answered)).scaledFont(15).foregroundStyle(Theme.muted)
                        }
                    } else {
                        Text("A new sentence check is due from \(probe.due.formatted(date: .abbreviated, time: .omitted)).")
                            .scaledFont(15).foregroundStyle(Theme.muted)
                    }
                }
            }
        }
    }

    private func account(_ probe: Probe, answered: Date) -> String {
        let outcome: String
        if probe.observed == nil { outcome = "left ungraded" }
        else if probe.firstAttempt != true || !(probe.assistance ?? []).isEmpty { outcome = "answered with a retry or help noted" }
        else { outcome = probe.observed == true ? "chosen correctly on the first attempt" : "a different form chosen on the first attempt" }
        var line = "\(answered.formatted(date: .abbreviated, time: .omitted)): \(outcome)."
        if let last = probe.lastExposure {
            let days = answered.timeIntervalSince(last) / 86_400
            line += String(format: " %.1f days since the last logged practice.", max(0, days))
        }
        line += " The scheduled delay was \(Int(probe.due.timeIntervalSince(probe.drawn) / 86_400)) days."
        return line
    }
}
