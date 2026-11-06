//! Portable, bounded audio instruments. These measurements describe a recording;
//! they do not assess Russian pronunciation, accent, proficiency, or intelligibility.
//!
//! Original interleaved PCM is retained. Analysis is versioned and resamples to
//! 16 kHz using a low-pass windowed-sinc filter. No network or device APIs are used.

mod analysis;
mod wav;

pub use analysis::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::future::{Future, ready};
use std::task::{Context, Poll, Waker};
use thiserror::Error;

pub const MAX_DURATION_SECONDS: usize = 60;
pub const MAX_SAMPLES: usize = 16_000_000;
pub const MAX_WAV_BYTES: usize = 64 * 1024 * 1024;
pub const ANALYSIS_VERSION: &str = "rufren-instruments-v1";

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AudioError {
    #[error("sample rate must be between 8000 and 192000 Hz")]
    SampleRate,
    #[error("audio must contain between 1 and 8 channels")]
    Channels,
    #[error("audio must contain complete, nonempty interleaved frames")]
    EmptyOrIncomplete,
    #[error("audio exceeds the 60-second, 16-million-sample or 64-MiB file limit")]
    TooLarge,
    #[error("PCM must contain only finite samples between -1 and 1")]
    InvalidSample,
    #[error("invalid or unsupported WAV: {0}")]
    Wav(&'static str),
    #[error("crop must be a nonempty frame range inside the recording")]
    InvalidCrop,
    #[error("audio analysis was cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMetadata {
    /// SHA-256 of version tag, little-endian rate/channels and canonical f32 PCM.
    /// A PCM identity, not the hash of the imported container bytes.
    pub content_hash: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: usize,
    pub duration_seconds: f64,
}

/// Validated normalized original PCM. No Deserialize implementation: all input
/// passes through constructors, including WAV and platform adapters.
#[derive(Debug, Clone)]
pub struct PcmAudio {
    samples: Vec<f32>,
    metadata: AudioMetadata,
}

impl PcmAudio {
    pub fn from_mono(sample_rate: u32, samples: Vec<f32>) -> Result<Self, AudioError> {
        Self::from_interleaved(sample_rate, 1, samples)
    }

    pub fn from_interleaved(
        sample_rate: u32,
        channels: u16,
        samples: Vec<f32>,
    ) -> Result<Self, AudioError> {
        run_ready(Self::from_interleaved_cooperatively(
            sample_rate,
            channels,
            samples,
            || ready(true),
        ))
    }

    /// Validate and hash in bounded chunks, giving the caller an opportunity to
    /// yield to its UI event loop. Return false from the callback to cancel.
    /// Merely awaiting an immediately-ready callback does not yield to a browser;
    /// the supplied callback must actually schedule an event-loop task.
    pub async fn from_interleaved_cooperatively<F, Fut>(
        sample_rate: u32,
        channels: u16,
        mut samples: Vec<f32>,
        mut yield_now: F,
    ) -> Result<Self, AudioError>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = bool>,
    {
        validate_shape(sample_rate, channels, samples.len())?;
        checkpoint(&mut yield_now).await?;
        let mut hash = Sha256::new();
        hash.update(b"rufren-pcm-f32-v1\0");
        hash.update(sample_rate.to_le_bytes());
        hash.update(channels.to_le_bytes());
        let mut encoded = Vec::with_capacity(65_536 * 4);
        for chunk in samples.chunks_mut(65_536) {
            encoded.clear();
            for x in chunk {
                if !x.is_finite() || x.abs() > 1.0 {
                    return Err(AudioError::InvalidSample);
                }
                // Negative zero carries no audio information; canonicalize identity.
                if *x == 0.0 {
                    *x = 0.0;
                }
                encoded.extend_from_slice(&x.to_le_bytes());
            }
            hash.update(&encoded);
            checkpoint(&mut yield_now).await?;
        }
        let frames = samples.len() / channels as usize;
        let metadata = AudioMetadata {
            content_hash: format!("{:x}", hash.finalize()),
            sample_rate,
            channels,
            frames,
            duration_seconds: frames as f64 / sample_rate as f64,
        };
        Ok(Self { samples, metadata })
    }

    pub fn metadata(&self) -> &AudioMetadata {
        &self.metadata
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Crop in original sample frames, preserving rate and channels. The original
    /// recording is unchanged; callers should retain its hash with the crop range.
    pub fn crop(&self, start_frame: usize, end_frame: usize) -> Result<Self, AudioError> {
        if start_frame >= end_frame || end_frame > self.metadata.frames {
            return Err(AudioError::InvalidCrop);
        }
        let channels = self.metadata.channels as usize;
        Self::from_interleaved(
            self.metadata.sample_rate,
            self.metadata.channels,
            self.samples[start_frame * channels..end_frame * channels].to_vec(),
        )
    }

    /// Decode RIFF/WAVE PCM 8/16/24/32-bit or IEEE float32. Compressed,
    /// extensible, RF64, and duplicate format/data chunks are explicitly rejected.
    pub fn from_wav(bytes: &[u8]) -> Result<Self, AudioError> {
        wav::decode(bytes)
    }

    /// Lossless IEEE float32 WAV with original rate, channels, and samples.
    pub fn to_wav(&self) -> Vec<u8> {
        wav::encode(self)
    }
}

pub(crate) async fn checkpoint<F, Fut>(yield_now: &mut F) -> Result<(), AudioError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    if yield_now().await {
        Ok(())
    } else {
        Err(AudioError::Cancelled)
    }
}

/// Synchronous wrappers use only `ready(true)` callbacks, so their shared async
/// implementation completes on its first poll. No executor/runtime is needed.
pub(crate) fn run_ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => unreachable!("internal synchronous yield callback is immediately ready"),
    }
}

fn validate_shape(sample_rate: u32, channels: u16, samples: usize) -> Result<(), AudioError> {
    if !(8_000..=192_000).contains(&sample_rate) {
        return Err(AudioError::SampleRate);
    }
    if !(1..=8).contains(&channels) {
        return Err(AudioError::Channels);
    }
    if samples == 0 || samples % channels as usize != 0 {
        return Err(AudioError::EmptyOrIncomplete);
    }
    if samples > MAX_SAMPLES
        || samples / channels as usize > sample_rate as usize * MAX_DURATION_SECONDS
    {
        return Err(AudioError::TooLarge);
    }
    Ok(())
}
