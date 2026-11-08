#!/usr/bin/env python3
"""Author source for Drussky's paradigms.json: Russian inflection as a visible system.

Nouns (three declensions, six cases, two numbers, the spelling rules and the animate
accusative), adjectives, personal and possessive pronouns, the two conjugations with the
irregular verbs the course uses, the past tense, imperatives, and the case *uses* the course
teaches. Every table cell names the component (`gram:*`) it exercises and the course words that
show it; every word carries its full set of forms.

Conventions (as build_course.py): an apostrophe after a Cyrillic vowel marks stress and becomes
U+0301 on output; ё is never marked; a monosyllable carries no mark. A word is authored as a
lemma plus a *stress pattern* (Zaliznyak's letters) and an *animacy*; the generator applies the
ending tables and spelling rules below, and `validate_paradigms.py` checks the result against
hand-copied Wiktionary tables (scripts/paradigm_sources.md). Irregular cells are written out
(`over=`) and the exceptions a learner must simply know are listed in EXCEPTIONS.

    python3 apple/scripts/build_paradigms.py && python3 apple/scripts/validate_paradigms.py
"""
import json
import re
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / "Drussky/Resources/paradigms.json"
ACUTE = re.compile(r"([аеиоуыэюяАЕИОУЫЭЮЯ])'")
VOWELS = "аеиоуыэюяё"
HUSHERS = "жшчщ"
VELARS = "кгх"


def s(text):
    """Apostrophe stress → U+0301."""
    return ACUTE.sub(lambda m: m.group(1) + "́", text)


def split_stress(w):
    """('кни'га') → ('книга', 2): the plain word and the index of its stressed vowel (ё counts;
    a monosyllable's only vowel counts)."""
    plain, idx = "", None
    for ch in w:
        if ch == "'":
            idx = len(plain) - 1
        else:
            plain += ch
    if idx is None:
        vowels = [i for i, c in enumerate(plain) if c in VOWELS]
        if "ё" in plain:
            idx = plain.index("ё")
        elif len(vowels) == 1:
            idx = vowels[0]
    return plain, idx


def mark(plain, idx):
    """The plain word with the course's stress mark at vowel index `idx` (none for ё or a
    monosyllable)."""
    if idx is None:
        return plain
    nvow = sum(c in VOWELS for c in plain)
    if plain[idx] == "ё" or nvow <= 1:
        return plain
    return plain[: idx + 1] + "'" + plain[idx + 1:]


def last_vowel(stem):
    return max(i for i, c in enumerate(stem) if c in VOWELS)


# --- cases and tables ----------------------------------------------------------------------
CASES = [("nom", "nominative", "who? what? — the subject"), ("gen", "genitive", "of; after нет, у, для, numbers"),
         ("dat", "dative", "to whom — the receiver"), ("acc", "accusative", "whom? what? — the object; в/на + where to"),
         ("instr", "instrumental", "with what, by what; с + with"), ("prep", "prepositional", "в/на + where; о + about")]
NUMBERS = [("sg", "singular"), ("pl", "plural")]
PERSONS = [("1sg", "я"), ("2sg", "ты"), ("3sg", "он, она́, оно́"), ("1pl", "мы"), ("2pl", "вы"), ("3pl", "они́")]
GENDERS = [("m", "masculine"), ("f", "feminine"), ("n", "neuter"), ("pl", "plural")]

# Legacy construction ids (lemmas.json) a cell also exemplifies, so the old patterns keep their evidence.
LEGACY = {"acc-sg@decl1": ["acc-u"], "prep-sg": ["prep-e"], "nom-pl": ["plural"], "gen-sg": ["genitive"], "gen-pl": ["genitive"]}


# --- nouns ---------------------------------------------------------------------------------
# Ending tables, unstressed: sg nom gen dat acc instr prep | pl nom gen dat acc instr prep.
# "=" means the accusative follows animacy (nominative for things, genitive for people and
# animals); "~" is the zero ending (with the inserted vowel of the genitive plural).
NOUN_ENDINGS = {
    "1h": (["а", "ы", "е", "у", "ой", "е"], ["ы", "~", "ам", "=", "ами", "ах"]),   # кни́га
    "1s": (["я", "и", "е", "ю", "ей", "е"], ["и", "~ь", "ям", "=", "ями", "ях"]),  # ку́хня, неде́ля
    "1i": (["ия", "ии", "ии", "ию", "ией", "ии"], ["ии", "ий", "иям", "=", "иями", "иях"]),  # ста́нция
    "2h": (["", "а", "у", "=", "ом", "е"], ["ы", "ов", "ам", "=", "ами", "ах"]),   # стол
    "2s": (["ь", "я", "ю", "=", "ем", "е"], ["и", "ей", "ям", "=", "ями", "ях"]),  # учи́тель
    "2j": (["й", "я", "ю", "=", "ем", "е"], ["и", "ев", "ям", "=", "ями", "ях"]),  # чай
    "2n": (["о", "а", "у", "о", "ом", "е"], ["а", "~", "ам", "=", "ами", "ах"]),   # окно́
    "2e": (["е", "я", "ю", "е", "ем", "е"], ["я", "ей", "ям", "=", "ями", "ях"]),  # мо́ре
    "2i": (["ие", "ия", "ию", "ие", "ием", "ии"], ["ия", "ий", "иям", "=", "иями", "иях"]),  # зна́ние
    "3": (["ь", "и", "и", "ь", "ью", "и"], ["и", "ей", "ям", "=", "ями", "ях"]),   # ночь
}
DECL_OF = {"1h": "decl1", "1s": "decl1", "1i": "decl1", "2h": "decl2", "2s": "decl2", "2j": "decl2", "2n": "decl2", "2e": "decl2", "2i": "decl2", "3": "decl3"}
CELLS = [f"{c}-{n}" for n, _ in NUMBERS for c, _, _ in CASES]


def noun_class(plain, gender):
    if plain.endswith("ия"):
        return "1i", plain[:-2]
    if plain.endswith("а"):
        return "1h", plain[:-1]
    if plain.endswith("я"):
        return "1s", plain[:-1]
    if gender == "f" and plain.endswith("ь"):
        return "3", plain[:-1]
    if gender == "m" and plain.endswith("ь"):
        return "2s", plain[:-1]
    if gender == "m" and plain.endswith("й"):
        return "2j", plain[:-1]
    if gender == "n" and plain.endswith("ие"):
        return "2i", plain[:-2]
    if gender == "n" and plain.endswith("о"):
        return "2n", plain[:-1]
    if gender == "n" and plain.endswith("е"):
        return "2e", plain[:-1]
    return "2h", plain


def insert_vowel(stem, stressed):
    """The genitive-plural zero ending: a vowel is slipped into a final consonant cluster whose
    last member is р л н м к ц (окно → окон, ложка → ложек, сестра → сестёр); о after a velar or
    before к, otherwise е (ё under stress). A ь or й before the cluster gives way (письмо → писем)."""
    if len(stem) < 2 or stem[-1] in VOWELS or stem[-2] in VOWELS:
        return stem
    c1, c2 = stem[-2], stem[-1]
    if c2 not in "рлнмкц":
        return stem
    if c1 in "ьй":
        return stem[:-2] + ("ё" if stressed else "е") + c2
    if c1 in VELARS or (c2 == "к" and c1 not in HUSHERS):
        return stem[:-1] + "о" + c2
    return stem[:-1] + ("ё" if stressed else "е") + c2


def spell(stem, ending, stressed):
    """Attach an ending, applying the spelling rules: ы → и after к г х ж ш ч щ; unstressed о → е
    after ж ш ч щ ц; я → а, ю → у after hushers."""
    if not ending:
        return stem
    last = stem[-1] if stem else ""
    e = ending
    if e[0] == "ы" and last in VELARS + HUSHERS:
        e = "и" + e[1:]
    if e[0] == "о" and last in HUSHERS + "ц" and not stressed:
        e = "е" + e[1:]
    if e[0] in "яю" and last in HUSHERS:
        e = {"я": "а", "ю": "у"}[e[0]] + e[1:]
    if e[0] == "о" and last in "ьй":  # soft stems never take a hard о (not reached by our classes, kept for safety)
        e = "е" + e[1:]
    return stem + e


def ending_stressed(pattern, cell):
    case, num = cell.split("-")
    return {
        "a": False,
        "b": True,
        "c": num == "pl",
        "d": num == "sg",
        "d'": num == "sg" and case != "acc",
        "e": num == "pl" and case != "nom",
    }[pattern]


def decline(spec):
    """Every cell of a noun, stressed, from its spec."""
    plain, idx = split_stress(spec["ru"])
    klass, stem = noun_class(plain, spec["gender"])
    stem_obl = spec.get("stem", stem)       # fleeting vowel: отец → отц-
    stem_pl = spec.get("stem_pl", stem_obl)  # ё in the plural: звезда → звёзд-
    sg_end, pl_end = NOUN_ENDINGS[klass]
    animate = spec.get("animate", False)
    pattern = spec.get("stress", "a")
    forms = {}
    for num, endings in (("sg", sg_end), ("pl", pl_end)):
        for (case, _, _), ending in zip(CASES, endings):
            cell = f"{case}-{num}"
            if ending == "=":
                src = "gen" if (animate and (num == "pl" or spec["gender"] == "m")) else "nom"
                forms[cell] = forms[f"{src}-{num}"]
                continue
            st = stem if cell == "nom-sg" else (stem_obl if num == "sg" else stem_pl)
            stressed = ending_stressed(pattern, cell)
            soft = klass in ("1s", "1i", "2s", "2j", "2e", "2i", "3")
            ending_vowels = []
            if ending.startswith("~"):
                st0 = st
                st = spec.get("stem_genpl", insert_vowel(st, False))
                body = st + ending[1:]
                # a stressed zero ending puts the stress on the stem's last vowel; ё already carries it
                i = None if "ё" in body else (last_vowel(body) if stressed else None)
                st = st0  # stem stress is placed as on the stem before the inserted vowel (пи́сем, о́кон)
            else:
                body = spell(st, ending, stressed)
                ending_vowels = [len(st) + k for k, ch in enumerate(body[len(st):]) if ch in VOWELS]
                # an ending without a vowel (стол, учи́тель, чай) carries the stress on the stem's last vowel
                i = (ending_vowels[0] if ending_vowels else last_vowel(body)) if stressed else None
                # a stressed е of a soft ending is written ё: днём, землёй, чаёв
                if stressed and soft and ((cell == "instr-sg" and ending in ("ем", "ей")) or (cell == "gen-pl" and ending == "ев")):
                    body = body[:-2] + "ё" + body[-1]
            if i is None and "ё" not in body:
                # stem stress: where the lemma had it, else (lemma stressed on the ending) the stem's last vowel
                if idx is not None and idx < len(st) and body[idx] in VOWELS:
                    i = idx
                elif idx is not None and idx == len(st) and idx < len(plain) - 1 and ending_vowels:
                    # the lemma's stressed vowel is the и the -ия/-ие classes count as ending (Росси́я → Росси́и, Росси́ю)
                    i = ending_vowels[0]
                else:
                    i = last_vowel(st)
            forms[cell] = mark(body, None if "ё" in body else i)
    forms.update(spec.get("over", {}))
    if spec.get("number") == "sg":
        forms = {c: f for c, f in forms.items() if c.endswith("-sg")}
    if spec.get("number") == "pl":
        forms = {c: f for c, f in forms.items() if c.endswith("-pl")}
    return DECL_OF[klass], klass, forms


# (lemma, English, gender, options...). Patterns: a stem throughout; b ending throughout; c stem
# in the singular, ending in the plural; d the reverse; d' as d but the accusative singular on the
# stem; e stem except the oblique plural. animate=True gives the animate accusative.
def N(ru, en, gender, stress="a", animate=False, number=None, stem=None, stem_pl=None, stem_genpl=None, over=None, note=None):
    d = dict(ru=ru, en=en, gender=gender, stress=stress, animate=animate)
    for k, v in dict(number=number, stem=stem, stem_pl=stem_pl, stem_genpl=stem_genpl, over=over, note=note).items():
        if v is not None:
            d[k] = v
    return d


NOUNS = [
    # --- declension I: -а / -я (feminine, and a few masculine people) ---
    N("ма'ма", "mom", "f", animate=True), N("кни'га", "book", "f"), N("раке'та", "rocket", "f"), N("плане'та", "planet", "f"),
    N("вода'", "water", "f", stress="d'"), N("звезда'", "star", "f", stress="d", stem_pl="звёзд"),
    N("сестра'", "sister", "f", stress="d", animate=True, stem_pl="сёстр", over={"gen-pl": "сестёр", "acc-pl": "сестёр"}),
    N("Земля'", "the Earth", "f", stress="d'", number="sg", note="земля́ (land) declines the same, lower-case, with a plural: зе́мли, земе́ль"),
    N("Луна'", "the Moon", "f", stress="d", number="sg"), N("ка'ша", "porridge", "f"), N("зада'ча", "problem, task", "f"), N("пра'вда", "truth", "f", number="sg"),
    N("у'лица", "street", "f"), N("ча'шка", "cup", "f"), N("ло'жка", "spoon", "f"), N("таре'лка", "plate", "f"), N("рабо'та", "work", "f"),
    N("ку'хня", "kitchen", "f", stem_genpl="кухон"), N("неде'ля", "week", "f"), N("су'мма", "sum", "f"), N("систе'ма", "system", "f"),
    N("структу'ра", "structure", "f"), N("теоре'ма", "theorem", "f"), N("ло'гика", "logic", "f", number="sg"), N("матема'тика", "mathematics", "f", number="sg"),
    N("ста'нция", "station", "f"), N("тео'рия", "theory", "f"), N("филосо'фия", "philosophy", "f"), N("геоме'трия", "geometry", "f"),
    N("фу'нкция", "function", "f"), N("абстра'кция", "abstraction", "f"), N("информа'ция", "information", "f", number="sg"), N("Росси'я", "Russia", "f", number="sg"),
    N("па'па", "dad", "m", animate=True, note="masculine, but declines like ма́ма"), N("Ми'ра", "Mira (a name)", "f", animate=True, number="sg"),
    # --- declension II: masculine in a consonant / -ь / -й, neuter in -о / -е ---
    N("стол", "table", "m", stress="b"), N("кот", "cat", "m", stress="b", animate=True), N("рот", "mouth", "m", stress="b", stem="рт"),
    N("мир", "world; peace", "m", stress="c"), N("торт", "cake", "m"), N("сок", "juice", "m"), N("нос", "nose", "m", stress="c"),
    N("сыр", "cheese", "m", stress="c"), N("дом", "house, home", "m", stress="c", over={"nom-pl": "дома'", "acc-pl": "дома'"}),
    N("го'род", "city, town", "m", stress="c", over={"nom-pl": "города'", "acc-pl": "города'"}), N("суп", "soup", "m", stress="c"),
    N("брат", "brother", "m", animate=True, over={"nom-pl": "бра'тья", "gen-pl": "бра'тьев", "dat-pl": "бра'тьям", "acc-pl": "бра'тьев", "instr-pl": "бра'тьями", "prep-pl": "бра'тьях"}),
    N("ро'бот", "robot", "m", animate=True), N("жук", "beetle", "m", stress="b", animate=True), N("хлеб", "bread", "m", number="sg"),
    N("цирк", "circus", "m"), N("факт", "fact", "m"), N("объе'кт", "object", "m"), N("лук", "onion", "m", number="sg"), N("люк", "hatch", "m"),
    N("стул", "chair", "m", over={"nom-pl": "сту'лья", "gen-pl": "сту'льев", "dat-pl": "сту'льям", "acc-pl": "сту'лья", "instr-pl": "сту'льями", "prep-pl": "сту'льях"}),
    N("за'втрак", "breakfast", "m"), N("у'жин", "dinner", "m"), N("обе'д", "lunch", "m"), N("са'хар", "sugar", "m", number="sg"), N("ко'смос", "space", "m", number="sg"),
    N("парк", "park", "m"), N("сигна'л", "signal", "m"), N("шум", "noise", "m"), N("отве'т", "answer", "m"), N("вход", "entrance", "m"), N("вы'ход", "exit", "m"),
    N("перехо'д", "crossing, transition", "m"), N("элеме'нт", "element", "m"), N("аргуме'нт", "argument", "m"), N("рабо'тник", "worker", "m", animate=True),
    N("учени'к", "pupil", "m", stress="b", animate=True), N("челове'к", "person", "m", animate=True,
      over={"nom-pl": "лю'ди", "gen-pl": "люде'й", "dat-pl": "лю'дям", "acc-pl": "люде'й", "instr-pl": "людьми'", "prep-pl": "лю'дях"}),
    N("ве'чер", "evening", "m", stress="c", over={"nom-pl": "вечера'", "acc-pl": "вечера'"}), N("день", "day", "m", stress="b", stem="дн"),
    N("оте'ц", "father", "m", stress="b", animate=True, stem="отц"), N("чай", "tea", "m", stress="c"),
    N("учи'тель", "teacher", "m", stress="c", animate=True, over={"nom-pl": "учителя'"}), N("писа'тель", "writer", "m", animate=True), N("чита'тель", "reader", "m", animate=True),
    N("окно'", "window", "n", stress="d"), N("сло'во", "word", "n", stress="c"), N("молоко'", "milk", "n", stress="b", number="sg"),
    N("письмо'", "a letter (mail)", "n", stress="d"), N("число'", "number", "n", stress="d"), N("у'тро", "morning", "n", stem_genpl="утр"),
    N("доказа'тельство", "proof", "n"), N("зна'ние", "knowledge", "n"), N("чте'ние", "reading", "n"),
    # --- declension III: feminine in -ь ---
    N("мысль", "thought", "f"), N("ночь", "night", "f", stress="e"), N("соль", "salt", "f", stress="e"), N("ру'копись", "manuscript", "f"),
    N("му'дрость", "wisdom", "f", number="sg"), N("справедли'вость", "justice", "f", number="sg"),
    N("мать", "mother", "f", animate=True, over={"gen-sg": "ма'тери", "dat-sg": "ма'тери", "instr-sg": "ма'терью", "prep-sg": "ма'тери", "nom-pl": "ма'тери",
                                                  "gen-pl": "матере'й", "dat-pl": "матеря'м", "acc-pl": "матере'й", "instr-pl": "матеря'ми", "prep-pl": "матеря'х"},
      note="the stem grows: ма́тер- outside the nominative and accusative singular"),
]

# --- adjectives ----------------------------------------------------------------------------
ADJ_ENDINGS = {  # per gender: nom gen dat acc instr prep
    "m": ["ый", "ого", "ому", "=", "ым", "ом"],
    "f": ["ая", "ой", "ой", "ую", "ой", "ой"],
    "n": ["ое", "ого", "ому", "ое", "ым", "ом"],
    "pl": ["ые", "ых", "ым", "=", "ыми", "ых"],
}
ADJ_CELLS = [f"{c}-{g}" for g, _ in GENDERS for c, _, _ in CASES]


def A(ru, en, note=None):
    return {k: v for k, v in dict(ru=ru, en=en, note=note).items() if v is not None}


ADJECTIVES = [
    A("но'вый", "new"), A("ста'рый", "old"), A("большо'й", "big"), A("стра'нный", "strange"), A("му'дрый", "wise"), A("справедли'вый", "just"),
    A("просто'й", "simple; prime (of a number)"), A("чётный", "even (of a number)"), A("нечётный", "odd (of a number)"), A("це'лый", "whole"),
    A("ка'ждый", "every"), A("любо'й", "any"), A("до'брый", "good, kind"), A("споко'йный", "calm"), A("прия'тный", "pleasant"),
    A("кото'рый", "which, that", note="a relative pronoun that declines exactly like an adjective"),
    A("ру'сский", "Russian", note="after к the ы of the endings is written и: ру́сский, ру́сские"),
]


def decline_adj(spec):
    plain, idx = split_stress(spec["ru"])
    stressed = plain.endswith("ой")
    stem = plain[:-2]
    forms = {}
    for g, _ in GENDERS:
        for (case, _, _), ending in zip(CASES, ADJ_ENDINGS[g]):
            cell = f"{case}-{g}"
            if ending == "=":
                continue
            e = ending
            if e == "ый" and stressed:
                e = "ой"
            body = spell(stem, e, stressed)
            if stressed:
                i = len(stem) + next(k for k, ch in enumerate(body[len(stem):]) if ch in VOWELS)
            else:
                i = idx
            forms[cell] = mark(body, None if "ё" in body else i)
    for g in ("m", "pl"):
        forms[f"acc-{g}"] = forms[f"nom-{g}"] + " / " + forms[f"gen-{g}"]
    return forms


# --- pronouns ------------------------------------------------------------------------------
PERSONAL = {  # nom gen dat acc instr prep; after a preposition the third person takes н-: у него́, к ней, с ни́ми
    "я": ["я", "меня'", "мне", "меня'", "мной", "мне"], "ты": ["ты", "тебя'", "тебе'", "тебя'", "тобо'й", "тебе'"],
    "он": ["он", "его'", "ему'", "его'", "им", "нём"], "она'": ["она'", "её", "ей", "её", "ей", "ней"],
    "оно'": ["оно'", "его'", "ему'", "его'", "им", "нём"], "мы": ["мы", "нас", "нам", "нас", "на'ми", "нас"],
    "вы": ["вы", "вас", "вам", "вас", "ва'ми", "вас"], "они'": ["они'", "их", "им", "их", "и'ми", "них"],
}
PERSONAL_EN = {"я": "I", "ты": "you (one, familiar)", "он": "he; it", "она'": "she; it", "оно'": "it", "мы": "we", "вы": "you (plural or polite)", "они'": "they"}
POSSESSIVE = {  # gender → nom gen dat acc instr prep
    "мой": {"m": ["мой", "моего'", "моему'", "мой / моего'", "мои'м", "моём"], "f": ["моя'", "мое'й", "мое'й", "мою'", "мое'й", "мое'й"],
            "n": ["моё", "моего'", "моему'", "моё", "мои'м", "моём"], "pl": ["мои'", "мои'х", "мои'м", "мои' / мои'х", "мои'ми", "мои'х"]},
    "твой": {"m": ["твой", "твоего'", "твоему'", "твой / твоего'", "твои'м", "твоём"], "f": ["твоя'", "твое'й", "твое'й", "твою'", "твое'й", "твое'й"],
             "n": ["твоё", "твоего'", "твоему'", "твоё", "твои'м", "твоём"], "pl": ["твои'", "твои'х", "твои'м", "твои' / твои'х", "твои'ми", "твои'х"]},
    "наш": {"m": ["наш", "на'шего", "на'шему", "наш / на'шего", "на'шим", "на'шем"], "f": ["на'ша", "на'шей", "на'шей", "на'шу", "на'шей", "на'шей"],
            "n": ["на'ше", "на'шего", "на'шему", "на'ше", "на'шим", "на'шем"], "pl": ["на'ши", "на'ших", "на'шим", "на'ши / на'ших", "на'шими", "на'ших"]},
    "ваш": {"m": ["ваш", "ва'шего", "ва'шему", "ваш / ва'шего", "ва'шим", "ва'шем"], "f": ["ва'ша", "ва'шей", "ва'шей", "ва'шу", "ва'шей", "ва'шей"],
            "n": ["ва'ше", "ва'шего", "ва'шему", "ва'ше", "ва'шим", "ва'шем"], "pl": ["ва'ши", "ва'ших", "ва'шим", "ва'ши / ва'ших", "ва'шими", "ва'ших"]},
}
POSSESSIVE_EN = {"мой": "my", "твой": "your (one person)", "наш": "our", "ваш": "your (plural or polite)"}


# --- verbs ---------------------------------------------------------------------------------
# inf, English, conjugation (1, 2 or "irr"), the six present forms, past (None = regular from the
# infinitive), imperative (None = not used in the course / not formed). Perfective verbs' "present"
# is their future.
def V(ru, en, conj, pres, past=None, imp=None, note=None, aspect="impf"):
    d = dict(ru=ru, en=en, conj=conj, pres=pres.split(), aspect=aspect)
    if past:
        d["past"] = past.split()
    if imp:
        d["imp"] = imp.split()
    if note:
        d["note"] = note
    return d


VERBS = [
    V("чита'ть", "to read", 1, "чита'ю чита'ешь чита'ет чита'ем чита'ете чита'ют", imp="чита'й чита'йте"),
    V("ду'мать", "to think", 1, "ду'маю ду'маешь ду'мает ду'маем ду'маете ду'мают", imp="ду'май ду'майте"),
    V("знать", "to know", 1, "зна'ю зна'ешь зна'ет зна'ем зна'ете зна'ют", imp="знай зна'йте"),
    V("понима'ть", "to understand", 1, "понима'ю понима'ешь понима'ет понима'ем понима'ете понима'ют", imp="понима'й понима'йте"),
    V("де'лать", "to do", 1, "де'лаю де'лаешь де'лает де'лаем де'лаете де'лают", imp="де'лай де'лайте"),
    V("рабо'тать", "to work", 1, "рабо'таю рабо'таешь рабо'тает рабо'таем рабо'таете рабо'тают", imp="рабо'тай рабо'тайте"),
    V("гуля'ть", "to go for a walk", 1, "гуля'ю гуля'ешь гуля'ет гуля'ем гуля'ете гуля'ют", imp="гуля'й гуля'йте"),
    V("слу'шать", "to listen", 1, "слу'шаю слу'шаешь слу'шает слу'шаем слу'шаете слу'шают", imp="слу'шай слу'шайте"),
    V("счита'ть", "to count; to consider", 1, "счита'ю счита'ешь счита'ет счита'ем счита'ете счита'ют", imp="счита'й счита'йте"),
    V("существова'ть", "to exist", 1, "существу'ю существу'ешь существу'ет существу'ем существу'ете существу'ют", imp="существу'й существу'йте",
      note="-овать verbs trade -ова- for -у- in the present: существу́ю"),
    V("писа'ть", "to write", 1, "пишу' пи'шешь пи'шет пи'шем пи'шете пи'шут", imp="пиши' пиши'те", note="с → ш before the present endings, and the stress steps back after я"),
    V("жить", "to live", 1, "живу' живёшь живёт живём живёте живу'т", past="жил жила' жи'ло жи'ли", imp="живи' живи'те", note="the present stem gains в: жив-"),
    V("ждать", "to wait", 1, "жду ждёшь ждёт ждём ждёте ждут", past="ждал ждала' жда'ло жда'ли", imp="жди жди'те"),
    V("идти'", "to go (on foot)", 1, "иду' идёшь идёт идём идёте иду'т", past="шёл шла шло шли", imp="иди' иди'те", note="the past is from another root: шёл, шла"),
    V("е'хать", "to go (by transport)", 1, "е'ду е'дешь е'дет е'дем е'дете е'дут", imp="поезжа'й поезжа'йте", note="the imperative is borrowed from поезжа́ть"),
    V("прочита'ть", "to read (to the end)", 1, "прочита'ю прочита'ешь прочита'ет прочита'ем прочита'ете прочита'ют", imp="прочита'й прочита'йте", aspect="pf"),
    V("сде'лать", "to do (and finish)", 1, "сде'лаю сде'лаешь сде'лает сде'лаем сде'лаете сде'лают", imp="сде'лай сде'лайте", aspect="pf"),
    V("написа'ть", "to write (and finish)", 1, "напишу' напи'шешь напи'шет напи'шем напи'шете напи'шут", imp="напиши' напиши'те", aspect="pf"),
    V("поня'ть", "to understand, to grasp", 1, "пойму' поймёшь поймёт поймём поймёте пойму'т", past="по'нял поняла' по'няло по'няли", imp="пойми' пойми'те", aspect="pf"),
    V("быть", "to be", 1, "бу'ду бу'дешь бу'дет бу'дем бу'дете бу'дут", past="был была' бы'ло бы'ли", imp="будь бу'дьте", note="no present tense; бу́ду is the future"),
    V("говори'ть", "to speak", 2, "говорю' говори'шь говори'т говори'м говори'те говоря'т", imp="говори' говори'те"),
    V("ви'деть", "to see", 2, "ви'жу ви'дишь ви'дит ви'дим ви'дите ви'дят", note="д → ж in the first person only"),
    V("люби'ть", "to love", 2, "люблю' лю'бишь лю'бит лю'бим лю'бите лю'бят", imp="люби' люби'те", note="б, п, в, м, ф take an л before -ю: люблю́"),
    V("ходи'ть", "to go (regularly, on foot)", 2, "хожу' хо'дишь хо'дит хо'дим хо'дите хо'дят", imp="ходи' ходи'те"),
    V("учи'ть", "to teach; to learn", 2, "учу' у'чишь у'чит у'чим у'чите у'чат", imp="учи' учи'те"),
    V("ве'рить", "to believe", 2, "ве'рю ве'ришь ве'рит ве'рим ве'рите ве'рят", imp="верь ве'рьте"),
    V("слы'шать", "to hear", 2, "слы'шу слы'шишь слы'шит слы'шим слы'шите слы'шат"),
    V("молча'ть", "to be silent", 2, "молчу' молчи'шь молчи'т молчи'м молчи'те молча'т", imp="молчи' молчи'те"),
    V("лете'ть", "to fly", 2, "лечу' лети'шь лети'т лети'м лети'те летя'т", imp="лети' лети'те"),
    V("смотре'ть", "to look", 2, "смотрю' смо'тришь смо'трит смо'трим смо'трите смо'трят", imp="смотри' смотри'те"),
    V("спать", "to sleep", 2, "сплю спишь спит спим спи'те спят", past="спал спала' спа'ло спа'ли", imp="спи спи'те"),
    V("зна'чить", "to mean", 2, "зна'чу зна'чишь зна'чит зна'чим зна'чите зна'чат"),
    V("хоте'ть", "to want", "irr", "хочу' хо'чешь хо'чет хоти'м хоти'те хотя'т", note="first conjugation in the singular (with т → ч), second in the plural"),
    V("есть", "to eat", "irr", "ем ешь ест еди'м еди'те едя'т", past="ел е'ла е'ло е'ли", imp="ешь е'шьте", note="one of four athematic verbs; есть also means 'there is'"),
    V("дать", "to give", "irr", "дам дашь даст дади'м дади'те даду'т", past="дал дала' дало' да'ли", imp="дай да'йте", aspect="pf",
      note="the neuter past is дало́ in careful speech; stem stress, as in да́ли, is also heard"),
    V("переда'ть", "to pass", "irr", "переда'м переда'шь переда'ст передади'м передади'те передаду'т", past="переда'л передала' переда'ло переда'ли",
      imp="переда'й переда'йте", aspect="pf", note="declines as дать"),
    V("мочь", "to be able", "irr", "могу' мо'жешь мо'жет мо'жем мо'жете мо'гут", past="мог могла' могло' могли'", note="г → ж except before у; no imperative"),
]
VERB_CELLS = [p for p, _ in PERSONS] + ["past-m", "past-f", "past-n", "past-pl", "imp-sg", "imp-pl"]


def conjugate(spec):
    plain, idx = split_stress(spec["ru"])
    forms = {"inf": spec["ru"]}
    for (p, _), f in zip(PERSONS, spec["pres"]):
        forms[p] = f
    if "past" in spec:
        past = spec["past"]
    else:
        stem = spec["ru"][:-2] if plain.endswith("ть") else None
        if stem is None:
            raise ValueError(f"{plain}: past tense must be given")
        # the infinitive's stress stays (чита́л); a monosyllabic infinitive (знать) marks its vowel once the word grows
        if idx is None or idx >= len(plain) - 2:
            raise ValueError(f"{plain}: past tense must be given")
        past = [mark(plain[:-2] + e, idx) for e in ("л", "ла", "ло", "ли")]
    for g, f in zip(("m", "f", "n", "pl"), past):
        forms[f"past-{g}"] = f
    if "imp" in spec:
        forms["imp-sg"], forms["imp-pl"] = spec["imp"]
    return forms


# --- case uses -----------------------------------------------------------------------------
# Each use: the case it takes, its component, a short rule, examples (ru/en and the inflected word
# with its cell), and gaps (a sentence with the form missing: before ___ after → answer).
def G(before, answer, after, hint, word, cell):
    return dict(before=before, answer=answer, after=after, hint=hint, word=word, cell=cell)


def E(ru, en, word=None, cell=None):
    d = dict(ru=ru, en=en)
    if word:
        d.update(word=word, cell=cell)
    return d


USES = [
    dict(id="acc-obj", case="acc", title="the object: чита'ю кни'гу", component="acc-obj", legacy=["acc-u"],
         rule="What a verb acts on goes into the accusative. For things it looks like the nominative, except -а → -у and -я → -ю; for people and animals the masculine takes the genitive form.",
         examples=[E("Я чита'ю кни'гу.", "I am reading a book.", "кни'га", "acc-sg"), E("Я ви'жу звезду'.", "I see a star.", "звезда'", "acc-sg"),
                   E("Он лю'бит сестру'.", "He loves his sister.", "сестра'", "acc-sg"), E("Мы ви'дим Зе'млю.", "We see the Earth.", "Земля'", "acc-sg")],
         gaps=[G("Я чита'ю ", "кни'гу", ".", "I am reading a book.", "кни'га", "acc-sg"), G("Я ви'жу ", "звезду'", ".", "I see a star.", "звезда'", "acc-sg"),
               G("Я ви'жу ", "раке'ту", ".", "I see a rocket.", "раке'та", "acc-sg"), G("Мы ви'дим ", "Зе'млю", ".", "We see the Earth.", "Земля'", "acc-sg"),
               G("Он лю'бит ", "сестру'", ".", "He loves his sister.", "сестра'", "acc-sg"), G("Я ви'жу ", "кота'", ".", "I see the cat.", "кот", "acc-sg"),
               G("Я зна'ю ", "отца'", ".", "I know the father.", "оте'ц", "acc-sg"), G("Я чита'ю ", "письмо'", ".", "I am reading a letter.", "письмо'", "acc-sg")]),
    dict(id="acc-dir", case="acc", title="в / на + where to", component="acc-dir",
         rule="Going somewhere: в or на with the accusative. Куда́? — в го'род, на рабо'ту.",
         examples=[E("Я иду' в го'род.", "I am going to the city.", "го'род", "acc-sg"), E("Я е'ду на рабо'ту.", "I am going to work.", "рабо'та", "acc-sg"),
                   E("Раке'та лети'т в ко'смос.", "The rocket is flying into space.", "ко'смос", "acc-sg")],
         gaps=[G("Я иду' в ", "го'род", ".", "I am going to the city.", "го'род", "acc-sg"), G("Я е'ду на ", "рабо'ту", ".", "I am going to work.", "рабо'та", "acc-sg"),
               G("Мы идём в ", "парк", ".", "We are going to the park.", "парк", "acc-sg"), G("Раке'та лети'т на ", "Луну'", ".", "The rocket is flying to the Moon.", "Луна'", "acc-sg")]),
    dict(id="prep-e", case="prep", title="в / на + where", component="prep-e",
         rule="Being somewhere: в or на with the prepositional, which ends in -е (-и after -ия, -ие and for -ь nouns). Где? — в до'ме, на столе'.",
         examples=[E("Кот в до'ме.", "The cat is in the house.", "дом", "prep-sg"), E("Кни'га на столе'.", "The book is on the table.", "стол", "prep-sg"),
                   E("Мы на ку'хне.", "We are in the kitchen.", "ку'хня", "prep-sg"), E("Он на ста'нции.", "He is at the station.", "ста'нция", "prep-sg")],
         gaps=[G("Кни'га на ", "столе'", ".", "The book is on the table.", "стол", "prep-sg"), G("Мы на ", "ку'хне", ".", "We are in the kitchen.", "ку'хня", "prep-sg"),
               G("Я живу' в ", "го'роде", ".", "I live in the city.", "го'род", "prep-sg"), G("Кот в ", "до'ме", ".", "The cat is in the house.", "дом", "prep-sg"),
               G("Он на ", "ста'нции", ".", "He is at the station.", "ста'нция", "prep-sg"),
               G("Мы гуля'ем в ", "па'рке", ".", "We are walking in the park.", "парк", "prep-sg"), G("Она' чита'ет о ", "Луне'", ".", "She is reading about the Moon.", "Луна'", "prep-sg"),
               G("Э'то сло'во в ", "кни'ге", ".", "This word is in the book.", "кни'га", "prep-sg")]),
    dict(id="have", case="gen", title="у меня' есть", component="have",
         rule="Having is said as 'at me there is': у + genitive + есть. The owner stands in the genitive; what is owned stays in the nominative.",
         examples=[E("У меня' есть кни'га.", "I have a book.", "я", "gen"), E("У тебя' есть кот?", "Do you have a cat?", "ты", "gen"),
                   E("У бра'та есть дом.", "My brother has a house.", "брат", "gen-sg")],
         gaps=[G("У ", "меня'", " есть кни'га.", "I have a book.", "я", "gen"), G("У ", "тебя'", " есть кот?", "Do you have a cat?", "ты", "gen"),
               G("У ", "бра'та", " есть дом.", "My brother has a house.", "брат", "gen-sg"), G("У ", "сестры'", " есть кни'га.", "My sister has a book.", "сестра'", "gen-sg"),
               G("У ", "ма'мы", " есть торт.", "Mom has a cake.", "ма'ма", "gen-sg"), G("У ", "нас", " есть вре'мя.", "We have time.", "мы", "gen")]),
    dict(id="gen-no", case="gen", title="нет + чего'?", component="gen-no", legacy=["genitive"],
         rule="What is missing stands in the genitive: нет хле'ба, нет со'ли. So does the whole in 'of': су'мма чи'сел.",
         examples=[E("У меня' нет хле'ба.", "I have no bread.", "хлеб", "gen-sg"), E("Нет со'ли.", "There is no salt.", "соль", "gen-sg"),
                   E("Нет молока'.", "There is no milk.", "молоко'", "gen-sg")],
         gaps=[G("У меня' нет ", "хле'ба", ".", "I have no bread.", "хлеб", "gen-sg"), G("Нет ", "со'ли", ".", "There is no salt.", "соль", "gen-sg"),
               G("Нет ", "молока'", ".", "There is no milk.", "молоко'", "gen-sg"), G("У меня' нет ", "сы'ра", ".", "I have no cheese.", "сыр", "gen-sg"),
               G("Здесь нет ", "окна'", ".", "There is no window here.", "окно'", "gen-sg"), G("У нас нет ", "кни'ги", ".", "We have no book.", "кни'га", "gen-sg"),
               G("Нет ", "ча'я", ".", "There is no tea.", "чай", "gen-sg"), G("Нет ", "воды'", ".", "There is no water.", "вода'", "gen-sg")]),
    dict(id="dat-to", case="dat", title="to whom: мне, тебе'", component="dat-to",
         rule="The receiver stands in the dative: переда'й мне, спаси'бо тебе'. Nouns end in -у/-ю (masculine, neuter) or -е (-и for -ия and -ь).",
         examples=[E("Переда'й мне соль.", "Pass me the salt.", "я", "dat"), E("Спаси'бо, и тебе'!", "Thanks, you too!", "ты", "dat"),
                   E("Я даю' кни'гу бра'ту.", "I am giving the book to my brother.", "брат", "dat-sg")],
         gaps=[G("Переда'й ", "мне", " соль.", "Pass me the salt.", "я", "dat"), G("Спаси'бо, и ", "тебе'", "!", "Thanks, you too!", "ты", "dat"),
               G("Я даю' кни'гу ", "бра'ту", ".", "I am giving the book to my brother.", "брат", "dat-sg"), G("Переда'й ", "ма'ме", " хлеб.", "Pass mom the bread.", "ма'ма", "dat-sg"),
               G("Он пи'шет письмо' ", "сестре'", ".", "He is writing a letter to his sister.", "сестра'", "dat-sg"), G("Переда'й ", "нам", " соль.", "Pass us the salt.", "мы", "dat")]),
    dict(id="instr-with", case="instr", title="с + with", component="instr-with",
         rule="With: с + instrumental. Masculine and neuter take -ом/-ем, feminine -ой/-ей, -ь nouns -ью; the plural is -ами/-ями.",
         examples=[E("Чай с молоко'м.", "Tea with milk.", "молоко'", "instr-sg"), E("Ко'фе с са'харом.", "Coffee with sugar.", "са'хар", "instr-sg"),
                   E("Я иду' с бра'том.", "I am walking with my brother.", "брат", "instr-sg")],
         gaps=[G("Чай с ", "молоко'м", ".", "Tea with milk.", "молоко'", "instr-sg"), G("Ко'фе с ", "са'харом", ".", "Coffee with sugar.", "са'хар", "instr-sg"),
               G("Я иду' с ", "бра'том", ".", "I am walking with my brother.", "брат", "instr-sg"), G("Ка'ша с ", "со'лью", ".", "Porridge with salt.", "соль", "instr-sg"),
               G("Я гуля'ю с ", "сестро'й", ".", "I am walking with my sister.", "сестра'", "instr-sg"), G("Суп с ", "хле'бом", ".", "Soup with bread.", "хлеб", "instr-sg"),
               G("Он говори'т с ", "ма'мой", ".", "He is talking with mom.", "ма'ма", "instr-sg")]),
    dict(id="time-instr", case="instr", title="when: у'тром, ве'чером", component="time-instr",
         rule="Times of day stand alone in the instrumental: у'тром, днём, ве'чером, но'чью.",
         examples=[E("У'тром я рабо'таю.", "In the morning I work.", "у'тро", "instr-sg"), E("Ве'чером мы чита'ем.", "In the evening we read.", "ве'чер", "instr-sg"),
                   E("Но'чью ти'хо.", "At night it is quiet.", "ночь", "instr-sg")],
         gaps=[G("", "У'тром", " я рабо'таю.", "In the morning I work.", "у'тро", "instr-sg"), G("", "Ве'чером", " мы чита'ем.", "In the evening we read.", "ве'чер", "instr-sg"),
               G("", "Но'чью", " ти'хо.", "At night it is quiet.", "ночь", "instr-sg"), G("", "Днём", " я гуля'ю.", "In the daytime I go for a walk.", "день", "instr-sg")]),
    dict(id="genitive", case="gen", title="of: су'мма чи'сел", component="genitive",
         rule="Of, and after numbers and для: су'мма двух чи'сел, для любо'го числа'. Two to four take the genitive singular, five and up the genitive plural.",
         examples=[E("су'мма двух чи'сел", "the sum of two numbers", "число'", "gen-pl"), E("для любо'го числа'", "for any number", "число'", "gen-sg"),
                   E("два до'ма", "two houses", "дом", "gen-sg")],
         gaps=[G("су'мма двух ", "чи'сел", "", "the sum of two numbers", "число'", "gen-pl"), G("для любо'го ", "числа'", "", "for any number", "число'", "gen-sg"),
               G("два ", "до'ма", "", "two houses", "дом", "gen-sg"), G("пять ", "домо'в", "", "five houses", "дом", "gen-pl"),
               G("два ", "стола'", "", "two tables", "стол", "gen-sg"), G("пять ", "столо'в", "", "five tables", "стол", "gen-pl"),
               G("три ", "кни'ги", "", "three books", "кни'га", "gen-sg"), G("пять ", "книг", "", "five books", "кни'га", "gen-pl")]),
]

# Forms the course uses that no table produces by rule; the learner meets them as words.
EXCEPTIONS = {
    "мать": "the stem grows: ма́тери, ма́терью (declension III with -ер-)",
    "кофе": "indeclinable: ко́фе in every case",
    "люди": "the plural of челове́к is from another word: лю́ди, люде́й",
    "дома": "dóма (at home) is an adverb; дома́ (houses) is the plural",
    "домой": "домо́й (homeward) is an old dative kept as an adverb",
    "щи": "plural only: щи, щей",
    "большее": "comparative of большо́й: бо́льший, бо́льшее",
    "делится": "reflexive verbs (-ся) sit outside these tables: де́лится = де́лит + ся",
    "кажется": "reflexive: ка́жется",
    "требовалось": "reflexive past: тре́бовалось",
    "шёл": "the past of идти́ is from another root: шёл, шла, шло, шли",
    "поезжай": "the imperative of е́хать is borrowed from поезжа́ть",
    "двух": "numbers decline too: два → двух; they are a chapter of their own",
    "одни": "оди́н agrees like an adjective: одна́, одно́, одни́",
    "одна": "оди́н agrees like an adjective: одна́, одно́, одни́",
    "одно": "оди́н agrees like an adjective: одна́, одно́, одни́",
    "каждое": "ка́ждый, neuter",
    "этих": "э́тот declines like a pronoun: э́того, э́тих",
    "его": "его́, её, их never change (my book → его́ кни́га, его́ кни́гу)",
    "их": "его́, её, их never change",
    "ней": "after a preposition the third person gains н-: у неё, с ней, о нём",
    "нём": "after a preposition the third person gains н-: у неё, с ней, о нём",
    "них": "after a preposition the third person gains н-: у них",
}


def build():
    components, tables, words, forms_index = [], [], {}, {}

    def comp(id, name, note, requires=None):
        d = dict(id=id, name=name, note=note)
        if requires:
            d["requires"] = requires
        components.append(d)

    def add_form(form, word_key, cell):
        k = form.replace("'", "").lower()
        if "/" in k:
            return
        forms_index.setdefault(k, [])
        if [word_key, cell] not in forms_index[k]:
            forms_index[k].append([word_key, cell])

    # Nouns: three tables.
    noun_meta = {
        "decl1": ("Nouns I · -а / -я", "feminine (and a few people: па́па): кни́га, ку́хня, ста́нция",
                  "Hard stems take -ы, soft stems -и; after к г х ж ш ч щ the ending ы is written и (кни́ги, зада́чи). The genitive plural has no ending — a vowel slips into a final cluster: ло́жек, о́кон."),
        "decl2": ("Nouns II · consonant, -ь, -й / -о, -е", "masculine: стол, учи́тель, чай · neuter: окно́, зна́ние",
                  "Masculine and neuter share their endings except in the nominative and accusative. People and animals (кот, брат) take the genitive form as their accusative."),
        "decl3": ("Nouns III · -ь", "feminine in -ь: ночь, мысль, соль",
                  "Five of six singular forms are the same two words: ночь and но́чи. The instrumental -ью is the one ending of its own."),
    }
    cell_rows = [dict(id=c, label=l, gloss=g) for c, l, g in CASES]
    cell_cols = [dict(id=n, label=l) for n, l in NUMBERS]
    noun_cells = {d: {} for d in noun_meta}
    for spec in NOUNS:
        decl, klass, forms = decline(spec)
        key = split_stress(spec["ru"])[0].lower()
        entry = dict(ru=s(spec["ru"]), en=spec["en"], table=decl, pos="noun", gender=spec["gender"], animate=spec.get("animate", False),
                     forms={c: s(f) for c, f in forms.items()})
        if spec.get("note"):
            entry["note"] = spec["note"]
        if spec.get("number"):
            entry["number"] = spec["number"]
        words[key] = entry
        for c, f in forms.items():
            add_form(f, key, c)
            noun_cells[decl].setdefault(c, []).append(key)
    for decl, (title, subtitle, note) in noun_meta.items():
        cells = []
        for cell in CELLS:
            case, num = cell.split("-")
            comp_id = f"{decl}-{cell}"
            legacy = LEGACY.get(f"{cell}@{decl}", LEGACY.get(cell, []))
            # endings shown in the header: the classes of this declension
            classes = [k for k, d in DECL_OF.items() if d == decl]
            endings = []
            for k in classes:
                e = NOUN_ENDINGS[k][0 if num == "sg" else 1][[c for c, _, _ in CASES].index(case)]
                shown = {"": "-∅", "~": "-∅", "~ь": "-ь", "=": "= nom / gen"}.get(e, "-" + e)
                if shown not in endings:
                    endings.append(shown)
            label = {"nom": "nom", "gen": "gen", "dat": "dat", "acc": "acc", "instr": "instr", "prep": "prep"}[case]
            roman = {"decl1": "I", "decl2": "II", "decl3": "III"}[decl]
            case_name = dict((c, l) for c, l, _ in CASES)[case]
            requires = ["gender"] if cell == "nom-sg" else ([f"{decl}-nom-sg"] if num == "sg" else [f"{decl}-{case}-sg"])
            comp(comp_id, f"{roman} {label} {num}", f"Nouns {roman}, {case_name} {dict(NUMBERS)[num]}: {' / '.join(endings)}", requires)
            cells.append(dict(id=cell, row=case, col=num, ending=" / ".join(endings), components=[f"gram:{comp_id}"] + [f"gram:{x}" for x in legacy],
                              words=noun_cells[decl].get(cell, [])))
        tables.append(dict(id=decl, kind="noun", title=title, subtitle=subtitle, note=note, rows=cell_rows, cols=cell_cols, cells=cells,
                           words=[k for k, w in words.items() if w["table"] == decl]))

    # Adjectives.
    adj_cells = {}
    for spec in ADJECTIVES:
        forms = decline_adj(spec)
        key = split_stress(spec["ru"])[0].lower()
        entry = dict(ru=s(spec["ru"]), en=spec["en"], table="adj", pos="adj", forms={c: s(f) for c, f in forms.items()})
        if spec.get("note"):
            entry["note"] = spec["note"]
        words[key] = entry
        for c, f in forms.items():
            for part in f.split(" / "):
                add_form(part, key, c)
            adj_cells.setdefault(c, []).append(key)
    cells = []
    for g, glabel in GENDERS:
        for (case, case_name, _), ending in zip(CASES, ADJ_ENDINGS[g]):
            cell = f"{case}-{g}"
            shown = {"=": "= nom / gen"}.get(ending, "-" + ending + (" / -о́й" if ending == "ый" else ""))
            legacy = ["gender"] if case == "nom" and g != "pl" else (["plural"] if cell == "nom-pl" else [])
            requires = ["gender"] if case == "nom" else [f"adj-nom-{g}"]
            comp(f"adj-{cell}", f"adj {case} {g}", f"Adjectives, {case_name} {glabel}: {shown}", requires)
            cells.append(dict(id=cell, row=case, col=g, ending=shown, components=[f"gram:adj-{cell}"] + [f"gram:{x}" for x in legacy], words=adj_cells.get(cell, [])))
    tables.append(dict(id="adj", kind="adj", title="Adjectives", subtitle="но́вый, но́вая, но́вое, но́вые · большо́й",
                       note="An adjective repeats its noun's gender, number and case. Stressed endings are -о́й, -о́го; after к г х ж ш ч щ the ы is written и (ру́сский, больши́е); after ж ш ч щ an unstressed о is written е (хоро́шего).",
                       rows=cell_rows, cols=[dict(id=g, label=l) for g, l in GENDERS], cells=cells, words=[k for k, w in words.items() if w["table"] == "adj"]))

    # Personal pronouns: cells are the forms themselves.
    cells = []
    for pron, fs in PERSONAL.items():
        key = split_stress(pron)[0]
        words[key] = dict(ru=s(pron), en=PERSONAL_EN[pron], table="pron", pos="pron", forms={c: s(f) for (c, _, _), f in zip(CASES, fs)})
        for (case, _, _), f in zip(CASES, fs):
            add_form(f, key, case)
            form_key = f.replace("'", "")
            comp_ref = f"lex:{form_key}" if form_key == key else f"form:{form_key}"
            cells.append(dict(id=f"{case}-{key}", row=case, col=key, ending=s(f), components=[comp_ref], words=[key]))
    tables.append(dict(id="pron", kind="pronoun", title="Personal pronouns", subtitle="я, ты, он, она́, оно́, мы, вы, они́",
                       note="Six forms each, learned as words. After a preposition the third person takes н-: у него́, к ней, с ни́ми, о нём.",
                       rows=cell_rows, cols=[dict(id=split_stress(p)[0], label=s(p)) for p in PERSONAL], cells=cells, words=[split_stress(p)[0] for p in PERSONAL]))

    # Possessives: one table per word (мой and наш shown; твой and ваш follow them).
    for pron, table in POSSESSIVE.items():
        key = pron
        forms = {}
        cells = []
        for g, fs in table.items():
            for (case, _, _), f in zip(CASES, fs):
                cell = f"{case}-{g}"
                forms[cell] = s(f)
                for part in f.split(" / "):
                    add_form(part, key, cell)
                first = f.split(" / ")[0].replace("'", "")
                comp_ref = f"lex:{first}" if first == key else f"form:{first}"
                cells.append(dict(id=cell, row=case, col=g, ending=s(f), components=[comp_ref], words=[key]))
        words[key] = dict(ru=s(pron), en=POSSESSIVE_EN[pron], table="poss-" + {"твой": "мой", "ваш": "наш"}.get(key, key), pos="poss", forms=forms)
        if key in ("мой", "наш"):
            twin = {"мой": "твой", "наш": "ваш"}[key]
            tables.append(dict(id=f"poss-{key}", kind="pronoun", title=f"Possessive · {s(pron)}", subtitle=f"{twin} declines the same way",
                               note="Possessives agree like adjectives: мой дом, моя́ кни́га, моё окно́, мои́ кни́ги — and change with the case: в моём до́ме.",
                               rows=cell_rows, cols=[dict(id=g, label=l) for g, l in GENDERS], cells=cells, words=[key, twin]))

    # Verbs: two conjugations, the irregulars, the past, the imperative.
    conj_cells = {1: {}, 2: {}}
    for spec in VERBS:
        forms = conjugate(spec)
        key = split_stress(spec["ru"])[0]
        table = {1: "conj1", 2: "conj2", "irr": "irregular"}[spec["conj"]]
        entry = dict(ru=s(spec["ru"]), en=spec["en"], table=table, pos="verb", aspect=spec["aspect"], forms={c: s(f) for c, f in forms.items()})
        if spec.get("note"):
            entry["note"] = spec["note"]
        words[key] = entry
        for c, f in forms.items():
            add_form(f, key, c)
            if spec["conj"] in (1, 2) and c in dict(PERSONS):
                conj_cells[spec["conj"]].setdefault(c, []).append(key)
    for conj, endings, title, subtitle, note in (
        (1, ["-ю / -у", "-ешь / -ёшь", "-ет / -ёт", "-ем / -ём", "-ете / -ёте", "-ют / -ут"], "Verbs I · -ешь", "чита́ть, знать, жить, идти́",
         "The first conjugation: е in the endings (ё when stressed). Most verbs in -ать and -ять belong here, and the -овать verbs with -у-: существу́ю."),
        (2, ["-ю / -у", "-ишь", "-ит", "-им", "-ите", "-ят / -ат"], "Verbs II · -ишь", "говори́ть, ви́деть, люби́ть",
         "The second conjugation: и in the endings. Most verbs in -ить belong here (and ви́деть, смотре́ть, слы́шать). The first person often changes its last consonant: ви́жу, хожу́, люблю́."),
    ):
        cells = []
        for (p, who), ending in zip(PERSONS, endings):
            comp_id = f"conj{conj}-{p}"
            requires = ["inf"] if p == "1sg" else [f"conj{conj}-1sg"]
            comp(comp_id, f"{'I' if conj == 1 else 'II'} {s(who)}", f"Conjugation {'I' if conj == 1 else 'II'}, {s(who)}: {ending}", requires)
            cells.append(dict(id=p, row=p, col="pres", ending=ending, components=[f"gram:{comp_id}", f"gram:pres-{p}"], words=conj_cells[conj].get(p, [])))
        tables.append(dict(id=f"conj{conj}", kind="verb", title=title, subtitle=subtitle, note=note,
                           rows=[dict(id=p, label=s(who)) for p, who in PERSONS], cols=[dict(id="pres", label="present")], cells=cells,
                           words=[k for k, w in words.items() if w["table"] == f"conj{conj}"]))
    irregular = ["хотеть", "есть", "дать", "мочь", "идти", "ехать"]
    cells = []
    for key in irregular:
        for p, _ in PERSONS:
            f = words[key]["forms"][p]
            cells.append(dict(id=f"{p}-{key}", row=p, col=key, ending=f, components=[f"form:{f.replace(chr(0x301), '')}"], words=[key]))
    tables.append(dict(id="irregular", kind="verb", title="Irregular verbs", subtitle="хоте́ть, есть, дать, мочь, идти́, е́хать",
                       note="хоте́ть follows I in the singular and II in the plural; есть and дать are the old athematic verbs (ем, ешь, ест; дам, дашь, даст); мочь turns г into ж except before у; идти́ and е́хать are regular in the present and odd in the past (шёл) and the imperative (поезжа́й).",
                       rows=[dict(id=p, label=s(who)) for p, who in PERSONS], cols=[dict(id=k, label=words[k]["ru"]) for k in irregular], cells=cells, words=irregular))
    cells = []
    for g, glabel, ending, who in (("m", "masculine", "-л", "он"), ("f", "feminine", "-ла", "она́"), ("n", "neuter", "-ло", "оно́"), ("pl", "plural", "-ли", "они́, мы, вы")):
        comp(f"past-{g}", f"past {who}", f"Past tense, {glabel}: stem + {ending}", ["past"])
        cells.append(dict(id=g, row=g, col="past", ending=ending, components=[f"gram:past-{g}", "gram:past"],
                          words=[k for k, w in words.items() if w["pos"] == "verb"]))
    tables.append(dict(id="past", kind="verb", title="Past tense", subtitle="чита́л, чита́ла, чита́ло, чита́ли",
                       note="One form per gender and one plural, whoever the subject is: я чита́л / чита́ла, мы чита́ли. Drop -ть, add -л. A few verbs move the stress onto -ла: была́, жила́, спала́.",
                       rows=[dict(id=g, label=l) for g, l in (("m", "он"), ("f", "она́"), ("n", "оно́"), ("pl", "они́ · мы · вы"))], cols=[dict(id="past", label="past")],
                       cells=cells, words=[k for k, w in words.items() if w["pos"] == "verb"]))
    cells = []
    for n, label, ending in (("sg", "ты", "-й / -и / -ь"), ("pl", "вы", "-йте / -ите / -ьте")):
        comp(f"imp-{n}", f"imperative {label}", f"Imperative to {label}: {ending}", ["pres-2sg"])
        cells.append(dict(id=n, row=n, col="imp", ending=ending, components=[f"gram:imp-{n}"],
                          words=[k for k, w in words.items() if w["pos"] == "verb" and "imp-sg" in w["forms"]]))
    tables.append(dict(id="imp", kind="verb", title="Imperative", subtitle="чита́й · говори́ · переда́й(те)",
                       note="Take the они́ form, drop its ending: after a vowel add -й (чита́й), after a consonant -и when the я form is stressed on the ending (говори́, пиши́), else -ь (верь). Add -те for вы or politeness.",
                       rows=[dict(id=n, label=l) for n, l in (("sg", "ты"), ("pl", "вы"))], cols=[dict(id="imp", label="imperative")], cells=cells,
                       words=[k for k, w in words.items() if w["pos"] == "verb" and "imp-sg" in w["forms"]]))

    # Case uses.
    uses = []
    for u in USES:
        if u["component"] not in ("acc-dir", "prep-e", "have", "time-instr", "genitive"):
            comp(u["component"], s(u["title"]), u["rule"].split(". ")[0] + ".", [{"acc": "decl1-acc-sg", "gen": "decl1-gen-sg", "dat": "decl1-dat-sg", "instr": "decl2-instr-sg"}[u["case"]]])
        out = dict(id=u["id"], case=u["case"], title=s(u["title"]), component="gram:" + u["component"], legacy=["gram:" + x for x in u.get("legacy", [])],
                   rule=s(u["rule"]), examples=[], gaps=[])
        for e in u["examples"]:
            d = dict(ru=s(e["ru"]), en=e["en"])
            if "word" in e:
                d.update(word=split_stress(e["word"])[0].lower(), cell=e["cell"])
            out["examples"].append(d)
        for g in u["gaps"]:
            out["gaps"].append(dict(before=s(g["before"]), answer=s(g["answer"]), after=s(g["after"]), hint=g["hint"], word=split_stress(g["word"])[0].lower(), cell=g["cell"]))
        uses.append(out)
    tables.append(dict(id="uses", kind="use", title="What the cases do", subtitle="у меня́ есть · в до́ме · чита́ю кни́гу · с молоко́м",
                       note="A case is a job a noun does in the sentence. These are the jobs the course gives them.",
                       rows=[dict(id=u["id"], label=u["title"], gloss=u["rule"].split(". ")[0]) for u in uses], cols=[dict(id="use", label="case")],
                       cells=[dict(id=u["id"], row=u["id"], col="use", ending=dict((c, l) for c, l, _ in CASES)[u["case"]], components=[u["component"]] + u["legacy"],
                                   words=sorted({e["word"] for e in u["examples"] if "word" in e})) for u in uses], words=[]))

    return dict(version=1, components=components, tables=tables, words=words, uses=uses,
                exceptions={k: s(v) for k, v in EXCEPTIONS.items()}, forms=dict(sorted(forms_index.items())))


if __name__ == "__main__":
    data = build()
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    n_forms = sum(len(w["forms"]) for w in data["words"].values())
    print(f"{OUT}: {len(data['tables'])} tables, {len(data['components'])} components, {len(data['words'])} words, {n_forms} forms, {len(data['uses'])} uses")
