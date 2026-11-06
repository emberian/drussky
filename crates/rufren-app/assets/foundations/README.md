# Prepared foundation references

112 finite offline WAV references ship with per-asset provenance in `manifest.json`: 27 licensed human recordings and 85 synthetic references. The original twenty retain their entries and audio bytes; the alphabet construction extension adds 15 human and 77 synthetic whole utterances. Playback embeds WAV bytes and needs no speech provider or account.

## Human recording credits

- **а:** The Shtooka Project, individual speaker not identified on the source page. [Commons source](https://commons.wikimedia.org/wiki/File:Ru-а.ogg), [CC BY 2.0 France](https://creativecommons.org/licenses/by/2.0/fr/). The source also carries a public-domain statement; we retain the explicit attribution/license rather than rely on that statement.
- **о:** Сергей Сахно, Vion Nicolas / The Shtooka Project. [Commons source](https://commons.wikimedia.org/wiki/File:Ru-о.ogg), [CC BY 3.0 United States](https://creativecommons.org/licenses/by/3.0/us/).
- **кот:** The Shtooka Project, individual speaker not identified on the source page. [Commons source](https://commons.wikimedia.org/wiki/File:Ru-кот.ogg), [CC BY 2.0 France](https://creativecommons.org/licenses/by/2.0/fr/).

Original Ogg Vorbis files and Commons metadata are retained in `sources/`; SHA1 matches Commons and SHA-256 is recorded. Changes: decoded with FFmpeg to PCM16 WAV for cross-device replay, without trimming, gain, rate or channel changes. Vorbis originals are lossy; conversion adds no lossy codec, while float-to-PCM16 quantization is explicit. These credits do not imply endorsement. Source descriptions identify Russian pronunciation; speaker nativeness and phonemic quality have not been independently certified.

## Synthetic reference credits

The eight introductory synthetic files and 77 alphabet extension files use pinned Silero `v5_6_ru`, Xenia voice, official model revision `e382298387332f2bf461fe6eadfde99104611b0c`, SHA-256 `04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e`, **CC-BY-NC-SA-4.0**. They were generated locally with the bounded CPU worker, with no voice cloning, carrier words, trimming, gain, resampling or splicing. Contextual ма, ка, та model consonant beginnings; no consonant glyph was synthesized as a phoneme. Practice syllables are not vocabulary.

An initial eleven-item synthetic set passed byte/signal checks but an independent ASR probe and spectral comparison raised questions about isolated о. Human а/о/кот replaced those three originals rather than shipping the doubtful isolated-vowel baseline. Previous synthetic originals, original manifest, probe and spectral evidence remain in `research/artifacts/foundations-v1/`; the old probe does **not** describe the replacement human files. No additional model calls were made for replacement.

Every shipped WAV was reopened and checked for its declared encoding/hash/frame identity, non-silence and absence of full-scale samples. This establishes intact playable bytes, not independent pronunciation certification or a learner score. `scripts/prepare-foundation-audio.py` records the initial synthetic recipe and refuses overwrite; it does not reconstruct the mixed curated set.

## Continuation human recording credits

The nine whole-word models retain exact source bytes and Commons SHA1 identities, plus PCM replay derivatives. Sources identify the lexical word; authored teaching stress is **на́, о́н, она́, окно́, оно́, и́, они́, я́, ты́**. Stress marks are teaching annotations, not inserted spoken material. The target for она/окно/оно/они includes initial unstressed О reduction and final stress; они also includes soft Н before И. Source identity and dictionary pronunciation checks are recorded separately from acoustic certification.

- **на, она, они, я, ты:** Сергей Сахно, Vion Nicolas / The Shtooka Project, [CC BY 3.0 US](https://creativecommons.org/licenses/by/3.0/us/).
- **он, окно, и:** The Shtooka Project; individual speaker not identified on Commons source pages, [CC BY 2.0 France](https://creativecommons.org/licenses/by/2.0/fr/).
- **оно:** [1640max / Forvo](https://commons.wikimedia.org/wiki/File:Ru-оно.ogg), [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/). Its PCM derivative is distributed under that same license. The Commons page's Forvo-1640max template includes Wikimedia VRT permission confirmation, ticket **2016092010020528**, stating copyright-holder approval was reviewed; the source-page snapshot is preserved. We rely on that published permission confirmation, not merely a reuploader's asserted license.

Exact per-word source links are in the manifest and in-app Audio credits. Changes: Ogg Vorbis decoded to PCM16 WAV, without gain, trim, sample-rate or channel changes. This adds no lossy codec; PCM quantization is explicit. No endorsement is implied.

The continuation is a mixture of source speakers and recording sessions. In particular оно comes from another named speaker and is a shorter clip; do not imply a single consistent tutor voice. Source descriptions and dictionary stress make these a stronger bounded lexical baseline than unreviewed synthesis; they do not constitute independent native-listener certification of stress, reduction or consonant softness. No inference was run for this continuation.

## Alphabet construction extension

The 92 additional targets are complete source utterances, copied byte for byte from their prepared WAVs. No trimming, cropping, splicing, gain, resampling or channel conversion was applied during installation. Human хор retains its source stereo channels. Rejected synthetic first takes for хор, ла and ра remain only in ignored local preparation storage; the shipped bindings use licensed human recordings.

Every human entry retains an original Ogg, its SHA-256 and Commons SHA1, a canonical per-asset Commons/preparation metadata JSON in `sources/`, attribution, source links, license and recorded decoding transformation. The 15 additions are э, у, ё, ю, лук, люк, соль, мёд, чай, щи, что, объект, хор, ла and ра. Their exact individual credits are in the manifest and generated in-app catalog. This is a mixture of source speakers and sessions, with no independent certification of speaker nativeness or phonemic quality.

Synthetic entries retain pinned model/runtime identity, actual synthesis text and input hashes, generation measurements and requested review checks. Their `review` fields retain recognition observations only when bound to the exact shipped WAV hash. ASR matches do not certify pronunciation; discrepancies remain unresolved lexical observations rather than phonetic pass/fail results. In particular, isolated моё, vowel/glide examples and the human лук/люк contrast must be read with the targeted acoustic observations in `research/artifacts/alphabet-audio-review-v1/REVIEW.md`. Those observations do not establish a native-listener verdict or a learner score. The relatively weak isolated у selection remains unresolved.

Run `scripts/generate-foundation-audio-catalog.py` after changing prepared assets, and use `--check` to verify the checked-in Rust catalog. This performs integrity/provenance checks only; it makes no inference calls and does not transform audio.
