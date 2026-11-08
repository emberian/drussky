import Foundation

extension QA {
    static func string(_ flag: String) -> String? {
        let args = ProcessInfo.processInfo.arguments
        guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
        return args[i + 1]
    }

    /// `-qa-speak STATE`: a read-aloud page held in one state of the speaking turn, for
    /// screenshots without a microphone.
    ///   idle | model | yourturn | listening | analysing   молоко́, no take yet
    ///   report          молоко́, another speaker's recording as the take
    ///   report-sound    был, with a recording of бил as the take (heard «бил»)
    ///   report-stress   замо́к, with a recording of за́мок as the take
    static func speakingPage(_ book: Book, turn: SpeakingTurn, voice: Voice, state: String) {
        let (target, take, heard): (String, String?, String?) = switch state {
        case "report": ("молоко́", "moloko-aleksey-pogrebnoj-al-f0a170", "молоко")
        case "report-sound": ("был", "bil-cdd458", "бил")
        case "report-stress": ("замо́к", voice.pairVoices("за́мок", exactStress: true).first?.deletingPathExtension().lastPathComponent, "замок")
        default: ("молоко́", nil, nil)
        }
        var ex = Exercise(kind: .readAloud)
        ex.text = target
        book.insertWrittenPage(title: "Speaking", intro: nil, unitTitle: "QA", exercises: [ex], newWords: [])
        guard let page = book.page else { return }
        let key = "\(page.id)-0"
        // After the page has turned to it (a page turn ends any turn in progress).
        Task { @MainActor in
            try? await Task.sleep(for: .seconds(1.5))
            switch state {
            case "model": turn.pose(.model(1, of: 2), key: key)
            case "yourturn": turn.pose(.cue, key: key)
            case "listening": turn.pose(.hearing, key: key, level: 0.55)
            case "analysing": turn.pose(.analysing, key: key)
            default: break
            }
        }
        guard let take, let source = Bundle.main.url(forResource: take, withExtension: "m4a", subdirectory: "Audio") else { return }
        let file = "\(page.id)-0-qa.m4a"
        let dest = book.voiceURL(file: file)
        try? FileManager.default.createDirectory(at: dest.deletingLastPathComponent(), withIntermediateDirectories: true)
        try? FileManager.default.removeItem(at: dest)
        try? FileManager.default.removeItem(at: SpeakingTurn.reportURL(for: dest))
        try? FileManager.default.copyItem(at: source, to: dest)
        book.commitTake(ExerciseRef(pageID: page.id, index: 0), round: 0, file: file, heard: heard, target: target,
                        recognitionAllowed: true, timedOut: false)
    }
}
