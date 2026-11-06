# Reproducible extension merge — canonical .4 activated

`beginner-v1-2026-11-05.4.json` is the deterministic candidate activated as
`data/beginner-v1.json` after the owner’s coordinated grant. The next build embeds
`.4`; the previously compiled first-iPad artifact still contains `.3` and is
unchanged. Nothing in this staging directory is selected directly by Rust
`include_str!`; the canonical data file contains the verified candidate bytes.

Acceptance: `cargo test -p rufren-content --test content_lineage` passed **3/3**
before activation. The activated pack then passed
`cargo test -p rufren-content --test content_contract` **9/9**. App composition and
new device/web artifact acceptance belong to the owner’s following build queue.

The draft preserves the pack ID, schema version, all existing item identities, all
103 base phrase payloads, AI-teacher ownership, pending generated speech, hidden
production references, and optional meal conversations. It appends the reviewed
Astra extension's six arrays, without copying extension-only root metadata.
Its revision is `2026-11-05.4`; ancestry is `.1`, `.2`, `.3` in order. The base
payloads acquire explicit immutable phrase lineage: `.3` with exact `.1/.2`
predecessors. New phrases use `.4` with an explicitly empty predecessor list.
The `.3` baseline is preserved under `tests/fixtures` so staging remains reproducible
after activation. The Python check reconstructs all 103 imported v2 Events and
compares their complete envelopes and facts, not only displayed text.

| Inventory | Base .3 | Addition | Draft .4 |
| --- | ---: | ---: | ---: |
| Units | 4 | 2 | 6 |
| Lessons | 16 | 8 | 24 |
| Phrases | 103 | 66 | 169 |
| Exercises | 116 | 56 | 172 |
| Targets | 34 | 16 | 50 |
| Sources | 8 | 4 | 12 |

The alphabet remains 33 letters. Listening choices remain balanced: 13 correct
first, 13 correct second. The prerequisite graph has 16 encounter waves, with
branches for everyday language before the advanced previews. For example,
quantities becomes available alongside describing, then preferences alongside
roles. These are encounter suggestions, not proficiency gates.

To recreate the draft and manifest, without writing the embedded source:

```text
python3 crates/rufren-content/tools/stage_extension.py
```

To check the staged bytes, immutable baseline/extension hashes, ancestry, unchanged base
items, graph reachability, AI-speech defaults, and combined editorial contracts
without writing the staged files:

```text
python3 crates/rufren-content/tools/stage_extension.py --check
```

The script calls the extension's read-only editorial validator against the draft.
That checks full links/membership, target cycles, stress/spelling consistency,
production hiding, audio requirements, and extension scaffolds/assessment shapes.
It does not run Cargo or certify generated speech, learning outcomes, or every
Russian sentence. The peer review and sources remain described in the extension
README and `research/CURRICULUM_NEXT.md`.

The coordinated activation completed the following content steps; retain them for
future editions:

1. Recheck that the immutable baseline/extension still match `merge-manifest.json` before
   activating this exact candidate as the embedded pack.
2. Replace the Rust contract test’s “one available lesson” assertion with a
   topological-reachability invariant. An available lesson must have all encounter
   prerequisites satisfied; accumulating available frontiers must reach every
   lesson without a cycle. Require an actual branch rather than a linear chain.
3. Update the explicit revision/ancestry assertions from `.3` to `.4`, and update
   inventory documentation. Existing anchor, script/position-leak, optional-friend,
   generated-speech, and old-key compatibility tests remain applicable.
4. Run the owner’s scheduled focused content targets. Those passed as recorded
   above; app composition and new artifact checks remain in the owner’s queue.
   The existing iPad artifact is unchanged.

The approved per-phrase revision seam preserves existing IDs, source attribution,
parents, event envelopes, review keys, and history. `tests/content_lineage.rs`
compares actual core Events, serialized bytes, spaced-review state in all four
skills, reverse import order, and a concurrent user edit. Imported authored events
stay version 2 even when the surrounding event protocol advances. See
[IDENTITY_FOLLOWUP.md](IDENTITY_FOLLOWUP.md) for the implemented contract and the
remaining content-equivalence question.
