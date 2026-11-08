import SwiftUI
import UIKit

/// Drussky's look: a Russian school exercise book (тетрадь) by day — cream paper, a faint
/// blue grid, a red margin rule, red-pen marks — and a luminous slate desk by night.
enum Theme {
    static func dynamic(_ light: UInt32, _ dark: UInt32, alpha: CGFloat = 1, darkAlpha: CGFloat? = nil) -> UIColor {
        UIColor { t in
            t.userInterfaceStyle == .dark ? UIColor(hex: dark, alpha: darkAlpha ?? alpha) : UIColor(hex: light, alpha: alpha)
        }
    }

    static let paperUI = dynamic(0xFBF8F0, 0x0E1621)
    static let deskUI = dynamic(0xE9E3D6, 0x080D14)
    static let gridUI = dynamic(0x6F93C8, 0x58C8E8, alpha: 0.16, darkAlpha: 0.10)
    static let ruleUI = dynamic(0x3D5F94, 0x7FD6F0, alpha: 0.45, darkAlpha: 0.42)
    static let marginUI = dynamic(0xD8524A, 0xFF6B5B, alpha: 0.55, darkAlpha: 0.45)
    static let inkUI = dynamic(0x1B2B4B, 0xE8F3FF)
    static let ghostUI = dynamic(0x6F93C8, 0x7FD6F0, alpha: 0.30, darkAlpha: 0.28)
    static let textUI = dynamic(0x222831, 0xE4ECF4)
    static let mutedUI = dynamic(0x6A6F78, 0x8FA0B3)
    static let accentUI = dynamic(0x2B4C7E, 0x6CC9E8)
    static let redPenUI = dynamic(0xC8372D, 0xFF7A68)
    static let goodUI = dynamic(0x2E7D4F, 0x5EE2A0)
    static let cardUI = dynamic(0xFFFFFF, 0x152131, alpha: 0.75, darkAlpha: 0.85)

    static let paper = Color(paperUI)
    static let desk = Color(deskUI)
    static let ink = Color(inkUI)
    static let text = Color(textUI)
    static let muted = Color(mutedUI)
    static let accent = Color(accentUI)
    static let redPen = Color(redPenUI)
    static let good = Color(goodUI)
    static let card = Color(cardUI)
    static let rule = Color(ruleUI)
    static let grid = Color(gridUI)
    static let margin = Color(marginUI)

    static let gridStep: CGFloat = 22

    /// Russian display text: New York, the system serif.
    static func russian(_ size: CGFloat, weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight, design: .serif)
    }
    static func cursive(_ size: CGFloat) -> Font { .custom("MarckScript-Regular", size: size) }
    static func label(_ size: CGFloat = 13) -> Font { .system(size: size, weight: .semibold, design: .rounded) }

    /// The font used for writing models on ruled paper.
    static func modelFont(size: CGFloat, cursive: Bool) -> UIFont {
        if cursive, let f = UIFont(name: "MarckScript-Regular", size: size * 1.12) { return f }
        let base = UIFont.systemFont(ofSize: size, weight: .regular)
        return base.fontDescriptor.withDesign(.rounded).map { UIFont(descriptor: $0, size: size) } ?? base
    }

    /// Russian with its stressed vowel shown in the accent colour rather than with a combining
    /// acute — at display sizes и́ is too easily read as й.
    static func stressed(_ text: String, show: Bool, color: Color = Theme.accent) -> AttributedString {
        var out = AttributedString()
        for ch in text {
            // "а́" is one Character (vowel + U+0301); strip the mark and colour the vowel.
            let scalars = ch.unicodeScalars.filter { $0 != "\u{0301}" }
            guard !scalars.isEmpty else { continue }
            var piece = AttributedString(String(String.UnicodeScalarView(scalars)))
            if show && scalars.count != ch.unicodeScalars.count {
                piece.foregroundColor = color
                piece.underlineStyle = .single
            }
            out += piece
        }
        return out
    }

    static func registerFonts() {
        guard let url = Bundle.main.url(forResource: "MarckScript-Regular", withExtension: "ttf") else { return }
        CTFontManagerRegisterFontsForURL(url as CFURL, .process, nil)
    }
}

extension UIColor {
    convenience init(hex: UInt32, alpha: CGFloat = 1) {
        self.init(red: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255,
                  blue: CGFloat(hex & 0xFF) / 255, alpha: alpha)
    }
}

/// The squared exercise-book background with a red margin rule.
struct GridPaper: View {
    var marginX: CGFloat?

    var body: some View {
        Canvas { ctx, size in
            let step = Theme.gridStep
            var path = Path()
            var x: CGFloat = 0
            while x <= size.width { path.move(to: .init(x: x, y: 0)); path.addLine(to: .init(x: x, y: size.height)); x += step }
            var y: CGFloat = 0
            while y <= size.height { path.move(to: .init(x: 0, y: y)); path.addLine(to: .init(x: size.width, y: y)); y += step }
            ctx.stroke(path, with: .color(Theme.grid), lineWidth: 0.6)
            if let marginX {
                var m = Path()
                m.move(to: .init(x: marginX, y: 0)); m.addLine(to: .init(x: marginX, y: size.height))
                ctx.stroke(m, with: .color(Theme.margin), lineWidth: 1.2)
            }
        }
        .background(Theme.paper)
        .ignoresSafeArea()
    }
}

/// A round speaker button that shows when it is playing.
struct HearButton: View {
    @Environment(Voice.self) private var voice
    var text: String
    var size: CGFloat = 44
    var label: String? = nil

    var body: some View {
        let playing = voice.speakingKey == RussianText.key(text)
        Button {
            playing ? voice.stop() : voice.play(text)
        } label: {
            HStack(spacing: 8) {
                Image(systemName: playing ? "speaker.wave.3.fill" : "speaker.wave.2.fill")
                    .symbolEffect(.variableColor.iterative, isActive: playing)
                    .font(.system(size: size * 0.42, weight: .semibold))
                    .frame(width: size, height: size)
                    .background(Circle().fill(Theme.accent.opacity(0.12)))
                if let label { Text(label).font(Theme.label(14)) }
            }
            .foregroundStyle(Theme.accent)
            .overlay(alignment: .topTrailing) {
                let voices = voice.speakerCount(text)
                if voices > 1 && label == nil {
                    Text("\(voices)")
                        .font(.system(size: 9, weight: .bold, design: .rounded))
                        .foregroundStyle(.white)
                        .frame(width: 15, height: 15)
                        .background(Circle().fill(Theme.accent))
                        .offset(x: 3, y: -3)
                        .accessibilityLabel("\(voices) voices")
                }
            }
            .frame(minWidth: 44, minHeight: 44)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label ?? "Hear \(RussianText.unstressed(text))")
    }
}

/// Small rounded control for secondary actions under a writing strip.
struct PillButton: View {
    var title: String
    var systemImage: String?
    var prominent = false
    var tint: Color = Theme.accent
    var action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                if let systemImage { Image(systemName: systemImage) }
                Text(title)
            }
            .font(Theme.label(14))
            .padding(.horizontal, 14).padding(.vertical, 9)
            .frame(minHeight: 44)
            .foregroundStyle(prominent ? Color.white : tint)
            .background(Capsule().fill(prominent ? tint : tint.opacity(0.10)))
        }
        .buttonStyle(.plain)
    }
}
