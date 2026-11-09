# Apple app review — 2026-11-08

Reviewer: Codex. Baseline: `cd09f43`. Scope: native `apple/`, especially persistence, learning evidence, tutor integration and asynchronous lifecycle. This began as a source review with focused executable probes, not a physical-device UX or linguistic audit. Ember subsequently authorized implementation. All ten findings are addressed; the original findings below are retained with the resolution record.

P1 = address before relying on the affected learning/data behavior; P2 = important robustness or maintenance defect. Reproduction instructions below are proposed regression tests unless explicitly described as executed.

## Resolution — implemented by Codex

| Finding | Resolution | Regression evidence |
|---|---|---|
| A01 | Flush returns failures; export refuses incomplete saves; close throws and leaves the same book/pending data usable. Restore swaps only after a successful close. | Failed state save blocks export and AppState restore; failed ink close retains the drawing and succeeds after repair. |
| A02 | Goal-stress tasks belong to Book, merge checks its lifecycle, and successful close cancels them. Registered notebook activities drain before the final flush. Talk's private microphone participates; cancelled speech jobs remain tracked until hard invalidation drains them. | Closed sidecar/metric calls cannot write into a replaced root; registered late writer finishes before the final flush. |
| A03 | Offline diary correction routing is disabled; `canGrade` also excludes on-device corrections at the assessment boundary. | Mechanically consistent father→mother correction from the device cannot grade or add review words. |
| A04 | Talk grants correctness only from consistent final corrections, excluding changed spans; typed turns and their confusions use writing. Legacy tutor evidence is re-evaluated from saved results without rewriting the raw log. | Typed/spoken live and backfill traces agree; unmarked clarification receives no correctness credit; old typed speech credit becomes writing across warm/cold loads. |
| A05 | LearningCore refolds late evidence in event-time order, including help/transfer state. | Late help matches full replay; delayed outcome agrees live, from snapshot+tail, and after full replay. |
| A06 | Snapshot format 2 carries SHA-256 of the exact log prefix, checked before tail replay. Import discards derived snapshots. | Same-length grade replacement invalidates cache; import removes stale cache. |
| A07 | Evidence and metrics share a checked append path: encode the whole batch before I/O, propagate errors, roll back partial writes. | Non-finite event/metric values throw; rejected event batch leaves bytes unchanged. |
| A08 | Restore decodes event and metric types and rejects unsupported versions with filename/line diagnostics. | `{}` and future-version records are rejected for both logs. |
| A09 | ZIP extraction uses system zlib with a fixed 64 KiB output buffer, CRC and actual-size checks, preflight expansion limits, and path/type validation. Validation runs off the main actor; directory imports reject links and share expansion limits. | Independent Python-generated stored/deflated/empty entries round-trip; changed payload checksum, truncation, oversized declaration and folder links are rejected. |
| A10 | Metrics expose pending failures, retry during flush, and refuse writes after close. | Last failed measurement reaches disk on flush without another measurement; stale tracker cannot write into replaced root. |

Validation: `apple/scripts/test.sh fast` passed: 36 LearningCore tests and 301 Drussky tests,
8 skipped, zero failures (130 s wall). This adds 15 regressions to the reviewed baseline.
The final focused notebook/knowledge/Talk check also passed: 58 tests reported, 2 skipped, zero failures.
Logs: `apple/.build/logs/test-fast.log`, `apple/.build/logs/review-final-targeted.log`.
No physical-device installation or microphone/Pencil evaluation was performed.

Intentional limits: imports allow 256 MiB archive input, 128 MiB per expanded file and 1 GiB total.
Legacy rounds whose transcripts no longer survive retain their raw responses as ungraded history;
previously kept vocabulary is preserved. Late events refold the model for correctness; this is the
exceptional path, while ordered events remain incremental.

## Findings

### A01 · P1 · Export and restore continue after saves fail

`apple/Drussky/Model/Book.swift:723–746`, `apple/Drussky/Model/InkWriter.swift:69–72`, `apple/Drussky/App/AppState.swift:103–112`.

`flush()` returns no success/failure result. A failed notebook save merely sets `persistenceError`; failed ink stays pending. `exportArchive()` nevertheless zips the on-disk folder. `close()` nevertheless closes the writers, and restore replaces the folder. Thus a storage failure can produce an apparently successful export missing the latest work, or leave pending ink only in a closed, discarded object after restore. Keeping the old folder protects persisted bytes, not those pending writes.

Fix: make flush/close return a structured result; require successful draining before export/install, preserving the live book on failure. Test with an unwritable state destination and an unwritable ink destination separately.

### A02 · P1 · A late goal-stress request writes into the restored notebook

`apple/Drussky/Goal/GoalStress.swift:130–150`.

`ensureGoalStress` starts an untracked task retaining the old `Book`. After its network await, `mergeGoalStress` writes directly to `store.root` without checking `isClosed`. Restore reuses that root path. If the response arrives after restore, the old task writes its stress sidecar into the new notebook. The close protection on `Book.record` and `KnowledgeTracker` does not cover this writer.

Fix: close-aware task ownership and a single notebook-scoped persistence boundary. Regression: suspend a stress response, close/swap the root, release the response, assert the restored folder is unchanged.

### A03 · P1 · Known unreliable offline corrections become authoritative learning evidence

`apple/Drussky/OnDevice/TutorBackend.swift:62–64`, `apple/Drussky/Diary/DiaryBook.swift:36–50`, `apple/Drussky/Diary/DiaryKnowledge.swift:29–55`.

The router permits `.diaryCorrection` on device. The project's own `research/ONDEVICE_TUTOR.md` benchmark reports six false edits in ten diary lines, including changing father to mother. The downstream safeguard only checks whether edit spans reproduce the corrected string. A coherent but linguistically false edit passes, adds replacement words to review, and records `again` evidence/confusions. Model attribution does not prevent this. The benchmark document still describes the fallback set as gloss-only.

Fix: keep this backend's corrections advisory/ungraded until its quality justifies grading, or disable this fallback. Test a mechanically consistent false edit from the on-device backend and ensure it cannot alter memories or introduce review words.

### A04 · P1 · Typed Talk replies are credited as successful speech

`apple/Drussky/Talk/TalkSession.swift:189–193,266–278`, `apple/Drussky/Talk/TalkKnowledge.swift:22–27,49–63`.

`submitTyped` correctly records `typed: true`, but both live `talkUnderstood` and completion/backfill discard that distinction. Every reply receives `.good` in `own-speech`; corrections also become speech-channel confusions. A learner can type the entire conversation without speaking and build a speaking history. Additionally, any tutor reply is treated as proof it understood every word, including a reply asking for clarification; the reply contract supplies no understanding judgment here.

Fix: carry input modality into evidence; distinguish successful communication from token correctness. Typed turns should contribute writing evidence. A clarification request should not award blanket successful production. Test live and backfill paths against identical typed/spoken conversations.

### A05 · P1 · Delayed events make learning state depend on whether a snapshot was used

`apple/LearningCore/Model.swift:162–179`, `apple/Drussky/Knowledge/KnowledgeTracker.swift:59–60,141–146`, `apple/Drussky/Diary/DiaryTutor.swift:81`, `apple/Drussky/Diary/DiaryKnowledge.swift:23`.

Full reconstruction sorts by event time; incremental `apply` explicitly requires time order but only marks a flag when an older event arrives. It still updates memory in arrival order. The diary correction timestamp is captured before awaiting the backend, so another exercise can finish while the request is in flight, then receive an older correction event afterwards. Snapshots preserve the arrival-order state; rebuilding without one sorts the same evidence and yields different traces/schedules.

Executed probe against the actual LearningCore sources: apply events at t=100,200,150, then reconstruct from those same events. `cached.traces != folded.traces` is true.

Fix: define event-time semantics centrally and refold affected traces (including help/transfer state) on late arrival, or assign commit timestamps consistently. Test delayed diary completion across a newer outcome, then compare live, snapshot and full-replay results.

### A06 · P2 · Snapshot validation never checks that the snapshot belongs to the log prefix

`apple/Drussky/Knowledge/KnowledgeTracker.swift:51–60`, `apple/LearningCore/Evidence.swift:143–165`.

The last-event-ID check compares two values inside the snapshot. The log check only requires enough bytes and a newline at the saved offset. Replacing/correcting a prefix with the same byte length leaves the old model accepted. Restore imports the cache too. A mismatched cache/log archive can therefore silently present different learning history than its event log.

Executed probe: write a `good` event and record the byte offset; replace it with the same-length `hard` event. `loadTail` accepts the offset and returns an empty tail while the cached grade remains good and disk grade is hard.

Fix: bind snapshots to log identity and content, with a documented invalidation strategy for restore/repair. Discard derived caches during import unless validated against their source.

### A07 · P2 · Event append can report success while discarding an event

`apple/LearningCore/Evidence.swift:173–185`, `apple/Drussky/Knowledge/KnowledgeTracker.swift:149–164`.

Encoding failure uses `continue`; append returns successful after writing the rest. The tracker then clears its pending batch and may snapshot the in-memory event that never reached the canonical log. This is a concrete contract violation, although no normal UI path producing non-finite event values was established in this review.

Executed probe: an event with `latency = Double.nan` makes append return true while the persisted event count remains unchanged.

Fix: encode the entire batch successfully before touching the file; return a typed encoding failure. Add a non-finite-value regression and ensure a rejected batch remains pending or is visibly rejected.

### A08 · P2 · Restore validates JSON syntax rather than the event schema

`apple/Drussky/Model/NotebookArchive.swift:84–101`, `apple/LearningCore/Evidence.swift:126–130`.

Any syntactically valid JSON object passes the event-line validation. `EventLog.load` then silently drops lines that do not decode as `LearningEvent`. For example, an archive with `{}` as an event line is accepted with zero events from that line. Future event variants or damaged required fields therefore become silent history loss during a supposedly validated restore. Page backfill cannot reconstruct every attempt/help/confusion.

Fix: decode supported event/metric types during validation, report rejected lines and unsupported schema versions, and make partial recovery an explicit outcome rather than a clean restore.

### A09 · P2 · ZIP import allocates from untrusted sizes without bounds or CRC checking

`apple/Drussky/Model/NotebookArchive.swift:37–45,175–201`.

Import loads the whole archive, copies it to `[UInt8]`, and allocates `Data(count: e.size)` directly from each directory entry. There is no per-entry or aggregate expansion limit; a tiny archive can request gigabytes. Extraction runs synchronously from the main-actor restore method. Neither stored nor deflated entries have CRC validation, so corruption in otherwise parseable content can pass. The review did not execute a memory-exhaustion payload.

Fix: bounded streaming extraction off the main actor with entry/total limits and checksum validation, preferably through a maintained archive implementation. Test malformed metadata and bounded rejection without allocating the advertised payload.

### A10 · P2 · Fluency persistence is outside the notebook flush/close contract

`apple/Drussky/Fluency/FluencyTracker.swift:14–28`, `apple/Drussky/Model/Book.swift:723–746`.

Fluency retains failed appends in `unwritten`, but only retries on another fluency measurement. There is no flush method, no surfaced failure, and no close guard. Backgrounding/exporting does not retry this queue, unlike knowledge events and ink. A transient failure on the last measurement loses it when the process ends. The ownership gap also permits stale callers to append to a reused root.

Fix: include metrics in the same flush/close lifecycle as all other notebook writers. Test failure followed by flush without another measurement.

## Engineering assessment

The important recurring problem is that contracts are conventions spread across feature folders. `Book.close` sounds global but does not close every writer; `verified` means text-edit consistency rather than pedagogical correctness; and event-time order is a precondition violated by ordinary asynchronous producers. More comments or feature-specific guards will only partially address these. Centralizing notebook writer ownership, making flush failure explicit, and carrying input/assessment provenance through typed evidence would simplify the system while fixing several entries together.

There is useful existing coverage: notebook recovery, deterministic backfill, stale speech results and snapshot tail replay are tested. The missing tests are chiefly compositions: failed flush followed by restore, pending feature work followed by restore, typed input followed by speech grading, and late events followed by snapshot/full replay. Those should take priority over another batch of isolated happy-path tests.

## Verification and limits

Focused probes compiled all `apple/LearningCore/*.swift` with a small temporary executable under `.local/apple-review-2026-11-08/`; the three observed outputs are recorded above. That probe changes only its own local fixture. Source review establishes the other control-flow findings; their fault-injection regression tests remain to be implemented. No network tutor requests, app-data restore, deployment, or physical-device interaction was performed.

The ordinary fast simulator suite (`apple/scripts/test.sh fast`) passed, exit 0, 123 seconds wall time: LearningCoreTests reported 34 tests; DrusskyTests reported 288 tests with 8 skipped; both had zero failures. Log: `apple/.build/logs/test-fast.log`. This establishes the baseline, not the missing cases above. This review does not certify accessibility, real Pencil/voice behavior, course pedagogy, pronunciation accuracy or all UI flows.
