"""Grammar pages: one page at the end of each unit that introduces a system (gender and the
three declensions, the pronoun cases behind у меня есть, the two conjugations, the irregular
verbs and с + instrumental, the past tense, the prepositional, the accusative of direction, the
genitive). Each shows the table itself and asks for cells of it.

Same conventions as build_course.py: an apostrophe after a Cyrillic vowel marks stress. Three
exercise kinds of their own (COURSE_SCHEMA.md): paradigm_fill (a table with cells to fill, by
Pencil or from tiles), transform (one form from a cue, bare or in a sentence with a gap) and
cue_sort (phrases sorted by the job their case does). Their forms are read from
Resources/paradigms.json so they can never drift from it; validate_course.py checks that too.
"""
import hashlib
import json
import re
from pathlib import Path

PARADIGMS = json.loads((Path(__file__).resolve().parents[1] / "Drussky/Resources/paradigms.json").read_text())
WORDS = PARADIGMS["words"]
USES = {u["id"]: u for u in PARADIGMS["uses"]}


def _key(s):
    return re.sub(r"[́']", "", s).lower()


def _form(word, cell):
    w = WORDS[_key(word)]
    try:
        return w["forms"][cell]
    except KeyError:
        raise KeyError(f"{word} has no cell {cell}") from None


def pf(word, cells, asked, tiles=None, table=None):
    """paradigm_fill: `cells` are shown (with their forms), `asked` among them are to be
    produced; `tiles` = extra wrong tiles, which turns the exercise into tap-to-fill."""
    w = WORDS[_key(word)]
    forms = {c: _form(word, c) for c in cells}
    e = {"kind": "paradigm_fill", "word": w["ru"], "paradigm": table or w["table"], "forms": forms, "cells": list(asked)}
    if tiles is not None:
        options = [forms[c] for c in asked] + list(tiles)
        order = sorted(range(len(options)), key=lambda i: hashlib.sha1(f"{word}|{options[i]}".encode()).hexdigest())
        e["options"] = [options[i] for i in order]
    return e


def tf(word, cue, before=None, after=None, hint=None):
    """transform: `word` in the `cue` cell; with before/after/hint a sentence with the form missing."""
    w = WORDS[_key(word)]
    answer = _form(word, cue).split(" / ")[0]
    e = {"kind": "transform", "word": w["ru"], "cue": cue, "answer": answer}
    if before is not None or after is not None:
        e.update(before=before or "", after=after or "", hint=hint)
    return e


def cs(buckets, items):
    """cue_sort: `buckets` are use ids (their titles label the buckets) or table ids; `items` are
    (text, bucket index) pairs. The item order is fixed by a hash, so a rebuild never moves it."""
    options = [USES[b]["title"] if b in USES else next(t["title"] for t in PARADIGMS["tables"] if t["id"] == b) for b in buckets]
    its = [{"text": t, "answer": a} for t, a in items]
    order = sorted(range(len(its)), key=lambda i: hashlib.sha1(its[i]["text"].encode()).hexdigest())
    return {"kind": "cue_sort", "paradigm": "uses" if buckets[0] in USES else "tables", "options": options, "items": [its[i] for i in order]}


# Words the tables need as headwords (their forms were in the course before the words were).
LEXICON = [{"ru": "хоте'ть", "en": "to want"}, {"ru": "идти'", "en": "to go (on foot), to be going"}, {"ru": "быть", "en": "to be"},
           {"ru": "рабо'та", "en": "work (the noun)"}]


def pages(h):
    """(unit id, page) pairs; each page is a 4-tuple with an explicit id, appended to its unit."""
    ex, tr, cp, lp, dc, ra, tl, fl, rm = (h[k] for k in ("ex", "tr", "cp", "lp", "dc", "ra", "tl", "fl", "rm"))
    return [
        ("u8", ("g1", "Three declensions", "Every noun belongs to one of three tables. The ending of the dictionary form tells you which.", [
            ex("Nouns I end in -а or -я (кни'га, ку'хня), nouns II in a consonant, -й, -ь or in -о, -е (стол, окно'), nouns III in -ь and are feminine (ночь, соль). The table decides every other ending the word will ever take.",
               [("кни'га", "book"), ("стол", "table"), ("окно'", "window"), ("ночь", "night")]),
            pf("кни'га", ["nom-sg", "nom-pl"], ["nom-pl"]),
            pf("стол", ["nom-sg", "nom-pl"], ["nom-pl"], tiles=["сто'ла", "столо'в"]),
            cs(["decl1", "decl2", "decl3"], [("ма'ма", 0), ("стол", 1), ("окно'", 1), ("ночь", 2), ("кни'га", 0), ("дом", 1)]),
            tf("окно'", "nom-pl"), tf("кот", "nom-pl"), tf("ма'ма", "nom-pl"),
            dc("Э'то столы' и о'кна."),
            ra("Стол, столы'. Окно', о'кна. Кни'га, кни'ги."),
        ])),
        ("u10", ("g2", "у меня' есть: the pronouns change", "Having is said as «at me there is». The person stands in the genitive; here is the whole table for я and ты.", [
            ex("Each pronoun has six forms, learned as words. After у (at) and нет (there is no) comes the genitive: меня', тебя'. The accusative, the object, looks the same: Он зна'ет меня'.",
               [("у меня' есть", "I have"), ("у тебя' есть", "you have"), ("меня'", "me")]),
            pf("я", ["nom", "gen", "acc"], ["gen"], tiles=["мне", "тебя'"], table="pron"),
            pf("ты", ["nom", "gen", "acc"], ["gen", "acc"]),
            tf("я", "gen", before="У ", after=" есть кот.", hint="I have a cat."),
            tf("ты", "gen", before="У ", after=" есть кни'га?", hint="Do you have a book?"),
            tf("я", "acc", before="Он зна'ет ", after=".", hint="He knows me."),
            dc("У тебя' есть сестра'?"),
            ra("У меня' есть брат. А у тебя'?"),
            cs(["have", "gen-no"], [("У меня' есть кни'га.", 0), ("У меня' нет хле'ба.", 1), ("У тебя' есть кот?", 0), ("Нет со'ли.", 1)]),
        ])),
        ("u12", ("g3", "Two conjugations", "Every present-tense verb runs on one of two sets of endings: -ешь or -ишь. Here are both tables, side by side.", [
            ex("Conjugation I has е in its endings: чита'ю, чита'ешь, чита'ет, чита'ем, чита'ете, чита'ют. Conjugation II has и: говорю', говори'шь, говори'т, говори'м, говори'те, говоря'т. The я form and the они' form give the rest away.",
               [("чита'ть", "to read"), ("говори'ть", "to speak")]),
            pf("чита'ть", ["1sg", "2sg", "3sg", "1pl", "2pl", "3pl"], ["2sg", "1pl", "3pl"]),
            pf("говори'ть", ["1sg", "2sg", "3sg", "1pl", "2pl", "3pl"], ["2sg", "3sg", "2pl"], tiles=["говоря'т", "чита'ете"]),
            tf("знать", "2pl", before="Вы ", after=", где кот?", hint="Do you know where the cat is?"),
            tf("ду'мать", "3pl", before="Они' ", after=", что э'то пра'вда.", hint="They think that it's true."),
            tf("говори'ть", "1pl", before="Мы ", after=" по-ру'сски.", hint="We speak Russian."),
            lp("говори'т", ["говори'т", "говори'м", "говоря'т"]),
            dc("Они' чита'ют и ду'мают."),
            ra("Я чита'ю, ты чита'ешь. Я говорю', ты говори'шь."),
        ])),
        ("u14", ("g4", "хоте́ть, есть, дать — and с + чем?", "Four verbs keep their own tables, and the word for «with» asks for the instrumental.", [
            ex("хоте'ть runs on conjugation I in the singular and II in the plural: хочу', хо'чешь, хо'чет, хоти'м, хоти'те, хотя'т. есть (to eat) and дать (to give) are older still: ем, ешь, ест; дам, дашь, даст. Переда'й is the imperative of переда'ть.",
               [("хоте'ть", "to want"), ("дать", "to give"), ("переда'й", "pass")]),
            pf("хоте'ть", ["1sg", "2sg", "3sg", "1pl", "2pl", "3pl"], ["3sg", "1pl", "3pl"], tiles=["хо'чем", "хоти'т"], table="irregular"),
            tf("хоте'ть", "2sg", before="Ты ", after=" чай?", hint="Do you want tea?"),
            ex("With: с + instrumental. Masculine and neuter nouns take -ом: с молоко'м, с са'харом; feminine -ой: с ма'мой.",
               [("с молоко'м", "with milk"), ("с са'харом", "with sugar")]),
            tf("молоко'", "instr-sg", before="Чай с ", after=".", hint="Tea with milk."),
            tf("са'хар", "instr-sg", before="Ко'фе с ", after=".", hint="Coffee with sugar."),
            tf("ма'ма", "instr-sg", before="Я иду' с ", after=".", hint="I am walking with mom."),
            dc("Я хочу' чай с молоко'м."),
            ra("Переда'й мне са'хар, пожа'луйста. Я хочу' ко'фе с са'харом."),
        ])),
        ("u17", ("g5", "The past-tense table", "One form per gender and one plural, whoever is speaking. Here is the table, and the verbs that move their stress.", [
            ex("Drop -ть, add -л, -ла, -ло, -ли: чита'л, чита'ла, чита'ло, чита'ли. The form agrees with the subject's gender or number, not its person: я чита'л, я чита'ла, мы чита'ли. быть moves its stress: был, была', бы'ло, бы'ли.",
               [("чита'л", "read (he)"), ("чита'ла", "read (she)"), ("быть", "to be"), ("бы'ли", "were")]),
            pf("чита'ть", ["past-m", "past-f", "past-n", "past-pl"], ["past-f", "past-n", "past-pl"], table="past"),
            pf("быть", ["past-m", "past-f", "past-n", "past-pl"], ["past-f", "past-pl"], tiles=["бы'ла", "бу'дет"], table="past"),
            tf("быть", "past-f", before="Она' ", after=" до'ма.", hint="She was at home."),
            tf("рабо'тать", "past-m", before="Он ", after=" вчера'.", hint="He worked yesterday."),
            tf("ду'мать", "past-pl", before="Они' ", after=" до'лго.", hint="They thought for a long time."),
            lp("была'", ["была'", "был", "бы'ли"]),
            dc("Она' чита'ла, а он ду'мал."),
            ra("Был, была', бы'ло, бы'ли. Чита'л, чита'ла, чита'ло, чита'ли."),
        ])),
        ("u18", ("g6", "Where: the prepositional", "Being somewhere takes в or на and the -е ending. The accusative marks what a verb acts on. Two cells of the noun tables, side by side.", [
            ex("Где? — в до'ме, на столе', в кни'ге: the prepositional ends in -е for all three tables (-и after -ия, -ие and for nouns in -ь: на ста'нции). The object of a verb is the accusative: for nouns I, -а → -у: Я чита'ю кни'гу.",
               [("на столе'", "on the table"), ("в кни'ге", "in the book"), ("кни'гу", "book (as the object)")]),
            pf("кни'га", ["nom-sg", "acc-sg", "prep-sg", "nom-pl"], ["acc-sg", "prep-sg"]),
            pf("стол", ["nom-sg", "acc-sg", "prep-sg", "nom-pl"], ["acc-sg", "prep-sg"], tiles=["стола'", "столы'"]),
            tf("стол", "prep-sg", before="Кни'га на ", after=".", hint="The book is on the table."),
            tf("звезда'", "acc-sg", before="Я ви'жу ", after=".", hint="I see a star."),
            tf("ку'хня", "prep-sg", before="Мы на ", after=".", hint="We are in the kitchen."),
            cs(["acc-obj", "prep-e"], [("Я чита'ю кни'гу.", 0), ("Кни'га на столе'.", 1), ("Я ви'жу звезду'.", 0), ("Кот в до'ме.", 1)]),
            dc("Кни'га на столе' в до'ме."),
            ra("Где кни'га? На столе'. Что я чита'ю? Кни'гу."),
        ])),
        ("u19", ("g7", "Where to: the accusative of direction", "Куда́? takes the same в and на — with the accusative. The table of идти́ and е́хать goes with it.", [
            ex("Where something is: в го'роде (prepositional). Where it is going: в го'род (accusative). Nouns II that are things keep their dictionary form as the accusative, so the preposition does all the work: в парк, but в па'рке.",
               [("идти'", "to go (on foot)"), ("рабо'та", "work"), ("в го'род", "to the city"), ("в го'роде", "in the city")]),
            pf("идти'", ["1sg", "2sg", "3sg", "1pl", "2pl", "3pl"], ["2sg", "3sg", "1pl", "3pl"], tiles=["иду'м", "е'дут"], table="irregular"),
            tf("рабо'та", "acc-sg", before="Я е'ду на ", after=".", hint="I am going to work."),
            tf("го'род", "acc-sg", before="Мы идём в ", after=".", hint="We are going to the city."),
            pf("рабо'та", ["nom-sg", "acc-sg", "prep-sg"], ["acc-sg", "prep-sg"]),
            tf("парк", "acc-sg", before="Мы идём в ", after=".", hint="We are going to the park."),
            cs(["acc-dir", "prep-e"], [("Я иду' в го'род.", 0), ("Я живу' в го'роде.", 1), ("Я е'ду на рабо'ту.", 0), ("Кни'га на столе'.", 1)]),
            dc("Я иду' в парк, а ты идёшь домо'й."),
            ra("Куда' ты идёшь? В го'род. Где ты живёшь? В го'роде."),
        ])),
        ("u21", ("g8", "The genitive: нет, of, and after numbers", "One case, three jobs: what is missing, what something is of, and what comes after a number.", [
            ex("Nouns II take -а (-я) in the genitive singular: нет хле'ба, для любо'го числа'. Nouns I take -ы (-и): нет кни'ги. In the plural, nouns I and neuter nouns II lose their ending — книг, чи'сел — and masculine nouns take -ов: пять столо'в.",
               [("хле'ба", "bread (after нет)"), ("для любо'го числа'", "for any number"), ("двух чётных чи'сел", "of two even numbers")]),
            pf("число'", ["nom-sg", "gen-sg", "nom-pl", "gen-pl"], ["gen-sg", "gen-pl"]),
            pf("кни'га", ["nom-sg", "gen-sg", "nom-pl", "gen-pl"], ["gen-sg", "gen-pl"], tiles=["кни'ге", "кни'гу"]),
            tf("хлеб", "gen-sg", before="У меня' нет ", after=".", hint="I have no bread."),
            tf("число'", "gen-pl", before="су'мма двух ", after="", hint="the sum of two numbers"),
            tf("стол", "gen-pl", before="пять ", after="", hint="five tables"),
            cs(["gen-no", "genitive", "have"], [("У меня' нет хле'ба.", 0), ("су'мма двух чи'сел", 1), ("У меня' есть кни'га.", 2), ("Нет молока'.", 0), ("для любо'го числа'", 1)]),
            dc("Су'мма двух чётных чи'сел — чётное число'."),
            ra("Оди'н стол, два стола', пять столо'в."),
        ])),
    ]
