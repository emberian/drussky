#!/usr/bin/env python3
"""Silero clips for strings the generated Word-families / Cognates pages speak (morphology.json)
that the course does not: lemmas, family members, cognates and their example sentences.

  .local/silero-tts/venv/bin/python apple/scripts/make_morphology_audio.py STRINGS.txt [--plan]

STRINGS.txt: one stress-marked Russian string per line (# comments allowed). Strings whose key
already has a clip in Audio/index.json are skipped. New clips are written as morph-<translit>-<hash>.m4a
through the shared pipeline (audio_store.Silero + encode, restore fields recorded) and APPENDED to
index.json / sources.json, tagged "set": "morphology"; both files are re-read right before writing
and no existing entry is changed. Generated WAVs are cached under .local/drussky-audio/morphology/.
"""
import hashlib
import json
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
REPO = APPLE.parent
sys.path.insert(0, str(APPLE / "scripts"))
from audio_store import Silero, encode, restore_fields, silero_fields  # noqa: E402
from make_audio import AUDIO, SYNTH_FINAL, SYNTH_OVERRIDES, silero_text, slug  # noqa: E402
from validate_course import audio_key  # noqa: E402

SCRATCH = REPO / ".local/drussky-audio/morphology"
PROVIDER = "Silero v5_6_ru, voice xenia (CC BY-NC-SA 4.0)"


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    lines = [l.strip() for l in Path(args[0]).read_text().splitlines() if l.strip() and not l.startswith("#")]
    wanted = {}
    for s in lines:
        wanted.setdefault(audio_key(s), s)  # a rising question is its own clip ("…?")
    index = json.loads((AUDIO / "index.json").read_text())
    todo = {k: s for k, s in wanted.items() if k not in index or not (AUDIO / index[k]).is_file()}
    print(f"{len(wanted)} keys, {len(wanted) - len(todo)} already have a clip, {len(todo)} to generate", flush=True)
    if "--plan" in sys.argv:
        for k, s in todo.items():
            print("  gen", repr(s), "->", SYNTH_FINAL.get(k) or silero_text(SYNTH_OVERRIDES.get(k, s)))
        return
    (SCRATCH / "wav").mkdir(parents=True, exist_ok=True)
    silero = Silero(threads=4)
    new_index, new_sources = {}, {}
    for n, (k, text) in enumerate(todo.items(), 1):
        name = "morph-" + slug(k)
        synth = SYNTH_FINAL.get(k) or silero_text(SYNTH_OVERRIDES.get(k, text))
        wav = SCRATCH / "wav" / f"{name}-{hashlib.sha1(synth.encode()).hexdigest()[:6]}.wav"
        if not wav.exists():
            data, frames = silero.wav(synth, "xenia")
            wav.with_suffix(".tmp").write_bytes(data)
            wav.with_suffix(".tmp").replace(wav)
            print(f"[{n}/{len(todo)}] {synth} {frames / 24000:.2f}s", flush=True)
        encode(wav, AUDIO / f"{name}.m4a", SCRATCH / "level")
        new_index[k] = f"{name}.m4a"
        new_sources[f"{name}.m4a"] = {"key": k, "kind": "ai_generated_speech", "set": "morphology", "provider": PROVIDER,
                                      "input": synth, **restore_fields(wav, synth=silero_fields(synth, "xenia"))}
    # Append only: other tools own the rest of these files.
    index = json.loads((AUDIO / "index.json").read_text())
    sources = json.loads((AUDIO / "sources.json").read_text())
    added = {k: f for k, f in new_index.items() if k not in index}
    index.update(added)
    sources.update({f: e for f, e in new_sources.items() if f not in sources and f in added.values()})
    (AUDIO / "index.json").write_text(json.dumps(dict(sorted(index.items())), ensure_ascii=False, indent=1) + "\n")
    (AUDIO / "sources.json").write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
    print(f"appended {len(added)} entries ({len(new_index) - len(added)} keys appeared meanwhile and were left alone)")


if __name__ == "__main__":
    main()
