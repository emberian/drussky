#!/usr/bin/env python3
"""Check Drussky/Resources/library-stress.json against library.json (no third-party packages).

  python3 apple/scripts/validate_stress.py [--library PATH] [--stress PATH] [--allow-missing]

Every text of the library and every Tatoeba sentence has a stressed form (the exact source sentence is stored
and must still be in the text at that index: an edited text invalidates its entry); removing U+0301 from the
stressed form gives the source back exactly; every Cyrillic word of two or more syllables carries exactly one
mark after a vowel, one-syllable words and words with ё none (library_stress.problems, the rule the app's
RussianStress.problems repeats); nothing is stored for a shelf sentence that was already stressed. Every
stressed word is compared with the course's own spelling (course.json, morphology.json, paradigms.json): a
word the course marks differently must be listed under "disagreements" or named as a homograph. Prints
coverage, the homographs and the disagreements for review. Exit 1 on any error. --allow-missing turns a
sentence without a stressed form into a warning (for a run in progress).
"""
import argparse
import json
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(APPLE / "scripts"))
from library_stress import ACUTE, SHELVES, course_forms, disagreements, lacks_stress, problems, sources  # noqa: E402

RES = APPLE / "Drussky/Resources"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--library", type=Path, default=RES / "library.json")
    ap.add_argument("--stress", type=Path, default=RES / "library-stress.json")
    ap.add_argument("--allow-missing", action="store_true")
    ap.add_argument("--quiet", action="store_true", help="counts only; no homograph or disagreement listing")
    args = ap.parse_args()

    errors, warnings = [], []
    err, warn = errors.append, warnings.append
    lib = json.loads(args.library.read_text())
    if not args.stress.exists():
        raise SystemExit(f"{args.stress} does not exist")
    data = json.loads(args.stress.read_text())
    if data.get("version") != 1:
        err("version must be 1")
    texts = data.get("texts", {})
    forms = course_forms()
    flagged = {(d["text"], d["index"], d["word"].lower().replace(ACUTE, "")) for d in data.get("disagreements", [])}
    named = {(h["text"], h["index"], h["word"].lower().replace(ACUTE, "")) for h in data.get("homographs", [])}
    counts = {"texts": 0, "sentences": 0, "stressed": 0, "words": 0, "marked": 0, "trivial": 0}
    seen_ids = set()

    for tid, sentences, origin in sources(lib, True, SHELVES):
        seen_ids.add(tid)
        entry = texts.get(tid)
        have = {s["index"]: s for s in (entry or {}).get("sentences", [])}
        extra = set(have) - set(range(len(sentences)))
        if extra:
            err(f"{tid}: stressed forms for sentences that do not exist: {sorted(extra)}")
        if origin not in ("library", "tatoeba"):
            # An authored shelf: already stressed, nothing should be stored unless a sentence lacked a mark.
            for i, s in have.items():
                if i < len(sentences) and not lacks_stress(sentences[i]):
                    err(f"{tid} S{i}: stored for a shelf sentence that already carries stress")
            wanted = [i for i, s in enumerate(sentences) if lacks_stress(s)]
            if not wanted and not have:
                continue
        else:
            wanted = list(range(len(sentences)))
            counts["texts"] += 1
        for i in wanted:
            src = sentences[i]
            where = f"{tid} S{i}"
            counts["sentences"] += 1
            s = have.get(i)
            if not s:
                (warn if args.allow_missing else err)(f"{where}: no stressed form")
                continue
            if s["text"] != src:
                err(f"{where}: stored sentence differs from the text (edited?): {s['text'][:40]!r} vs {src[:40]!r}")
                continue
            ps = problems(src, s["stressed"])
            for p in ps:
                err(f"{where}: {p}")
            if ps:
                continue
            counts["stressed"] += 1
            if s["stressed"] == src:
                counts["trivial"] += 1
            counts["marked"] += s["stressed"].count(ACUTE)
            for tok, known in disagreements(s["stressed"], forms):
                bare = tok.lower().replace(ACUTE, "")
                if (tid, i, bare) not in flagged and (tid, i, bare) not in named:
                    err(f"{where}: {tok} differs from the course's {'/'.join(known)} and is neither flagged nor a named homograph")
    unknown = set(texts) - seen_ids
    if unknown:
        warn(f"stressed forms for texts not in the library or on a shelf: {sorted(unknown)}")
    for h in data.get("homographs", []):
        if h["text"] not in texts or h["index"] not in {s["index"] for s in texts[h["text"]]["sentences"]}:
            err(f"homograph {h['word']} points at a sentence with no entry: {h['text']} S{h['index']}")
    for d in data.get("disagreements", []):
        if d["text"] not in texts or d["index"] not in {s["index"] for s in texts[d["text"]]["sentences"]}:
            err(f"disagreement {d['word']} points at a sentence with no entry: {d['text']} S{d['index']}")

    if not args.quiet:
        homs = data.get("homographs", [])
        if homs:
            print(f"{len(homs)} homographs (the model's call, for review):")
            for h in homs:
                alts = ", ".join(h.get("alternatives", []))
                print(f"  {h['text']} S{h['index']}: {h['word']} (not {alts}) — {h.get('reason', '')}")
        dis = data.get("disagreements", [])
        if dis:
            print(f"{len(dis)} words the course marks differently (kept as the model wrote them, for review):")
            for d in dis:
                print(f"  {d['text']} S{d['index']}: {d['word']} vs the course's {'/'.join(d['course'])}")
    for w in warnings:
        print("warning:", w)
    for e in errors:
        print("error:", e)
    print(f"{counts['texts']} texts, {counts['stressed']}/{counts['sentences']} sentences stressed "
          f"({counts['trivial']} needed no mark), {counts['marked']} marks placed; {len(data.get('homographs', []))} homographs, "
          f"{len(data.get('disagreements', []))} disagreements with the course; {len(errors)} errors, {len(warnings)} warnings")
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
