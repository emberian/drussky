"""Unit 23: word families and cognates — the course tail that teaches Russian as a system of
roots, prefixes and suffixes, and the Greek/Latin bridge into mathematics and philosophy.

Same conventions as build_course.py: an apostrophe after a Cyrillic vowel marks stress. Two
exercise kinds of its own (COURSE_SCHEMA.md): build_word (tiles → word) and family_guess (a new
word on a known root, guessed in a sentence). Their data agrees with Resources/morphology.json;
validate_course.py checks that.
"""
import hashlib


def bw(en, answer, parts, extra=()):
    """build_word: `parts` spell `answer` in order; `extra` tiles don't belong. The tray order is
    fixed by a hash of the answer, so a rebuild never moves it."""
    tiles = list(parts) + list(extra)
    order = sorted(range(len(tiles)), key=lambda i: hashlib.sha1(f"{answer}|{tiles[i]}".encode()).hexdigest())
    e = {"kind": "build_word", "en": en, "answer": answer, "parts": list(parts), "options": [tiles[i] for i in order]}
    return e


def fg(text, word, root, options, en, answer=0):
    """family_guess: `word` (a lemma inside `text`) is new; `root` is its root id; options[answer]
    is its meaning, placed by a hash of the word."""
    right, others = options[answer], [o for i, o in enumerate(options) if i != answer]
    pos = int(hashlib.sha1(word.encode()).hexdigest(), 16) % len(options)
    return {"kind": "family_guess", "text": text, "word": word, "root": root, "options": others[:pos] + [right] + others[pos:],
            "answer": pos, "en": en}


def units(h):
    ex, tr, cp, lp, dc, ra, tl, fl, rm = (h[k] for k in ("ex", "tr", "cp", "lp", "dc", "ra", "tl", "fl", "rm"))
    return [("u23", "Word families", "ко'рни · приста'вки · су'ффиксы · Greek and Latin", [
        ("One root, many words: пис-", "Russian builds its words from a few hundred roots. Learn the root and its prefixes and suffixes, and whole families come at once.", [
            ex("The root пис- (пиш- before some endings) means write: писа'ть to write, письмо' a letter, писа'тель a writer, написа'ть to write and finish. Prefixes and suffixes do the rest.",
               [("писа'ть", "to write"), ("письмо'", "a letter (mail)"), ("писа'тель", "a writer"), ("написа'ть", "to write (and finish)"), ("пи'шет", "writes")]),
            bw("a writer", "писа'тель", ["пис", "-а'-", "-тель"], ["пере-", "-ник"]),
            fg("Я хочу' написа'ть письмо'.", "написа'ть", "пис", ["to write (and finish)", "to read (to the end)", "to sell"], "I want to write a letter."),
            tl("to write", "писа'ть"), tl("a letter (mail)", "письмо'"),
            fl("Мой брат — ", "писа'тель", ".", "My brother is a writer."),
            dc("Писа'тель пи'шет кни'гу."),
            cp("писа'ть, написа'ть, письмо'", 1),
            ra("Писа'тель пи'шет письмо'."),
        ]),
        ("Reading and counting: чит-", "One root for reading and counting, and число́ grows from it too.", [
            ex("Чита'ть and счита'ть, to count, share one root: чит-, чт-, чёт-. So do число', a number, and чётное, even. A reader is чита'тель; reading is чте'ние.",
               [("чита'тель", "a reader"), ("чте'ние", "reading"), ("счита'ть", "to count; to consider"), ("число'", "number"), ("чётное", "even (of a number)")]),
            bw("a reader", "чита'тель", ["чит", "-а'-", "-тель"], ["про-", "-ник"]),
            fg("Ро'бот лю'бит счита'ть звёзды.", "счита'ть", "чит", ["to count", "to read to the end", "to rewrite"], "The robot loves counting stars."),
            tl("a reader", "чита'тель"), tl("reading", "чте'ние"),
            fl("Чита'тель чита'ет, а писа'тель ", "пи'шет", ".", "The reader reads, and the writer writes."),
            dc("Я люблю' чте'ние."),
            cp("чита'ть, счита'ть, число'", 1),
            ra("Счита'ть — э'то чита'ть чи'сла."),
        ]),
        ("In, out, across: prefixes on ход-", "Ходи́ть and идти́ share the root ход-. A prefix gives the direction, and a noun falls out.", [
            ex("В- is in, вы- is out, пере- is across. On the root ход-, go: вход the way in, вы'ход the way out, перехо'д a crossing.",
               [("вход", "entrance"), ("вы'ход", "exit"), ("перехо'д", "crossing, transition"), ("ходи'ть", "to go (regularly, on foot)")]),
            bw("crossing, transition", "перехо'д", ["пере-", "хо'д"], ["вы-", "-ник"]),
            fg("Где вход в парк?", "вход", "ход", ["entrance", "exit", "crossing"], "Where is the entrance to the park?"),
            tl("exit", "вы'ход"), tl("entrance", "вход"),
            fl("Где ", "вы'ход", "?", "Where is the exit?"),
            dc("Вход там, а вы'ход тут."),
            cp("вход, вы'ход, перехо'д", 1),
            ra("Вход, вы'ход, перехо'д."),
        ]),
        ("Suffixes: -тель, -ник, -ость, -ние", "A suffix says what kind of word you have: a person, a quality, an activity.", [
            ex("-тель and -ник make a person: учи'тель teacher, учени'к pupil, рабо'тник worker. -ость makes a quality: му'дрость wisdom. -ние makes an activity: зна'ние knowledge.",
               [("учи'ть", "to teach; to learn"), ("учи'тель", "teacher"), ("учени'к", "pupil"), ("рабо'тник", "worker"), ("зна'ние", "knowledge")]),
            bw("teacher", "учи'тель", ["уч", "-и'-", "-тель"], ["на-", "-ник"]),
            fg("Рабо'тник рабо'тает ка'ждый день.", "рабо'тник", "раб", ["worker", "teacher", "pupil"], "The worker works every day."),
            tl("teacher", "учи'тель"), tl("pupil", "учени'к"),
            fl("Му'дрый челове'к зна'ет, что тако'е ", "му'дрость", ".", "A wise person knows what wisdom is."),
            dc("Учи'тель и учени'к."),
            cp("учи'ть, учи'тель, учени'к", 1),
            ra("Учи'тель, учени'к, рабо'тник, зна'ние."),
        ]),
        ("Greek you already know", "Greek came into Russian sound by sound. Read these as Greek and they are yours.", [
            ex("Th is т (тео'рия), ph is ф (филосо'фия), -y is -ия, and -ics is -ика (ло'гика, матема'тика). The pieces are Greek too: фило- love, -со'фия wisdom, гео- earth, -ме'трия measuring.",
               [("тео'рия", "theory"), ("филосо'фия", "philosophy"), ("ло'гика", "logic"), ("систе'ма", "system"), ("матема'тика", "mathematics"), ("геоме'трия", "geometry")]),
            bw("philosophy", "филосо'фия", ["фило-", "-со'фия"], ["гео-", "-ло'гия"]),
            bw("geometry", "геоме'трия", ["гео-", "-ме'трия"], ["фило-", "-со'фия"]),
            tl("theory", "тео'рия"), tl("logic", "ло'гика"),
            fl("Э'то не тео'рия, э'то ", "факт", ".", "This is not a theory, it is a fact."),
            dc("Э'то моя' тео'рия."),
            cp("филосо'фия, геоме'трия", 1),
            ra("Тео'рия, филосо'фия, ло'гика, систе'ма, матема'тика."),
        ]),
        ("Latin you already know", "Latin came the same way, with its own endings.", [
            ex("-tion is -ция (фу'нкция), -ure is -ура (структу'ра), -ment is -мент (аргуме'нт, элеме'нт). Abstractio is абстра'кция; informatio, информа'ция.",
               [("фу'нкция", "function"), ("структу'ра", "structure"), ("аргуме'нт", "argument"), ("абстра'кция", "abstraction"), ("информа'ция", "information"), ("элеме'нт", "element")]),
            bw("function", "фу'нкция", ["фу'нк-", "-ция"], ["структ-", "-у'ра"]),
            bw("structure", "структу'ра", ["структ-", "-у'ра"], ["-ция", "-ме'нт"]),
            tl("argument", "аргуме'нт"), tl("information", "информа'ция"),
            fl("Два — э'то ", "элеме'нт", ", а не систе'ма.", "Two is an element, not a system."),
            dc("Э'то абстра'кция."),
            cp("фу'нкция, структу'ра", 1),
            ra("Фу'нкция, структу'ра, аргуме'нт, абстра'кция."),
        ]),
    ])]


W = lambda ru, en, emoji=None, note=None: {k: v for k, v in dict(ru=ru, en=en, emoji=emoji, note=note).items() if v is not None}
LEXICON = [
    # u23
    W("писа'ть", "to write"), W("письмо'", "a letter (mail)", "✉️"), W("писа'тель", "a writer"), W("написа'ть", "to write (and finish)"),
    W("чита'тель", "a reader"), W("чте'ние", "reading"), W("счита'ть", "to count; to consider"),
    W("вход", "entrance"), W("вы'ход", "exit"), W("перехо'д", "crossing, transition"),
    W("учи'ть", "to teach; to learn"), W("учи'тель", "teacher"), W("учени'к", "pupil"), W("рабо'тник", "worker"), W("зна'ние", "knowledge"),
    W("тео'рия", "theory"), W("филосо'фия", "philosophy"), W("ло'гика", "logic"), W("систе'ма", "system"), W("матема'тика", "mathematics"),
    W("геоме'трия", "geometry"),
    W("фу'нкция", "function"), W("структу'ра", "structure"), W("аргуме'нт", "argument"), W("абстра'кция", "abstraction"),
    W("информа'ция", "information"), W("элеме'нт", "element"),
]
