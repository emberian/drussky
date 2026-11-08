#!/usr/bin/env python3
"""The Claude half of the on-device tutor benchmark, and the scorer.

  set -a; source ~/dev/allgame/.env; set +a
  uv run --with anthropic research/ondevice/claude_side.py answer   # -> claude.json (what the app's Tutor.stream would say)
  uv run --with anthropic research/ondevice/claude_side.py score    # -> scores.json + a report on stdout

`answer` sends each case the way the app does (apple/Drussky/Services/Tutor.swift: claude-opus-5-5,
effort low, server-side fallbacks; the diary and lemma system prompts are the app's own). `score`
grades both sides: glosses and ending answers by a Claude judge against the reference (the same judge
for both sides, blind to which is which), diary edits and lemmas programmatically against the keys.
"""
import json
import re
import sys
import time
import unicodedata
from pathlib import Path

import anthropic

HERE = Path(__file__).resolve().parent
CASES = json.loads((HERE / "cases.json").read_text())
MODEL = "claude-opus-5-5"
STRESS = "́"

client = anthropic.Anthropic()


def key(s: str) -> str:
    return unicodedata.normalize("NFC", s.replace(STRESS, "")).lower().replace("ё", "е").strip()


def ask(system: str, user: str, effort="low", max_tokens=4000) -> tuple[str, float]:
    t0 = time.time()
    r = client.beta.messages.create(
        model=MODEL, max_tokens=max_tokens, system=system,
        messages=[{"role": "user", "content": user}],
        output_config={"effort": effort},
        betas=["server-side-fallback-2026-07-01"], fallbacks="default",
    )
    if r.stop_reason == "refusal":
        return "", time.time() - t0
    return "".join(b.text for b in r.content if b.type == "text"), time.time() - t0


def extract_json(s: str):
    start = s.find("{")
    if start < 0:
        start = s.find("[")
    if start < 0:
        return None
    depth, in_str, esc = 0, False, False
    opener = s[start]
    closer = "}" if opener == "{" else "]"
    for i in range(start, len(s)):
        c = s[i]
        if in_str:
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c in "{[":
            depth += 1
        elif c in "}]":
            depth -= 1
            if depth == 0:
                try:
                    return json.loads(s[start:i + 1])
                except json.JSONDecodeError:
                    return None
    return None


# --- the app's prompts (kept in step with Tutor.swift / DiaryTutor.swift / GoalLemmaTable.swift) ---

TUTOR_SYSTEM = """You are the tutor inside Drussky, a Russian exercise book used by one adult learner, Ember: quick, curious, a beginner in Russian, interested in linguistics, science fiction, mathematics and philosophy. Working memory is sometimes stretched, so keep answers short and concrete: 2–6 sentences, or a tiny table/list when it really helps. Answer in English. Write every Russian word in Cyrillic with a stress mark (´) on words of two or more syllables, and give a rough pronunciation only when asked or when it is the point. Prefer one vivid example over a rule list. If the learner's answer was wrong, say exactly what differs. Don't use letters they haven't met unless needed; if you do, say so briefly. No preamble, no sign-off. Mark every Russian word or phrase in your answer as [[ру́сский|english]], with stress marks: the learner taps it to hear it and can keep it in their workbook. Use **bold** and short lists freely.

Where the learner is right now:
{context}"""

GLOSS_SYSTEM = """You are a Russian glossary. You receive one Russian word and the sentence it appears in. Reply with ONE JSON object and nothing else: {"gloss": "1–4 English words for the meaning in this context", "lemma": "dictionary form in Cyrillic"}."""

DIARY_SYSTEM = """You are the tutor inside Drussky, a Russian exercise book used by one adult English speaker, Ember: quick, curious, a beginner in Russian. They have just written a few words of their OWN Russian in their diary, with a pencil. Correct it the way a kind, exact teacher marks homework in red pen. Reply with ONE JSON object and nothing else:
{"corrected": "the whole text, corrected, with stress marks (U+0301) on every word of 2+ syllables, never on ё",
 "edits": [{"span": "their words exactly as written", "replacement": "the right form, stress-marked ('' to delete)",
            "category": "spelling|ending|agreement|word-choice|word-order|stress|punctuation|other",
            "why": "one line in English saying what differs and why", "en": "English meaning of the replacement",
            "pattern": "a construction id from the list below when one explains it, else omit"}],
 "praise": "one specific, warm sentence about what they got right",
 "teach": [{"title": "the pattern behind the biggest error, in 3-6 words",
            "note": "at most 2 sentences teaching it, with one example marked [[ру́сский|english]]",
            "pattern": "construction id when one fits, else a short slug"}]}

Rules: keep their meaning and their words wherever they are right; fix what is wrong, never restyle. Every `span` must be a verbatim substring of their text (a word or a few words), and the edits must not overlap. Handwriting recognition may have mangled a letter: if the error looks like a recognizer slip (a Latin look-alike, a lost soft sign), still correct it, category spelling, and say so in `why`. If the text is already correct, `edits` is [] and `teach` is []. 0–2 teach items; teach the pattern behind the biggest slip, not a list of rules. No preamble, no sign-off."""

LEMMA_SYSTEM = """You are a Russian lemmatizer and glossary. You receive Russian word forms, one per line, lowercase, without stress marks. Reply with ONE JSON object and nothing else: each input form as a key, its value a two-element array [lemma, gloss]. The lemma is the dictionary form (nominative singular for nouns and adjectives — masculine for adjectives —, infinitive for verbs including participles and gerunds, with ё where it belongs); the gloss is 1–4 English words for the word's usual meaning. Proper names: lemma is the nominative, gloss "name". Keep every key exactly as given."""

BRIEFING = "Letters met: all of them.\nWords they know: мама, папа, брат, книга, читать, школа, дом, друг, кофе, чай, парк.\nConstruction ids: none."


def answer():
    out = {"glosses": [], "endings": [], "diaries": [], "lemmas": []}
    for c in CASES["glosses"]:
        text, secs = ask(GLOSS_SYSTEM, f"Sentence: {c['context']}\nWord: {c['word']}", max_tokens=300)
        j = extract_json(text) or {}
        out["glosses"].append({"id": c["id"], "seconds": secs, "answer": j.get("gloss"), "lemma": j.get("lemma")})
        print(c["id"], c["word"], "→", j.get("gloss"), "|", j.get("lemma"), f"{secs:.1f}s")
    for c in CASES["endings"]:
        ctx = f"The sentence on the page: {c['sentence']}"
        text, secs = ask(TUTOR_SYSTEM.format(context=ctx), f"Why does «{c['word']}» have this ending here?")
        out["endings"].append({"id": c["id"], "seconds": secs, "answer": text})
        print(c["id"], c["word"], "→", text[:160].replace("\n", " "), f"{secs:.1f}s")
    for c in CASES["diaries"]:
        user = f"{BRIEFING}\n\nThe diary prompt was: “{c['prompt']}”\nThey wrote:\n---\n{c['text']}\n---"
        text, secs = ask(DIARY_SYSTEM, user, effort="medium")
        j = extract_json(text)
        out["diaries"].append({"id": c["id"], "seconds": secs, "json": json.dumps(j, ensure_ascii=False) if j else None,
                               "error": None if j else "unreadable"})
        print(c["id"], "→", (json.dumps(j, ensure_ascii=False) if j else text)[:300], f"{secs:.1f}s")
    forms = [c["form"] for c in CASES["lemmas"]]
    text, secs = ask(LEMMA_SYSTEM, "\n".join(forms), max_tokens=4000)
    j = extract_json(text) or {}
    for c in CASES["lemmas"]:
        v = j.get(c["form"])
        lemma, gloss = (v[0], v[1] if len(v) > 1 else None) if isinstance(v, list) and v else (v if isinstance(v, str) else None, None)
        out["lemmas"].append({"id": c["id"], "seconds": secs / len(forms), "lemma": lemma, "answer": gloss,
                              "error": None if lemma else "form missing from table"})
        print(c["id"], c["form"], "→", lemma, "|", gloss, "gold", c["gold"])
    (HERE / "claude.json").write_text(json.dumps(out, ensure_ascii=False, indent=1))
    print("wrote", HERE / "claude.json")


# --- scoring ---

def by_id(results):
    return {r["id"]: r for r in results}


def judge(items: list[dict], rubric: str) -> dict:
    """items: [{id, ...fields}] -> {id: verdict}. One call, blind to the backend."""
    system = ("You grade short answers from a Russian tutor against a reference. Reply with ONE JSON object and nothing "
              "else: each item id as a key, its value one of the allowed verdicts. " + rubric)
    text, _ = ask(system, json.dumps(items, ensure_ascii=False), effort="medium", max_tokens=4000)
    return extract_json(text) or {}


GLOSS_RUBRIC = ('Verdicts: "ok" when the candidate gloss gives the same meaning as the reference for this word in this '
                'sentence (wording may differ; extra detail is fine; a wrong case/preposition note does not matter if the '
                'meaning is right), "wrong" otherwise, including an empty or missing candidate.')

ENDING_RUBRIC = ('Verdicts: "correct" when the answer names the same grammatical form (case/person/gender etc.) AND the '
                 'same reason as the key, "partial" when the form is right but the reason is missing or wrong (or vice '
                 'versa), "wrong" when the form is wrong, the answer contradicts the key, or it is empty. Judge the '
                 'grammar, not the style.')


def score_diary(cases, results):
    rows = []
    for c in cases:
        r = results.get(c["id"], {})
        j = json.loads(r["json"]) if r.get("json") else None
        planted = c["planted"]
        if not j:
            rows.append({"id": c["id"], "caught": 0, "planted": len(planted), "false": 0, "corrected_ok": False, "unreadable": True})
            continue
        edits = j.get("edits") or []
        caught = 0
        used = set()
        for p in planted:
            for i, e in enumerate(edits):
                if i in used:
                    continue
                span, rep = key(e.get("span", "")), key(e.get("replacement", ""))
                if key(p["span"]) in span and any(key(x) in rep for x in p["replacement"]):
                    caught += 1
                    used.add(i)
                    break
        false = len(edits) - len(used)
        corrected_ok = key(j.get("corrected", "")).rstrip(".") == key(c["gold"]).rstrip(".")
        if not corrected_ok and planted and len(planted[0]["replacement"]) > 1:
            # alternatives (книги/книгу): accept the gold with the other replacement
            alt = key(c["gold"]).replace(key(planted[0]["replacement"][0]), key(planted[0]["replacement"][1]))
            corrected_ok = key(j.get("corrected", "")).rstrip(".") == alt.rstrip(".")
        rows.append({"id": c["id"], "caught": caught, "planted": len(planted), "false": false,
                     "corrected_ok": corrected_ok, "unreadable": False, "seconds": r.get("seconds")})
    return rows


def score_lemmas(cases, results):
    rows = []
    for c in cases:
        r = results.get(c["id"], {})
        got = key(r.get("lemma") or "")
        rows.append({"id": c["id"], "form": c["form"], "got": got, "ok": got in [key(g) for g in c["gold"]], "seconds": r.get("seconds")})
    return rows


def mean(xs):
    xs = [x for x in xs if x is not None]
    return sum(xs) / len(xs) if xs else 0


def score():
    od = json.loads((HERE / "ondevice.json").read_text())
    cl = json.loads((HERE / "claude.json").read_text())
    report, scores = [], {}

    # glosses: one blind judge call over both backends' candidates
    items = []
    for side, res in (("od", by_id(od.get("glosses", []))), ("cl", by_id(cl["glosses"]))):
        for c in CASES["glosses"]:
            r = res.get(c["id"], {})
            items.append({"id": f"{side}:{c['id']}", "word": c["word"], "sentence": c["context"],
                          "reference": c["reference"], "candidate": r.get("answer") or ""})
    verdicts = judge(items, GLOSS_RUBRIC)
    for side, name, res in (("od", "on-device", by_id(od.get("glosses", []))), ("cl", "claude", by_id(cl["glosses"]))):
        ok = sum(1 for c in CASES["glosses"] if verdicts.get(f"{side}:{c['id']}") == "ok")
        errs = sum(1 for c in CASES["glosses"] if res.get(c["id"], {}).get("error"))
        lat = mean([res.get(c["id"], {}).get("seconds") for c in CASES["glosses"]])
        scores[f"gloss/{name}"] = {"ok": ok, "n": len(CASES["glosses"]), "errors": errs, "mean_seconds": lat}
        report.append(f"gloss     {name:10s} {ok}/{len(CASES['glosses'])} ok, {errs} errors, {lat:.1f}s mean")
    scores["gloss/verdicts"] = verdicts

    items = []
    for side, res in (("od", by_id(od.get("endings", []))), ("cl", by_id(cl["endings"]))):
        for c in CASES["endings"]:
            r = res.get(c["id"], {})
            items.append({"id": f"{side}:{c['id']}", "sentence": c["sentence"], "word": c["word"],
                          "key": c["key"], "answer": r.get("answer") or ""})
    verdicts = judge(items, ENDING_RUBRIC)
    for side, name, res in (("od", "on-device", by_id(od.get("endings", []))), ("cl", "claude", by_id(cl["endings"]))):
        v = [verdicts.get(f"{side}:{c['id']}") for c in CASES["endings"]]
        lat = mean([res.get(c["id"], {}).get("seconds") for c in CASES["endings"]])
        scores[f"ending/{name}"] = {"correct": v.count("correct"), "partial": v.count("partial"), "wrong": v.count("wrong"),
                                   "n": len(v), "mean_seconds": lat}
        report.append(f"ending    {name:10s} {v.count('correct')} correct, {v.count('partial')} partial, {v.count('wrong')} wrong of {len(v)}, {lat:.1f}s mean")
    scores["ending/verdicts"] = verdicts

    for name, res in (("on-device", by_id(od.get("diaries", []))), ("claude", by_id(cl["diaries"]))):
        rows = score_diary(CASES["diaries"], res)
        caught = sum(r["caught"] for r in rows); planted = sum(r["planted"] for r in rows)
        false = sum(r["false"] for r in rows); cok = sum(r["corrected_ok"] for r in rows)
        unread = sum(r["unreadable"] for r in rows)
        scores[f"diary/{name}"] = {"caught": caught, "planted": planted, "false_edits": false, "corrected_ok": cok,
                                  "n": len(rows), "unreadable": unread, "mean_seconds": mean([r.get("seconds") for r in rows]), "rows": rows}
        report.append(f"diary     {name:10s} {caught}/{planted} planted slips caught, {false} false edits, corrected text right {cok}/{len(rows)}, {unread} unreadable, {mean([r.get('seconds') for r in rows]):.1f}s mean")

    for name, res in (("on-device", by_id(od.get("lemmas", []))), ("claude", by_id(cl["lemmas"]))):
        rows = score_lemmas(CASES["lemmas"], res)
        ok = sum(r["ok"] for r in rows)
        scores[f"lemma/{name}"] = {"ok": ok, "n": len(rows), "mean_seconds": mean([r.get("seconds") for r in rows]), "rows": rows}
        report.append(f"lemma     {name:10s} {ok}/{len(rows)} exact, {mean([r.get('seconds') for r in rows]):.2f}s per form")

    (HERE / "scores.json").write_text(json.dumps(scores, ensure_ascii=False, indent=1))
    print("\n".join(report))


if __name__ == "__main__":
    {"answer": answer, "score": score}[sys.argv[1]]()
