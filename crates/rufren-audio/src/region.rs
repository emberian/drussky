//! Measurements on explicit original-frame regions, with original identity and
//! timing retained. Region boundaries are selections, never phoneme labels.
use std::future::{Future, ready};

use serde::{Deserialize, Serialize};

use crate::{
    AudioError, AudioMetadata, MAX_DURATION_SECONDS, MAX_SAMPLES, PcmAudio, RecordingQuality,
    quality, run_ready,
};

pub const REGION_METRICS_VERSION: &str = "rufren-region-metrics-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionSpan {
    /// Half-open ORIGINAL sample-frame coordinates, before analysis resampling.
    pub start_frame: u64,
    pub end_frame: u64,
}
impl RegionSpan {
    pub fn validate_for(&self, audio: &PcmAudio) -> Result<(), AudioError> {
        self.validate_bounds(audio.metadata().frames as u64)
    }

    /// Convenience for UI time controls. Rejects nonfinite, reversed and
    /// out-of-range times, then rounds each boundary to its nearest original
    /// sample frame (positive ties upward). The returned frame span is exact;
    /// playback must use these frames/rate instead of the unrounded input times.
    pub fn from_seconds(audio: &PcmAudio, start: f64, end: f64) -> Result<Self, AudioError> {
        let source = audio.metadata();
        if !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || start >= end
            || end > source.duration_seconds
        {
            return Err(AudioError::InvalidCrop);
        }
        let span = Self {
            start_frame: (start * f64::from(source.sample_rate)).round() as u64,
            end_frame: (end * f64::from(source.sample_rate)).round() as u64,
        };
        span.validate_for(audio)?;
        Ok(span)
    }
    fn validate_bounds(&self, frames: u64) -> Result<(), AudioError> {
        if self.start_frame >= self.end_frame || self.end_frame > frames {
            Err(AudioError::InvalidCrop)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionMetrics {
    pub version: String,
    /// Whole original PCM identity and format; selecting a region does not
    /// replace this with a crop hash. Encoded-asset hashes remain caller bindings.
    pub source: AudioMetadata,
    pub span: RegionSpan,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub duration_seconds: f64,
    /// RMS of the arithmetic channel mean, before any resampling/normalization.
    pub rms: f32,
    /// Selected-region observations. Quiet blocks are 20 ms, beginning at this
    /// region's boundary; they are not VAD. Peak/clipping use original channels.
    /// Recording-only TooShort/DC-offset warnings are omitted: a deliberately
    /// short region may contain only part of a waveform cycle.
    pub quality: RecordingQuality,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionComparison {
    pub region_a: RegionMetrics,
    pub region_b: RegionMetrics,
    /// B/A of exact selected-region durations, never whole-take durations.
    pub duration_ratio: f64,
    /// B minus A, in seconds.
    pub duration_delta_seconds: f64,
    /// B minus A of displayed RMS levels. A floor warning means this difference
    /// is not resolved; the -100 dBFS display floor is not a measured noise level.
    pub rms_delta_db: f32,
    pub warnings: Vec<String>,
}

pub fn measure_region(audio: &PcmAudio, span: RegionSpan) -> Result<RegionMetrics, AudioError> {
    run_ready(measure_region_cooperatively(audio, span, || ready(true)))
}

/// One streaming scan of the selected original samples. No new FFT, pitch
/// estimation, resampling, crop allocation or crop identity is introduced.
pub async fn measure_region_cooperatively<F, Fut>(
    audio: &PcmAudio,
    span: RegionSpan,
    mut yield_now: F,
) -> Result<RegionMetrics, AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    span.validate_for(audio)?;
    let source = audio.metadata();
    let channels = usize::from(source.channels);
    let start = usize::try_from(span.start_frame).map_err(|_| AudioError::InvalidCrop)? * channels;
    let end = usize::try_from(span.end_frame).map_err(|_| AudioError::InvalidCrop)? * channels;
    let (rms, quality) = quality::observe(
        &audio.samples()[start..end],
        source.sample_rate,
        source.channels,
        quality::QualityContext::Region,
        &mut yield_now,
    )
    .await?;
    Ok(RegionMetrics {
        version: REGION_METRICS_VERSION.into(),
        source: source.clone(),
        span,
        start_seconds: span.start_frame as f64 / f64::from(source.sample_rate),
        end_seconds: span.end_frame as f64 / f64::from(source.sample_rate),
        duration_seconds: (span.end_frame - span.start_frame) as f64
            / f64::from(source.sample_rate),
        rms,
        quality,
    })
}

impl RegionMetrics {
    /// Also used for deserialized comparison inputs, so forged NaN/zero duration
    /// and mismatched span/time/format metadata cannot produce invalid ratios.
    pub fn validate(&self) -> Result<(), AudioError> {
        let s = &self.source;
        let q = &self.quality;
        let valid_source = (8_000..=192_000).contains(&s.sample_rate)
            && (1..=8).contains(&s.channels)
            && s.frames > 0
            && s.frames <= s.sample_rate as usize * MAX_DURATION_SECONDS
            && s.frames
                .checked_mul(usize::from(s.channels))
                .is_some_and(|n| n <= MAX_SAMPLES)
            && s.content_hash.len() == 64
            && s.content_hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            && s.duration_seconds == s.frames as f64 / f64::from(s.sample_rate);
        if !valid_source
            || self.version != REGION_METRICS_VERSION
            || self.span.validate_bounds(s.frames as u64).is_err()
        {
            return Err(AudioError::InvalidRegionMetrics);
        }
        let duration =
            (self.span.end_frame - self.span.start_frame) as f64 / f64::from(s.sample_rate);
        if self.start_seconds != self.span.start_frame as f64 / f64::from(s.sample_rate)
            || self.end_seconds != self.span.end_frame as f64 / f64::from(s.sample_rate)
            || self.duration_seconds != duration
            || !self.rms.is_finite()
            || !(0.0..=1.0).contains(&self.rms)
            // f32 log10 may round slightly differently across native/WASM
            // math libraries; retain a tiny unit tolerance without accepting NaN.
            || !q.rms_dbfs.is_finite()
            || (q.rms_dbfs - quality::dbfs(self.rms)).abs() > 0.0001
            || !q.peak.is_finite()
            || !(0.0..=1.0).contains(&q.peak)
            || !q.clipping_fraction.is_finite()
            || !(0.0..=1.0).contains(&q.clipping_fraction)
            || !q.silent_fraction.is_finite()
            || !(0.0..=1.0).contains(&q.silent_fraction)
            || !q.leading_silence_seconds.is_finite()
            || !(0.0..=duration).contains(&q.leading_silence_seconds)
            || !q.trailing_silence_seconds.is_finite()
            || !(0.0..=duration).contains(&q.trailing_silence_seconds)
            || q.flags.len() > 7
        {
            return Err(AudioError::InvalidRegionMetrics);
        }
        Ok(())
    }
}

pub fn compare_regions(
    a: &RegionMetrics,
    b: &RegionMetrics,
) -> Result<RegionComparison, AudioError> {
    a.validate()?;
    b.validate()?;
    let mut warnings=vec![
        "Boundaries are selected regions, not phoneme labels. Duration depends on context and phrase position; there is no universal stress-duration ratio.".into(),
        "Level depends on voice, microphone, distance and gain. Louder does not establish stress or pronunciation correctness.".into(),
    ];
    if a.quality.rms_dbfs <= -100.0 || b.quality.rms_dbfs <= -100.0 {
        warnings.push("At least one region reaches the -100 dBFS display floor; its level difference is not resolved.".into());
    }
    if !a.quality.flags.is_empty() || !b.quality.flags.is_empty() {
        warnings.push("The selected signal has quality observations; replay to interpret quiet, clipping or channel-cancellation flags.".into());
    }
    Ok(RegionComparison {
        region_a: a.clone(),
        region_b: b.clone(),
        duration_ratio: b.duration_seconds / a.duration_seconds,
        duration_delta_seconds: b.duration_seconds - a.duration_seconds,
        rms_delta_db: b.quality.rms_dbfs - a.quality.rms_dbfs,
        warnings,
    })
}
