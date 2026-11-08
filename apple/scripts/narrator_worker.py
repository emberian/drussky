"""The 2026 narrator engines, run inside their own venvs (see research/VOICE_2026.md).

audio_store.Narrator starts this as a subprocess with the engine's interpreter and talks JSON lines:
  request   {"text": "+Это з+амок.", "out": "/abs/path.wav"}       text in the corpus '+' stress notation
  response  {"ok": true, "seconds": 1.12, "generation_seconds": 2.3} or {"ok": false, "error": "..."}
One process, model loaded once, the reference clip encoded once. Engines:
  voxcpm2   mlx-community/VoxCPM2-bf16 through mlx-audio (48 kHz); U+0301 marks are passed but not honoured
  espeech   ESpeech/ESpeech-TTS-1_RL-V2 (F5-TTS, PyTorch MPS, 24 kHz); '+' marks are its native stress syntax
Both clone the same designed narrator reference, so the timbre is one voice whichever engine renders a clip.

  .local/tts-2026/mlx/venv/bin/python apple/scripts/narrator_worker.py voxcpm2 <ref.wav> <ref-text-with-plus-marks>
"""
import json
import os
import re
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
STATE = REPO / ".local/tts-2026"
os.environ.update(HF_HOME=str(STATE / "hf-cache"), HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", TOKENIZERS_PARALLELISM="false")
for key in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "VECLIB_MAXIMUM_THREADS", "MKL_NUM_THREADS"):
    os.environ.setdefault(key, "4")

VOWELS = "аеёиоуыэюяАЕЁИОУЫЭЮЯ"
SEED = 42
PAUSE = 0.35  # seconds between parts of a split input (same as the Silero path)


def plus_to_acute(text):
    return re.sub(rf"\+([{VOWELS}])", lambda m: m.group(1) + "́", text)


def plus_to_plain(text):
    return re.sub(rf"\+([{VOWELS}])", r"\1", text)


def parts(text, limit=160):
    """Sentence-split long inputs, as the Silero path does; VoxCPM2's guide also asks for short segments."""
    if len(text) <= limit:
        return [text]
    return [x for x in re.split(r"(?<=[.!?…»])\s+", text) if x.strip()]


class VoxCPM2:
    SR = 48000
    SETTINGS = {"inference_timesteps": 24, "cfg_value": 2.0}

    def __init__(self, ref, ref_text):
        import mlx.core as mx
        from mlx_audio.tts.utils import load_model
        mx.set_memory_limit(20 * 1024**3)
        self.mx, self.model, self.ref = mx, load_model(str(STATE / "models/VoxCPM2-bf16")), str(ref)

    def say(self, text):
        import numpy as np
        self.mx.random.seed(SEED)
        res = list(self.model.generate(text=plus_to_acute(text), ref_audio=self.ref, **self.SETTINGS))
        return np.concatenate([np.asarray(r.audio, dtype=np.float32).reshape(-1) for r in res])


class ESpeech:
    SR = 24000
    MODEL_CFG = dict(dim=1024, depth=22, heads=16, ff_mult=2, text_dim=512, conv_layers=4)  # the model card's config
    SETTINGS = {"nfe_step": 48, "cfg_strength": 2.0, "speed": 1.0, "sway_sampling_coef": -1.0, "cross_fade_duration": 0.15}

    def __init__(self, ref, ref_text):
        from f5_tts.infer.utils_infer import infer_process, load_model, load_vocoder, preprocess_ref_audio_text
        from f5_tts.model import DiT
        from f5_tts.model.utils import seed_everything
        d = STATE / "models/ESpeech-TTS-1_RL-V2"
        self.device = "mps"
        self.model = load_model(DiT, self.MODEL_CFG, str(d / "espeech_tts_rlv2.pt"), vocab_file=str(d / "vocab.txt"), device=self.device)
        self.vocoder = load_vocoder("vocos", True, str(STATE / "models/vocos-mel-24khz"), device=self.device)
        quiet = lambda *a, **k: None  # noqa: E731
        self.ref, self.ref_text = preprocess_ref_audio_text(str(ref), ref_text, show_info=quiet)
        self.infer, self.seed, self.quiet = infer_process, seed_everything, quiet

    SHORT_CHARS, SHORT_SPEED = 12, 0.7  # F5-TTS sizes the output from the reference ratio; bare words came out cut ("мука" -> "м")

    def say(self, text):
        self.seed(SEED)
        settings = dict(self.SETTINGS)
        if len(plus_to_plain(text).strip(" .!?…")) < self.SHORT_CHARS:
            settings["speed"] = self.SHORT_SPEED
        wav, sr, _ = self.infer(self.ref, self.ref_text, text, self.model, self.vocoder, show_info=self.quiet, progress=None,
                                device=self.device, **settings)
        assert sr == self.SR
        return wav


ENGINES = {"voxcpm2": VoxCPM2, "espeech": ESpeech}


def main():
    import numpy as np
    import soundfile as sf
    # The protocol owns the real stdout; the libraries' progress prints (F5-TTS is chatty) go to stderr.
    proto = os.fdopen(os.dup(1), "w")
    os.dup2(2, 1)
    sys.stdout = sys.stderr

    def send(obj):
        proto.write(json.dumps(obj) + "\n")
        proto.flush()

    engine, ref, ref_text = sys.argv[1], sys.argv[2], sys.argv[3]
    t = time.monotonic()
    tts = ENGINES[engine](ref, ref_text)
    send({"ready": True, "load_seconds": round(time.monotonic() - t, 2)})
    for line in sys.stdin:
        req = json.loads(line)
        try:
            t = time.monotonic()
            chunks = []
            for i, part in enumerate(parts(req["text"])):
                if i:
                    chunks.append(np.zeros(int(tts.SR * PAUSE), np.float32))
                chunks.append(np.asarray(tts.say(part), dtype=np.float32))
            audio = np.concatenate(chunks)
            peak = float(np.abs(audio).max())
            if peak > 0.99:  # keep the original un-clipped; audio_level.level sets the final loudness anyway
                audio = audio * (0.99 / peak)
            Path(req["out"]).parent.mkdir(parents=True, exist_ok=True)
            sf.write(req["out"], audio, tts.SR, subtype="PCM_16")
            send({"ok": True, "seconds": round(len(audio) / tts.SR, 3), "generation_seconds": round(time.monotonic() - t, 3)})
        except Exception as e:  # noqa: BLE001 -- the client reports it per clip
            send({"ok": False, "error": f"{type(e).__name__}: {e}"})


if __name__ == "__main__":
    main()
