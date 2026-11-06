//! Shared streaming signal observations for recordings and selected regions.
use std::future::Future;

use crate::{AudioError, QualityFlag, RecordingQuality, checkpoint};

#[derive(Clone, Copy)]
pub(crate) enum QualityContext {
    Recording,
    Region,
}

pub(crate) fn dbfs(amplitude: f32) -> f32 {
    (20.0 * amplitude.max(1e-5).log10()).max(-100.0)
}

/// Input is a nonempty, validated interleaved view into original PCM. Quiet
/// blocks start at the selected interval boundary and retain partial tail blocks.
/// Nothing is resampled, trimmed, normalized, copied, or assigned a new identity.
pub(crate) async fn observe<F, Fut>(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    context: QualityContext,
    yield_now: &mut F,
) -> Result<(f32, RecordingQuality), AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    checkpoint(yield_now).await?;
    let channels = usize::from(channels);
    let frames = samples.len() / channels;
    if frames == 0 {
        return Err(AudioError::InvalidCrop);
    }
    let block_frames = sample_rate as usize / 50;
    let mut mono_squares = 0.0_f64;
    let mut original_squares = 0.0_f64;
    let mut mono_sum = 0.0_f64;
    let mut mono_peak = 0.0_f32;
    let mut peak = 0.0_f32;
    let mut clipped = 0_usize;
    let mut silent = 0_usize;
    let mut leading = 0_usize;
    let mut trailing = 0_usize;
    let mut still_leading = true;
    let mut pending_samples = 0;
    for block in samples.chunks(block_frames * channels) {
        let count = block.len() / channels;
        let mut block_squares = 0.0_f64;
        for frame in block.chunks_exact(channels) {
            let mut mono = 0.0_f32;
            for &sample in frame {
                mono += sample;
                peak = peak.max(sample.abs());
                clipped += usize::from(sample.abs() >= 0.999);
                original_squares += f64::from(sample).powi(2);
            }
            mono /= channels as f32;
            let square = f64::from(mono).powi(2);
            mono_squares += square;
            block_squares += square;
            mono_sum += f64::from(mono);
            mono_peak = mono_peak.max(mono.abs());
        }
        let quiet = ((block_squares / count as f64).sqrt() as f32) < 0.01;
        if quiet {
            silent += count;
            trailing += count;
            if still_leading {
                leading += count;
            }
        } else {
            still_leading = false;
            trailing = 0;
        }
        pending_samples += block.len();
        if pending_samples >= 65_536 {
            checkpoint(yield_now).await?;
            pending_samples = 0;
        }
    }
    checkpoint(yield_now).await?;
    let rms = (mono_squares / frames as f64).sqrt() as f32;
    let rms_dbfs = dbfs(rms);
    let clipping_fraction = clipped as f32 / samples.len() as f32;
    let silent_fraction = silent as f32 / frames as f32;
    let duration = frames as f64 / f64::from(sample_rate);
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
    if matches!(context, QualityContext::Recording) {
        if duration < 0.15 {
            flags.push(QualityFlag::TooShort);
        }
        if (mono_sum / frames as f64).abs() > 0.05 {
            flags.push(QualityFlag::DcOffset);
        }
    }
    let original_rms = (original_squares / samples.len() as f64).sqrt() as f32;
    if original_rms > 0.01 && rms < original_rms * 0.1 {
        flags.push(QualityFlag::DownmixCancellation);
    }
    Ok((
        rms,
        RecordingQuality {
            rms_dbfs,
            peak,
            clipping_fraction,
            silent_fraction,
            leading_silence_seconds: leading as f64 / f64::from(sample_rate),
            trailing_silence_seconds: trailing as f64 / f64::from(sample_rate),
            flags,
        },
    ))
}
