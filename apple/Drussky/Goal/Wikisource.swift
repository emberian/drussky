import Foundation

/// A goal text from a link: a ru.wikisource page through the MediaWiki parse API (its
/// paragraphs, without the header, footnotes and navigation), or the paragraphs of any page.
enum Wikisource {
    struct Fetched: Sendable {
        var title: String
        var author: String?
        var text: String
    }

    enum Failure: LocalizedError {
        case notAPage, noText, http(Int)
        var errorDescription: String? {
            switch self {
            case .notAPage: "That link doesn't point at a page I can read."
            case .noText: "No Russian text was found at that link."
            case .http(let code): "The page couldn't be fetched (\(code))."
            }
        }
    }

    static func isWikisource(_ url: URL) -> Bool { url.host?.hasSuffix("wikisource.org") ?? false }

    /// The page title in a /wiki/ link, decoded.
    static func pageTitle(of url: URL) -> String? {
        let path = url.path
        guard let r = path.range(of: "/wiki/") else { return nil }
        let t = String(path[r.upperBound...]).replacingOccurrences(of: "_", with: " ")
        return t.isEmpty ? nil : (t.removingPercentEncoding ?? t)
    }

    /// "Мы (Замятин)/Запись 1-ая" → title "Мы: Запись 1-ая", author "Замятин".
    static func split(title: String) -> (title: String, author: String?) {
        var parts = title.split(separator: "/").map { String($0).trimmingCharacters(in: .whitespaces) }
        guard var first = parts.first else { return (title, nil) }
        var author: String?
        if let open = first.lastIndex(of: "("), first.hasSuffix(")") {
            author = String(first[first.index(after: open)..<first.index(before: first.endIndex)])
            first = String(first[..<open]).trimmingCharacters(in: .whitespaces)
        }
        parts[0] = first
        return (parts.joined(separator: ": "), author)
    }

    static func fetch(_ url: URL) async throws -> Fetched {
        if isWikisource(url), let page = pageTitle(of: url), let host = url.host {
            var c = URLComponents()
            c.scheme = "https"; c.host = host; c.path = "/w/api.php"
            c.queryItems = [.init(name: "action", value: "parse"), .init(name: "page", value: page), .init(name: "prop", value: "text"),
                            .init(name: "format", value: "json"), .init(name: "formatversion", value: "2"),
                            .init(name: "disabletoc", value: "1"), .init(name: "disableeditsection", value: "1")]
            let (data, response) = try await URLSession.shared.data(for: request(c.url!))
            let status = (response as? HTTPURLResponse)?.statusCode ?? 0
            guard status == 200 else { throw Failure.http(status) }
            guard let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                  let parse = object["parse"] as? [String: Any], let html = parse["text"] as? String else { throw Failure.notAPage }
            let text = paragraphs(inHTML: html)
            guard !text.isEmpty else { throw Failure.noText }
            let (title, author) = split(title: (parse["title"] as? String) ?? page)
            return Fetched(title: title, author: author, text: text)
        }
        let (data, response) = try await URLSession.shared.data(for: request(url))
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard status == 200 else { throw Failure.http(status) }
        let html = String(decoding: data, as: UTF8.self)
        let text = paragraphs(inHTML: html)
        guard !text.isEmpty else { throw Failure.noText }
        let title = firstMatch(/<title[^>]*>(.*?)<\/title>/, in: html).map { decode($0) } ?? (url.host ?? "Text")
        return Fetched(title: title, author: nil, text: text)
    }

    private static func request(_ url: URL) -> URLRequest {
        var r = URLRequest(url: url)
        r.setValue("Drussky/2.0 (Russian learning workbook; ember.software)", forHTTPHeaderField: "User-Agent")
        r.timeoutInterval = 30
        return r
    }

    /// The text of every <p> that contains Cyrillic: footnote markers and tags removed, entities
    /// decoded, <br> kept as line breaks, paragraphs on their own lines.
    static func paragraphs(inHTML html: String) -> String {
        var out: [String] = []
        for m in html.matches(of: /<p\b[^>]*>(.*?)<\/p>/.dotMatchesNewlines()) {
            var p = String(m.1).replacingOccurrences(of: "\n", with: " ")
            p = p.replacing(/<sup\b[^>]*class="[^"]*reference[^"]*"[^>]*>.*?<\/sup>/.dotMatchesNewlines(), with: "")
            p = p.replacing(/<span\b[^>]*class="[^"]*(?:ws-noexport|mw-editsection)[^"]*"[^>]*>.*?<\/span>/.dotMatchesNewlines(), with: "")
            p = p.replacing(/<br\b[^>]*>/, with: "\n")
            p = p.replacing(/<[^>]+>/, with: "")
            p = decode(p)
            let lines = p.split(separator: "\n").map { $0.split(whereSeparator: { $0 == " " || $0 == "\u{00A0}" || $0 == "\t" }).joined(separator: " ") }
                .filter { !$0.isEmpty }
            let text = lines.joined(separator: "\n")
            guard text.unicodeScalars.contains(where: { (0x0400...0x04FF).contains($0.value) }) else { continue }
            out.append(text)
        }
        return out.joined(separator: "\n")
    }

    private static func firstMatch(_ re: Regex<(Substring, Substring)>, in s: String) -> String? {
        s.firstMatch(of: re).map { String($0.1) }
    }

    /// HTML entities → characters.
    static func decode(_ s: String) -> String {
        var out = s
        for (e, c) in [("&nbsp;", "\u{00A0}"), ("&amp;", "&"), ("&lt;", "<"), ("&gt;", ">"), ("&quot;", "\""), ("&#39;", "'"), ("&apos;", "'"),
                       ("&laquo;", "«"), ("&raquo;", "»"), ("&mdash;", "—"), ("&ndash;", "–"), ("&hellip;", "…")] {
            out = out.replacingOccurrences(of: e, with: c)
        }
        out = out.replacing(/&#(\d+);/) { m in UnicodeScalar(UInt32(m.1) ?? 0).map { String(Character($0)) } ?? "" }
        out = out.replacing(/&#x([0-9a-fA-F]+);/) { m in UnicodeScalar(UInt32(m.1, radix: 16) ?? 0).map { String(Character($0)) } ?? "" }
        return out
    }
}
