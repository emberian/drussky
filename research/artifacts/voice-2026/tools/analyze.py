#!/usr/bin/env python3
"""Acoustic checks for the audition: stress placement in two-syllable words, question pitch, levels.

  .local/tts-2026/mlx/venv/bin/python tools/analyze.py audition/voxcpm2 audition/omnivoice ...

Stress heuristic (the cue research/SILERO_STRESS_AUDIT.md found reliable on Silero: vowel DURATION, not
pitch or loudness). Vowel nuclei are the peaks of the smoothed 300-3000 Hz band energy (vowels are loud
there, nasals/fricatives/stops are not), at least 70 ms apart. For a bare two-syllable word the two most
prominent peaks are the two vowels; for "Это X." the last two peaks are X's vowels. Each nucleus lasts
from where the energy rises to within 8 dB of its peak to where it falls below again (bounded by the
valley). The longer nucleus is called stressed; within 15 ms the louder one. Everything is written to
DIR/acoustics.json so a disagreement can be inspected rather than trusted.

Question pitch: F0 contour (pyin) of Q01/Q02, peak F0, position of the peak, F0 range.
"""
import json
import sys
from pathlib import Path

import librosa
import numpy as np
import soundfile as sf
from scipy.signal import find_peaks

HERE = Path(__file__).resolve().parent
ITEMS = json.loads((HERE.parent / "items.json").read_text())
STRESS = {w["id"]: w["stressed_syllable"] for w in ITEMS["stress_words"]}
STRESS.update({"H01": 1, "H02": 2})
SR = 24000
HOP = 0.005
DROP_DB = 12


def band_energy(y, sr, lo=300, hi=3000, win=0.025, hop=HOP):
    n_fft = 1024
    S = np.abs(librosa.stft(y, n_fft=n_fft, hop_length=int(hop * sr), win_length=int(win * sr))) ** 2
    f = librosa.fft_frequencies(sr=sr, n_fft=n_fft)
    band = S[(f >= lo) & (f <= hi)].sum(0)
    e = 10 * np.log10(band + 1e-10)
    k = 7  # 35 ms smoothing
    return np.convolve(e, np.ones(k) / k, mode="same")


def nuclei(e, last_two=False):
    peaks, props = find_peaks(e, distance=14, prominence=3, height=e.max() - 30)
    if len(peaks) < 2:
        return None
    if last_two:
        chosen = peaks[-2:]
    else:
        order = np.argsort(props["prominences"])[::-1][:2]
        chosen = np.sort(peaks[order])
    p1, p2 = chosen
    valley = p1 + int(np.argmin(e[p1:p2 + 1]))
    out = []
    for p, lo, hi in ((p1, 0, valley), (p2, valley, len(e))):
        thr = e[p] - DROP_DB
        s = p
        while s > lo and e[s - 1] >= thr:
            s -= 1
        t = p
        while t < hi - 1 and e[t + 1] >= thr:
            t += 1
        out.append((s, t + 1, p))
    return out


def f0_track(y, sr):
    f0, voiced, _ = librosa.pyin(y, fmin=80, fmax=450, sr=sr, frame_length=1024, hop_length=int(HOP * sr))
    return f0, voiced


def analyse_word(y, sr, want, last_two):
    e = band_energy(y, sr)
    n = nuclei(e, last_two)
    if not n:
        return {"stress_measurable": False}
    f0, voiced = f0_track(y, sr)
    out = {"stress_measurable": True, "wanted": want}
    for k, (s, t, p) in zip(("v1", "v2"), n):
        f = f0[s:t][voiced[s:t] & ~np.isnan(f0[s:t])]
        out[k] = {"start_s": round(s * HOP, 3), "end_s": round(t * HOP, 3), "duration_ms": int((t - s) * HOP * 1000),
                  "peak_band_db": round(float(e[p]), 1), "median_f0": round(float(np.median(f)), 1) if len(f) else None}
    d1, d2 = out["v1"]["duration_ms"], out["v2"]["duration_ms"]
    if abs(d1 - d2) <= 15:
        got = 1 if out["v1"]["peak_band_db"] >= out["v2"]["peak_band_db"] else 2
        out["cue"] = "energy (durations within 15 ms)"
    else:
        got = 1 if d1 > d2 else 2
        out["cue"] = "duration"
    out["heard_stress"] = int(got)
    out["correct"] = bool(got == want)
    return out


def main(dirs):
    for d in map(Path, dirs):
        res = {}
        for wav in sorted(d.glob("*.wav")):
            y, sr = sf.read(wav, dtype="float32")
            if y.ndim > 1:
                y = y.mean(1)
            if sr != SR:
                y = librosa.resample(y, orig_sr=sr, target_sr=SR); sr = SR
            yt, _ = librosa.effects.trim(y, top_db=35)
            entry = {"duration_s": round(len(y) / sr, 3), "trimmed_s": round(len(yt) / sr, 3),
                     "peak_dbfs": round(float(20 * np.log10(np.abs(y).max() + 1e-9)), 1),
                     "rms_dbfs": round(float(20 * np.log10(np.sqrt((yt ** 2).mean()) + 1e-9)), 1)}
            stem = wav.stem
            if stem in STRESS:
                entry["stress"] = analyse_word(yt, sr, STRESS[stem], last_two=stem.startswith("H"))
            if stem in ("Q01", "Q02"):
                f0, voiced = f0_track(yt, sr)
                f = f0[voiced & ~np.isnan(f0)]
                if len(f):
                    peak_i = int(np.nanargmax(np.where(voiced, f0, np.nan)))
                    third = f0[-len(f0) // 3:]
                    entry["pitch"] = {"f0_min": round(float(f.min()), 1), "f0_max": round(float(f.max()), 1),
                                      "f0_range_semitones": round(float(12 * np.log2(f.max() / f.min())), 1),
                                      "peak_position": round(peak_i * HOP / (len(yt) / sr), 2),
                                      "final_third_mean": round(float(np.nanmean(third)), 1) if not np.all(np.isnan(third)) else None,
                                      "contour": [None if np.isnan(v) else round(float(v), 1) for v in f0[::4]]}
            res[stem] = entry
        measurable = [v["stress"] for v in res.values() if "stress" in v and v["stress"].get("stress_measurable")]
        # Pairwise control test, robust to the segmenter's systematic bias: for each minimal pair the
        # authored stress moves from syllable 1 to 2, so the V1/V2 duration ratio must drop. A model that
        # ignores the marks yields the same ratio for both members.
        pairs = [("W%02d" % i, "W%02d" % (i + 1)) for i in range(1, 16, 2)] + [("H01", "H02")]
        pair_results = {}
        for a, b in pairs:
            sa, sb = res.get(a, {}).get("stress", {}), res.get(b, {}).get("stress", {})
            if sa.get("stress_measurable") and sb.get("stress_measurable"):
                ra = sa["v1"]["duration_ms"] / max(sa["v2"]["duration_ms"], 1)
                rb = sb["v1"]["duration_ms"] / max(sb["v2"]["duration_ms"], 1)
                pair_results[f"{a}/{b}"] = {"ratio_syll1_stressed": round(ra, 2), "ratio_syll2_stressed": round(rb, 2),
                                            "controlled": bool(ra > rb * 1.15)}
        summary = {"stress_items": sum("stress" in v for v in res.values()), "stress_measurable": len(measurable),
                   "stress_correct": sum(s["correct"] for s in measurable),
                   "pairs_measurable": len(pair_results), "pairs_controlled": sum(p["controlled"] for p in pair_results.values()),
                   "pairs": pair_results}
        res["_summary"] = summary
        (d / "acoustics.json").write_text(json.dumps(res, ensure_ascii=False, indent=1) + "\n")
        print(d, summary)


if __name__ == "__main__":
    main(sys.argv[1:])
