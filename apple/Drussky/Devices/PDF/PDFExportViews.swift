import SwiftUI

/// Settings → the whole notebook as a PDF, on A4 or Letter, through the share sheet.
struct PDFExportSection: View {
    @Environment(Book.self) private var book
    @State private var url: URL?
    @State private var rendering = false
    @AppStorage("pdf.paper") private var paperChoice = "local"

    private var paper: NotebookPDF.Paper {
        switch paperChoice {
        case "a4": .a4
        case "letter": .letter
        default: .local
        }
    }

    var body: some View {
        Section {
            Picker("Paper", selection: $paperChoice) {
                Text(NotebookPDF.Paper.local.size == NotebookPDF.Paper.a4.size ? "A4 (here)" : "A4").tag("a4")
                Text(NotebookPDF.Paper.local.size == NotebookPDF.Paper.letter.size ? "Letter (here)" : "Letter").tag("letter")
            }
            .onChange(of: paperChoice) { _, _ in url = nil }
            if let url {
                ShareLink(item: url) { Label("Export notebook as PDF", systemImage: "printer") }
            } else {
                Button {
                    rendering = true
                    Task { url = await NotebookPDF.export(book: book, paper: paper); rendering = false }
                } label: {
                    Label(rendering ? "Laying out \(book.pages.count) pages…" : "Export notebook as PDF", systemImage: "printer")
                }
                .disabled(rendering)
            }
        } footer: {
            Text("Every page as it is in the book — the squared paper, your handwriting, the red pen and the marks — ready to print or keep.")
        }
        .onAppear { if paperChoice == "local" { paperChoice = NotebookPDF.Paper.local.size == NotebookPDF.Paper.a4.size ? "a4" : "letter" } }
    }
}

/// The page footer's "Print this page": one page as a PDF, straight to the share sheet.
struct PagePrintButton: View {
    @Environment(Book.self) private var book
    let pageIndex: Int
    @State private var url: URL?
    @State private var rendering = false

    var body: some View {
        if let url {
            ShareLink(item: url) {
                Label("Share this page as PDF", systemImage: "square.and.arrow.up")
                    .scaledFont(15, weight: .medium).foregroundStyle(Theme.muted)
            }
            .buttonStyle(.plain)
        } else {
            Button {
                guard book.pages.indices.contains(pageIndex) else { return }
                rendering = true
                let page = book.pages[pageIndex]
                Task { url = await NotebookPDF.export(book: book, pages: [page]); rendering = false }
            } label: {
                Label(rendering ? "Laying out…" : "Print this page", systemImage: "printer")
                    .scaledFont(15, weight: .medium).foregroundStyle(Theme.muted)
            }
            .buttonStyle(.plain)
            .disabled(rendering)
            .onChange(of: book.pages[safe: pageIndex]?.results) { _, _ in url = nil }  // a new answer: lay it out again
        }
    }
}
