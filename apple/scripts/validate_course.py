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
WRITING = {"meet", "trace", "copy", "dictation", "translate", "fill", "transform", "paradigm_fill"}


def is_writing(e):
    """Written with the Pencil (a paradigm table filled from tiles is not)."""
    return e.get("kind") in WRITING and not (e.get("kind") == "paradigm_fill" and "options" in e)
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
    "passage": ({"text", "en"}, {"glosses"}),
    "diary": ({"prompt"}, set()),
    # Morphology/ (see COURSE_SCHEMA.md): tiles → word, and a new word on a known root.
    "build_word": ({"en", "answer", "parts"}, {"options", "alternatives"}),
    "family_guess": ({"text", "word", "root", "options", "answer", "en"}, set()),
    # Grammar/ (see COURSE_SCHEMA.md): a table to fill, a form from a cue, phrases sorted by pattern.
    "paradigm_fill": ({"word", "paradigm", "forms", "cells"}, {"options"}),
    "transform": ({"word", "cue", "answer"}, {"paradigm", "before", "after", "hint"}),
    "cue_sort": ({"options", "items"}, {"paradigm"}),
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


# Question words (and their forms) that make a question "falling" (Russian IK-2, like a
# statement) when they open it, after any leading conjunction or preposition.
WH_WORDS = set("кто кого кому кем ком что чего чему чем чём где куда откуда когда почему зачем отчего как "
               "какой какая какое какие какого каком какую каким каких какому чей чья чьё чьи сколько "
               "который которая которое которые".split())
LEAD_WORDS = set("а и но ну так о об обо с со в во на к ко у от до из за по для про".split())


def rising_question(s):
    """Same as RussianText.isRisingQuestion: the text ends in a question without a fronted
    question word (yes/no, "А ты?"), which is said with a rise, unlike a statement."""
    t = (s or "").replace(ACUTE, "").replace("+", "").rstrip("»\"”) \n")
    if not t.endswith("?"):
        return False
    last = re.split(r"[.!?…]\s+", t[:-1])[-1]
    words = key(last).split()
    while words and words[0] in LEAD_WORDS:
        words = words[1:]
    return not (words and words[0] in WH_WORDS)


def audio_key(s):
    """Same as RussianText.audioKey: the index key of a clip. A rising question and the
    statement with the same words are different clips ("это чай?" vs "это чай")."""
    return key(s) + ("?" if rising_question(s) else "")


def tokens(s):
    w = "[\u0400-\u04FF\u0301]+"
    return re.findall(w + "(?:-" + w + ")*", s)


def letters_in(s):
    return {c for c in s.replace(ACUTE, "").upper() if CYR.match(c)}


def check_stress(where, s):
    for m in re.finditer("[Ѐ-ӿ́]+(?:-[Ѐ-ӿ́]+)*", s):
        tok = m.group(0)
        # A morpheme written with a hyphen on one side (пере-, -ние) is a fragment, not a word.
        if (m.start() > 0 and s[m.start() - 1] == "-") or (m.end() < len(s) and s[m.end()] == "-"):
            continue
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


def check_tile(where, tile):
    """A morpheme tile ("пере-", "пис", "-а'тель") is a fragment: at most one stress mark, after a vowel."""
    low = tile.replace("-", "").lower()
    if low.count(ACUTE) > 1:
        err(where, f"tile {tile!r} carries more than one stress mark")
    for i, c in enumerate(low):
        if c == ACUTE and (i == 0 or low[i - 1] not in VOWELS):
            err(where, f"acute not after a vowel in tile {tile!r}")


def tile_key(tile):
    return key(tile).replace("-", "").replace(" ", "")


def check_letters(where, s, known):
    extra = letters_in(s) - known
    if extra:
        err(where, f"uses letters not yet met {''.join(sorted(extra))} in {s!r}")


def russian_fields(e):
    """(field label, string) pairs of Russian an exercise shows or speaks."""
    k = e["kind"]
    out = []
    if k in ("trace", "copy", "dictation", "read_aloud", "read_meaning", "passage"):
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
    if k == "build_word":
        out.append(("answer", e["answer"]))
    if k == "family_guess":
        out += [("text", e["text"]), ("word", e["word"])]
    if k == "paradigm_fill":
        out.append(("word", e["word"]))
        out += [(f"forms.{c}", f) for c, f in e["forms"].items()]
        out += [("options", o) for o in e.get("options", [])]
    if k == "transform":
        out += [("word", e["word"]), ("answer", e["answer"]), ("before", e.get("before", "")), ("after", e.get("after", ""))]
    if k == "cue_sort":
        out += [("items", it["text"]) for it in e["items"]]
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
                if k in ("trace", "copy", "dictation", "read_aloud", "read_meaning", "passage"):
                    out.append(e["text"])
                if k == "explain":
                    out += [x["ru"] for x in e.get("examples", [])]
                if k == "listen_pick":
                    out += [e["say"]] + e["options"]
                if k == "translate":
                    out.append(e["answer"])
                if k == "fill":
                    out += [e["answer"], e["before"] + e["answer"] + e["after"]]
                if k == "build_word":
                    out.append(e["answer"])
                if k == "family_guess":
                    out += [e["text"], e["word"]]
                if k == "transform":
                    out.append(e["answer"])
    return [s for s in out if s and CYR.search(s)]


def check_meaning_before_recall(course, lex_keys):
    """Every word the learner must RECALL (translate/fill answer, dictation and read_meaning text)
    was first PRESENTED with its meaning, earlier in course order or earlier on the same page.

    Presented, mirroring where the app shows a gloss: a trace/copy/read_aloud target or a
    listen_pick `say` that is itself a lexicon entry (MeaningTag looks the whole target up), a
    meet card's letter example, every lexicon entry inside an explain example (its English is
    shown beside it), and every word of a passage (each is tappable for its gloss). A recalled
    string passes when it splits into presented lexicon entries, or tokens of presented
    multiword entries (меня from у меня есть, читаем from мы читаем)."""
    letters = {l["upper"]: l for l in course["letters"]}
    shown, shown_tokens = set(), set()

    def show_entry(k):
        if k in lex_keys:
            shown.add(k)
            shown_tokens.update(k.split())

    def show_inside(s):
        toks = key(s).split()
        for i in range(len(toks)):
            for j in range(i + 1, len(toks) + 1):
                show_entry(" ".join(toks[i:j]))

    def missing(s):
        toks = key(s).split()
        best = [None] * (len(toks) + 1)  # best[i]: fewest unshown tokens covering toks[i:]
        best[len(toks)] = []
        for i in range(len(toks) - 1, -1, -1):
            opts = [best[j] for j in range(i + 1, len(toks) + 1) if " ".join(toks[i:j]) in shown]
            if toks[i] in shown_tokens:
                opts.append(best[i + 1])
            opts.append([toks[i]] + best[i + 1])
            best[i] = min(opts, key=len)
        return best[0]

    for u in course["units"]:
        for p in u["pages"]:
            for n, e in enumerate(p["exercises"], 1):
                k = e.get("kind")
                recall = []
                if k == "meet" and e.get("letter") in letters and letters[e["letter"]].get("example"):
                    show_entry(key(letters[e["letter"]]["example"]))
                elif k in ("trace", "copy", "read_aloud"):
                    show_entry(key(e["text"]))
                elif k == "listen_pick":
                    show_entry(key(e["say"]))
                elif k == "explain":
                    for x in e.get("examples", []):
                        show_inside(x["ru"])
                elif k == "passage":
                    show_inside(e["text"])
                    for g in e.get("glosses", {}):
                        shown_tokens.add(key(g))
                elif k == "build_word":
                    show_entry(key(e["answer"]))  # the meaning is given; the word is assembled under it
                elif k == "family_guess":
                    show_entry(key(e["word"]))  # guessed, then told
                    shown_tokens.add(key(e["word"]))
                elif k in ("paradigm_fill", "transform"):
                    recall = [e["word"]]  # the headword must be known; its forms are the system being learned
                    for f in ([e["answer"]] if k == "transform" else list(e["forms"].values())):
                        for part in f.split(" / "):
                            show_entry(key(part))
                            shown_tokens.add(key(part))
                elif k in ("translate", "fill"):
                    recall = [e["answer"]]
                elif k in ("dictation", "read_meaning"):
                    recall = [e["text"]]
                for r in recall:
                    gap = missing(r)
                    if gap:
                        err(f"{p['id']}#{n} {k}", f"recalls {gap} before its meaning was shown (in {r!r})")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    path = Path(args[0]) if args else DEFAULT
    course = json.loads(path.read_text())
    morph_path = path.parent / "morphology.json"
    morphology = json.loads(morph_path.read_text()) if morph_path.exists() else {"roots": [], "lemmas": {}, "forms": {}, "cognates": []}
    roots = {r["id"]: r for r in morphology["roots"]}
    par_path = path.parent / "paradigms.json"
    paradigms = json.loads(par_path.read_text()) if par_path.exists() else {"words": {}, "uses": [], "tables": []}

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
            # The diary line is an appendix to the page: at most one, always last, not counted
            # in the page's size or its writing share.
            diaries = [i for i, e in enumerate(exs) if e.get("kind") == "diary"]
            if len(diaries) > 1 or (diaries and diaries[0] != len(exs) - 1):
                err(pw, "one diary line at most, and it must be the last exercise")
            body = [e for e in exs if e.get("kind") != "diary"]
            if not 6 <= len(body) <= 10:
                err(pw, f"{len(body)} exercises (need 6-10)")
            kinds = [e.get("kind") for e in body]
            writing = sum(is_writing(e) for e in body)
            if writing * 2 <= len(body):
                err(pw, f"writing is {writing}/{len(body)}, not a majority")
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
                if k == "passage":
                    t = e["text"]
                    n = len(re.findall(r"[.!?…]+[»\"]?(?=\s|$)", t))
                    if not 2 <= n <= 8:
                        err(w, f"passage has {n} sentences (need 2-8)")
                    if any(not line.strip() or line != line.strip() for line in t.split("\n")):
                        err(w, "passage lines must be non-empty with no stray spaces")
                    if not e["en"].strip() or CYR.search(e["en"]):
                        err(w, "en must be an English translation")
                    glosses = e.get("glosses", {})
                    for gk, gv in glosses.items():
                        if gk != gk.lower() or ACUTE in gk or key(gk) != gk.replace("-", " ") or not gv.strip():
                            err(w, f"gloss key {gk!r} must be lowercase, unstressed, unpunctuated, with a meaning")
                    gkeys = {key(g) for g in glosses}
                    words = {key(tok.strip(".,?!—–:;…«»\"()")) for tok in t.split()}
                    words.discard("")
                    for g in gkeys - words:
                        err(w, f"gloss {g!r} not in the text")
                    missing = sorted(x for x in words if x not in gkeys and x not in lex_keys)
                    if missing:
                        err(w, f"words with no gloss or lexicon entry: {missing}")
                if k == "diary":
                    if not e["prompt"].strip():
                        err(w, "empty prompt")
                    if BANNED.search(e["prompt"]):
                        err(w, f"voice: {BANNED.search(e['prompt']).group(0)!r}")
                    # Russian in the prompt obeys the same rules as any shown Russian.
                    check_stress(f"{w}.prompt", e["prompt"])
                    check_letters(f"{w}.prompt", e["prompt"], known)
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
                if k == "build_word":
                    tiles = e.get("options", e["parts"])
                    for tile in e["parts"] + tiles:
                        check_tile(w, tile)
                        check_letters(f"{w}.tiles", tile, known)
                    if "".join(tile_key(x) for x in e["parts"]) != tile_key(e["answer"]):
                        err(w, f"parts {e['parts']} do not spell {e['answer']!r}")
                    if not all(x in tiles for x in e["parts"]) or len(set(tiles)) != len(tiles):
                        err(w, "options must contain every part, without repeats")
                    if key(e["answer"]) not in lex_keys:
                        err(w, f"answer {e['answer']!r} not in lexicon")
                    if CYR.search(e["en"]):
                        err(w, "en must be English")
                    for alt in e.get("alternatives", []):
                        check_stress(w + ".alternatives", alt)
                    lemma = morphology["lemmas"].get(key(e["answer"]))
                    cognate = next((c for c in morphology["cognates"] if key(c["ru"]) == key(e["answer"])), None)
                    if lemma:
                        forms = {f for r in lemma["roots"] for f in roots.get(r, {}).get("forms", [])}
                        if forms and not any(tile_key(x) in forms for x in e["parts"]):
                            err(w, f"no tile of {e['parts']} is a root form of {lemma['roots']} ({sorted(forms)})")
                    elif cognate:
                        if cognate.get("parts") and {tile_key(x) for x in e["parts"]} != {tile_key(p["tile"]) for p in cognate["parts"]}:
                            err(w, f"tiles {e['parts']} differ from morphology.json's parts for {e['answer']!r}")
                    else:
                        err(w, f"{e['answer']!r} is neither a lemma nor a cognate in morphology.json")
                if k == "family_guess":
                    wk = key(e["word"])
                    toks = key(e["text"]).split()
                    if wk not in toks:
                        err(w, f"word {e['word']!r} is not in the text")
                    if e["root"] not in roots:
                        err(w, f"unknown root {e['root']!r}")
                    entry = morphology["lemmas"].get(wk) or morphology["lemmas"].get(morphology["forms"].get(wk, ""))
                    if not entry:
                        err(w, f"{e['word']!r} has no morphology entry")
                    elif e["root"] not in entry["roots"]:
                        err(w, f"{e['word']!r} is not built on root {e['root']} (its roots: {entry['roots']})")
                    a = e["answer"]
                    if not isinstance(a, int) or isinstance(a, bool) or not 0 <= a < len(e["options"]):
                        err(w, f"answer index {a!r} out of range")
                    if any(CYR.search(o) for o in e["options"]) or len(set(e["options"])) != len(e["options"]):
                        err(w, "options must be distinct English")
                    if CYR.search(e["en"]):
                        err(w, "en must be an English translation")
                    missing = [x for x in toks if x != wk and x not in lex_tokens]
                    if missing:
                        err(w, f"words around the new one not in lexicon: {missing}")
                if k in ("paradigm_fill", "transform"):
                    par_word = paradigms["words"].get(key(e["word"]))
                    if not par_word:
                        err(w, f"{e['word']!r} is not a paradigms.json word")
                    else:
                        if par_word["ru"] != e["word"]:
                            err(w, f"word {e['word']!r} is written {par_word['ru']!r} in paradigms.json")
                        table_id = e.get("paradigm")
                        own = ({par_word["table"], "past", "imp"} if par_word["pos"] == "verb" else {par_word["table"]}) \
                            | {t["id"] for t in paradigms["tables"] if key(e["word"]) in t["words"]}
                        if table_id and table_id not in own:
                            err(w, f"table {table_id!r} is not {e['word']}'s ({sorted(own)})")
                        cells = e["cells"] if k == "paradigm_fill" else [e["cue"]]
                        for c in cells:
                            if c not in par_word["forms"]:
                                err(w, f"{e['word']} has no cell {c!r}")
                        if k == "paradigm_fill":
                            for c, f in e["forms"].items():
                                if par_word["forms"].get(c) != f:
                                    err(w, f"forms[{c}] = {f!r}, paradigms.json says {par_word['forms'].get(c)!r}")
                            if not set(e["cells"]) <= set(e["forms"]) or not e["cells"]:
                                err(w, "cells must be a non-empty subset of forms")
                            if "options" in e:
                                answers = [e["forms"][c] for c in e["cells"]]
                                if not all(a in e["options"] for a in answers) or len(set(e["options"])) != len(e["options"]):
                                    err(w, "options must hold every asked form once, without repeats")
                        else:
                            want = par_word["forms"].get(e["cue"], "").split(" / ")
                            if e["answer"] not in want:
                                err(w, f"answer {e['answer']!r} is not {e['word']} {e['cue']} = {want}")
                            if ("before" in e) != ("after" in e) or ("before" in e and "hint" not in e):
                                err(w, "a gap needs before, after and hint together")
                            if e.get("before") and not e["before"].endswith(" "):
                                err(w, "before must end with a space")
                            if e.get("after") and not (e["after"][0] in " " or e["after"][0] in PUNCT):
                                err(w, "after must start with a space or punctuation")
                            if e.get("hint") and CYR.search(e["hint"]):
                                err(w, "hint must be English")
                            toks = [t for t in key(e.get("before", "") + " " + e.get("after", "")).split() if t not in lex_tokens]
                            if toks:
                                err(w, f"words around the gap not in lexicon: {toks}")
                if k == "cue_sort":
                    titles = {u["title"] for u in paradigms["uses"]} | {u["id"] for u in paradigms["uses"]} | {t["title"] for t in paradigms["tables"]} | {t["id"] for t in paradigms["tables"]}
                    if len(set(e["options"])) != len(e["options"]) or len(e["options"]) < 2:
                        err(w, "options must be 2+ distinct buckets")
                    for o in e["options"]:
                        if o not in titles:
                            err(w, f"bucket {o!r} is no paradigms.json use or table")
                    if len(e["items"]) < 2:
                        err(w, "at least two items")
                    for it in e["items"]:
                        if set(it) != {"text", "answer"} or not isinstance(it["answer"], int) or not 0 <= it["answer"] < len(e["options"]):
                            err(w, f"item {it!r}")
                        missing = [t for t in key(it["text"]).split() if t not in lex_tokens]
                        if missing:
                            err(w, f"words not in lexicon: {missing}")
                    if len({it["answer"] for it in e["items"]}) < 2:
                        err(w, "items must fall into at least two buckets")
                if k in ("dictation", "translate", "fill", "read_meaning"):
                    s = e["text"] if k in ("dictation", "read_meaning") else (e["before"] + e["answer"] + e["after"] if k == "fill" else e["answer"])
                    missing = [t for t in key(s).split() if t not in lex_tokens]
                    if missing:
                        err(w, f"words not in lexicon: {missing}")
    check_meaning_before_recall(course, lex_keys)
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
                elif k == "build_word":
                    produced.append(key(e["answer"]))
                elif k == "transform":
                    produced.append(key(e["answer"]))
                elif k == "paradigm_fill":
                    produced.append(key(e["word"]))  # the headword's table is filled under it
                    produced += [key(e["forms"][c]) for c in e["cells"]]
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
        for s in sorted({audio_key(s) for s in spoken_strings(course)}):
            f = index.get(s)
            if not f:
                err("audio", f"no recording for {s!r}")
            elif not (audio / f).is_file():
                err("audio", f"missing file {f} for {s!r}")
        sources = json.loads((audio / "sources.json").read_text())
        variants_path = audio / "variants.json"
        variants = json.loads(variants_path.read_text()) if variants_path.exists() else {}
        for k, files in sorted(variants.items()):
            if k not in index:
                err("audio", f"variants key {k!r} has no primary in index.json")
            if len(set(files)) != len(files) or index.get(k) in files:
                err("audio", f"variants for {k!r} repeat a file or the primary")
            for f in files:
                if not (audio / f).is_file():
                    err("audio", f"missing variant file {f} for {k!r}")
                src = sources.get(f)
                if not src:
                    err("audio", f"variant {f} has no sources.json entry")
                elif src.get("key") != k or src.get("kind") != "human_recording" or not (src.get("license") and src.get("source_page")):
                    err("audio", f"variant {f}: sources.json entry lacks key/kind/license/source_page for {k!r}")

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
