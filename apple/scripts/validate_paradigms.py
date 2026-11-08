#!/usr/bin/env python3
"""Checks Resources/paradigms.json (built by build_paradigms.py).

1. Reference tables: the generated forms of the words below equal the tables copied by hand from
   Wiktionary (scripts/paradigm_sources.md), form for form, stress for stress.
2. Shape: every word has every cell of its table (or is marked singular-only), every form is
   Cyrillic with one stress mark (none for ё or a monosyllable), every table cell names existing
   components, every use example contains the form it names and every gap's answer is that form.
3. Coverage: every inflected token of course.json and library.json whose lemma (lemmas.json) has a
   paradigm is one of that word's cells — or is listed in `exceptions`. Every lemmas.json
   construction tag on such a token agrees with the cell (книгу: acc-u ⇔ acc-sg).

    python3 apple/scripts/validate_paradigms.py
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1] / "Drussky/Resources"
ACUTE = "́"
CYR = re.compile("[Ѐ-ӿ]")
VOWELS = "аеиоуыэюяё"
errors = []


def err(where, msg):
    errors.append(f"{where}: {msg}")


def key(s):
    s = s.replace(ACUTE, "").lower()
    for ch in ".,?!—–-\"«»:;…()":
        s = s.replace(ch, " ")
    return " ".join(s.split())


def st(text):
    """Apostrophe stress → U+0301 (the authoring convention)."""
    return re.sub(r"([аеиоуыэюяАЕИОУЫЭЮЯ])'", lambda m: m.group(1) + ACUTE, text)


# --- 1. reference tables (Wiktionary, see paradigm_sources.md) ----------------------------------
NOUN_CELLS = ["nom-sg", "gen-sg", "dat-sg", "acc-sg", "instr-sg", "prep-sg", "nom-pl", "gen-pl", "dat-pl", "acc-pl", "instr-pl", "prep-pl"]
REFERENCE_NOUNS = {  # 12 forms, sg then pl, each case in order
    "книга": "кни'га кни'ги кни'ге кни'гу кни'гой кни'ге кни'ги книг кни'гам кни'ги кни'гами кни'гах",
    "кухня": "ку'хня ку'хни ку'хне ку'хню ку'хней ку'хне ку'хни ку'хонь ку'хням ку'хни ку'хнями ку'хнях",
    "станция": "ста'нция ста'нции ста'нции ста'нцию ста'нцией ста'нции ста'нции ста'нций ста'нциям ста'нции ста'нциями ста'нциях",
    "задача": "зада'ча зада'чи зада'че зада'чу зада'чей зада'че зада'чи зада'ч зада'чам зада'чи зада'чами зада'чах",
    "улица": "у'лица у'лицы у'лице у'лицу у'лицей у'лице у'лицы у'лиц у'лицам у'лицы у'лицами у'лицах",
    "ложка": "ло'жка ло'жки ло'жке ло'жку ло'жкой ло'жке ло'жки ло'жек ло'жкам ло'жки ло'жками ло'жках",
    "тарелка": "таре'лка таре'лки таре'лке таре'лку таре'лкой таре'лке таре'лки таре'лок таре'лкам таре'лки таре'лками таре'лках",
    "папа": "па'па па'пы па'пе па'пу па'пой па'пе па'пы пап па'пам пап па'пами па'пах",
    "вода": "вода' воды' воде' во'ду водо'й воде' во'ды вод во'дам во'ды во'дами во'дах",
    "звезда": "звезда' звезды' звезде' звезду' звездо'й звезде' звёзды звёзд звёздам звёзды звёздами звёздах",
    "сестра": "сестра' сестры' сестре' сестру' сестро'й сестре' сёстры сестёр сёстрам сестёр сёстрами сёстрах",
    "стол": "стол стола' столу' стол столо'м столе' столы' столо'в стола'м столы' стола'ми стола'х",
    "кот": "кот кота' коту' кота' кото'м коте' коты' кото'в кота'м кото'в кота'ми кота'х",
    "город": "го'род го'рода го'роду го'род го'родом го'роде города' городо'в города'м города' города'ми города'х",
    "дом": "дом до'ма до'му дом до'мом до'ме дома' домо'в дома'м дома' дома'ми дома'х",
    "учитель": "учи'тель учи'теля учи'телю учи'теля учи'телем учи'теле учителя' учителе'й учителя'м учителе'й учителя'ми учителя'х",
    "отец": "оте'ц отца' отцу' отца' отцо'м отце' отцы' отцо'в отца'м отцо'в отца'ми отца'х",
    "день": "день дня дню день днём дне дни дней дням дни дня'ми днях",
    "чай": "чай ча'я ча'ю чай ча'ем ча'е чаи' чаёв чая'м чаи' чая'ми чая'х",
    "брат": "брат бра'та бра'ту бра'та бра'том бра'те бра'тья бра'тьев бра'тьям бра'тьев бра'тьями бра'тьях",
    "человек": "челове'к челове'ка челове'ку челове'ка челове'ком челове'ке лю'ди люде'й лю'дям люде'й людьми' лю'дях",
    "окно": "окно' окна' окну' окно' окно'м окне' о'кна о'кон о'кнам о'кна о'кнами о'кнах",
    "слово": "сло'во сло'ва сло'ву сло'во сло'вом сло'ве слова' слов слова'м слова' слова'ми слова'х",
    "письмо": "письмо' письма' письму' письмо' письмо'м письме' пи'сьма пи'сем пи'сьмам пи'сьма пи'сьмами пи'сьмах",
    "число": "число' числа' числу' число' число'м числе' чи'сла чи'сел чи'слам чи'сла чи'слами чи'слах",
    "знание": "зна'ние зна'ния зна'нию зна'ние зна'нием зна'нии зна'ния зна'ний зна'ниям зна'ния зна'ниями зна'ниях",
    "ночь": "ночь но'чи но'чи ночь но'чью но'чи но'чи ноче'й ноча'м но'чи ноча'ми ноча'х",
    "соль": "соль со'ли со'ли соль со'лью со'ли со'ли соле'й соля'м со'ли соля'ми соля'х",
    "мысль": "мысль мы'сли мы'сли мысль мы'слью мы'сли мы'сли мы'слей мы'слям мы'сли мы'слями мы'слях",
    "мать": "мать ма'тери ма'тери мать ма'терью ма'тери ма'тери матере'й матеря'м матере'й матеря'ми матеря'х",
}
REFERENCE_SG = {  # singular-only words: 6 forms
    "молоко": "молоко' молока' молоку' молоко' молоко'м молоке'",
    "земля": "Земля' Земли' Земле' Зе'млю Землёй Земле'",
    "россия": "Росси'я Росси'и Росси'и Росси'ю Росси'ей Росси'и",
}
ADJ_CELLS = [f"{c}-{g}" for g in ("m", "f", "n", "pl") for c in ("nom", "gen", "dat", "acc", "instr", "prep")]
REFERENCE_ADJ = {
    "новый": "но'вый но'вого но'вому но'вый/но'вого но'вым но'вом но'вая но'вой но'вой но'вую но'вой но'вой но'вое но'вого но'вому но'вое но'вым но'вом но'вые но'вых но'вым но'вые/но'вых но'выми но'вых",
    "большой": "большо'й большо'го большо'му большо'й/большо'го больши'м большо'м больша'я большо'й большо'й большу'ю большо'й большо'й большо'е большо'го большо'му большо'е больши'м большо'м больши'е больши'х больши'м больши'е/больши'х больши'ми больши'х",
    "русский": "ру'сский ру'сского ру'сскому ру'сский/ру'сского ру'сским ру'сском ру'сская ру'сской ру'сской ру'сскую ру'сской ру'сской ру'сское ру'сского ру'сскому ру'сское ру'сским ру'сском ру'сские ру'сских ру'сским ру'сские/ру'сских ру'сскими ру'сских",
}
VERB_CELLS = ["1sg", "2sg", "3sg", "1pl", "2pl", "3pl", "past-m", "past-f", "past-n", "past-pl", "imp-sg", "imp-pl"]
REFERENCE_VERBS = {
    "читать": "чита'ю чита'ешь чита'ет чита'ем чита'ете чита'ют чита'л чита'ла чита'ло чита'ли чита'й чита'йте",
    "говорить": "говорю' говори'шь говори'т говори'м говори'те говоря'т говори'л говори'ла говори'ло говори'ли говори' говори'те",
    "писать": "пишу' пи'шешь пи'шет пи'шем пи'шете пи'шут писа'л писа'ла писа'ло писа'ли пиши' пиши'те",
    "любить": "люблю' лю'бишь лю'бит лю'бим лю'бите лю'бят люби'л люби'ла люби'ло люби'ли люби' люби'те",
    "идти": "иду' идёшь идёт идём идёте иду'т шёл шла шло шли иди' иди'те",
    "ехать": "е'ду е'дешь е'дет е'дем е'дете е'дут е'хал е'хала е'хало е'хали поезжа'й поезжа'йте",
    "есть": "ем ешь ест еди'м еди'те едя'т ел е'ла е'ло е'ли ешь е'шьте",
    "дать": "дам дашь даст дади'м дади'те даду'т дал дала' дало' да'ли дай да'йте",
    "быть": "бу'ду бу'дешь бу'дет бу'дем бу'дете бу'дут был была' бы'ло бы'ли будь бу'дьте",
    "жить": "живу' живёшь живёт живём живёте живу'т жил жила' жи'ло жи'ли живи' живи'те",
}
REFERENCE_VERBS_NO_IMP = {
    "хотеть": "хочу' хо'чешь хо'чет хоти'м хоти'те хотя'т хоте'л хоте'ла хоте'ло хоте'ли",
    "мочь": "могу' мо'жешь мо'жет мо'жем мо'жете мо'гут мог могла' могло' могли'",
    "видеть": "ви'жу ви'дишь ви'дит ви'дим ви'дите ви'дят ви'дел ви'дела ви'дело ви'дели",
}
PRONOUN_CELLS = ["nom", "gen", "dat", "acc", "instr", "prep"]
REFERENCE_PRONOUNS = {"я": "я меня' мне меня' мной мне", "она": "она' её ей её ей ней", "они": "они' их им их и'ми них"}
CONJ_ENDINGS = {"conj1": ["ю у", "ешь ёшь", "ет ёт", "ем ём", "ете ёте", "ют ут"], "conj2": ["ю у", "ишь", "ит", "им", "ите", "ят ат"]}


def check_reference(words, table, cells, reference):
    for w, ref in reference.items():
        if w not in words:
            err(f"reference {w}", "missing from paradigms.json")
            continue
        forms = words[w]["forms"]
        for cell, want in zip(cells, ref.split()):
            want = st(want).replace("/", " / ")
            got = forms.get(cell)
            if got != want:
                err(f"{table} {w} {cell}", f"generated {got!r}, Wiktionary {want!r}")


def check_shape(data):
    comps = {c["id"] for c in data["components"]}
    lemmas = json.loads((ROOT / "lemmas.json").read_text())
    legacy = {c["id"] for c in lemmas["constructions"]}
    words = data["words"]
    tables = {t["id"]: t for t in data["tables"]}
    for key_, w in words.items():
        for cell, f in w["forms"].items():
            for part in f.split(" / "):
                if not CYR.search(part) or re.search(r"[^Ѐ-ӿ́]", part):
                    err(f"{key_} {cell}", f"not Cyrillic: {part!r}")
                plain = part.replace(ACUTE, "")
                nvow = sum(c in VOWELS for c in plain.lower())
                marks = part.count(ACUTE)
                if "ё" in plain.lower():
                    if marks:
                        err(f"{key_} {cell}", f"{part!r}: ё is never marked")
                elif nvow >= 2 and marks != 1:
                    err(f"{key_} {cell}", f"{part!r}: needs one stress mark")
                elif nvow < 2 and marks:
                    err(f"{key_} {cell}", f"{part!r}: a monosyllable carries no mark")
        if w["pos"] == "noun":
            want = [c for c in NOUN_CELLS if w.get("number") != "sg" or c.endswith("-sg")]
            if w.get("number") == "pl":
                want = [c for c in NOUN_CELLS if c.endswith("-pl")]
            if sorted(w["forms"]) != sorted(want):
                err(key_, f"cells {sorted(w['forms'])}")
            if key(w["forms"].get("nom-sg", w["ru"])) != key_ and w.get("number") != "pl":
                err(key_, "key is not the nominative singular")
        if w["pos"] == "adj" and sorted(w["forms"]) != sorted(ADJ_CELLS):
            err(key_, f"adjective cells {sorted(w['forms'])}")
        if w["pos"] == "verb":
            need = set(VERB_CELLS[:10] + ["inf"])
            have = set(w["forms"])
            if not need <= have or have - need - {"imp-sg", "imp-pl"}:
                err(key_, f"verb cells {sorted(have)}")
            if w["table"] in CONJ_ENDINGS:
                for cell, endings in zip(VERB_CELLS[:6], CONJ_ENDINGS[w["table"]]):
                    f = w["forms"][cell].replace(ACUTE, "")
                    if not any(f.endswith(e) for e in endings.split()):
                        err(f"{key_} {cell}", f"{w['forms'][cell]!r} is not a {w['table']} ending ({endings})")
            for g, e in (("m", "л"), ("f", "ла"), ("n", "ло"), ("pl", "ли")):
                if g == "m" and w.get("note") and not w["forms"]["past-m"].endswith("л"):
                    continue  # мог: a noted irregular
                if not w["forms"][f"past-{g}"].replace(ACUTE, "").endswith(e):
                    err(f"{key_} past-{g}", f"{w['forms'][f'past-{g}']!r} does not end in -{e}")
            if "imp-pl" in w["forms"] and w["forms"]["imp-pl"].replace(ACUTE, "") != w["forms"]["imp-sg"].replace(ACUTE, "") + "те":
                err(f"{key_} imp-pl", "the plural imperative is the singular + те")
        if w["table"] not in tables:
            err(key_, f"unknown table {w['table']}")
    for t in data["tables"]:
        rows, cols = {r["id"] for r in t["rows"]}, {c["id"] for c in t["cols"]}
        for cell in t["cells"]:
            if cell["row"] not in rows or cell["col"] not in cols:
                err(f"{t['id']} {cell['id']}", "row/col not in the table")
            for ref in cell["components"]:
                kind, _, k = ref.partition(":")
                if kind == "gram" and k not in comps and k not in legacy:
                    err(f"{t['id']} {cell['id']}", f"unknown component {ref}")
                if kind not in ("gram", "form", "lex"):
                    err(f"{t['id']} {cell['id']}", f"component kind {kind!r}")
            for wk in cell["words"]:
                if wk not in words:
                    err(f"{t['id']} {cell['id']}", f"unknown word {wk}")
        for wk in t["words"]:
            if wk not in words:
                err(t["id"], f"unknown word {wk}")
    for c in data["components"]:
        for r in c.get("requires", []):
            if r not in comps and r not in legacy:
                err(f"component {c['id']}", f"requires unknown {r}")
    for u in data["uses"]:
        kind, _, k = u["component"].partition(":")
        if k not in comps and k not in legacy:
            err(f"use {u['id']}", f"unknown component {u['component']}")
        for e in u["examples"]:
            if "word" in e:
                form = words.get(e["word"], {}).get("forms", {}).get(e["cell"])
                if form is None:
                    err(f"use {u['id']}", f"{e['word']} has no cell {e['cell']}")
                elif key(form) not in key(e["ru"]).split():
                    err(f"use {u['id']}", f"{form!r} is not in {e['ru']!r}")
        for g in u["gaps"]:
            form = words.get(g["word"], {}).get("forms", {}).get(g["cell"])
            if form is None:
                err(f"use {u['id']} gap", f"{g['word']} has no cell {g['cell']}")
            elif key(form) != key(g["answer"]):
                err(f"use {u['id']} gap", f"answer {g['answer']!r} is not {g['word']} {g['cell']} = {form!r}")
            if CYR.search(g["hint"]):
                err(f"use {u['id']} gap", "hint must be English")
    # the forms index agrees with the words
    for form, refs in data["forms"].items():
        for wk, cell in refs:
            f = words[wk]["forms"].get(cell)
            if f is None or key(form) not in [key(p) for p in f.split(" / ")]:
                err(f"forms {form}", f"{wk} {cell} is {f!r}")


# --- 3. coverage of the course and the library ---------------------------------------------------
TAG_CELLS = {"acc-u": ("acc-sg",), "prep-e": ("prep-sg",), "plural": ("nom-pl", "acc-pl", "gen-pl", "dat-pl", "instr-pl", "prep-pl"),
             "genitive": ("gen-sg", "gen-pl", "gen-m", "gen-f", "gen-n", "dat-f", "instr-f", "prep-f", "acc-m"),
             "time-instr": ("instr-sg",), "inf": ("inf",), "past": ("past-m", "past-f", "past-n", "past-pl"),
             "pres-1sg": ("1sg",), "pres-2sg": ("2sg",), "pres-3sg": ("3sg",), "pres-1pl": ("1pl",), "pres-2pl": ("2pl",), "pres-3pl": ("3pl",),
             "gender": ("nom-m", "nom-f", "nom-n")}


def course_tokens(course):
    out = []
    for w in course["lexicon"]:
        out.append(("lexicon", w["ru"]))
    for u in course["units"]:
        for p in u["pages"]:
            for e in p["exercises"]:
                for f in ("text", "answer", "before", "after", "say", "word", "prompt"):
                    v = e.get(f)
                    if isinstance(v, str) and f != "prompt":
                        out.append((p["id"], v))
                for o in e.get("options", []) if e.get("kind") in ("listen_pick", "sound_pair") else []:
                    out.append((p["id"], o))
                for x in e.get("examples", []):
                    out.append((p["id"], x["ru"]))
                for g in e.get("glosses", {}):
                    out.append((p["id"], g))
                for f in ("forms",):
                    for v in (e.get(f) or {}).values():
                        out.append((p["id"], v))
                for it in e.get("items", []) or []:
                    out.append((p["id"], it["text"]))
    return out


def check_coverage(data):
    words, forms, exceptions = data["words"], data["forms"], data["exceptions"]
    lemmas = json.loads((ROOT / "lemmas.json").read_text())["forms"]
    course = json.loads((ROOT / "course.json").read_text())
    texts = [(w, s) for w, s in course_tokens(course)]
    lib = ROOT / "library.json"
    if lib.exists():
        for t in json.loads(lib.read_text()).get("texts", []):
            texts.append((f"library {t['id']}", t["text"]))
    seen = set()
    for where, s in texts:
        for tok in key(s).split():
            if tok in seen or not CYR.search(tok):
                continue
            seen.add(tok)
            entry = lemmas.get(tok)
            lemma = entry[0] if entry else tok
            tags = entry[1:] if entry else []
            if lemma not in words:
                continue
            if tok in exceptions or lemma in exceptions:
                continue
            cells = [c for wk, c in forms.get(tok, []) if wk == lemma]
            if not cells and tok != lemma:
                err(f"{where} {tok!r}", f"not a form of {lemma} (cells: {sorted(words[lemma]['forms'].items())[:3]}…) and not an exception")
            for tag in tags:
                if words[lemma]["pos"] in ("pron", "poss") and tag == "gender":
                    continue  # она́, оно́: gender is the word itself
                want = TAG_CELLS.get(tag)
                if want and cells and not set(cells) & set(want):
                    err(f"{where} {tok!r}", f"lemmas.json says {tag}, paradigms say {cells}")


def main():
    data = json.loads((ROOT / "paradigms.json").read_text())
    words = data["words"]
    check_reference(words, "noun", NOUN_CELLS, REFERENCE_NOUNS)
    check_reference(words, "noun", NOUN_CELLS[:6], REFERENCE_SG)
    check_reference(words, "adj", ADJ_CELLS, REFERENCE_ADJ)
    check_reference(words, "verb", VERB_CELLS, REFERENCE_VERBS)
    check_reference(words, "verb", VERB_CELLS[:10], REFERENCE_VERBS_NO_IMP)
    check_reference(words, "pronoun", PRONOUN_CELLS, REFERENCE_PRONOUNS)
    for w in REFERENCE_VERBS_NO_IMP:
        if "imp-sg" in words.get(w, {}).get("forms", {}):
            err(w, "has an imperative it should not")
    check_shape(data)
    check_coverage(data)
    n_forms = sum(len(w["forms"]) for w in words.values())
    print(f"{len(data['tables'])} tables, {len(data['components'])} components, {len(words)} words, {n_forms} forms, {len(data['uses'])} uses, {len(data['exceptions'])} exceptions")
    if errors:
        print(f"{len(errors)} problem(s):")
        for e in errors:
            print("  " + e)
        sys.exit(1)
    print("OK")


if __name__ == "__main__":
    main()
