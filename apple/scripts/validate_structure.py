#!/usr/bin/env python3
"""Check Drussky/Resources/library-structure.json against library.json (no third-party packages).

  python3 apple/scripts/validate_structure.py [--library PATH] [--structure PATH] [--allow-missing]

Every text of the library has structure for every sentence (the exact source sentence is stored and must
still be in the text at that index: an edited text invalidates its entry); every chunk, desugar and
governance span falls on word boundaries of that sentence and uses a known role; member chunks never
partially overlap each other, nor do clause chunks; referent heads point at a word that exists (in this
sentence or an earlier one of the same text); every paraphrase and case frame carries one stress mark on
each word of two or more syllables and none on a word with ё; every x-ray spells its word. Exit 1 on any
error. --allow-missing turns a text or sentence without structure into a warning (for a run in progress).
"""
import argparse
import json
import re
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(APPLE / "scripts"))
from library_structure import ROLES, REFERENT_KINDS, DESUGAR_KINDS, CASES, check_stress, sentences_of, words_of  # noqa: E402

RES = APPLE / "Drussky/Resources"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--library", type=Path, default=RES / "library.json")
    ap.add_argument("--structure", type=Path, default=RES / "library-structure.json")
    ap.add_argument("--allow-missing", action="store_true")
    args = ap.parse_args()

    errors, warnings = [], []
    err, warn = errors.append, warnings.append
    lib = json.loads(args.library.read_text())
    if not args.structure.exists():
        raise SystemExit(f"{args.structure} does not exist")
    data = json.loads(args.structure.read_text())
    if data.get("version") != 1:
        err("version must be 1")
    if data.get("roles") != ROLES:
        err("roles table differs from library_structure.ROLES")
    texts = data.get("texts", {})
    counts = {"texts": 0, "sentences": 0, "annotated": 0, "chunks": 0, "referents": 0, "desugar": 0, "roots": 0, "governance": 0}

    for t in lib["texts"]:
        tid = t["id"]
        counts["texts"] += 1
        sentences = sentences_of(t["text"])
        counts["sentences"] += len(sentences)
        entry = texts.get(tid)
        if not entry:
            (warn if args.allow_missing else err)(f"{tid}: no structure")
            continue
        have = {s["index"]: s for s in entry.get("sentences", [])}
        for i, (_, src) in enumerate(sentences):
            s = have.get(i)
            where = f"{tid} S{i}"
            if not s:
                (warn if args.allow_missing else err)(f"{where}: no structure")
                continue
            if s["text"] != src:
                err(f"{where}: stored sentence differs from the text (edited?): {s['text'][:40]!r} vs {src[:40]!r}")
                continue
            words = words_of(src)
            if s.get("words") != words:
                err(f"{where}: stored word spans differ from the sentence's words")
                continue
            counts["annotated"] += 1
            n = len(words)
            boundaries_start = {a for a, _ in words}
            boundaries_end = {b for _, b in words}

            def span_ok(item, tag):
                sp, ws = item.get("span"), item.get("words")
                if not (isinstance(ws, list) and len(ws) == 2 and 0 <= ws[0] <= ws[1] < n):
                    err(f"{where} {tag}: word range {ws} outside 0..{n - 1}")
                    return False
                if not (isinstance(sp, list) and len(sp) == 2 and sp[0] in boundaries_start and sp[1] in boundaries_end
                        and sp == [words[ws[0]][0], words[ws[1]][1]]):
                    err(f"{where} {tag}: span {sp} is not on word boundaries of words {ws}")
                    return False
                return True

            tiers = {"member": [], "clause": []}
            for c in s.get("chunks", []):
                if c.get("role") not in ROLES:
                    err(f"{where} chunk: unknown role {c.get('role')!r}")
                    continue
                if span_ok(c, f"chunk {c['role']}"):
                    tiers[ROLES[c["role"]]["tier"]].append((c["words"][0], c["words"][1], c["role"]))
                    counts["chunks"] += 1
            for tier, spans in tiers.items():
                for a in spans:
                    for b in spans:
                        if a < b and a[0] <= b[1] and b[0] <= a[1]:  # overlapping
                            nested = (a[0] <= b[0] and b[1] <= a[1]) or (b[0] <= a[0] and a[1] <= b[1])
                            if tier == "member" or not nested:
                                err(f"{where}: {tier} chunks {a} and {b} overlap")
            for r in s.get("referents", []):
                hs = r.get("headSentence", i)
                if r.get("kind") not in REFERENT_KINDS:
                    err(f"{where} referent: unknown kind {r.get('kind')!r}")
                if not (0 <= r.get("word", -1) < n):
                    err(f"{where} referent: word {r.get('word')} outside the sentence")
                    continue
                if not (0 <= hs <= i):
                    err(f"{where} referent: head sentence {hs} is not this one or an earlier one")
                    continue
                hn = len(words_of(sentences[hs][1]))
                if not (0 <= r.get("head", -1) < hn):
                    err(f"{where} referent: head {r.get('head')} outside S{hs}")
                    continue
                if hs == i and r["head"] == r["word"]:
                    err(f"{where} referent: word {r['word']} points at itself")
                counts["referents"] += 1
            for d in s.get("desugar", []):
                if d.get("kind") not in DESUGAR_KINDS:
                    err(f"{where} desugar: unknown kind {d.get('kind')!r}")
                if not span_ok(d, "desugar"):
                    continue
                if not d.get("ru", "").strip():
                    err(f"{where} desugar: empty Russian paraphrase")
                if not re.search("[Ѐ-ӿ]", d.get("ru", "")):
                    err(f"{where} desugar: paraphrase has no Russian: {d.get('ru')!r}")
                for p in check_stress(d.get("ru", "")):
                    err(f"{where} desugar: {p}")
                if not d.get("en", "").strip():
                    err(f"{where} desugar: empty English")
                counts["desugar"] += 1
            seen = set()
            for x in s.get("roots", []):
                w = x.get("word", -1)
                if not (0 <= w < n):
                    err(f"{where} x-ray: word {w} outside the sentence")
                    continue
                if w in seen:
                    err(f"{where} x-ray: word {w} analysed twice")
                seen.add(w)
                written = src[words[w][0]:words[w][1]].lower().replace("́", "")
                joined = "".join(x.get(k, "") for k in ("prefix", "root", "suffix", "inflection"))
                if joined != written:
                    err(f"{where} x-ray: {joined!r} does not spell {written!r}")
                if not x.get("root"):
                    err(f"{where} x-ray: {written} has no root")
                if x.get("source") not in ("morphology", "model"):
                    err(f"{where} x-ray: source must be morphology or model")
                for rel in x.get("relatives", []):
                    for p in check_stress(rel):
                        err(f"{where} x-ray relative: {p}")
                counts["roots"] += 1
            for g in s.get("governance", []):
                if not (0 <= g.get("head", -1) < n):
                    err(f"{where} governance: head {g.get('head')} outside the sentence")
                    continue
                if any(not (0 <= d < n) for d in g.get("dependents", [])):
                    err(f"{where} governance: dependent outside the sentence")
                if g.get("case") not in CASES:
                    err(f"{where} governance: unknown case {g.get('case')!r}")
                if not g.get("frame", "").strip():
                    err(f"{where} governance: empty frame")
                for p in check_stress(g.get("frame", "")):
                    err(f"{where} governance frame: {p}")
                counts["governance"] += 1
        extra = set(have) - set(range(len(sentences)))
        if extra:
            err(f"{tid}: structure for sentences that do not exist: {sorted(extra)}")
    unknown = set(texts) - {t["id"] for t in lib["texts"]}
    if unknown:
        warn(f"structure for texts not in the library: {sorted(unknown)}")

    for w in warnings:
        print("warning:", w)
    for e in errors:
        print("error:", e)
    print(f"{counts['texts']} texts, {counts['annotated']}/{counts['sentences']} sentences with structure; "
          f"{counts['chunks']} chunks, {counts['referents']} referents, {counts['desugar']} desugarings, "
          f"{counts['roots']} x-rays, {counts['governance']} case frames; {len(errors)} errors, {len(warnings)} warnings")
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
