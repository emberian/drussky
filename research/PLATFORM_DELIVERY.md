# Platform delivery

Delivery checked 2026-11-06 against Dioxus CLI **0.7.10**, the locked Wry **0.53.5**, and the SDK files on this Mac. The source of truth for packaging is [`Dioxus.toml`](../crates/rufren-app/Dioxus.toml). A build or simulator launch does not establish physical-device microphone, routing, keyboard, or suspension behavior.

Current selection: `.local/ipad-delivery/frozen-v6-teacher-revisits/`, client event/sync protocol 4, physical SQLite schema 1, content `2026-11-05.4` / 24 lessons, paired with the accepted server policy `rufren-ai-teacher-v6`. The actual CLI response provenance is `rufren-ai-teacher-v6+cli-structured-v1`. Web remains `35002145a785dcd4330d`; both native executables are byte-identical to the preserved v5 revisit clients because the changed prompt code is server-only. Generic/v4 data aliases select this v6 pairing; the v3 data alias and every previous freeze remain preserved. iPad provisioning is still blocked by the existing Apple agreement request, and native UI regression remains pending the locked Mac desktop.

## Browser delivery

The checked-in `index.html` supplies a local, language-tagged document; Dioxus injects its actual loader. It removes the default debug HTML's external font import. `public/` holds an SVG drawn without font dependencies, its reproducible PNG derivatives (including an Apple touch icon), install manifest, and registration script.

Run from the repository root, after coordinating the one-build-per-tree slot:

```sh
scripts/platform/build-web.sh debug
# Use release instead when producing the optimized deliverable.
node scripts/platform/serve-prod-preview.mjs target/dx/rufren-app/debug/web/public 8091
```

The wrapper requires `.tools/dx` to report 0.7.10. `RUFREN_DX=/absolute/path/to/dx` overrides the executable. **Do not assume PATH's `dx` is Dioxus**: this Mac also has Deno's unrelated command, while `~/.cargo/bin/dx` is an older Dioxus version.

`prepare-web.mjs` runs after Dioxus has emitted the build. It enumerates the actual `.wasm`, JavaScript (including wasm-bindgen snippets), CSS, font and image names under `assets/`, `wasm/`, and `icons/`; it does not guess filename hashes. It adds the manifest and registration tags to the final HTML and writes a deterministic, content-derived worker version and `offline-build.json`. Repeating preparation without changing the build yields the same version. Development-server HTML is rejected. Preparing a build alone does not register its worker in the live development preview.

The service worker installs the complete shell before becoming eligible to activate. A failed installation removes only that incomplete cache, preserving the preceding version. Once controlled, navigation to the app root and exact static files use the same cached version, including while offline. The very first visit must be online long enough for installation; the browser must permit service workers and storage. Browser storage eviction can remove the shell or notebook; installation does not make backups unnecessary.

Updates wait until **all Rufren windows/tabs are closed**. The existing app shows a dismissible update notice and keeps running; it never reloads a draft or recording. On activation the worker removes only previous Rufren shell caches for its own scope. The notebook's database and user recordings are separate from shell caches and are never deleted by update code.

There is no runtime response cache. Only GET requests for the enumerated shell assets and exact root navigation are intercepted. Requests with an Authorization header, query parameters, another origin or another method pass straight through; relay/API paths have no fallback. Installation fetches use `credentials: omit`. Dynamic notebook content and authenticated relay responses do not enter Cache Storage.

Serve the output directory over HTTPS (localhost HTTP is supported for QA), with `.wasm` as `application/wasm`, `.js` as a JavaScript MIME type and `.webmanifest` as `application/manifest+json`. Serve `sw.js`, HTML and the manifest with revalidation (`Cache-Control: no-cache`); do not rewrite missing assets/API responses to `index.html`. The preview server supplies these behaviors. Publish an entire prepared directory atomically and retain old content-addressed assets while clients may still be on the preceding version. The current wrappers target an origin root; subpath hosting requires building with Dioxus's matching `--base-path` and exercising the same offline journey there.

```sh
node --test scripts/platform/offline-shell.test.mjs
node scripts/platform/qa-offline.mjs http://127.0.0.1:8091/
```

The browser journey uses its own clean Playwright context. It checks controlled installation, keyboard focus, Cyrillic/combining marks, an offline reload and fresh offline page, draft/hint recovery, completed-attempt persistence with assistance preserved, authenticated/API cache bypass, cache contents, and phone/tablet-width overflow. It also checks document language, the main landmark, visible form labels and current-navigation announcements, and saves an accessibility tree; this is a focused sanity check, not a full screen-reader audit. It reads Playwright from a normal local installation or `~/tools/playwright/node_modules/playwright`; `RUFREN_PLAYWRIGHT` overrides that module path. Artifacts go into ignored `.local/platform-qa/`. The script does not exercise a real relay or use personal browser storage.

`scripts/test-control-focus.cjs` adds a scoped rendered-control check using disposable passage data and one existing public synthetic WAV. It measures actual field boundaries, selection markers and authored focus rings against their solid adjacent backgrounds, verifies keyboard Tab/Shift+Tab return, and checks region-slider pointer focus, exact-frame arrow movement, allowed ARIA bounds and keyboard-help association. On actual bundled web `b75df8c3bfa4db30620f` it passed 30 paint checks (minimum 3.9887:1), including desktop/tablet/phone region focus, selected lesson semantics/check mark, and zero browser errors or API/provider attempts. Evidence and reviewed screenshots are under ignored `.local/control-focus-v4-final/`. Native popup/checkbox paints, antialiasing, VoiceOver and a complete accessibility audit remain outside this check.

```sh
RUFREN_PLAYWRIGHT_MODULE=/Users/ember/tools/playwright/node_modules/playwright \
  node scripts/test-control-focus.cjs
```

`RUFREN_QA_URL` selects the prepared preview and `RUFREN_FOCUS_OUTPUT` selects the evidence directory. `RUFREN_FOCUS_PREVIEW=1` injects current source CSS for iteration and labels the result as a source preview; omit it for bundled-artifact acceptance.

### Actual old-to-new worker acceptance

`qa-shell-update.mjs` runs two genuine prepared Dioxus artifacts on its own loopback server, with a new isolated browser profile and no relay/provider endpoints. Preserve the entire older `public/` directory before the next build overwrites it. Start the harness with that immutable directory, a **new** run directory and a control JSON file containing `{}`. Once the next prepared artifact is ready, write its absolute directory as the control file's `candidate` value; the running harness copies it before serving it. No Cargo/Dioxus build is performed by this test.

```sh
node scripts/platform/qa-shell-update.mjs \
  /absolute/immutable-old/public .local/shell-update-qa/new-run \
  .local/shell-update-qa/control.json
# In another shell, after the next genuine build finishes:
python3 - <<'PY'
import json, pathlib
candidate = pathlib.Path('target/dx/rufren-app/debug/web/public').resolve()
pathlib.Path('.local/shell-update-qa/control.json').write_text(
    json.dumps({'candidate': str(candidate)}) + '\n')
PY
```

Observed acceptance on 2026-11-05: actual `bbeafe30c15f101d80aa` → `592a0f8c692529f1f857`, with different real emitted WASM hashes. A deliberate HTTP 503 for the new WASM made the installing worker redundant and removed its incomplete cache; the old Dioxus app still booted offline. The next installation waited without reloading the unfinished response. Reloading one old tab kept the old coherent shell, closing only one old tab did not activate the update, and closing both let the new worker activate. A fresh offline app page then ran the new WASM, recovered the exact Cyrillic/combining-accent draft, support state and all 14 immutable facts, and retained the saved teacher response. Only the new shell cache remained. Authenticated offline requests were not served cached responses; no API/external request or uncaught page error occurred.

The completed assisted writing attempt was created through the real UI. Transcript/confirmation/assessment facts were **explicitly synthetic**, validated and imported through Restore JSON; the saved teacher DTO was clearly labeled as a synthetic persistence fixture through the normal browser-store adapter. No learner recording, recognition or new teacher request is claimed. Screenshots confirmed the update notice beside the retained draft and the new renderer's recovered teacher/transcript display. Full receipts, lifecycle events, HTTP requests, fixture backups and screenshots are under ignored `.local/shell-update-qa/run-01/`. A second independent run at `run-02-portable/` passed using the self-contained checked-in fixture. Both owned servers and browser contexts closed after completion. These earlier runs exercise Chromium tab/worker lifecycle; the later protocol-upgrade run below also covers a complete browser-process restart. Safari/iPad suspension and storage eviction remain separate checks.

The actual data-3 → data-4 journey passed on 2026-11-06 from preserved `a92e051c55b4661e1b06` to final `ce9e4272bbacd9c858f3`. It repeated real interrupted installation, pinned old-tab reload, waiting/activation and private-cache boundaries, then checked all **15 original event JSON strings byte for byte in authoritative IndexedDB**. The exact combining-accent draft, support cues, session/workspace identity and original teacher cache survived. The new app recovered one event-version-4 typed teacher document with the exact request/content/examples/practice/provenance/usage and unchanged document identity. Its receipt is `recovered_local_cache`: no original receive time or context fact IDs were invented.

After fully closing and restarting Chromium with the same isolated profile, the new app started offline, retained the exact original practice, and did not duplicate or alter that archive. A separate workspace reopened the saved notebook entry marked “Saved earlier” and displayed its original response through “Read the full saved lesson”; deliberate reference reading there left the first workspace's draft/support unchanged. There were zero actual API/provider calls and no uncaught page errors. Evidence, original-byte hashes, recovered archive, worker transitions and reviewed screenshots are in `.local/shell-update-qa/data3-to-data4-final-ce9-01/`. Use `RUFREN_SHELL_REQUIRE_TEACHER_ARCHIVE=1` for this upgrade assertion; the helper retains its older non-archive mode. Provisional harness failures are preserved and classified: an invalid synthetic lesson with empty practice was correctly left unarchived, and reading reference in the active practice correctly added new support. Both fixture/test-sequence issues were corrected without app or service-worker changes.

The narrower same-protocol revisit update also passed: actual data-4 `ce9e4272bbacd9c858f3` → `35002145a785dcd4330d`. `qa-revisit-shell-update.mjs` saves one exact Teacher task and a different unfinished guided-step draft through the old UI, then exercises real worker waiting, pinned old reload and activation after all old clients close. The new shell starts offline with the same document/cursor/draft identities and all six original event JSON strings unchanged in authoritative IndexedDB. Only the new UI then creates a revisit: its separate `teacher_revisit` draft/active namespaces preserve Cyrillic text, support and self-report through offline reload and a complete browser-process restart, and a second workspace gets an independent draft/session without altering either old guided draft. No provider/relay was contacted; the old saved attempt is recorded before the unconfigured connection refuses feedback locally. Final receipts and reviewed screenshot are in `.local/shell-update-qa/revisits-update-final-350021-01/`. The earlier `61194…` run remains provisional evidence. The unchanged protocol-3 migration/interrupted-install suite was not repeated for this focused check.

## Native packaging

```sh
scripts/platform/build-native.sh macos
scripts/platform/build-native.sh ios-simulator
scripts/platform/build-native.sh ios-device
scripts/platform/build-native.sh android
```

Run one at a time. These are build commands, with no simulator boot, device install, cloud host provisioning or deployment. Apple builds use deployment environment variables matching the configuration (macOS 12 / iOS 15). After Dioxus finishes, `finalize-apple-bundle.py` finds the generated app by its identity/executable and normalizes its display name to “Rufren,” preserving generated version/executable/platform metadata; the folder path remains the CLI's actual emitted path. It also supplies the Mac ICNS: this CLI build path wrote an icon reference without the actual icon. The checked-in ICNS is reproducible from the shared SVG with `generate-macos-icon.py` (librsvg + macOS iconutil). Signing comes afterward. Android manifest, Gradle and native-toolchain minima are aligned at API 24; target/compile API 36 is explicit. The Rust `mobile`/`desktop` feature is selected without the default web feature. Android builds need the installed Android Studio JDK/SDK/NDK exposed as described in the upstream mobile setup; do not embed a keystore password in `Dioxus.toml`.

`[permissions].microphone` produces `NSMicrophoneUsageDescription` for Apple and `RECORD_AUDIO` for Android. Android also declares `MODIFY_AUDIO_SETTINGS`, which Wry's audio permission callback requests, and `INTERNET` for deliberate relay sync. No camera, background recording, notifications, photo library or blanket storage permission is requested. Dioxus's generated iOS template includes both iPhone and iPad device families and rotation orientations.

For a local Mac `.app`, apply and inspect an ad-hoc signature explicitly:

```sh
scripts/platform/sign-macos-local.sh /actual/path/to/Rufren.app
python3 scripts/platform/check-apple-bundle.py macos /actual/path/to/Rufren.app --signed
python3 scripts/platform/check-apple-bundle.py ios /actual/path/to/Rufren.app
```

The local signer uses `codesign --sign -`, so it does not select a personal certificate, unlock a private key or provision an account. Its entitlement file permits sandboxed microphone capture, outgoing relay connections, Dioxus's local renderer edit-channel TCP listener, and user-selected files. Although assets use a custom protocol, the native renderer still requires `network.server`; an actual sandbox launch without it failed to bind its edit socket. Device iOS signing still requires a matching provisioning profile and suitable certificate. Distribution signing/notarization and App Store submission are separate from these local commands.

Two upstream details matter:

- Although `ApplicationConfig` comments describe merging, CLI 0.7.10's `info_plist_contents` returns a custom application plist verbatim. A partial plist would erase the generated bundle identity/executable metadata. Rufren uses the generated template plus unified permission/plist entries instead.
- The 0.7.10 macOS `microphone` entitlement shortcut emits `com.apple.security.device.microphone`. Apple's documented key is `com.apple.security.device.audio-input`; Rufren declares that literal key and supplies the same key in the local-signing entitlement file. Passing `--apple-entitlements` to `dx` also enables its signing path, which may auto-provision if an identity/profile is missing. The local script avoids that path.

The locked Wry Apple `WKUIDelegate` implements the media-capture callback and grants the WebView request; the OS still checks the purpose string and user consent. Android's `RustWebChromeClient.onPermissionRequest` asks for audio runtime permissions and grants or denies the WebView request from the result. Dioxus enables Android's HTTPS custom scheme for secure-context APIs. Apple's app uses `dioxus://index.html/`; actual `navigator.mediaDevices` availability and capture from that origin must be checked at runtime. These inspected delegates are evidence of a code path, not evidence that recording works on an iPhone.

### Observed Mac runtime

The initial native bundle built successfully with two compile jobs in 118.9 seconds at `target/dx/rufren-app/debug/macos/RufrenApp.app`. Its generated identifier, microphone purpose, deployment version and education category were inspected, and the actual deliverable received a verified ad-hoc signature with the checked-in production entitlements.

Native QA used a copy under `.local/native-platform-v1/` with a separate bundle identifier. `prepare-macos-qa.py` makes this reproducible without overwriting an existing fixture or changing the deliverable. Its QA-only sandbox exception grants just the fixture's SQLite directory, allowing `RUFREN_DATA_DIR` under `.local`; the original signing entitlements contain no exception. Keep the fixture's `fixture.json` for the exact executable, identifier, environment and entitlements needed to reopen it. Selecting an app through an automation tool may auto-launch it without that environment, so explicitly launch the fixture executable with its recorded `RUFREN_DATA_DIR` before inspection.

`RUFREN_DATA_DIR` selects **SQLite text/history storage only**. The audio adapter stores blobs in WebKit IndexedDB. Dioxus defaults to WebKit's persistent `WKWebsiteDataStore.defaultDataStore`; its `with_data_directory` setting does not select a macOS WebKit profile. A distinct QA app identifier separated this test's WebKit/sandbox profile from personal Rufren.

Observed through native UI: a Cyrillic passage saved exactly by paste; an actual 3.657-second microphone take at 44.1 kHz saved, showed quality/waveform information and entered playback. After graceful quit and explicit relaunch, the passage remained in SQLite, the same take appeared in the saved-recording menu, and selecting it reconstructed its waveform/quality display and started playback. This establishes captured-audio persistence across process launches on this Mac. It does not establish portable SQLite audio, mobile behavior, pronunciation quality, external-device routing or interruption recovery.

The initial native file-picker test selected a WAV without visibly importing it. The cause was verified in Dioxus's native interpreter: it assigns empty DOM `File` placeholders and sends picker completion directly to Rust instead of dispatching a DOM change event. The app now handles native `FormEvent`/`FileData.read_bytes()` and passes bounded original bytes into the existing decoding/storage adapter. A desktop-only launch configuration sets the window title to “Rufren”; bundle display-name/icon normalization is verified separately. These fixes require a rebuilt UI regression. Rapid synthetic character typing also dropped characters in the initial run, whereas paste preserved the complete text; an ordinary typing defect has not been established. The initial microphone/reopen result remains under ignored `.local/native-platform-v1/result.json`; no test recording was exported to the repository.

The earlier protocol-3 / content-0.4 gateway Mac build succeeded in 10.72 seconds and received a verified ad-hoc production signature. Its executable SHA-256 begins `5c1435fa1e31`; the original receipt is `.local/ipad-delivery/macos-v3-final-artifact.json`, and the production bundle is now preserved separately at `.local/ipad-delivery/frozen-v3-gateway-final/Rufren-macos.app` with `macos-v3-gateway-frozen.json`. Its isolated QA copy under `.local/native-platform-v3-final/` started a native process and initialized its own SQLite store without stderr errors. **Its UI regression is pending the user's return to the Mac:** CUA could not obtain either the Rufren or existing Xcode window, and read-only IOConsole state confirmed `CGSSessionScreenIsLocked=true`. This is a desktop availability blocker, not an observed app failure. The process and test data are preserved; no unlock bypass or extra user prompt was attempted.

The teacher/region refresh built on 2026-11-06 in 21.81 seconds and remains frozen independently at `.local/ipad-delivery/frozen-v3-teacher-regions/Rufren-macos.app`. Its executable SHA-256 is `71e5f701c2f09cdf13c480c2828425e2fb183a8d3610f2dc2cddcac06923b346`. Both the production bundle and its frozen copy passed metadata, ad-hoc signature and entitlement verification. `macos-v3-teacher-regions-artifact.json` records the exact artifact and logs. It includes bounded teacher context, rich prose/exposure tracking, inline generated voice and original-frame region UI, matching accepted web build `31471f59453c4ddeade7`. The desktop remained locked in a fresh read-only check; this bundle has no native UI pass claimed and did not replace or stop the older QA process.

The readable-UI refresh remains frozen at `.local/ipad-delivery/frozen-v3-readable-ui/Rufren-macos.app`, with executable SHA-256 `d7a041c96ea7b2c45fcf320de330b3262740a103fca59d586ad51286fef2df2e`. It built in 2.98 seconds and passed metadata and ad-hoc signature/entitlement checks on the frozen copy. This refresh incorporated only the shared/Tutor foreground and placeholder contrast adjustments after accepted web build `3c220ad5d98c19691d7d` (3,023 actual text paints across 28 scene/viewports). `macos-v3-readable-ui-artifact.json` and `v3-readable-ui-build-source.json` record exact inputs and verification. Native rendering was not rechecked while the desktop is locked; no UI or existing-process interaction was attempted.

The oral-teacher refresh built on 2026-11-06 in 9.28 seconds and remains frozen independently at `.local/ipad-delivery/frozen-v4-oral-teacher/Rufren-macos.app`. Its executable SHA-256 is `1726cc262901f16a6ba1eedaf69c12db232b5e36ebba04acb3630d50b4664568`. Both the production bundle and frozen copy passed metadata, ad-hoc signature and entitlement checks. `macos-v4-oral-teacher-artifact.json` identifies the bundle and verification logs. It includes teacher prompt `rufren-ai-teacher-v4`, oral-teacher capture/history UI and the control-boundary/focus fixes, matching its accepted web source `a92e051c55b4661e1b06`. **The folder's v4 names the teacher prompt, not the data protocol:** this archived build uses data protocol 3 and content `2026-11-05.4` / 24 lessons.

The historical guided-notebook Mac bundle is `.local/ipad-delivery/frozen-v5-guided-notebook-final/Rufren-macos.app`, executable SHA-256 `e0f664074624ae3a426ffdb42e111168023427bfb543983a40bba8bf4fdb561a`. The accepted warm retry took 1.314 seconds including packaging; production and frozen copies passed metadata, ad-hoc signature and entitlement checks. It matches final web `ce9e4272bbacd9c858f3`, teacher prompt v5 / event-sync protocol 4 / physical SQLite schema 1 / content `.4` / 24 lessons. `macos-v5-guided-notebook-final-artifact.json` is the exact receipt. The first completed pair's source receipt was rejected after a concurrent test-file change and remains preserved; the accepted retry records all 162 inputs unchanged. No locked-desktop UI interaction occurred, and the old QA process/data remain intact. Native picker/title/Cyrillic and new-feature runtime checks remain pending.

The current Mac selection is `.local/ipad-delivery/frozen-v6-teacher-revisits/Rufren-macos.app`, executable SHA-256 `f49e94a0484c884a5a1a38669cb4e02c950fcad30c552469b6cb2e098ec4d173`. Metadata and the frozen copy's ad-hoc signature/entitlements passed. The warm build/packaging took 1.266 seconds. `macos-v6-teacher-revisits-artifact.json` identifies the client and explicitly labels prompt v6 as the expected server pairing, not embedded client prompt code. Its bytes match the preserved v5 revisit Mac exactly. No native UI pass is inferred from that identity or build.

The minimum continuation checks are the visible “Rufren” title, paced Cyrillic input plus exact paste/save, native picker import of a known 48,044-byte PCM16 WAV, original SHA-256/length preservation, playback, then process reopen and replay. The exact fixture hash and pending checks are recorded in `.local/platform-native-assets/native-import-expected.json` and `.local/native-platform-v3-final/result.json`. None of those new UI checks is marked passed merely because the app compiled.

## Local capability inventory

`python3 scripts/platform/inventory.py` performs a bounded, read-only inventory. It avoids `xcrun simctl`: Xcode 27.1's shim can invoke `xcodebuild -runFirstLaunch` and install components before even listing runtimes. The initial investigation encountered this; its own first-launch child was terminated, and subsequent inventory uses existing SDK/runtime files. No simulator/device was booted for this platform lane.

Observed on 2026-11-05:

| Capability | Available files / read-only result |
| --- | --- |
| Project CLI | Dioxus 0.7.10, `.tools/dx` |
| Xcode | 27.1 |
| SDKs | macOS 27.0, iPhoneOS/iPhoneSimulator 27.1 |
| Mounted iOS runtimes | 17.4, 17.5, 18.6, 26.2 |
| Signing identities | Two valid identities; key usability and profile match not tested; identifiers omitted |
| Android | SDK platforms through 36; build-tools 36.0.0; NDK 29.0.13846066 |
| Android Java / AVD | Android Studio JDK 21; Pixel 7 API 35 AVD |
| Rust target libraries | Apple desktop/device/simulator, wasm32 and Android ARM64 present, among other installed targets |

SDK presence does not establish compatible Gradle/plugin tooling, a working packaged build, valid mobile signing or a functional device port.

## Physical iPad delivery

The user subsequently connected an iPad and explicitly requested installation. Read-only CoreDevice discovery found a paired wired iPad on iPadOS 26.4.2. Developer Mode was initially disabled; after the user enabled it and restarted the iPad, it reported enabled, with development services available and the USB tunnel connected. No existing `town.lunar.rufren` app was installed. Device identifiers and account details remain in ignored `.local/ipad-delivery/` records.

Both previously saved provisioning profiles had expired, and neither covered this iPad. A valid Apple Development identity for the existing team's account remains available; signing a small task-owned local probe succeeded without an unlock prompt, establishing private-key usability. The checked-in `Rufren.xcodeproj` is a packaging/signing wrapper: its only build phase stages the **actual compiled Dioxus Rust app**, retaining its executable and resources. It contains no replacement UI or native mock. `sign-ios-development.py` invokes Xcode automatic signing/provisioning for the explicitly selected device using the existing account, including device registration when required. It neither installs SDKs nor creates accounts or accepts legal agreements. Keep signing logs/DerivedData under `.local` and run within the build slot.

```sh
scripts/platform/build-native.sh ios-device
python3 scripts/platform/sign-ios-development.py /actual/Dioxus/RufrenApp.app \
  --team-id "$RUFREN_APPLE_TEAM" --device "$RUFREN_IPAD_DEVICE" \
  --output .local/ipad-delivery/signing
```

The iOS finalizer supplies opaque iPhone/iPad icon derivatives and normalizes `CFBundleSupportedPlatforms` for device versus simulator; the upstream 0.7.10 template includes `iPadOS` even on simulator output. The purpose string for local networking covers an optional private relay; no ATS exception or Bonjour browsing entitlement is added.

For device commands use the existing CoreDevice executable directly at `/Library/Developer/PrivateFrameworks/CoreDevice.framework/Versions/A/Resources/bin/devicectl`. Xcode's shell shim can trigger first-launch installation when framework versions differ. The real command supports JSON output, app installation, process launch and screenshot capture. Installation and launch evidence is recorded only after those steps actually succeed; this preparation is not an installed-app claim.

The first actual physical-device Rust build succeeded in 159.8 seconds with the development CA embedded. The generated bundle passed iPad family/orientation and microphone/name checks; its Mach-O reports platform iOS, minimum 15.0, SDK 27.1. The first Xcode provisioning attempt then failed with **“No Accounts: Add a new account in Accounts settings.”** Stored historical Xcode account preferences did not provide a usable current account session. Xcode's Apple Accounts settings were opened for the user to sign in; optional platform downloads were skipped. That historical account sign-in was subsequently completed; the separate PLA gate below then became the blocker. Current delivery selects the accepted v6/data-4 immutable artifact, rather than this first historical build.

After the user signed in, the CLI still returned the generic account error. Downloading profiles through Xcode and expanding its actual error revealed the underlying Apple gate: `DeveloperAPIServiceErrorDomain` code 5, **“PLA Update available.”** Apple requires the account holder to review/accept the latest Program License Agreement in their developer account before provisioning resources are accessible. The assistant does not accept that agreement. Signing/install retries use the current compatible frozen artifact after the user completes that existing PLA step.

The initial device artifact remains independently preserved under ignored `.local/ipad-delivery/frozen-v2/Rufren.app` (data protocol 2 / content lineage 0.3), with its original manifest archived as `frozen-artifact-v2.json`. **Do not install that archived v2 build against the active protocol-4 relay.**

The first protocol-3 / content-0.4 device build succeeded in 10.7 seconds and remains preserved at `.local/ipad-delivery/frozen-v3/Rufren.app`, with its receipt archived in `frozen-artifact-v3-initial.json` (executable SHA-256 begins `bdbabf053abe`).

The final gateway refresh then built for physical iOS in 5.27 seconds and is frozen separately at `.local/ipad-delivery/frozen-v3-gateway-final/Rufren.app`. Its executable SHA-256 is `211f060405b07f333c3776a1b831199d37a0ae67c59c7e8447f3c41618836c2f`. It contains protocol 3, content revision `2026-11-05.4`, and 24 lessons, matching the relay at that time. Device bundle checks and Mach-O platform/minimum/SDK checks passed (iOS / 15.0 / 27.1). `final-native-build-source.json` records selected source/config/content hashes unchanged across the serialized iPad/Mac builds, the exact gateway Rust/JavaScript inputs, Git baseline, reference web build `bbeafe30c15f101d80aa`, and unchanged development CA hash. Full artifact/build/Mach-O receipts are in `frozen-artifact-v3-final.json`.

The subsequent teacher/region refresh built for physical iOS on 2026-11-06 in 21.42 seconds and is frozen separately at `.local/ipad-delivery/frozen-v3-teacher-regions/Rufren.app`. Its executable SHA-256 is `e0b2d1a059166e2bcf7b1350e06fbce04975b836e7f144f83096080a0d2995c1`. Device bundle metadata and Mach-O checks passed: iOS platform, minimum 15.0, SDK 27.1, correct iPad family/orientations and microphone purpose. It retains protocol 3 / content `2026-11-05.4` / 24 lessons and the unchanged development CA. `frozen-artifact-v3-teacher-regions.json` identifies the artifact; `v3-teacher-regions-build-source.json` records the input hashes unchanged across the serialized iOS/Mac builds and the accepted reference web build `31471f59453c4ddeade7`.

The readable-UI refresh built for physical iOS in 2.93 seconds and remains frozen separately at `.local/ipad-delivery/frozen-v3-readable-ui/Rufren.app`. Its executable SHA-256 is `ce4fecc25531a70127fea1241586bb170fd5f67f01142ae0c4bbf5277b6bd9e4`. Device metadata and Mach-O platform/minimum/SDK checks passed. `frozen-artifact-v3-readable-ui.json` records the artifact; `v3-readable-ui-build-source.json` records 145 inputs unchanged across both builds, accepted web build `3c220ad5d98c19691d7d`, and the unchanged CA/protocol/content.

The oral-teacher refresh built for physical iOS in 9.30 seconds and is frozen at `.local/ipad-delivery/frozen-v4-oral-teacher/Rufren.app`, with executable SHA-256 `26f62ccf187c1cc0a67123143eab3a382f7591a1e1eea8e6cad3fca88a54fd7a`. Bundle metadata and Mach-O checks passed: iOS platform, minimum 15.0, SDK 27.1, iPhone/iPad family/orientations, icons and microphone purpose. `frozen-artifact-v4-oral-teacher.json` records the exact bundle and teacher prompt `rufren-ai-teacher-v4`; data protocol remains 3, content `2026-11-05.4`, and 24 lessons. `v4-oral-teacher-build-source.json` records 148 inputs unchanged across both serialized builds, accepted web build `a92e051c55b4661e1b06`, and the unchanged development CA. All readable-UI, teacher/region and earlier frozen artifacts remain preserved and their hashes were rechecked.

The protocol-3 alias `frozen-artifact-v3.json` remains this preserved oral-teacher copy. It must not be installed against the now-active protocol-4 relay. The generic active receipt and protocol-4 alias now select the v6 Teacher-revisit pairing described below. The delivery runner verifies the frozen executable hash and actual private-TLS relay protocol before any Apple action; its signing output is separated by protocol/hash, with no implicit Rust rebuild. Apple agreement acceptance remains pending; no iPad signing/provisioning/install/launch retry occurred.

### Historical guided-notebook protocol-4 cutover

The historical guided-notebook physical-iOS bundle is `.local/ipad-delivery/frozen-v5-guided-notebook-final/Rufren.app`, executable SHA-256 `5271983e92995867ca5a72476900cea91fb354db7dedabdc31b96c12d6cceab8`. The accepted warm retry took 1.328 seconds including packaging. Bundle and Mach-O checks passed: physical iOS, minimum 15.0, SDK 27.1, iPhone/iPad family/orientations, microphone purpose and icons. `frozen-artifact-v5-guided-notebook-final.json` records teacher prompt `rufren-ai-teacher-v5`, event/sync protocol 4, physical SQLite schema 1, and unchanged content `2026-11-05.4` / 24 lessons. `v5-guided-notebook-final-build-source.json` records 162 inputs unchanged across the accepted serialized pair and final web `ce9e4272bbacd9c858f3`. The first pair's receipt failed because a test source changed during capture; its files/logs remain separate, and incomplete source-verification evidence is refused by the delivery preflight.

The native supervisor staged the candidate without changing active aliases or services. After root accepted the full workspace/app contracts and actual final UI/browser migration, root made the main relay-4 cutover and verified private TLS/authenticated capabilities. The pre-cutover SQLite/audio snapshot is `.local/ipad-delivery/pre-data4-20261106T101921Z/receipt.json`: it contains 0 events and 0 audio assets, so it does not itself demonstrate migration of populated historical data. Populated legacy-event preservation is covered by the core/store tests and the real 15-fact browser upgrade above.

Own live frozen-artifact/CA/health preflight then passed against `rufren-relay/4`. Only after that result, authorized promotion set generic `frozen-artifact.json` and new data-protocol alias `frozen-artifact-v4.json` to the final guided-notebook receipt. The old `frozen-artifact-v3.json` was preserved **byte for byte**. At that cutover `status.json` recorded relay/artifact protocol 4 and the then-current prompt-v5 pairing. The later v6 promotion below replaces the generic/v4 active aliases; the Apple PLA and native UI boundaries remain pending. Proof is in `v5-guided-notebook-final-live-preflight.json`, `delivery-v5-data4-alias-activation.json`, and root's `relay-v5-data4-activation.json`. This promotion touched only task metadata after public read-only `/health`; it performed no account, signing, device or native UI action.

The inventory separated independent versions:

| Area | Protocol-4 handling |
| --- | --- |
| Apple package/signing helpers | No protocol constant; identifiers, permissions, SDK targets and existing account unchanged |
| TLS/bootstrap/Keychain | Existing URL/token/CA/origin identity retained; no protocol field, credential rotation or Keychain reset required |
| Delivery state | Generic active receipt and v4 data alias now select the v6 revisit pairing; old v3 data alias and all historical freezes remain unchanged |
| Delivery runner | Generic frozen receipt, completed stable-source proof and live TLS compatibility preflight before Apple actions; signing output separated by protocol/executable hash |
| Recovery harness | Storage owner supplies explicit protocol expectations; existing v3 receipts stay historical |
| Shell-update fixture | Original synthetic version-3 facts stay version 3; the new recovered teacher document is genuinely event version 4 |
| SQLite/backup formats | Physical `user_version=1`, TLS configuration format 1 and backup manifest formats 1/2 remain independent; no blanket replacement with 4 |

Current read-only preflight command:

```sh
python3 scripts/platform/check-delivery-artifact.py \
  .local/ipad-delivery/frozen-artifact.json \
  --expected-sync-version 4 --expected-teacher-prompt rufren-ai-teacher-v6 \
  --bootstrap .local/ipad-delivery/bootstrap.json \
  --ca-cert .local/ipad-delivery/ca.pem
```

`--offline` validates only the frozen artifact and cannot establish live compatibility. The preserved old data-3 baseline and all 16 cached assets remain under `.local/shell-update-qa/baseline-data3-a92e051c55b4661e1b06/`. Device signing/install still waits for the existing Apple agreement reply; no duplicate login request or agreement acceptance was attempted.

### Current Teacher-revisit client / server-v6 pairing

The initial v5 revisit clients remain preserved at `.local/ipad-delivery/frozen-v5-teacher-revisits/`, with their original receipts and 13.259-second iOS / 13.758-second Mac build evidence. Their UI and same-protocol shell/draft preservation passed, but one actual v5 feedback response was editorial-rejected: it treated historical guided teaching notes as current delayed-review help/first-exposure context. That server-policy pairing was not promoted.

The accepted v6 release pairing is frozen separately at `.local/ipad-delivery/frozen-v6-teacher-revisits/`. Current iOS `Rufren.app` has executable SHA-256 `79d6a113c89f7cbb41aa0844cfeae66b7c5324a1dc23f83a08ecbdc0af7bb70b`; Mac `Rufren-macos.app` has SHA-256 `f49e94a0484c884a5a1a38669cb4e02c950fcad30c552469b6cb2e098ec4d173`. Receipts are `frozen-artifact-v6-teacher-revisits.json` and `macos-v6-teacher-revisits-artifact.json`. iOS build/packaging took 1.332 seconds, and physical-iOS metadata/Mach-O checks passed (minimum 15.0, SDK 27.1, iPhone/iPad family, icons and microphone purpose). The Mac checks are recorded above.

All 168 inputs stayed unchanged across the warm pair. Relative to the v5 revisit capture, only `rufren-tutor/src/teacher.rs`, `claude_cli.rs` and `tests.rs` changed. These are gated behind the tutor's server feature; the app dependency uses `default-features=false`. `v6-teacher-revisits-build-source.json` records the source differences and **byte identity of both client executables**, plus the exact server-prompt source hash. Root's warm web build also reproduced the identical `35002145a785dcd4330d` / 16-asset output. The existing accepted UI/offline/workspace journeys therefore continue to apply; they were not pointlessly rerun. Event/sync 4, physical SQLite schema 1, content `.4` and 24 lessons are unchanged. Manifest `teacher_prompt_binding=expected-server-prompt-not-client-embedded` makes the release association explicit.

One revised actual feedback was accepted in 15.373 seconds, with exact task/reference/fact binding and no repeated false first-exposure/current-help/retention claim in that case. The unchanged saved response is `.local/teacher-revisit-v6-live/acceptance/feedback.json`, whose full provenance is exactly `rufren-ai-teacher-v6+cli-structured-v1`; the server-policy version remains bare `rufren-ai-teacher-v6`. Root activated main using the accepted rebuilt relay executable SHA-256 `06a4124385c5f0ae99d7d25e0997e75b523d0c2c97a3a92917a9ca24fdc2b756` and prompt-source SHA-256 `c09a55cb65b6f23e98223dad34e7471c4df56174e9663641a801924476ceeaad`, preserving private configuration/CA identity and data protocol 4.

Authorized promotion independently verified root's accepted activation receipt: live PID, relay executable hash, prompt-source hash matching the native source capture, accepted response bytes/hash/id and the exact full provenance above. It then repeated strict private-TLS health-4 compatibility preflight. **Health 4 alone is not proof of server prompt 6.** Generic `frozen-artifact.json` and data-protocol alias `frozen-artifact-v4.json` now select the v6 pairing; `frozen-artifact-v3.json` remains byte-identical to its historical data-3 copy. `status.json` records client data 4 and expected server prompt 6. Evidence: `relay-v6-data4-activation.json`, `v6-teacher-revisits-live-preflight.json` and `delivery-v6-teacher-revisits-alias-activation.json`.

All v5 and earlier artifacts remain preserved. Platform promotion made no further service restart, iPad/account signing, terms acceptance, device action or locked-desktop UI check. The existing Apple agreement reply and native UI regression remain outstanding.

## Delivery tracker

| Item | Evidence / remaining work |
| --- | --- |
| Shell cache boundary and failed update | Actual data3 `a92e051c55b4661e1b06` → data4 `ce9e4272bbacd9c858f3` passes interrupted installation, old-tab pinning/waiting/activation, exact old fact bytes/draft/support, typed recovered teacher archive, full offline browser-process restart and separate-workspace notebook reopening. Evidence `.local/shell-update-qa/data3-to-data4-final-ce9-01/`; no app/SW fix required. Earlier generic update and Node boundary tests also pass |
| Browser startup offline | `bbeafe30c15f101d80aa` passes 14 isolated Chromium checks: controlled 16-asset cache, fresh offline startup/reload, Cyrillic/combining-accent draft/hint/completed-attempt recovery, assistance preservation, auth/API bypass, keyboard/labels/landmarks and phone/tablet overflow. Evidence under ignored `.local/platform-qa-v3-final/`; zero browser errors. New `592a0f8c692529f1f857` additionally passes actual offline startup after worker update with the same durable work. Earlier `7add33ae5c0e25c28f73` evidence remains in `.local/platform-qa-integrated/`. The later ce9 protocol-upgrade run also passes full offline browser-process restart with exact old work; complete VoiceOver and Safari/iPad lifecycle remain separate |
| Install metadata | SVG and generated PNGs, manifest and Apple touch metadata present; manual iOS Home Screen installation not exercised |
| Mac native | Current v6 server pairing with byte-identical Teacher-revisit data-v4 clients; metadata, icon and ad-hoc production/frozen signature verified; immutable bundle and exact receipt preserved. Initial isolated sandbox UI captured, saved, reopened and played actual microphone audio across process restart; exact Cyrillic passage persisted in SQLite. Required renderer TCP entitlement fixed from actual launch failure. Native picker/title/Cyrillic and new-feature runtime regression remain pending the locked desktop; older QA process/data preserved, no new UI pass claimed |
| iPhone/iPad | Earlier device checks established pairing, user-enabled Developer Mode and development services. Current Teacher-revisit data-protocol-4 / content-0.4 24-lesson physical iOS client and metadata/Mach-O checks pass, with explicit accepted server-v6 pairing and unchanged web350 bytes; older artifacts remain archived. User signed in to Xcode; fresh provisioning remains blocked by Apple's latest Program License Agreement (DeveloperAPIServiceErrorDomain code 5), requiring user review/acceptance. Active frozen v6-paired Teacher-revisit bundle can then be signed/installed without rebuilding Rust. No new provisioning attempt or install/launch claim |
| Android | Tooling present; build, WebView consent/capture, offline reopen and lifecycle still required |
| Linux/Windows | Shared Rust targets are present; native system dependencies, packaging and runtime smoke coverage remain required on suitable hosts |

## Primary sources

- [Dioxus 0.7 mobile setup](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/)
- [CLI 0.7.10 application config](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/src/config/app.rs), [unified manifests](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/src/config/manifest.rs), [permission mapping](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/src/config/manifest_mapper.rs)
- [CLI Apple generation/signing](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/src/build/apple.rs), [iOS plist template](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/assets/ios/ios.plist.hbs), [web output/custom HTML](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/cli/src/build/web.rs)
- [Dioxus native WebView origin/navigation](https://github.com/DioxusLabs/dioxus/blob/v0.7.10/packages/desktop/src/webview.rs)
- [Wry 0.53.5 Apple delegate](https://docs.rs/crate/wry/0.53.5/source/src/wkwebview/class/wry_web_view_ui_delegate.rs), [Android permission callback](https://docs.rs/crate/wry/0.53.5/source/src/android/kotlin/RustWebChromeClient.kt)
- [Apple audio-input entitlement](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.device.audio-input), [microphone purpose string](https://developer.apple.com/documentation/bundleresources/information-property-list/nsmicrophoneusagedescription)
- [Android WebChromeClient permission callback](https://developer.android.com/reference/android/webkit/WebChromeClient#onPermissionRequest(android.webkit.PermissionRequest))
