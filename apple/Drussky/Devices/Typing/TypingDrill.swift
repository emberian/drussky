import SwiftUI

/// Touch-typing Russian, in three stages: the home row, every letter, then words you know.
/// Lines are generated here; the view times them and keeps a log so speed is seen over weeks.
enum TypingDrill {
    enum Stage: String, Codable, CaseIterable, Identifiable, Sendable {
        case homeRow = "home", letters, words
        var id: String { rawValue }
        var title: String {
            switch self {
            case .homeRow: "Home row"
            case .letters: "All letters"
            case .words: "My words"
            }
        }
        var note: String {
            switch self {
            case .homeRow: "фыва · олдж — fingers on the middle row, thumbs on space."
            case .letters: "Every letter of ЙЦУКЕН, in groups. Look at the map, not the keys."
            case .words: "Words from your pages. Speed comes by itself; aim for no deletes."
            }
        }
    }

    /// One line to type. Letter stages give groups of 3–5 letters; the word stage joins 3–4 words.
    static func line(stage: Stage, words: [String], using rng: inout some RandomNumberGenerator) -> String {
        switch stage {
        case .homeRow: return groups(from: RussianKeyboard.homeRow, count: 5, using: &rng)
        case .letters: return groups(from: RussianKeyboard.allLetters, count: 5, using: &rng)
        case .words:
            let pool = words.isEmpty ? ["мама", "кот", "дом", "вода", "да", "нет"] : words
            let n = min(pool.count, Int.random(in: 3...4, using: &rng))
            return Array(pool.shuffled(using: &rng).prefix(n)).joined(separator: " ")
        }
    }

    private static func groups(from letters: [Character], count: Int, using rng: inout some RandomNumberGenerator) -> String {
        (0..<count).map { _ in
            String((0..<Int.random(in: 3...5, using: &rng)).map { _ in letters.randomElement(using: &rng)! })
        }.joined(separator: " ")
    }

    /// Words per minute (five characters to a word) and accuracy for one timed line.
    struct Score: Codable, Sendable, Equatable {
        var characters: Int
        var seconds: Double
        var errors: Int
        var wpm: Double { seconds > 0 ? Double(characters) / 5 / (seconds / 60) : 0 }
        var accuracy: Double { characters > 0 ? max(0, 1 - Double(errors) / Double(characters)) : 0 }
    }

    /// Every line ever typed, by day, kept in the notebook folder (typing.json).
    struct Entry: Codable, Sendable, Equatable {
        var at: Date
        var stage: Stage
        var score: Score
    }

    struct Log: Codable, Sendable, Equatable {
        var entries: [Entry] = []

        static func url(root: URL) -> URL { root.appendingPathComponent("typing.json") }
        static func load(root: URL) -> Log {
            let dec = JSONDecoder()
            dec.dateDecodingStrategy = .iso8601
            guard let data = try? Data(contentsOf: url(root: root)), let log = try? dec.decode(Log.self, from: data) else { return Log() }
            return log
        }
        func save(root: URL) throws {
            let enc = JSONEncoder()
            enc.dateEncodingStrategy = .iso8601
            try enc.encode(self).write(to: Self.url(root: root), options: .atomic)
        }

        /// Median WPM per calendar day for a stage, oldest first: the calm line over time.
        func daily(stage: Stage, calendar: Calendar = .current) -> [(day: Date, wpm: Double)] {
            let grouped = Dictionary(grouping: entries.filter { $0.stage == stage }) { calendar.startOfDay(for: $0.at) }
            return grouped.keys.sorted().map { day in
                let v = grouped[day]!.map(\.score.wpm).sorted()
                return (day, v[v.count / 2])
            }
        }
        func best(stage: Stage) -> Double { entries.filter { $0.stage == stage }.map(\.score.wpm).max() ?? 0 }
    }
}

/// The drill itself, opened from ＋ on keyboard devices. One line at a time; the next key lit
/// on the map; speed and accuracy shown after each line, and the days so far as a thin line.
struct TypingDrillView: View {
    @Environment(Book.self) private var book
    @Environment(\.dismiss) private var dismiss
    @AppStorage("typing.phonetic") private var phonetic = false
    @State private var stage: TypingDrill.Stage = .homeRow
    @State private var line = ""
    @State private var typed = ""
    @State private var latin = ""
    @State private var started: Date?
    @State private var errors = 0
    @State private var last: TypingDrill.Score?
    @State private var log = TypingDrill.Log()
    @State private var rng = SystemRandomNumberGenerator()
    @FocusState private var focused: Bool

    private var shown: String { phonetic ? Transliteration.russian(latin) : typed }
    private var next: RussianKeyboard.Next { RussianKeyboard.next(target: line, typed: shown) }

    /// Words the learner has met, unstressed, single words only.
    private var knownWords: [String] {
        book.targets.keys.filter { $0.hasPrefix("word:") }.map { String($0.dropFirst(5)) }
            .compactMap { book.word($0)?.ru }.map(RussianText.unstressed).filter { !$0.contains(" ") }.sorted()
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 22) {
                    Picker("Stage", selection: $stage) {
                        ForEach(TypingDrill.Stage.allCases) { Text($0.title).tag($0) }
                    }
                    .pickerStyle(.segmented)
                    Text(stage.note).scaledFont(15).foregroundStyle(Theme.muted).fixedSize(horizontal: false, vertical: true)
                    target
                    field
                    KeyboardMapView(next: next, phonetic: phonetic, keyWidth: 34)
                    Toggle(isOn: $phonetic) { Text("Phonetic (sh → ш, ya → я, ' → ь)") }
                        .toggleStyle(.switch).font(Theme.label(13)).foregroundStyle(Theme.muted).fixedSize()
                    speed
                }
                .padding(24)
                .frame(maxWidth: 760)
                .frame(maxWidth: .infinity)
            }
            .background(GridPaper(marginX: nil))
            .navigationTitle("Typing drill")
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() } } }
            .onAppear { log = TypingDrill.Log.load(root: book.store.root); newLine(); focused = true }
            .onChange(of: stage) { _, _ in newLine() }
            .onChange(of: phonetic) { _, _ in typed = ""; latin = ""; started = nil; errors = 0 }
            .onChange(of: shown) { old, new in
                if started == nil, !new.isEmpty { started = Date() }
                // A key that left the target counts as an error once, when it is struck.
                if new.count > old.count, case .backspace = next { errors += 1 }
                if case .done = next { finishLine() }
            }
        }
    }

    /// The line, typed letters in ink, the rest in the model's grey; the one needed next underlined.
    private var target: some View {
        let want = Array(line), have = Array(shown)
        var text = AttributedString()
        for (i, ch) in want.enumerated() {
            var piece = AttributedString(String(ch))
            if i < have.count {
                let ok = Character(String(have[i]).lowercased()) == Character(String(ch).lowercased())
                piece.foregroundColor = ok ? Theme.ink : Theme.redPen
                if !ok { piece.underlineStyle = .single }
            } else {
                piece.foregroundColor = Theme.muted.opacity(0.7)
                if i == have.count { piece.underlineStyle = .single; piece.foregroundColor = Theme.accent }
            }
            text += piece
        }
        return Text(text)
            .font(Theme.russian(stage == .words ? 36 : 40, weight: .medium))
            .monospaced(stage != .words)
            .fixedSize(horizontal: false, vertical: true)
            .accessibilityLabel("Type: \(line)")
    }

    private var field: some View {
        Group {
            if phonetic { TextField("type phonetically", text: $latin) } else { TextField("напиши́те", text: $typed) }
        }
        .textFieldStyle(.plain)
        .font(phonetic ? .system(size: 22, design: .monospaced) : Theme.russian(28, weight: .medium))
        .autocorrectionDisabled()
        .textInputAutocapitalization(.never)
        .focused($focused)
        .onSubmit { if case .done = next { finishLine() } }
        .padding(.horizontal, 14).padding(.vertical, 10)
        .background(RoundedRectangle(cornerRadius: 12).fill(Theme.card))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(Theme.rule.opacity(0.5), lineWidth: 1))
        .accessibilityIdentifier("drill-field")
    }

    private var speed: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .firstTextBaseline, spacing: 18) {
                if let last {
                    Text("\(Int(last.wpm.rounded())) wpm").font(.system(size: 28, weight: .bold, design: .serif)).foregroundStyle(Theme.text)
                        .monospacedDigit()
                    Text("\(Int((last.accuracy * 100).rounded()))% clean").scaledFont(15).foregroundStyle(Theme.muted)
                } else {
                    Text("Type the line; your speed appears here.").scaledFont(15).foregroundStyle(Theme.muted)
                }
                Spacer()
                let best = log.best(stage: stage)
                if best > 0 { Text("best \(Int(best.rounded()))").font(Theme.label(13)).foregroundStyle(Theme.muted) }
            }
            let days = log.daily(stage: stage)
            if days.count >= 2 {
                WPMLine(points: days.map(\.wpm)).frame(height: 44)
                Text("\(days.count) days of \(stage.title.lowercased()), median speed each day").font(Theme.label(12)).foregroundStyle(Theme.muted)
            }
        }
        .padding(16)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
    }

    private func newLine() {
        line = TypingDrill.line(stage: stage, words: knownWords, using: &rng)
        typed = ""; latin = ""; started = nil; errors = 0
    }

    private func finishLine() {
        guard !book.isClosed else { return }
        guard let started else { newLine(); return }
        let score = TypingDrill.Score(characters: line.count, seconds: Date().timeIntervalSince(started), errors: errors)
        last = score
        log.entries.append(TypingDrill.Entry(at: Date(), stage: stage, score: score))
        do { try log.save(root: book.store.root) } catch { book.report("typing log: \((error as NSError).localizedDescription)") }
        newLine()
    }
}

/// A thin line of speeds over days; no axes, the numbers are beside it.
struct WPMLine: View {
    let points: [Double]
    var body: some View {
        Canvas { ctx, size in
            guard points.count >= 2, let hi = points.max(), hi > 0 else { return }
            var path = Path()
            for (i, p) in points.enumerated() {
                let x = size.width * CGFloat(i) / CGFloat(points.count - 1)
                let y = size.height - size.height * CGFloat(p / hi) * 0.9 - 2
                if i == 0 { path.move(to: CGPoint(x: x, y: y)) } else { path.addLine(to: CGPoint(x: x, y: y)) }
            }
            ctx.stroke(path, with: .color(Theme.accent), style: StrokeStyle(lineWidth: 2, lineCap: .round, lineJoin: .round))
        }
        .accessibilityLabel("Speed over \(points.count) days")
    }
}
