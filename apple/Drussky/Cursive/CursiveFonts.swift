import CoreText
import Foundation
import UIKit

/// The bundled handwriting fonts (`Resources/Fonts/`), each OFL-licensed with its licence text
/// beside it and listed in `fonts.json`. They stand in for other people's hands when the learner
/// reads handwriting; the school hand itself is the stroke models.
enum CursiveFonts {
    struct Entry: Decodable, Sendable, Equatable {
        var file: String
        var family: String
        var postscript: String
        var licence: String
        var licenceFile: String
        var source: String
    }

    static func manifest(bundle: Bundle = .main) -> [Entry] {
        guard let url = bundle.url(forResource: "fonts", withExtension: "json"), let data = try? Data(contentsOf: url),
              let entries = try? JSONDecoder().decode([Entry].self, from: data) else { return [] }
        return entries
    }

    /// Registers every manifest font with the process (idempotent; already-registered fonts are skipped).
    static func register(bundle: Bundle = .main) {
        for e in manifest(bundle: bundle) {
            let name = (e.file as NSString).deletingPathExtension
            let ext = (e.file as NSString).pathExtension
            guard let url = bundle.url(forResource: name, withExtension: ext) else { continue }
            CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
        }
    }

    /// PostScript names of the fonts that loaded and have a Cyrillic ж, in manifest order.
    static var names: [String] {
        manifest().map(\.postscript).filter { hasCyrillic($0) }
    }

    static func hasCyrillic(_ postscript: String) -> Bool {
        guard let font = UIFont(name: postscript, size: 20) else { return false }
        var glyphs = [CGGlyph](repeating: 0, count: 2)
        let chars: [UniChar] = Array("жЖ".utf16)
        return CTFontGetGlyphsForCharacters(font as CTFont, chars, &glyphs, 2) && glyphs.allSatisfy { $0 != 0 }
    }
}
