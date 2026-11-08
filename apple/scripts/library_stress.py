#!/usr/bin/env -S uv run --quiet --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["anthropic>=0.80"]
# ///
"""Stress marks for the Russian that came from outside the course: every sentence of every library text
(library.json), the Tatoeba sentences, and any sentence of the authored shelves (math.json, series.json,
kitchen.json) that lacks a mark -- written by Claude (claude-opus-5-5, structured JSON), checked here
mechanically, and stored for the app to show through its stress rendering (Model/LibraryStress.swift).

  uv run apple/scripts/library_stress.py                        # everything, resumable
  uv run apple/scripts/library_stress.py --ids tolstoy-lgun     # some texts (or "sentences" for Tatoeba)
  uv run apple/scripts/library_stress.py --library apple/PrivatePack/library.json --out apple/PrivatePack/library-stress.json --no-shelves
  python3 apple/scripts/library_stress.py --plan                # what is done, what is left (no SDK, no key)

Output (default Drussky/Resources/library-stress.json), keyed by text id then sentence index, keeps the exact
source sentence beside its stressed form, so editing a text invalidates its entry visibly (validate_stress.py).
The Tatoeba sentences are the pseudo-text "sentences" (one entry per sentence, in file order); shelf texts and
episodes are keyed by their own ids and appear only where a sentence lacked stress.

What is verified before anything is stored (the model is never trusted on mechanics):
  - removing U+0301 gives the source sentence back exactly;
  - every Cyrillic word of two or more syllables carries exactly one mark, after a vowel; one-syllable words
    and words with ё carry none; in a hyphenated compound each part follows that rule, except that a
    compound of one-syllable parts may carry one mark in all (кто́-то);
  - a sentence that fails is asked once more with the problems named, then dropped (the app shows it bare).
Homographs (за́мок/замо́к, бо́льшая/больша́я, ру́ки/руки́ ...) are the one place the model's judgement
decides; it lists them and they are kept under "homographs" for review. Every stressed word is also compared
with the course (course.json, morphology.json, paradigms.json): where those know the form with one stress
and the model chose another, the sentence is kept as the model wrote it and the word is listed under
"disagreements" (a grammatical homograph the course has only one of, or a mistake: review decides).

The key is ANTHROPIC_API_KEY in the environment (never printed, never written anywhere). Requests go one at
a time, MAX_SENTENCES sentences each with two earlier sentences as context, and the file is rewritten after
every reply, so a stopped run resumes where it was. A sentence with no word needing a mark is filled in
locally. Output ordering is deterministic (sorted keys, sentences by index).
"""
import argparse
import json
import os
import re
import sys
import time
import unicodedata
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(APPLE / "scripts"))
from library_structure import ACUTE, VOWELS, WORD, sentences_of, strip_stress  # noqa: E402

RES = APPLE / "Drussky/Resources"
LIBRARY = RES / "library.json"
OUT = RES / "library-stress.json"
MODEL = "claude-opus-5-5"
MAX_SENTENCES = 14
SHELVES = ["math.json", "series.json", "kitchen.json"]
# Files whose stressed Russian is the course's own word: the cross-check reads every string in them.
COURSE_FILES = ["course.json", "morphology.json", "paradigms.json"]
CYRILLIC = re.compile("[Ѐ-ӿ]")

SYSTEM = """You mark word stress in Russian sentences so that an adult beginner can read real texts aloud. \
You receive numbered sentences; return each sentence marked ANNOTATE exactly as given, with U+0301 (COMBINING \
ACUTE ACCENT) inserted immediately after the stressed vowel of every word of two or more syllables, and nothing \
else changed: not a letter, not the spelling (keep pre-reform or dialect spellings as they are), not punctuation, \
spacing, case or line breaks. Sentences marked CONTEXT are only there so you can read the ANNOTATE ones in context.

Rules: one mark per word of two or more syllables; no mark on a one-syllable word; never mark a word that \
contains ё (ё is always stressed); hyphenated compounds whose parts are all one syllable take one mark in all \
(кто́-то, что́-то); otherwise each part of a hyphenated word follows the rules (по-ру́сски, нау́чно-фантасти́ческий). \
Prepositions and particles that take the stress of a phrase (на́ берег, не́ был) are marked where a careful reader \
stresses them; otherwise a one-syllable preposition stays unmarked. Proper names and archaic forms get the \
stress a native reader uses for that text; in verse follow the metre. Latin words, numbers and symbols are left \
alone.

Homographs: when the written word could carry its stress in more than one place with a different meaning or \
grammatical form (за́мок/замо́к, бо́льшая/больша́я, сто́ит/стои́т, у́же/уже́, ру́ки/руки́, го́рода/города́, \
се́ла/села́, сто́роны/стороны́, мо́ря/моря́, пи́сьма/письма́ ...), choose from the context and list it under \
homographs with the alternative(s) and one short reason. Write nothing but the JSON."""


# MARK: Checking

def cyrillic_words(sentence):
    """(start, end, token) for every token of the sentence that has a Cyrillic letter (the app's word rule)."""
    return [(m.start(), m.end(), m.group(0)) for m in WORD.finditer(sentence) if CYRILLIC.search(m.group(0))]


def syllables(bare):
    return sum(c in VOWELS for c in bare.lower())


def needs_mark(bare):
    """A written word that must carry a stress mark: two or more vowels and no ё (hyphen parts separately)."""
    return any(syllables(p) >= 2 and "ё" not in p.lower() for p in bare.split("-"))


def lacks_stress(sentence):
    """A sentence with a Cyrillic word that needs a mark and has none (an authored shelf's sentence is fine)."""
    for _, _, tok in cyrillic_words(sentence):
        if ACUTE not in tok and needs_mark(tok):
            return True
    return False


def word_problems(tok):
    """Problems with the stress marking of one written word (NFC, may be hyphenated)."""
    out = []
    low = tok.lower()
    for i, c in enumerate(low):
        if c == ACUTE and (i == 0 or low[i - 1] not in VOWELS):
            out.append(f"mark not after a vowel in {tok!r}")
    parts = low.split("-")
    if len(parts) > 1 and all(syllables(p.replace(ACUTE, "")) <= 1 for p in parts):
        if low.count(ACUTE) > 1:
            out.append(f"{tok!r}: a compound of one-syllable parts takes at most one mark")
        return out
    for p in parts:
        plain = p.replace(ACUTE, "")
        marks = p.count(ACUTE)
        if "ё" in plain:
            if marks:
                out.append(f"{tok!r}: a word with ё takes no mark")
        elif syllables(plain) >= 2:
            if marks != 1:
                out.append(f"{tok!r}: {syllables(plain)} syllables need exactly one mark, found {marks}")
        elif marks:
            out.append(f"{tok!r}: a one-syllable word takes no mark")
    return out


def problems(source, stressed):
    """Everything wrong with `stressed` as the stress marking of `source`; empty when it is sound."""
    stressed = unicodedata.normalize("NFC", stressed)
    if strip_stress(stressed) != unicodedata.normalize("NFC", source):
        return ["the text differs from the source once marks are removed"]
    out = []
    for m in WORD.finditer(stressed):
        tok = m.group(0)
        if not CYRILLIC.search(tok):
            if ACUTE in tok:
                out.append(f"{tok!r}: mark on a non-Cyrillic token")
            continue
        out += word_problems(tok)
    return out


# MARK: The course's stressed forms

def course_forms(files=COURSE_FILES):
    """bare lowercase word -> set of stressed spellings the course uses (only words that need a mark)."""
    forms = {}

    def walk(x):
        if isinstance(x, str):
            if ACUTE in x:
                for _, _, tok in cyrillic_words(unicodedata.normalize("NFC", x)):
                    if ACUTE in tok and not word_problems(tok):
                        bare = tok.replace(ACUTE, "").lower()
                        if needs_mark(bare):
                            forms.setdefault(bare, set()).add(tok.lower())
        elif isinstance(x, dict):
            for v in x.values():
                walk(v)
        elif isinstance(x, list):
            for v in x:
                walk(v)

    for name in files:
        p = RES / name
        if p.exists():
            walk(json.loads(p.read_text()))
    return forms


def disagreements(stressed, forms):
    """(word as marked, the course's spellings) for each word the course marks differently, with one voice."""
    out = []
    for _, _, tok in cyrillic_words(stressed):
        low = tok.lower()
        bare = low.replace(ACUTE, "")
        known = forms.get(bare)
        if known and len(known) == 1 and low not in known:
            out.append((tok, sorted(known)))
    return out


# MARK: Sources

def sources(lib, include_sentences=True, shelves=SHELVES):
    """(id, [sentence, ...]) for everything to stress: library texts, the Tatoeba sentences as one pseudo-text,
    and each shelf text or episode reduced to the sentences that lack a mark (usually none)."""
    out = []
    for t in lib.get("texts", []):
        out.append((t["id"], [s for _, s in sentences_of(t["text"])], "library"))
    if include_sentences and lib.get("sentences"):
        out.append(("sentences", [s["ru"] for s in lib["sentences"]], "tatoeba"))
    for name in shelves:
        p = RES / name
        if not p.exists():
            continue
        data = json.loads(p.read_text())
        items = list(data.get("texts", [])) + [ep for s in data.get("series", []) for ep in s.get("episodes", [])]
        for it in items:
            sents = [s for _, s in sentences_of(it["text"])]
            out.append((it["id"], sents, name))
    return out


def shelf_gaps(sentences):
    """Indices of the sentences of an authored (already stressed) text that still lack a mark."""
    return [i for i, s in enumerate(sentences) if lacks_stress(s)]


# MARK: Prompt and reply

def schema():
    obj = lambda props, req=None: {"type": "object", "properties": props, "required": req or list(props), "additionalProperties": False}
    return obj({
        "sentences": {"type": "array", "items": obj({"index": {"type": "integer"}, "stressed": {"type": "string"}})},
        "homographs": {"type": "array", "items": obj({
            "index": {"type": "integer"}, "word": {"type": "string"},
            "alternatives": {"type": "array", "items": {"type": "string"}}, "reason": {"type": "string"}})},
    })


def render(shown, annotate, retry=None):
    lines = []
    for i, s in shown:
        lines.append(f"S{i} {'ANNOTATE' if i in annotate else 'CONTEXT'}: {s}")
    if retry:
        lines.append("")
        lines.append("Your previous answer had these problems; fix them and return every ANNOTATE sentence again:")
        for i, ps in sorted(retry.items()):
            lines.append(f"S{i}: " + "; ".join(ps))
    return "\n".join(lines)


def ask(client, model, prompt, effort):
    import anthropic
    for attempt in range(6):
        try:
            with client.messages.stream(
                model=model, max_tokens=16_000, system=SYSTEM,
                messages=[{"role": "user", "content": prompt}],
                output_config={"effort": effort, "format": {"type": "json_schema", "schema": schema()}},
            ) as stream:
                msg = stream.get_final_message()
            if msg.stop_reason == "refusal":
                raise RuntimeError(f"refused: {getattr(msg, 'stop_details', None)}")
            if msg.stop_reason == "max_tokens":
                raise RuntimeError("reply cut off at max_tokens")
            text = next(b.text for b in msg.content if b.type == "text")
            # NFC keeps й and ё single characters; Cyrillic vowels have no precomposed acute forms, so marks stay.
            return json.loads(unicodedata.normalize("NFC", text)), msg.usage
        except (anthropic.RateLimitError, anthropic.APIConnectionError, anthropic.InternalServerError) as e:
            wait = min(120, 10 * 2 ** attempt)
            print(f"  {type(e).__name__}; waiting {wait}s", file=sys.stderr)
            time.sleep(wait)
    raise RuntimeError("gave up after repeated API errors")


def absorb(reply, batch, forms):
    """The reply checked against the source: (good {index: entry}, failed {index: [problem]}, homographs, disagreements)."""
    by_index = dict(batch)
    good, failed, homs, dis = {}, {}, [], []
    for r in reply.get("sentences", []):
        i = r.get("index")
        if i not in by_index or i in good:
            continue
        src = by_index[i]
        stressed = unicodedata.normalize("NFC", r.get("stressed", ""))
        ps = problems(src, stressed)
        if ps:
            failed[i] = ps
            continue
        good[i] = {"index": i, "text": src, "stressed": stressed}
    listed = {}
    for h in reply.get("homographs", []):
        i = h.get("index")
        if i in good:
            w = unicodedata.normalize("NFC", h.get("word", "")).strip()
            listed.setdefault(i, set()).add(w.lower().replace(ACUTE, ""))
            homs.append({"index": i, "word": w, "alternatives": [unicodedata.normalize("NFC", a) for a in h.get("alternatives", [])],
                         "reason": h.get("reason", "").strip()})
    for i, e in good.items():
        for tok, known in disagreements(e["stressed"], forms):
            if tok.lower().replace(ACUTE, "") in listed.get(i, set()):
                continue  # a homograph the model named: its call
            dis.append({"index": i, "word": tok, "course": known})
    for i, _ in batch:
        if i not in good and i not in failed:
            failed[i] = ["missing from the reply"]
    return good, failed, homs, dis


# MARK: Driving

def load_out(path):
    if path.exists():
        return json.loads(path.read_text())
    return {"version": 1, "model": MODEL, "texts": {}, "homographs": [], "disagreements": []}


def save_out(path, data):
    order = {tid: n for n, tid in enumerate(data["texts"])}
    for section in ("homographs", "disagreements"):
        seen = set()
        items = []
        for h in data.get(section, []):
            k = json.dumps(h, ensure_ascii=False, sort_keys=True)
            if k not in seen:
                seen.add(k)
                items.append(h)
        data[section] = sorted(items, key=lambda h: (order.get(h["text"], 1 << 30), h["index"], h["word"]))
    tmp = path.with_suffix(".tmp.json")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=1, sort_keys=True) + "\n")
    tmp.replace(path)


def done_indices(entry, sentences):
    have = {s["index"]: s for s in entry.get("sentences", [])}
    return {i for i, s in enumerate(sentences) if i in have and have[i]["text"] == s}


def store(data, tid, sentences, got):
    """Entries for sentences still in the text, plus the new ones, in index order; stale ones and
    their review items dropped."""
    entry = data["texts"].setdefault(tid, {"sentences": []})
    kept = {s["index"]: s for s in entry["sentences"] if s["index"] < len(sentences) and s["text"] == sentences[s["index"]]}
    kept.update(got)
    entry["sentences"] = [kept[i] for i in sorted(kept)]
    for section in ("homographs", "disagreements"):
        data[section] = [h for h in data.get(section, []) if h["text"] != tid or h["index"] in kept]


def batches(indices, size=MAX_SENTENCES):
    indices = sorted(indices)
    return [indices[k:k + size] for k in range(0, len(indices), size)]


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--library", type=Path, default=LIBRARY)
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--ids", nargs="*", help="only these text ids ('sentences' for Tatoeba)")
    ap.add_argument("--no-sentences", action="store_true", help="leave the Tatoeba sentences out")
    ap.add_argument("--no-shelves", action="store_true", help="do not scan math.json, series.json and kitchen.json")
    ap.add_argument("--model", default=MODEL)
    ap.add_argument("--effort", default="high", choices=["low", "medium", "high", "xhigh", "max"])
    ap.add_argument("--plan", action="store_true", help="report what is done and what is left; no requests")
    ap.add_argument("--pause", type=float, default=0.5, help="seconds between requests")
    args = ap.parse_args()

    lib = json.loads(args.library.read_text())
    srcs = sources(lib, not args.no_sentences, [] if args.no_shelves else SHELVES)
    if args.ids:
        srcs = [s for s in srcs if s[0] in set(args.ids)]
    data = load_out(args.out)
    forms = course_forms()

    todo = []
    total = done = trivial = 0
    shelf_total = shelf_lacking = 0
    for tid, sentences, origin in srcs:
        entry = data["texts"].get(tid, {"sentences": []})
        if origin not in ("library", "tatoeba"):
            gaps = shelf_gaps(sentences)
            shelf_total += len(sentences)
            shelf_lacking += len(gaps)
            if not gaps:
                continue
            wanted = set(gaps)
        else:
            wanted = set(range(len(sentences)))
        have = done_indices(entry, sentences)
        total += len(wanted)
        done += len(have & wanted)
        left = sorted(wanted - have)
        # Sentences with nothing to mark are their own stressed form.
        easy = {i: {"index": i, "text": sentences[i], "stressed": sentences[i]} for i in left if not lacks_stress(sentences[i])}
        if easy:
            store(data, tid, sentences, easy)
            trivial += len(easy)
            done += len(easy)
        left = [i for i in left if i not in easy]
        for b in batches(left):
            todo.append((tid, sentences, b))
    if trivial:
        save_out(args.out, data)
    print(f"{len(srcs)} sources ({len(forms)} course forms for the cross-check); {total} sentences, {done} stressed "
          f"({trivial} needed no mark), {len(todo)} requests to go; shelves: {shelf_lacking}/{shelf_total} sentences lack stress",
          file=sys.stderr)
    if args.plan or not todo:
        return

    if not os.environ.get("ANTHROPIC_API_KEY"):
        raise SystemExit("ANTHROPIC_API_KEY is not in the environment")
    import anthropic
    client = anthropic.Anthropic(max_retries=3)
    data["model"] = args.model
    dropped = []
    usage_in = usage_out = 0
    for n, (tid, sentences, b) in enumerate(todo, 1):
        context = list(range(max(0, b[0] - 2), b[0]))
        shown = [(i, sentences[i]) for i in context + b]
        batch = [(i, sentences[i]) for i in b]
        t0 = time.time()
        reply, usage = ask(client, args.model, render(shown, set(b)), args.effort)
        usage_in += usage.input_tokens
        usage_out += usage.output_tokens
        good, failed, homs, dis = absorb(reply, batch, forms)
        if failed:
            retry_batch = [(i, sentences[i]) for i in sorted(failed)]
            reply2, usage2 = ask(client, args.model, render(shown, set(failed), retry=failed), args.effort)
            usage_in += usage2.input_tokens
            usage_out += usage2.output_tokens
            good2, failed, homs2, dis2 = absorb(reply2, retry_batch, forms)
            good.update(good2)
            homs += homs2
            dis += dis2
        for i, ps in sorted(failed.items()):
            dropped.append(f"{tid} S{i}: " + "; ".join(ps))
        store(data, tid, sentences, good)
        data["homographs"] = data.get("homographs", []) + [{"text": tid, **h} for h in homs]
        data["disagreements"] = data.get("disagreements", []) + [{"text": tid, **d} for d in dis]
        save_out(args.out, data)
        print(f"[{n}/{len(todo)}] {tid} S{b[0]}..S{b[-1]}: {len(good)}/{len(b)} sentences, {len(homs)} homographs, "
              f"{len(dis)} disagreements in {time.time() - t0:.0f}s (tokens in {usage.input_tokens}, out {usage.output_tokens})",
              file=sys.stderr)
        time.sleep(args.pause)
    print(f"done: {usage_in} input tokens, {usage_out} output tokens", file=sys.stderr)
    if dropped:
        print(f"{len(dropped)} sentences dropped (the app shows them bare):", file=sys.stderr)
        for p in dropped:
            print("  " + p, file=sys.stderr)


if __name__ == "__main__":
    main()
