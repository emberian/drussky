import SwiftUI
import UIKit

/// The whole app: one book, open at the current page. Pages turn with a page curl.
struct BookView: View {
    @Environment(Book.self) private var book
    @State private var showContents = false
    @State private var showSettings = false
    @State private var writeMode: WritePageSheet.Mode?
    @State private var showLibrary = false
    @Environment(\.horizontalSizeClass) private var sizeClass

    var body: some View {
        ZStack(alignment: .top) {
            Theme.desk.ignoresSafeArea()
            PageCurl(book: book)
                .ignoresSafeArea(edges: .bottom)
            topBar
        }
        .sheet(isPresented: $showContents) { ContentsView().presentationSizing(.page) }
        .sheet(isPresented: $showSettings) { SettingsView() }
        .sheet(isPresented: $showLibrary) { LibrarySheet().presentationSizing(.page) }
        .sheet(isPresented: Binding(get: { writeMode != nil }, set: { if !$0 { writeMode = nil } })) {
            if let writeMode { WritePageSheet(mode: writeMode) }
        }
        .onAppear {
            showContents = QA.has("-qa-contents")
            showSettings = QA.has("-qa-settings")
        }
    }

    private var topBar: some View {
        HStack(spacing: 6) {
            Button { showContents = true } label: {
                Label("Alphabet & pages", systemImage: "character.book.closed")
                    .labelStyle(.iconOnly)
                    .font(.system(size: 20, weight: .medium))
                    .frame(width: 44, height: 44)
            }
            if sizeClass != .compact {
                Text("Drussky")
                    .font(.system(size: 17, weight: .bold, design: .serif))
                    .foregroundStyle(Theme.text)
            }
            Spacer()
            Button { withAnimation { book.turnBack() } } label: {
                Image(systemName: "chevron.left").font(.system(size: 17, weight: .semibold)).frame(width: 44, height: 44)
            }
            .disabled(book.currentPage == 0)
            .accessibilityLabel("Previous page")
            Text("стр. \(book.currentPage + 1)")
                .font(.system(size: 15, weight: .semibold, design: .rounded))
                .monospacedDigit()
                .foregroundStyle(Theme.text)
                .frame(minWidth: 56)
            Button { withAnimation { _ = book.turnForward() } } label: {
                Image(systemName: "chevron.right").font(.system(size: 17, weight: .semibold)).frame(width: 44, height: 44)
            }
            .disabled(book.currentPage == book.pages.count - 1 && !(book.page?.isFinished ?? false))
            .accessibilityLabel("Next page")
            Button { book.marginMode.toggle() } label: {
                Image(systemName: book.marginMode ? "pencil.tip.crop.circle.fill" : "pencil.tip.crop.circle")
                    .font(.system(size: 20, weight: .medium)).frame(width: 44, height: 44)
                    .foregroundStyle(book.marginMode ? Theme.redPen : Theme.accent)
            }
            .accessibilityLabel(book.marginMode ? "Stop writing in the margins" : "Write in the margins")
            Menu {
                Button { writeMode = .practice } label: { Label("A page from my mistakes", systemImage: "wand.and.stars") }
                Button { writeMode = .text } label: { Label("Read my own text…", systemImage: "doc.text") }
                Button { showLibrary = true } label: { Label("Library", systemImage: "books.vertical") }
            } label: {
                Image(systemName: "plus.square.on.square").font(.system(size: 18, weight: .medium)).frame(width: 44, height: 44)
            }
            .accessibilityLabel("Add a page")
            Button { showSettings = true } label: {
                Image(systemName: "gearshape").font(.system(size: 18, weight: .medium)).frame(width: 44, height: 44)
            }
            .accessibilityLabel("Settings")
        }
        .tint(Theme.accent)
        .padding(.horizontal, 12)
        .background(.ultraThinMaterial, in: Capsule())
        .padding(.horizontal, 16)
        .padding(.top, 6)
    }
}

/// UIPageViewController with a page-curl transition, driven by `book.currentPage`.
/// Gestures are off so the Pencil never turns a page by accident.
struct PageCurl: UIViewControllerRepresentable {
    let book: Book
    @Environment(Voice.self) private var voice
    @Environment(SpeechCheck.self) private var speech

    func makeUIViewController(context: Context) -> UIPageViewController {
        let pvc = UIPageViewController(transitionStyle: .pageCurl, navigationOrientation: .horizontal,
                                       options: [.spineLocation: NSNumber(value: UIPageViewController.SpineLocation.min.rawValue)])
        pvc.isDoubleSided = false
        pvc.view.backgroundColor = Theme.deskUI
        pvc.setViewControllers([host(book.currentPage)], direction: .forward, animated: false)
        context.coordinator.shown = book.currentPage
        for g in pvc.gestureRecognizers { g.isEnabled = false }
        return pvc
    }

    func updateUIViewController(_ pvc: UIPageViewController, context: Context) {
        let target = book.currentPage
        guard target != context.coordinator.shown else { return }
        let direction: UIPageViewController.NavigationDirection = target > context.coordinator.shown ? .forward : .reverse
        context.coordinator.shown = target
        voice.stop()
        if speech.isRecording { speech.cancel() }
        let animated = UIView.areAnimationsEnabled && !UIAccessibility.isReduceMotionEnabled
        pvc.setViewControllers([host(target)], direction: direction, animated: animated)
    }

    private func host(_ index: Int) -> UIViewController {
        let root = PageView(pageIndex: index)
            .environment(book)
            .environment(voice)
            .environment(speech)
        let vc = UIHostingController(rootView: root)
        vc.view.backgroundColor = Theme.paperUI
        return vc
    }

    func makeCoordinator() -> Coordinator { Coordinator() }
    final class Coordinator { var shown = 0 }
}
