#!/usr/bin/env python3
"""Check Drussky/Resources/library.json (and PrivatePack/library.json when present).

  python3 apple/scripts/validate_library.py
  python3 apple/scripts/validate_library.py apple/Drussky/Resources/math.json

Schema and lengths for texts and sentences; every sentence clip exists and has provenance in its
audio directory's sources.json; and nothing in the bundled (redistributed) library carries a
non-redistributable license. Exit status 1 on any error.
"""
import json
import re
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
PUBLIC = APPLE / "Drussky/Resources"
PRIVATE = APPLE / "PrivatePack"
LEVELS = {"A1", "A2", "B1", "B2", "C1", "C2"}
TOPICS = {"classic", "scifi", "math", "science", "philosophy", "everyday"}
MAX_TEXT = 3000
MAX_WORDS = 10
TEXT_FIELDS = {"id": str, "title": str, "author": str, "level": str, "topic": str, "text": str, "license": str, "source": str}
OPTIONAL_TEXT = {"title_en": str, "year": int, "summary_en": str, "narration": str, "words": str, "reader": str,
                 # Edition facts behind the licence claim, and the narration's own rights basis (credits.json).
                 "publication": str, "narration_license": str}
# Optional authored-reading support, checked in detail by validate_math.py.
OPTIONAL_TEXT.update({"rung": int, "after_page": str, "en": str, "glosses": dict,
                      "questions": list, "how_read": list, "notation": list, "references": list})
SENTENCE_FIELDS = {"ru": str, "en": str}
OPTIONAL_SENTENCE = {"audio": str, "speaker": str, "license": str, "source": str,
                     # Tatoeba's own record of who wrote each half and under what licence (scripts/fetch_provenance.py).
                     "ru_author": str, "en_author": str, "ru_license": str, "en_license": str, "en_source": str}
# A license string is redistributable when it names one of these and none of the restrictions.
FREE = re.compile(r"^public domain|\bCC0\b|\bCC BY(-SA)? \d", re.I)
NOT_FREE = re.compile(r"\bNC\b|\bND\b|-NC|-ND|non-?commercial|no ?deriv|personal|all rights reserved|copyright", re.I)
CYRILLIC = re.compile(r"[Ѐ-ӿ]")


def redistributable(lic):
    parts = [p for p in re.split(r";\s*(?=(?:text|audio)\s)", lic) if p and p.strip()]
    return bool(parts) and not NOT_FREE.search(lic) and all(FREE.search(re.sub(r"^(text|audio)\s+", "", p.strip())) for p in parts)


def words(s):
    return re.findall(r"[Ѐ-ӿ́-]+", s)


def check_narration(root, t, tag, public, err):
    """A narrated text: clip registered in index-narration.json with provenance, word timings that match the text."""
    from narration import check_words
    audio = root / "Audio"
    if not (t.get("narration") and t.get("words") and t.get("reader")):
        err(f"{tag}: narration needs 'narration', 'words' and 'reader' together")
        return
    index = json.loads((audio / "index-narration.json").read_text()) if (audio / "index-narration.json").exists() else {}
    sources = json.loads((audio / "sources-narration.json").read_text()) if (audio / "sources-narration.json").exists() else {}
    if index.get(f"narration:{t['id']}") != t["narration"]:
        err(f"{tag}: {t['narration']} is not index-narration.json's clip for narration:{t['id']}")
    src = sources.get(t["narration"])
    if not src:
        err(f"{tag}: {t['narration']} has no sources-narration.json entry")
    else:
        for f in ("license", "source_page", "reader", "cut", "orig_url"):
            if not src.get(f):
                err(f"{tag}: sources-narration.json entry lacks '{f}'")
        if public and not redistributable(src.get("license", "")):
            err(f"{tag}: narration license {src.get('license')!r} is not redistributable")
    wp = audio / t["words"]
    if not wp.is_file():
        err(f"{tag}: word timings {t['words']} missing")
        return
    timing = json.loads(wp.read_text())
    if timing.get("file") != t["narration"]:
        err(f"{tag}: {t['words']} is for {timing.get('file')}, not {t['narration']}")
    for p in check_words(t["text"], timing):
        err(f"{tag}: {p}")


def check(root, public, errors, notes, filename="library.json"):
    path = root / filename
    try:
        where = str(path.relative_to(APPLE))
    except ValueError:
        where = str(path)
    if not path.exists():
        if public:
            errors.append(f"{where}: missing")
        return
    try:
        lib = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        errors.append(f"{where}: not JSON ({e})")
        return
    err = lambda m: errors.append(f"{where}: {m}")
    if lib.get("version") != 1:
        err("version must be 1")
    if not isinstance(lib.get("texts"), list):
        err("texts must be a list")
        lib["texts"] = []
    ids = set()
    for i, t in enumerate(lib["texts"]):
        tag = f"texts[{i}] {t.get('id', '?')}"
        for f, ty in TEXT_FIELDS.items():
            if not isinstance(t.get(f), ty) or (ty is str and not t[f].strip()):
                err(f"{tag}: '{f}' must be a non-empty {ty.__name__}")
        for f, ty in OPTIONAL_TEXT.items():
            if f in t and not isinstance(t[f], ty):
                err(f"{tag}: '{f}' must be {ty.__name__}")
        for f in set(t) - set(TEXT_FIELDS) - set(OPTIONAL_TEXT):
            err(f"{tag}: unknown field '{f}'")
        if not all(isinstance(t.get(f), str) for f in TEXT_FIELDS):
            continue
        if not re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*", t["id"]):
            err(f"{tag}: id must be lowercase-hyphenated ascii")
        if t["id"] in ids:
            err(f"{tag}: duplicate id")
        ids.add(t["id"])
        if t["level"] not in LEVELS:
            err(f"{tag}: level {t['level']!r} not in {sorted(LEVELS)}")
        if t["topic"] not in TOPICS:
            err(f"{tag}: topic {t['topic']!r} not in {sorted(TOPICS)}")
        if len(t["text"]) > MAX_TEXT:
            err(f"{tag}: text is {len(t['text'])} chars (max {MAX_TEXT})")
        if len(CYRILLIC.findall(t["text"])) < 0.5 * len(re.findall(r"\w", t["text"])):
            err(f"{tag}: text is not mostly Cyrillic")
        # Inequalities and absolute-value/conditional bars are mathematical notation.
        markup = r"[{}]|\[\d+\]|\[\[|&[a-z]+;|''" if t["topic"] == "math" else r"[<>{}|]|\[\d+\]|\[\[|&[a-z]+;|''"
        if re.search(markup, t["text"]) or re.search(r"</?[A-Za-z][^>]*>", t["text"]):
            err(f"{tag}: text has leftover markup")
        if "\n\n" in t["text"] or t["text"] != t["text"].strip():
            err(f"{tag}: paragraphs are separated by a single \\n, no surrounding whitespace")
        if not re.match(r"https://", t["source"]):
            err(f"{tag}: source must be an https URL")
        if "year" in t and not 1000 <= t["year"] <= 2100:
            err(f"{tag}: implausible year {t['year']}")
        if public and not redistributable(t["license"]):
            err(f"{tag}: license {t['license']!r} is not redistributable; it belongs in PrivatePack")
        if "narration" in t or "words" in t:
            check_narration(root, t, tag, public, err)
    sentences = lib.get("sentences", [])
    if not isinstance(sentences, list):
        err("sentences must be a list")
        sentences = []
    audio = root / "Audio"
    sources = json.loads((audio / "sources.json").read_text()) if (audio / "sources.json").exists() else {}
    seen = set()
    speakers = set()
    for i, s in enumerate(sentences):
        tag = f"sentences[{i}] {s.get('ru', '?')!r}"
        for f, ty in SENTENCE_FIELDS.items():
            if not isinstance(s.get(f), ty) or not s[f].strip():
                err(f"{tag}: '{f}' must be a non-empty string")
        for f, ty in OPTIONAL_SENTENCE.items():
            if f in s and not isinstance(s[f], ty):
                err(f"{tag}: '{f}' must be {ty.__name__}")
        for f in set(s) - set(SENTENCE_FIELDS) - set(OPTIONAL_SENTENCE):
            err(f"{tag}: unknown field '{f}'")
        if not isinstance(s.get("ru"), str):
            continue
        if len(words(s["ru"])) > MAX_WORDS:
            err(f"{tag}: more than {MAX_WORDS} words")
        if s["ru"] in seen:
            err(f"{tag}: duplicate sentence")
        seen.add(s["ru"])
        if public and not redistributable(s.get("license", "")):
            err(f"{tag}: license {s.get('license')!r} is not redistributable; it belongs in PrivatePack")
        if "audio" in s:
            speakers.add(s.get("speaker"))
            if not (audio / s["audio"]).is_file():
                err(f"{tag}: audio {s['audio']} missing from {audio.relative_to(APPLE)}")
            src = sources.get(s["audio"])
            if not src:
                err(f"{tag}: audio {s['audio']} has no sources.json entry")
            else:
                for f in ("license", "source_page"):
                    if not src.get(f):
                        err(f"{tag}: sources.json entry lacks '{f}'")
                if public and not redistributable(src.get("license", "")):
                    err(f"{tag}: audio license {src.get('license')!r} is not redistributable")
                if src.get("license") and src["license"] not in s.get("license", ""):
                    err(f"{tag}: sentence license does not mention the audio license {src['license']!r}")
    by_level = {}
    for t in lib["texts"]:
        by_level[t.get("level")] = by_level.get(t.get("level"), 0) + 1
    narrated = [t for t in lib["texts"] if "narration" in t]
    notes.append(f"{where}: {len(lib['texts'])} texts {dict(sorted(by_level.items()))}, {len(narrated)} narrated, "
                 f"{len(sentences)} sentences, {len([s for s in sentences if 'audio' in s])} with audio, {len(speakers)} speakers")


def main():
    errors, notes = [], []
    if len(sys.argv) > 1:
        path = Path(sys.argv[1]).resolve()
        check(path.parent, True, errors, notes, filename=path.name)
    else:
        check(PUBLIC, True, errors, notes)
        check(PRIVATE, False, errors, notes)
    for n in notes:
        print(n)
    for e in errors:
        print("ERROR", e)
    print("clean" if not errors else f"{len(errors)} errors")
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
