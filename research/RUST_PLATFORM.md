# Rust platform assessment

Checked 2026-11-05 against upstream documentation. Ember subsequently selected Dioxus, and the [implemented foundation](../ARCHITECTURE.md) now runs in the browser and passes the native Mac build check. Rust owns the shared learning model, content representation, and future scheduling/signal processing. The alternatives below preserve the selection rationale; physical-device audio validation remains open.

The destination includes iPhone/iPad/Mac first and Android/Linux/Windows/web later. This changes the earlier Apple-only recommendation. “Rust application” can mean application code in Rust while using OS APIs, browser infrastructure, and some platform bindings. A literal zero-foreign-code stack is a different and much less useful requirement.

## Candidates

| Route | Relevant upstream facts | Rufren judgment |
| --- | --- | --- |
| Dioxus | Rust UI/application code; desktop and mobile WebViews; browser target; experimental WGPU mobile rendering | First candidate for rich reading, writing, and cross-platform UI. Use the documented WebView route initially. |
| Slint | Rust-supported iOS/Android and desktop; declarative `.slint` UI; browser builds render into a canvas | Credible alternative for a consistently rendered interface. Web documentation lists accessibility and normal-browser-integration limits, which matter for a reading app. |
| Xilem | Upstream still identifies the framework as experimental | Existing abcdeez experience is useful, but is not evidence of a ready, complete six-platform route. |
| SwiftUI plus Rust core | Native Apple UI, with shared Rust exposed through a binding boundary | Fallback for Apple usability; retains learning/audio work but requires other frontends for the broader destination. |

Dioxus desktop runs Rust natively while rendering in a WebView. Its documentation explicitly warns that browser APIs are not directly available to native Rust and describes JavaScript evaluation as an escape hatch. Do not promise that every capability can be written without any bridge code. Mobile documentation lists WebView rendering and experimental WGPU rendering. [Desktop](https://dioxuslabs.com/learn/0.7/guides/platforms/desktop/), [mobile](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/), [0.7 release](https://dioxuslabs.com/blog/release-070/).

Slint documents iPhone, iPad, and simulator support through Winit and Skia. Its web target uses a canvas rather than DOM text, and currently documents missing screen-reader support. That is a substantive tradeoff for text selection, long-form reading, and accessibility, not just visual taste. [iOS](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/ios/), [mobile](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/general/), [web](https://docs.slint.dev/latest/docs/slint/guide/platforms/web/).

Xilem's upstream describes it as experimental. abcdeez already has custom iOS/Android wrappers and XCFramework integration, but its README includes limited-platform testing and historical toolchain details. Those are leads to inspect, not wrappers to copy unquestioningly. [Upstream](https://github.com/linebender/xilem).

## Shared boundaries

- A platform-independent Rust core owns curriculum state, attempt events, scheduling, and model/policy versions.
- A signal-processing component consumes PCM plus timing/sample-rate metadata and emits measurements and display data. Keep this independent of microphone APIs and UI rendering.
- An audio adapter owns capture, playback, device routing, permissions, and interruptions. CPAL is a candidate abstraction, with platform-specific behavior to validate. Browser capture has a separate environment and must not be assumed equivalent to desktop capture. [CPAL documentation](https://docs.rs/cpal/latest/cpal/).
- Storage/network adapters allow native files and a browser-appropriate store to share the same domain model. Core logic must not require unrestricted filesystem access.
- The UI consumes downsampled display data. Do not route every microphone sample through a WebView message or allocate UI objects on an audio callback.

## The feasibility test that should decide the UI

Build one thin vertical slice before investing in many screens:

1. Display a selectable Russian passage with stress annotations, a gloss, and an editable Cyrillic answer. Test combining accents, selection, copy/paste, keyboard switching, and soft-keyboard layout.
2. Record a short clip, persist it, reload it, and play reference/learner clips. Handle denied permission, an interruption, and a changed audio route.
3. Stream an inexpensive level display and a spectrogram without audible glitches or runaway memory. Benchmark actual bridge/rendering cost.
4. Reopen the app and recover the lesson. Verify the core and a browser view share content/event semantics.
5. Run on Mac, an iPhone/iPad device, and a browser; make an Android emulator build early. A simulator result does not validate phone microphone/routing behavior. Add Windows/Linux smoke coverage before claiming those ports work.

Use a narrow build on the Mac and respect `~/swarm/RULES.md` and designated hosts if heavier work becomes necessary. This assessment alone does not require installing SDKs or running broad builds.

Success means a pleasant, reliable lesson on real devices. If the Dioxus route requires disproportionate glue for core text/audio interactions, compare Slint or use SwiftUI while retaining the Rust core. Keep the decision tied to observed behavior rather than to framework slogans.
