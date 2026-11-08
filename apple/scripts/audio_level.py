"""Trim silence and level a clip so human and synthetic recordings sit at one volume.

Pure stdlib apart from ffmpeg (decode any input to 24 kHz mono PCM16).
"""
import array
import io
import math
import subprocess
import wave

RATE = 24000
TARGET_RMS_DB = -20.0   # RMS over voiced frames
PEAK_CEIL_DB = -1.0
FRAME = RATE // 100     # 10 ms


def decode(path):
    out = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-ac", "1", "-ar", str(RATE), "-f", "s16le", "-"],
                         check=True, capture_output=True).stdout
    return array.array("h", out)


def db(x):
    return 20 * math.log10(max(x, 1e-9))


def level(pcm):
    """Return (pcm, info): trimmed to speech with short pads, gain to TARGET_RMS_DB, peak-capped."""
    n = len(pcm) // FRAME
    if n == 0:
        raise ValueError("empty clip")
    energies = []
    for i in range(n):
        seg = pcm[i * FRAME:(i + 1) * FRAME]
        energies.append(math.sqrt(sum(v * v for v in seg) / FRAME) / 32768)
    loudest = max(energies)
    threshold = max(loudest * 10 ** (-35 / 20), 10 ** (-55 / 20))
    voiced = [i for i, e in enumerate(energies) if e >= threshold]
    first, last = voiced[0], voiced[-1]
    start = max(0, (first - 6) * FRAME)              # 60 ms before
    end = min(len(pcm), (last + 1 + 12) * FRAME)     # 120 ms after
    clip = pcm[start:end]
    active = [energies[i] for i in voiced]
    rms = math.sqrt(sum(e * e for e in active) / len(active))
    peak = max(abs(v) for v in clip) / 32768
    gain_db = TARGET_RMS_DB - db(rms)
    gain_db = min(gain_db, PEAK_CEIL_DB - db(peak))
    g = 10 ** (gain_db / 20)
    fade = RATE // 200  # 5 ms
    out = array.array("h", bytes(2 * len(clip)))
    for i, v in enumerate(clip):
        f = min(1.0, i / fade, (len(clip) - 1 - i) / fade)
        out[i] = max(-32767, min(32767, round(v * g * f)))
    info = {"trimmed_seconds": round(len(clip) / RATE, 3), "speech_seconds": round((last - first + 1) / 100, 2),
            "start_seconds": round(start / RATE, 3),
            "gain_db": round(gain_db, 1), "noise_floor_db": round(db(sorted(energies)[len(energies) // 10]), 1),
            "peak_db_before": round(db(peak), 1)}
    return out, info


def to_wav(pcm):
    buf = io.BytesIO()
    with wave.open(buf, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE); w.writeframes(pcm.tobytes())
    return buf.getvalue()
