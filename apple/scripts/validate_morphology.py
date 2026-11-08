#!/usr/bin/env python3
"""Validate apple/Drussky/Resources/morphology.json against the course, the lemma map and the
library.

    python3 apple/scripts/validate_morphology.py [morphology.json]

Checks: every lemma's morphemes spell the word; every root a lemma names exists and the root
morpheme is one of that root's forms; every member of a family carries the family's root;
prefixes and suffixes are declared; `base` points at a lemma; every course lexicon token (via
lemmas.json) and the ~300 most frequent library words resolve to an entry; cognate tiles spell
the word and their rules exist; stress marks are well formed everywhere.
"""
import collections
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import validate_course as vc  # noqa: E402

ROOT = Path(__file__).resolve().parents[1] / "Drussky/Resources"
DEFAULT = ROOT / "morphology.json"
POS = {"noun", "verb", "adj", "adv", "pron", "num", "prep", "conj", "part", "pred"}
LIBRARY_TOP = 300
errors = []


def err(where, msg):
    errors.append(f"{where}: {msg}")


def strip(s):
    return s.replace(vc.ACUTE, "")


def library_top(lib, n):
    cnt = collections.Counter()
    for t in lib["texts"]:
        for w in re.findall(r"[а-яё]+(?:-[а-яё]+)*", strip(t["text"]).lower()):
            cnt[w] += 1
    return [w for w, _ in cnt.most_common(n)]


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    path = Path(args[0]) if args else DEFAULT
    m = json.loads(path.read_text())
    course = json.loads((ROOT / "course.json").read_text())
    lemma_map = json.loads((ROOT / "lemmas.json").read_text())["forms"]
    library = json.loads((ROOT / "library.json").read_text())

    if m.get("version") != 1:
        err("top", "version must be 1")
    roots = {}
    for r in m["roots"]:
        if r["id"] in roots:
            err(f"root {r['id']}", "duplicate id")
        roots[r["id"]] = r
        if not r.get("forms") or not r.get("gloss"):
            err(f"root {r['id']}", "needs forms and a gloss")
        for f in r.get("forms", []):
            if not f or f != strip(f).lower() or not vc.CYR.search(f):
                err(f"root {r['id']}", f"form {f!r} must be lowercase Cyrillic without stress")
    prefixes = {p["id"]: p for p in m["prefixes"]}
    prefix_forms = {f for p in m["prefixes"] for f in p["forms"]}
    suffixes = {x["id"]: x for x in m["suffixes"]}
    suffix_forms = {f for x in m["suffixes"] for f in x["forms"]}
    if len(prefixes) != len(m["prefixes"]) or len(suffixes) != len(m["suffixes"]):
        err("affixes", "duplicate prefix or suffix id")
    for p in m["prefixes"] + m["suffixes"]:
        for x in p.get("examples", []):
            vc.check_stress(f"affix {p['id']}", x)

    lemmas = m["lemmas"]
    members = collections.defaultdict(list)
    for head, e in lemmas.items():
        w = f"lemma {head}"
        if head != strip(e["ru"]).lower() and head != strip(e["ru"]):
            err(w, f"key is not the unstressed headword of {e['ru']!r}")
        vc.check_stress(w, e["ru"])
        if "".join(strip(x) for x in e["morphemes"]).replace("-", "") != strip(e["ru"]).replace("-", ""):
            err(w, f"morphemes {e['morphemes']} do not spell {e['ru']!r}")
        if e["pos"] not in POS:
            err(w, f"pos {e['pos']!r}")
        if not e.get("en", "").strip():
            err(w, "no English")
        plain = [strip(x).lower() for x in e["morphemes"]]
        for rid in e["roots"]:
            if rid not in roots:
                err(w, f"unknown root {rid!r}")
            elif not any(x in roots[rid]["forms"] for x in plain):
                err(w, f"no morpheme of {plain} is a form of root {rid} {roots[rid]['forms']}")
        fam = e.get("family")
        if fam:
            if fam not in roots:
                err(w, f"unknown family {fam!r}")
            elif fam not in e["roots"]:
                err(w, f"family {fam} is not among its roots {e['roots']}")
            members[fam].append(head)
        elif e["roots"]:
            err(w, "has roots but no family")
        for p in e["prefixes"]:
            if p not in prefixes and p not in prefix_forms:
                err(w, f"undeclared prefix {p!r}")
        for x in e["suffixes"]:
            if x not in suffix_forms:
                err(w, f"undeclared suffix {x!r}")
        if "base" in e:
            b = e["base"]
            if b not in lemmas and b not in roots:
                err(w, f"base {b!r} is not a lemma")
            elif b in lemmas and e.get("family") and lemmas[b].get("family") and lemmas[b]["family"] != e["family"]:
                err(w, f"derived from {b} ({lemmas[b]['family']}) but in family {e['family']}")
        if "example" in e:
            ex = e["example"]
            vc.check_stress(w + ".example", ex["ru"])
            if head not in {vc.key(t) for t in ex["ru"].split()} and not any(
                    vc.key(t).startswith(strip(e["morphemes"][0]).lower()[:3]) for t in ex["ru"].split()):
                err(w, f"example {ex['ru']!r} does not use the word")
            if not ex.get("en", "").strip() or vc.CYR.search(ex["en"]):
                err(w, "example needs an English translation")
    # every family's members share the root (checked above per lemma); a family needs 2+ members to be one
    singles = sorted(fam for fam, ms in members.items() if len(ms) < 2)
    for rid in roots:
        if rid not in members:
            err(f"root {rid}", "no lemma belongs to it")

    forms = m["forms"]
    for f, head in forms.items():
        if head not in lemmas:
            err(f"form {f}", f"maps to unknown lemma {head!r}")
        if f != strip(f).lower():
            err(f"form {f}", "must be lowercase and unstressed")

    names = set(m.get("names", []))
    cognate_keys = {vc.key(c["ru"]) for c in m["cognates"]}

    def resolve(token):
        t = vc.key(token)
        if t in names or t in cognate_keys or not t:
            return True
        for cand in (lemma_map.get(t, [None])[0], forms.get(t), t):
            if cand and (cand in lemmas or cand in forms and forms[cand] in lemmas):
                return True
        return False

    # coverage: course lexicon
    missing = sorted({t for w in course["lexicon"] for t in vc.key(w["ru"]).split() if not resolve(t)})
    if missing:
        err("coverage", f"{len(missing)} course lexicon words without a morphology entry: {missing}")
    # coverage: the lemma map's own lemmas
    missing = sorted({v[0] for v in lemma_map.values() if not resolve(v[0])})
    if missing:
        err("coverage", f"lemma-map lemmas without an entry: {missing}")
    # coverage: library
    missing = [t for t in library_top(library, LIBRARY_TOP) if not resolve(t)]
    if missing:
        err("coverage", f"{len(missing)} of the {LIBRARY_TOP} most frequent library words without an entry: {missing}")

    rules = {r["id"] for r in m["rules"]}
    if len(rules) != len(m["rules"]):
        err("rules", "duplicate rule id")
    for r in m["rules"]:
        for x in r.get("examples", []):
            vc.check_stress(f"rule {r['id']}", x)
    seen = set()
    for c in m["cognates"]:
        w = f"cognate {c['ru']}"
        vc.check_stress(w, c["ru"])
        k = vc.key(c["ru"])
        if k in seen:
            err(w, "duplicate")
        seen.add(k)
        if not c.get("en") or not c.get("source"):
            err(w, "needs en and source")
        for rid in c["rules"]:
            if rid not in rules:
                err(w, f"unknown rule {rid!r}")
        if "parts" in c:
            built = "".join(strip(p["tile"]).replace("-", "") for p in c["parts"])
            if built != strip(c["ru"]).replace("-", ""):
                err(w, f"tiles {[p['tile'] for p in c['parts']]} spell {built!r}, not the word")
            for p in c["parts"]:
                vc.check_stress(w + ".parts", p["tile"].replace("-", ""))
                if not p.get("gloss"):
                    err(w, f"tile {p['tile']!r} has no gloss")
            stressed = [p["tile"] for p in c["parts"] if vc.ACUTE in p["tile"]]
            if len(stressed) > 1:
                err(w, f"more than one tile carries the stress: {stressed}")

    print(f"{len(roots)} roots, {len(prefixes)} prefixes, {len(suffixes)} suffixes, {len(lemmas)} lemmas, "
          f"{len(forms)} forms, {sum(1 for e in lemmas.values() if 'example' in e)} examples, "
          f"{len(m['cognates'])} cognates, {len(rules)} rules; families with 2+ members: {sum(1 for v in members.values() if len(v) >= 2)}, "
          f"roots with one member so far: {len(singles)}")
    if errors:
        print(f"{len(errors)} problem(s):")
        for e in errors:
            print("  " + e)
        sys.exit(1)
    print("OK")


if __name__ == "__main__":
    main()
