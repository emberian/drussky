import Foundation
import LearningCore

extension RussianDomain {
    /// What an analysed take says about sounds, beyond the exercise's own attempt or outcome:
    /// an attempt in the see → say modality on each sound contrast the take got wrong, carrying
    /// a speech confusion for each letter that came out as another (ы said as и: letter:Ы →
    /// letter:И). A stress on the wrong syllable touches `sound:stress`. nil when the take got
    /// none wrong: an analysed take is not a recall grade, the exercise's outcome is.
    /// The id is the take's file, so a take is never counted twice.
    func speechEvidence(page: BookPage, index: Int, take: String, report: PronunciationReport,
                        attempt: Int, at t: Date, source: String = "live") -> LearningEvent? {
        guard page.exercises.indices.contains(index) else { return nil }
        // A word-level report compared the recognizer's spelling with the target's: that «бил» was
        // written for «был» is a hypothesis about what was heard, not a measurement of the ы. Only
        // phone-level analysis asserts sound defects, so only it leaves sound evidence.
        guard !report.isWordLevel else { return nil }
        let ex = page.exercises[index]
        let target = report.target
        var contrasts: [String] = []
        var letters: [ComponentID] = []
        var confusions: [Confusion] = []
        func note(_ c: String?) { if let c, !contrasts.contains(c) { contrasts.append(c) } }
        for s in report.sounds where s.verdict == .substituted || s.verdict == .missing {
            guard let want = WordLevelAnalyzer.letter(of: s, in: target) else { continue }
            let got = WordLevelAnalyzer.heardLetter(s)
            let tipContrast = report.tips.first { t in t.letterRange.map { r in s.letterRange.map(r.overlaps) ?? false } ?? false }?.contrast
            note(tipContrast.flatMap(SoundContrasts.id(for:)) ?? SoundContrasts.id(expected: want, heard: got))
            let wantID = ComponentID(.letter, String(want).uppercased())
            if !letters.contains(wantID) { letters.append(wantID) }
            if let got, got != want {
                confusions.append(Confusion(target: wantID, produced: ComponentID(.letter, String(got).uppercased()), channel: .speech))
            }
        }
        for tip in report.tips { note(tip.contrast.flatMap(SoundContrasts.id(for:))) }
        guard !contrasts.isEmpty || !confusions.isEmpty else { return nil }
        let components = contrasts.map { ComponentID(.sound, $0) } + letters
        return LearningEvent(id: "speech:\(take)", t: t, type: .attempt, source: source, item: "\(page.id)#\(index)", activity: ex.kind.rawValue,
                             modality: .say, components: components, correct: false, attempt: attempt, expected: target,
                             response: report.transcript, confusions: confusions.isEmpty ? nil : confusions,
                             review: ex.reviewKey != nil ? true : nil)
    }
}
