import Foundation
import FoundationModels

// Availability probe for Apple's on-device model (compile: see ONDEVICE_TUTOR.md).
@main
struct Probe {
    static func main() async {
        let m = SystemLanguageModel.default
        print("availability:", m.availability)
        print("supportedLanguages:", m.supportedLanguages.map { $0.maximalIdentifier }.sorted())
        print("russian supported:", m.supportedLanguages.contains(where: { $0.languageCode?.identifier == "ru" }))
        guard case .available = m.availability else { return }
        let session = LanguageModelSession(instructions: "You are a Russian tutor. Answer in English in one sentence.")
        let t0 = Date()
        do {
            let r = try await session.respond(to: "What does the Russian word «книга» mean?")
            print("response:", r.content, "| \(Date().timeIntervalSince(t0))s")
        } catch { print("error:", error) }
    }
}
