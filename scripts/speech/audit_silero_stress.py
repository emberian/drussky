#!/usr/bin/env python3
"""Reproduce the two-token Silero stress signal audit using existing WAVs only.

No model, network, ASR, TTS, phone alignment or pronunciation classifier. The
candidate vowel boundaries below were selected from the shared-scale spectrogram,
not inferred by this script. Their sensitivity ranges are not confidence intervals.
Requires NumPy/SciPy (the existing ASR venv) and installed gnuplot.
"""
import os
for name in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "MKL_NUM_THREADS"):
    os.environ[name] = "2"
import hashlib
import json
import math
from pathlib import Path
import subprocess
import wave
import numpy as np
from scipy import linalg, signal
import scipy

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "research/artifacts/silero-stress-v1"
SCRATCH = ROOT / ".local/speech-acoustics/arrays"
SOURCES = {
    "castle": "3ce677ebdad80003b514c177abdb2eb7efbc7674a92acbc2abf2f80b8d51ebe4",
    "lock": "3f0198c2415844291042e3161e445520e6d9aad7aec0c6956596a8c1e2d0ec76",
}
# (nominal region, plausible onset range, plausible offset range, spectral interior)
# The low-frequency nasal interval between V1 and V2 is deliberately excluded.
REGIONS = {
    "castle": [((.295, .435), (.285, .305), (.425, .445), (.325, .415)),
               ((.500, .550), (.490, .510), (.540, .560), (.510, .540))],
    "lock": [((.275, .350), (.265, .285), (.340, .360), (.290, .335)),
             ((.395, .540), (.385, .405), (.530, .550), (.420, .515))],
}


def finite_median(values):
    values = np.asarray(values)
    return float(np.median(values[np.isfinite(values)])) if np.isfinite(values).any() else None


def tracks(x, rate):
    y = signal.resample_poly(x, 1, 2)
    fs, result, poles = rate // 2, [], []
    for center in range(420, len(y) - 420, 60):
        time = center / fs
        level = math.sqrt(float(np.mean(y[center - 120:center + 120] ** 2)))
        block = y[center - 300:center + 300]
        # Fixed 30 ms YIN difference window; 80–400 Hz candidates; 5 ms hop.
        difference = np.array([np.sum((block[:360] - block[lag:lag + 360]) ** 2) for lag in range(151)])
        cmnd = np.ones(len(difference))
        cmnd[1:] = difference[1:] * np.arange(1, len(difference)) / np.maximum(np.cumsum(difference[1:]), 1e-20)
        lag = None
        for candidate in range(30, 150):
            if cmnd[candidate] < .15:
                lag = candidate
                while lag < 150 and cmnd[lag + 1] < cmnd[lag]:
                    lag += 1
                break
        f0, periodicity = np.nan, np.nan
        if lag is not None and lag < 150 and level > 1e-4:
            denominator = cmnd[lag - 1] - 2 * cmnd[lag] + cmnd[lag + 1]
            delta = .5 * (cmnd[lag - 1] - cmnd[lag + 1]) / denominator if abs(denominator) > 1e-12 else 0
            f0, periodicity = fs / (lag + delta), 1 - cmnd[lag]
        result.append((time, 20 * math.log10(max(level, 1e-6)), f0, periodicity))
        frame = y[center - 180:center + 180]
        windowed = signal.lfilter([1, -.97], [1], frame) * signal.windows.hamming(len(frame), sym=False)
        correlation = signal.correlate(windowed, windowed, mode="full", method="direct")[len(frame) - 1:]
        for order in (12, 14, 16):
            try:
                coefficients = linalg.solve_toeplitz((correlation[:order], correlation[:order]), -correlation[1:order + 1])
                roots = np.roots(np.r_[1, coefficients])
                roots = roots[np.imag(roots) > 0]
                frequencies = np.angle(roots) * fs / (2 * np.pi)
                bandwidths = -fs / np.pi * np.log(np.maximum(np.abs(roots), 1e-20))
                candidates = [(frequencies[i], bandwidths[i]) for i in np.argsort(frequencies)
                              if 150 < frequencies[i] < 4500 and 30 < bandwidths[i] < 700]
                for number, (frequency, bandwidth) in enumerate(candidates[:3], 1):
                    poles.append((time, order, number, frequency, bandwidth))
            except (ValueError, linalg.LinAlgError):
                pass
    return np.array(result), np.array(poles)


def analyze(label):
    source = ROOT / f".local/silero-tts/samples/silero-stress-{label}-v1.wav"
    sidecar = json.loads(source.with_suffix(".json").read_text())
    digest = hashlib.sha256(source.read_bytes()).hexdigest()
    if digest != SOURCES[label] or digest != sidecar["sha256"]:
        raise ValueError("Source audio changed; this audit is tied to the exact controlled pair")
    with wave.open(str(source), "rb") as wav:
        if (wav.getframerate(), wav.getnchannels(), wav.getsampwidth()) != (24000, 1, 2):
            raise ValueError("Unexpected sample format")
        frames, rate = wav.getnframes(), wav.getframerate()
        pcm = wav.readframes(frames)
    x = np.frombuffer(pcm, dtype="<i2").astype(np.float64) / 32768
    frequencies, times, spectrum = signal.stft(x, rate, window="hann", nperseg=480, noverlap=360,
                                             nfft=1024, boundary=None, padded=False)
    levels = 20 * np.log10(np.maximum(2 * np.abs(spectrum), 1e-5))
    with (SCRATCH / f"{label}-spectrogram.tsv").open("w") as output:
        for index, time in enumerate(times):
            for f_index, frequency in enumerate(frequencies):
                if 0 < frequency <= 4500:
                    output.write(f"{time:.6f}\t{frequency:.4f}\t{levels[f_index, index]:.6f}\n")
            output.write("\n")
    measurements, poles = tracks(x, rate)
    np.savetxt(SCRATCH / f"{label}-tracks.tsv", measurements, delimiter="\t", fmt="%.7f")
    np.savetxt(SCRATCH / f"{label}-lpc.tsv", poles, delimiter="\t", fmt="%.7f")
    regions = []
    for index, (nominal, onset, offset, interior) in enumerate(REGIONS[label], 1):
        start, end = (round(t * rate) for t in nominal)
        window = (measurements[:, 0] >= interior[0]) & (measurements[:, 0] <= interior[1])
        segment = x[round(interior[0] * rate):round(interior[1] * rate)]
        frequencies, power = signal.welch(segment, rate, nperseg=min(len(segment), 480), nfft=2048)
        low = power[(frequencies >= 300) & (frequencies < 1000)].sum()
        mid = power[(frequencies >= 1000) & (frequencies < 2000)].sum()
        candidates = {}
        for order in (12, 14, 16):
            candidates[str(order)] = []
            for number in (1, 2):
                selected = poles[(poles[:, 0] >= interior[0]) & (poles[:, 0] <= interior[1])
                                 & (poles[:, 1] == order) & (poles[:, 2] == number)]
                candidates[str(order)].append({"candidate_index": number,
                                              "median_frequency_hz": finite_median(selected[:, 3]),
                                              "median_bandwidth_hz": finite_median(selected[:, 4])})
        regions.append({"id": f"vowel_candidate_{index}", "start_frame": start, "end_frame": end,
                        "duration_ms": (end - start) / rate * 1000,
                        "boundary_sensitivity_duration_range_ms": [(offset[0] - onset[1]) * 1000, (offset[1] - onset[0]) * 1000],
                        "boundary_ranges_are": "analyst-selected sensitivity ranges, not confidence intervals or measured alignment errors",
                        "interior_frames": [round(t * rate) for t in interior],
                        "interior_median_rms_dbfs": finite_median(measurements[window, 1]),
                        "interior_median_f0_hz": finite_median(measurements[window, 2]),
                        "interior_median_yin_periodicity": finite_median(measurements[window, 3]),
                        "low_to_mid_band_power_db": 10 * math.log10(low / mid),
                        "band_power_definition": "300–1000 Hz / 1000–2000 Hz; descriptive Welch power ratio, not a stress classifier",
                        "lpc_poles_by_order": candidates,
                        "formant_labels_validated": False})
    # Context-preserving original-byte crop, never time-normalized or gain-matched.
    crop_start, crop_end = round(.235 * rate), round(.690 * rate)
    crop_name = f"{label}-word-replay.wav"
    with wave.open(str(OUT / crop_name), "wb") as output:
        output.setnchannels(1); output.setsampwidth(2); output.setframerate(rate)
        output.writeframes(pcm[crop_start * 2:crop_end * 2])
    return {"source_text": sidecar["source_text"], "source_audio_sha256": digest,
            "source_relative_path": str(source.relative_to(ROOT)), "provenance": sidecar["provenance"],
            "sample_rate": rate, "frames": frames, "regions": regions,
            "word_replay": {"path": crop_name, "start_frame": crop_start, "end_frame": crop_end,
                            "sha256": hashlib.sha256((OUT / crop_name).read_bytes()).hexdigest(),
                            "coordinates": "original frames; half-open; analyst-selected context crop"}}


def quote(path):
    return "'" + str(path).replace("'", "''") + "'"


def plot():
    commands = ["set encoding utf8", "set terminal pngcairo size 1800,1320 enhanced font 'Arial,15'",
                f"set output {quote(OUT / 'stress-comparison.png')}",
                'set multiplot layout 3,2 title "Stress control changes vowel timing\\nExisting Silero pair | shared scales | exploratory vowel regions" font ",20"',
                "set xrange [.235:.69]", "set xtics .05", "set grid xtics", "set key off",
                "set lmargin 9", "set rmargin 10", "set tmargin 3", "set bmargin 4",
                "set palette defined (-70 '#faf9f6',-55 '#d9e6e6',-40 '#7babae',-25 '#26686f',-10 '#13242d')"]
    def shades(label):
        left, right = (.08, .43) if label == "castle" else (.58, .93)
        result = ["unset object", "unset label", f"set lmargin at screen {left}", f"set rmargin at screen {right}"]
        for index, (region, _, _, _) in enumerate(REGIONS[label], 1):
            color = "#d89b3e" if index == 1 else "#855ca8"
            result.append(f"set object {index} rect from {region[0]},graph 0 to {region[1]},graph 1 front fillcolor rgb '{color}' fillstyle transparent solid 0.12 noborder")
        return result
    for label, title in (("castle", "за́мок · castle"), ("lock", "замо́к · lock")):
        commands += shades(label)
        commands += ["set yrange [0:4500]", "set ytics 1000", "set ylabel 'Frequency (Hz)'", "set xlabel 'Original time (s)'",
                     "set cbrange [-70:-10]", "set colorbox", "set cblabel 'Sinusoid-relative dBFS'", f"set title '{title}'"]
        for index, (region, _, _, _) in enumerate(REGIONS[label], 1):
            commands.append(f"set label {index} 'V{index}: {round((region[1] - region[0]) * 1000)} ms' at {(region[0] + region[1]) / 2},graph .94 center front font ',12'")
        commands.append(f"plot {quote(SCRATCH / (label + '-spectrogram.tsv'))} u 1:2:3 with image")
    for label in ("castle", "lock"):
        commands += shades(label)
        commands += ["unset colorbox", "set yrange [-45:-10]", "set ytics 10 nomirror", "set y2range [140:240]", "set y2tics 20",
                     "set ylabel 'RMS (dBFS)'", "set y2label 'F0 estimate (Hz)'", "set title 'Level and pitch do not simply switch peaks'", "set key bottom right",
                     f"plot {quote(SCRATCH / (label + '-tracks.tsv'))} u 1:2 w l lw 2 lc rgb '#257c86' t 'RMS', '' u 1:3 axes x1y2 w p pt 7 ps .45 lc rgb '#993366' t 'F0'"]
    for label in ("castle", "lock"):
        commands += shades(label)
        commands += ["unset y2tics", "unset y2label", "set ytics 500 mirror", "set yrange [150:3200]", "set ylabel 'LPC pole frequency (Hz)'",
                     "set title 'Exploratory resonances: sensitive to LPC model order'", "set key top right",
                     f"plot {quote(SCRATCH / (label + '-lpc.tsv'))} u 1:($2==12?$4:1/0) w p pt 6 ps .4 lc rgb '#a65032' t '12', '' u 1:($2==14?$4:1/0) w p pt 7 ps .35 lc rgb '#296477' t '14', '' u 1:($2==16?$4:1/0) w p pt 4 ps .45 lc rgb '#925782' t '16'"]
    commands.append("unset multiplot")
    path = SCRATCH / "comparison.gnuplot"
    path.write_text("\n".join(commands) + "\n")
    subprocess.run(["gnuplot", str(path)], check=True)
    svg = "\n".join(commands).replace("set terminal pngcairo size 1800,1320 enhanced font 'Arial,15'", "set terminal svg size 1800,1320 enhanced font 'Arial,15'")
    svg = svg.replace(str(OUT / "stress-comparison.png"), str(OUT / "stress-comparison.svg"))
    path.write_text(svg + "\n")
    subprocess.run(["gnuplot", str(path)], check=True)
    artifact = OUT / "stress-comparison.svg"
    artifact.write_text("\n".join(line.rstrip() for line in artifact.read_text().splitlines()).rstrip() + "\n")


def main():
    OUT.mkdir(parents=True, exist_ok=True); SCRATCH.mkdir(parents=True, exist_ok=True)
    summary = {"format": "rufren-two-token-signal-audit-v1", "scope": "two existing synthetic recordings, no native correctness or calibration claim",
               "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
               "numpy_version": np.__version__, "scipy_version": scipy.__version__,
               "methods": {"spectrogram": "24 kHz, 20 ms Hann, 5 ms hop, 1024 FFT; 2|STFT| levels; no gain normalization",
                           "rms": "20 ms original-time windows; 5 ms hop, after polyphase downsampling to 12 kHz",
                           "pitch": "YIN-style fixed 30 ms difference region plus lag support, 80–400 Hz, threshold .15; no interpolation across missing F0",
                           "lpc": "12 kHz, 30 ms Hamming, preemphasis .97, autocorrelation LPC orders 12/14/16, positive-frequency poles with 30–700 Hz bandwidth",
                           "regions": "analyst-selected from spectral transitions; interiors exclude edges; sensitivity ranges are not error bars"},
               "tokens": {label: analyze(label) for label in SOURCES}}
    (OUT / "measurements.json").write_text(json.dumps(summary, ensure_ascii=False, allow_nan=False, indent=2) + "\n")
    plot()
    print("Wrote shared-scale PNG/SVG, exact replay crops and descriptive measurements; no model inference.")


if __name__ == "__main__":
    main()
