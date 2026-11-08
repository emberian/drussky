# LearningCore

A learner model that knows nothing about any subject. Drussky (Russian) is its first domain;
the core never imports it.

Three layers:

1. **Components** — typed ids `kind:key` (`letter:Ж`, `lex:читать`, `gram:past`). Kinds,
   modalities, edge kinds and confusion channels are open names the domain chooses.
2. **Edges** — structural ones from the curriculum (`form-of`, `uses`, `part-of`,
   `exemplifies`, `prerequisite`), and learned `confused-with` edges from evidence.
3. **Evidence** — an append-only log of events. Everything else (memory, confusions,
   transfer, growth, the frontier) is a pure fold over it: replaying the log rebuilds the model.

A domain implements one protocol, `KnowledgeDomain`: its modalities, its graph, the event an
attempt yields (components touched, modality, grade, confusions), answer normalization, and
the components each token of a free text would exercise.

## Memory

Per (component, modality), FSRS-5 with published default weights (formulas in `FSRS.swift`).
Knowing a word by ear is not knowing how to write it, so each modality has its own trace.

- **Grades**: 1 again, 2 hard, 3 good, 4 easy. A `help` event on an item caps that item's
  next outcome at hard.
- **Transfer**: for each modality pair, first-try success in B on components already strong
  (R ≥ 0.7) in A, against first tries on components with no evidence anywhere. It sets the
  estimate for a modality never practised.
- **Confusions**: each observation weighs 1 and halves every 30 days.
- **Scheduling** (`Scheduling.swift`, the one scheduler): the domain names *review candidates*
  (a thing to recall, the components its review exercises, the modalities it can be reviewed
  in, when it was introduced). A candidate is due in a modality when the weakest of its traces
  there reaches the desired retention (0.9); after a lapse (`again`) it comes back after a short
  relearning step (10 minutes) whatever its stability; one never reviewed in any of its
  modalities gets a first recall after a delay. Urgency: relearning, then (1 − R), then
  confusion pressure. Only outcomes move a trace, so skipping an item never postpones it.
- **Secure** (for fading scaffolds): present recall ≥ 0.85 and stability ≥ 2 days in one of the
  given modalities, with no unrecovered lapse — present memory, not lifetime counts.
- **Review order** (`reviewOrder`): due memories ranked by (1 − R) plus confusion pressure, each
  one's strongest confusion partner right after it.
- **Frontier**: components not yet known (strength < 0.5) whose prerequisites are all at
  R ≥ 0.8. These are ready to learn next.
- **Growth**: on each day, the sum over components of the best R within a modality group,
  giving the expected number recallable.
- **Coverage**: the share of a text's tokens with some candidate component at R ≥ 0.5.
- **Targets** (`Target.swift`): something the learner wants to be able to do, as weighted units
  (each a key — the thing to learn — with candidate components that cover it) grouped into
  segments (sentences). `distance(to:)` gives coverage by weight, distinct keys known, each
  segment's known share, and a ladder of unknown keys by the weight they would unlock.

## Contract: `events.jsonl`

One JSON object per line, UTF-8, ISO-8601 dates. Fields are only ever added, never renamed
or given a new meaning. A line that fails to decode is skipped, and stays in the file.

```json
{"v":1, "id":"…", "t":"2026-11-07T10:31:02Z", "type":"attempt|outcome|help",
 "source":"live|backfill", "item":"p0007-3fa9c1#4", "activity":"dictation",
 "modality":"hear-write", "components":["lex:кот","letter:К"],
 "grade":1, "correct":false, "attempt":2, "expected":"кот", "response":"кат",
 "latency":8.4, "help":["revealed"],
 "confusions":[{"target":"letter:О","produced":"letter:А","channel":"writing"}],
 "review":true}
```

Only `v`, `id`, `t`, `type`, `source`, `item` and `components` are always present. Outcomes
carry `grade` when the domain judges them evidence of recall; an outcome without `grade` records
that the item was finished (e.g. rewritten after the answer was shown) and moves no memory.
Attempts carry `correct: false`. `latency` is the number of seconds since
the previous recorded activity in the same context (a page), when known.

## Contract: graph export

```json
{"format":"learningcore.graph", "version":1, "domain":"drussky-ru", "generated":"…",
 "modalities":["see-write","hear-write",…],
 "nodes":[{"id":"lex:читать","kind":"lex","label":"читать","gloss":"to read","order":212,
           "unit":"Doing things","strength":0.83,"events":14,
           "modalities":{"hear-write":{"retrievability":0.83,"stability":11.2,"difficulty":5.3,
                                       "reviews":4,"lapses":1,"last":"…","due":"…"}}}],
 "edges":[{"from":"form:читаю","to":"lex:читать","kind":"form-of","weight":1},
          {"from":"lex:она","to":"lex:оно","kind":"confused-with","weight":1.9,"note":"writing"}],
 "transfer":[{"from":"hear-write","to":"meaning-write","primedSuccess":9,"primedTrials":11,
              "coldSuccess":3,"coldTrials":8}],
 "frontier":["gram:past","phrase:у меня есть"]}
```

Tests in `LearningCoreTests/` use a toy domain and need no course.
