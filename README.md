# Rufren

A personal Russian learning app for Ember, from zero Russian to conversation and independent reading of science fiction, literature, mathematics, and philosophy. Speaking, listening, reading, and writing are all first-class goals. Native-like pronunciation is an explicit ambition.

Target devices: iPhone, iPad, and Mac first, with Android, Linux, Windows, and web in the destination. Rust + Dioxus is the selected direction. A human coach will contribute examples, recordings, corrections, and judgments. Working-memory demands should be adjustable, and personalization should develop from actual use.

## Current state

Runnable foundation, 2026-11-05: Dioxus screens, persistent passage editing, reading/writing practice with resumable drafts and explicit hints, practice history, JSON backup/restore, and an optional authenticated sync relay. Concurrent passage revisions are preserved for explicit resolution. The speech studio is a preview; recording, pronunciation analysis, a reviewed beginner curriculum, and adaptive teaching are still to come.

- [Design](DESIGN.md): learning experience, speech analysis, architecture, and evaluation.
- [Architecture and flow maps](ARCHITECTURE.md): crate boundaries, stored facts, sync protocol, screen and practice flows, and extension points.
- [Evidence](research/EVIDENCE.md): findings, limits, and decisions they inform.
- [Efficient learning models](research/OPTIMAL_LEARNING.md): optimal control, pedagogical planning, and experimental grammar/vocabulary teaching.
- [Rust platform assessment](research/RUST_PLATFORM.md): Dioxus, Slint, shared core, and device feasibility criteria.
- [Work tracker](TASKS.md): implementation stages and unresolved research questions.

Source copies and extracted text are in `~/paperbin/rufren/`; acquisition manifests record their origins and download failures. Literature searches used `~/swarm/research.py` and primary publisher/author sources. This is a targeted initial review, not a systematic review or evidence of comparative app superiority.

Keep personal recordings, credentials, and learner records outside source control. Existing abcdeez work remains untouched; its useful design ideas are assessed in the evidence notes.

## Run

Use Dioxus CLI **0.7.10**, matching the pinned library. This machine also has an unrelated Deno command named `dx`; the project-local `.tools/dx` is the Dioxus binary downloaded from [the upstream release](https://github.com/DioxusLabs/dioxus/releases/tag/v0.7.10). It is ignored by Git. On another machine install the matching CLI using upstream installation instructions.

```sh
cd /Users/ember/dev/rufren/crates/rufren-app
../../.tools/dx serve --web --port 8080 --addr 127.0.0.1 --open false --interactive false
```

Open <http://127.0.0.1:8080>. Browser records belong to that exact origin: `localhost` and `127.0.0.1` have separate libraries. Export before clearing site data. Browser persistence currently uses localStorage for small text records; offline page loading and larger audio assets need the forthcoming IndexedDB/service-worker adapters.

For the native Mac app, from the workspace root:

```sh
cargo run -p rufren-app --no-default-features --features desktop
```

Native data goes to the platform's application data directory, or `RUFREN_DATA_DIR` when set. Web and native builds share the same core facts and sync protocol. Other device targets need their SDKs and device validation.

## Optional sync

Start the relay from the workspace root after setting `RUFREN_SYNC_TOKEN` to a private random value of at least 32 characters:

```sh
cargo run -p rufren-relay
```

Defaults: `127.0.0.1:8787`, database `private/relay.sqlite`, allowed web origins `http://127.0.0.1:8080,http://localhost:8080`. Overrides: `RUFREN_RELAY_BIND`, `RUFREN_RELAY_DB`, `RUFREN_ALLOWED_ORIGINS` (comma-separated). Connect in Settings using the relay URL and the same key. The app retains that key only in memory. Sync is manual in this foundation.

For different physical devices, use a reachable HTTPS host and configure exact allowed web origins. The relay is for one personal library; it is not deployed, end-to-end encrypted, or a multi-user service. Passages and attempts sync; drafts, raw audio, and provider credentials do not. A changed relay identity requires an explicit disconnect/reconnect.

## Focused checks

```sh
cargo fmt --all -- --check
cargo test -p rufren-core --test replication replication_
cargo test -p rufren-store --test durability durability_
cargo test -p rufren-relay --test network network_
cargo check -p rufren-app --target wasm32-unknown-unknown
cargo check -p rufren-app --no-default-features --features desktop
```

The workspace caps Cargo at two build jobs. `.local/logs/` contains local build/QA logs and is not committed.
