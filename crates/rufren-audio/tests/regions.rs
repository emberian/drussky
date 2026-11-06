use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use rufren_audio::{
    AudioError, PcmAudio, QualityFlag, RegionSpan, analyze, compare_regions, measure_region,
    measure_region_cooperatively,
};

fn mono(rate: u32, samples: Vec<f32>) -> PcmAudio {
    PcmAudio::from_mono(rate, samples).unwrap()
}
fn span(start_frame: u64, end_frame: u64) -> RegionSpan {
    RegionSpan {
        start_frame,
        end_frame,
    }
}

#[test]
fn region_original_frame_boundaries_and_identity_hold_at_all_supported_rates() {
    for rate in [8_000, 16_000, 24_000, 44_100, 48_000, 192_000] {
        let audio = mono(rate, vec![0.25; 128]);
        let original = audio.metadata().clone();
        let selected = span(13, 97);
        let measured = measure_region(&audio, selected).unwrap();
        measured.validate().unwrap();
        assert_eq!(measured.source, original);
        assert_eq!(audio.metadata(), &original);
        assert_eq!(measured.span, selected);
        assert_eq!(measured.start_seconds, 13.0 / f64::from(rate));
        assert_eq!(measured.end_seconds, 97.0 / f64::from(rate));
        assert_eq!(measured.duration_seconds, 84.0 / f64::from(rate));
        assert_eq!(measured.rms, 0.25);
        assert_eq!(
            RegionSpan::from_seconds(&audio, 13.0 / f64::from(rate), 97.0 / f64::from(rate))
                .unwrap(),
            selected
        );
        assert_eq!(
            measure_region(&audio, span(127, 128))
                .unwrap()
                .duration_seconds,
            1.0 / f64::from(rate)
        );
        assert!(!measured.quality.flags.contains(&QualityFlag::TooShort));
        assert!(!measured.quality.flags.contains(&QualityFlag::DcOffset));
    }
}

#[test]
fn region_rounding_validates_times_and_never_silently_clamps() {
    let audio = mono(8_000, vec![0.2; 100]);
    assert_eq!(
        RegionSpan::from_seconds(&audio, 0.5 / 8_000.0, 4.5 / 8_000.0).unwrap(),
        span(1, 5)
    );
    assert!(RegionSpan::from_seconds(&audio, 1.1 / 8_000.0, 1.2 / 8_000.0).is_err());
    for (start, end) in [
        (f64::NAN, 0.01),
        (0.0, f64::INFINITY),
        (-0.001, 0.01),
        (0.01, 0.01),
        (0.01, 0.0),
        (0.0, 0.012501),
    ] {
        assert_eq!(
            RegionSpan::from_seconds(&audio, start, end).unwrap_err(),
            AudioError::InvalidCrop
        );
    }
    for invalid in [
        span(0, 0),
        span(4, 3),
        span(100, 101),
        span(0, 101),
        span(0, u64::MAX),
        span(u64::MAX, u64::MAX),
    ] {
        assert!(invalid.validate_for(&audio).is_err());
        assert_eq!(
            measure_region(&audio, invalid).unwrap_err(),
            AudioError::InvalidCrop
        );
    }
}

#[test]
fn region_stereo_rms_averages_channels_and_quality_stays_inside_half_open_selection() {
    let audio = PcmAudio::from_interleaved(
        8_000,
        2,
        vec![
            1.0, 1.0, 0.0, 0.0, 0.5, -0.25, -0.5, 0.25, 0.0, 0.0, -1.0, -1.0,
        ],
    )
    .unwrap();
    let selected = measure_region(&audio, span(2, 4)).unwrap();
    assert_eq!(selected.rms, 0.125);
    assert_eq!(selected.quality.peak, 0.5);
    assert_eq!(selected.quality.clipping_fraction, 0.0);
    assert!(!selected.quality.flags.contains(&QualityFlag::Clipped));
    let full = measure_region(&audio, span(0, 6)).unwrap();
    assert!(full.quality.flags.contains(&QualityFlag::Clipped));
    assert_eq!(full.quality.clipping_fraction, 4.0 / 12.0);
    let near_scale = PcmAudio::from_interleaved(8_000, 2, vec![1.0, 0.0, -1.0, 0.0]).unwrap();
    let measured = measure_region(&near_scale, span(0, 2)).unwrap();
    assert_eq!(measured.rms, 0.5);
    assert_eq!(measured.quality.clipping_fraction, 0.5);
    let cancelling = PcmAudio::from_interleaved(8_000, 2, vec![0.5, -0.5, -0.5, 0.5]).unwrap();
    let measured = measure_region(&cancelling, span(0, 2)).unwrap();
    assert_eq!(measured.rms, 0.0);
    assert_eq!(measured.quality.peak, 0.5);
    assert_eq!(measured.quality.rms_dbfs, -100.0);
    assert!(
        measured
            .quality
            .flags
            .contains(&QualityFlag::DownmixCancellation)
    );
}

#[test]
fn region_quiet_blocks_are_local_and_weight_partial_tail_by_actual_frames() {
    let mut samples = vec![1.0; 3];
    samples.extend(vec![0.0; 160]);
    samples.extend(vec![0.5; 160]);
    samples.extend(vec![0.0; 80]);
    samples.push(1.0);
    let audio = mono(8_000, samples);
    let measured = measure_region(&audio, span(3, 403)).unwrap();
    assert_eq!(measured.duration_seconds, 0.05);
    assert_eq!(measured.quality.leading_silence_seconds, 0.02);
    assert_eq!(measured.quality.trailing_silence_seconds, 0.01);
    assert_eq!(measured.quality.silent_fraction, 0.6);
    assert_eq!(measured.quality.peak, 0.5);
    assert_eq!(measured.quality.clipping_fraction, 0.0);
}

#[test]
fn region_comparison_uses_selected_durations_levels_and_preserves_whole_take_summaries() {
    let a = mono(8_000, vec![0.5; 8_000]);
    let b = mono(48_000, vec![0.25; 96_000]);
    let whole_a = analyze(&a).unwrap();
    let whole_b = analyze(&b).unwrap();
    let region_a = measure_region(&a, span(800, 1600)).unwrap();
    let region_b = measure_region(&b, span(48_000, 55_200)).unwrap();
    let comparison = compare_regions(&region_a, &region_b).unwrap();
    assert!((comparison.duration_ratio - 1.5).abs() < 1e-12);
    assert!((comparison.duration_delta_seconds - 0.05).abs() < 1e-12);
    assert!((comparison.rms_delta_db + 6.0206).abs() < 1e-3);
    assert_eq!(comparison.region_a, region_a);
    assert_eq!(comparison.region_b, region_b);
    assert_eq!(whole_a.duration_seconds, 1.0);
    assert_eq!(whole_b.duration_seconds, 2.0);
    assert_eq!(region_a.source.duration_seconds, 1.0);
    assert_eq!(region_b.source.duration_seconds, 2.0);
}

#[test]
fn region_silence_has_finite_display_floor_and_no_resolved_level_difference_claim() {
    let audio = mono(16_000, vec![0.0; 1600]);
    let a = measure_region(&audio, span(0, 800)).unwrap();
    let b = measure_region(&audio, span(800, 1600)).unwrap();
    assert_eq!(a.rms, 0.0);
    assert_eq!(a.quality.rms_dbfs, -100.0);
    assert_eq!(a.quality.leading_silence_seconds, 0.05);
    assert_eq!(a.quality.trailing_silence_seconds, 0.05);
    assert_eq!(a.quality.silent_fraction, 1.0);
    let comparison = compare_regions(&a, &b).unwrap();
    assert_eq!(comparison.rms_delta_db, 0.0);
    assert!(comparison.duration_ratio.is_finite());
    assert!(
        comparison
            .warnings
            .iter()
            .any(|warning| warning.contains("not resolved"))
    );
}

#[test]
fn region_comparison_rejects_forged_nonfinite_or_inconsistent_measurements() {
    let audio = mono(16_000, vec![0.5; 160]);
    let good = measure_region(&audio, span(0, 160)).unwrap();
    let mut bad = good.clone();
    bad.duration_seconds = 0.0;
    assert!(compare_regions(&bad, &good).is_err());
    let mut bad = good.clone();
    bad.rms = f32::NAN;
    assert!(compare_regions(&bad, &good).is_err());
    let mut bad = good.clone();
    bad.quality.rms_dbfs = f32::INFINITY;
    assert!(compare_regions(&bad, &good).is_err());
    let mut bad = good.clone();
    bad.source.sample_rate = 0;
    assert!(compare_regions(&bad, &good).is_err());
    let mut bad = good.clone();
    bad.source.frames = usize::MAX;
    assert!(compare_regions(&bad, &good).is_err());
    let mut bad = good.clone();
    bad.span.end_frame = u64::MAX;
    assert!(compare_regions(&bad, &good).is_err());
    let mut bad = good.clone();
    bad.version = "future-unknown".into();
    assert!(compare_regions(&bad, &good).is_err());
}

struct SuspendOnce {
    pending: bool,
    keep_going: bool,
}
impl Future for SuspendOnce {
    type Output = bool;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<bool> {
        if self.pending {
            self.pending = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(self.keep_going)
        }
    }
}
fn finish<T>(future: impl Future<Output = T>) -> (T, usize) {
    let mut future = std::pin::pin!(future);
    let mut suspended = 0;
    loop {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(value) => return (value, suspended),
            Poll::Pending => suspended += 1,
        };
        assert!(suspended < 10_000);
    }
}

#[test]
fn region_cooperative_scan_resumes_exactly_and_cancels_without_mutating_source() {
    let audio = mono(48_000, vec![0.25; 192_000]);
    let original = audio.metadata().clone();
    let selected = span(13, 191_990);
    let expected = measure_region(&audio, selected).unwrap();
    let (actual, count) = finish(measure_region_cooperatively(&audio, selected, || {
        SuspendOnce {
            pending: true,
            keep_going: true,
        }
    }));
    assert_eq!(actual.unwrap(), expected);
    assert!(count >= 4);
    for stop_at in 1..=count {
        let calls = Cell::new(0);
        let (result, _) = finish(measure_region_cooperatively(&audio, selected, || {
            calls.set(calls.get() + 1);
            SuspendOnce {
                pending: true,
                keep_going: calls.get() < stop_at,
            }
        }));
        assert_eq!(result.unwrap_err(), AudioError::Cancelled);
        assert_eq!(calls.get(), stop_at);
    }
    assert_eq!(audio.metadata(), &original);
    assert!(audio.samples().iter().all(|sample| *sample == 0.25));
}

#[test]
fn region_original_wav_decode_yields_and_preserves_samples_metadata_and_cancellation() {
    let audio = PcmAudio::from_interleaved(48_000, 2, vec![0.25; 192_000]).unwrap();
    let bytes = audio.to_wav();
    let (decoded, count) = finish(PcmAudio::from_wav_cooperatively(&bytes, || SuspendOnce {
        pending: true,
        keep_going: true,
    }));
    let decoded = decoded.unwrap();
    assert_eq!(decoded.metadata(), audio.metadata());
    assert_eq!(decoded.samples(), audio.samples());
    assert!(count >= 8);
    let selected = span(24_001, 48_001);
    assert_eq!(
        measure_region(&decoded, selected).unwrap(),
        measure_region(&audio, selected).unwrap()
    );
    for stop_at in 1..=count {
        let calls = Cell::new(0);
        let (result, _) = finish(PcmAudio::from_wav_cooperatively(&bytes, || {
            calls.set(calls.get() + 1);
            SuspendOnce {
                pending: true,
                keep_going: calls.get() < stop_at,
            }
        }));
        assert_eq!(result.unwrap_err(), AudioError::Cancelled);
    }
}

#[test]
fn region_wav_header_scan_can_cancel_before_sample_allocation() {
    let original = mono(8_000, vec![0.25]).to_wav();
    let mut bytes = original[..12].to_vec();
    for _ in 0..2048 {
        bytes.extend_from_slice(b"JUNK");
        bytes.extend_from_slice(&0_u32.to_le_bytes());
    }
    bytes.extend_from_slice(&original[12..]);
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    let calls = Cell::new(0);
    let (result, _) = finish(PcmAudio::from_wav_cooperatively(&bytes, || {
        calls.set(calls.get() + 1);
        SuspendOnce {
            pending: true,
            keep_going: calls.get() < 3,
        }
    }));
    assert_eq!(result.unwrap_err(), AudioError::Cancelled);
    assert_eq!(calls.get(), 3);
    assert_eq!(PcmAudio::from_wav(&bytes).unwrap().samples(), &[0.25]);
}
