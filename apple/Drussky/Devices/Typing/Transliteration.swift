import Foundation

/// Phonetic Russian on a Latin keyboard, for answering with a hardware keyboard before the
/// ЙЦУКЕН layout is learned. The whole Latin buffer is re-read on every keystroke, longest
/// match first, so `s` → с becomes ш the moment `h` follows.
///
/// The scheme (lowercase shown; a capital Latin letter gives the capital Cyrillic one):
///
///     a а   b б   v в   g г   d д   e е   eh э   yo ё   zh ж   z з   i и   j й   k к
///     l л   m м   n н   o о   p п   r р   s с   t т   u у   f ф   h х   x х   c ц
///     ch ч  sh ш  sch/shch/shh щ  y ы   yu ю  ya я   ' ь   '' ъ  (or # ъ)
///
/// `y` alone is ы and `j` is й, so `novyj` is новый and `moj` is мой; `ya`/`yu`/`yo` are the
/// soft vowels (ы is never followed by those sounds). `e` is е; э takes `eh` (`eto` is ето, `ehto`
/// is это). Anything else (digits, punctuation, spaces) passes through unchanged.
enum Transliteration {
    /// Multi-letter sequences first, so they win over their prefixes.
    static let table: [(String, String)] = [
        ("shch", "щ"), ("sch", "щ"), ("shh", "щ"),
        ("zh", "ж"), ("ch", "ч"), ("sh", "ш"), ("yo", "ё"), ("yu", "ю"), ("ya", "я"), ("eh", "э"), ("''", "ъ"),
        ("a", "а"), ("b", "б"), ("v", "в"), ("g", "г"), ("d", "д"), ("e", "е"), ("z", "з"), ("i", "и"), ("j", "й"),
        ("k", "к"), ("l", "л"), ("m", "м"), ("n", "н"), ("o", "о"), ("p", "п"), ("r", "р"), ("s", "с"), ("t", "т"),
        ("u", "у"), ("f", "ф"), ("h", "х"), ("x", "х"), ("c", "ц"), ("y", "ы"), ("w", "в"), ("q", "я"),
        ("'", "ь"), ("#", "ъ"),
    ]

    /// Latin text → Russian, by the scheme above.
    static func russian(_ latin: String) -> String {
        let chars = Array(latin)
        var out = ""
        var i = 0
        while i < chars.count {
            var matched = false
            for (key, value) in table {
                let n = key.count
                guard i + n <= chars.count else { continue }
                let piece = String(chars[i..<(i + n)])
                guard piece.lowercased() == key else { continue }
                // Capitalised when the first Latin letter is: Zh → Ж, SH → Ш.
                out += chars[i].isUppercase ? value.uppercased() : value
                i += n
                matched = true
                break
            }
            if !matched { out.append(chars[i]); i += 1 }
        }
        return out
    }

    /// The Latin keys that produce a Cyrillic letter (the shortest spelling), for the hint under
    /// the keyboard map in phonetic mode.
    static func latin(for cyrillic: Character) -> String? {
        let lower = String(cyrillic).lowercased()
        return table.filter { $0.1 == lower }.min { $0.0.count < $1.0.count }?.0
    }
}
