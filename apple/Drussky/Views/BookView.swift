import SwiftUI
import UIKit

/// The whole app: one book, open at the current page. Pages turn with a page curl.
struct BookView: View {
    @Environment(Book.self) private var book
    @Environment(AppState.self) private var app
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
        .alert(noticeTitle, isPresented: Binding(get: { notice != nil }, set: { if !$0 { notice = nil } })) {
            Button("OK") { notice = nil }
        } message: { Text(notice ?? "") }
        .sheet(isPresented: $showLibrary) { LibrarySheet().presentationSizing(.page) }
        .sheet(isPresented: Binding(get: { writeMode != nil }, set: { if !$0 { writeMode = nil } })) {
            if let writeMode { WritePageSheet(mode: writeMode) }
        }
        .overlay(alignment: .bottom) {
            if let problem = book.persistenceError ?? book.knowledge.logProblem {
                Label("Not saved yet: \(problem)", systemImage: "exclamationmark.triangle.fill")
                    .font(.footnote.weight(.medium)).foregroundStyle(.white)
                    .padding(.horizontal, 14).padding(.vertical, 8)
                    .background(Capsule().fill(Theme.redPen))
                    .padding(.bottom, 12)
                    .accessibilityAddTraits(.isStaticText)
            }
        }
        .alert("Notebook", isPresented: Binding(get: { app.notice != nil }, set: { if !$0 { app.notice = nil } })) {
            Button("OK") { app.notice = nil }
        } message: { Text(app.notice ?? "") }
        .onAppear {
            showContents = QA.has("-qa-contents")
            showSettings = QA.has("-qa-settings")
            if QA.has("-qa-pairs") { makeSoundPairs() }
            if QA.has("-qa-intonation") { QA.intonationPage(book) }
            if let state = QA.string("-qa-speak") { QA.speakingPage(book, turn: speech, voice: voice, state: state) }
            if QA.has("-qa-diary") { QA.diaryPages(book) }
        }
        .onChange(of: book.currentPage) { _, _ in
            // The goal (Goal/): its first page opens when readable; a brand-new course page that
            // adds nothing toward it gives way to a page written from the goal's own sentences.
            if book.openFirstPageIfReady() { return }
            if let page = book.page, writeMode == nil, book.goalPullDue(reaching: page) { writeMode = .goalAuto }
        }
    }

    @Environment(Voice.self) private var voice
    @Environment(SpeakingTurn.self) private var speech
    @State private var notice: String?
    @State private var noticeTitle = "Ear training"

    private func makeSoundPairs() {
        guard book.insertSoundPairsPage(voices: { voice.pairVoices($0, exactStress: $1).count }) else {
            noticeTitle = "Sound pairs"
            notice = "The pairs use letters you haven't met yet — a few more pages and they open up."
            return
        }
    }

    private func makeEarTraining() {
        let exercises = book.earTraining(from: Library.load().sentences ?? [], hasAudio: voice.hasRecording)
        guard exercises.count >= 4 else {
            noticeTitle = "Ear training"
            notice = "Not enough recorded sentences you can read yet — keep going and try again soon."
            return
        }
        book.insertWrittenPage(title: "Ear training", intro: "Real voices saying sentences built from words you know. Write what you hear; tap to replay as often as you like.",
                               unitTitle: "Listening", exercises: exercises, newWords: [])
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
                Button { writeMode = .goal } label: { Label("Toward my book", systemImage: "arrow.right.to.line") }
                    .disabled(book.currentGoal == nil)
                Button { showLibrary = true } label: { Label("Library", systemImage: "books.vertical") }
                Button { makeEarTraining() } label: { Label("Ear training", systemImage: "ear") }
                Button { makeSoundPairs() } label: { Label("Sound pairs", systemImage: "ear.badge.waveform") }
                Button { book.insertDiaryPage() } label: { Label("Write in my diary", systemImage: "pencil.line") }
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
    @Environment(SpeakingTurn.self) private var speech

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
        // A take being recorded (or waiting on permission) stops; one already being listened to
        // finishes and lands on its own exercise, which it addresses by id.
        speech.cancelCapture()
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
