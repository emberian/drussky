# Relay recovery acceptance

Verified 2026-11-06 against protocol 3 using a frozen relay binary and the prepared Dioxus preview. The accepted receipt is `.local/relay-recovery-qa/run-02/result.json`; it is a private, ignored local QA artifact, not a checked-in learner export.

## What changed

The existing `scripts/relay-backup.py` remains the operational workflow. This task fixed three boundaries:

- Recording/transcription facts can legitimately arrive before original-byte sync. The former registry-only scan could omit those recordings silently. Complete-audio backup now checks both fact references and registered takes; pending takes produce an actionable incomplete-audio error and no published recovery manifest. Existing facts remain intact.
- Manifest schema, event count, cursor, and relay identity must match the actual SQLite snapshot with exact JSON types. Booleans and floats cannot masquerade as integer counts or lengths.
- The online backup now anchors one read transaction, explicitly closes its connections, and finalizes a self-contained database before hashing. Python connection context managers alone do not close connections.

Existing two WAL/audio regressions and three new boundary tests pass. The new tests cover pending fact-only audio, wrong manifest identity/count/type claims, missing originals, and contradictory reference hashes. They deliberately exercise the backup tool's contract, rather than duplicating the Rust event validator.

## Snapshot and publication contract

The SQLite online backup reads facts, sequence cursor, relay UUID, and take registry from one consistent WAL snapshot while writes continue. After that snapshot is complete, the tool copies only its referenced content-addressed originals. Bytes are verified by SHA-256 and length before and after copying. Distinct take IDs and timed cues remain in SQLite; sharing a byte hash never merges takes or their decoding observations. Unreferenced blob files are excluded.

The server commits immutable bytes before accepting a take registry record. Facts can precede that record because audio sync is explicit and optional. Therefore, pending recording sync is an expected replica state: finish syncing originals and metadata from the originating device, then make a new complete backup. A failed attempt leaves its destination for inspection and cannot be restored without a valid manifest. It does not delete or rewrite source facts.

The versioned manifest is published after database and referenced originals have been copied and checked. A missing/truncated manifest, changed database, changed/missing original, or contradictory manifest claim is refused. Existing destinations are never overwritten. Keep the SQLite file, `.json` manifest, and `.sqlite.assets/` directory together. Hashes detect accidental corruption, not an adversary able to replace the complete bundle.

## Actual recovery result

`scripts/test-relay-recovery.mjs` creates synthetic data through the real Dioxus UI, reuses the clearly labeled v3 speech fixture shared with shell-update QA, and starts its own relay and TLS proxy on disposable loopback ports. No main data, configuration, credentials, keys, services, or browser profiles are used. Providers are disabled; one synthetic Teacher submission is intercepted locally so its durable question/context can be tested without inference.

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

The first harness attempt stopped because its diagnostic counter included an automatic read-only Teacher-status request as well as the intentionally intercepted synthetic submission. The counter was narrowed to POST requests; this was a QA orchestration error, not a data-recovery failure. Both runs' owned PID markers were removed and their relay/TLS listeners were confirmed stopped. Main services were never restarted by the test.

## Reproduce and operate

The harness requires an already built protocol-3 relay, a prepared Dioxus preview, OpenSSL, Caddy, and Playwright. It performs no build, download, container launch, model inference, or deployment:

```sh
python3 scripts/test-relay-backup.py
python3 scripts/test-relay-recovery-boundaries.py
node scripts/test-relay-recovery.mjs .local/relay-recovery-qa/new-run-name
```

Use a new run name. Set `RUFREN_PREVIEW_URL`, `RUFREN_PLAYWRIGHT_MODULE`, or `RUFREN_CHROMIUM_EXECUTABLE` when they differ from defaults. The harness retains its synthetic fixtures and receipts for inspection, and stops only the processes named in its own PID files. The reusable production backup/restore commands remain in [deploy/README.md](../deploy/README.md).

For a real restore, verify the bundle, restore to a fresh database/assets path, run the real Rust reader on that working copy, and validate clients before changing the service's data paths. Keep the original database and its sidecars together until acceptance; never overwrite a live database. An ahead cursor requires deliberate reconnect/reseed, with recordings enabled if newer originals must be recovered. A different relay UUID requires an explicit library decision rather than silent rebinding.

Service credentials and TLS keys are a separate recovery contract. The data backup intentionally excludes them and never exports their values. Retain them in appropriately protected storage or rotate/bootstrap replacements. Pinned models can be reinstalled from their manifests; local browser drafts and unsynced originals require their device's own export/recovery path.

## Next engineering priority

The largest remaining operational gap is automated, encrypted off-host backup with a visible last-verified recovery result. This local proof does not protect against loss of the Mac and its local backups. Before scheduling it, expose pending original-sync status so an operator can distinguish an incomplete audio replica from damaged data without silently uploading recordings. This task creates no schedule, retention deletion, cloud account, or secret-backup mechanism.
