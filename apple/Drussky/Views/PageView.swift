import SwiftUI

/// One workbook page: the dated header a Russian school notebook starts with, then the
/// numbered exercises down the page, then the teacher's mark.
struct PageView: View {
    @Environment(Book.self) private var book
    @Environment(\.horizontalSizeClass) private var sizeClass
    let pageIndex: Int
    var topInset: CGFloat = 64
    @State private var lastActive: Int?
    @State private var asking: AskTarget?

    struct AskTarget: Identifiable { var exercise: Int?; var id: Int { exercise ?? -1 } }

    private var compact: Bool { sizeClass == .compact }
    private var marginWidth: CGFloat { compact ? 40 : 64 }
    private let maxWidth: CGFloat = 900

    var body: some View {
        if book.pages.indices.contains(pageIndex) {
            let page = book.pages[pageIndex]
            let active = page.firstOpenIndex
            GeometryReader { geo in
                let contentWidth = min(geo.size.width - (compact ? 16 : 48), maxWidth)
                let marginX = (geo.size.width - contentWidth) / 2 + marginWidth
                ScrollViewReader { proxy in
                    ScrollView {
                        VStack(alignment: .leading, spacing: 0) {
                            header(page)
                                .padding(.leading, marginWidth + 18)
                                .padding(.bottom, 12)
                            ForEach(page.exercises.indices, id: \.self) { i in
                                ExerciseRow(slot: ExerciseSlot(book: book, pageIndex: pageIndex, index: i),
                                            isActive: active == i, marginWidth: marginWidth,
                                            onAsk: { asking = AskTarget(exercise: i) })
                                    .id(i)
                            }
                            PageNotesView(pageIndex: pageIndex) { asking = AskTarget(exercise: $0) }
                                .padding(.leading, marginWidth + 18)
                            footer(page)
                                .padding(.leading, marginWidth + 18)
                                .id("end")
                        }
                        .frame(width: contentWidth)
                        .overlay { MarginLayer(book: book, page: page, enabled: book.marginMode, fingerDrawing: book.settings.fingerDrawing) }
                        .padding(.top, topInset)
                        .padding(.bottom, 120)
                        .frame(maxWidth: .infinity)
                        .background(GridPaper(marginX: marginX))
                    }
                    .scrollDismissesKeyboard(.immediately)
                    .onAppear {
                        lastActive = active
                        if QA.has("-qa-bottom") { DispatchQueue.main.async { proxy.scrollTo("end", anchor: .bottom) } }
                        if let active, active > 1, !QA.has("-qa-top") {
                            DispatchQueue.main.async { proxy.scrollTo(active, anchor: UnitPoint(x: 0.5, y: 0.15)) }
                        }
                    }
                    .onChange(of: book.advanceSignal) { _, _ in
                        guard book.advancePageID == book.pages[pageIndex].id else { return }
                        let new = book.pages[pageIndex].firstOpenIndex
                        Task {
                            try? await Task.sleep(for: .milliseconds(450))
                            withAnimation(.easeInOut(duration: 0.55)) {
                                if let new { proxy.scrollTo(new, anchor: UnitPoint(x: 0.5, y: 0.15)) }
                                else { proxy.scrollTo("end", anchor: .bottom) }
                            }
                        }
                    }
                }
            }
            .background(Theme.paper.ignoresSafeArea())
            .sheet(item: $asking) { target in
                TutorSheet(pageIndex: pageIndex, exercise: target.exercise)
                    .presentationSizing(.page)
            }
        }
    }

    private func header(_ page: BookPage) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .firstTextBaseline) {
                Text(Self.russianDate(page.created))
                    .font(Theme.cursive(compact ? 26 : 32))
                    .foregroundStyle(Theme.ink)
                Spacer()
                Text(page.isReview ? "Повторе́ние" : "Кла́ссная рабо́та")
                    .font(Theme.cursive(compact ? 22 : 28))
                    .foregroundStyle(Theme.ink)
            }
            .padding(.trailing, 12)
            Text(page.unitTitle.uppercased())
                .font(Theme.label(12))
                .tracking(1.6)
                .foregroundStyle(Theme.accent)
                .padding(.top, 4)
            Text(page.title)
                .font(.system(size: compact ? 28 : 34, weight: .bold, design: .serif))
                .foregroundStyle(Theme.text)
            if page.isGoalFirstPage { FirstPageStamp() }
            if page.number == 1 {
                HowThisBookWorks()
                    .padding(.vertical, 6)
            }
            if let intro = page.intro {
                Text(Theme.stressed(intro, show: book.settings.showStress))
                    .scaledFont(18)
                    .lineSpacing(3)
                    .foregroundStyle(Theme.text.opacity(0.82))
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: 640, alignment: .leading)
            }
        }
    }

    @ViewBuilder private func footer(_ page: BookPage) -> some View {
        VStack(alignment: .leading, spacing: 18) {
            DeskLine(pageIndex: pageIndex)
            if page.isFinished {
                HStack(alignment: .center, spacing: 18) {
                    GradeStamp(page: page)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(GradeStamp.praise(for: page))
                            .font(Theme.cursive(34))
                            .foregroundStyle(Theme.redPen)
                        Text(summary(page))
                            .scaledFont(15)
                            .foregroundStyle(Theme.muted)
                    }
                }
                .transition(.scale(scale: 0.8).combined(with: .opacity))
            }
            ShelfOfferCard(pageIndex: pageIndex)  // an episode or reading the course has reached (Views/ShelfViews.swift)
            Button {
                withAnimation { _ = book.turnForward() }
            } label: {
                HStack(spacing: 10) {
                    Text(page.isFinished ? "Turn the page" : "Skip to the next page")
                    Image(systemName: "arrow.right")
                }
                .font(.system(size: page.isFinished ? 19 : 15, weight: .semibold, design: .rounded))
                .padding(.horizontal, page.isFinished ? 26 : 0).padding(.vertical, page.isFinished ? 15 : 6)
                .foregroundStyle(page.isFinished ? Color.white : Theme.muted)
                .background(Capsule().fill(page.isFinished ? Theme.accent : Color.clear))
            }
            .buttonStyle(.plain)
            .disabled(book.courseFinished && pageIndex == book.pages.count - 1 && book.dueCount == 0)
            Button { asking = AskTarget(exercise: nil) } label: {
                Label("Ask about this page", systemImage: "questionmark.bubble")
                    .scaledFont(15, weight: .medium)
                    .foregroundStyle(Theme.muted)
            }
            .buttonStyle(.plain)
            PagePrintButton(pageIndex: pageIndex)  // Devices/PDF
        }
        .padding(.top, 30)
        .animation(.spring(response: 0.5, dampingFraction: 0.7), value: page.isFinished)
    }

    private func summary(_ page: BookPage) -> String {
        let firstTry = page.results.filter(\.isRightFirstTime).count
        let missed = page.results.filter { $0.status == .missed }.count
        var parts = [page.exercises.count == 1 ? "1 exercise" : "\(page.exercises.count) exercises"]
        if firstTry > 0 { parts.append("\(firstTry) right first time") }
        if missed > 0 { parts.append("\(missed) to bring back soon") }
        let skipped = page.results.filter { $0.status == .skipped }.count
        if skipped > 0 { parts.append("\(skipped) skipped") }
        return parts.joined(separator: " · ")
    }

    static func russianDate(_ date: Date) -> String {
        let f = DateFormatter()
        f.locale = Locale(identifier: "ru_RU")
        f.setLocalizedDateFormatFromTemplate("d MMMM")
        return f.string(from: date)
    }
}

/// A Russian teacher's mark: a red 5 (отлично) or 4 in the corner of a finished page — or a tick
/// when the page was completed but nothing on it was assessed.
struct GradeStamp: View {
    let page: BookPage

    /// The mark, or nil when nothing on the page was assessed: practice, skips, speaking the
    /// learner judged themselves, diary lines nobody corrected or whose correction didn't hold up.
    /// A diary line the tutor marked counts: right as written, or with a few slips, is fine;
    /// three or more corrections is a miss.
    static func grade(for page: BookPage) -> Int? {
        var graded = 0, missed = 0
        for (ex, r) in zip(page.exercises, page.results) {
            switch r.status {
            case .correct: graded += 1
            case .missed: graded += 1; missed += 1
            case .done:
                if ex.kind == .diary, let c = r.diary?.correction, c.verified != false {
                    graded += 1
                    if c.edits.count >= 3 { missed += 1 }
                }
            case .open, .skipped: break
            }
        }
        guard graded > 0 else { return nil }
        return missed == 0 ? 5 : missed <= 2 ? 4 : 3
    }

    /// What goes in the corner: the mark, or a tick.
    static func mark(for page: BookPage) -> String { grade(for: page).map(String.init) ?? "✓" }

    static func praise(for page: BookPage) -> String {
        switch grade(for: page) {
        case 5?: "Отли́чно!"
        case 4?: "Хорошо́!"
        case .some: "Молоде́ц — да́льше!"
        case nil: "Готово!"
        }
    }

    var body: some View {
        Text(Self.mark(for: page))
            .font(Theme.cursive(84))
            .foregroundStyle(Theme.redPen)
            .rotationEffect(.degrees(-10))
            .frame(width: 84, height: 84)
            .accessibilityLabel(Self.grade(for: page).map { "Mark: \($0) out of 5" } ?? "Done, not graded")
    }
}

/// Three lines on the very first page, so nobody has to guess.
struct HowThisBookWorks: View {
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            row("pencil.and.scribble", "Write with your Pencil right on the lines. Each exercise ticks itself when it reads your writing; tap Done if it doesn't.")
            row("speaker.wave.2", "Tap any speaker to hear it again, as often as you like.")
            row("questionmark.bubble", "Stuck or curious? The ? beside an exercise asks your tutor.")
            row("book.pages", "Finish the page, then turn it. Earlier pages keep everything you wrote.")
        }
        .padding(16)
        .frame(maxWidth: 640, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(Theme.card))
    }

    private func row(_ icon: String, _ text: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 12) {
            Image(systemName: icon).foregroundStyle(Theme.accent).frame(width: 22)
            Text(text).scaledFont(16).foregroundStyle(Theme.text)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

/// Owns the page's margin-ink controller so it survives re-renders.
struct MarginLayer: View {
    @Environment(Book.self) private var book
    let page: BookPage
    let enabled: Bool
    let fingerDrawing: Bool
    @StateObject private var controller: PadController

    init(book: Book, page: BookPage, enabled: Bool, fingerDrawing: Bool) {
        self.page = page
        self.enabled = enabled
        self.fingerDrawing = fingerDrawing
        _controller = StateObject(wrappedValue: PadController(fileURL: book.marginURL(page: page), writer: book.store.ink))
    }

    var body: some View {
        MarginInk(controller: controller, enabled: enabled, fingerDrawing: fingerDrawing)
            .allowsHitTesting(enabled)
            .overlay(alignment: .top) {
                if enabled {
                    Text("Margin notes — write anywhere. Double-tap the Pencil to erase.")
                        .font(Theme.label(12)).foregroundStyle(Theme.redPen)
                        .padding(.horizontal, 12).padding(.vertical, 6)
                        .background(Capsule().fill(Theme.card))
                        .padding(.top, 4)
                        .allowsHitTesting(false)
                }
            }
    }
}
