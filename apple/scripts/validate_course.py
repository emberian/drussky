#!/usr/bin/env python3
"""Validate apple/Drussky/Resources/course.json against apple/COURSE_SCHEMA.md.

    python3 apple/scripts/validate_course.py [course.json] [--audio]

--audio also checks that every string the app can speak has an entry in
Resources/Audio/index.json whose file exists.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT = ROOT / "Drussky/Resources/course.json"
ACUTE = "́"
ALPHABET = "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ"
VOWELS = set("аеёиоуыэюя")
CYR = re.compile(r"[Ѐ-ӿ]")
PUNCT = set(".,?!—–-\"«»:;…()")
FAMILIES = {"familiar", "false_friend", "new", "sign"}
WRITING = {"meet", "trace", "copy", "dictation", "translate", "fill"}
KINDS = {
    "meet": ({"letter"}, set()),
    "explain": ({"text"}, {"examples"}),
    "trace": ({"text"}, set()),
    "copy": ({"text", "times"}, set()),
    "listen_pick": ({"say", "options"}, set()),
    "dictation": ({"text"}, set()),
    "read_aloud": ({"text"}, set()),
    "translate": ({"en", "answer"}, set()),
    "fill": ({"before", "after", "hint", "answer"}, set()),
    "read_meaning": ({"text", "options", "answer"}, set()),
}
BANNED = re.compile(r"\b(optional|optionally|recording|recordings|recorded|accuracy|accurate|the app|this app|model voice|disclaimer|friend|friends|coach|tutor|might not|may not be)\b", re.I)

errors = []


def err(where, msg):
    errors.append(f"{where}: {msg}")


def key(s):
    """Same as RussianText.key in the Swift app."""
    s = s.replace(ACUTE, "").lower()
    s = "".join(" " if c in PUNCT else c for c in s)
    return " ".join(s.split())


def tokens(s):
    return re.findall(r"[Ѐ-ӿ́]+", s)


def letters_in(s):
    return {c for c in s.replace(ACUTE, "").upper() if CYR.match(c)}


def check_stress(where, s):
    for tok in tokens(s):
        low = tok.lower()
        for i, c in enumerate(low):
            if c == ACUTE and (i == 0 or low[i - 1] not in VOWELS):
                err(where, f"acute not after a vowel in {tok!r}")
        plain = low.replace(ACUTE, "")
        vowels = sum(c in VOWELS for c in plain)
        marks = low.count(ACUTE)
        if "ё" in plain:
            if marks:
                err(where, f"ё word must not carry an acute: {tok!r}")
            if plain.count("ё") > 1:
                err(where, f"more than one ё: {tok!r}")
        elif vowels >= 2 and marks != 1:
            err(where, f"{vowels}-syllable word needs exactly one stress mark: {tok!r}")
        elif vowels <= 1 and marks:
            err(where, f"monosyllable must not be marked: {tok!r}")


def check_letters(where, s, known):
    extra = letters_in(s) - known
    if extra:
        err(where, f"uses letters not yet met {''.join(sorted(extra))} in {s!r}")


def russian_fields(e):
    """(field label, string) pairs of Russian an exercise shows or speaks."""
    k = e["kind"]
    out = []
    if k in ("trace", "copy", "dictation", "read_aloud", "read_meaning"):
        out.append(("text", e["text"]))
    if k == "explain":
        out.append(("text", e["text"]))
        out += [("examples.ru", x["ru"]) for x in e.get("examples", [])]
    if k == "listen_pick":
        out.append(("say", e["say"]))
        out += [("options", o) for o in e["options"]]
    if k in ("translate", "fill"):
        out.append(("answer", e["answer"]))
    if k == "fill":
        out += [("before", e["before"]), ("after", e["after"])]
    return out


def spoken_strings(course):
    """Every Russian string the app may play (mirrors Exercise.spoken plus taps)."""
    out = []
    for l in course["letters"]:
        out += [l["say"], l.get("example") or ""]
    out += [w["ru"] for w in course["lexicon"]]
    for u in course["units"]:
        for p in u["pages"]:
            for e in p["exercises"]:
                k = e["kind"]
                if k in ("trace", "copy", "dictation", "read_aloud", "read_meaning"):
                    out.append(e["text"])
                if k == "explain":
                    out += [x["ru"] for x in e.get("examples", [])]
                if k == "listen_pick":
                    out += [e["say"]] + e["options"]
                if k == "translate":
                    out.append(e["answer"])
                if k == "fill":
                    out += [e["answer"], e["before"] + e["answer"] + e["after"]]
    return [s for s in out if s and CYR.search(s)]


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    path = Path(args[0]) if args else DEFAULT
    course = json.loads(path.read_text())

    if set(course) != {"version", "letters", "lexicon", "units"}:
        err("top", f"keys {sorted(course)}")
    if course.get("version") != 1:
        err("top", "version must be 1")

    # letters
    letters = course["letters"]
    if [l.get("upper") for l in letters] != list(ALPHABET):
        err("letters", "must be all 33 in alphabet order")
    for l in letters:
        w = f"letter {l.get('upper')}"
        req = {"upper", "lower", "say", "sound", "family", "tip"}
        if not req <= set(l) or set(l) - req - {"example"}:
            err(w, f"keys {sorted(l)}")
            continue
        if l["lower"] != l["upper"].lower():
            err(w, "lower mismatch")
        if l["family"] not in FAMILIES:
            err(w, f"family {l['family']!r}")
        for f in ("say", "example"):
            if l.get(f):
                check_stress(f"{w}.{f}", l[f])

    # lexicon
    lexicon = course["lexicon"]
    lex_keys, lex_tokens = {}, set()
    for i, wd in enumerate(lexicon):
        w = f"lexicon[{i}] {wd.get('ru')}"
        if not {"ru", "en"} <= set(wd) or set(wd) - {"ru", "en", "emoji", "note"}:
            err(w, f"keys {sorted(wd)}")
            continue
        if not all(isinstance(wd[k], str) and wd[k].strip() for k in wd):
            err(w, "empty field")
        check_stress(w, wd["ru"])
        k = key(wd["ru"])
        if k in lex_keys:
            err(w, f"duplicate of lexicon[{lex_keys[k]}]")
        lex_keys[k] = i
        lex_tokens |= set(k.split())

    # units / pages / exercises
    known, met, ids = set(), [], set()
    letter_rows = {l["upper"]: l for l in letters}
    for u in course["units"]:
        if set(u) - {"id", "title", "subtitle", "pages"} or not {"id", "title", "pages"} <= set(u):
            err(u.get("id"), f"unit keys {sorted(u)}")
        if u["id"] in ids:
            err(u["id"], "duplicate id")
        ids.add(u["id"])
        check_stress(u["id"] + ".title/subtitle", u["title"] + " " + (u.get("subtitle") or ""))
        for p in u["pages"]:
            pw = p.get("id")
            if set(p) - {"id", "title", "intro", "exercises"} or not {"id", "title", "exercises"} <= set(p):
                err(pw, f"page keys {sorted(p)}")
            if pw in ids:
                err(pw, "duplicate id")
            ids.add(pw)
            check_stress(pw + ".title/intro", p["title"] + " " + (p.get("intro") or ""))
            for text in (p["title"], p.get("intro") or ""):
                if BANNED.search(text):
                    err(pw, f"voice: {BANNED.search(text).group(0)!r}")
            exs = p["exercises"]
            page_known = known | {e.get("letter") for e in exs if e.get("kind") == "meet"}
            for e in exs:
                if e.get("kind") == "meet" and e.get("letter") in letter_rows:
                    row = letter_rows[e["letter"]]
                    for f in ("say", "example"):
                        if row.get(f):
                            check_letters(f"letter {row['upper']}.{f} (by end of {pw})", row[f], page_known)
            if not 6 <= len(exs) <= 10:
                err(pw, f"{len(exs)} exercises (need 6-10)")
            kinds = [e.get("kind") for e in exs]
            writing = sum(k in WRITING for k in kinds)
            if writing * 2 <= len(exs):
                err(pw, f"writing is {writing}/{len(exs)}, not a majority")
            if not {"listen_pick", "dictation"} & set(kinds):
                err(pw, "no hearing exercise")
            if "read_aloud" not in kinds:
                err(pw, "no speaking exercise")
            for n, e in enumerate(exs, 1):
                w = f"{pw}#{n} {e.get('kind')}"
                k = e.get("kind")
                if k not in KINDS:
                    err(w, "unknown kind")
                    continue
                req, opt = KINDS[k]
                if set(e) - {"kind"} - req - opt or not req <= set(e):
                    err(w, f"fields {sorted(e)}")
                    continue
                if k == "meet":
                    up = e["letter"]
                    if up not in letter_rows:
                        err(w, f"unknown letter {up!r}")
                        continue
                    if up in known:
                        err(w, f"{up} met twice")
                    known.add(up)
                    met.append(up)
                    continue
                for label, s in russian_fields(e):
                    check_letters(f"{w}.{label}", s, known)
                    check_stress(f"{w}.{label}", s)
                if k == "explain":
                    t = e["text"]
                    sentences = 1 + len(re.findall(r"[.!?]\s+(?=[A-ZА-ЯЁ])", t.strip()))
                    if sentences > 3:
                        err(w, f"{sentences} sentences (max 3)")
                    if BANNED.search(t):
                        err(w, f"voice: {BANNED.search(t).group(0)!r}")
                    for x in e.get("examples", []):
                        if set(x) != {"ru", "en"}:
                            err(w, "example keys")
                if k == "copy" and e["times"] not in (1, 2, 3):
                    err(w, "times must be 1-3")
                if k == "listen_pick":
                    if e["say"] not in e["options"]:
                        err(w, "say not in options")
                    okeys = [key(o) for o in e["options"]]
                    if len(set(okeys)) != len(okeys) or len(okeys) < 2:
                        err(w, f"options must be 2+ and distinct when spoken: {okeys}")
                if k == "translate" and key(e["answer"]) not in lex_keys:
                    err(w, f"answer {e['answer']!r} not in lexicon")
                if k == "translate" and CYR.search(e["en"]):
                    err(w, "en must be English")
                if k == "fill":
                    if not e["answer"].strip():
                        err(w, "empty answer")
                    if e["before"] and not e["before"].endswith(" "):
                        err(w, "before must end with a space")
                    if e["after"] and not (e["after"][0] in " " or e["after"][0] in PUNCT):
                        err(w, "after must start with a space or punctuation")
                    if CYR.search(e["hint"]):
                        err(w, "hint must be English")
                if k == "read_meaning":
                    a = e["answer"]
                    if not isinstance(a, int) or isinstance(a, bool) or not 0 <= a < len(e["options"]):
                        err(w, f"answer index {a!r} out of range")
                    if any(CYR.search(o) for o in e["options"]) or len(set(e["options"])) != len(e["options"]):
                        err(w, "options must be distinct English")
                if k in ("dictation", "translate", "fill", "read_meaning"):
                    s = e["text"] if k in ("dictation", "read_meaning") else (e["before"] + e["answer"] + e["after"] if k == "fill" else e["answer"])
                    missing = [t for t in key(s).split() if t not in lex_tokens]
                    if missing:
                        err(w, f"words not in lexicon: {missing}")
    # every lexicon entry is produced somewhere (written, said, or picked by ear), and glosses are distinct
    produced = []
    for u in course["units"]:
        for p in u["pages"]:
            for e in p["exercises"]:
                k = e.get("kind")
                if k in ("trace", "copy", "dictation", "read_aloud"):
                    produced.append(key(e["text"]))
                elif k in ("translate", "fill"):
                    produced.append(key(e["answer"]))
                    if k == "fill":
                        produced.append(key(e["before"] + e["answer"] + e["after"]))
                elif k == "listen_pick":
                    produced.append(key(e["say"]))
    padded = [f" {t} " for t in produced]
    for wd in lexicon:
        k = key(wd["ru"])
        if not any(f" {k} " in t for t in padded):
            err(f"lexicon {wd['ru']}", "never produced (no trace/copy/dictation/read_aloud/translate/fill/listen_pick target)")
    glosses = {}
    for wd in lexicon:
        if wd["en"] in glosses:
            err(f"lexicon {wd['ru']}", f"gloss {wd['en']!r} shared with {glosses[wd['en']]}")
        glosses[wd["en"]] = wd["ru"]

    if sorted(met) != sorted(ALPHABET) or len(met) != 33:
        err("meet", f"letters met {len(met)}; missing {sorted(set(ALPHABET) - set(met))}")

    if "--audio" in sys.argv:
        audio = path.parent / "Audio"
        index = json.loads((audio / "index.json").read_text())
        for s in sorted({key(s) for s in spoken_strings(course)}):
            f = index.get(s)
            if not f:
                err("audio", f"no recording for {s!r}")
            elif not (audio / f).is_file():
                err("audio", f"missing file {f} for {s!r}")

    pages = sum(len(u["pages"]) for u in course["units"])
    exercises = sum(len(p["exercises"]) for u in course["units"] for p in u["pages"])
    print(f"{len(course['units'])} units, {pages} pages, {exercises} exercises, {len(lexicon)} lexicon, letter order {''.join(met)}")
    if errors:
        print(f"{len(errors)} problem(s):")
        for e in errors:
            print("  " + e)
        sys.exit(1)
    print("OK")


if __name__ == "__main__":
    main()
