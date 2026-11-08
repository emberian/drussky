import SwiftUI

/// A system font at a designed size that follows Dynamic Type (scaled like `relativeTo`).
struct ScaledFont: ViewModifier {
    @ScaledMetric private var size: CGFloat
    let weight: Font.Weight
    let design: Font.Design

    init(_ size: CGFloat, weight: Font.Weight, design: Font.Design, relativeTo style: Font.TextStyle) {
        _size = ScaledMetric(wrappedValue: size, relativeTo: style)
        self.weight = weight
        self.design = design
    }

    func body(content: Content) -> some View {
        content.font(.system(size: size, weight: weight, design: design))
    }
}

extension View {
    /// `.font(.system(size:weight:design:))` that grows and shrinks with the reader's text size.
    func scaledFont(_ size: CGFloat, weight: Font.Weight = .regular, design: Font.Design = .default,
                    relativeTo style: Font.TextStyle = .body) -> some View {
        modifier(ScaledFont(size, weight: weight, design: design, relativeTo: style))
    }

    /// Russian display text (the system serif) that follows Dynamic Type.
    func russianFont(_ size: CGFloat, weight: Font.Weight = .regular) -> some View {
        modifier(ScaledFont(size, weight: weight, design: .serif, relativeTo: size >= 28 ? .title : .body))
    }
}
