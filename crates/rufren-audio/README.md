# Rufren audio instruments

Portable Rust PCM/WAV handling and reproducible recording displays. No device,
file-system, network, native-library, or platform-specific dependency is needed.
The same analysis runs in native applications and `wasm32-unknown-unknown`.

```rust
use rufren_audio::{PcmAudio, analyze, compare};

# fn example(reference_bytes: &[u8], rate: u32, samples: Vec<f32>) -> Result<(), rufren_audio::AudioError> {
let reference = PcmAudio::from_wav(reference_bytes)?;
let attempt = PcmAudio::from_mono(rate, samples)?;
let reference_view = analyze(&reference)?;
let attempt_view = analyze(&attempt)?;
let observations = compare(&reference_view, &attempt_view);
let preserved_original = attempt.to_wav();
# Ok(())
# }
```

`PcmAudio` retains validated original interleaved floating-point samples and
sample-rate/channel metadata. Its SHA-256 identifies canonical decoded PCM, so
container metadata does not change identity. WAV export preserves f32 values;
WAV import supports unsigned 8-bit and signed 16/24/32-bit PCM and IEEE float32.
Compressed WAV, WAVE_FORMAT_EXTENSIBLE and RF64 are explicitly unsupported. Never
discard the original capture merely because an analysis derivative exists.

Input is limited to 60 seconds, 16 million channel samples, eight channels, rates
from 8 to 192 kHz, and 64 MiB WAV files. NaN, infinities, samples outside [-1, 1],
incomplete frames, malformed sizes and truncated chunks are rejected before
derived allocation. All parsed chunk offsets are bounded. The API does not
deserialize `PcmAudio`, so decoding cannot bypass validation.

`analyze` is a bounded batch operation, suitable for a worker/background thread,
not an audio callback. Stereo/multichannel input is averaged to mono for analysis;
original samples remain intact and a strong cancellation warning is available.
For browser UIs, use `PcmAudio::from_interleaved_cooperatively` and
`analyze_cooperatively`. Both accept a callback returning a future of `bool`.
Schedule a real event-loop task in that callback; return false when the view is
unmounted or a newer recording replaces the work. All expensive phases yield in
bounded chunks, and cancellation returns `AudioError::Cancelled` without a
partial analysis. The synchronous functions share the same computation with
immediately-ready callbacks, producing identical measurements. A callback that
is immediately ready does not itself keep a browser responsive.

`cargo run -p rufren-audio --release --example timing` explicitly probes the
longest supported mono clip at the highest rate, reporting total time and the
longest CPU segment between checkpoints. Timing depends on the host, compiler
profile, and load; a native measurement is not a measured browser latency.

Analysis version `rufren-instruments-v1` specifies:

- Original-time envelope with at most 1,200 min/max/RMS bins.
- Windowed-sinc low-pass resampling to 16 kHz, with 256 fractional phases and
  filter support that expands for downsampling. There is no amplitude normalization.
- 512-sample periodic Hann STFT, 160-sample hops, one-sided coherent-gain
  corrected bin levels, and a -100 dB display floor. Bin levels are relative to
  a full-scale sinusoid, not SPL or spectral density. Tail frames are zero-padded.
- A YIN-style pitch instrument at 8 kHz: a 40 ms difference window, 20 ms lag
  support and hops, and the first normalized-difference trough below 0.15.
  Candidate range is 50–400 Hz. Confidence describes periodicity, not a calibrated
  probability. Quiet/aperiodic frames have no F0; a missing F0 is not an error.
- Source-domain RMS, original-channel peak/full-scale sample fraction, 20 ms
  blocks below -40 dBFS, and leading/trailing quiet duration. Quiet blocks are
  not a speech activity detector: some consonants are naturally quiet.

The pitch algorithm follows the cumulative normalized difference approach in
[de Cheveigné and Kawahara (2002)](https://pubmed.ncbi.nlm.nih.gov/12002874/).
It has synthetic regression coverage, not a labeled Russian speech evaluation.
Octave errors, creaky voice, breathiness, rapid pitch movement and out-of-range
voices require listening and careful review. No formant, phoneme alignment,
background-noise estimate, interruption detector, stress classifier, or native
accent assessment is claimed. Room noise may itself be periodic.

`compare` preserves duration differences and reports duration ratio, RMS level
difference and optional median voiced-pitch difference. These are observations
for matched phrase/crop boundaries. Microphone gain and distance affect RMS;
voices have different pitch ranges. There is no spectral-distance objective or
global pronunciation/proficiency score. A whole-phrase pitch median is not an
intonation contour comparison. Timed observations and A/B listening help the
learner and teacher decide what to practice. A friend can offer an additional
listener's perspective without becoming a formal instructor or an approval gate.

`tests/instruments.rs` uses known sines, a chirp, harmonics, white noise, silence,
pauses, full-scale samples, antiphase stereo, out-of-band energy, integer WAV
extremes and mutated/truncated containers. The tests check measurements rather
than asserting that acoustic similarity establishes correct Russian.
