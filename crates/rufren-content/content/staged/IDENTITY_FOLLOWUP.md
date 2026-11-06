# Immutable phrase lineage across pack editions

The `.4` integration fixes the pack-wide reset of unchanged phrase history. It does
not change stable phrase UUID derivation or rewrite existing attempts.

`Phrase` now accepts two optional fields, declared together:

```json
{
  "phrase_revision": "2026-11-05.3",
  "previous_phrase_revisions": ["2026-11-05.1", "2026-11-05.2"]
}
```

`ContentPack::resolved_phrase_lineage(&Phrase)` returns `ResolvedPhraseLineage`
containing borrowed `revision` and `previous_revisions`. When both optional fields
are absent, it returns the original pack revision and pack ancestry. Explicit empty
predecessors mean no predecessors; they never fall back. Validation rejects partial
pairs, empty/oversized/unknown revisions, duplicate predecessors, and self-parents.
The optional fields default and are omitted when absent, so old snapshots remain
readable under schema 1.

The 103 unchanged base payloads are pinned to `.3` and its exact existing `.1/.2`
parent set. Each of the 66 new extension phrases begins at `.4` with `[]` parents.
Pack-level `.1/.2/.3` ancestry remains edition metadata and the legacy fallback; it
does not manufacture ancestors for explicitly new items.

Every deterministic authored import must derive **all** immutable fields from the
same resolution: revision UUID, source attribution suffix, and parent UUID set.
Using a stable UUID while emitting a new pack source or parent list is a collision,
not a harmless metadata update. The namespace/device and recorded time remain
unchanged. Imported authored `PassageRevised` envelopes remain **version 2**, because
those exact IDs were already emitted at v2; a general event-protocol version bump
must not silently alter them.

The regression fixture preserves the exact `.3` pack. `content_lineage.rs` checks
all 103 actual core Event values and serialized bytes, all four skill ReviewKeys,
existing spaced-review due/history state, reverse-order import convergence, new
phrases with no fabricated parents, and preservation of a concurrent user edit.
All authored payload fields—including stress and targets—must also remain equal
before a historical revision is reused.

A future edit to an existing phrase must choose a new phrase revision and declare
its actual previous authored revisions. Keep human-edited heads as real independent
history; publishing an authored update does not authorize superseding them. The
current fix does not infer mastery or merge distinct user/reference histories.

A finer content-addressed or explicitly declared equivalence mechanism remains a
future design question. A new generated voice or display note may change packaging
without changing the target message; a changed stress or intended meaning can
change the learning object even when ordinary spelling is identical (за́мок versus
замо́к). Prompt/cue versions can change what an answer demonstrates separately from
reference text. Any later aggregation across versions must retain immutable
attempts, exact audio/exercise provenance, deterministic replication, and an
inspectable reason. Do not replace the current pinned lineage with an ad hoc hash
of arbitrary JSON or silently transfer evidence between unequal content.
