import Foundation
import Security

/// Asks Claude (Messages API, streamed) for a short tutor answer about the current exercise.
enum Tutor {
    /// A Russian phrase the tutor marked up as [[ру́сский|english]].
    struct Phrase: Hashable, Sendable {
        var ru: String
        var en: String
    }

    /// Phrases in an answer: the tutor's [[ru|en]] markup, or bare Cyrillic runs when it forgot.
    static func phrases(in text: String) -> [Phrase] {
        var out: [Phrase] = []
        let marked = /\[\[([^|\]]+)\|([^\]]*)\]\]/
        for m in text.matches(of: marked) {
            out.append(Phrase(ru: String(m.1).trimmingCharacters(in: .whitespaces), en: String(m.2).trimmingCharacters(in: .whitespaces)))
        }
        if out.isEmpty {
            let cyrillic = /[А-Яа-яЁё\u0301]+(?:[ -][А-Яа-яЁё\u0301]+)*/
            for m in text.matches(of: cyrillic.matchingSemantics(.unicodeScalar)) { out.append(Phrase(ru: String(m.0), en: "")) }
        }
        var seen = Set<String>()
        return out.filter { seen.insert(RussianText.key($0.ru)).inserted }
    }

    /// Markdown for display: markup becomes a tappable link that speaks the Russian.
    static func displayMarkdown(_ text: String) -> String {
        text.replacing(/\[\[([^|\]]+)\|([^\]]*)\]\]/) { m in
            let ru = String(m.1)
            let q = ru.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? ru
            let en = String(m.2)
            return "[\(ru)](drussky-say:?t=\(q))" + (en.isEmpty ? "" : " (\(en))")
        }
    }

    static let model = "claude-opus-5-5"

    struct Turn: Codable, Sendable, Hashable {
        var role: String   // "user" | "assistant"
        var text: String
        /// Who answered (OnDevice/TutorBackend.swift); nil on questions and on notes from before it was recorded.
        var provenance: TutorProvenance? = nil
    }

    enum Failure: LocalizedError {
        case noKey, http(Int, String), refused
        var errorDescription: String? {
            switch self {
            case .noKey: "Add your Anthropic API key in Settings to ask questions."
            case .http(let code, let body): "The tutor couldn't answer (\(code)). \(body.prefix(160))"
            case .refused: "The tutor declined to answer that one."
            }
        }
    }

    static func system(context: String) -> String {
        """
        You are the tutor inside Drussky, a Russian exercise book used by one adult learner, Ember: \
        quick, curious, a beginner in Russian, interested in linguistics, science fiction, mathematics \
        and philosophy. Working memory is sometimes stretched, so keep answers short and concrete: \
        2–6 sentences, or a tiny table/list when it really helps. Answer in English. Write every Russian \
        word in Cyrillic with a stress mark (´) on words of two or more syllables, and give a rough \
        pronunciation only when asked or when it is the point. Prefer one vivid example over a rule list. \
        If the learner's answer was wrong, say exactly what differs. Don't use letters they haven't met \
        unless needed; if you do, say so briefly. No preamble, no sign-off. \
        Mark every Russian word or phrase in your answer as [[ру́сский|english]], with stress marks: \
        the learner taps it to hear it and can keep it in their workbook. Use **bold** and short lists freely.

        Where the learner is right now:
        \(context)
        """
    }

    static func stream(_ turns: [Turn], context: String) -> AsyncThrowingStream<String, Error> {
        stream(turns, system: system(context: context), maxTokens: 4000, effort: "low")
    }

    static func stream(_ turns: [Turn], system systemPrompt: String, maxTokens: Int, effort: String,
                       model: String = Tutor.model) -> AsyncThrowingStream<String, Error> {
        AsyncThrowingStream { continuation in
            let task = Task {
                do {
                    guard let key = Keychain.apiKey, !key.isEmpty else { throw Failure.noKey }
                    var request = URLRequest(url: URL(string: "https://api.anthropic.com/v1/messages")!)
                    request.httpMethod = "POST"
                    request.setValue("application/json", forHTTPHeaderField: "content-type")
                    request.setValue(key, forHTTPHeaderField: "x-api-key")
                    request.setValue("2026-10-21", forHTTPHeaderField: "anthropic-version")
                    request.setValue("server-side-fallback-2026-07-01", forHTTPHeaderField: "anthropic-beta")
                    request.timeoutInterval = 120
                    let body: [String: Any] = [
                        "model": model,
                        "max_tokens": maxTokens,
                        "stream": true,
                        "fallbacks": "default",
                        "output_config": ["effort": effort],
                        "system": systemPrompt,
                        "messages": turns.map { ["role": $0.role, "content": $0.text] },
                    ]
                    request.httpBody = try JSONSerialization.data(withJSONObject: body)
                    let (bytes, response) = try await URLSession.shared.bytes(for: request)
                    let status = (response as? HTTPURLResponse)?.statusCode ?? 0
                    if status != 200 {
                        var text = ""
                        for try await line in bytes.lines { text += line; if text.count > 600 { break } }
                        throw Failure.http(status, text)
                    }
                    for try await line in bytes.lines {
                        guard line.hasPrefix("data: "), let data = line.dropFirst(6).data(using: .utf8),
                              let event = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { continue }
                        switch event["type"] as? String {
                        case "content_block_delta":
                            if let delta = event["delta"] as? [String: Any], delta["type"] as? String == "text_delta",
                               let text = delta["text"] as? String {
                                continuation.yield(text)
                            }
                        case "message_delta":
                            if let delta = event["delta"] as? [String: Any], delta["stop_reason"] as? String == "refusal" {
                                throw Failure.refused
                            }
                        case "error":
                            let message = ((event["error"] as? [String: Any])?["message"] as? String) ?? "stream error"
                            throw Failure.http(0, message)
                        default: break
                        }
                    }
                    continuation.finish()
                } catch {
                    continuation.finish(throwing: error)
                }
            }
            continuation.onTermination = { _ in task.cancel() }
        }
    }
}

/// The API key lives in the Keychain, never in the notebook export.
enum Keychain {
    private static let service = "software.ember.drussky"
    private static let account = "anthropic-api-key"

    static var apiKey: String? {
        get {
            let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
                                        kSecAttrAccount as String: account, kSecReturnData as String: true]
            var out: AnyObject?
            guard SecItemCopyMatching(query as CFDictionary, &out) == errSecSuccess, let data = out as? Data else { return nil }
            return String(data: data, encoding: .utf8)
        }
        set {
            let base: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
                                       kSecAttrAccount as String: account]
            SecItemDelete(base as CFDictionary)
            guard let newValue, !newValue.isEmpty else { return }
            var add = base
            add[kSecValueData as String] = Data(newValue.utf8)
            add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
            SecItemAdd(add as CFDictionary, nil)
        }
    }
}
