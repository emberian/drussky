"""What it takes to rebuild one bundled clip: shared by the audio generators and scripts/audio.py.

Every clip in Drussky/Resources/Audio is `encode(original)`: the original take (a Commons or
Tatoeba download, a repo foundation WAV, or a local Silero / macOS synthesis), trimmed and levelled
by audio_level.level, then AAC 64 kbps. The sources*.json entry of each clip records enough to get
the original back (`restore_fields`):

  orig        path of the original, relative to the repo root (".local/..." for downloads and
              synthesis output, which are not in git)
  orig_sha1   SHA-1 of that file's bytes
  orig_url    where to download it again (human recordings)
  input/voice/synth   exact Silero (or 2026 narrator engine, or macOS `say`) input and settings (synthetic speech)
  pipeline    PIPELINE
"""
import hashlib
import re
import subprocess
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
REPO = APPLE.parent
sys.path.insert(0, str(APPLE / "scripts"))
from audio_level import decode, level, to_wav  # noqa: E402

PIPELINE = "audio_level.level (trim, -20 dB voiced RMS, -1 dB peak cap) -> afconvert -f m4af -d aac -b 64000"
SILERO_SETTINGS = ("Silero v5_6_ru, 24 kHz, torch.manual_seed(42) per part, explicit-stress flags (put_accent etc. off); "
                   "inputs over 160 chars split after .!?…» with 0.35 s silence between parts")
MACOS_SETTINGS = "macOS `say -v Milena -o <aiff> <input>`"

# The 2026 narrator: one designed voice (VoxCPM2 voice design, research/VOICE_2026.md) cloned by whichever
# engine renders a clip. The reference clip and its '+'-marked transcript are part of the recipe.
NARRATOR_VOICE = "narrator-2026-1"
NARRATOR_REF = REPO / "research/artifacts/voice-2026/narrator/narrator-voxcpm2-1.wav"
NARRATOR_REF_TEXT = ("Здр+авствуйте. Сег+одня м+ы б+удем чит+ать и сл+ушать. Сл+ушайте вним+ательно и повтор+яйте за мн+ой: "
                     "сн+ачала м+едленно, пот+ом быстр+ее.")
_SPLIT = "inputs over 160 chars split after .!?…» with 0.35 s silence between parts"
NARRATOR_SETTINGS = {
    "voxcpm2": ("VoxCPM2 (mlx-community/VoxCPM2-bf16, mlx-audio 0.6.4), 48 kHz, cloned from research/artifacts/voice-2026/narrator/"
                f"narrator-voxcpm2-1.wav, mx.random.seed(42) per part, 24 diffusion steps, cfg 2.0, '+' stress passed as U+0301; {_SPLIT}"),
    "espeech": ("ESpeech-TTS-1 RL-V2 (F5-TTS DiT, f5-tts 1.1.22, torch MPS, Vocos 24 kHz), cloned from research/artifacts/voice-2026/"
                f"narrator/narrator-voxcpm2-1.wav with its '+'-marked transcript, seed 42 per part, 48 NFE steps, cfg 2.0, speed 1.0 "
                f"(0.7 for inputs under 12 letters), native '+' stress; {_SPLIT}"),
}
NARRATOR_PROVIDER = {"voxcpm2": "VoxCPM2 2B (Apache-2.0), designed narrator voice", "espeech": "ESpeech-TTS-1 RL-V2 (Apache-2.0), designed narrator voice"}
NARRATOR_PYTHON = {"voxcpm2": REPO / ".local/tts-2026/mlx/venv/bin/python", "espeech": REPO / ".local/tts-2026/espeech/venv/bin/python"}
UA = {"User-Agent": "DrusskyAudioRestore/1.0 (https://ember.software; personal language-learning app) python-urllib"}


def sha1(path):
    return hashlib.sha1(Path(path).read_bytes()).hexdigest()


def commons_url(title):
    """Direct upload URL of a Wikimedia Commons file (title with or without "File:")."""
    name = title.removeprefix("File:").replace(" ", "_")
    h = hashlib.md5(name.encode()).hexdigest()
    return f"https://upload.wikimedia.org/wikipedia/commons/{h[0]}/{h[:2]}/{urllib.parse.quote(name)}"


def repo_relative(path):
    """Repo-relative spelling of a path; anything under some checkout's .local/ is recorded as .local/..."""
    path = Path(path).resolve()
    try:
        return str(path.relative_to(REPO.resolve()))
    except ValueError:
        parts = path.parts
        if ".local" in parts:
            return str(Path(*parts[parts.index(".local"):]))
        raise


def restore_fields(orig, url=None, synth=None):
    """Fields a sources entry needs for scripts/audio.py restore. synth: {"input", "voice", "synth"}."""
    out = {"orig": repo_relative(orig)}
    if Path(orig).exists() or not synth:  # a synthetic original can be regenerated from its input
        out["orig_sha1"] = sha1(orig)
    if url:
        out["orig_url"] = url
    out.update(synth or {})
    out["pipeline"] = PIPELINE
    return out


def silero_fields(text, voice):
    return {"input": text, "voice": voice, "synth": SILERO_SETTINGS}


def narrator_fields(text, engine):
    return {"input": text, "voice": NARRATOR_VOICE, "synth": NARRATOR_SETTINGS[engine]}


def narrator_engine(synth):
    """Which narrator engine a sources entry's `synth` string names, or None (Silero / macOS)."""
    return next((e for e, s in NARRATOR_SETTINGS.items() if s == synth), None)


def encode(src, m4a, level_dir):
    """Trim silence, level to the common loudness, encode AAC (the one processing step for every clip).
    Returns audio_level's info (where the trim started, the gain applied)."""
    pcm, info = level(decode(src))
    leveled = Path(level_dir) / (Path(m4a).stem + ".wav")
    leveled.parent.mkdir(parents=True, exist_ok=True)
    leveled.write_bytes(to_wav(pcm))
    tmp = Path(m4a).with_suffix(".tmp.m4a")
    subprocess.run(["afconvert", "-f", "m4af", "-d", "aac", "-b", "64000", str(leveled), str(tmp)], check=True)
    tmp.replace(m4a)
    return info


def cut(src, start, end, wav):
    """The stretch [start, end] seconds of a recording as 24 kHz mono WAV (a narration excerpt out of its chapter).
    Sample-exact for the recorded offsets, so restore reproduces the clip and its word times."""
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", f"{start:.3f}", "-to", f"{end:.3f}", "-i", str(src),
                    "-ac", "1", "-ar", "24000", "-c:a", "pcm_s16le", str(wav)], check=True)
    return Path(wav)


def fetch(url, dest, want_sha1=None):
    """One polite sequential download (retries on HTTP 429), checked against the recorded SHA-1."""
    for attempt in range(6):
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
                data = r.read()
            break
        except urllib.error.HTTPError as e:
            if e.code != 429:
                raise
            time.sleep(30 * (attempt + 1))
    else:
        raise RuntimeError(f"rate limited: {url}")
    if want_sha1 and hashlib.sha1(data).hexdigest() != want_sha1:
        raise ValueError(f"{url}: SHA-1 differs from the recorded original")
    dest.parent.mkdir(parents=True, exist_ok=True)
    dest.with_suffix(".part").write_bytes(data)
    dest.with_suffix(".part").replace(dest)
    time.sleep(1)


class Silero:
    """The pinned local Silero model (scripts/speech), loaded on first use."""

    def __init__(self, threads=4):
        self.threads, self.loaded = threads, None

    def wav(self, text, voice):
        """WAV bytes for `text` exactly as the course and pair generators produce them."""
        if self.loaded is None:
            sys.path.insert(0, str(REPO / "scripts/speech"))
            import silero_gateway as gateway
            from silero_worker import encode_wav
            self.loaded, self.flags, self.encode_wav = gateway.load_model(), dict(gateway.EXPLICIT_FLAGS), encode_wav
            self.loaded.torch.set_num_threads(self.threads)
        torch, model = self.loaded.torch, self.loaded.model
        parts = [text] if len(text) <= 160 else [x for x in re.split(r"(?<=[.!?…»])\s+", text) if x.strip()]
        samples = []
        for i, part in enumerate(parts):
            if i:
                samples += [0.0] * int(24000 * 0.35)
            torch.manual_seed(42)
            with torch.inference_mode():
                samples += model.apply_tts(text=part, speaker=voice, sample_rate=24000, **self.flags).detach().cpu().tolist()
        data, frames, signals = self.encode_wav(samples)
        assert signals["clipped_samples"] == 0, (text, signals)
        return data, frames


class Narrator:
    """A narrator engine in its own venv (apple/scripts/narrator_worker.py), started on first use."""

    def __init__(self, engine):
        if engine not in NARRATOR_SETTINGS:
            raise ValueError(f"unknown narrator engine {engine!r}")
        self.engine, self.proc = engine, None

    def start(self):
        import json
        python = NARRATOR_PYTHON[self.engine]
        if not python.exists():
            raise RuntimeError(f"{self.engine} venv missing: {python} (see research/VOICE_2026.md, section 4)")
        if not NARRATOR_REF.exists():
            raise RuntimeError(f"narrator reference missing: {NARRATOR_REF}")
        self.proc = subprocess.Popen([str(python), str(APPLE / "scripts/narrator_worker.py"), self.engine, str(NARRATOR_REF), NARRATOR_REF_TEXT],
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, bufsize=1)
        ready = json.loads(self.proc.stdout.readline() or "{}")
        if not ready.get("ready"):
            raise RuntimeError(f"{self.engine} worker did not start")
        self.load_seconds = ready["load_seconds"]

    def wav(self, text, dest):
        """Synthesize `text` (corpus '+' notation) to the WAV file `dest`; returns the worker's timing dict."""
        import json
        if self.proc is None:
            self.start()
        Path(dest).parent.mkdir(parents=True, exist_ok=True)
        self.proc.stdin.write(json.dumps({"text": text, "out": str(dest)}, ensure_ascii=False) + "\n")
        self.proc.stdin.flush()
        reply = json.loads(self.proc.stdout.readline() or '{"ok": false, "error": "worker exited"}')
        if not reply.get("ok"):
            raise RuntimeError(f"{self.engine}: {reply.get('error')}")
        return reply

    def close(self):
        if self.proc is not None:
            self.proc.stdin.close()
            self.proc.wait(timeout=60)
            self.proc = None


def macos_say(text, voice, dest):
    subprocess.run(["say", "-v", voice, "-o", str(dest), text], check=True)
