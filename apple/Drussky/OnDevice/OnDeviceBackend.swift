import Foundation
#if canImport(FoundationModels)
import FoundationModels
#endif

/// Apple's on-device model as a TutorBackend. Absent (never available) on an OS or device
/// without the framework; the structured answers in OnDeviceSchemas.swift are mapped onto the
/// app's contracts here, through the same lenient parsers Claude's replies go through.
struct OnDeviceBackend: TutorBackend {
    var provenance: TutorProvenance { .onDevice }

    enum Failure: LocalizedError {
        case absent
        var errorDescription: String? { "The on-device tutor isn't available on this device." }
    }

    /// Why the model can't answer right now, in the learner's terms; nil when it can.
    static var unavailableReason: String? {
        #if canImport(FoundationModels)
        if #available(iOS 26.0, macOS 26.0, *) {
            switch OnDeviceModel.availability {
            case .available: return nil
            case .unavailable(.deviceNotEligible): return "This device has no on-device model."
            case .unavailable(.appleIntelligenceNotEnabled): return "Turn on Apple Intelligence in Settings to use the on-device tutor."
            case .unavailable(.modelNotReady): return "The on-device model is still downloading."
            case .unavailable: return "The on-device model isn't available right now."
            }
        }
        #endif
        return Failure.absent.errorDescription
    }

    static var isAvailable: Bool { unavailableReason == nil }

    func gloss(word: String, context: String) async throws -> TutorGloss {
        #if canImport(FoundationModels)
        if #available(iOS 26.0, macOS 26.0, *) {
            let g = try await OnDeviceModel.gloss(word: word, context: context)
            return TutorGloss(en: g.gloss.trimmingCharacters(in: .whitespacesAndNewlines),
                              lemma: Self.cyrillic(g.lemma), provenance: .onDevice)
        }
        #endif
        throw Failure.absent
    }

    func quickAnswer(question: String, context: String) async throws -> String {
        #if canImport(FoundationModels)
        if #available(iOS 26.0, macOS 26.0, *) {
            return try await OnDeviceModel.quickAnswer(question: question, context: context)
        }
        #endif
        throw Failure.absent
    }

    func correctDiary(text: String, prompt: String, briefing: String, at date: Date) async throws -> DiaryCorrection {
        #if canImport(FoundationModels)
        if #available(iOS 26.0, macOS 26.0, *) {
            let answer = try await OnDeviceModel.correctDiary(text: text, prompt: prompt, briefing: briefing)
            guard var c = await DiaryTutor.parse(Self.contractJSON(answer), fallbackText: text, at: date) else {
                throw DiaryTutor.Failure.unreadable
            }
            c.model = Self.modelName
            return c
        }
        #endif
        throw Failure.absent
    }

    func lemmatize(_ forms: [String]) async throws -> [String: Goal.LemmaEntry] {
        #if canImport(FoundationModels)
        if #available(iOS 26.0, macOS 26.0, *) {
            let table = try await OnDeviceModel.lemmatize(forms)
            return await GoalLemmaTable.entries(in: Self.contractObject(table), asked: forms)
        }
        #endif
        throw Failure.absent
    }

    /// What DiaryCorrection.model records for an on-device correction.
    static let modelName = "apple-on-device"

    /// A lemma the model wrote in Latin letters is no lemma.
    static func cyrillic(_ s: String) -> String? {
        let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, t.unicodeScalars.allSatisfy({
            !CharacterSet.letters.contains($0) || (0x0400...0x04FF).contains($0.value) || $0.value == 0x0301  // the stress mark
        }) else { return nil }
        return t
    }

    #if canImport(FoundationModels)
    /// The diary answer in DiaryTutor's JSON contract, so DiaryTutor.parse reads it exactly as
    /// it reads Claude's (loose categories, dropped empty spans, trimmed text).
    @available(iOS 26.0, macOS 26.0, *)
    static func contractJSON(_ a: OnDeviceDiaryCorrection) -> String {
        let object: [String: Any] = [
            "corrected": a.corrected,
            "edits": a.edits.map { e -> [String: Any] in
                ["span": e.span, "replacement": e.replacement, "category": contractCategory(e.category), "why": e.why, "en": e.en]
            },
            "praise": a.praise,
            "teach": a.teach.map { ["title": $0.title, "note": $0.note] },
        ]
        let data = (try? JSONSerialization.data(withJSONObject: object)) ?? Data()
        return String(decoding: data, as: UTF8.self)
    }

    /// The @Generable category spelled as the contract spells it.
    @available(iOS 26.0, macOS 26.0, *)
    static func contractCategory(_ c: OnDeviceDiaryCorrection.Category) -> String {
        switch c {
        case .wordChoice: "word-choice"
        case .wordOrder: "word-order"
        default: c.rawValue
        }
    }

    /// The lemma table in GoalLemmaTable's shape: form → [lemma, gloss].
    @available(iOS 26.0, macOS 26.0, *)
    static func contractObject(_ t: OnDeviceLemmaTable) -> [String: Any] {
        var object: [String: Any] = [:]
        for row in t.rows { object[row.form] = [row.lemma, row.gloss] }
        return object
    }
    #endif
}
