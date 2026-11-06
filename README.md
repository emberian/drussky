# Rufren

A personal Russian learning environment for Ember, from zero Russian to conversation and independent reading of science fiction, literature, mathematics and philosophy. Speaking, listening, reading and writing are first-class goals; native-like pronunciation remains an ambition to work toward and evaluate.

The AI teacher carries instruction, explanations, examples, corrections, practice and review. Conversations with the friend over meals are optional real-world use, without preparation or grading duties. Rust + Dioxus is the selected implementation, with Apple devices first and Android, Linux, Windows and web in the destination.

## Current capabilities

- **Learn and respond:** teacher-led lessons proceed one purposeful step at a time, with optional explanation and examples available when wanted. Reading, writing, listening and speaking practice stays tied to its exact question and response. Listening keeps its transcript hidden until reveal and requires actual playback before submission. Feedback returns to its original saved lesson and exercise. Navigation, guided responses and whole-answer judgments do not manufacture mastery scores.
- **Return to the same question:** delayed Teacher returns keep the original skill, question and reference, with a fresh response before optional help. Lesson drafts and scheduled returns stay separate. Unrated completion, your own recall report, assistance and exact-task AI opinions remain distinct; the fixed starter cadence is a scheduling hypothesis, not a memory or mastery estimate. Four-skill browser, timing/support and independent sync reconstruction checks pass; interval customization remains pending.
- **Hear examples and discuss your words:** generate and replay spoken examples inline, retain exact text/voice/input provenance, and cache originals for offline use. A compact recorder keeps speech practice simple, with sound tools available when wanted. Saved exact takes can be transcribed independently, then confirmed or edited before the teacher discusses their language content; stale or wrong-revision selections cannot substitute another take.
- **Inspect two sound regions:** choose original-frame intervals in one or two recordings, compare durations and shared-scale plots, replay at natural or supported slow speed, and export exact source/span bindings. These are editable listening instruments, not an accent classifier or dependable automatic formant tracker.
- **Follow interesting material:** 24 original lessons and 169 phrases connect everyday exchanges, Cyrillic, grammar, science fiction, mathematics and philosophy. Personal passages preserve source/edition information; unchanged curriculum items retain their earlier histories across pack updates.
- **Keep the work:** a searchable teaching notebook retains complete lessons and feedback as immutable, syncable documents. Each lesson keeps its local place and unfinished responses; pause to Today and resume without regenerating teaching. Practice/editor/transcript drafts, isolated browser workspaces, transparent spaced review and explicit conflicts preserve ongoing work. Notebook and original-audio exports remain separate; relay backups verify a coherent database and its referenced originals.

The private service runs prompt v6 on data protocol 4. A genuine guided Claude lesson and corrected exact-task feedback are accepted. Repeatable notebook, oral and delayed-return journeys use synthetic teaching/audio/ASR, an explicit browser clock and isolated profiles; a separate real HTTP/SQLite/fresh-client reconstruction also passes. The live correction used a deliberately wrong synthetic response to an unchanged saved lesson. These checks establish scoped behavior and editorial quality, not measured learning or learner pronunciation. [TASKS.md](TASKS.md) records exact artifacts, provenance and remaining acceptance.

Matching delayed-return Mac and iPad artifacts are built and selected, with frozen Mac signature and the live service/prompt binding verified. Native UI checks await desktop unlock; iPad signing/install awaits Ember’s acceptance of Apple’s updated Program License Agreement. Earlier Mac microphone/playback/reopen checks passed, but physical iPad behavior and environment-free Keychain reconnection are still untested. [Platform delivery](research/PLATFORM_DELIVERY.md) keeps exact receipts and boundaries.

The app is not complete against the [completion contract](COMPLETENESS.md). Acoustic/listener evaluation, independently delayed learning outcomes, full VoiceOver and interaction-state acceptance, broader device/OS use and real-host disaster recovery remain open. Scoped text, control-border, selection and keyboard-focus contrast checks pass across desktop, tablet and phone views. Visible Russian in lessons, alphabet/examples, tutor prose and transcripts has scoped exposure checks; presentation records an opportunity, not attention. Generated speech, ASR agreement and model confidence do not establish native-like learner pronunciation.

## Design and research

- [Design](DESIGN.md): learning experience, speech analysis, architecture, and evaluation.
- [Architecture and flow maps](ARCHITECTURE.md): crate boundaries, facts, storage, practice, audio, optional human feedback and provider contracts.
- [Evidence](research/EVIDENCE.md): findings, limits, and decisions they inform.
- [Exploratory stress instruments](research/SILERO_STRESS_AUDIT.md): the existing two-token Silero comparison and exact replay crops; recommendation for editable region comparison, which motivated the now-accepted original-frame two-region view, without a fitted stress classifier or dependable automatic formants.
- [Efficient learning models](research/OPTIMAL_LEARNING.md): scheduling, optimal control, pedagogical planning and experimental grammar teaching.
- [Rust platform assessment](research/RUST_PLATFORM.md) and [platform delivery](research/PLATFORM_DELIVERY.md): selection criteria, packaging/offline commands and outstanding device checks.
- [Private tutor providers](research/PROVIDERS.md): credentials, actual API path, bounds and subscription-credit limitations.
- [Private relay deployment and recovery](deploy/README.md): optional HTTPS hosting, original-audio storage and backup/restore drill.
- [Work tracker](TASKS.md) and [completion contract](COMPLETENESS.md): actual outstanding work and evidence required for the destination.

Source copies and extracted research text are in `~/paperbin/rufren/`; acquisition manifests record origins and failures. Literature searches used `~/swarm/research.py` and primary publisher/author sources. This is a targeted review, not a systematic review or evidence of comparative app superiority. Existing abcdeez work remains untouched; useful ideas are assessed in the evidence notes.

Keep personal recordings, credentials, and learner records outside source control.

## Run the browser app

Use Dioxus CLI **0.7.10**, matching the pinned library. This Mac also has an unrelated Deno command named `dx`; `.tools/dx` is the ignored project-local Dioxus binary from [the upstream release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10). `RUFREN_DX` overrides it in the build wrappers.

For development:

```sh
cd /Users/ember/dev/rufren/crates/rufren-app
../../.tools/dx serve --web --port 8080 --addr 127.0.0.1 --open false --interactive false
```

For the prepared static/offline build, from the workspace root:

```sh
scripts/platform/build-web.sh debug
node scripts/platform/serve-prod-preview.mjs target/dx/rufren-app/debug/web/public 8091
```

Open the corresponding localhost URL. Browser data belongs to that exact origin: `localhost`, `127.0.0.1`, and different ports have separate notebooks and recordings. The prepared build needs an initial online visit to install its shell; the development server is not the offline deliverable. Shell caching excludes relay responses and private content. Export before clearing site data; browser eviction can remove data despite installation. See [platform delivery](research/PLATFORM_DELIVERY.md) for updates and hosting requirements.

For native development on the Mac:

```sh
cargo run -p rufren-app --no-default-features --features desktop
```

Native facts go to the platform application data directory, or `RUFREN_DATA_DIR` when set. Original audio currently lives in a separate WebView IndexedDB database, including in the native UI: copying the SQLite database alone does not back it up. Packaging commands are `scripts/platform/build-native.sh macos`, `ios-simulator`, `ios-device`, or `android`, one at a time. Their existence and declared permissions do not establish runtime capture or signed-device readiness.

## Private relay and AI teaching

After privately setting `RUFREN_SYNC_TOKEN` to a random value of at least 32 characters:

```sh
cargo run -p rufren-relay
```

Defaults are `127.0.0.1:8787`, `private/relay.sqlite`, a sibling `private/relay.sqlite.assets` original-audio directory, a 512 MiB audio quota, and allowed browser origins `http://127.0.0.1:8080,http://localhost:8080`. Set `RUFREN_RELAY_BIND`, `RUFREN_RELAY_DB`, `RUFREN_RELAY_ASSETS`, `RUFREN_RELAY_ASSET_QUOTA_BYTES`, or comma-separated `RUFREN_ALLOWED_ORIGINS` as needed. A preview on port 8091 must be explicitly added to the allowlist. Connect in Settings with the relay URL and key. Browser credentials remain in memory; native Apple bootstrap can retain the private connection in Keychain. Environment-free physical-device relaunch still needs acceptance.

“Sync now” sends notebook facts, including complete archived teaching. “Include original recordings and timed cues” separately enables binary audio transfer to that same bound relay. Lesson selection/cursors, unfinished drafts, preferences and credentials remain local metadata. The current app, private service and selected native artifacts use data/sync protocol 4; the teaching prompt is version 6 and physical SQLite schema remains 1. Different physical devices require a reachable HTTPS relay with exact allowed origins. This is a single personal library with a shared bearer key, plaintext server storage and no end-to-end encryption. Nothing is deployed automatically.

Teacher backends are explicitly configured. The interactive CLI path uses `RUFREN_TUTOR_PROVIDER=claude_cli`, the configured absolute Claude executable and its existing native login, with Opus/high and returned model/profile provenance. Max remains an explicit preparation option. The separate Anthropic HTTP API backend retains its server-only credential and API accounting contract. Requests, saved responses, exact corrections and drafts survive interruption; there is no silent provider/model fallback or automatic paid retry.

Speech and recognition have their own explicit configuration/provenance. The main private service now uses the explicit `local_russian` Qwen/Silero router and local Qwen ASR. Milena remains an explicitly configurable separate backend, not a silent failure fallback. Exact marked-stress routing/canonical WAV/provenance and no-inference ambiguity guidance pass; automatic stress choices and acoustic quality remain provisional. See [teaching providers](research/PROVIDERS.md), [speech providers](research/SPEECH_PROVIDERS.md), [speech teacher](research/SPEECH_TEACHER.md) and [platform delivery](research/PLATFORM_DELIVERY.md) for precise current limits.

## Keep both kinds of backup

Settings exports a versioned notebook JSON containing immutable facts, including full archived teaching, completed attempts, optional human feedback and recording links. It excludes local lesson cursors/drafts/preferences, keys and original audio bytes. The speech studio exports a separate `rufren-audio-v1` backup containing original recordings, per-take metadata and timed cues. Keep both when recordings matter; a hash/link without its bytes cannot replay audio. Relay recovery likewise needs its SQLite snapshot, manifest and matching audio directory; follow the [restore drill](deploy/README.md).

## Focused checks

Coordinate one build per tree; Cargo is capped at two jobs. Native-host app tests must use `--no-default-features --features desktop`; the default web feature is for the browser target. Choose checks for the changed contract rather than repeatedly running every target:

```sh
cargo test -p rufren-core --test replication replication_
cargo test -p rufren-core --test learning learning_
cargo test -p rufren-store --test durability durability_
cargo test -p rufren-relay --test network network_
cargo test -p rufren-relay --test assets asset_
cargo test -p rufren-tutor provider_
cargo test -p rufren-app --no-default-features --features desktop --test model_journey model_journey_
cargo check -p rufren-app --target wasm32-unknown-unknown
cargo check -p rufren-app --no-default-features --features desktop
```

Content, audio instruments/cooperative analysis, draft/retry audits, browser storage, offline shell, audio relay and backup scripts provide additional scoped checks; see their crate READMEs and delivery documentation. Local logs and QA artifacts live under ignored `.local/`. Current test results, running-artifact identities, live failures and outstanding physical-device acceptance are recorded in [TASKS.md](TASKS.md).
