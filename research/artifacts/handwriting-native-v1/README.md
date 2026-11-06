# Synthetic native handwriting probe

These three drawings are fabricated geometric uppercase forms, not Ember's handwriting. They establish a small integration observation and expose a script-disambiguation issue. They do not measure real handwriting recognition accuracy or learning quality.

The labels were stored separately from the strokes. Neither native recognizer received an expected word, a search query, or a custom vocabulary. The displayed PNGs are derivatives rendered from the exact PencilKit archives under explicit light appearance; original ink geometry was not changed for Vision.

| Synthetic label | Image | PencilKit 9, full app lifecycle | Vision revision 3, raw Unicode | Vision request time |
| --- | --- | --- | --- | --- |
| КОТ | [ink001](ink001.png) | No result | `KOT` (Latin) | 102.293 s |
| ОНА | [ink002](ink002.png) | No result | `OHA` (Latin) | 0.075 s |
| ТАМ КОТ. | [ink003](ink003.png) | No result | `TAM KOT.` (Latin) | 0.077 s |

Vision requested `ru-RU`, accurate recognition, no language correction and no custom words. Its returned confidence values are retained in [receipt.json](receipt.json); they are not calibrated probabilities of correct Russian handwriting. The first request's 102-second delay was observed, not diagnosed or treated as a representative average.

These particular uppercase shapes look the same in Cyrillic and Latin. Consequently, a raw Unicode mismatch is not evidence that the marks are wrong. The receipt separately demonstrates the target-independent substitutions `A→А`, `H→Н`, `K→К`, `M→М`, `O→О`, `T→Т`, yielding the three intended labels. That is script disambiguation for a Russian-only task, not phonetic transliteration or spelling correction. It must not be applied indiscriminately to future mixed-script mathematics.

PencilKit's Russian capability query succeeded, but both the packaged async-main and full `NSApplication.run` variants returned no text for all three same archived drawings. The recorded hashes demonstrate that the lifecycle comparison did not change the ink. Further retries stopped. Real native Pencil ink and an ML Kit comparison remain the next useful evidence.

The deterministic generator can recreate the neutral stroke input and separate labels:

```sh
python3 scripts/research/make_synthetic_ink.py --out /tmp/drussky-synthetic-ink
```

The output directory must be new. `synthetic-ink.json` must have SHA-256 `5f6c65bb73acdfb024985567135b3ac6ddd4fe54d719ed5902191a31dcf5f0bd`. Feed only that file's geometry to a recognizer; use the labels after recognition. Full exploratory native sources, archives and failure receipts remain in the ignored `.local/research-writing-2026/algorithms/` directory. See [the comparison note](../../HANDWRITING_ALGORITHMS.md) for the primary sources and limits.
