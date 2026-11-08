#!/usr/bin/env python3
"""Silero clips for strings the generated Word-families / Cognates pages speak (morphology.json)
that the course does not: lemmas, family members, cognates and their example sentences.

  .local/silero-tts/venv/bin/python apple/scripts/make_morphology_audio.py STRINGS.txt [--plan] [--set=shelves] [--voice=xenia|aidar|baya]

STRINGS.txt: one stress-marked Russian string per line (# comments allowed). Strings whose key
already has a clip in Audio/index.json are skipped. New clips are written as morph-<translit>-<hash>.m4a
through the shared pipeline (audio_store.Silero + encode, restore fields recorded) and APPENDED to
index.json / sources.json, tagged "set": "morphology"; both files are re-read right before writing
and no existing entry is changed. Generated WAVs are cached under .local/drussky-audio/morphology/.
--voice picks the Silero speaker (default xenia, the course voice); a clip in another voice is
named <prefix><voice>-<slug>.m4a. `generate()` is the function other drivers call
(make_listening_audio.py), with `variants=True` to get the entries back instead of writing index.json.
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
VOICES = ("xenia", "aidar", "baya")
_SILERO = None


def silero():
    """One model for the process: torch refuses a second load once its threads are set."""
    global _SILERO
    if _SILERO is None:
        _SILERO = Silero(threads=4)
    return _SILERO


def provider(voice):
    return f"Silero v5_6_ru, voice {voice} (CC BY-NC-SA 4.0)"


def generate(lines, voice="xenia", tag="morphology", plan=False, variants=False):
    """Synthesise every line (deduplicated by audio key) that has no clip yet.

    Primaries (variants=False): clips whose key is not in index.json are made and appended to
    index.json / sources.json. Variants (variants=True): every line is made in `voice` whether or
    not its key has a primary, nothing is written to index.json or sources.json, and the result is
    {key: (file, sources entry)} for the caller to file (e.g. in variants-listening.json).
    """
    assert voice in VOICES, voice
    prefix = "morph-" if tag == "morphology" else tag + "-"
    if voice != "xenia" or tag == "listening":
        prefix += voice + "-"
    wanted = {}
    for s in lines:
        s = s.strip()
        if s and not s.startswith("#"):
            wanted.setdefault(audio_key(s), s)  # a rising question is its own clip ("…?")
    index = json.loads((AUDIO / "index.json").read_text())
    if variants:
        todo = {k: s for k, s in wanted.items() if not (AUDIO / f"{prefix}{slug(k)}.m4a").is_file()}
    else:
        todo = {k: s for k, s in wanted.items() if k not in index or not (AUDIO / index[k]).is_file()}
    print(f"{voice}: {len(wanted)} keys, {len(wanted) - len(todo)} already have a clip, {len(todo)} to generate", flush=True)
    if plan:
        for k, s in todo.items():
            print("  gen", repr(s), "->", SYNTH_FINAL.get(k) or silero_text(SYNTH_OVERRIDES.get(k, s)))
        return {}
    (SCRATCH / "wav").mkdir(parents=True, exist_ok=True)
    new_index, new_sources = {}, {}
    for n, (k, text) in enumerate(todo.items(), 1):
        name = prefix + slug(k)
        synth = SYNTH_FINAL.get(k) or silero_text(SYNTH_OVERRIDES.get(k, text))
        wav = SCRATCH / "wav" / f"{name}-{hashlib.sha1(synth.encode()).hexdigest()[:6]}.wav"
        if not wav.exists():
            data, frames = silero().wav(synth, voice)
            wav.with_suffix(".tmp").write_bytes(data)
            wav.with_suffix(".tmp").replace(wav)
            print(f"[{n}/{len(todo)}] {voice} {synth} {frames / 24000:.2f}s", flush=True)
        encode(wav, AUDIO / f"{name}.m4a", SCRATCH / "level")
        new_index[k] = f"{name}.m4a"
        new_sources[f"{name}.m4a"] = {"key": k, "kind": "ai_generated_speech", "set": tag, "provider": provider(voice),
                                      "input": synth, "voice": voice, **restore_fields(wav, synth=silero_fields(synth, voice))}
    if variants:
        return {k: (f, new_sources[f]) for k, f in new_index.items()}
    # Append only: other tools own the rest of these files.
    index = json.loads((AUDIO / "index.json").read_text())
    sources = json.loads((AUDIO / "sources.json").read_text())
    added = {k: f for k, f in new_index.items() if k not in index}
    index.update(added)
    sources.update({f: e for f, e in new_sources.items() if f not in sources and f in added.values()})
    (AUDIO / "index.json").write_text(json.dumps(dict(sorted(index.items())), ensure_ascii=False, indent=1) + "\n")
    (AUDIO / "sources.json").write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
    print(f"appended {len(added)} entries ({len(new_index) - len(added)} keys appeared meanwhile and were left alone)")
    return {k: (f, new_sources[f]) for k, f in added.items()}


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    # --set NAME tags the clips (sources.json "set") and prefixes their files; default "morphology"/"morph-".
    tag = next((a.split("=", 1)[1] for a in sys.argv[1:] if a.startswith("--set=")), "morphology")
    voice = next((a.split("=", 1)[1] for a in sys.argv[1:] if a.startswith("--voice=")), "xenia")
    lines = Path(args[0]).read_text().splitlines()
    generate(lines, voice=voice, tag=tag, plan="--plan" in sys.argv)


if __name__ == "__main__":
    main()
