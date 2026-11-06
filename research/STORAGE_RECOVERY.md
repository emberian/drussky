# Relay recovery acceptance

Current live protocol is 8, with immutable relay `6cbc84fa05b2068882559ef7c39cb05c1eee9ea3e01cc571156b88c37ad52d9f` and reference preview `6cba8f48f26be1e0a700`. Root completed the 7→8 service cutover and native alias promotion, recorded in `.local/ipad-delivery/pre-data8-review-timing-6cbc/receipt.json` and `.local/ipad-delivery/delivery-data8-review-timing-alias-activation.json`. The main database remained empty: zero events and cursor zero, with identical logical snapshot hashes before and after. Private configuration and historical v3–v7 artifact aliases were preserved. Populated protocol-8 recovery is the separate isolated 72-event proof below; no device installation or model call was performed by that proof or promotion.

Historical Reader acceptance on 2026-11-06 verified protocol 7 with an immutable relay and Dioxus preview `763446549975442cc9ca`. The accepted Reader recovery receipt is `.local/reader-relay-qa/long-final-01/result.json`. Root separately completed the main-service cutover and frozen native-artifact promotion, recorded in `.local/ipad-delivery/pre-data7-reader-676dee/receipt.json` and `delivery-data7-supported-reader-alias-activation.json`. The live database was empty; populated recovery is demonstrated by the isolated 166-event fixture. These are private, ignored QA receipts, not learner exports or a device-installation claim.

Accepted protocol-6 Sound recovery remains `.local/sound-focus-relay-qa/final-81bb/result.json` with preview `81bb821f96d6b620462e`; its earlier main-service cutover is `.local/ipad-delivery/pre-data6-sound-81bb/receipt.json`. That live database was also empty, independently of its populated 93-event recovery proof.

Historical protocol-4 notebook/TLS acceptance remains `.local/relay-recovery-qa/run-03-v4/result.json` with preview `a572b35bce5be4d9b942`; protocol-3 acceptance remains `.local/relay-recovery-qa/run-02/result.json`. Those tests left the then-current main protocol-3 service untouched.

## Review timing protocol 8 recovery

The now-active protocol-8 relay passed independent recovery with preview `6cba8f48f26be1e0a700` and immutable relay SHA-256 `6cbc84fa05b2068882559ef7c39cb05c1eee9ea3e01cc571156b88c37ad52d9f`. The producer receipt is `.local/review-timing-qa/web6cba8f48f26be1e0a700-01/result.json`; independent consumer evidence is `.local/review-timing-relay-qa/web6cba-final-01/result.json`. That isolated recovery proof preceded the separately recorded root service cutover and native alias promotion; it did not itself change the main service or install a device build.

A closed 44-event text-only seed was selected before the actual two-client UI journey. Its final 72 immutable events passed bounded HTTP upload, retry, old-protocol refusal, coherent SQLite backup, fresh restore, an empty browser client and the immutable relay's typed database reader. The browser restored both exact Teacher and passage timing states and both issued start snapshots. Concurrent preferences and task choices retain their causal identities; they do not become clock or UUID winners. The producer resumed the exact original unfinished Teacher draft through offline reload. That local draft is a producer continuity check, not a claim that browser-local draft metadata synchronizes to a new client. A late source remains excluded from the earlier pinned start and correctly appears among the later generic start's known inputs.

This fixture has zero audio references, original recordings or played assets. It makes no new audio-recovery claim; the populated Reader7 and Sound6 audio proofs above remain historical evidence. Backup schema 2 and physical database schema 1 are unchanged. Teacher6/dialogue4 policies are unchanged, and the previously recorded dialogue4 case-01 broad-evidence dismissal/unrelated-station-parse editorial limitation remains unresolved; these timing checks do not establish that all Teacher editorial cases pass.

The existing cutover helper has an explicit `review-timing-data8` profile for 7→8. Its pinned acceptance checks require the actual producer and consumer, exact immutable history and starts, late-history scope, restored task states and the matching native/shared-source build. Only this text-only timing profile permits zero originals; historical Reader7/Sound6 playback requirements remain intact. The accepted immutable Reader7 relay is the rollback candidate, usable only against an unchanged logical snapshot containing no protocol-8 writes. No rollback may overwrite changed history.

## What changed

The existing `scripts/relay-backup.py` remains the operational workflow. This task fixed three boundaries:

- Recording/transcription facts can legitimately arrive before original-byte sync. The former registry-only scan could omit those recordings silently. Complete-audio backup now checks both fact references and registered takes; pending takes produce an actionable incomplete-audio error and no published recovery manifest. Existing facts remain intact.
- Manifest schema, event count, cursor, and relay identity must match the actual SQLite snapshot with exact JSON types. Booleans and floats cannot masquerade as integer counts or lengths.
- The online backup now anchors one read transaction, explicitly closes its connections, and finalizes a self-contained database before hashing. Python connection context managers alone do not close connections.

Existing two WAL/audio regressions and three new boundary tests pass. The new tests cover pending fact-only audio, wrong manifest identity/count/type claims, missing originals, and contradictory reference hashes. They deliberately exercise the backup tool's contract, rather than duplicating the Rust event validator.

The same complete-audio boundary now covers dialogue attachments/transcriptions and the direct nested original pointers in Sound prepared trials, playback records, and speaking completions. An unfinished prepared trial still requires its exact model originals. The focused Sound regression checks all three shapes: missing registry records fail with an actionable pending-sync message, complete registered originals restore exactly, and a contradictory nested hash prevents publication of a usable manifest. The physical database remains schema 1; the backup manifest remains version 2.

Reader originals, chunk spans, support editions and exact excerpt links are immutable event data. Their original/scaffold audio uses canonical reference revisions and existing `RecordingAttached` facts, so the existing SQLite/audio backup traversal applies without a new binary format.

## Snapshot and publication contract

The SQLite online backup reads facts, sequence cursor, relay UUID, and take registry from one consistent WAL snapshot while writes continue. After that snapshot is complete, the tool copies only its referenced content-addressed originals. Bytes are verified by SHA-256 and length before and after copying. Distinct take IDs and timed cues remain in SQLite; sharing a byte hash never merges takes or their decoding observations. Unreferenced blob files are excluded.

The server commits immutable bytes before accepting a take registry record. Facts can precede that record because audio sync is explicit and optional. Therefore, pending recording sync is an expected replica state: finish syncing originals and metadata from the originating device, then make a new complete backup. A failed attempt leaves its destination for inspection and cannot be restored without a valid manifest. It does not delete or rewrite source facts.

The versioned manifest is published after database and referenced originals have been copied and checked. A missing/truncated manifest, changed database, changed/missing original, or contradictory manifest claim is refused. Existing destinations are never overwritten. Keep the SQLite file, `.json` manifest, and `.sqlite.assets/` directory together. Hashes detect accidental corruption, not an adversary able to replace the complete bundle.

## Actual recovery result

### Protocol 7: Reader originals, excerpts and exact source return

The original browser producer `.local/reader-qa/run-03/` remains unchanged. `scripts/extend-reader-fixture.cjs` imports its immutable graph through Settings, adds one near-limit custom original through the actual Reader UI, and exports a new fixture at `.local/reader-qa/long-source-02/`. The 63,785-byte synthetic original includes combining stress, ё, em dashes, nonbreaking spaces and paragraph separators. Its 22 contiguous UTF-8 chunks reconstruct every byte. Late-part navigation and a private note survive a genuine service-worker offline reload; normal navigation reopens Reader from Today. The first supplemental attempt incorrectly expected the Reader route itself to survive reload; its retained `long-source-01` failure is a harness assumption, not lost data.

`scripts/test-reader-relay.mjs` then consumes that extended fixture through an independently started loopback relay, coherent SQLite/audio snapshot, fresh restore and empty browser profile. It uses the immutable Teacher6/dialogue4 executable with SHA-256 `676deec90373a41053ff821ec20d63b094a027aeb558b87a36b63644db93ac89`; no provider is enabled or called by this consumer.

| Acceptance boundary | Protocol-7 observed result |
|---|---|
| Exact recovered graph | 166 immutable events in 11 bounded upload pages; HTTP pulls, restored app export and stopped SQLite preserve identical compact event JSON; Rust reader reports `Verified 166 immutable events and 1 recording metadata records` |
| Original and supported editions | Six documents, 57 rendered chunks: three AI-authored readings, one attributed authentic excerpt, and two learner-provided texts. Source/rights metadata and exact original wording survive; opening meaning/stress support does not replace the original |
| Substantial custom source | The 63,785-byte/22-chunk source restores exactly and remains navigable. The other five originals are short samples, 242–666 bytes; this result does not relabel those samples as long-form literature |
| Kept phrases and lineage | Two exact kept excerpts retain source document, global UTF-8 spans, canonical passage revisions and chunk links; Library returns to the exact original part, including through the retained edited-passage lineage |
| Saved contextual dialogue | Two user/reply paths restore independently. After deliberately setting a different local part, each saved conversation returns to its own original chunk, parts 3 and 4 of the same source; no latest-cursor guess |
| Local state remains local | All eight producer Reader selection/import/state keys are absent before opening a document. Each restored document starts at its own first chunk in Continuous mode, stress off, with no copied supports or private notes |
| Original audio | One explicitly synthetic test-tone WAV retains original bytes, hash, take metadata, Reader revision and source-text hash; actual restored playback produces a new correctly bound Audio presentation. This checks transfer/playback, not Russian speech quality |
| Retry and compatibility | Protocol 6 is refused before mutation/download; a duplicate accepted upload consumes no new cursor. Separate store/relay tests preserve literal versions 1–6 verbatim under physical schema 1 |
| No fabricated learning or inference | Source opening creates no graded attempt; no inference, forbidden requests or browser errors. All owned relay/browser/supervisor processes stopped and the listener closed; main protocol 6 remained untouched throughout this isolated run |

The sustained-reading transport boundary is separately exercised by `rufren-relay/tests/reader_bounds.rs`: 20 valid documents of about 519 KiB encoded size each, with 64,000-byte originals, duplicated chunk/stress text and heavily escaped bounded support. A count-only request exceeds the unchanged 2 MiB HTTP cap and receives 413 without saving a prefix or cursor. Existing byte-aware packing and pull pagination transfer the shelf exactly within the 1 MiB event-page budget and 1,100,000-byte client response cap; retry cursors remain unchanged. No server limit increase or transport rewrite was needed. The scoped store/relay batch passed all four compatibility/bounds tests.

Root accepted Reader functionality, with an explicit editorial limit: dialogue-v4 case 1 still contains a broad evidence dismissal and unrelated station parsing. Its exact response is preserved at `.local/reader-dialogue-v4-live/acceptance/case-01/response.json`; it has not been polished into a passing answer. Case 2's written-stress repair was accepted. These storage and navigation results do **not** claim that all Teacher editorial cases passed.

### Accepted protocol-6 to protocol-7 service cutover

The reusable guarded helper now takes `--transition-profile reader-data7`, requiring Teacher6/dialogue4 and the actual immutable relay build receipt. Historical invocations without this flag retain Sound's 5→6 defaults. Seventeen isolated guards pass, including rejection of the provisional dialogue3 candidate, mismatched binary/build/native shared sources, incomplete Reader consumer evidence, and any rollback6 attempt after changed logical rows or version-7 writes. No old snapshot is restored over changed history.

Root executed the accepted Reader transition and then separately promoted the frozen native-artifact aliases after verified private-CA HTTPS preflight. The relay-only receipt records an exact quiescent snapshot/post-cutover logical match: **0 events, 0 originals, cursor 0, no assigned relay UUID**, physical schema 1. Configuration and aliases remained unchanged during that relay-only step; the subsequent promotion preserves historical v3/v4/v5/v6 artifacts. All expected authenticated teaching/speech/recognition capabilities are enabled. No inference or device-installation action is claimed by these operations.

The accepted preparation receipt is `.local/ipad-delivery/accepted-data7-reader-relay.json`, SHA-256 `0bc02cbdb307b5b94d15da69ba9af7113c2e4e5c2b3fea7f97d2481841fee28a`. It pins the 166-event consumer, substantial-original producer, exact dialogue4 build receipt, and final native pairing:

- `.local/ipad-delivery/data7-supported-reader-build-source.json`: 225 unchanged complete inputs, including the Reader anthology/fixture JSON and embedded Sound research inputs; SHA-256 `153ceba6393ac0618454f7920980dc9eb25ee05625217d8a93f7cb3783677648`.
- `.local/ipad-delivery/frozen-artifact-data7-supported-reader.json`: SHA-256 `2a620909378481b8402d82f1a01a3b9ebfe14bd30e42722297a425f0e201480b`, paired with web `763446549975442cc9ca`. Eighty-two shared Cargo/core/store/relay/tutor/contract/speech/audio source hashes also match the **actual** `.local/reader-data7-dialogue4-candidate/receipt.json` build map, rather than merely a newly drafted claim.
- The 31 worker/dependency file hashes equal the previously root-reviewed protocol-6 snapshot. They remain **files-only evidence**, with no loaded-worker-code, installed-model/dependency, or inference-quality attestation. The accepted receipt also records the unresolved editorial limit explicitly.

### Protocol 6: Sound facts and original recordings

`scripts/test-sound-focus-relay.mjs` consumes the final real-app producer export in `.local/sound-focus-qa/final/`. The producer used a synthetic learner journey, packaged generated models, and the browser capture pipeline. The independent consumer copies the accepted executable into its own run directory, starts a provider-disabled loopback relay with a new database/token, uploads bounded pages, takes a coherent SQLite/audio backup, restores into a fresh path, and downloads through a separate empty app profile. It injects no local Sound workspace metadata to make history appear.

| Acceptance boundary | Protocol-6 observed result |
|---|---|
| Exact immutable graph | All 93 events round-trip through actual HTTP, backup/restore, app export and SQLite as identical compact event JSON; physical schema 1 |
| Discoverable Sound history | Progress opens both saved blocks and renders all 10 prepared trials with exact trial/preparation identities, modes, positions, original choice orders and saved choices |
| Completion versus setup | Five completed responses survive: four listening choices and one kept speaking take. Five other prepared trials remain visibly unfinished, including an abandoned speaking intent; setup is not converted into a response |
| Playback evidence | Saved completion/attempt identities and the original before-response model-start counts survive; opening history does not rewrite those immutable facts |
| Original bytes and metadata | Four linked take records and four original SHA-256 hashes survive registry upload, coherent restore and empty-client IndexedDB download; metadata, lengths and every byte match the producer |
| Actual restored playback | Both exact model assets and the kept fresh speaking take play in the empty app. The fourth, unkept but linked original is byte-verified; this receipt does not claim it was played |
| Rust semantic reader | `Verified 93 immutable events and 4 recording metadata records` on the stopped restored working copy |
| Old-client and retry boundary | Protocol 5 receives HTTP 409 before mutation/download; retrying an accepted upload changes neither sequence cursor nor fact count; relay identity survives restore |
| No inference or main mutation | Zero inference calls, forbidden requests or browser errors; isolated relay/browser/supervisor stopped and its listener closed. Main data/services were not used by this consumer |

The producer also records preserved unlinked imports in `local-orphans.json`. Those are explicitly outside this graph-linked original-audio recovery receipt; they were not silently adopted into a trial or counted as synced bytes. The result proves persistence and playback of these exact examples, not independent listening transfer, pronunciation quality, or a human acoustic assessment. Separate store/relay compatibility tests retain literal event versions 1–5 verbatim, accept a typed version-6 prepared trial, and refuse protocol-5 processing before identity/cursor/write changes.

This current consumer uses disposable loopback HTTP. The historical notebook proof below separately exercises a verified fixture-CA HTTPS boundary. The live cutover preflight checks the private HTTPS origin and certificate with redirects and ambient proxies disabled; neither proof is an iPad/Safari certificate-store or network test.

### Accepted protocol-5 to protocol-6 service cutover

Root subsequently executed `scripts/platform/cutover-sound-data6.py` using the accepted immutable relay SHA-256 `240c62d32c09119fa7582be82ef3eb3f96eb1ff290851e84036ae6715f283773`. Before activation it stopped only the ownership-checked relay, verified a quiescent SQLite-plus-originals snapshot, and compared all logical database rows before starting the new binary. The actual main snapshot contained **0 events and 0 originals**, with cursor 0 and no assigned relay UUID. Its exact logical digest matches the post-cutover database; this operation did not exercise populated-history migration.

The accepted cutover receipt reports protocol 6, physical schema 1, unchanged private configuration and artifact aliases during the relay-only step, and enabled authenticated `claude_cli` teaching, `local_russian` speech and `local_qwen3_asr` recognition capabilities. No inference endpoint was called. Artifact-alias promotion is a separate delivery action; the cutover receipt does not claim a device installation.

The accepted receipt `.local/ipad-delivery/accepted-data6-sound-relay.json` pins the exact relay, passed consumer, final web build and corrected complete-inputs native evidence:

- `.local/ipad-delivery/data6-sound-focus-complete-inputs-build-source.json` records 213 unchanged source inputs, including all nine external research WAV/JSON inputs embedded by Rust. Its SHA-256 is `0400dae173d6e78f91e16f3cb66419078f5dce88adedbbdbbabb67dbbe985e4e`.
- `.local/ipad-delivery/frozen-artifact-data6-sound-focus-complete-inputs.json` pins the frozen iOS artifact; SHA-256 `4d306a1255d626ebad7f7740746ca1e536eed016815d18f7f0f0a8487034bf61`. The earlier 201-input receipt is preserved but is not the accepted complete-source pairing.
- Seventy-nine shared Cargo/core/store/relay/tutor/contract/**speech/audio** source-file hashes agree with that frozen native evidence. `.local/sound-focus-data6-candidate/source-consumer-pairing-complete-inputs.json` preserves the corrected pairing without replacing the earlier receipt.
- Root separately reviewed 31 file hashes under `scripts/speech`, including worker code and dependency locks. Their snapshot is `.local/sound-focus-data6-candidate/worker-source.json`. This is **file evidence only**: it does not attest which Python code is already loaded by a running worker, installed model/dependency state, or new inference quality. The cutover receipt explicitly records `running_worker_code_attested: false`.

At protocol-6 acceptance, the cutover helper's 11 isolated guards covered receipt/source mismatches, strict HTTPS validation before attaching credentials, pending-original backup failure, and rollback refusal after any logical-row or registry change. Restarting the old protocol-5 binary is allowed only while the exact snapshot is unchanged and contains no version-6 event. Changed history is preserved for forward repair; the helper never restores an old snapshot over it. The accepted activation succeeded, so these guard tests do not constitute an actual production rollback rehearsal.

### Historical protocol-4 notebook and protocol-3 TLS proofs

`scripts/test-relay-recovery.mjs` creates synthetic data through the real Dioxus UI, reuses the clearly labeled v3 speech fixture shared with shell-update QA, and starts its own relay and TLS proxy on disposable loopback ports. No main relay data, configuration, credentials, keys, or existing browser profiles are used. The shared preview supplies read-only app assets; no main relay/TLS endpoint is contacted. Providers are disabled. In the protocol-4 run, two explicit Teacher UI requests receive clearly attributed synthetic lesson/feedback fixtures through local interception, without executing a model. The earlier protocol-3 run intercepted its one submission as an unavailable provider.

The protocol-4 run passed all **16** acceptance boundaries on its first attempt:

| Acceptance boundary | Protocol-4 observed result |
|---|---|
| Coherent online snapshot | 34 immutable events, versions 2/3/4 in physical database schema 1; 7 acknowledged writes overlapped the backup |
| Full Teacher notebook | Exact guided lesson and exact feedback documents, including echoed requests, all content/flow, provenance, usage, and live-receipt metadata; two guided steps retained |
| Context integrity | Exact prepared baseline context retained, with its known empty source-fact list; unrelated saved practice was not attributed to that lesson |
| Rust semantic check | `Verified 34 immutable events and 3 recording metadata records` on the restored working copy |
| Original audio/associations | Existing recording→ASR→confirmation→AI-feedback chain and all take metadata retained; 3 take IDs, 1 original hash, exact 64,044-byte WAV, distinct 16/48 kHz decoding observations and timed cue |
| Old-client boundary | A protocol-3 upload is refused with 409 before changing facts or sequence cursor, and receives no data page |
| Restore/reseed | Same relay UUID and rollback protection; deliberate reconnect reseeds the exact 72-event app union; authentication rotation remains separate |
| Empty-client notebook | A separate empty browser profile downloads exact documents and originals, opens the guided lesson from saved teaching at its own local step 0, advances, opens feedback, and returns to the exact original exercise |
| No regeneration | Opening restored teaching issues no new Teacher request; only the two explicit synthetic source requests occurred, with zero inference calls |
| Faults and integrity | The same incomplete/mismatched bundle, wrong-library, digest and original-byte checks pass; the original backup digest remains unchanged after Rust validation |
| Cleanup | Own relay/TLS listeners closed and PID markers removed; main relay/TLS/data/configuration/credentials untouched |

The guided fixture is accepted through the normal Teacher response path, not disguised as a passage or written directly into the archive. Its practice submission creates the actual immutable question/context; its exact feedback is accepted and archived. The restored empty client discovers those documents through the notebook UI. Local selection and lesson cursor are not synced or converted into completion/presentation facts. This run checks notebook persistence and navigation with synthetic teaching, not real-model pedagogy or guided oral generation.

The earlier protocol-3 result remains useful evidence for the pre-notebook path:

| Acceptance boundary | Observed result |
|---|---|
| Consistent snapshot during activity | 7 acknowledged protocol writes overlapped the backup run |
| Snapshot graph | 30 exact events: v2/v3, including a qualified Teacher question/context, 4 presentations, original recording links, an ASR hypothesis, learner confirmation, and attributed AI feedback |
| Original audio | 3 distinct take records, 1 shared SHA-256, exact 64,044-byte synthetic float WAV; separate 16/48 kHz decoding observations and timed cue retained |
| Rust semantic check | `Verified 30 immutable events and 3 recording metadata records` on the fresh restored copy |
| Snapshot immutability | Original backup SHA-256 still matches its manifest after the Rust check |
| Restore identity | The SQLite relay UUID survives; a separately rotated authentication key works, while the prior key is refused |
| Rollback protection | An ahead-of-snapshot app client receives a conflict, retaining its cursor and all local facts |
| Explicit recovery | Disconnect/reconnect with original-audio sync reseeds the restored relay to the exact 68-event client union |
| Fresh-client acceptance | A separate empty browser profile downloads all 68 events and all 3 take records; every original matches the source SHA/bytes and decodes as a one-second mono WAV |
| Wrong-library guard | A different relay identity refuses a pending local upload; client facts/binding remain intact and the wrong relay stays empty |
| Faults | Missing/truncated manifest, changed relay claim, changed database, and altered original are refused before creating a restore destination |
| Model activity | Zero inference calls |

The app's requests in this harness pass through a Node HTTPS transport that verifies the fixture CA and certificate hostname. Certificate verification is enabled; there is no ignore-certificate flag or global trust-store change. This proves the actual app/API recovery path over that verified TLS boundary. It does **not** test Safari's certificate store, iPad device networking, or native Keychain restoration.

The first protocol-3 harness attempt stopped because its diagnostic counter included an automatic read-only Teacher-status request as well as the intentionally intercepted synthetic submission. The counter was narrowed to POST requests; this was a QA orchestration error, not a data-recovery failure. Both protocol-3 runs and the protocol-4 run have no remaining owned PID markers or relay/TLS listeners. Main services were never restarted by these tests.

## Reproduce and operate

The current shared consumer derives its default from core `SYNC_VERSION` (7 at Reader acceptance) and takes explicit frozen producer artifacts. With the matching already-built relay and app preview, use a new output directory:

```sh
RUFREN_RELAY_QA_BINARY=/absolute/path/to/accepted/immutable/rufren-relay \
  node scripts/test-reader-relay.mjs \
  .local/reader-qa/long-source-02/library.json .local/reader-relay-qa/new-run-name
```

It performs no build or inference and stops its own disposable services. `RUFREN_QA_URL`, `RUFREN_PLAYWRIGHT_MODULE` and `RUFREN_CHROMIUM_EXECUTABLE` select the prepared preview/browser when needed. Historical shared-consumer replay requires explicit `RUFREN_RELAY_QA_PROTOCOL` with that frozen protocol's relay and app; do not relabel old fixture events. The Sound and dialogue wrappers reuse the same engine.

The historical `scripts/test-relay-recovery.mjs` harness defaults to an already built protocol-4 relay and matching prepared Dioxus preview, plus OpenSSL, Caddy, and Playwright. Set `RUFREN_RECOVERY_PROTOCOL=3` only with the matching historical relay/app artifacts. It performs no build, download, container launch, model inference, or deployment:

```sh
python3 scripts/test-relay-backup.py
python3 scripts/test-relay-recovery-boundaries.py
RUFREN_RECOVERY_PROTOCOL=4 node scripts/test-relay-recovery.mjs .local/relay-recovery-qa/new-run-name
```

Use a new run name. Set `RUFREN_PREVIEW_URL`, `RUFREN_PLAYWRIGHT_MODULE`, or `RUFREN_CHROMIUM_EXECUTABLE` when they differ from defaults. The harness retains its synthetic fixtures and receipts for inspection, and stops only the processes named in its own PID files. The reusable production backup/restore commands remain in [deploy/README.md](../deploy/README.md).

For a real restore, verify the bundle, restore to a fresh database/assets path, run the real Rust reader on that working copy, and validate clients before changing the service's data paths. Keep the original database and its sidecars together until acceptance; never overwrite a live database. An ahead cursor requires deliberate reconnect/reseed, with recordings enabled if newer originals must be recovered. A different relay UUID requires an explicit library decision rather than silent rebinding.

Service credentials and TLS keys are a separate recovery contract. The data backup intentionally excludes them and never exports their values. Retain them in appropriately protected storage or rotate/bootstrap replacements. Pinned models can be reinstalled from their manifests; local browser drafts and unsynced originals require their device's own export/recovery path.

## Next engineering priority

The largest remaining operational gap is automated, encrypted off-host backup with a visible last-verified recovery result. This local proof does not protect against loss of the Mac and its local backups. Before scheduling it, expose pending original-sync status so an operator can distinguish an incomplete audio replica from damaged data without silently uploading recordings. This task creates no schedule, retention deletion, cloud account, or secret-backup mechanism.
