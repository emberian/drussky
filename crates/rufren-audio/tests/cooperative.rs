use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use rufren_audio::{AudioError, PcmAudio, analyze, analyze_cooperatively};

/// Real suspension at each callback, so a test cannot pass merely because the
/// async facade actually performs all work before its first suspension.
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
    let mut pending = 0;
    loop {
        match future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        {
            Poll::Ready(result) => return (result, pending),
            Poll::Pending => pending += 1,
        }
        assert!(pending < 10_000, "bounded test unexpectedly did not finish");
    }
}

fn voice() -> Vec<f32> {
    (0..48_000)
        .flat_map(|i| {
            let x = (std::f64::consts::TAU * 143.0 * i as f64 / 48_000.0).sin() as f32;
            [x * 0.6, x * 0.4]
        })
        .collect()
}

#[test]
fn cooperative_constructor_and_all_analysis_stages_match_synchronous_results() {
    let samples = voice();
    let expected_pcm = PcmAudio::from_interleaved(48_000, 2, samples.clone()).unwrap();
    let (pcm, constructor_yields) = finish(PcmAudio::from_interleaved_cooperatively(
        48_000,
        2,
        samples,
        || SuspendOnce {
            pending: true,
            keep_going: true,
        },
    ));
    let pcm = pcm.unwrap();
    assert!(constructor_yields >= 3);
    assert_eq!(pcm.metadata(), expected_pcm.metadata());
    assert_eq!(pcm.samples(), expected_pcm.samples());
    let expected = analyze(&expected_pcm).unwrap();
    let (actual, analysis_yields) = finish(analyze_cooperatively(&pcm, || SuspendOnce {
        pending: true,
        keep_going: true,
    }));
    assert_eq!(actual.unwrap(), expected);
    assert!(
        analysis_yields > 20,
        "all stages must offer scheduling opportunities"
    );
}

#[test]
fn cooperative_constructor_cancels_before_hashing_and_between_input_chunks() {
    for stop_at in [1, 2, 3] {
        let calls = Cell::new(0);
        let (result, yields) = finish(PcmAudio::from_interleaved_cooperatively(
            48_000,
            2,
            voice(),
            || {
                calls.set(calls.get() + 1);
                SuspendOnce {
                    pending: true,
                    keep_going: calls.get() < stop_at,
                }
            },
        ));
        assert_eq!(result.unwrap_err(), AudioError::Cancelled);
        assert_eq!(calls.get(), stop_at);
        assert_eq!(yields, stop_at);
    }
}

#[test]
fn cooperative_analysis_cancels_at_every_pipeline_checkpoint() {
    let pcm = PcmAudio::from_interleaved(48_000, 2, voice()).unwrap();
    let total = Cell::new(0);
    finish(analyze_cooperatively(&pcm, || {
        total.set(total.get() + 1);
        SuspendOnce {
            pending: true,
            keep_going: true,
        }
    }))
    .0
    .unwrap();
    // Covers downmix, source quality, envelope, both resamplers, FFT, and F0.
    for stop_at in 1..=total.get() {
        let calls = Cell::new(0);
        let result = finish(analyze_cooperatively(&pcm, || {
            calls.set(calls.get() + 1);
            SuspendOnce {
                pending: true,
                keep_going: calls.get() < stop_at,
            }
        }))
        .0;
        assert_eq!(
            result.unwrap_err(),
            AudioError::Cancelled,
            "checkpoint {stop_at}"
        );
        assert_eq!(calls.get(), stop_at);
    }
}

#[test]
fn cooperative_constructor_still_validates_limits_and_nonfinite_values() {
    let invalid_shape = finish(PcmAudio::from_interleaved_cooperatively(
        192_000,
        8,
        vec![0.0; 16_000_008],
        || std::future::ready(true),
    ))
    .0;
    assert_eq!(invalid_shape.unwrap_err(), AudioError::TooLarge);
    let invalid_value = finish(PcmAudio::from_interleaved_cooperatively(
        48_000,
        1,
        vec![f32::NAN],
        || std::future::ready(true),
    ))
    .0;
    assert_eq!(invalid_value.unwrap_err(), AudioError::InvalidSample);
}
