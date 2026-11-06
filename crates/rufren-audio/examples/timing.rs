//! Explicit longest-input timing probe; not part of the default test suite.
use std::cell::Cell;
use std::future::{Future, ready};
use std::task::{Context, Poll, Waker};
use std::time::{Duration, Instant};

use rufren_audio::{PcmAudio, analyze_cooperatively};

fn complete<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(result) => result,
        Poll::Pending => unreachable!("this timing probe uses only immediately-ready callbacks"),
    }
}

fn main() {
    let rate = 192_000_u32;
    let seconds = 60;
    let samples: Vec<f32> = (0..rate as usize * seconds)
        .map(|i| (0.5 * (std::f64::consts::TAU * 150.0 * i as f64 / rate as f64).sin()) as f32)
        .collect();
    let begin = Instant::now();
    let previous = Cell::new(begin);
    let longest = Cell::new(Duration::ZERO);
    let checkpoints = Cell::new(0);
    let mark = || {
        let now = Instant::now();
        longest.set(longest.get().max(now.duration_since(previous.replace(now))));
        checkpoints.set(checkpoints.get() + 1);
        ready(true)
    };
    let pcm = complete(PcmAudio::from_interleaved_cooperatively(
        rate, 1, samples, &mark,
    ))
    .unwrap();
    let validated = begin.elapsed();
    let constructor_longest = longest.replace(Duration::ZERO);
    let begin = Instant::now();
    previous.set(begin);
    let analysis = complete(analyze_cooperatively(&pcm, &mark)).unwrap();
    println!(
        "60 s mono at 192 kHz: validate/hash {:.3}s (max segment {:.3}ms); analysis {:.3}s (max segment {:.3}ms); {} checkpoints; {} spectrum frames, {} pitch frames, {} waveform bins",
        validated.as_secs_f64(),
        constructor_longest.as_secs_f64() * 1000.0,
        begin.elapsed().as_secs_f64(),
        longest.get().as_secs_f64() * 1000.0,
        checkpoints.get(),
        analysis.spectrogram.frame_times_seconds.len(),
        analysis.pitch.len(),
        analysis.waveform.len()
    );
    assert_eq!(analysis.duration_seconds, 60.0);
    assert_eq!(analysis.waveform.len(), 1200);
    assert!(analysis.spectrogram.power_db.iter().all(|x| x.is_finite()));
}
