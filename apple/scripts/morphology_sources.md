# Sources for `morphology.json`

The data in `build_morphology.py` (roots, prefixes, suffixes, lemma segmentations, the cognate
bridge) is a learner's view of Russian word formation, not a new analysis. Every root, affix and
segmentation comes from, and can be checked against, the standard references below. Nothing is
invented; where the references disagree, the policy in the next section says which reading the
file takes, and the entry carries a `note`.

## References

- **Wiktionary**, Russian entries (en.wiktionary.org and ru.wiktionary.org). Each lemma's
  *Морфологические и синтаксические свойства* section gives the morphemic segmentation
  (`Корень: -пис-; суффикс: -а; глагольное окончание: -ть`, cited there to Tikhonov) and the
  *Этимология* section its origin (cited there to Vasmer). This was the first stop for every
  segmentation and for every cognate's Greek or Latin source.
- **А. Н. Тихонов, Морфемно-орфографический словарь** (2002) — the synchronic segmentation
  ru.wiktionary quotes; the segmentations in `lemmas` follow its prefix | root | suffix | ending
  layout.
- **А. И. Кузнецова, Т. Ф. Ефремова, Словарь морфем русского языка** (1986) — the list of
  productive roots, prefixes and suffixes with their frequencies; the choice of the ~190 roots and
  the 26 prefixes and 19 suffixes in the file follows its frequency ranks, kept to what the course
  and the library actually use.
- **М. Фасмер, Этимологический словарь русского языка** (Russian edition by О. Н. Трубачёв) — the
  etymologies behind the family notes (вчера from вечер, завтра from за утра, неделя as the
  no-work day, окно from око, спасибо from спаси Бог, робот from Czech robota, число with читать)
  and behind every borrowing's source language in `cognates` and `loan`.
- **А. А. Зализняк, Грамматический словарь русского языка** — parts of speech and inflection
  (which forms in `forms` belong to which headword).
- **Л. П. Крысин, Толковый словарь иноязычных слов** — the borrowings: source word and route
  (Byzantine Greek vs Western), which decides th → ф (арифметика, миф) against th → т (теория).
- **Liddell–Scott–Jones** and **Lewis & Short** — the Greek and Latin glosses on the cognate tiles
  (θεωρ-, σοφία, functio, structura).

## Policy

1. **Segmentation is Tikhonov's**, prefix | root | suffix(es) | ending, written in `build_morphology.py`
   as `пере|[пис]|а'|ть`. The infinitive `-ть`/`-ти` and the thematic vowel are separate pieces so
   that tiles can be built from them; the validator only requires that the pieces spell the word.
2. **Roots are etymological where the link is transparent to a learner, synchronic otherwise.**
   Where Tikhonov treats a historically compound stem as one root (справедлив-, приятн-) but the
   course has the parent word (правда), the entry names the etymological root and says so in a
   `note` (справедли́вый → прав). Where the link is opaque to a modern reader (приятный ← прияти,
   ужин ← юг, улица ← ула, опять ← пята) the lemma has no family and the `note` tells the story.
3. **Homographic roots are separate ids**: вод (water) and вед/вод (lead), дел (do) and дели
   (divide), долг (long) and долж (debt), лет (fly) and лето (summer/years), им (take) and им2
   (name). An id is a label, not a form; `forms` lists the allomorphs.
4. **Allomorphs** are listed as they appear inside words (чит/чт/чёт/чис; ход/ид/й/ш/шл/шед;
   мог/мож/моч/мо), so each member's root morpheme can be checked against the list exactly.
5. **Families need no minimum size**: a root with one member today (мор, лун) stays, so the
   next member the learner meets lands in it; the inside cover only shows roots with a known
   member, and the generated page only uses roots with two.
6. **Cognate rules** are the regular correspondences a reader of English can apply (th → т/ф,
   ph → ф, h → г or dropped, y → и, x → кс, c → ц/к, -tion → -ция, -ure → -ура, -ment → -мент,
   -ism → -изм, -ics → -ика, -ia → -ия, -ma → -ма, Latin -us/-um and Greek -os/-on dropped, a few
   -us kept). Each cognate lists the rules it shows and, for the compound ones, its Greek/Latin
   parts with glosses; a `note` records the exceptions (арифметика, иерархия, матрица, конус).

## Checked

`validate_morphology.py` checks what can be checked mechanically: pieces spell words, root pieces
are listed forms, families share roots, affixes are declared, derivations point at lemmas, coverage
of the course lexicon, the lemma map and the 300 most frequent library words, cognate tiles spell
their words and name existing rules, and stress marks are well formed. The etymologies and glosses
themselves were read against the references above when the tables were written (2026-11-07); a
reader who finds one wrong should fix the table and rebuild.
