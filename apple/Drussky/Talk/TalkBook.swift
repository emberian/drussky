import Foundation

/// A conversation as it appears in "My Russian".
struct TalkRecord: Identifiable, Hashable {
    var ref: ExerciseRef
    var pageNumber: Int
    var conversation: TalkConversation
    var id: String { ref.item }
}

extension Book {
    /// ＋ → Talk: a page with one conversation, placed right after the current page.
    func insertTalkPage() {
        let ex = Exercise(kind: .talk)
        insertWrittenPage(title: "Разгово́р · Talk", intro: "A short conversation with your tutor, in Russian, out loud. Pick a scene; answer when it's your turn; afterwards the whole dialogue is marked in red.",
                          unitTitle: "My Russian", exercises: [ex], newWords: [])
    }

    /// Every conversation ever held, newest first.
    var talkRecords: [TalkRecord] {
        var out: [TalkRecord] = []
        for page in pages {
            for (i, r) in page.results.enumerated() where page.exercises[i].kind == .talk {
                if let t = r.talk, !t.lines.isEmpty {
                    out.append(TalkRecord(ref: ExerciseRef(pageID: page.id, index: i), pageNumber: page.number, conversation: t))
                }
            }
        }
        return out.sorted { ($0.conversation.started ?? .distantPast) > ($1.conversation.started ?? .distantPast) }
    }

    /// Writes the conversation as it stands (every line lands here as it happens, so a page turn
    /// or a crash loses nothing said so far).
    func saveTalk(_ ref: ExerciseRef, _ conversation: TalkConversation) {
        record(ref) { r in
            r.talk = conversation
            r.attempts = max(r.attempts, conversation.learnerTurns)
        }
    }

    /// The red pen landed: the exercise finishes, and every corrected form and new word the
    /// tutor used joins the learner's words.
    func finishTalk(_ ref: ExerciseRef, _ conversation: TalkConversation) {
        for line in conversation.lines {
            for w in line.newWords ?? [] where !w.en.isEmpty && !RussianText.letters(in: w.ru).isEmpty { keep(w.ru, en: w.en) }
            for e in line.correction?.edits ?? [] where !RussianText.key(e.replacement).isEmpty && !RussianText.letters(in: e.replacement).isEmpty {
                keep(e.replacement, en: e.en ?? "")
            }
        }
        let clean = conversation.learnerLines.allSatisfy { ($0.correction?.edits ?? []).isEmpty }
        record(ref) { r in
            r.talk = conversation
            r.status = clean ? .correct : .done
        }
        requestAdvance(page: ref.pageID)
    }

    /// Ended without the pen (no network afterwards, or the learner chose so): finished, ungraded.
    func keepTalkUnmarked(_ ref: ExerciseRef) {
        guard result(ref)?.talk != nil else { return }
        record(ref) { $0.status = .done }
        requestAdvance(page: ref.pageID)
    }

    /// Nothing said: left for now.
    func skipTalk(_ ref: ExerciseRef) {
        record(ref) { $0.status = .skipped }
        requestAdvance(page: ref.pageID)
    }
}
