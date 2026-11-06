# Foundation synthetic-reference probe

One independent, target-unconditioned local Qwen3-ASR probe was run on a documented derivative of the eleven original foundation models. The derivative retains original samples in manifest order with 0.5 seconds of silence after each model; it applies no gain, trimming, or time changes. `lexical-probe-source.json` records original hashes and exact half-open spans. `lexical-probe-transcript.json` retains ASR identity, resampling provenance and resource/timing evidence.

Expected source sequence: **а / ма / ам / да / нет / о / ка / та / кот / там / Там кот.**

Actual transcript hypothesis: **Амаамда нет Акта кат там там кат**.

This result merges initial syllables, does not independently recover the о/ка/та sequence, and writes **кат** for the intended **кот** twice. These are flags for targeted listening, especially the stressed О models and кот. Short synthetic syllables and artificial disconnected lists can themselves challenge ASR. The transcript cannot establish whether the source is wrong, whether the recognizer is wrong, or whether an isolated syllable is a letter name. No model was regenerated on this basis and no native/phonemic certification is claimed.

Every original WAV passed independent decoding, hash/frame/encoding consistency, non-silence and zero full-scale/clipping checks. Prepared means available and intact; it does not settle these perceptual flags. No learner pronunciation score or automatic correction follows from this probe.

An additional CPU-only spectral sanity comparison is recorded in `spectral-sanity.json`. It selects the highest-RMS 60 ms original window, without phoneme alignment. The isolated а and о models have very similar band centroids (~1083 Hz within 250–2500 Hz) and band power ratios (~−11 dB for 300–800 relative to 800–1500 Hz); the strongest кот window differs (~549 Hz and +13.5 dB). This raises a useful listening question about isolated о alongside the ASR flag. Spectral bands are not formants or vowel labels and do not adjudicate correctness. No generation was repeated.

## Baseline replacement

The shipped а/о/кот assets were subsequently replaced with licensed human Shtooka recordings obtained from Wikimedia Commons, without new synthesis or recognition. This probe and spectral sanity file refer exclusively to the initial synthetic baseline. Its three replaced original WAVs are preserved in `synthetic-originals/`; `synthetic-manifest.json` retains the original eleven-item identities. The shipped mixed-source manifest and original licensed OGGs are under `crates/rufren-app/assets/foundations/`. Do not apply this old probe transcript to the replacement files.
