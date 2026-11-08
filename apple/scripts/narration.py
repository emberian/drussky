#!/usr/bin/env python3
"""Human narration for library texts: find the excerpt inside a public-domain recording, time every
word, cut the clip, and register it so the app can read along and `audio.py restore` can rebuild it.

  .local/drussky-librivox/venv/bin/python apple/scripts/narration.py align <text-id>...   # word times in the source recording
  python3 apple/scripts/narration.py build <text-id>...                                   # cut, level, encode, register
  python3 apple/scripts/narration.py check                                                # every words.json matches its text

Sources: scripts/narration_sources.json (recording URL, reader, licence per text id). Originals are
cached in .local/drussky-librivox/orig/ (download: audio_store.fetch, polite and sequential).

align   runs the MMS forced aligner (ctc-forced-aligner, ONNX, 16 kHz) over the whole recording with
        a <star> wildcard between words, so an excerpt is located inside a longer chapter and read
        digits or asides the text does not spell out are absorbed. Output: .local/drussky-librivox/align/<id>.json
        with [word, start, end, score] in seconds of the original recording.
build   cuts [first word - PAD_BEFORE, last word + PAD_AFTER] from the original (audio_store.cut), then
        trims/levels/encodes exactly like every other clip (audio_store.encode); word times are shifted
        into the finished clip and written to Audio/narr-<id>.words.json; Audio/index-narration.json,
        Audio/sources-narration.json and library.json get their entries. index.json/sources.json are
        never touched.

Word timing JSON (Audio/narr-<id>.words.json):
  {"version": 1, "text": "<library id>", "file": "narr-<id>.m4a", "duration": s, "reader": "...",
   "words": [[word, start, end], ...]}
`words` are, in order, every whitespace token of the text whose bare form (punctuation stripped,
stress marks removed) contains a letter -- the same rule the app applies (Narration.swift), so the
app zips its own tokens with this list. A token with no letter (digits, ***, a dash) has no entry.
"""
import argparse
import json
import os
import re
import sys
import unicodedata
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
REPO = APPLE.parent
sys.path.insert(0, str(APPLE / "scripts"))
import audio_store as store  # noqa: E402

AUDIO = APPLE / "Drussky/Resources/Audio"
LIBRARY = APPLE / "Drussky/Resources/library.json"
SOURCES = json.loads((APPLE / "scripts/narration_sources.json").read_text())
# The cache lives beside the main checkout even when this runs from a git worktree under .claude/worktrees/.
MAIN = Path(*REPO.parts[:REPO.parts.index(".claude")]) if ".claude" in REPO.parts else REPO
LOCAL = Path(os.environ.get("DRUSSKY_LOCAL", MAIN / ".local")) / "drussky-librivox"
ALIGN_DIR = LOCAL / "align"
MODEL = LOCAL / "models/mms-fa.onnx"
SET = "narration"
PAD_BEFORE, PAD_AFTER = 0.5, 0.9
PUNCT = ".,?!—–-:;…«»\"()*'’“”‘[]"
PIPELINE = f"audio_store.cut (ffmpeg, recorded offsets) -> {store.PIPELINE}"


def strip_stress(s):
    return "".join(c for c in unicodedata.normalize("NFD", s) if c != "́")


def bare(token):
    """The app's PassageExercise.bare: outer punctuation off, stress marks off."""
    return strip_stress(token.strip(PUNCT))


def timed_tokens(text):
    """(token index, bare word) for every whitespace token that has a letter -- the words that get times."""
    out = []
    for i, tok in enumerate(text.split()):
        b = bare(tok)
        if re.search(r"[^\W\d_]", b):
            out.append((i, b))
    return out


def library():
    return json.loads(LIBRARY.read_text())


def text_entry(lib, text_id):
    for t in lib["texts"]:
        if t["id"] == text_id:
            return t
    raise SystemExit(f"{text_id}: not in library.json")


def original(text_id):
    src = SOURCES[text_id]
    p = LOCAL.parent / Path(src["orig"]).relative_to(".local")
    if not p.exists():
        print(f"  fetching {src['url']}")
        store.fetch(src["url"], p)
    return p


# ---------------------------------------------------------------- align

def align_words(wav16, words, breaks=()):
    """Forced alignment of `words` (Cyrillic) to a 16 kHz mono wav; [start, end, score] per word, in seconds.

    The text is one contiguous run inside the recording: a <star> wildcard before the first word
    absorbs the announcement and anything read before the excerpt, one after the last word the rest
    of the chapter, and one before each word in `breaks` whatever the reader says for a token the
    text does not spell out in letters (a year in digits, ***). No wildcards between other words:
    a free wildcard everywhere lets short words wander to look-alike sounds minutes away.
    A word's span is its first to last letter spike (no padding into the pauses)."""
    import numpy as np
    from ctc_forced_aligner import (Alignment, ensure_onnx_model, MODEL_URL, generate_emissions, get_alignments,
                                    get_spans, get_uroman_tokens, load_audio, text_normalize)
    ensure_onnx_model(str(MODEL), MODEL_URL)
    aligner = Alignment(str(MODEL))
    waveform = load_audio(str(wav16))
    emissions, stride = generate_emissions(aligner.alignment_model, waveform, batch_size=4)
    norm = [text_normalize(w.lower(), "rus") for w in words]
    tokens = get_uroman_tokens(norm, "rus")
    for i, t in enumerate(tokens):
        if not t:  # nothing romanizable (should not happen for Cyrillic words): give it a vowel so it keeps a slot
            tokens[i] = "a"
    starred, word_at = ["<star>"], []
    for i, t in enumerate(tokens):
        if i in breaks and i > 0:
            starred.append("<star>")
        word_at.append(len(starred))
        starred.append(t)
    starred.append("<star>")
    segments, scores, blank = get_alignments(emissions, starred, aligner.alignment_tokenizer)
    spans = get_spans(starred, segments, blank)
    scores = np.asarray(scores).reshape(-1)
    out = []
    for k in word_at:
        letters = [s for s in spans[k] if s.label != blank]  # drop the half-pause padding get_spans adds
        a, b = letters[0].start, letters[-1].end
        out.append([round(a * stride / 1000, 3), round(b * stride / 1000, 3), round(float(scores[a:b].sum()), 2)])
    return tidy(words, out)


def tidy(words, times):
    """Make word spans usable for highlighting: a letter spike is a point, so every word gets at least
    60 ms and a short tail up to the next word; a word of one or two letters that landed alone
    seconds away from both neighbours (a preposition snapped to a look-alike sound) is put back
    just before the word that follows it."""
    n = len(times)
    for i, (w, t) in enumerate(zip(words, times)):
        if len(w) > 2 or n < 2:
            continue
        prev_gap = t[0] - times[i - 1][1] if i > 0 else None
        next_gap = times[i + 1][0] - t[1] if i + 1 < n else None
        stray_before = prev_gap is None or prev_gap > 1.5
        stray_after = next_gap is None or next_gap > 1.5
        if stray_before and stray_after and (next_gap is not None or prev_gap is not None):
            if next_gap is not None:
                times[i] = [round(times[i + 1][0] - 0.2, 3), round(times[i + 1][0] - 0.05, 3), t[2]]
            else:
                times[i] = [round(times[i - 1][1] + 0.05, 3), round(times[i - 1][1] + 0.2, 3), t[2]]
    for i, t in enumerate(times):
        nxt = times[i + 1][0] if i + 1 < n else t[1] + 0.15
        end = max(t[1], t[0] + 0.06)
        end = min(max(end, min(end + 0.06, nxt - 0.01)), max(nxt - 0.01, t[0] + 0.06))
        t[1] = round(end, 3)
    return times


def breaks_before(text):
    """Timed-word indices preceded by a token the reader voices differently from its spelling (digits, ***)."""
    out, k, pending = set(), 0, False
    for tok in text.split():
        b = bare(tok)
        if re.search(r"[^\W\d_]", b):
            if pending:
                out.add(k)
            pending = False
            k += 1
        elif re.search(r"[\d*]", tok):
            pending = True
    return out


def cmd_align(args):
    import subprocess
    import tempfile
    lib = library()
    ALIGN_DIR.mkdir(parents=True, exist_ok=True)
    for text_id in args.ids:
        t = text_entry(lib, text_id)
        orig = original(text_id)
        toks = timed_tokens(t["text"])
        words = [b for _, b in toks]
        with tempfile.TemporaryDirectory(prefix="drussky-narr-") as tmp:
            wav = Path(tmp) / "a.wav"
            subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", str(orig), "-ac", "1", "-ar", "16000", str(wav)], check=True)
            times = align_words(wav, words, breaks_before(t["text"]))
        total = float(subprocess.run(["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", str(orig)],
                                     check=True, capture_output=True, text=True).stdout.strip())
        out = {"text": text_id, "orig": SOURCES[text_id]["orig"], "orig_duration": round(total, 3),
               "words": [[w, s, e, sc] for (w, (s, e, sc)) in zip(words, times)]}
        (ALIGN_DIR / f"{text_id}.json").write_text(json.dumps(out, ensure_ascii=False, indent=0) + "\n")
        span = times[-1][1] - times[0][0]
        print(f"{text_id}: {len(words)} words, excerpt {times[0][0]:.1f}s–{times[-1][1]:.1f}s of {total:.0f}s "
              f"({len(t['text']) / max(span, 0.1):.1f} chars/s), mean score {sum(x[2] for x in times) / len(times):.2f}")
    return 0


# ---------------------------------------------------------------- build

def load_json(p, default):
    return json.loads(p.read_text()) if p.exists() else default


def dump(p, obj):
    p.write_text(json.dumps(obj, ensure_ascii=False, indent=1) + "\n")


def cmd_build(args):
    import tempfile
    lib = library()
    index = load_json(AUDIO / "index-narration.json", {})
    sources = load_json(AUDIO / "sources-narration.json", {})
    with tempfile.TemporaryDirectory(prefix="drussky-narr-") as tmp:
        for text_id in args.ids:
            t = text_entry(lib, text_id)
            src = SOURCES[text_id]
            aligned = json.loads((ALIGN_DIR / f"{text_id}.json").read_text())
            words = aligned["words"]
            if [w for w, *_ in words] != [b for _, b in timed_tokens(t["text"])]:
                raise SystemExit(f"{text_id}: alignment is for another version of the text; re-run align")
            orig = original(text_id)
            start = max(0.0, words[0][1] - PAD_BEFORE)
            end = min(aligned["orig_duration"], words[-1][2] + PAD_AFTER)
            m4a_name, words_name = f"narr-{text_id}.m4a", f"narr-{text_id}.words.json"
            cut_wav = Path(tmp) / f"{text_id}-cut.wav"
            store.cut(orig, start, end, cut_wav)
            info = store.encode(cut_wav, AUDIO / m4a_name, tmp)
            shift = start + info["start_seconds"]
            timed = [[w, round(max(0.0, s - shift), 2), round(max(0.0, e - shift), 2)] for w, s, e, _ in words]
            for i in range(len(timed) - 1):  # rounding to 10 ms must not let a word end after the next begins
                timed[i][2] = min(timed[i][2], timed[i + 1][1])
            timed[-1][2] = min(timed[-1][2], info["trimmed_seconds"])
            dump(AUDIO / words_name, {"version": 1, "text": text_id, "file": m4a_name, "duration": info["trimmed_seconds"],
                                      "reader": src["reader"], "words": timed})
            key = f"narration:{text_id}"
            index[key] = m4a_name
            sources[m4a_name] = {
                "key": key, "kind": "human_recording", "set": SET, "text_id": text_id, "words": words_name,
                "provider": src["provider"], "title": src["title"], "author": t["author"], "reader": src["reader"],
                "speaker": src["reader"], "license": src["license"], "source_page": src["source_page"],
                "credit": src["statement"], "audio_url": src["url"],
                "processing": f"cut {start:.3f}–{end:.3f} s of the original with ffmpeg, then trimmed and levelled "
                              f"with scripts/audio_level.py, AAC 64 kbps; words timed by MMS forced alignment (ctc-forced-aligner)",
                "cut": {"start": round(start, 3), "end": round(end, 3)},
                **store.restore_fields(orig, url=src["url"]),
                "pipeline": PIPELINE,
            }
            t["narration"] = m4a_name
            t["words"] = words_name
            t["reader"] = src["reader"]
            print(f"{text_id}: {m4a_name} {info['trimmed_seconds']}s, {len(timed)} words, gain {info['gain_db']} dB")
    dump(AUDIO / "index-narration.json", dict(sorted(index.items())))
    dump(AUDIO / "sources-narration.json", dict(sorted(sources.items())))
    dump(LIBRARY, lib)
    return 0


# ---------------------------------------------------------------- check

def check_words(text, timing):
    """Problems in a words.json against its text: token mismatch, order, bounds."""
    problems = []
    expected = [b for _, b in timed_tokens(text)]
    got = [w for w, *_ in timing["words"]]
    if expected != got:
        problems.append(f"words differ from the text ({len(got)} timed, {len(expected)} in text)")
    last = 0.0
    for w, s, e in timing["words"]:
        if not (0 <= s <= e <= timing["duration"] + 0.01):
            problems.append(f"{w}: {s}–{e} outside 0–{timing['duration']}")
        if s < last - 0.005:
            problems.append(f"{w}: starts at {s} before the previous word ended ({last})")
        last = e
    return problems


def cmd_check(args):
    lib = library()
    index = load_json(AUDIO / "index-narration.json", {})
    sources = load_json(AUDIO / "sources-narration.json", {})
    errors = 0
    narrated = [t for t in lib["texts"] if "narration" in t]
    for t in narrated:
        tag = t["id"]
        wp = AUDIO / t.get("words", "")
        if not wp.is_file():
            print(f"ERROR {tag}: words file {t.get('words')} missing"); errors += 1; continue
        timing = json.loads(wp.read_text())
        if timing.get("file") != t["narration"] or index.get(f"narration:{t['id']}") != t["narration"]:
            print(f"ERROR {tag}: narration file not consistent across library.json, words.json, index-narration.json"); errors += 1
        if t["narration"] not in sources:
            print(f"ERROR {tag}: {t['narration']} has no sources-narration.json entry"); errors += 1
        for p in check_words(t["text"], timing):
            print(f"ERROR {tag}: {p}"); errors += 1
    print(f"{len(narrated)} narrated texts, {errors} problems")
    return 1 if errors else 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", choices=["align", "build", "check"])
    ap.add_argument("ids", nargs="*")
    args = ap.parse_args()
    if args.command != "check" and not args.ids:
        args.ids = [i for i in SOURCES if not i.startswith("_")]
    return {"align": cmd_align, "build": cmd_build, "check": cmd_check}[args.command](args)


if __name__ == "__main__":
    sys.exit(main())
