import SwiftUI

/// One workbook page: the dated header a Russian school notebook starts with, then the
/// numbered exercises down the page, then the teacher's mark.
struct PageView: View {
    @Environment(Book.self) private var book
    @Environment(\.horizontalSizeClass) private var sizeClass
    let pageIndex: Int
    var topInset: CGFloat = 64
    @State private var lastActive: Int?

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
                                            isActive: active == i, marginWidth: marginWidth)
                                    .id(i)
                            }
                            footer(page)
                                .padding(.leading, marginWidth + 18)
                                .id("end")
                        }
                        .frame(width: contentWidth)
                        .padding(.top, topInset)
                        .padding(.bottom, 120)
                        .frame(maxWidth: .infinity)
                        .background(GridPaper(marginX: marginX))
                    }
                    .scrollDismissesKeyboard(.immediately)
                    .onAppear {
                        lastActive = active
                        if let active, active > 1 {
                            DispatchQueue.main.async { proxy.scrollTo(active, anchor: UnitPoint(x: 0.5, y: 0.15)) }
                        }
                    }
                    .onChange(of: active) { old, new in
                        guard old != nil else { return }
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
        }
    }

    private func header(_ page: BookPage) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(alignment: .firstTextBaseline) {
                Text(Self.russianDate(page.created))
                    .font(Theme.cursive(compact ? 26 : 32))
                    .foregroundStyle(Theme.ink)
                Spacer()
                Text(page.isReview ? "Повторение" : "Классная работа")
                    .font(Theme.cursive(compact ? 22 : 28))
                    .foregroundStyle(Theme.ink)
            }
            .padding(.trailing, 12)
            Text(page.unitTitle.uppercased())
                .font(Theme.label(12))
                .tracking(1.6)
                .foregroundStyle(Theme.accent)
                .padding(.top, 10)
            Text(page.title)
                .font(.system(size: compact ? 30 : 40, weight: .bold, design: .serif))
                .foregroundStyle(Theme.text)
            if let intro = page.intro {
                Text(intro)
                    .font(.system(size: 18))
                    .lineSpacing(3)
                    .foregroundStyle(Theme.text.opacity(0.82))
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxWidth: 640, alignment: .leading)
            }
        }
    }

    @ViewBuilder private func footer(_ page: BookPage) -> some View {
        VStack(alignment: .leading, spacing: 18) {
            if page.isFinished {
                HStack(alignment: .center, spacing: 18) {
                    GradeStamp(page: page)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(GradeStamp.praise(for: page))
                            .font(Theme.cursive(34))
                            .foregroundStyle(Theme.redPen)
                        Text(summary(page))
                            .font(.system(size: 15))
                            .foregroundStyle(Theme.muted)
                    }
                }
                .transition(.scale(scale: 0.8).combined(with: .opacity))
            }
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
        }
        .padding(.top, 30)
        .animation(.spring(response: 0.5, dampingFraction: 0.7), value: page.isFinished)
    }

    private func summary(_ page: BookPage) -> String {
        let firstTry = page.results.filter { $0.status == .correct }.count
        let missed = page.results.filter { $0.status == .missed }.count
        var parts = ["\(page.exercises.count) exercises"]
        if firstTry > 0 { parts.append("\(firstTry) right first time") }
        if missed > 0 { parts.append("\(missed) to bring back soon") }
        return parts.joined(separator: " · ")
    }

    static func russianDate(_ date: Date) -> String {
        let f = DateFormatter()
        f.locale = Locale(identifier: "ru_RU")
        f.setLocalizedDateFormatFromTemplate("d MMMM")
        return f.string(from: date)
    }
}

/// A Russian teacher's mark: a red 5 (отлично) or 4 in the corner of a finished page.
struct GradeStamp: View {
    let page: BookPage

    static func grade(for page: BookPage) -> Int {
        let graded = page.results.filter { $0.status != .done }
        guard !graded.isEmpty else { return 5 }
        let missed = graded.filter { $0.status == .missed }.count
        return missed == 0 ? 5 : missed <= 2 ? 4 : 3
    }

    static func praise(for page: BookPage) -> String {
        switch grade(for: page) {
        case 5: "Отлично!"
        case 4: "Хорошо!"
        default: "Молодец — дальше!"
        }
    }

    var body: some View {
        Text("\(Self.grade(for: page))")
            .font(Theme.cursive(84))
            .foregroundStyle(Theme.redPen)
            .rotationEffect(.degrees(-10))
            .frame(width: 84, height: 84)
            .accessibilityLabel("Mark: \(Self.grade(for: page)) out of 5")
    }
}
