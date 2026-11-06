use std::f32::consts::PI;
use std::future::{Future, ready};

use rustfft::{FftPlanner, num_complex::Complex};
use serde::{Deserialize, Serialize};

use crate::{ANALYSIS_VERSION, AudioError, PcmAudio, checkpoint, run_ready};

const RATE: u32 = 16_000;
const FFT_SIZE: usize = 512;
const HOP: usize = 160;
const DB_FLOOR: f32 = -100.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioAnalysis {
    pub version: String,
    pub content_hash: String,
    /// Original duration, never duration after display alignment or trimming.
    pub duration_seconds: f64,
    pub sample_rate: u32,
    pub waveform: Vec<EnvelopeBin>,
    pub spectrogram: Spectrogram,
    pub pitch: Vec<PitchFrame>,
    pub quality: RecordingQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeBin {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub min: f32,
    pub max: f32,
    pub rms: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spectrogram {
    pub fft_size: usize,
    pub hop_size: usize,
    pub sample_rate: u32,
    pub frequency_bins: usize,
    /// Window centers on the original time axis. Last windows may be zero-padded.
    pub frame_times_seconds: Vec<f64>,
    /// Row-major time × frequency, including DC/Nyquist. Hann coherent-gain
    /// corrected one-sided bin power, dB relative to a full-scale sinusoid.
    /// This is a display spectrum, not a calibrated SPL or power spectral density.
    /// Values below -100 dB are floored. Bin k is k * rate / fft_size Hz.
    pub power_db: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PitchFrame {
    pub time_seconds: f64,
    /// Instrument voicing decision: a candidate passed level and periodicity
    /// thresholds. Periodic noise can also pass; this does not prove speech.
    pub voiced: bool,
    /// Candidate F0 in 50–400 Hz, or None for quiet/aperiodic/uncertain frames.
    pub hz: Option<f32>,
    /// 1 minus YIN cumulative normalized difference at the candidate lag.
    /// Periodicity evidence, not a probability of correct voicing/pronunciation.
    pub confidence: f32,
    pub rms: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityFlag {
    Silent,
    Quiet,
    Clipped,
    MostlySilent,
    TooShort,
    DcOffset,
    DownmixCancellation,
}

impl QualityFlag {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Silent => "No audible signal detected",
            Self::Quiet => "Low recording level",
            Self::Clipped => "Samples near full scale; possible clipping",
            Self::MostlySilent => "Most of the recording is below -40 dBFS",
            Self::TooShort => "Very short recording; limited pitch evidence",
            Self::DcOffset => "Large DC offset in the recording",
            Self::DownmixCancellation => "Channels may cancel when mixed to mono",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordingQuality {
    /// Mono RMS, floor -100 dBFS. Device gain changes this measure.
    pub rms_dbfs: f32,
    /// Peak over original channels, before filtering and downmix.
    pub peak: f32,
    /// Fraction of original channel samples at or above |0.999|.
    pub clipping_fraction: f32,
    /// Fraction of original time in 20 ms mono blocks below RMS 0.01.
    /// Quiet consonants can fall below this threshold; it is not VAD.
    pub silent_fraction: f32,
    pub leading_silence_seconds: f64,
    pub trailing_silence_seconds: f64,
    pub flags: Vec<QualityFlag>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    pub reference_duration_seconds: f64,
    pub attempt_duration_seconds: f64,
    pub duration_ratio: f64,
    pub rms_delta_db: f32,
    /// Attempt minus reference, across separately estimated voiced-frame medians.
    /// Speaker pitch range differs; this is not an intonation or stress score.
    pub median_pitch_difference_semitones: Option<f32>,
    pub warnings: Vec<String>,
}

/// Fixed, reproducible instrument settings (v1). Original samples are unchanged.
/// For interactive applications, call on a worker/background thread; maximum
/// duration is bounded but analysis is not a real-time callback operation.
pub fn analyze(audio: &PcmAudio) -> Result<AudioAnalysis, AudioError> {
    run_ready(analyze_cooperatively(audio, || ready(true)))
}

/// Analyze in bounded CPU chunks with caller-provided event-loop yielding and
/// cancellation. A callback result of false stops work without a partial result.
/// This produces the same measurements as `analyze`; no signal is segmented,
/// trimmed, or independently filtered at yield boundaries.
pub async fn analyze_cooperatively<F, Fut>(
    audio: &PcmAudio,
    mut yield_now: F,
) -> Result<AudioAnalysis, AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    checkpoint(&mut yield_now).await?;
    let meta = audio.metadata();
    let channels = meta.channels as usize;
    let mut mono = Vec::with_capacity(meta.frames);
    // Bound work by total channel samples, rather than making mono recordings
    // yield eight times as often as eight-channel recordings.
    for chunk in audio.samples().chunks(65_536 / channels * channels) {
        mono.extend(
            chunk
                .chunks_exact(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32),
        );
        checkpoint(&mut yield_now).await?;
    }
    let quality = quality(audio, &mono, &mut yield_now).await?;
    let waveform = envelope(&mono, meta.sample_rate, &mut yield_now).await?;
    let normalized = resample(&mono, meta.sample_rate, RATE, &mut yield_now).await?;
    let spectrogram = spectrum(&normalized, &mut yield_now).await?;
    let pitch_samples = resample(&normalized, RATE, 8_000, &mut yield_now).await?;
    let pitch = pitch_track(&pitch_samples, &mut yield_now).await?;
    Ok(AudioAnalysis {
        version: ANALYSIS_VERSION.to_owned(),
        content_hash: meta.content_hash.clone(),
        duration_seconds: meta.duration_seconds,
        sample_rate: RATE,
        waveform,
        spectrogram,
        pitch,
        quality,
    })
}

/// Descriptive A/B measurements on original durations. No time warping, acoustic
/// distance, linguistic correctness estimate, or overall score is computed.
pub fn compare(reference: &AudioAnalysis, attempt: &AudioAnalysis) -> Comparison {
    let median_pitch_difference_semitones = median_pitch(reference)
        .zip(median_pitch(attempt))
        .map(|(a, b)| 12.0 * (b / a).log2());
    let mut warnings = vec![
        "Duration includes pauses; compare matched phrase boundaries.".into(),
        "Loudness depends on the microphone, distance and gain.".into(),
        "Pitch differs across voices; these measurements do not judge pronunciation.".into(),
    ];
    if !reference.quality.flags.is_empty() || !attempt.quality.flags.is_empty() {
        warnings.push("Review recording quality before interpreting the comparison.".into());
    }
    if median_pitch_difference_semitones.is_none() {
        warnings.push("Insufficient confident voiced frames for a median pitch comparison.".into());
    }
    if reference.version != attempt.version {
        warnings.push("The recordings were analyzed with different instrument versions.".into());
    }
    Comparison {
        reference_duration_seconds: reference.duration_seconds,
        attempt_duration_seconds: attempt.duration_seconds,
        duration_ratio: attempt.duration_seconds / reference.duration_seconds,
        rms_delta_db: attempt.quality.rms_dbfs - reference.quality.rms_dbfs,
        median_pitch_difference_semitones,
        warnings,
    }
}

fn median_pitch(analysis: &AudioAnalysis) -> Option<f32> {
    let mut voiced: Vec<f32> = analysis
        .pitch
        .iter()
        .filter(|p| p.confidence >= 0.8)
        .filter_map(|p| p.hz)
        .filter(|hz| hz.is_finite() && *hz > 0.0)
        .collect();
    if voiced.len() < 3 {
        return None;
    }
    voiced.sort_unstable_by(f32::total_cmp);
    let mid = voiced.len() / 2;
    Some(if voiced.len() % 2 == 0 {
        (voiced[mid - 1] + voiced[mid]) / 2.0
    } else {
        voiced[mid]
    })
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt() as f32
}

fn db(amplitude: f32) -> f32 {
    (20.0 * amplitude.max(1e-5).log10()).max(DB_FLOOR)
}

async fn envelope<F, Fut>(
    mono: &[f32],
    rate: u32,
    yield_now: &mut F,
) -> Result<Vec<EnvelopeBin>, AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let size = mono.len().div_ceil(1200);
    let mut result = Vec::with_capacity(mono.len().div_ceil(size));
    for (i, chunk) in mono.chunks(size).enumerate() {
        result.push(EnvelopeBin {
            start_seconds: (i * size) as f64 / rate as f64,
            end_seconds: (i * size + chunk.len()) as f64 / rate as f64,
            min: chunk.iter().copied().fold(f32::INFINITY, f32::min),
            max: chunk.iter().copied().fold(f32::NEG_INFINITY, f32::max),
            rms: rms(chunk),
        });
        if i % 32 == 0 {
            checkpoint(yield_now).await?;
        }
    }
    Ok(result)
}

async fn quality<F, Fut>(
    audio: &PcmAudio,
    mono: &[f32],
    yield_now: &mut F,
) -> Result<RecordingQuality, AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let rate = audio.metadata().sample_rate as usize;
    let block_size = rate / 50;
    let mut blocks = Vec::with_capacity(mono.len().div_ceil(block_size));
    let mut mono_peak = 0.0_f32;
    let mut mono_squares = 0.0_f64;
    let mut mono_sum = 0.0_f64;
    for (i, block) in mono.chunks(block_size).enumerate() {
        let mut block_squares = 0.0;
        for &x in block {
            mono_peak = mono_peak.max(x.abs());
            let square = (x as f64).powi(2);
            block_squares += square;
            mono_squares += square;
            mono_sum += x as f64;
        }
        blocks.push((
            block.len(),
            ((block_squares / block.len() as f64).sqrt() as f32) < 0.01,
        ));
        if i % 32 == 0 {
            checkpoint(yield_now).await?;
        }
    }
    let silent = blocks
        .iter()
        .filter(|(_, quiet)| *quiet)
        .map(|(len, _)| len)
        .sum::<usize>();
    let leading = blocks
        .iter()
        .take_while(|(_, quiet)| *quiet)
        .map(|(len, _)| len)
        .sum::<usize>();
    let trailing = blocks
        .iter()
        .rev()
        .take_while(|(_, quiet)| *quiet)
        .map(|(len, _)| len)
        .sum::<usize>();
    let mut peak = 0.0_f32;
    let mut clipped = 0_usize;
    let mut original_squares = 0.0_f64;
    for chunk in audio.samples().chunks(65_536) {
        for &x in chunk {
            peak = peak.max(x.abs());
            clipped += usize::from(x.abs() >= 0.999);
            original_squares += (x as f64).powi(2);
        }
        checkpoint(yield_now).await?;
    }
    let clipping_fraction = clipped as f32 / audio.samples().len() as f32;
    let mono_rms = (mono_squares / mono.len() as f64).sqrt() as f32;
    let rms_dbfs = db(mono_rms);
    let silent_fraction = silent as f32 / mono.len() as f32;
    let mut flags = Vec::new();
    if mono_peak < 0.0001 {
        flags.push(QualityFlag::Silent);
    }
    if rms_dbfs < -35.0 {
        flags.push(QualityFlag::Quiet);
    }
    if clipping_fraction > 0.0 {
        flags.push(QualityFlag::Clipped);
    }
    if silent_fraction > 0.6 {
        flags.push(QualityFlag::MostlySilent);
    }
    if audio.metadata().duration_seconds < 0.15 {
        flags.push(QualityFlag::TooShort);
    }
    if (mono_sum / mono.len() as f64).abs() > 0.05 {
        flags.push(QualityFlag::DcOffset);
    }
    let original_rms = (original_squares / audio.samples().len() as f64).sqrt() as f32;
    if original_rms > 0.01 && mono_rms < original_rms * 0.1 {
        flags.push(QualityFlag::DownmixCancellation);
    }
    Ok(RecordingQuality {
        rms_dbfs,
        peak,
        clipping_fraction,
        silent_fraction,
        leading_silence_seconds: leading as f64 / rate as f64,
        trailing_silence_seconds: trailing as f64 / rate as f64,
        flags,
    })
}

/// Polyphase low-pass windowed-sinc resampling. 256 fractional phases; Hann
/// support expands when downsampling to preserve anti-alias rejection. Boundary
/// taps are renormalized, retaining DC without manufacturing edge silence.
async fn resample<F, Fut>(
    input: &[f32],
    source: u32,
    target: u32,
    yield_now: &mut F,
) -> Result<Vec<f32>, AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    if source == target {
        let mut result = Vec::with_capacity(input.len());
        for chunk in input.chunks(65_536) {
            result.extend_from_slice(chunk);
            checkpoint(yield_now).await?;
        }
        return Ok(result);
    }
    let ratio = source as f64 / target as f64;
    let radius = (16.0 * ratio.max(1.0)).ceil() as isize;
    let cutoff = 0.45 / ratio.max(1.0);
    let kernels: Vec<Vec<f32>> = (0..256)
        .map(|phase| {
            (-radius..=radius)
                .map(|tap| {
                    let x = tap as f64 - phase as f64 / 256.0;
                    let sinc = if x.abs() < 1e-10 {
                        2.0 * cutoff
                    } else {
                        (2.0 * std::f64::consts::PI * cutoff * x).sin() / (std::f64::consts::PI * x)
                    };
                    (sinc * (0.5 + 0.5 * (std::f64::consts::PI * x / (radius as f64 + 1.0)).cos()))
                        as f32
                })
                .collect()
        })
        .collect();
    let count = ((input.len() as f64 / ratio).round() as usize).max(1);
    let mut result = Vec::with_capacity(count);
    for i in 0..count {
        let position = i as f64 * ratio;
        let base = position.floor() as isize;
        let phase = ((position.fract() * 256.0).floor() as usize).min(255);
        let mut sum = 0.0;
        let mut weight = 0.0;
        for (j, k) in kernels[phase].iter().enumerate() {
            let index = base + j as isize - radius;
            if index >= 0 && (index as usize) < input.len() {
                sum += input[index as usize] * k;
                weight += k;
            }
        }
        result.push(if weight.abs() > 1e-6 {
            sum / weight
        } else {
            0.0
        });
        if i % 4096 == 0 {
            checkpoint(yield_now).await?;
        }
    }
    Ok(result)
}

async fn spectrum<F, Fut>(samples: &[f32], yield_now: &mut F) -> Result<Spectrogram, AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FFT_SIZE);
    let mut scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
    let mut buffer = vec![Complex::default(); FFT_SIZE];
    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|n| 0.5 - 0.5 * (2.0 * PI * n as f32 / FFT_SIZE as f32).cos())
        .collect();
    let window_sum: f32 = window.iter().sum();
    let bins = FFT_SIZE / 2 + 1;
    let count = samples.len().div_ceil(HOP);
    let mut power_db = Vec::with_capacity(count * bins);
    let mut frame_times_seconds = Vec::with_capacity(count);
    for start in (0..samples.len()).step_by(HOP) {
        for (i, item) in buffer.iter_mut().enumerate() {
            *item = Complex::new(
                samples.get(start + i).copied().unwrap_or(0.0) * window[i],
                0.0,
            );
        }
        fft.process_with_scratch(&mut buffer, &mut scratch);
        for (bin, value) in buffer[..bins].iter().enumerate() {
            let multiplier = if bin == 0 || bin == FFT_SIZE / 2 {
                1.0
            } else {
                2.0
            };
            power_db.push(db(value.norm() * multiplier / window_sum));
        }
        frame_times_seconds
            .push((start as f64 + FFT_SIZE.min(samples.len() - start) as f64 / 2.0) / RATE as f64);
        if start / HOP % 64 == 0 {
            checkpoint(yield_now).await?;
        }
    }
    Ok(Spectrogram {
        fft_size: FFT_SIZE,
        hop_size: HOP,
        sample_rate: RATE,
        frequency_bins: bins,
        frame_times_seconds,
        power_db,
    })
}

/// YIN-style cumulative normalized difference, fixed 40 ms difference window
/// plus 20 ms lag support at 8 kHz, 20 ms hops. A 0.15 trough threshold selects
/// the first sufficiently periodic candidate. No language-specific validation.
async fn pitch_track<F, Fut>(
    samples: &[f32],
    yield_now: &mut F,
) -> Result<Vec<PitchFrame>, AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    const WINDOW: usize = 320;
    const MIN_LAG: usize = 20;
    const MAX_LAG: usize = 160;
    const FRAME: usize = WINDOW + MAX_LAG;
    if samples.len() < FRAME {
        return Ok(Vec::new());
    }
    let mut difference = [0.0_f64; MAX_LAG + 1];
    let mut normalized = [1.0_f64; MAX_LAG + 1];
    let mut result = Vec::with_capacity(samples.len() / 160);
    for start in (0..=samples.len() - FRAME).step_by(160) {
        let frame = &samples[start..start + FRAME];
        let frame_rms = rms(frame);
        let mut point = PitchFrame {
            time_seconds: (start + FRAME / 2) as f64 / 8_000.0,
            voiced: false,
            hz: None,
            confidence: 0.0,
            rms: frame_rms,
        };
        if frame_rms >= 0.01 {
            let mut cumulative = 0.0;
            for lag in 1..=MAX_LAG {
                difference[lag] = (0..WINDOW)
                    .map(|j| {
                        let delta = frame[j] as f64 - frame[j + lag] as f64;
                        delta * delta
                    })
                    .sum();
                cumulative += difference[lag];
                normalized[lag] = if cumulative > 1e-15 {
                    difference[lag] * lag as f64 / cumulative
                } else {
                    1.0
                };
            }
            let mut lag = MIN_LAG;
            let mut candidate = None;
            while lag <= MAX_LAG {
                if normalized[lag] < 0.15 {
                    while lag < MAX_LAG && normalized[lag + 1] < normalized[lag] {
                        lag += 1;
                    }
                    candidate = Some(lag);
                    break;
                }
                lag += 1;
            }
            // Preserve weak periodicity evidence while withholding a pitch estimate.
            let best = normalized[MIN_LAG..].iter().copied().fold(1.0, f64::min);
            point.confidence = (1.0 - best).clamp(0.0, 1.0) as f32;
            if let Some(lag) = candidate {
                let offset = if lag > MIN_LAG && lag < MAX_LAG {
                    let denom = normalized[lag - 1] - 2.0 * normalized[lag] + normalized[lag + 1];
                    if denom.abs() > 1e-12 {
                        (0.5 * (normalized[lag - 1] - normalized[lag + 1]) / denom).clamp(-0.5, 0.5)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                point.hz = Some((8_000.0 / (lag as f64 + offset)) as f32);
                point.voiced = true;
                point.confidence = (1.0 - normalized[lag]).clamp(0.0, 1.0) as f32;
            }
        }
        result.push(point);
        if start / 160 % 32 == 0 {
            checkpoint(yield_now).await?;
        }
    }
    Ok(result)
}
