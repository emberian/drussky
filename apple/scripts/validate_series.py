#!/usr/bin/env python3
"""Validate apple/Drussky/Resources/series.json against course.json and schema rules.

Checks:
  1. Top-level & per-episode schema (fields, types, numbering, non-decreasing after_page).
  2. Word count per episode (100-250 words).
  3. Stress marks on all Russian strings (U+0301 on every 2+ vowel Cyrillic word without ё;
     never on ё; never on monosyllables; never after a consonant).
  4. Letter progression: episodes before all 33 letters are met use only letters met by after_page.
  5. Gloss coverage: every word in episode.text not yet in the course lexicon / presented by
     after_page is in episode.glosses; every key in episode.glosses is lowercase, unstressed,
     and occurs in episode.text.
  6. Comprehension questions: 3 read_meaning questions per episode, valid answer index,
     distinct English options with no Cyrillic, stressed Russian question text.
  7. Prints a per-episode vocabulary coverage report against the course.
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
COURSE_PATH = ROOT / "Drussky" / "Resources" / "course.json"
SERIES_PATH = ROOT / "Drussky" / "Resources" / "series.json"
LEMMAS_PATH = ROOT / "Drussky" / "Resources" / "lemmas.json"

ACUTE = "\u0301"
VOWELS = set("аеёиоуыэюяАЕЁИОУЫЭЮЯ")
CYRILLIC = set("абвгдеёжзийклмнопрстуфхцчшщъыьэюяАБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ")
PUNCT = ".,!?;:«»—–…()\"' \t\n"

WORD_RE = re.compile(r"[А-Яа-яЁё\u0301]+(?:-[А-Яа-яЁё\u0301]+)*")


def unstressed(s: str) -> str:
    return s.replace(ACUTE, "")


def key(s: str) -> str:
    return unstressed(s).strip(PUNCT).lower()


def words(s: str) -> list[str]:
    return WORD_RE.findall(s)


def letters_in(s: str) -> set[str]:
    return {ch.upper() for ch in unstressed(s) if ch in CYRILLIC}


def check_stress(text: str, where: str, errs: list[str]) -> None:
    for w in words(text):
        plain = unstressed(w)
        vowels = [ch for ch in plain if ch in VOWELS]
        acutes = [i for i, ch in enumerate(w) if ch == ACUTE]
        has_yo = "ё" in plain or "Ё" in plain
        if len(acutes) > 1:
            errs.append(f"{where}: multiple stress marks in '{w}'")
        for i in acutes:
            if i == 0 or w[i - 1] not in VOWELS:
                errs.append(f"{where}: stress mark not after vowel in '{w}'")
        if has_yo and acutes:
            errs.append(f"{where}: redundant acute on ё-word '{w}'")
        if len(vowels) >= 2 and not has_yo and not acutes:
            errs.append(f"{where}: missing stress mark on '{w}' in {text!r}")
        if len(vowels) <= 1 and acutes:
            errs.append(f"{where}: stress mark on monosyllable '{w}'")


def build_course_snapshots(course: dict) -> tuple[dict[str, int], dict[str, set[str]], dict[str, set[str]], set[str]]:
    """Return (page_index, letters_by_page, words_by_page, lexicon_single_words)."""
    lex_all = {key(w["ru"]) for w in course.get("lexicon", [])}
    lex_single = {k for k in lex_all if " " not in k}

    page_index: dict[str, int] = {}
    letters_by_page: dict[str, set[str]] = {}
    words_by_page: dict[str, set[str]] = {}

    met_letters: set[str] = set()
    presented_words: set[str] = set()
    idx = 0

    for unit in course.get("units", []):
        for page in unit.get("pages", []):
            idx += 1
            pid = page["id"]
            page_index[pid] = idx
            for ex in page.get("exercises", []):
                kind = ex.get("kind")
                if kind == "meet":
                    met_letters.add(ex["letter"])
                    letter_card = next(
                        (l for l in course.get("letters", []) if l["upper"] == ex["letter"]),
                        None,
                    )
                    if letter_card and letter_card.get("example"):
                        ex_word = letter_card["example"]
                        if letters_in(ex_word) <= met_letters:
                            presented_words.add(key(ex_word))
                elif kind in ("trace", "copy", "read_aloud", "listen_pick", "dictation", "translate", "fill", "read_meaning", "build_word", "family_guess"):
                    texts = []
                    if kind == "listen_pick":
                        texts.append(ex.get("say", ""))
                        texts.extend(ex.get("options", []))
                    elif kind in ("translate", "build_word"):
                        texts.append(ex.get("answer", ""))
                    elif kind == "fill":
                        texts.extend([ex.get("before", ""), ex.get("answer", ""), ex.get("after", "")])
                    elif kind == "family_guess":
                        texts.append(ex.get("text", ""))
                        if ex.get("word"):
                            texts.append(ex["word"])
                    else:
                        texts.append(ex.get("text", ""))
                    for t in texts:
                        if t:
                            presented_words.add(key(t))
                            for w in words(t):
                                presented_words.add(key(w))
                elif kind == "explain":
                    for example in ex.get("examples", []):
                        ru = example.get("ru", "")
                        presented_words.add(key(ru))
                        for w in words(ru):
                            presented_words.add(key(w))
                elif kind == "passage":
                    for w in words(ex.get("text", "")):
                        presented_words.add(key(w))
                    for gk in (ex.get("glosses") or {}):
                        presented_words.add(gk)

            letters_by_page[pid] = set(met_letters)
            # A word is known in the course lexicon by `pid` if it has been presented by `pid`.
            words_by_page[pid] = set(presented_words)

    return page_index, letters_by_page, words_by_page, lex_single


def validate() -> int:
    errs: list[str] = []

    if not COURSE_PATH.exists():
        print(f"MISSING {COURSE_PATH}", file=sys.stderr)
        return 1
    if not SERIES_PATH.exists():
        print(f"MISSING {SERIES_PATH}", file=sys.stderr)
        return 1

    course = json.loads(COURSE_PATH.read_text(encoding="utf-8"))
    data = json.loads(SERIES_PATH.read_text(encoding="utf-8"))
    lemmas_map: dict[str, str] = {}
    if LEMMAS_PATH.exists():
        # forms maps an inflected form to [lemma, construction]; older files mapped to the lemma string.
        forms = json.loads(LEMMAS_PATH.read_text(encoding="utf-8")).get("forms", {})
        lemmas_map = {k: (v[0] if isinstance(v, list) else v) for k, v in forms.items()}

    page_index, letters_by_page, words_by_page, lex_single = build_course_snapshots(course)

    if data.get("version") != 1:
        errs.append(f"top-level version must be 1, got {data.get('version')!r}")

    series_list = data.get("series")
    if not isinstance(series_list, list) or not series_list:
        errs.append("top-level 'series' must be a non-empty list")
        series_list = []

    for s in series_list:
        sid = s.get("id", "<missing-id>")
        if not isinstance(s.get("id"), str) or not s["id"]:
            errs.append("series missing non-empty 'id'")
        if not isinstance(s.get("title"), str) or not s["title"]:
            errs.append(f"series {sid}: missing 'title'")
        else:
            check_stress(s["title"], f"series {sid} title", errs)
        if not isinstance(s.get("title_en"), str) or not s["title_en"]:
            errs.append(f"series {sid}: missing 'title_en'")
        if not isinstance(s.get("summary_en"), str) or not s["summary_en"]:
            errs.append(f"series {sid}: missing 'summary_en'")

        episodes = s.get("episodes")
        if not isinstance(episodes, list) or len(episodes) < 24:
            errs.append(f"series {sid}: expected at least 24 episodes, got {len(episodes) if isinstance(episodes, list) else type(episodes).__name__}")
            if not isinstance(episodes, list):
                continue

        print(f"Series «{unstressed(s.get('title', sid))}» ({s.get('title_en', '')}) — {len(episodes)} episodes")
        print(f"{'Ep':>3}  {'ID':<13} {'After':<8} {'Pg#':>3}  {'Words':>5}  {'Uniq':>4}  {'Course%':>7}  {'Course+Lem%':>11}  {'Glossed':>7}  {'Reused':>6}")
        print("-" * 82)

        seen_ids: set[str] = set()
        prev_page_num = 0
        prior_glossed: set[str] = set()
        total_words = 0

        for idx, ep in enumerate(episodes, start=1):
            loc = f"{ep.get('id', f'ep#{idx}')}"
            eid = ep.get("id")
            if not isinstance(eid, str) or not eid:
                errs.append(f"episode #{idx}: missing 'id'")
            elif eid in seen_ids:
                errs.append(f"{loc}: duplicate episode id '{eid}'")
            else:
                seen_ids.add(eid)

            if ep.get("number") != idx:
                errs.append(f"{loc}: expected number={idx}, got {ep.get('number')!r}")

            for field in ("title", "title_en", "after_page", "text", "en", "prompt"):
                if not isinstance(ep.get(field), str) or not ep.get(field, "").strip():
                    errs.append(f"{loc}: missing or empty '{field}'")

            after_page = ep.get("after_page", "")
            if after_page not in page_index:
                errs.append(f"{loc}: after_page '{after_page}' is not a valid page id in course.json")
                pnum = prev_page_num
                known_letters = set(CYRILLIC)
                known_words = set()
            else:
                pnum = page_index[after_page]
                if pnum < prev_page_num:
                    errs.append(f"{loc}: after_page '{after_page}' (page #{pnum}) is earlier than previous episode (page #{prev_page_num})")
                prev_page_num = pnum
                known_letters = letters_by_page[after_page]
                known_words = words_by_page[after_page]

            if isinstance(ep.get("title"), str):
                check_stress(ep["title"], f"{loc} title", errs)

            text = ep.get("text", "")
            if isinstance(text, str) and text.strip():
                check_stress(text, f"{loc} text", errs)
                unmet = letters_in(text) - known_letters
                if unmet:
                    errs.append(f"{loc}: text uses letters not yet met by {after_page}: {sorted(unmet)}")

            en_text = ep.get("en", "")
            if isinstance(en_text, str) and (set(en_text) & CYRILLIC):
                errs.append(f"{loc}: 'en' translation contains Cyrillic characters")

            prompt_text = ep.get("prompt", "")
            if isinstance(prompt_text, str) and prompt_text:
                check_stress(prompt_text, f"{loc} prompt", errs)

            ep_words = words(text)
            wcount = len(ep_words)
            total_words += wcount
            if not (100 <= wcount <= 250):
                errs.append(f"{loc}: word count must be 100..250, got {wcount}")

            glosses = ep.get("glosses")
            if not isinstance(glosses, dict):
                errs.append(f"{loc}: 'glosses' must be a dict")
                glosses = {}

            ep_word_keys = [key(w) for w in ep_words if key(w)]
            ep_unique_keys = set(ep_word_keys)

            for gk, gv in glosses.items():
                if not gk or ACUTE in gk or gk != gk.lower() or gk != key(gk):
                    errs.append(f"{loc}: gloss key {gk!r} must be lowercase, unstressed, and unpunctuated")
                if not isinstance(gv, str) or not gv.strip():
                    errs.append(f"{loc}: empty gloss for {gk!r}")
                elif set(gv) & CYRILLIC:
                    errs.append(f"{loc}: gloss value for {gk!r} contains Cyrillic: {gv!r}")
                if gk not in ep_unique_keys:
                    errs.append(f"{loc}: gloss key {gk!r} does not appear in episode text")

            # Every word in the episode text not yet in the course by after_page must be glossed.
            # Also ensure that if a word is not a single-word entry in course.lexicon, it is glossed
            # so the in-app tap lookup (which checks episode.glosses then course.lexicon) always succeeds.
            course_known_here = known_words & lex_single
            for w in ep_words:
                wk = key(w)
                if not wk:
                    continue
                if wk not in course_known_here and wk not in glosses:
                    errs.append(f"{loc}: word '{w}' (key '{wk}') not in course lexicon by {after_page} and missing from glosses")

            # Questions validation
            questions = ep.get("questions")
            if not isinstance(questions, list) or len(questions) != 3:
                errs.append(f"{loc}: expected exactly 3 comprehension questions, got {len(questions) if isinstance(questions, list) else type(questions).__name__}")
                questions = questions if isinstance(questions, list) else []

            for qi, q in enumerate(questions):
                qloc = f"{loc} q#{qi + 1}"
                if not isinstance(q, dict):
                    errs.append(f"{qloc}: question must be an object")
                    continue
                if q.get("kind") != "read_meaning":
                    errs.append(f"{qloc}: kind must be 'read_meaning', got {q.get('kind')!r}")
                qtext = q.get("text", "")
                if not isinstance(qtext, str) or not qtext.strip():
                    errs.append(f"{qloc}: missing or empty 'text'")
                else:
                    check_stress(qtext, qloc, errs)
                    unmet_q = letters_in(qtext) - known_letters
                    if unmet_q:
                        errs.append(f"{qloc}: question uses letters not yet met by {after_page}: {sorted(unmet_q)}")
                opts = q.get("options")
                ans = q.get("answer")
                if not isinstance(opts, list) or len(opts) < 3:
                    errs.append(f"{qloc}: 'options' must be a list of at least 3 strings")
                else:
                    if len(set(opts)) != len(opts):
                        errs.append(f"{qloc}: duplicate option strings in {opts!r}")
                    for opt in opts:
                        if not isinstance(opt, str) or not opt.strip():
                            errs.append(f"{qloc}: empty option in {opts!r}")
                        elif set(opt) & CYRILLIC:
                            errs.append(f"{qloc}: English option contains Cyrillic: {opt!r}")
                    if type(ans) is not int or not (0 <= ans < len(opts)):
                        errs.append(f"{qloc}: answer index {ans!r} out of range for {len(opts)} options")

            # Coverage statistics
            in_course_tokens = sum(1 for wk in ep_word_keys if wk in known_words)
            in_course_or_lemma_tokens = sum(
                1
                for wk in ep_word_keys
                if wk in known_words or lemmas_map.get(wk, wk) in known_words
            )
            course_pct = (100.0 * in_course_tokens / len(ep_word_keys)) if ep_word_keys else 0.0
            lemma_pct = (100.0 * in_course_or_lemma_tokens / len(ep_word_keys)) if ep_word_keys else 0.0
            reused_count = len(set(glosses.keys()) & prior_glossed)
            prior_glossed.update(glosses.keys())

            print(
                f"{idx:>3}  {eid:<13} {after_page:<8} {pnum:>3}  {wcount:>5}  {len(ep_unique_keys):>4}  "
                f"{course_pct:>6.1f}%  {lemma_pct:>10.1f}%  {len(glosses):>7}  {reused_count:>6}"
            )

        print("-" * 82)
        print(f"Total words across {len(episodes)} episodes: {total_words} (avg {total_words / max(len(episodes), 1):.1f}/ep), unique glossed forms: {len(prior_glossed)}")

    if errs:
        print(f"\nFAILED ({len(errs)} error(s)):", file=sys.stderr)
        for e in errs:
            print(f"  - {e}", file=sys.stderr)
        return 1

    print("\nOK: series.json passed all schema, stress, letter, gloss, and question checks.")
    return 0


if __name__ == "__main__":
    sys.exit(validate())
