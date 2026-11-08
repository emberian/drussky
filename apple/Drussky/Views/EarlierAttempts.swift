import PencilKit
import SwiftUI

/// The writings kept when an exercise was done again, small and faint under the strip.
struct EarlierAttempts: View {
    let urls: [URL]
    var height: CGFloat = 44
    @State private var images: [UIImage] = []
    @Environment(\.colorScheme) private var scheme

    var body: some View {
        if !images.isEmpty {
            HStack(spacing: 10) {
                Text(images.count == 1 ? "Earlier try" : "Earlier tries")
                    .font(Theme.label(12)).foregroundStyle(Theme.muted)
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 12) {
                        ForEach(Array(images.enumerated()), id: \.offset) { _, image in
                            Image(uiImage: image).resizable().scaledToFit().frame(height: height)
                                .opacity(0.7)
                        }
                    }
                }
            }
            .accessibilityElement(children: .ignore)
            .accessibilityLabel("\(images.count) earlier \(images.count == 1 ? "try" : "tries") kept")
        }
        Color.clear.frame(height: 0)
            .task(id: urls.map(\.path).joined() + "\(scheme)") { images = Self.render(urls, dark: scheme == .dark) }
    }

    static func render(_ urls: [URL], dark: Bool) -> [UIImage] {
        let traits = UITraitCollection(userInterfaceStyle: dark ? .dark : .light)
        var out: [UIImage] = []
        traits.performAsCurrent {
            for url in urls {
                guard let data = try? Data(contentsOf: url), let d = try? PKDrawing(data: data), !d.strokes.isEmpty else { continue }
                out.append(d.image(from: d.bounds.insetBy(dx: -6, dy: -6), scale: 2))
            }
        }
        return out
    }
}
