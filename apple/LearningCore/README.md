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

## Time and the learner

Three more files, still knowing nothing about any subject; the domain names its candidates and
components, the core shapes the day.

- **A day has a shape** (`Day.swift`). `DayShape.shape` reads the last activity against the clock:
  the first page on a new calendar day is a *morning* page; after an absence of
  `DayPolicy.welcomeBackAfter` (7 days) it is a *welcome-back* page. `DayComposition.morning`
  takes the easiest due items (highest recall, no lapse) as a warm-up of `warmUp` (3), then the most
  urgent up to what is left of the day's `reviewCap` (25, settable) and the page's own
  `morningReviews` (12); everything else is **deferred, most urgent first** (`overflowOrder`:
  `DueReview.priority` = relearning + (1 − R) + confusion pressure, so the most overdue and most
  confused come back first on the next page or day). `welcomeBack` re-tests a stratified sample
  (`welcomeBackSample` = 8, spread evenly over the urgency range from the most urgent to the
  easiest) rather than everything. `weave` places up to 3 on an ordinary page, within the cap. The
  desired retention (`ReviewPolicy.retention`, 0.8–0.95) is the one knob on when anything comes due.
- **Frontier pulls** (`Day.swift`, `KnowledgeModel.frontierDecision`). The course stays the spine.
  Given the components the next course page introduces: if at least `FrontierPolicy.minWeak` (3)
  of their prerequisites outside the page have been met and are weak (strength < 0.6 or relearning),
  the decision is `.remediate(weakest first, at most 6)`: the domain builds a short page from those
  items before the course goes on. Otherwise, when enough is due for a review page
  (`reviewPageThreshold`, 10) but every not-yet-learned component on the page has all its
  prerequisites at R ≥ 0.8, the decision is `.preferCourse`: the course page, with reviews woven in,
  rather than a page of review. Prerequisites never met are not weak — the course will teach them.
- **Measured retention** (`Probes.swift`). Every `ProbePolicy.every` (7 days) `drawProbes` samples
  `sampleSize` (6) practised candidates stratified by present recall into thirds, each in one of its
  practised modalities, and schedules one probe per delay (1, 7, 30 days) — deterministically from a
  seed. A `Probe` is placed on a page as an ordinary exercise; only the `ProbeLedger` knows. When it
  is answered, the trace's `recallBefore` (R just before that review) is recorded against success.
  `Calibration` bins predictions (5 bands) per modality and overall: observed rate vs mean prediction,
  Brier score, expected calibration error, and `overconfidence` = predicted − observed.
  `ParameterNudge` turns that into one bounded step: outside a ±0.05 deadband and with at least
  `minObservations` (12), the FSRS stability scale moves 2% towards the evidence, within 0.7–1.3;
  `FSRS.scalingStability(by:)` scales the four initial stabilities and the growth factor (e^w8),
  leaving difficulty alone. The graph export carries `calibration`.
- **The desk notices you** (`Fatigue.swift`). `Fatigue.estimate` reads the current session (events
  with no gap over 45 minutes) for four signals, each explained in words: answers slowing (median
  latency of the last third over the first third), misses in the last 8 outcomes, tries per finished
  item, and minutes on the page. Score = 0.3·slower + 0.3·missing + 0.2·retrying + 0.2·long;
  ≥ 0.55 is *tired*, ≤ 0.15 with no misses and no slowing is *sharp*; fewer than 6 outcomes and
  only time counts. `HourlyPreference` is first-try success per modality by hour band (night,
  morning, afternoon, evening): `order` sorts modalities best-first for the hour (8 trials needed),
  `standout` names one when it clearly leads. What a domain does with these is its own: Drussky
  orders the morning page's reviews by the hour's preference, shortens a page when tired (two
  remaining reviews become recognition, the rest wait, ungraded), and allows one frontier word when sharp.

## Judging one's own writing

For recall written with a pen (write what you hear, write the Russian for this, fill the gap) the
learner is the judge and the recognizer is a witness. After writing, Check shows the model answer
beside the ink and the learner says *right* or *not quite*; the recognizer reads silently, hints at
the likely verdict, and its reading goes into the log beside the verdict. Why:

- **The generation effect and self-assessment** are both well supported: producing the answer from
  memory and then comparing it with the model is the learning event, and the comparison is itself
  retrieval practice. A recognizer that rules first short-circuits the comparison.
- **A misreading erodes trust in the instrument.** When the reader rules "wrong" over a right
  answer the learner has to argue with the book ("I wrote it right"); when it rules first and ticks
  a wrong answer, the error passes unexamined. Either way the learner stops reading the model.
- **Self-verdicts are honest enough when they are checked.** The reader's reading is kept as
  evidence, so a learner who marks "right" over something the reader read as something else gets
  a *worth another look* note on the word, never an accusation.

In the evidence, `judge` says who decided ("self" or "reader") and `confidence` how sure the
record is. The verdict decides success; the reader only modulates: right and the reader agrees →
*good* (first time) or *hard*, confidence 1; right with no reading → the same, confidence 0.7;
right while the reader read otherwise → *hard* (one notch of doubt), confidence 0.4, a
disagreement; *not quite* → *again* whatever the reader saw (confidence 0.6 when it had agreed).
Tracing and copying stay ticked by the reader: there is nothing to recall. The learner can hand
the chair back to the reader in Settings ("Judge my writing"); the domain's `grade` reads either
kind of result (`Drussky/Judgement/`).

## Metrics

`Metrics.swift` is a time series per (component, metric name): `MetricStore` appends samples,
answers `samples`, `latest`, `mean` over a window, and `change` (median of the last window against
the median of the one before, nil until each side has two), and `MetricLog` keeps them as JSON
lines (`metrics.jsonl`), append-only like the events. The core does not know what a metric means;
Drussky's fluency instruments (words a minute read with a finger or said aloud, strokes a second
and legibility of a word written from memory, narrated sentences caught at 1× and ¾×) name theirs
in `Drussky/Fluency/`.

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
 "review":true, "judge":"self", "confidence":0.4}
```

`judge` and `confidence` appear on finished recall writing (see "Judging one's own writing").

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
 "frontier":["gram:past","phrase:у меня есть"],
 "calibration":{"binCount":5,"overall":{"count":24,"predicted":0.81,"observed":0.75,"brier":0.17,"error":0.08,
                "bins":[{"lower":0.6,"upper":0.8,"predicted":0.72,"observed":0.6,"count":10},…]},
                "series":[{"modality":"hear-write",…}]}}
```

`calibration` is present once any retention probe has been answered (Probes.swift).

Tests in `LearningCoreTests/` use a toy domain and need no course.
