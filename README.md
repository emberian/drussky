# Rufren

A personal Russian learning environment for Ember, from zero Russian to conversation and independent reading of science fiction, literature, mathematics, and philosophy. Speaking, listening, reading, and writing are first-class goals. Native-like pronunciation is an explicit ambition, not a current assessment result.

Apple devices come first, with Android, Linux, Windows, and web in the destination. Rust + Dioxus is the selected implementation. The AI teacher carries instruction, explanations, corrections, examples, pronunciation demonstrations, practice and review. Generated or curated audio is supplied by the product. Ember’s friend is a welcome conversation partner over meals, with no recording, grading or curriculum-approval duties. Working-memory supports remain adjustable. The [completion contract](COMPLETENESS.md) preserves the full destination beyond the current implementation.

## What works now

The v3 web app completes a real AI-led learning journey, accepted on **`d15099e7a027a13cc45d`**: ask for a lesson, answer an inline exercise, get feedback about that exact question, keep a phrase, supply playable speech, transcribe an exact saved take, confirm/edit the words, discuss them with the Teacher, and recover the history offline. The accepted live run used `claude-opus-5-5`/high: lesson 21.0s, written feedback 12.8s, local ASR 7.1s and confirmed-word feedback 15.5s; all 24 events restored offline with zero browser errors. The teacher caught an answer unrelated to its question, acknowledged ё/e ambiguity, and made no acoustic claim.

The current web artifact **`bbeafe30c15f101d80aa`** built in 12.98s and changes bounded gateway-error reporting; the accepted Teacher/ASR functional code is unchanged, with no extra model rerun. New native/stream/browser error checks pass: actionable speech guidance is visible, arbitrary upstream error bodies are hidden.

The main private TLS relay is active on protocol 3 with Opus/high, `local_russian` generation and local Qwen ASR. Actual Qwen neural-reference generation through the new UI also passed in 3.323s, with exact pedagogical source, explicit synthesis transformation, WAV/hash, saved asset, playback and offline replay. The active router sends authored stress marks to Silero with exact acute→`+` provenance, ordinary contextual utterance shapes to Qwen and other ordinary short text to Silero automatic controls. A real marked castle request passed in 4.059155s. Known short unmarked ambiguity returns actionable HTTP 400 before inference or allowance use; one ordinary short greeting also passed in 3.830693s with equal source/input hashes and explicit automatic controls. Generated audio/ASR do not certify stress, naturalness or a learner’s accent. The current bbeafe bundle also passed all 14 existing offline/cache/keyboard/label/phone/tablet sanity checks, with 16 correctly versioned cached assets and zero errors; full VoiceOver/contrast and old→new service-worker lifecycle are still separate.

Embedded curriculum **`2026-11-05.4`** now contains 6 units, 24 lessons, 169 phrases, 172 exercises, 50 targets and 12 sources. Per-phrase lineage preserves all 103 inherited `.3` phrases as byte-identical authored v2 Events, retaining history and all four skill review keys while adding 66 new phrases. Core now writes/syncs v3 while preserving v1/v2 reads. Actual visible-text/audio opportunities, independent transcripts, explicit transcript adoption and absent self-ratings (`NotReported`) have separate evidence semantics.

Source acceptance includes 39 core checks, selected native-host app contracts (teacher interaction 6, teacher journeys 8, original-byte reads 4, generated-reference guards 2 and model journeys 12), curriculum lineage 3/contract 9, visible-text observer 8 and workspace 11 checks. The current UI acceptance is separate evidence, not an inference from those counts. [TASKS.md](TASKS.md) identifies logs, artifacts and remaining work.

Earlier native Mac QA passed exact Cyrillic paste/save, real microphone capture/playback and original-take recovery after quit/relaunch. The **final Mac v3 bundle builds in 10.72s and its ad-hoc signature verifies**, but final picker/title/Cyrillic/import/replay UI checks are pending desktop unlock: the QA process is alive with initialized SQLite/clean stderr, while the locked desktop exposes no Rufren or Xcode window to automation. This is an external UI blocker, not evidence that those checks passed or that the app failed.

The final iPad **v3/`.4` artifact built in 5.27s**, passed device bundle/Mach-O checks and is frozen at `.local/ipad-delivery/frozen-v3-gateway-final/Rufren.app`, matching the active server. It is not signed/installed: Apple provisioning still awaits Ember’s acceptance of the updated Program License Agreement. The incompatible old v2 build is archived and must not be installed against server 3. Physical iPad behavior and environment-free Keychain reconnection remain untested; [TASKS.md](TASKS.md) records exact native hashes/receipts.

The foundation includes source-preserving drafts, explicit conflicts, separate skills/support, bounded waveform/spectrogram/pitch instruments, native SQLite, browser IndexedDB recovery, authenticated manual fact/audio sync and separate notebook/original-audio backups. The actual learning/voice workflow works; platform breadth, Russian speech-output evaluation, reliable acoustic assessment and measured long-term learning remain unfinished. There is no blanket human-approval gate or homework queue for the friend.

## Design and research

- [Design](DESIGN.md): learning experience, speech analysis, architecture, and evaluation.
- [Architecture and flow maps](ARCHITECTURE.md): crate boundaries, facts, storage, practice, audio, optional human feedback and provider contracts.
- [Evidence](research/EVIDENCE.md): findings, limits, and decisions they inform.
- [Exploratory stress instruments](research/SILERO_STRESS_AUDIT.md): the existing two-token Silero comparison and exact replay crops; recommendation for editable region comparison, without a fitted stress classifier or dependable automatic formants.
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

“Sync now” sends notebook facts. “Include original recordings and timed cues” separately enables binary audio transfer to that same bound relay. Drafts, preferences, the last AI suggestion and credentials are local metadata rather than replicated facts. Different physical devices require a reachable HTTPS relay with exact allowed origins. This is a single personal library with a shared bearer key, plaintext server storage and no end-to-end encryption. Nothing is deployed automatically.

Teacher backends are explicitly configured. The working interactive CLI path uses `RUFREN_TUTOR_PROVIDER=claude_cli`, the configured absolute Claude executable and its existing native login, with Opus/high and returned model/profile provenance. Max remains explicit for preparation; earlier max calls timed out, while real high-profile lessons now succeed. The separate Anthropic HTTP API backend retains its server-only credential and API accounting contract. Requests, saved responses, exact corrections and drafts survive interruption; there is no silent provider/model fallback or automatic paid retry.

Speech and recognition have their own explicit configuration/provenance. The main private service now uses the explicit `local_russian` Qwen/Silero router and local Qwen ASR. Milena remains an explicitly configurable separate backend, not a silent failure fallback. Exact marked-stress routing/canonical WAV/provenance and no-inference ambiguity guidance pass; automatic stress choices and acoustic quality remain provisional. See [teaching providers](research/PROVIDERS.md), [speech providers](research/SPEECH_PROVIDERS.md), [speech teacher](research/SPEECH_TEACHER.md) and [platform delivery](research/PLATFORM_DELIVERY.md) for precise current limits.

## Keep both kinds of backup

Settings exports a versioned notebook JSON containing immutable facts, including completed attempts, optional human feedback and recording links. It excludes local drafts/preferences, keys and original audio bytes. The speech studio exports a separate `rufren-audio-v1` backup containing original recordings, per-take metadata and timed cues. Keep both when recordings matter; a hash/link without its bytes cannot replay audio. Relay recovery likewise needs its SQLite snapshot, manifest and matching audio directory; follow the [restore drill](deploy/README.md).

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
