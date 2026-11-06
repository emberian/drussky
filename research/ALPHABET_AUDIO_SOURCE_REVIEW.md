# Alphabet audio source preparation, 2026-11-06

The ignored `.local/alphabet-audio-source-review/` contains four acquired human vowel originals and lossless PCM16 decoding derivatives, exact source metadata, and a routing plan for all 92 requested references. No runtime or packaged audio changed. `prepared.json` binds each acquired file to its exact request ID, ordinary spelling, original and derivative SHA256, Commons SHA1, source URL, license, credit, and decoding measurements. These are candidates, not independently approved pronunciations.

## Concrete source candidates

| Target | Commons source title | Attribution/license | Local acquisition |
| --- | --- | --- | --- |
| э | [Ru-э.ogg](https://commons.wikimedia.org/wiki/File:Ru-э.ogg) | Pogrebnoj-Alexandroff; CC BY-SA 2.5 | Original + WAV |
| у | [Ru-21-буква-У.ogg](https://commons.wikimedia.org/wiki/File:Ru-21-буква-У.ogg) | Cherus; CC BY-SA 3.0 | Original + WAV |
| ё | [Ru-ё (jɵ).ogg](https://commons.wikimedia.org/wiki/File:Ru-ё_(jɵ).ogg) | Pogrebnoj-Alexandroff; CC BY-SA 3.0 | Original + WAV |
| ю | [Ru-32-буква-Ю.ogg](https://commons.wikimedia.org/wiki/File:Ru-32-буква-Ю.ogg) | Cherus; CC BY-SA 3.0 | Original + WAV |
| лук, соль, чай | Commons `Ru-TARGET.ogg` | Shtooka Project; individual speaker unreported; CC BY 2.0 FR | Source metadata only |
| мёд, щи | Commons `Ru-TARGET.ogg` | Shtooka Project; individual speaker unreported; CC BY 2.0 FR | Source metadata and page wikitext only |
| люк, объект | Commons `Ru-TARGET.ogg` | 1640max, originally Forvo; Commons records VRTS permission; CC BY-SA 4.0 | Source metadata only |
| что | Commons `Ru-что.ogg` | Exact credit/license retained in selection.json | Source metadata only |

Do not substitute `Ru-ё (ɵ)` for initial ё: the selected `(jɵ)` source specifically names the glide. У and Ю originate from letter-pronunciation files. Their names are not evidence that the audio contains only the intended sound. First screen for extra spoken labels, then inspect/audition onset, vowel and ending. Their complete decoded durations are 2.247s and 1.646s; duration includes silence and is not a claim about speech content. Э is 0.454s; Ё 0.758s. All four decode completely, remain mono at their original 44.1/22.05kHz rates, and have zero full-scale PCM16 samples. No gain, trimming, resampling, speech recognition or phonetic certification was performed here.

Commons returned HTTP429 on the third acquisition, and again after two additional successful downloads during the next spaced attempt. Acquisition stopped at four originals rather than hammering the service. The twelve exact candidates were recorded in `selection.json` before fetching. `acquire.py` is a review-only helper with an 8MiB/file bound and no overwrite of originals; do not rerun it until service cooldown. The selected originals total under 60KiB. `routing-plan.json` distinguishes acquired human candidates, rate-limited candidates and the 80 root-owned synthetic targets. Root may synthesize the eight pending words if acquisition remains unavailable; preserving a complete usable course should not require an unavailable source service.

## Reuse and quality decisions

The existing foundation manifest supplies licensed human а/о/кот plus further words, but none matches a new isolated vowel. Existing RQ02/RQ03 preserve `Лук.`/`Люк.` with punctuation and upper case, whereas the new ordinary targets are `лук`/`люк`. Treat them as useful comparator recordings with explicitly different authored source text; do not silently rewrite their provenance. The long OmniVoice utterance `Это лук, а это люк.` is not an exact whole-word asset or a source to crop into a supposedly natural new utterance. No natural phrase is assembled from word recordings.

The retained foundation synthetic probe and actual `spectral-sanity.json` flag the old isolated о: its high-RMS-window band centroid nearly matches а, and ASR writes кат for кот. These observations motivated human anchors but do not prove vowel identity. RQ06's actual review retains a perfect lexical ASR transcript while its intended homograph stress reversal remains unresolved. That is direct evidence against using ASR text as a stress pass.

[Silero's upstream documentation](https://github.com/snakers4/silero-models) describes Russian stress/homograph controls; this supports retaining explicit marked stress through the existing checked frontend, not claiming every emitted accent is correct. The pinned full-precision Qwen/Silero references remain the runnable baseline. Existing [OmniVoice audition](OMNIVOICE_AUDITION.md) found a useful numeric wording result but a possible ending/duration defect. Its [upstream voice-design card](https://huggingface.co/k2-fsa/OmniVoice/blob/c5fdb5ccb189668d56333f77ba2629f4cd7535f4/README.md) does not establish better isolated Russian teaching sounds. No new model acquisition, API entitlement, billing or model sweep is justified by this source review. See [VOICE_QUALITY_NEXT.md](VOICE_QUALITY_NEXT.md) for the bounded candidate comparison and actual memory costs.

For each of the 92 references, preserve whole original audio, actual synthesis/source text and exact authoring instruction. Screen word identity, additions, truncation and inflection separately from stress, softness, reduction and naturalness. For the four human vowels ASR can flag extra words only. For лук/люк and соль inspect the consonant-to-vowel transition rather than assigning a whole-utterance similarity score. [Primary Russian palatalization research](https://arxiv.org/abs/2306.12789) studies context-dependent articulation and incomplete neutralization; a universal F2 cutoff is not a validated learner assessor. Keep source acoustic measurements as localized evidence and obtain qualified listening before a claim of native phonetic acceptability. The friend has no recording or grading duty.

This review used `~/swarm/research.py kagi` to locate primary Commons and upstream documentation, followed by direct Commons API metadata/wikitext. All acquisition and conversion commands were bounded, one at a time, without models, compilation, browser startup or runtime changes.

## Controlled resumed acquisition

A later single resumed pass acquired all eight pending word originals at ten-second request intervals. Each source passed its exact Commons SHA1 before decoding; all twelve originals and WAVs are now present. `resumed-acquisition-receipt-20261106.json` records the eight actual requests and SHA256s, while `resumed-acquisition-bound-20261106/result.json` records 82.75 seconds elapsed, 21.85MiB sampled peak, exit zero and no remaining owned processes under the 384MiB/180-second guard. No HTTP429 recurred. `prepared.json` now contains all twelve completed valid assets and plaintext credits alongside unchanged HTML metadata. This supersedes the earlier pending acquisition status; phonetic/extra-word review remains open.

The eight word sources are: Shtooka Project (individual speaker unreported), CC BY 2.0 FR, for лук/соль/мёд/щи/что/чай; 1640max, CC BY-SA 4.0, for люк/объект. No exact new original hash matched an existing packaged OGG, and no new decoded WAV hash matched existing assets or research WAVs. They therefore provide new independent source recordings rather than disguised reuse of synthetic examples. Text semantics remain whole single words; source provenance and recording differences remain visible for contrasts.

## Targeted lexical-flag follow-up

The first87-item ASR screen found19 text mismatches, dominated by disconnected syllables. Detailed triage is in `.local/alphabet-audio-source-review/replacements-01/ASR-JUDGEMENT.md`: это/эта in Это книга can reflect unstressed-vowel neutralization; моё окно→Мое окно is ё orthography; isolated letter-name/syllable outputs do not establish their acoustic identity. They do not authorize nineteen retries until ASR agrees.

Exact human хор and four direct syllable candidates ла/ра/жа/ща were selected from Commons metadata before acquisition, preserving their licenses and speaker attribution. Exact моё was absent from direct metadata and Commons audio-title search; only longer expressions were found, which were deliberately not cropped. A second exact Russian люк source was also absent from search; the additional hit was Belarusian and was not substituted. Follow-up candidates and acquisition outcome remain separate in the `replacements-01` directory.
