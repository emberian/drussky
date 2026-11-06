# Prepared foundation references

Eleven finite offline WAV references ship with per-asset provenance in `manifest.json`. **а, о and кот are licensed human recordings; the other eight are synthetic.** Playback embeds WAV bytes and needs no speech provider or account.

## Human recording credits

- **а:** The Shtooka Project, individual speaker not identified on the source page. [Commons source](https://commons.wikimedia.org/wiki/File:Ru-а.ogg), [CC BY 2.0 France](https://creativecommons.org/licenses/by/2.0/fr/). The source also carries a public-domain statement; we retain the explicit attribution/license rather than rely on that statement.
- **о:** Сергей Сахно, Vion Nicolas / The Shtooka Project. [Commons source](https://commons.wikimedia.org/wiki/File:Ru-о.ogg), [CC BY 3.0 United States](https://creativecommons.org/licenses/by/3.0/us/).
- **кот:** The Shtooka Project, individual speaker not identified on the source page. [Commons source](https://commons.wikimedia.org/wiki/File:Ru-кот.ogg), [CC BY 2.0 France](https://creativecommons.org/licenses/by/2.0/fr/).

Original Ogg Vorbis files and Commons metadata are retained in `sources/`; SHA1 matches Commons and SHA-256 is recorded. Changes: decoded with FFmpeg to PCM16 WAV for cross-device replay, without trimming, gain, rate or channel changes. Vorbis originals are lossy; conversion adds no lossy codec, while float-to-PCM16 quantization is explicit. These credits do not imply endorsement. Source descriptions identify Russian pronunciation; speaker nativeness and phonemic quality have not been independently certified.

## Synthetic reference credits

The other eight files use pinned Silero `v5_6_ru`, Xenia voice, official model revision `e382298387332f2bf461fe6eadfde99104611b0c`, SHA-256 `04cc9b1d08ce38bcaa9d02c488d80d3e6806d17f7eaa7c95982f36a3cbf50b2e`, **CC-BY-NC-SA-4.0**. They were generated locally with the bounded CPU worker, with no voice cloning, carrier words, trimming, gain, resampling or splicing. Contextual ма, ка, та model consonant beginnings; no consonant glyph was synthesized as a phoneme. Practice syllables are not vocabulary.

An initial eleven-item synthetic set passed byte/signal checks but an independent ASR probe and spectral comparison raised questions about isolated о. Human а/о/кот replaced those three originals rather than shipping the doubtful isolated-vowel baseline. Previous synthetic originals, original manifest, probe and spectral evidence remain in `research/artifacts/foundations-v1/`; the old probe does **not** describe the replacement human files. No additional model calls were made for replacement.

Every shipped WAV was reopened and checked for its declared encoding/hash/frame identity, non-silence and absence of full-scale samples. This establishes intact playable bytes, not independent pronunciation certification or a learner score. `scripts/prepare-foundation-audio.py` records the initial synthetic recipe and refuses overwrite; it does not reconstruct the mixed curated set.
