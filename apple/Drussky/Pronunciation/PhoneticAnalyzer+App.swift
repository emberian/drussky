import Foundation

extension PhoneticAnalyzer {
    /// The app's analyzer: the bundled phone recognizer, or, when `models.py fetch` has not
    /// been run, on-device word recognition compared sound by sound.
    static let shared = PhoneticAnalyzer(transcribe: { url, hint in await SpeechCheck.transcribe(url, hint: hint) })
}
