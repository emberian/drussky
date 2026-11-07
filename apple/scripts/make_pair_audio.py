#!/usr/bin/env python3
"""Audio for the sound-pairs pages (Drussky/Resources/pairs.json).

Every pair word gets as many distinct human voices as Wikimedia Commons offers (Shtooka "Ru-…"
and Lingua Libre "LL-Q7737 (rus)-…" uploads, CC BY / BY-SA / PD), and local Silero voices when
there are fewer than two humans. Words in stress pairs (за́мок / замо́к) only take human clips
whose title pins the stress; otherwise Silero with explicit stress.

Keys are the pair word lowercased with its stress mark kept (`мы́ло`), so the two members of
a stress pair never share a clip. Writes, without touching index.json / sources.json /
variants.json:
  Audio/index-pairs.json     key -> primary clip
  Audio/variants-pairs.json  key -> further speakers
  Audio/sources-pairs.json   clip -> provenance

Stages (each resumable; caches under .local/drussky-audio/pairs/ in the main checkout):
  python3 apple/scripts/make_pair_audio.py --fetch                         # Commons metadata + originals
  .local/qwen3-asr/venv/bin/python -I apple/scripts/make_pair_audio.py --asr  # transcript check
  .local/silero-tts/venv/bin/python -I apple/scripts/make_pair_audio.py --build
"""
import hashlib
import html
import json
import os
import re
import sys
import unicodedata
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
MAIN = Path("/Users/ember/dev/rufren")          # model weights and Commons caches live here
WORK = MAIN / ".local/drussky-audio/pairs"
COMMONS = MAIN / ".local/drussky-audio"
AUDIO = APPLE / "Drussky/Resources/Audio"
PAIRS = APPLE / "Drussky/Resources/pairs.json"
ACUTE = "́"
SILERO_VOICES = ["xenia", "aidar", "baya"]
# Commons titles whose stress is pinned by an IPA or accent in the title.
STRESS_TITLES = {
    "за́мок": ["File:Ru-замок (ˈzamɐk).ogg"],
    "замо́к": ["File:Ru-замок (zaˈmok).ogg"],
    "му́ка": ["File:Ru-му́ка.ogg"],
    "мука́": ["File:Ru-мука́.ogg"],
    "дома́": ["File:Ru-дома́.ogg"],
}
# A yes/no question and a statement by one speaker, for the pitch tracker's tests, the intonation
# QA page, and as read-aloud models for these sentences. (Silero's "Это кот?" comes out nearly
# flat, so synthetic speech is no model for question intonation.)
INTONATION = {
    "intonation-vy-lyubite-muzyku.m4a": ("вы любите музыку", "File:LL-Q7737 (rus)-DoctorandusManhattan-вы любите музыку ?.wav"),
    "intonation-ya-goloden.m4a": ("я голоден", "File:LL-Q7737 (rus)-DoctorandusManhattan-я голоден.wav"),
}
ALIAS = {"Pogrebnoj-Alexandroff": "Aleksey Pogrebnoj-Alexandroff",
         "Сергей Сахно, Vion Nicolas": "Shtooka", "Sergueï Sakhno": "Shtooka", "The Shtooka Project": "Shtooka"}


def nfc(s):
    return unicodedata.normalize("NFC", s)


def pair_key(ru):
    """RussianText.pairKey: lowercase, punctuation-free, stress mark kept."""
    s = nfc(ru).lower()
    s = "".join(" " if c in ".,?!—–-\"«»:;…()" else c for c in s)
    return " ".join(s.split())


def plain(s):
    return pair_key(s).replace(ACUTE, "")


def words():
    """pair key -> (authored form, needs pinned stress)."""
    data = json.loads(PAIRS.read_text())
    out = {}
    for p in data["pairs"]:
        forms = [w["ru"] for w in p["words"]]
        pinned = len({plain(f) for f in forms}) == 1
        for f in forms:
            k = pair_key(f)
            out[k] = (f, out.get(k, (f, False))[1] or pinned)
    return out


def speaker_of(title, meta):
    if title.startswith("File:LL-Q7737 (rus)-"):
        return title[len("File:LL-Q7737 (rus)-"):].partition("-")[0]
    artist = " ".join(html.unescape(re.sub(r"<[^>]+>", " ", (meta or {}).get("artist") or "")).split())
    return ALIAS.get(artist, artist or "Shtooka")


def candidates():
    titles = json.loads((COMMONS / "variants/titles.json").read_text())
    want = words()
    cand = {k: [] for k in want}
    for k, (form, pinned) in want.items():
        if pinned:
            cand[k] = list(STRESS_TITLES.get(k, []))
    loose = {plain(k): k for k, (_, pinned) in want.items() if not pinned}
    for t in titles["LL-Q7737 (rus)"]:
        w = t[len("File:LL-Q7737 (rus)-"):].rsplit(".", 1)[0].partition("-")[2]
        if (k := loose.get(nfc(w).lower())):
            cand[k].append(t)
    for t in titles["Ru-"]:
        if not re.search(r"\.(ogg|oga|wav|flac|mp3|opus)$", t, re.I):
            continue
        w = t[len("File:Ru-"):].rsplit(".", 1)[0]
        w = re.sub(r"(-\d+| \([^)]*\)|-(male|female|m|f))$", "", w)
        if ACUTE in w:
            continue
        if (k := loose.get(nfc(w).lower())):
            cand[k].append(t)
    return cand


def orig_name(title):
    return hashlib.sha1(title.encode()).hexdigest()[:12] + "." + title.rsplit(".", 1)[1].lower()


def find_orig(title):
    for d in (WORK / "orig", COMMONS / "human/orig", COMMONS / "variants/orig"):
        if (d / orig_name(title)).exists():
            return d / orig_name(title)
    return None


def fetch():
    sys.path.insert(0, str(COMMONS / "human"))
    from commons import info, download
    cand = candidates()
    cand["(intonation)"] = [t for _, t in INTONATION.values()]
    (WORK / "orig").mkdir(parents=True, exist_ok=True)
    metap = WORK / "meta.json"
    meta = json.loads(metap.read_text()) if metap.exists() else {}
    for f in (COMMONS / "human/meta.json", COMMONS / "variants/meta.json"):
        old = json.loads(f.read_text())
        meta.update({t: old[t] for v in cand.values() for t in v if t in old and t not in meta})
    need = sorted({t for v in cand.values() for t in v} - set(meta))
    if need:
        meta.update(info(need))
    metap.write_text(json.dumps(meta, ensure_ascii=False, indent=0))
    (WORK / "cand.json").write_text(json.dumps(cand, ensure_ascii=False, indent=1))
    for t in sorted({t for v in cand.values() for t in v}):
        m = meta.get(t)
        if not m or find_orig(t):
            continue
        p = WORK / "orig" / orig_name(t)
        download(m["url"], p)
        assert hashlib.sha1(p.read_bytes()).hexdigest() == m["sha1"], t
        print("got", t, flush=True)
    print(sum(map(len, cand.values())), "candidate takes for", len(cand), "words")


def asr():
    sys.path.insert(0, str(APPLE / "scripts"))
    sys.path.insert(0, str(MAIN / "scripts/speech"))
    os.environ.update(HF_HUB_OFFLINE="1", TRANSFORMERS_OFFLINE="1", OMP_NUM_THREADS="2")
    from audio_level import decode, level
    from asr_model import MODEL_PATH, verify_install
    import numpy as np
    from scipy.signal import resample_poly
    verify_install()
    import mlx.core as mx
    mx.set_memory_limit(8 * 1024**3); mx.set_cache_limit(1024**3)
    from mlx_audio.stt import load
    model = load(str(MODEL_PATH))
    cand = json.loads((WORK / "cand.json").read_text())
    outp = WORK / "heard.json"
    heard = json.loads(outp.read_text()) if outp.exists() else {}
    for k, titles in cand.items():
        for t in titles:
            src = find_orig(t)
            if t in heard or src is None:
                continue
            try:
                pcm, info = level(decode(src))
            except Exception as e:  # noqa: BLE001
                heard[t] = {"error": str(e)}
                continue
            x = np.frombuffer(pcm.tobytes(), dtype="<i2").astype(np.float32) / 32768.0
            x = np.concatenate([np.zeros(4000, np.float32), resample_poly(x, 2, 3).astype(np.float32), np.zeros(4000, np.float32)])
            text = model.generate(x, language="Russian", temperature=0.0, max_tokens=64, batch_size=1, verbose=False,
                                  system_prompt=None, hotwords=None).text
            heard[t] = {"heard": text, **info}
            print(k, "|", t, "=>", text, info, flush=True)
    outp.write_text(json.dumps(heard, ensure_ascii=False, indent=1))


def norm_heard(s):
    return " ".join(re.sub(r"[^\w\s]", " ", s.lower().replace("ё", "е")).split())


def distance(a, b):
    row = list(range(len(b) + 1))
    for i, x in enumerate(a, 1):
        prev, row[0] = row[0], i
        for j, y in enumerate(b, 1):
            prev, row[j] = row[j], min(row[j] + 1, row[j - 1] + 1, prev + (x != y))
    return row[-1]


def usable(k, t, h):
    """A take is kept when it is clean, unclipped, and the recognizer heard the word or a near
    neighbour. ASR on isolated words merges exactly these contrasts (пыл → был, люк → лук,
    кот → код), so the Commons title is the label and ASR only screens out wrong files."""
    if not h or "error" in h:
        return False
    ok_text = distance(norm_heard(h["heard"]), norm_heard(plain(k))) <= 2
    return ok_text and h["noise_floor_db"] <= -42 and h["peak_db_before"] < -0.2 and 0.15 <= h["speech_seconds"] <= 2.5


def silero_input(form):
    s = form.replace("ё", "+ё")
    out = []
    for c in nfc(s):
        if c == ACUTE:
            v = out.pop(); out += ["+", v]
        else:
            out.append(c)
    s = "".join(out)
    if "+" not in s:
        i = next(i for i, c in enumerate(s) if c.lower() in "аеёиоуыэюя")
        s = s[:i] + "+" + s[i:]
    return s + "."


def translit(s, n):
    sys.path.insert(0, str(APPLE / "scripts"))
    from make_audio import TRANSLIT
    s = "".join(TRANSLIT.get(c, c if c.isascii() and c.isalnum() else "-") for c in s.lower().replace(ACUTE, ""))
    return re.sub(r"-+", "-", s)[:n].strip("-") or "x"


def build():
    sys.path.insert(0, str(APPLE / "scripts"))
    from make_audio import to_m4a
    from audio_store import Silero, commons_url, restore_fields, silero_fields
    want = words()
    cand = json.loads((WORK / "cand.json").read_text())
    meta = json.loads((WORK / "meta.json").read_text())
    heard = json.loads((WORK / "heard.json").read_text())
    main_index = json.loads((AUDIO / "index.json").read_text())
    main_sources = json.loads((AUDIO / "sources.json").read_text())
    main_variants = json.loads((AUDIO / "variants.json").read_text())
    used_titles = {v.get("title") for v in main_sources.values()}
    index, variants, sources = {}, {}, {}
    silero = Silero(threads=2)
    for k, (form, pinned) in sorted(want.items()):
        # Voices the main audio already has for this word (only when stress cannot be confused).
        existing = []
        if not pinned and plain(k) in main_index:
            for f in [main_index[plain(k)]] + main_variants.get(plain(k), []):
                if main_sources.get(f, {}).get("kind") == "human_recording":
                    existing.append(f)
        speakers = {speaker_of("File:" + (main_sources[f].get("title") or ""), {"artist": main_sources[f].get("author")})
                    for f in existing}
        files = []
        for t in cand.get(k, []):
            if t[5:] in used_titles:
                continue
            if not (t in STRESS_TITLES.get(k, []) or usable(k, t, heard.get(t))):
                continue
            m = meta.get(t, {})
            spk = speaker_of(t, m)
            if spk in speakers or find_orig(t) is None:
                continue
            speakers.add(spk)
            name = f"pair-{translit(k, 24)}-{translit(spk, 16)}-{hashlib.sha1(t.encode()).hexdigest()[:6]}.m4a"
            if not (AUDIO / name).exists():
                to_m4a(find_orig(t), AUDIO / name)
            sources[name] = {"key": k, "kind": "human_recording", "title": t[5:], "author": spk,
                             "license": m.get("license"), "source_page": m.get("page"), "speaker": spk,
                             **restore_fields(find_orig(t), url=commons_url(t))}
            files.append(name)
        humans = len(existing) + len(files)
        for voice in SILERO_VOICES[: max(0, 2 - humans) + (1 if humans == 0 else 0)]:
            synth = silero_input(form)
            name = f"pair-{translit(k, 24)}-silero-{voice}-{hashlib.sha1((voice + synth).encode()).hexdigest()[:6]}.m4a"
            if not (AUDIO / name).exists():
                wav = WORK / "wav" / (name[:-4] + ".wav")
                wav.parent.mkdir(parents=True, exist_ok=True)
                if not wav.exists():
                    data, _ = silero.wav(synth, voice)
                    wav.write_bytes(data)
                    print("silero", voice, synth, flush=True)
                to_m4a(wav, AUDIO / name)
            sources[name] = {"key": k, "kind": "ai_generated_speech", "provider": f"Silero v5_6_ru, voice {voice} (CC BY-NC-SA 4.0)", "input": synth,
                             **restore_fields(WORK / "wav" / (name[:-4] + ".wav"), synth=silero_fields(synth, voice))}
            files.append(name)
        if not existing and files:
            index[k] = files[0]
            files = files[1:]
        elif not existing:
            print("NO AUDIO", k)
        if files:
            variants[k] = files
        print(f"{k}: {humans} human ({len(existing)} already bundled), {len(files) + (k in index)} new clips")
    intonation(index, sources)
    dump(AUDIO / "index-pairs.json", index)
    dump(AUDIO / "variants-pairs.json", variants)
    dump(AUDIO / "sources-pairs.json", sources)
    stale = {p.name for p in AUDIO.glob("pair-*.m4a")} - set(sources)
    if stale:
        print("stale pair clips (left in place):", sorted(stale))


def dump(p, d):
    p.write_text(json.dumps(dict(sorted(d.items())), ensure_ascii=False, indent=1) + "\n")


def intonation(index, sources):
    """Copies the INTONATION clips in and files them as read-aloud models for their sentences."""
    sys.path.insert(0, str(APPLE / "scripts"))
    from make_audio import to_m4a
    from audio_store import commons_url, restore_fields
    meta = json.loads((WORK / "meta.json").read_text())
    for name, (key, title) in INTONATION.items():
        if not (AUDIO / name).exists():
            to_m4a(find_orig(title), AUDIO / name)
        m = meta[title]
        spk = speaker_of(title, m)
        index[key] = name
        sources[name] = {"key": key, "kind": "human_recording", "title": title[5:], "author": spk,
                         "license": m.get("license"), "source_page": m.get("page"), "speaker": spk,
                         **restore_fields(find_orig(title), url=commons_url(title))}


if __name__ == "__main__":
    {"--fetch": fetch, "--asr": asr, "--build": build}[sys.argv[1]]()
