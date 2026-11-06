# Rufren work tracker

## Completed

- [x] Establish personal scope: zero Russian, all four skills, native-like pronunciation ambition, science fiction/literature/mathematics/philosophy reading, human coaching, Apple-first cross-platform destination.
- [x] Inspect research tools and paperbin; retrieve an initial speech/learning evidence collection.
- [x] Audit abcdeez concepts and relevant conversations without altering its working tree.
- [x] Write evidence-linked design and distinguish supported mechanisms from app hypotheses.
- [x] Verify official Apple feasibility documentation and current eligible Claude/OpenAI API-credit benefits.
- [x] Research Rust routes using upstream documentation; select shared Rust core and Dioxus as the first UI candidate to validate.
- [x] Extend Kagi research to optimal control of memory, POMDP teaching, Bayesian pedagogy, interleaved grammar practice, and task-local adaptive challenge.
- [x] Adopt Ember's Dioxus selection and establish the four-crate Rust workspace.
- [x] Implement versioned passage/attempt facts, deterministic merge, preserved concurrent revisions, and explicit resolution.
- [x] Implement native SQLite and browser text persistence, resumable local practice, and JSON backup/restore.
- [x] Implement a private authenticated relay and bounded, retry-safe manual sync for passages and attempts.
- [x] Build Today, Library, Practice, Speech studio preview, Progress, and Settings; document screen and sync flows in ARCHITECTURE.md.
- [x] Pass 8 focused integration tests covering merge, durability, pagination, rollback, disconnect, authorization, and HTTP convergence with concurrent edits.
- [x] Compile the Dioxus browser and Mac targets. Exercise reading/writing, Cyrillic input, draft/hint recovery after reload, backup replay, sync, conflict resolution across two browser origins, and disconnect/reopen in the browser.

## Next implementation work

- [ ] Connect native/browser capture and playback, including microphone permission, interruptions, and app lifecycle. Validate on Mac and a physical iPhone/iPad before promising a mobile audio experience.
- [ ] Introduce a content-addressed audio repository and IndexedDB browser storage; add offline browser startup.
- [ ] Build a small coach-reviewed zero-Russian sequence joining Cyrillic, sound, and meaningful exchanges. The current passage loop is infrastructure for this, not that curriculum.
- [ ] Add coach judgments and annotations as facts with provenance; model delayed probes and spaced review separately from self-ratings.
- [ ] Validate keyboard navigation, VoiceOver, contrast, and phone/tablet layouts on actual devices.
- [ ] Add a private HTTPS relay deployment/backup recipe and background sync after the manual path has real use.

## First usable implementation

Owner: Codex with Ember and coach feedback. These are ordered implementation stages, not reductions of the destination.

1. **Rust feasibility and lesson loop.** Build a shared Rust core and a Dioxus text/audio slice using the [device criteria](research/RUST_PLATFORM.md). Check real Russian text input, recording, playback, persistence, and spectral display on Mac/Apple mobile/browser, with an early Android build. Retain SwiftUI plus Rust as the fallback if the device experience fails. Extend the successful slice with coach phrase entry, annotations, export/import, and a lesson traversing all four skills.
2. **Memory and teaching policies.** Persist attempts with modality and cue use. Implement a simple spaced baseline and an experimental short-horizon teaching policy over a small reviewed grammar domain. Add coach correction, delayed probes, and new-context practice. Confirm resuming a session preserves state and that hints never count as unassisted recall. Log model assumptions and predictions; simulation success is not human-learning evidence.
3. **Speech instruments.** Add a spectrogram and reliable elementary measurements with quality flags. Check synthetic signals and known recordings, then coach-labeled Russian examples. Do not ship a global accent score inferred from transcript or spectral distance.
4. **AI tutoring.** Benchmark Russian language and speech providers on a small approved set. Implement one useful provider path with bounded cost, visible upload behavior, recoverable errors, and content review. Verify actual credits/key availability without printing secrets or changing billing settings.
5. **Cross-device continuity.** Implement and test sync of lesson versions, attempts, annotations, and audio, including offline conflicts and interruption recovery. Export remains available.

## Growing into the destination

- Extend curriculum from daily conversation to cases, aspect, motion, longer listening, and writing.
- Add science fiction/literature and mathematics/philosophy strands early, using wanted passages to inform vocabulary and construction priorities; increase complexity and independent comprehension.
- Test representative phrase anchors, deliberately informative contrasts, interleaving, and task-local difficulty adjustment as described in [the theory review](research/OPTIMAL_LEARNING.md).
- Evaluate the incremental benefit of visual feedback, coaching prompts, and personalization using delayed/novel items and manageable effort.
- Refine acoustic models against held-out recordings and proficient listener judgments; separate pronunciation, fluency, and intelligibility.

## Questions to resolve through use

- Which working-memory supports help Ember, when, and at what cue level? Do not front-load a diagnostic battery.
- What Russian variety/reference voices suit the pronunciation goal, and what can the coach confidently judge?
- Which specific science fiction, literature, mathematics, and philosophy texts are motivating early destinations?
- What are the devices/OS versions, signing/sync capabilities, and actual provider entitlements?

## Current limits

The app is a runnable foundation with text practice and optional manual sync. The starter text is not a complete beginner curriculum. Speech capture/playback, acoustic feedback, coach judgments, spaced scheduling, AI providers, audio/annotation sync, remote deployment, and physical-device validation remain open. The least-practiced selection policy makes no optimal-learning claim. There is no learner recording corpus or measured learning result yet. Initial literature acquisition includes failures in `~/paperbin/rufren/acquisition.json`; the Russian ultrasound full text was subsequently retrieved via HTML and recorded in `acquisition-additional.json`.
