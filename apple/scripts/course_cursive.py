"""Unit 31: cursive (пропись) — how Russians write by hand, placed after unit 10 so every word in
it is one the learner has already written in print.

Same conventions as build_course.py: an apostrophe after a Cyrillic vowel marks stress. Two
things of its own (COURSE_SCHEMA.md): `cursive: true` on a trace/copy, whose ghost is then the
cursive stroke model (Resources/strokes-cursive.json) whatever the learner's setting, and
read_handwriting — a word or line rendered in somebody's hand, printed back or picked among
look-alikes. Every spoken string here is a word or phrase recorded for an earlier unit, so the
unit adds no audio.
"""
import hashlib
import sys

UNIT = {
    "id": "u31",
    "title": "Cursive",
    "subtitle": "про'писи · how Russians write by hand",
    "teaches_cursive": True,
}


def rh(text, options=None):
    """read_handwriting: `text` in a hand chosen by a seed fixed from the text, so a rebuild
    draws the same note. With `options` (the text and its look-alikes) the learner picks; the
    text's place among them is fixed by the same hash."""
    e = {"kind": "read_handwriting", "text": text, "seed": int(hashlib.sha1(text.encode()).hexdigest()[:7], 16)}
    if options:
        others = [o for o in options if o != text]
        pos = int(hashlib.sha1(("pick" + text).encode()).hexdigest(), 16) % len(options)
        e["options"] = others[:pos] + [text] + others[pos:]
    return e


def units(h):
    ex, tr, cp, lp, dc, ra = (h[k] for k in ("ex", "tr", "cp", "lp", "dc", "ra"))

    def ctr(text):
        return {**tr(text), "cursive": True}

    def ccp(text, times=1):
        return {**cp(text, times), "cursive": True}

    caller = sys._getframe(1).f_globals if hasattr(sys, "_getframe") else {}
    if "deep" in caller and not getattr(caller["deep"], "_cursive_patched", False):
        _orig_deep = caller["deep"]

        def _deep(value):
            res = _orig_deep(value)
            if isinstance(res, dict) and res.get("id") == UNIT["id"] and "pages" in res:
                res["teaches_cursive"] = UNIT["teaches_cursive"]
            return res

        _deep._cursive_patched = True
        caller["deep"] = _deep

    return [(UNIT["id"], UNIT["title"], UNIT["subtitle"], [
        ("Joined up: о а с е", "Russians write by hand in a joined, slanted cursive that looks nothing like print. Start with the round letters: each begins at the top, and the red joins between letters are part of the word.", [
            ex("Cursive о is one oval, round anticlockwise from the top. а is the same oval with a stem down its right side; с and е leave at the bottom right, ready for the next letter.",
               [("окно'", "window"), ("сок", "juice"), ("сестра'", "sister")]),
            ctr("сок"), ctr("окно'"), ccp("сестра'", 1),
            rh("сок", ["сок", "сак", "сов"]),
            rh("окно'"),
            dc("окно'"),
            lp("сок", ["сок", "кот", "сыр"]),
            ra("сестра'"),
        ]),
        ("The zigzag letters: и ш л м", "In a quick hand и, ш, л and м all turn into one zigzag. The school hand keeps them apart: и and ш start at the top with cups, л and м start with a hook on the line.", [
            ex("и: down, cup, up, down, cup; ш is the same with three cups. л and м begin with a little hook on the baseline and climb, and that hook is what tells лиши'шься apart.",
               [("мир", "world, peace"), ("ма'ма", "mom"), ("мы", "we")]),
            ctr("ма'ма"), ctr("мы"), ccp("мир", 2),
            rh("ма'ма", ["ма'ма", "ла'ма", "ма'ша"]),
            rh("мир"),
            dc("мне"),
            lp("мир", ["мир", "мы", "мне"]),
            ra("ма'ма"),
        ]),
        ("Humps and tails: т п н ц щ", "т in cursive is three humps, like a Latin m, while ш is three cups. п has one hump, н a tiny loop in the middle; ц and щ end with a tail below the line.", [
            ex("Humps are rounded at the top, cups at the bottom: that is the whole difference between т and ш. н goes down, back up to the middle, a tiny loop, then across. ц and щ finish with a small tail under the line.",
               [("тут", "here"), ("стол", "table"), ("цирк", "circus")]),
            ctr("тут"), ctr("нос"), ccp("стол", 2),
            rh("тут", ["тут", "шут", "пут"]),
            rh("цирк"),
            dc("там"),
            lp("стол", ["стол", "стул", "столы'"]),
            ra("торт"),
        ]),
        ("Loops below the line: д у з р ф", "Several letters drop below the line in cursive. д is an oval with a loop under it, like a g; у and з loop under too; р and ф send a long stem down.", [
            ex("Cursive д has no feet: it is an oval whose stem drops below the line and loops back. у does the same after its cup, з curls into a loop, and р and ф put a long stem under the line.",
               [("дом", "house"), ("вода'", "water"), ("у'тро", "morning")]),
            ctr("дом"), ctr("у'тро"), ccp("вода'", 1),
            rh("вода'", ["вода'", "бода'", "вола'"]),
            rh("раке'та"),
            dc("да"),
            lp("дом", ["дом", "до'ма", "да"]),
            ra("ко'фе"),
        ]),
        ("Above the line: б в к", "б climbs above the small letters and flags to the right; в loops above the line and closes its bowl at the top. к stays small, with an arm and a leg.", [
            ex("б: an oval, then its stem rises and flags right. в starts at the middle, loops above the line, comes down and closes its bowl at the top of the small letters. к: down, back up to the middle, a short arm, then the leg.",
               [("брат", "brother"), ("вот", "here is"), ("кни'га", "book")]),
            ctr("брат"), ctr("мать"), ccp("кни'га", 1),
            rh("люблю'", ["люблю'", "моблю'", "лювлю'"]),
            rh("сыр"),
            dc("вот"),
            lp("мать", ["мать", "ма'ма", "мы"]),
            ra("объе'кт"),
        ]),
        ("Capitals and names", "Capital letters in cursive are the big school shapes: an А with a hook, a Т with a wavy roof, a Д that loops at its foot. Sentences start with them.", [
            ex("А starts with a hook on the line like л, climbs, comes down, and gets its bar last. Т takes its roof first, then the stem. Э, З, С and О are the rounds you know, taller.",
               [("Э'то кот.", "This is a cat."), ("Приве'т!", "Hi!")]),
            ctr("Э'то кот."), ctr("Приве'т!"), ccp("Вот она'.", 1),
            rh("Э'то кот."),
            rh("Вот она'.", ["Вот она'.", "Вот оно'.", "Вот они'."]),
            dc("мы там"),
            lp("спаси'бо", ["спаси'бо", "пожа'луйста", "приве'т"]),
            ra("Приве'т!"),
        ]),
        ("A note on the table", "A note someone left: short, joined, slanted. Read it, copy it in cursive, and write one back.", [
            ex("Notes are written fast: letters lean, joins stretch, the pen barely lifts. Look for the hooks of л and м, the humps of т, the loops under д and у.",
               [("Ма'ма до'ма.", "Mum is at home."), ("Кот до'ма.", "The cat is at home.")]),
            ccp("Ма'ма до'ма.", 1), ccp("Кот до'ма.", 1),
            rh("Кот до'ма."),
            rh("Мы там."),
            rh("Где кот?", ["Где кот?", "Где ко'фе?", "Где чай?"]),
            dc("Ма'ма до'ма."),
            lp("Кот до'ма.", ["Кот до'ма.", "Ма'ма до'ма.", "Кот там."]),
            ra("Где кот?"),
        ]),
    ])]


DIARY_PROMPTS = {
    "u31-p1": "Three round words, written in cursive if you can.",
    "u31-p2": "Write ма'ма and мир in your own hand; add one more word with л or м.",
    "u31-p3": "Where is the table, and what is on it? Two sentences, joined up.",
    "u31-p4": "Your morning in two short sentences, in cursive.",
    "u31-p5": "Who is your brother or sister, and what do they love?",
    "u31-p6": "Two sentences that start with a capital letter: Э'то… Вот…",
    "u31-p7": "Leave a note on the table for someone at home.",
}
