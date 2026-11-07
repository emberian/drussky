#!/usr/bin/env python3
"""Build Drussky/Resources/Audio from course.json.

Reuses the rufren foundation recordings where the spoken text matches, and
synthesizes the rest with the pinned local Silero model (voice xenia) using the
course's explicit stress marks. Run with the Silero venv python:

  .local/silero-tts/venv/bin/python apple/scripts/make_audio.py [--plan]

--plan only prints what would be reused/generated. Existing generated WAVs in the
scratch directory are reused, so the script resumes after interruption.
"""
import hashlib
import re
import json
import subprocess
import sys
import wave
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
REPO = APPLE.parent
sys.path.insert(0, str(APPLE / "scripts"))
from validate_course import key, spoken_strings  # noqa: E402
from audio_level import decode, level, to_wav  # noqa: E402

COURSE = APPLE / "Drussky/Resources/course.json"
AUDIO = APPLE / "Drussky/Resources/Audio"
SCRATCH = REPO / ".local/drussky-audio"
FOUNDATION = REPO / "crates/rufren-app/assets/foundations"
# Chosen human (or macOS) takes: key -> {orig, kind, title, source_page, license, author, provider}
HUMAN = APPLE / "scripts/human_audio.json"
FOUNDATION_TEXT = REPO / "crates/rufren-content/content/foundations-v1.json"
# research/ALPHABET_AUDIO_SOURCE_REVIEW.md: these two have review problems.
DO_NOT_REUSE = {"foundation-a11-u-model", "foundation-a17-my-n"}
# Isolated sounds where plain text gives Silero too little to say clearly.
SYNTH_OVERRIDES = {"у": "+у", "е": "+е"}
# Exact Silero inputs chosen by an ASR spot check where the default transform did worse.
SYNTH_FINAL = {"всё": "вс+ё", "девять": "д+евять", "у": "+у", "сыр": "с+ыр", "ваше окно": "в+аше окн+о"}
# Foundation syllables that were synthesized without stress; regenerate with it.
FORCE_GENERATE = {"ва", "га", "ка", "та", "са", "за", "ха", "ца", "ща"}
VOWELS = set("аеёиоуыэюяАЕЁИОУЫЭЮЯ")
TRANSLIT = dict(zip("абвгдеёжзийклмнопрстуфхцчшщъыьэюя",
                    ["a", "b", "v", "g", "d", "e", "yo", "zh", "z", "i", "y", "k", "l", "m", "n", "o", "p", "r", "s", "t",
                     "u", "f", "kh", "ts", "ch", "sh", "shch", "", "y", "", "e", "yu", "ya"]))


def slug(k):
    base = "".join(TRANSLIT.get(c, "-" if c == " " else "") for c in k)[:40].strip("-") or "x"
    return f"{base}-{hashlib.sha1(k.encode()).hexdigest()[:6]}"


# Unstressed monosyllables in running speech. Everything else gets an explicit
# stress: with put_accent off, Silero otherwise reduces кот to "kat", чай to "chey".
CLITICS = {"не", "и", "а", "у", "в", "на", "по", "но", "за", "к", "с", "о", "ни", "же", "ли", "бы", "то", "из", "от", "до"}


def silero_text(text):
    """Authored acute -> Silero '+' before the vowel; ё and stressed monosyllables get '+'."""
    out = []
    for c in text:
        if c == "\u0301":
            v = out.pop()
            assert v in VOWELS, text
            out += ["+", v]
        else:
            out.append(c)
    s = " ".join("".join(out).split())  # passage line breaks become spaces
    s = re.sub(r"(?<=\w)-(?=\w)", "", s)  # по-русски -> порусски (Silero misreads the hyphen)
    parts = s.split(" ")
    words = []
    for w in parts:
        bare = re.sub(r"[^\w+]", "", w).lower()
        if "+" not in w:
            if "ё" in bare:
                i = max(w.find("ё"), w.find("Ё"))
                w = w[:i] + "+" + w[i:]
            elif sum(c in VOWELS for c in bare) == 1 and (bare not in CLITICS or len(parts) == 1):
                i = next(i for i, c in enumerate(w) if c in VOWELS)
                w = w[:i] + "+" + w[i:]
        words.append(w)
    s = " ".join(words)
    # A closing full stop gives isolated words a natural falling cadence instead of a clipped one.
    return s if s[-1] in ".?!…" else s + "."


def plan():
    course = json.loads(COURSE.read_text())
    wanted = {}
    for s in spoken_strings(course):
        wanted.setdefault(key(s), s)
    foundation = {}
    manifest = json.loads((FOUNDATION / "manifest.json").read_text())
    kinds = {a["id"]: a.get("provenance", a.get("source_response", {}).get("provenance", {})).get("kind") for a in manifest["assets"]}
    for a in json.loads(FOUNDATION_TEXT.read_text())["assets"]:
        if a["id"] in DO_NOT_REUSE or key(a["spoken"]) in FORCE_GENERATE:
            continue
        k = key(a["spoken"])
        # prefer human recordings when two foundation assets share a key
        if k not in foundation or kinds.get(a["id"]) == "human_recording":
            foundation[k] = a["id"]
    human = json.loads(HUMAN.read_text()) if HUMAN.exists() else {}
    human = {k: v for k, v in human.items() if k in wanted}
    reuse = {k: foundation[k] for k in wanted if k in foundation and k not in human}
    generate = {k: wanted[k] for k in wanted if k not in foundation and k not in human}
    return wanted, human, reuse, generate, kinds, manifest


def to_m4a(src, m4a):
    """Trim silence, level to a common loudness, encode AAC."""
    pcm, _ = level(decode(src))
    leveled = SCRATCH / "level" / (m4a.stem + ".wav")
    leveled.parent.mkdir(parents=True, exist_ok=True)
    leveled.write_bytes(to_wav(pcm))
    tmp = m4a.with_suffix(".tmp.m4a")
    subprocess.run(["afconvert", "-f", "m4af", "-d", "aac", "-b", "64000", str(leveled), str(tmp)], check=True)
    tmp.replace(m4a)


def main():
    wanted, human, reuse, generate, kinds, manifest = plan()
    print(f"{len(wanted)} keys: chosen human/macOS {len(human)}, reuse {len(reuse)} ({sum(kinds.get(v) == 'human_recording' for v in reuse.values())} human), generate {len(generate)}")
    if "--plan" in sys.argv:
        for k, s in generate.items():
            print("  gen", repr(s), "->", SYNTH_FINAL.get(k) or silero_text(SYNTH_OVERRIDES.get(k, s)))
        return
    AUDIO.mkdir(parents=True, exist_ok=True)
    (SCRATCH / "wav").mkdir(parents=True, exist_ok=True)
    index, sources = {}, {}
    credits = {a["id"]: a.get("provenance", {}) for a in manifest["assets"]}

    for k, h in human.items():
        name = slug(k) + ".m4a"
        to_m4a(REPO / h["orig"], AUDIO / name)
        index[k] = name
        sources[name] = {"key": k, **{f: h[f] for f in ("kind", "title", "author", "credit", "license", "source_page", "provider") if h.get(f)}}

    for k, fid in reuse.items():
        name = slug(k) + ".m4a"
        to_m4a(FOUNDATION / f"{fid}.wav", AUDIO / name)
        index[k] = name
        p = credits.get(fid, {})
        sources[name] = {"key": k, "from": f"crates/rufren-app/assets/foundations/{fid}.wav", "kind": kinds.get(fid),
                         **({"credit": p.get("credit"), "license": p.get("license"), "source_page": p.get("source_page")} if kinds.get(fid) == "human_recording" else {})}

    loaded = None
    for n, (k, text) in enumerate(generate.items(), 1):
        name = slug(k)
        synth = SYNTH_FINAL.get(k) or silero_text(SYNTH_OVERRIDES.get(k, text))
        wav = SCRATCH / "wav" / f"{name}-{hashlib.sha1(synth.encode()).hexdigest()[:6]}.wav"
        if not wav.exists():
            if loaded is None:
                sys.path.insert(0, str(REPO / "scripts/speech"))
                import silero_gateway as gateway
                from silero_worker import encode_wav
                loaded = gateway.load_model()
                loaded.torch.set_num_threads(4)
                flags = dict(gateway.EXPLICIT_FLAGS)
            # Long texts (passages) are synthesized sentence by sentence and joined with short pauses.
            parts = [synth] if len(synth) <= 160 else [x for x in re.split(r"(?<=[.!?…»])\s+", synth) if x.strip()]
            samples = []
            for i, part in enumerate(parts):
                if i:
                    samples += [0.0] * int(24000 * 0.35)
                samples += loaded.samples(part, flags)
            data, frames, signals = encode_wav(samples)
            assert signals["clipped_samples"] == 0, (text, signals)
            wav.with_suffix(".tmp").write_bytes(data)
            wav.with_suffix(".tmp").replace(wav)
            print(f"[{n}/{len(generate)}] {synth} {frames / 24000:.2f}s", flush=True)
        to_m4a(wav, AUDIO / f"{name}.m4a")
        index[k] = f"{name}.m4a"
        sources[f"{name}.m4a"] = {"key": k, "kind": "ai_generated_speech", "provider": "Silero v5_6_ru, voice xenia (CC BY-NC-SA 4.0)", "input": synth}

    # Extra-speaker clips are built by make_variants.py; keep their provenance.
    variants_path = AUDIO / "variants.json"
    extra = {f for fs in json.loads(variants_path.read_text()).values() for f in fs} if variants_path.exists() else set()
    if extra:
        old = json.loads((AUDIO / "sources.json").read_text())
        sources.update({f: old[f] for f in extra if f in old})
    (AUDIO / "index.json").write_text(json.dumps(dict(sorted(index.items())), ensure_ascii=False, indent=1) + "\n")
    (AUDIO / "sources.json").write_text(json.dumps(dict(sorted(sources.items())), ensure_ascii=False, indent=1) + "\n")
    stale = {p.name for p in AUDIO.glob("*.m4a")} - set(index.values()) - extra
    if stale:
        print("stale files not in index (left in place):", sorted(stale))
    print(f"wrote {len(index)} entries")


if __name__ == "__main__":
    main()
