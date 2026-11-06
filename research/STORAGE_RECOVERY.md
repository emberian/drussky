# Relay recovery acceptance

Verified 2026-11-06 against current protocol 6 with an immutable relay and Dioxus preview `81bb821f96d6b620462e`. The accepted Sound recovery receipt is `.local/sound-focus-relay-qa/final-81bb/result.json`. A separate, subsequently accepted main-service cutover is recorded in `.local/ipad-delivery/pre-data6-sound-81bb/receipt.json`; its database was empty. Populated recovery is demonstrated by the isolated 93-event fixture, not by that live cutover. These are private, ignored QA receipts, not learner exports or a device-installation claim.

Historical protocol-4 notebook/TLS acceptance remains `.local/relay-recovery-qa/run-03-v4/result.json` with preview `a572b35bce5be4d9b942`; protocol-3 acceptance remains `.local/relay-recovery-qa/run-02/result.json`. Those tests left the then-current main protocol-3 service untouched.

## What changed

The existing `scripts/relay-backup.py` remains the operational workflow. This task fixed three boundaries:

- Recording/transcription facts can legitimately arrive before original-byte sync. The former registry-only scan could omit those recordings silently. Complete-audio backup now checks both fact references and registered takes; pending takes produce an actionable incomplete-audio error and no published recovery manifest. Existing facts remain intact.
- Manifest schema, event count, cursor, and relay identity must match the actual SQLite snapshot with exact JSON types. Booleans and floats cannot masquerade as integer counts or lengths.
- The online backup now anchors one read transaction, explicitly closes its connections, and finalizes a self-contained database before hashing. Python connection context managers alone do not close connections.

Existing two WAL/audio regressions and three new boundary tests pass. The new tests cover pending fact-only audio, wrong manifest identity/count/type claims, missing originals, and contradictory reference hashes. They deliberately exercise the backup tool's contract, rather than duplicating the Rust event validator.

The same complete-audio boundary now covers dialogue attachments/transcriptions and the direct nested original pointers in Sound prepared trials, playback records, and speaking completions. An unfinished prepared trial still requires its exact model originals. The focused Sound regression checks all three shapes: missing registry records fail with an actionable pending-sync message, complete registered originals restore exactly, and a contradictory nested hash prevents publication of a usable manifest. The physical database remains schema 1; the backup manifest remains version 2.

## Snapshot and publication contract

The SQLite online backup reads facts, sequence cursor, relay UUID, and take registry from one consistent WAL snapshot while writes continue. After that snapshot is complete, the tool copies only its referenced content-addressed originals. Bytes are verified by SHA-256 and length before and after copying. Distinct take IDs and timed cues remain in SQLite; sharing a byte hash never merges takes or their decoding observations. Unreferenced blob files are excluded.

The server commits immutable bytes before accepting a take registry record. Facts can precede that record because audio sync is explicit and optional. Therefore, pending recording sync is an expected replica state: finish syncing originals and metadata from the originating device, then make a new complete backup. A failed attempt leaves its destination for inspection and cannot be restored without a valid manifest. It does not delete or rewrite source facts.

The versioned manifest is published after database and referenced originals have been copied and checked. A missing/truncated manifest, changed database, changed/missing original, or contradictory manifest claim is refused. Existing destinations are never overwritten. Keep the SQLite file, `.json` manifest, and `.sqlite.assets/` directory together. Hashes detect accidental corruption, not an adversary able to replace the complete bundle.

## Actual recovery result

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

The cutover helper's 11 isolated guard tests cover receipt/source mismatches, strict HTTPS validation before attaching credentials, pending-original backup failure, and rollback refusal after any logical-row or registry change. Restarting the old protocol-5 binary is allowed only while the exact snapshot is unchanged and contains no version-6 event. Changed history is preserved for forward repair; the helper never restores an old snapshot over it. The accepted activation succeeded, so these guard tests do not constitute an actual production rollback rehearsal.

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

The current shared consumer defaults to protocol 6 and takes explicit frozen producer artifacts. With the matching already-built relay and app preview, use a new output directory:

```sh
RUFREN_RELAY_QA_BINARY=/absolute/path/to/accepted/immutable/rufren-relay \
  node scripts/test-sound-focus-relay.mjs \
  .local/sound-focus-qa/final/library.json .local/sound-focus-relay-qa/new-run-name
```

It performs no build or inference and stops its own disposable services. `RUFREN_QA_URL`, `RUFREN_PLAYWRIGHT_MODULE` and `RUFREN_CHROMIUM_EXECUTABLE` select the prepared preview/browser when needed. Historical shared-consumer replay requires explicit `RUFREN_RELAY_QA_PROTOCOL=4` or `5` with that frozen protocol's relay and app; do not relabel old fixture events.

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
