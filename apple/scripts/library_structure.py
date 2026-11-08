#!/usr/bin/env -S uv run --quiet --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["anthropic>=0.80"]
# ///
"""The structure layer of library texts: for every sentence, its chunks (clause and phrase spans with a
member-of-sentence role), referents (what each pronoun, participle and adjective points at), desugarings
(participial and gerund constructions as flat Russian with который / когда / так как), the root x-ray of
every content word, and the case frames it uses -- written by Claude (claude-opus-5-5, structured JSON),
checked here, and stored for the app's «Структу́ра» toggle.

  uv run apple/scripts/library_structure.py                      # every text of library.json, resumable
  uv run apple/scripts/library_structure.py --ids tolstoy-lgun   # some texts
  uv run apple/scripts/library_structure.py --library apple/PrivatePack/library.json --out apple/PrivatePack/library-structure.json
  python3 apple/scripts/library_structure.py --plan              # what is done, what is left (no SDK, no key)

Input: any file in the Library schema (COURSE_SCHEMA.md, "Library"): its `texts` are annotated; the Tatoeba
`sentences` are not (--sentences adds them as the pseudo-text "sentences"). Output (default
Drussky/Resources/library-structure.json), keyed by text id then sentence index, keeps the exact source
sentence beside its structure, so editing a text invalidates its entry visibly (validate_structure.py).

The key is ANTHROPIC_API_KEY in the environment (never printed, never written anywhere). Requests go one
at a time, one paragraph each (long paragraphs in groups of at most MAX_SENTENCES sentences, with the two
sentences before them shown as context so a pronoun can point back), and the file is rewritten after every
reply, so a stopped run resumes where it was: a sentence whose text and index are already in the file is
skipped. Output ordering is deterministic (library order, sorted keys).

Roots: morphology.json decides the root, its gloss and the taught relatives whenever it knows the word
(lemma or listed form); the model only segments the word as written and fills the gaps morphology has
(`"source": "morphology"` or `"model"` on each entry). Relatives are restricted to words the course
teaches (course.json lexicon, morphology.json lemmas and cognates), so the app can show the ones already met.
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
RES = APPLE / "Drussky/Resources"
LIBRARY = RES / "library.json"
OUT = RES / "library-structure.json"
MODEL = "claude-opus-5-5"
MAX_SENTENCES = 8
ACUTE = "́"
VOWELS = set("аеёиоуыэюя")

# Member-of-sentence and clause roles. The Russian label is how a teacher marks the exercise book
# (подлежа́щее one line, сказу́емое two, дополне́ние dashed, определе́ние wavy, обстоя́тельство dash-dot).
ROLES = {
    "subject": {"ru": "подлежа́щее", "en": "subject", "tier": "member"},
    "predicate": {"ru": "сказу́емое", "en": "predicate", "tier": "member"},
    "object": {"ru": "дополне́ние", "en": "object", "tier": "member"},
    "attribute": {"ru": "определе́ние", "en": "attribute (modifier)", "tier": "member"},
    "adverbial": {"ru": "обстоя́тельство", "en": "adverbial", "tier": "member"},
    "prepositional_phrase": {"ru": "предло́жная гру́ппа", "en": "prepositional phrase", "tier": "member"},
    "address": {"ru": "обраще́ние", "en": "address (vocative)", "tier": "member"},
    "apposition": {"ru": "приложе́ние", "en": "apposition", "tier": "member"},
    "interjection": {"ru": "междоме́тие", "en": "interjection", "tier": "member"},
    "conjunction": {"ru": "сою́з", "en": "conjunction", "tier": "member"},
    "particle": {"ru": "части́ца", "en": "particle", "tier": "member"},
    "introductory": {"ru": "вво́дное сло́во", "en": "introductory word", "tier": "member"},
    "subordinate_clause": {"ru": "прида́точное предложе́ние", "en": "subordinate clause", "tier": "clause"},
    "relative_clause": {"ru": "определи́тельное прида́точное", "en": "relative clause", "tier": "clause"},
    "participial_clause": {"ru": "прича́стный оборо́т", "en": "participial clause", "tier": "clause"},
    "gerund_clause": {"ru": "дееприча́стный оборо́т", "en": "gerund (verbal adverb) clause", "tier": "clause"},
    "coordinate_clause": {"ru": "часть сло́жного предложе́ния", "en": "coordinate clause", "tier": "clause"},
    "direct_speech": {"ru": "пряма́я речь", "en": "direct speech", "tier": "clause"},
    "parenthetical": {"ru": "вста́вная констру́кция", "en": "parenthetical", "tier": "clause"},
}
REFERENT_KINDS = ["pronoun", "participle", "adjective", "relative", "possessive", "numeral"]
DESUGAR_KINDS = ["participle", "gerund", "other"]
CASES = ["nom", "gen", "dat", "acc", "ins", "loc"]

# A word for span purposes: letters (Cyrillic or Latin), digits and stress marks, hyphenated compounds
# as one word. Punctuation, dashes between words and spaces are never inside a word. The app's
# StructureText.words applies the same rule (Structure.swift).
WORD = re.compile(r"(?:[^\W_]|́)+(?:-(?:[^\W_]|́)+)*", re.U)

SYSTEM = f"""You annotate Russian sentences for an adult English-speaking learner in the first weeks of Russian who \
wants to read real texts (fiction, mathematics, philosophy) by seeing their skeleton long before knowing the words. \
Your output is drawn onto the page as a teacher's pencil marks, so it must be exact, sparing and never wrong.

You receive numbered sentences, each with its words numbered from 0. Refer to words only by those numbers; spans \
are inclusive [from, to] word ranges inside one sentence. Annotate only the sentences marked ANNOTATE; sentences \
marked CONTEXT are there so a pronoun can point back to them.

For each annotated sentence give:
1. chunks -- the sentence's members and embedded clauses. Member tier (roles: {", ".join(k for k, v in ROLES.items() if v["tier"] == "member")}): \
cover every word of the sentence except punctuation, with no overlaps inside the tier; a predicate includes its \
auxiliaries and negation (не мог сделать); a prepositional phrase is the preposition with its whole noun phrase; \
an attribute is a modifier that is not already inside a larger member you marked; conjunctions and particles are \
their own chunks. Clause tier (roles: {", ".join(k for k, v in ROLES.items() if v["tier"] == "clause")}): one span per \
embedded clause or turn of direct speech; a clause may contain members and other clauses, but never partially \
overlap another clause. Do not mark the main clause of a simple sentence. Give the role of a member inside a \
subordinate clause as its role within that clause.
2. referents -- for every pronoun, participle, adjective, relative word (который, где), possessive and numeral that \
agrees with or refers to a noun (or noun phrase head) present in this sentence or an earlier one, the head word's \
sentence number and word number. Omit I/you/we (я, ты, мы, вы) and anything with no head in the text. Use the \
head noun itself, not its article or modifier.
3. desugar -- every participial, gerund (деепричастие) or other dense construction (a long attributive phrase, an \
impersonal dative construction, a nominalisation) rewritten as flat Russian in one or two short kernel sentences or \
a который / когда / так как / после того как clause, keeping the original words where possible. Russian first, \
then a plain English rendering. Stress every Russian word of two or more syllables with U+0301 after the stressed \
vowel; never mark ё; never mark one-syllable words. If a sentence has no such construction, give an empty list.
4. roots -- for every content word (nouns, verbs, adjectives, adverbs, participles, gerunds; not names, numerals, \
pronouns, prepositions, conjunctions, particles): the word as written split into prefix, root, suffix, inflection \
(each may be empty; their concatenation must spell the word exactly as written, lowercased, without stress marks). \
Treat a verb's -ть/-ти/-л/-ла/-ли as inflection, thematic vowels and derivational pieces as suffix, a compound's \
second root with its linking vowel as suffix. Give the root's bare meaning in one or two English words and up to \
three common everyday Russian words built on the same root (dictionary forms, with stress marks). When a word \
comes with a HINT root, use exactly that root form and gloss.
5. governance -- each verb, preposition, adjective or noun in the sentence that requires a case of its dependents: \
the head word number, its dependents' word numbers, the frame as a Russian learner's note (зави́сеть от + Gen, \
принадлежа́ть + Dat, по + Dat, у + Gen), and the case. Prepositions always count; verbs only when the case is not \
a plain nominative subject or accusative direct object, or when the verb is reflexive or takes an infinitive.

Be exact: a referent to the wrong head, a chunk that splits a prepositional phrase, or a paraphrase with wrong \
stress is worse than nothing. Write nothing but the JSON."""


def strip_stress(s):
    """Without U+0301, recomposed (й and ё stay single characters)."""
    return unicodedata.normalize("NFC", "".join(c for c in unicodedata.normalize("NFD", s) if c != ACUTE))


def key(s):
    """RussianText.key: lowercase, no stress, punctuation to spaces, one space."""
    s = strip_stress(s).lower()
    s = re.sub(r'[.,?!—–\-"«»:;…()]', " ", s)
    return " ".join(s.split())


def split_sentences(paragraph):
    """GoalAnalysis.splitSentences, so the app finds the same sentences in a passage."""
    out, current = [], ""
    chars = list(paragraph)
    terminators, closers = set(".!?…"), set("»\"”’)'")
    i = 0
    while i < len(chars):
        c = chars[i]
        current += c
        if c in terminators:
            j = i + 1
            while j < len(chars) and (chars[j] in terminators or chars[j] in closers):
                current += chars[j]
                j += 1
            at_end = j >= len(chars)
            next_space = not at_end and chars[j].isspace()
            last = current.split()[-1] if current.split() else ""
            word = "".join(ch for ch in last if ch not in terminators and ch not in closers)
            if (at_end or next_space) and len(word) != 1:
                s = current.strip(" \t")
                if s:
                    out.append(s)
                current = ""
            i = j
            continue
        i += 1
    rest = current.strip(" \t")
    if rest:
        out.append(rest)
    return out


def sentences_of(text):
    """(paragraph index, sentence text) for every sentence of a library text, in order."""
    out = []
    for p, para in enumerate(text.split("\n")):
        for s in split_sentences(para):
            out.append((p, s))
    return out


def words_of(sentence):
    """[start, end) character spans of the words of a sentence (the app's StructureText.words)."""
    return [[m.start(), m.end()] for m in WORD.finditer(sentence)]


def check_stress(s):
    """Problems with the stress marking of a Russian string (validate_course's rule)."""
    problems = []
    for m in re.finditer("[Ѐ-ӿ́]+(?:-[Ѐ-ӿ́]+)*", s):
        tok = m.group(0).lower()
        for i, c in enumerate(tok):
            if c == ACUTE and (i == 0 or tok[i - 1] not in VOWELS):
                problems.append(f"acute not after a vowel in {tok!r}")
        plain = tok.replace(ACUTE, "")
        vowels = sum(c in VOWELS for c in plain)
        marks = tok.count(ACUTE)
        if "ё" in plain:
            if marks:
                problems.append(f"ё word must not carry an acute: {tok!r}")
        elif vowels >= 2 and marks != 1:
            problems.append(f"{vowels}-syllable word needs exactly one stress mark: {tok!r}")
        elif vowels <= 1 and marks:
            problems.append(f"one-syllable word must not carry an acute: {tok!r}")
    return problems


# MARK: Morphology

class Taught:
    """What the course already teaches: morphology roots and families, lexicon words."""

    def __init__(self):
        m = json.loads((RES / "morphology.json").read_text())
        course = json.loads((RES / "course.json").read_text())
        lemmas = json.loads((RES / "lemmas.json").read_text())
        self.roots = {r["id"]: r for r in m["roots"]}
        self.form_to_root = {}
        for r in m["roots"]:
            for f in r["forms"]:
                self.form_to_root.setdefault(f, r["id"])
        self.lemmas = m["lemmas"]
        self.forms = dict(m["forms"])
        for form, info in lemmas["forms"].items():
            self.forms.setdefault(form, info[0])
        self.lexicon = {key(w["ru"]): w["ru"] for w in course["lexicon"]}
        self.members = {}
        for k, e in self.lemmas.items():
            fam = e.get("family")
            if fam:
                self.members.setdefault(fam, []).append(e["ru"])
        for fam in self.members:
            self.members[fam].sort(key=lambda w: (len(strip_stress(w)), w))
        self.known_words = set(self.lexicon) | set(self.lemmas) | {key(c["ru"]) for c in m["cognates"]}

    def entry(self, word):
        k = key(word)
        if k in self.lemmas:
            return self.lemmas[k]
        head = self.forms.get(k)
        return self.lemmas.get(head) if head else None

    def hint(self, word):
        """(root form, root id, gloss) when morphology knows the word's root."""
        e = self.entry(word)
        if not e or not e.get("roots"):
            return None
        rid = e["roots"][0]
        root = self.roots.get(rid)
        if not root:
            return None
        k = key(word)
        form = next((f for f in sorted(root["forms"], key=len, reverse=True) if f in k), root["forms"][0])
        return form, rid, root["gloss"]

    def relatives(self, rid, word, limit=3):
        """Taught words of the family, the word itself and its lemma left out."""
        k = key(word)
        e = self.entry(word)
        own = {k, key(e["ru"]) if e else ""}
        out = []
        for w in self.members.get(rid, []):
            if key(w) in own:
                continue
            if key(w) in self.lexicon or key(w) in self.lemmas:
                out.append(w)
            if len(out) >= limit:
                break
        return out


# MARK: Prompt and reply

def schema():
    span = {"type": "array", "items": {"type": "integer"}}
    obj = lambda props, req=None: {"type": "object", "properties": props, "required": req or list(props), "additionalProperties": False}
    return obj({
        "sentences": {"type": "array", "items": obj({
            "index": {"type": "integer"},
            "chunks": {"type": "array", "items": obj({"span": span, "role": {"type": "string", "enum": list(ROLES)}})},
            "referents": {"type": "array", "items": obj({
                "word": {"type": "integer"}, "head_sentence": {"type": "integer"}, "head": {"type": "integer"},
                "kind": {"type": "string", "enum": REFERENT_KINDS}})},
            "desugar": {"type": "array", "items": obj({
                "span": span, "kind": {"type": "string", "enum": DESUGAR_KINDS}, "ru": {"type": "string"}, "en": {"type": "string"}})},
            "roots": {"type": "array", "items": obj({
                "word": {"type": "integer"}, "prefix": {"type": "string"}, "root": {"type": "string"}, "suffix": {"type": "string"},
                "inflection": {"type": "string"}, "gloss": {"type": "string"}, "relatives": {"type": "array", "items": {"type": "string"}}})},
            "governance": {"type": "array", "items": obj({
                "head": {"type": "integer"}, "dependents": {"type": "array", "items": {"type": "integer"}},
                "frame": {"type": "string"}, "case": {"type": "string", "enum": CASES}})},
        })},
    })


def render(sentences, annotate, taught):
    """The user message: numbered sentences with numbered words and morphology hints."""
    lines = []
    for i, s in sentences:
        tag = "ANNOTATE" if i in annotate else "CONTEXT"
        toks = []
        hints = []
        for w, (a, b) in enumerate(words_of(s)):
            toks.append(f"[{w}]{s[a:b]}")
            if tag == "ANNOTATE":
                h = taught.hint(s[a:b])
                if h:
                    hints.append(f"[{w}] root {h[0]} = {h[2]}")
        lines.append(f"S{i} {tag}: " + " ".join(toks))
        if hints:
            lines.append("  HINT " + "; ".join(hints))
    return "\n".join(lines)


def ask(client, model, prompt, effort):
    import anthropic
    for attempt in range(6):
        try:
            with client.messages.stream(
                model=model, max_tokens=48_000, system=SYSTEM,
                messages=[{"role": "user", "content": prompt}],
                output_config={"effort": effort, "format": {"type": "json_schema", "schema": schema()}},
            ) as stream:
                msg = stream.get_final_message()
            if msg.stop_reason == "refusal":
                raise RuntimeError(f"refused: {getattr(msg, 'stop_details', None)}")
            if msg.stop_reason == "max_tokens":
                raise RuntimeError("reply cut off at max_tokens")
            text = next(b.text for b in msg.content if b.type == "text")
            # NFC keeps й and ё single characters; Cyrillic vowels have no precomposed acute forms, so stress marks stay.
            return json.loads(unicodedata.normalize("NFC", text)), msg.usage
        except (anthropic.RateLimitError, anthropic.APIConnectionError, anthropic.InternalServerError) as e:
            wait = min(120, 10 * 2 ** attempt)
            print(f"  {type(e).__name__}; waiting {wait}s", file=sys.stderr)
            time.sleep(wait)
    raise RuntimeError("gave up after repeated API errors")


def absorb(reply, batch, text_id, taught, problems):
    """The reply's sentences as stored entries, checked against the source; bad items dropped and reported."""
    by_index = {i: s for i, s in batch}
    out = {}
    for r in reply.get("sentences", []):
        i = r.get("index")
        if i not in by_index:
            problems.append(f"{text_id} S{i}: not an annotated sentence")
            continue
        s = by_index[i]
        words = words_of(s)
        n = len(words)

        def ok_span(sp):
            return isinstance(sp, list) and len(sp) == 2 and 0 <= sp[0] <= sp[1] < n

        def where(tag):
            return f"{text_id} S{i} {tag}"

        chunks = []
        for c in r.get("chunks", []):
            if not ok_span(c["span"]):
                problems.append(where(f"chunk span {c['span']} outside 0..{n - 1}"))
                continue
            chunks.append({"words": c["span"], "span": [words[c["span"][0]][0], words[c["span"][1]][1]], "role": c["role"]})
        chunks.sort(key=lambda c: (c["words"][0], -c["words"][1], c["role"]))

        referents = []
        for ref in r.get("referents", []):
            hs = ref.get("head_sentence", i)
            if not (0 <= ref["word"] < n):
                problems.append(where(f"referent word {ref['word']} outside the sentence"))
                continue
            referents.append({"word": ref["word"], "head": ref["head"], "headSentence": hs, "kind": ref["kind"]})
        referents.sort(key=lambda x: x["word"])

        desugar = []
        for d in r.get("desugar", []):
            if not ok_span(d["span"]):
                problems.append(where(f"desugar span {d['span']} outside the sentence"))
                continue
            bad = check_stress(d["ru"])
            if bad:
                problems.append(where("paraphrase stress: " + "; ".join(bad)))
                continue
            desugar.append({"words": d["span"], "span": [words[d["span"][0]][0], words[d["span"][1]][1]],
                            "kind": d["kind"], "ru": d["ru"].strip(), "en": d["en"].strip()})
        desugar.sort(key=lambda x: (x["words"][0], x["words"][1]))

        roots = []
        seen = set()
        for x in r.get("roots", []):
            w = x["word"]
            if not (0 <= w < n) or w in seen:
                continue
            written = key(s[words[w][0]:words[w][1]]).replace(" ", "")
            pieces = [strip_stress(x[k]).lower() for k in ("prefix", "root", "suffix", "inflection")]
            if "".join(pieces) != written:
                problems.append(where(f"x-ray {'+'.join(pieces)} does not spell {written}"))
                continue
            if not pieces[1]:
                problems.append(where(f"x-ray of {written} has no root"))
                continue
            entry = {"word": w, "prefix": pieces[0], "root": pieces[1], "suffix": pieces[2], "inflection": pieces[3]}
            hint = taught.hint(written)
            rid = hint[1] if hint else taught.form_to_root.get(pieces[1])
            if rid:
                entry["rootID"] = rid
                entry["gloss"] = taught.roots[rid]["gloss"]
                entry["relatives"] = taught.relatives(rid, written)
                entry["source"] = "morphology"
            else:
                entry["gloss"] = x["gloss"].strip()
                rel = []
                for cand in x.get("relatives", []):
                    kc = key(cand)
                    if kc and kc != written and kc in taught.known_words and not check_stress(cand) and cand not in rel:
                        rel.append(cand)
                entry["relatives"] = rel[:3]
                entry["source"] = "model"
            roots.append(entry)
            seen.add(w)
        roots.sort(key=lambda x: x["word"])

        governance = []
        for g in r.get("governance", []):
            deps = sorted({d for d in g.get("dependents", []) if 0 <= d < n})
            if not (0 <= g["head"] < n):
                problems.append(where(f"governance head {g['head']} outside the sentence"))
                continue
            bad = check_stress(g["frame"])
            if bad:
                problems.append(where("frame stress: " + "; ".join(bad)))
                continue
            governance.append({"head": g["head"], "dependents": deps, "frame": g["frame"].strip(), "case": g["case"]})
        governance.sort(key=lambda x: (x["head"], x["dependents"]))

        out[i] = {"index": i, "text": s, "words": words, "chunks": chunks, "referents": referents,
                  "desugar": desugar, "roots": roots, "governance": governance}
    for i, _ in batch:
        if i not in out:
            problems.append(f"{text_id} S{i}: missing from the reply")
    return out


# MARK: Driving

def load_out(path):
    if path.exists():
        return json.loads(path.read_text())
    return {"version": 1, "model": MODEL, "roles": ROLES, "texts": {}}


def save_out(path, data):
    data["roles"] = ROLES
    tmp = path.with_suffix(".tmp.json")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=1, sort_keys=True) + "\n")
    tmp.replace(path)


def library_texts(lib, include_sentences):
    texts = list(lib.get("texts", []))
    if include_sentences and lib.get("sentences"):
        texts.append({"id": "sentences", "text": "\n".join(s["ru"] for s in lib["sentences"])})
    return texts


def batches(sentences):
    """Groups of sentence indices to annotate per request: at most MAX_SENTENCES, broken at paragraph
    ends where possible (short dialogue paragraphs share a request; a long paragraph is split)."""
    paragraphs = []
    for i, (p, _) in enumerate(sentences):
        if paragraphs and paragraphs[-1][0] == p:
            paragraphs[-1][1].append(i)
        else:
            paragraphs.append((p, [i]))
    out, cur = [], []
    for _, idx in paragraphs:
        if cur and len(cur) + len(idx) > MAX_SENTENCES:
            out.append(cur)
            cur = []
        for k in range(0, len(idx), MAX_SENTENCES):
            piece = idx[k:k + MAX_SENTENCES]
            if len(cur) + len(piece) > MAX_SENTENCES:
                out.append(cur)
                cur = []
            cur += piece
    if cur:
        out.append(cur)
    return out


def done_indices(entry, sentences):
    """Sentence indices already annotated against the current text."""
    have = {s["index"]: s for s in entry.get("sentences", [])}
    return {i for i, (_, s) in enumerate(sentences) if i in have and have[i]["text"] == s}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--library", type=Path, default=LIBRARY)
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--ids", nargs="*", help="only these text ids")
    ap.add_argument("--sentences", action="store_true", help="also the Tatoeba sentences, as the text 'sentences'")
    ap.add_argument("--model", default=MODEL)
    ap.add_argument("--effort", default="high", choices=["low", "medium", "high", "xhigh", "max"])
    ap.add_argument("--plan", action="store_true", help="report what is done and what is left; no requests")
    ap.add_argument("--pause", type=float, default=1.0, help="seconds between requests")
    args = ap.parse_args()

    lib = json.loads(args.library.read_text())
    texts = library_texts(lib, args.sentences)
    if args.ids:
        texts = [t for t in texts if t["id"] in set(args.ids)]
    data = load_out(args.out)
    taught = Taught()

    todo = []
    total = done = 0
    for t in texts:
        sentences = sentences_of(t["text"])
        entry = data["texts"].get(t["id"], {"sentences": []})
        have = done_indices(entry, sentences)
        total += len(sentences)
        done += len(have)
        for b in batches(sentences):
            left = [i for i in b if i not in have]
            if left:
                todo.append((t, sentences, b))
    print(f"{len(texts)} texts, {total} sentences, {done} annotated, {len(todo)} requests to go", file=sys.stderr)
    if args.plan or not todo:
        return

    if not os.environ.get("ANTHROPIC_API_KEY"):
        raise SystemExit("ANTHROPIC_API_KEY is not in the environment")
    import anthropic
    client = anthropic.Anthropic(max_retries=3)
    data["model"] = args.model
    problems = []
    usage_in = usage_out = 0
    for n, (t, sentences, b) in enumerate(todo, 1):
        context = list(range(max(0, b[0] - 2), b[0]))
        shown = [(i, sentences[i][1]) for i in context + b]
        prompt = render(shown, set(b), taught)
        t0 = time.time()
        reply, usage = ask(client, args.model, prompt, args.effort)
        usage_in += usage.input_tokens
        usage_out += usage.output_tokens
        got = absorb(reply, [(i, sentences[i][1]) for i in b], t["id"], taught, problems)
        entry = data["texts"].setdefault(t["id"], {"sentences": []})
        kept = {s["index"]: s for s in entry["sentences"] if s["index"] < len(sentences) and s["text"] == sentences[s["index"]][1]}
        kept.update(got)
        entry["sentences"] = [kept[i] for i in sorted(kept)]
        save_out(args.out, data)
        print(f"[{n}/{len(todo)}] {t['id']} S{b[0]}..S{b[-1]}: {len(got)}/{len(b)} sentences in {time.time() - t0:.0f}s "
              f"(tokens in {usage.input_tokens}, out {usage.output_tokens})", file=sys.stderr)
        time.sleep(args.pause)
    print(f"done: {usage_in} input tokens, {usage_out} output tokens", file=sys.stderr)
    if problems:
        print(f"{len(problems)} items dropped:", file=sys.stderr)
        for p in problems:
            print("  " + p, file=sys.stderr)


if __name__ == "__main__":
    main()
