# Drussky

Drussky (Dr. Russky) is Ember’s personal Russian learning app: start with no Russian, then build toward conversation and independent reading of stories, science fiction, mathematics and philosophy. The repository, crates, commands and existing artifact names remain `rufren`.

The AI teacher supplies explanations, examples, corrections and practice. Short, untimed activities keep one immediate action visible, with replay, help and a smaller step close at hand. The intended learning loop is to **hear, read, draw, say, recall and construct language**, returning to useful material across sessions. Native-like pronunciation is a goal to evaluate, not a score the app can currently certify. A friend can be a conversation partner without preparing lessons or grading homework.

## What you can use now

The installed iPad course contains seven beginner episodes and 73 short steps: sound/script mappings, small meaningful phrases, I/you and he/she/it/they, keyboard spelling, and hidden-model recall. Twenty prepared audio clips work offline (12 human recordings and eight Silero models). The course introduces prerequisites before asking a zero beginner to decode or produce a dialogue. Completing an activity records an encounter or response, not mastery.

The app also has five destinations: **Today, Learn, Talk, Read and Notebook**. Today resumes unfinished work. Learn holds guided practice and sound tools; Talk holds saved AI conversations; Read preserves original passages alongside optional translations and contextual help; Notebook holds teaching, progress and settings. Phones use bottom tabs, larger screens a sidebar.

- Guided teaching supports reading, writing, listening and speaking. Feedback keeps the exact question and saved answer beside the correction; “Try this question again” opens supported practice while retaining the original. Oral feedback discusses confirmed words and keeps the exact recording available.
- Saved conversations can stay anchored to an exact lesson, feedback item or reading passage. Reader place, help and unfinished notes resume locally; keeping a phrase is an explicit choice.
- Short sessions mix useful returns. Adjustable timing, exact-task dates and pauses stay distinct from recall or correctness; changing a preference never grades an answer.
- Cached speech models, recording, A/B replay and optional sound-region plots support listening and production. Recognition suggests words; it does not assess pronunciation. Sound Focus offers listening contrasts and fresh production.
- Local drafts and immutable teaching/attempt histories preserve work through ordinary interruption. Optional private sync and separate notebook/audio backups support recovery.

## Current work and limits

The seven-episode course passed complete offline browser journeys at phone and tablet sizes and is installed on the iPad. Later returns open a blank answer while keeping your exact place and draft in the course. Upgrading preserves the earlier three-episode history. Real Mac Save/Cancel and recovery of original audio bytes pass; physical iPad share-sheet behavior and authenticated connection restoration after the new launch remain unobserved. [TASKS.md](TASKS.md) records acceptance and delivery scope.

Connected handwriting practice is being implemented and is not yet installed. It will span syllables, words and messages: hear → write → compare → say, or see → copy → hear → say, with later returns under different cues. Native PencilKit is the leading iPad capture path; Rust retains the activity model and a portable ink adapter. Printed Cyrillic and cursive need distinct teaching paths. Explicit grammar construction—combining words and changing forms to express meaning—is part of the continuing curriculum.

The [product comparison](research/WRITING_PRODUCT_COMPARISON.md), [primary learning comparisons](research/HANDWRITING_ACTIVITIES.md), [recognition algorithms](research/HANDWRITING_ALGORITHMS.md) and [adaptive learning algorithms](research/MULTIMODAL_LEARNING_ALGORITHMS.md) explain the current choices and the evaluations still needed. Recognition accuracy, handwriting form, spelling, pronunciation and later recall are separate questions.

Claude’s actual Voice Mode lives in Claude’s app/web interface; there is no embedded Voice Mode API integration here. Drussky’s own recording → transcription → Claude text teaching → generated speech path is not yet a complete hands-free conversation loop.

The private service uses data/sync protocol 8, Teacher-v8 and dialogue-v6. Targeted model and browser checks support specific feedback repairs, but generated task criteria and explanation precision still have known defects. Delayed learning, unfamiliar-context transfer, pronunciation assessment, broad accessibility and device behavior require further evaluation. See the [completion contract](COMPLETENESS.md), not test counts, for the remaining destination.

## Implementation

Rust implements content, event history, scheduling, teaching contracts and audio instruments; Dioxus supplies browser and native UI. Native facts use SQLite, browser facts use IndexedDB, and original recordings are stored separately. Local drafts are not cross-device handoff. Optional private relay sync preserves immutable history and explicit conflicts.

The primary text teacher uses an explicitly configured Claude provider. The subscription-backed path calls the installed Claude CLI with its native login; a separate Anthropic API adapter has its own credentials and accounting. Local Russian speech generation uses an explicit Qwen/Silero router, and local Qwen ASR supplies transcription. These models have separate roles: text teaching, speech examples and recognition do not imply acoustic assessment. No silent paid-provider fallback is enabled.

Use a backed-up upgrade path for installed devices: retain notebook **and** original-audio backups, use the coordinated client/relay artifacts and follow [platform delivery](research/PLATFORM_DELIVERY.md). Mac/iPad packaging and installation are separate from browser acceptance. Keep credentials, personal recordings and learner records outside source control.

## Run the browser app

Use Dioxus CLI **0.7.10**, matching the pinned library. This Mac also has an unrelated Deno command named `dx`; `.tools/dx` is the ignored project-local Dioxus binary from [the upstream release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10). `RUFREN_DX` overrides it in the build wrappers.

For development:

```sh
cd crates/rufren-app
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

Defaults are `127.0.0.1:8787`, `private/relay.sqlite`, a sibling `private/relay.sqlite.assets` original-audio directory, a 512 MiB audio quota, and allowed browser origins `http://127.0.0.1:8080,http://localhost:8080`. Set `RUFREN_RELAY_BIND`, `RUFREN_RELAY_DB`, `RUFREN_RELAY_ASSETS`, `RUFREN_RELAY_ASSET_QUOTA_BYTES`, or comma-separated `RUFREN_ALLOWED_ORIGINS` as needed. A preview on port 8091 must be explicitly added to the allowlist. Connect in Settings with the relay URL and key. Browser credentials remain in memory; native Apple bootstrap can retain the private connection in Keychain. See the delivery notes for the accepted device configuration and remaining relaunch checks.

“Sync now” sends notebook facts, including complete archived teaching, Reader originals, kept source spans and shared review-timing choices. “Include original recordings and timed cues” separately enables binary audio transfer to that same bound relay. Lesson/reading places, unsent notes and drafts, preferences and credentials remain local metadata. The current app, accepted private service and selected native artifacts use data/sync protocol 8; lesson/feedback policy is version 8, dialogue policy is version 6, and physical SQLite schema remains 1. Older facts retain their original encodings. Different physical devices require a reachable HTTPS relay with exact allowed origins. This is a single personal library with a shared bearer key, plaintext server storage and no end-to-end encryption. Nothing is deployed automatically.

Teacher backends are explicitly configured. The interactive CLI path uses `RUFREN_TUTOR_PROVIDER=claude_cli`, the configured absolute Claude executable and its existing native login, with Opus/high and returned model/profile provenance. Max remains an explicit preparation option. The separate Anthropic HTTP API backend retains its server-only credential and API accounting contract. Requests, saved responses, exact corrections and drafts survive interruption; there is no silent provider/model fallback or automatic paid retry.

Speech and recognition have their own explicit configuration/provenance. The main private service now uses the explicit `local_russian` Qwen/Silero router and local Qwen ASR. Milena remains an explicitly configurable separate backend, not a silent failure fallback. Exact marked-stress routing/canonical WAV/provenance and no-inference ambiguity guidance pass; automatic stress choices and acoustic quality remain provisional. See [teaching providers](research/PROVIDERS.md), [speech providers](research/SPEECH_PROVIDERS.md), [speech teacher](research/SPEECH_TEACHER.md) and [platform delivery](research/PLATFORM_DELIVERY.md) for precise current limits.

## Keep both kinds of backup

Settings exports a versioned notebook JSON containing immutable facts, including full archived teaching and conversations, completed attempts, optional human feedback and recording links. It excludes local lesson cursors/drafts/preferences, keys and original audio bytes. The speech studio exports a separate `rufren-audio-v1` backup containing original recordings, per-take metadata and timed cues. Keep both when recordings matter; a hash/link without its bytes cannot replay audio. Relay recovery likewise needs its SQLite snapshot, manifest and matching audio directory; follow the [restore drill](deploy/README.md).

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

## Further reading

[Design](DESIGN.md) describes the learning experience; [Architecture](ARCHITECTURE.md) maps implementation boundaries. [Evidence](research/EVIDENCE.md), [learning models](research/OPTIMAL_LEARNING.md) and [review policy](research/PERSONAL_REVIEW_POLICY.md) distinguish research from product assumptions. [Teaching providers](research/PROVIDERS.md), [speech providers](research/SPEECH_PROVIDERS.md) and [deployment/recovery](deploy/README.md) cover operational contracts.
