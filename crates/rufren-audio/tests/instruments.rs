use std::f32::consts::TAU;

use rufren_audio::{AudioError, PcmAudio, QualityFlag, analyze, compare};

fn tone(rate: u32, seconds: f32, frequency: f32, amplitude: f32) -> Vec<f32> {
    (0..(rate as f32 * seconds) as usize)
        .map(|n| amplitude * (TAU * frequency * n as f32 / rate as f32).sin())
        .collect()
}

fn audio(rate: u32, samples: Vec<f32>) -> PcmAudio {
    PcmAudio::from_mono(rate, samples).unwrap()
}

fn median(values: impl Iterator<Item = f32>) -> f32 {
    let mut values: Vec<_> = values.collect();
    assert!(!values.is_empty());
    values.sort_unstable_by(f32::total_cmp);
    values[values.len() / 2]
}

#[test]
fn calibrated_sine_exposes_frequency_amplitude_and_pitch() {
    // Exactly bin 7 at 16 kHz / 512, independently known frequency/amplitude.
    let result = analyze(&audio(16_000, tone(16_000, 0.5, 218.75, 0.5))).unwrap();
    assert_eq!(result.sample_rate, 16_000);
    assert_eq!(result.duration_seconds, 0.5);
    assert!((result.quality.rms_dbfs - -9.03).abs() < 0.1);
    let spectrum = &result.spectrogram;
    let row = &spectrum.power_db[spectrum.frequency_bins * 10..spectrum.frequency_bins * 11];
    let peak = row
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .unwrap();
    assert_eq!(peak.0, 7);
    assert!((*peak.1 - -6.02).abs() < 0.1);
    let pitch = median(result.pitch.iter().filter_map(|p| p.hz));
    assert!((pitch - 218.75).abs() < 1.0, "pitch {pitch}");
    assert!(result.pitch.iter().all(|p| p.confidence > 0.95));
    assert!(
        result
            .waveform
            .iter()
            .all(|b| b.min >= -0.501 && b.max <= 0.501)
    );
    assert_eq!(
        spectrum.power_db.len(),
        spectrum.frequency_bins * spectrum.frame_times_seconds.len()
    );
    assert!(spectrum.power_db.iter().all(|v| v.is_finite()));
}

#[test]
fn chirp_pitch_tracks_time_without_warping_duration() {
    let rate = 16_000;
    let samples: Vec<_> = (0..rate)
        .map(|n| {
            let t = n as f32 / rate as f32;
            0.5 * (TAU * (100.0 * t + 60.0 * t * t)).sin()
        })
        .collect();
    let result = analyze(&audio(rate, samples)).unwrap();
    assert_eq!(result.duration_seconds, 1.0);
    for p in &result.pitch {
        let expected = 100.0 + 120.0 * p.time_seconds as f32;
        let found = p.hz.unwrap();
        assert!(
            (found - expected).abs() < 4.0,
            "time={} found={found} expected={expected}",
            p.time_seconds
        );
    }
}

#[test]
fn silence_clipping_dc_and_stereo_cancellation_are_distinguished() {
    let silence = analyze(&audio(8_000, vec![0.0; 800])).unwrap();
    assert!(silence.quality.flags.contains(&QualityFlag::Silent));
    assert_eq!(silence.quality.silent_fraction, 1.0);
    assert_eq!(silence.quality.leading_silence_seconds, 0.1);
    assert!(
        silence
            .pitch
            .iter()
            .all(|p| p.hz.is_none() && p.confidence == 0.0)
    );
    assert!(silence.spectrogram.power_db.iter().all(|v| *v == -100.0));
    let clipped = analyze(&audio(16_000, vec![1.0; 3200])).unwrap();
    assert!(clipped.quality.flags.contains(&QualityFlag::Clipped));
    assert!(clipped.quality.flags.contains(&QualityFlag::DcOffset));
    assert_eq!(clipped.quality.clipping_fraction, 1.0);
    assert!(clipped.pitch.iter().all(|p| p.hz.is_none()));
    let stereo: Vec<_> = tone(16_000, 0.2, 200.0, 0.5)
        .into_iter()
        .flat_map(|x| [x, -x])
        .collect();
    let cancelled = analyze(&PcmAudio::from_interleaved(16_000, 2, stereo).unwrap()).unwrap();
    assert!(
        cancelled
            .quality
            .flags
            .contains(&QualityFlag::DownmixCancellation)
    );
    assert!(cancelled.quality.flags.contains(&QualityFlag::Silent));
    assert!(cancelled.quality.peak > 0.49);
}

#[test]
fn pause_duration_and_envelope_retain_original_time_axis() {
    let mut samples = vec![0.0; 3200];
    samples.extend(tone(16_000, 0.4, 200.0, 0.5));
    samples.extend(vec![0.0; 6400]);
    let pcm = audio(16_000, samples);
    let result = analyze(&pcm).unwrap();
    assert_eq!(result.duration_seconds, 1.0);
    assert!((result.quality.leading_silence_seconds - 0.2).abs() < 1e-6);
    assert!((result.quality.trailing_silence_seconds - 0.4).abs() < 1e-6);
    assert!((result.quality.silent_fraction - 0.6).abs() < 1e-6);
    assert_eq!(result.waveform.first().unwrap().start_seconds, 0.0);
    assert_eq!(result.waveform.last().unwrap().end_seconds, 1.0);
    let peak_bins: Vec<_> = result.waveform.iter().filter(|b| b.rms > 0.1).collect();
    assert!(peak_bins.first().unwrap().start_seconds >= 0.199);
    assert!(peak_bins.last().unwrap().end_seconds <= 0.601);
    let crop = pcm.crop(3200, 9600).unwrap();
    assert_eq!(crop.metadata().duration_seconds, 0.4);
    assert_ne!(crop.metadata().content_hash, pcm.metadata().content_hash);
    assert_eq!(pcm.metadata().duration_seconds, 1.0);
}

#[test]
fn anti_aliasing_rejects_out_of_band_input_and_preserves_speech_band() {
    let low = analyze(&audio(48_000, tone(48_000, 0.3, 1000.0, 0.5))).unwrap();
    let high = analyze(&audio(48_000, tone(48_000, 0.3, 11_000.0, 0.5))).unwrap();
    let row = |a: &rufren_audio::AudioAnalysis| {
        let bins = a.spectrogram.frequency_bins;
        a.spectrogram.power_db[10 * bins..11 * bins].to_vec()
    };
    let low_row = row(&low);
    let low_max = low_row.iter().copied().fold(-100.0, f32::max);
    assert!((low_max - -6.02).abs() < 0.2);
    assert_eq!(
        low_row
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .unwrap()
            .0,
        32
    );
    let high_max = row(&high).iter().copied().fold(-100.0, f32::max);
    assert!(high_max < -50.0, "aliased high-frequency peak {high_max}");
    assert_eq!(low.duration_seconds, high.duration_seconds);
}

#[test]
fn noise_does_not_get_a_confident_pitch_and_harmonic_voice_does() {
    let mut seed = 923_411_u32;
    let noise: Vec<_> = (0..8000)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed as f64 / u32::MAX as f64 * 0.8 - 0.4) as f32
        })
        .collect();
    let noise = analyze(&audio(16_000, noise)).unwrap();
    assert!(noise.pitch.iter().all(|p| p.hz.is_none()));
    let voiced: Vec<_> = tone(16_000, 0.5, 130.0, 0.3)
        .iter()
        .zip(tone(16_000, 0.5, 260.0, 0.4))
        .map(|(a, b)| a + b)
        .collect();
    let voiced = analyze(&audio(16_000, voiced)).unwrap();
    assert!((median(voiced.pitch.iter().filter_map(|p| p.hz)) - 130.0).abs() < 1.0);
}

#[test]
fn comparison_is_descriptive_and_preserves_pause_and_gain_differences() {
    let reference = analyze(&audio(16_000, tone(16_000, 0.3, 150.0, 0.5))).unwrap();
    let attempt = analyze(&audio(16_000, tone(16_000, 0.6, 300.0, 0.25))).unwrap();
    let diff = compare(&reference, &attempt);
    assert!((diff.duration_ratio - 2.0).abs() < 1e-6);
    assert!((diff.rms_delta_db - -6.02).abs() < 0.1);
    assert!((diff.median_pitch_difference_semitones.unwrap() - 12.0).abs() < 0.1);
    let silence = analyze(&audio(16_000, vec![0.0; 3200])).unwrap();
    assert!(
        compare(&reference, &silence)
            .median_pitch_difference_semitones
            .is_none()
    );
}

#[test]
fn pcm_wav_roundtrip_hash_and_stereo_channels_are_exact() {
    let pcm = PcmAudio::from_interleaved(44_100, 2, vec![0.0, -0.0, 0.1, -0.7, 1.0, -1.0]).unwrap();
    let restored = PcmAudio::from_wav(&pcm.to_wav()).unwrap();
    assert_eq!(restored.metadata(), pcm.metadata());
    assert_eq!(restored.samples(), pcm.samples());
    let canonical =
        PcmAudio::from_interleaved(44_100, 2, vec![-0.0, 0.0, 0.1, -0.7, 1.0, -1.0]).unwrap();
    assert_eq!(
        canonical.metadata().content_hash,
        pcm.metadata().content_hash
    );
    assert_ne!(
        audio(16_000, vec![0.0, 0.1]).metadata().content_hash,
        audio(8_000, vec![0.0, 0.1]).metadata().content_hash
    );
}

#[test]
fn hostile_pcm_lengths_rates_values_and_crops_are_rejected() {
    assert_eq!(
        PcmAudio::from_mono(0, vec![0.0]).unwrap_err(),
        AudioError::SampleRate
    );
    assert_eq!(
        PcmAudio::from_mono(u32::MAX, vec![0.0]).unwrap_err(),
        AudioError::SampleRate
    );
    assert_eq!(
        PcmAudio::from_interleaved(16_000, u16::MAX, vec![0.0]).unwrap_err(),
        AudioError::Channels
    );
    assert_eq!(
        PcmAudio::from_interleaved(16_000, 2, vec![0.0]).unwrap_err(),
        AudioError::EmptyOrIncomplete
    );
    assert_eq!(
        PcmAudio::from_mono(16_000, vec![]).unwrap_err(),
        AudioError::EmptyOrIncomplete
    );
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.001, -1.001] {
        assert_eq!(
            PcmAudio::from_mono(16_000, vec![value]).unwrap_err(),
            AudioError::InvalidSample
        );
    }
    assert_eq!(
        PcmAudio::from_mono(8_000, vec![0.0; 480_001]).unwrap_err(),
        AudioError::TooLarge
    );
    let one = audio(192_000, vec![0.1]);
    assert!(one.crop(0, 0).is_err());
    assert!(one.crop(0, usize::MAX).is_err());
    assert!(one.crop(1, 0).is_err());
    // Extremely short valid audio remains analyzable without zero-size windows.
    let tiny = analyze(&one).unwrap();
    assert!(tiny.quality.flags.contains(&QualityFlag::TooShort));
    assert_eq!(tiny.waveform.len(), 1);
    assert!(tiny.pitch.is_empty());
}

fn integer_wav(bits: u16, payload: &[u8]) -> Vec<u8> {
    let width = bits as u32 / 8;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&(36 + payload.len() as u32 + payload.len() as u32 % 2).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&16_000_u32.to_le_bytes());
    bytes.extend_from_slice(&(16_000_u32 * width).to_le_bytes());
    bytes.extend_from_slice(&(width as u16).to_le_bytes());
    bytes.extend_from_slice(&bits.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    if payload.len() % 2 != 0 {
        bytes.push(0);
    }
    bytes
}

#[test]
fn integer_wav_depths_decode_signedness_and_extremes() {
    for (bits, payload) in [
        (8, vec![0, 128, 255]),
        (16, vec![0, 128, 0, 0, 255, 127]),
        (24, vec![0, 0, 128, 0, 0, 0, 255, 255, 127]),
        (32, vec![0, 0, 0, 128, 0, 0, 0, 0, 255, 255, 255, 127]),
    ] {
        let decoded = PcmAudio::from_wav(&integer_wav(bits, &payload)).unwrap();
        assert_eq!(decoded.samples()[0], -1.0);
        assert_eq!(decoded.samples()[1], 0.0);
        assert!(decoded.samples()[2] > 0.99);
        assert!(decoded.samples()[2] <= 1.0);
    }
}

#[test]
fn malformed_wavs_are_rejected_without_panics_or_payload_allocation() {
    let valid = audio(16_000, vec![0.0, 0.5, -0.5]).to_wav();
    // Every proper prefix is invalid; RIFF length and chunk bounds are strict.
    for len in 0..valid.len() {
        assert!(PcmAudio::from_wav(&valid[..len]).is_err(), "prefix {len}");
    }
    for (offset, replacement) in [(20, 99_u32), (24, u32::MAX), (28, 0), (54, u32::MAX)] {
        let mut corrupt = valid.clone();
        corrupt[offset..offset + 4].copy_from_slice(&replacement.to_le_bytes());
        assert!(PcmAudio::from_wav(&corrupt).is_err(), "offset {offset}");
    }
    let mut nan = valid.clone();
    nan[58..62].copy_from_slice(&f32::NAN.to_le_bytes());
    assert_eq!(
        PcmAudio::from_wav(&nan).unwrap_err(),
        AudioError::InvalidSample
    );
    // Mutated header bytes cover parser state transitions without relying on a
    // fuzzing dependency; valid mutations may succeed, but none may panic.
    for offset in 0..58 {
        for replacement in [0, 1, 127, 255] {
            let mut mutated = valid.clone();
            mutated[offset] = replacement;
            let _ = PcmAudio::from_wav(&mutated);
        }
    }
}
