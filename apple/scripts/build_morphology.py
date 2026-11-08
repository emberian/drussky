#!/usr/bin/env python3
"""Author source for Drussky's morphology.json: roots, prefixes, suffixes, the segmentation of
every course lemma (and the most frequent library lemmas), and the Greek/Latin cognate bridge.

Conventions (same as build_course.py): an apostrophe after a Cyrillic vowel marks stress and
becomes U+0301 on output. A lemma is written as its segmentation, morphemes separated by `|`,
each root in square brackets:  "пере|[пис]|а'|ть".  The headword is the segmentation with the
marks removed. The first bracketed root is the word's family unless `family=` says otherwise;
a lemma with no bracketed root (да, здесь, прия'тный) belongs to no family.

Sources and the segmentation policy are in scripts/morphology_sources.md. Run:
    python3 apple/scripts/build_morphology.py && python3 apple/scripts/validate_morphology.py
"""
import json
import re
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[1] / "Drussky/Resources/morphology.json"
ACUTE = re.compile(r"([аеиоуыэюяАЕИОУЫЭЮЯ])'")


def s(text):
    return ACUTE.sub(lambda m: m.group(1) + "́", text)


# --- roots -------------------------------------------------------------------------------
# (id, forms (allomorphs as they appear inside words), gloss, note)
R = lambda id, forms, gloss, note=None: {k: v for k, v in dict(id=id, forms=forms.split(), gloss=gloss, note=note).items() if v is not None}
ROOTS = [
    # knowing, thinking, saying
    R("зна", "зна знак знач", "know", "знак is a sign (what lets you know); зна'чить, to mean, is built on it."),
    R("дум", "дум", "think"),
    R("мысл", "мысл мышл", "thought, think"),
    R("ум", "ум умн", "mind", "ра'зум is reason (раз- + ум); у'мный is clever."),
    R("вер", "вер", "believe, trust", "ве'рный is faithful or correct; наве'рное (probably) is 'for certain', used ironically."),
    R("прав", "прав", "right, true, straight", "пра'вда truth, пра'во a right, спра'ва on the right, пра'вильно correctly."),
    R("истин", "истин", "truth (what is)"),
    R("говор", "говор", "speak"),
    R("каз", "каз каж", "show; say", "сказа'ть to say, показа'ть to show, доказа'ть to prove (show to the end), каза'ться to seem."),
    R("прос", "прос праш", "ask", "вопро'с a question, спроси'ть to ask (once), спра'шивать to ask (repeatedly)."),
    R("вет", "вет веч", "speak, counsel (old)", "отве'т answer, приве'т greeting, сове'т advice (council)."),
    R("молч", "молч молк", "be silent"),
    R("слух", "слух слыш слуш", "hear", "слы'шать to hear, слу'шать to listen."),
    R("вид", "вид виж", "see", "ви'деть to see, вид a view."),
    R("смотр", "смотр", "look"),
    R("чит", "чит чт чёт чис", "read; count", "Reading and counting were one verb: счита'ть to count, чётный even, число' a number, чте'ние reading."),
    R("пис", "пис пиш", "write"),
    R("уч", "уч ук", "learn, teach", "учи'ть to teach or learn, учени'к pupil, учи'тель teacher, нау'ка science."),
    R("мер", "мер", "measure", "изме'рить to measure, разме'р size, приме'р an example (a measure held up)."),
    R("след", "след", "track, follow", "сле'довательно therefore (following), сле'дующий next, иссле'дование research."),
    R("им", "им ня ем я", "take, have", "име'ть to have, поня'ть to understand (take in), приня'ть to accept, взять to take."),
    R("мн", "мн мя мин", "remember, think", "по'мнить to remember, па'мять memory, мне'ние opinion."),
    R("треб", "треб", "require"),
    R("сущ", "сущ сут", "be, exist", "существова'ть to exist, су'щность essence, суть the gist."),
    R("бы", "бы буд бу", "be", "быть to be, бу'ду I will be, быт everyday life."),
    R("ес", "ес ед", "eat", "есть to eat, еда' food, обе'д lunch (eating round the table), съесть to eat up."),
    R("пи", "пи пь", "drink", "пить to drink, напи'ток a drink."),
    R("сп", "сп сн сон", "sleep", "спать to sleep, сон a dream or sleep."),
    R("покой", "покой поко", "rest, calm", "споко'йный calm, поко'й peace and quiet."),
    R("жи", "жи жив", "live", "жить to live, жизнь life, живо'й alive."),
    R("люб", "люб", "love", "люби'ть to love, любо'й any (whichever pleases)."),
    R("хот", "хот хоч", "want"),
    R("мог", "мог мож моч мо", "be able", "мочь to be able, мо'жно one may, мо'жет maybe, мощь might."),
    R("раб", "раб роб", "work, slave", "рабо'та work, рабо'тать to work, ро'бот (Čapek, from Czech robota)."),
    R("дел", "дел", "do, deed", "де'лать to do, де'ло a matter, неде'ля a week (once: the no-work day, Sunday)."),
    R("дели", "дел", "divide, part", "дели'ть to divide, преде'л limit, определе'ние definition, отде'л department."),
    R("да", "да", "give", "дать to give, зада'ча a task (what is set), переда'ть to pass, прода'ть to sell."),
    R("ход", "ход ид й ш шл шед", "go (on foot)", "ходи'ть to go, идти' to be going, вход entrance, вы'ход exit, перехо'д crossing."),
    R("ех", "ех езд е", "ride, travel", "е'хать to be going (by transport), е'здить to travel, по'езд train."),
    R("лет", "лет лёт", "fly", "лете'ть to fly, самолёт aeroplane (self-flier), полёт a flight."),
    R("бег", "бег беж", "run"),
    R("нес", "нес нос", "carry", "нести' to carry, носи'ть to wear or carry about."),
    R("вед", "вед вод", "lead", "води'ть to lead or drive, перево'д translation (leading across), вы'вод conclusion."),
    R("ста", "ста стан сто стой став", "stand, become", "стать to become, стоя'ть to stand, оста'вить to leave behind, состоя'ние state."),
    R("сыл", "сыл сл сол", "send", "посла'ть to send, посо'л ambassador, посы'лка parcel."),
    R("пад", "пад па", "fall", "па'дать to fall, упа'л fell."),
    R("луч", "луч", "obtain; happen", "получи'ть to receive, слу'чай a case or chance, случи'ться to happen."),
    R("служ", "служ слуг", "serve", "служи'ть to serve, заслу'живать to deserve, слуга' servant."),
    R("пас", "пас", "save; guard", "спасти' to save, спаси'бо thank you (спаси' Бог, God save you)."),
    R("жал", "жал", "pity; favour", "жаль a pity, пожа'ловать to grant, пожа'луйста please (пожа'луй + old polite -ста)."),
    R("греть", "гре", "warm", "греть to warm, согре'ться to get warm."),
    R("гор", "гор гар", "burn", "горе'ть to burn, горя'чий hot, зага'р suntan."),
    R("ду", "ду", "blow", "дуть to blow."),
    R("ча", "ча чин нач", "begin", "нача'ть to begin, нача'ло a beginning."),
    R("двиг", "двиг движ", "move", "дви'гать to move, движе'ние movement."),
    # people
    R("мам", "мам", "mom"), R("пап", "пап", "dad"),
    R("мат", "мат матер", "mother", "мать, ма'тери; cognate with Latin mater."),
    R("отц", "отц отец отеч", "father", "оте'ц father, оте'чество fatherland, о'тчество patronymic."),
    R("брат", "брат", "brother"), R("сестр", "сестр", "sister"), R("сын", "сын", "son"),
    R("дет", "дет дит", "child", "де'ти children, де'тский children's."),
    R("друг", "друг друж", "friend; other", "друг friend, друго'й other, дру'жба friendship."),
    R("челов", "челов", "human being"),
    R("люд", "люд", "people"),
    R("жен", "жен", "woman, wife", "же'нщина woman, жена' wife."),
    R("муж", "муж", "man, husband"),
    R("дев", "дев", "girl", "де'вочка little girl, де'вушка young woman."),
    R("мал", "мал", "small", "ма'ленький little, ма'льчик boy, ма'ло little (few)."),
    R("стар", "стар", "old", "ста'рый old, стари'к old man, стару'ха old woman."),
    R("нов", "нов", "new"),
    R("молод", "молод млад", "young"),
    R("хозя", "хозя", "master, host", "хозя'ин master of the house."),
    # the world
    R("мир", "мир", "world; peace"),
    R("свет", "свет", "light; world"),
    R("звезд", "звезд звёзд", "star"),
    R("лун", "лун", "moon"),
    R("земл", "земл зем", "earth, land"),
    R("вод", "вод", "water", "вода' water, подво'дный underwater; not во'дить, which is lead."),
    R("мор", "мор", "sea"),
    R("лёд", "лёд льд", "ice"),
    R("дух", "дух дыш душ", "breath, spirit", "дух spirit, во'здух air, душа' soul, дыша'ть to breathe."),
    R("ветр", "ветр ветер", "wind"),
    R("пар", "пар", "steam"),
    R("тепл", "тепл тёпл", "warm"),
    R("ночь", "ночь ноч", "night"),
    R("день", "день дн дня", "day", "день day, днём by day, сего'дня today (of this day)."),
    R("утр", "утр втр", "morning", "у'тро morning, за'втра tomorrow (after the morning), за'втрак breakfast."),
    R("вечер", "вечер вчер", "evening", "ве'чер evening, вчера' yesterday (of the evening)."),
    R("врем", "врем времен", "time"),
    R("год", "год", "year; fit", "год year, пого'да weather, го'дный fit."),
    R("лето", "лет", "summer; years", "ле'то summer, пять лет five years. Spelled like лет- fly, a different root."),
    R("час", "час", "hour; time", "час hour, сейча'с now (this hour)."),
    R("дорог", "дорог дорож", "road"),
    R("город", "город град", "city", "го'род city, Волгогра'д, гра'ждане citizens."),
    R("дом", "дом", "house, home"),
    R("мест", "мест мещ", "place", "ме'сто place, вме'сте together (in one place)."),
    R("сторон", "сторон стран", "side; country", "сторона' side, страна' country, стра'нный strange (from the other side)."),
    R("ин", "ин", "other", "ино'й other, иногда' sometimes, ина'че otherwise."),
    R("кон", "кон", "end", "коне'ц end, коне'чный finite, бесконе'чный infinite, зако'н law, наконе'ц at last."),
    R("перед", "перед пред прежд", "front, before"),
    R("зад", "зад", "back, behind"),
    R("кол", "кол", "wheel, circle", "о'коло around, колесо' wheel, кольцо' ring."),
    R("круг", "круг круж", "circle"),
    R("угол", "угол угл", "corner, angle", "у'гол angle, треуго'льник triangle."),
    R("верх", "верх верш", "top", "верх top, верши'на summit or vertex."),
    R("част", "част", "part", "часть part, части'ца particle, сча'стье happiness (a good share)."),
    R("ряд", "ряд", "row, order", "ряд row or series, поря'док order."),
    R("равн", "равн ровн", "equal, even", "ра'вный equal, уравне'ние equation, ро'вно exactly."),
    R("мног", "мног множ", "many", "мно'го many, мно'жество a set, умно'жить to multiply."),
    R("цел", "цел", "whole", "це'лый whole, це'лое число' an integer."),
    R("прост", "прост", "simple", "просто'й simple, просто'е число' a prime."),
    R("слож", "слож", "put together, complex"),
    R("точ", "точ тк", "point", "то'чка a point (from ткнуть, to poke), то'чно exactly."),
    R("раз", "раз", "strike; time, once", "раз once, о'браз image (what is cut out), ра'зный different."),
    # qualities
    R("бол", "бол больш", "big", "большо'й big, бо'льше more."),
    R("добр", "добр", "good, kind"),
    R("хорош", "хорош", "good"),
    R("плох", "плох", "bad"),
    R("крас", "крас", "beautiful; red", "краси'вый beautiful, кра'сный red."),
    R("мил", "мил", "dear, nice"),
    R("тих", "тих", "quiet"),
    R("шум", "шум", "noise"),
    R("сил", "сил", "strength"),
    R("вкус", "вкус кус", "taste; bite", "вкус taste, вку'сно tasty, куса'ть to bite, кусо'к a piece."),
    R("мудр", "мудр", "wise"),
    R("медл", "медл", "slow"),
    R("скор", "скор", "fast", "ско'ро soon, ско'рость speed."),
    R("долг", "долг", "long (time)", "до'лго for a long time, до'лгий long."),
    R("долж", "долж долг", "debt; must", "до'лжен must (is in debt), долг a debt."),
    R("лёг", "лёг легк", "light, easy", "лёгкий light or easy, легко' easily."),
    R("толст", "толст", "thick, fat"),
    R("тонк", "тонк", "thin"),
    R("дал", "дал", "far", "далеко' far, даль the distance."),
    R("перв", "перв", "first"),
    R("нуж", "нуж нужд", "need"),
    R("готов", "готов", "ready"),
    # things
    R("книг", "книг книж", "book"),
    R("слов", "слов", "word", "сло'во word, слова'рь dictionary."),
    R("хлеб", "хлеб", "bread", "An early Germanic loan, cognate with English loaf."),
    R("молок", "молок млек", "milk", "Мле'чный путь is the Milky Way."),
    R("сол", "сол", "salt", "Cognate with salt and Latin sal."),
    R("сыр", "сыр", "cheese"), R("сок", "сок", "juice"), R("чай", "чай", "tea"), R("каш", "каш", "porridge"),
    R("стол", "стол", "table", "стол table, столо'вая canteen, столя'р joiner."),
    R("ок", "ок оч", "eye", "о'ко eye (poetic), окно' window (a little eye), очки' glasses."),
    R("кот", "кот кош", "cat", "кот tomcat, ко'шка cat, котёнок kitten."),
    R("собак", "собак собач", "dog"),
    R("рыб", "рыб", "fish"),
    R("рук", "рук руч", "hand, arm", "рука' hand, ру'копись manuscript (hand-writing), ру'чка pen."),
    R("ног", "ног нож", "foot, leg"),
    R("глаз", "глаз", "eye"),
    R("голов", "голов глав", "head", "голова' head, гла'вный main, глава' chapter."),
    R("лиц", "лиц лич", "face; person", "лицо' face, ли'чный personal."),
    R("желез", "желез", "iron"),
    R("черв", "черв", "worm"),
    R("шап", "шап", "cap"),
    R("им2", "им имен", "name", "и'мя name, и'менно namely, by name."),
    R("один", "один одн един", "one", "оди'н one, оди'ннадцать eleven, еди'ный single, united."),
    R("дв", "дв два две дво", "two"),
    R("тр", "тр три тре", "three"),
    R("четыр", "четыр четвер", "four"),
    R("пят", "пят", "five"), R("шест", "шест", "six"), R("сем", "сем седьм", "seven"),
    R("восем", "восем восьм", "eight"), R("девят", "девят девя", "nine"),
    R("десят", "десят дцать", "ten", "-дцать in оди'ннадцать … два'дцать is десять worn down: one-on-ten."),
    R("сто", "сто сот", "hundred"),
    R("сорок", "сорок", "forty (a bundle of forty furs)"),
    R("рус", "рус", "Russian"),
    R("англ", "англ", "English"),
    R("вс", "вс весь", "all"),
    R("сам", "сам", "self"),
    R("рот", "рот", "mouth"), R("нос", "нос", "nose"),
    R("жук", "жук", "beetle"), R("лук", "лук", "onion; bow"),
    R("стих", "стих", "verse"),
    R("конь", "конь кон", "horse"),
    R("волк", "волк", "wolf"),
    R("корабл", "корабл", "ship"),
    R("дав", "дав", "long ago"),
    R("син", "син", "blue"),
    R("жд", "жд жид", "wait"),
    R("гул", "гул", "stroll"),
]
ROOT_IDS = {r["id"] for r in ROOTS}

# --- prefixes --------------------------------------------------------------------------------
P = lambda id, forms, gloss, examples: dict(id=id, forms=forms.split(), gloss=gloss, examples=examples.split())
PREFIXES = [
    P("в", "в во въ", "in, into", "вход вме'сте"),
    P("вы", "вы", "out", "вы'ход вы'вод"),
    P("до", "до", "up to, to the end", "доказа'ть дочита'ть"),
    P("за", "за", "begin; behind, beyond", "за'втра зада'ча зако'н"),
    P("из", "из ис", "out of", "изме'рить иссле'дование"),
    P("на", "на", "onto; done (perfective)", "написа'ть нача'ть наконе'ц"),
    P("над", "над", "above, over", "надпи'сь"),
    P("не", "не", "not", "неде'ля непра'вда нечётный"),
    P("о", "о об обо объ", "around, about", "о'коло обе'д о'браз объе'кт"),
    P("от", "от ото", "away from; back", "отве'т отвеча'ть отде'л"),
    P("пере", "пере", "across; again, re-", "перево'д перехо'д переписа'ть"),
    P("по", "по", "a little; begin; done", "поня'ть пойти' пото'м"),
    P("под", "под", "under; approach", "подво'дный подойти'"),
    P("пред", "пред пре", "before, pre-; very", "предста'вить преде'л прекра'сный"),
    P("при", "при", "arrive, attach", "приве'т приходи'ть приня'ть"),
    P("про", "про", "through, past; to the end", "прочита'ть пройти'"),
    P("раз", "раз рас", "apart; un-", "ра'зум разме'р расска'з"),
    P("с", "с со съ", "together; down; done", "сде'лать сказа'ть сове'т съесть"),
    P("у", "у", "away; completely", "узна'ть умно'жить уйти'"),
    P("без", "без бес", "without", "бесконе'чный"),
    P("воз", "воз вз вс", "up", "во'здух взять"),
    P("между", "между меж", "between, inter-", "междунаро'дный"),
    P("после", "после", "after", "после'дний"),
    P("полу", "полу", "half, semi-", "полуо'стров"),
    P("само", "само", "self", "самолёт"),
    P("ни", "ни", "not any", "никогда' ничего'"),
]
PREFIX_IDS = {p["id"] for p in PREFIXES}
PREFIX_FORMS = {f: p["id"] for p in PREFIXES for f in p["forms"]}

# --- suffixes --------------------------------------------------------------------------------
S = lambda id, forms, gloss, pos, examples: dict(id=id, forms=forms.split(), gloss=gloss, pos=pos, examples=examples.split())
SUFFIXES = [
    S("тель", "тель", "one who does", "noun", "учи'тель писа'тель чита'тель"),
    S("ник", "ник", "person or thing connected with", "noun", "учени'к рабо'тник спу'тник"),
    S("ость", "ость", "a quality (-ness)", "noun", "му'дрость справедли'вость ско'рость"),
    S("ние", "ние ение ение", "a process or its result (-ing, -tion)", "noun", "чте'ние движе'ние значе'ние"),
    S("ство", "ство тельство", "a state, a collective (-hood, -ship)", "noun", "доказа'тельство госуда'рство существо'"),
    S("ка", "ка", "a small one; a feminine or a tool", "noun", "ча'шка ло'жка ры'бка"),
    S("ик", "ик чик", "a small one", "noun", "до'мик ко'тик ма'льчик"),
    S("ист", "ист", "a practitioner (-ist)", "noun", "артиллери'ст реали'ст"),
    S("изм", "изм", "a doctrine (-ism)", "noun", "реали'зм механи'зм"),
    S("ец", "ец", "a person or thing; an end", "noun", "коне'ц учени'ец"),
    S("н", "н", "makes an adjective", "adj", "чётный стра'нный вку'сный"),
    S("ск", "ск", "relating to (-ish, -ic)", "adj", "ру'сский де'тский"),
    S("тельн", "тельн", "makes an adjective from -тель", "adj", "сле'довательно"),
    S("лив", "лив", "inclined to", "adj", "справедли'вый"),
    S("о", "о", "makes an adverb", "adv", "хорошо' ти'хо вку'сно"),
    S("ова", "ова ева", "makes a verb", "verb", "существова'ть тре'бовать"),
    S("ива", "ива ыва", "makes the ongoing (imperfective) verb", "verb", "спра'шивать заслу'живать"),
    S("ть", "ть ти", "the infinitive", "verb", "чита'ть идти'"),
    S("ся", "ся", "oneself; passive", "verb", "каза'ться дели'ться"),
]
SUFFIX_IDS = {x["id"] for x in SUFFIXES}
SUFFIX_FORMS = {f: x["id"] for x in SUFFIXES for f in x["forms"]}

# --- lemmas ----------------------------------------------------------------------------------
LEMMAS = []


def M(seg, pos, en, family=None, base=None, example=None, note=None, loan=None):
    """A lemma by its segmentation. `base` is the word it is derived from (a derived-from edge);
    `example` is a sentence (ru, en) that shows it, for family_guess; `loan` names the source
    language of a borrowing."""
    LEMMAS.append({k: v for k, v in dict(seg=seg, pos=pos, en=en, family=family, base=base, example=example, note=note, loan=loan).items() if v is not None})


# pronouns, particles, conjunctions, prepositions: no family
for w, pos, en in [
    ("а", "conj", "and, but"), ("и", "conj", "and"), ("но", "conj", "but"), ("да", "part", "yes"), ("не", "part", "not"),
    ("нет", "part", "no; there is not"), ("ни", "part", "not any"), ("ли", "part", "whether"), ("же", "part", "the very (emphasis)"),
    ("бы", "part", "would"), ("ну", "part", "well"), ("уж", "part", "really (emphasis)"), ("ведь", "part", "after all"),
    ("бу'дто", "conj", "as if"), ("хотя'", "conj", "although"), ("е'сли", "conj", "if"), ("что'бы", "conj", "so that"),
    ("чтоб", "conj", "so that"), ("и'ли", "conj", "or"), ("лишь", "part", "only"), ("столь", "adv", "so (much)"),
    ("в", "prep", "in, into"), ("на", "prep", "on, onto"), ("с", "prep", "with; off"), ("у", "prep", "at, by"),
    ("к", "prep", "towards"), ("о", "prep", "about"), ("об", "prep", "about"), ("по", "prep", "along, by"),
    ("до", "prep", "up to"), ("для", "prep", "for"), ("из", "prep", "out of"), ("от", "prep", "from"),
    ("за", "prep", "behind, for"), ("под", "prep", "under"), ("над", "prep", "above"), ("при", "prep", "by, at"),
    ("без", "prep", "without"), ("че'рез", "prep", "through, across"), 
    ("ме'жду", "prep", "between"), ("по'дле", "prep", "beside"), ("о'коло", "prep", "around, near"),
    ("я", "pron", "I"), ("ты", "pron", "you (one)"), ("он", "pron", "he"), ("она'", "pron", "she"), ("оно'", "pron", "it"),
    ("мы", "pron", "we"), ("вы", "pron", "you (plural)"), ("они'", "pron", "they"), ("себя'", "pron", "oneself"),
    ("мой", "pron", "my"), ("твой", "pron", "your"), ("наш", "pron", "our"), ("ваш", "pron", "your (plural)"), ("свой", "pron", "one's own"),
    ("кто", "pron", "who"), ("что", "pron", "what; that"), ("кто'-то", "pron", "someone"), ("что'-то", "pron", "something"),
    ("никто'", "pron", "nobody"), ("ничто'", "pron", "nothing"),
    ("э'то", "pron", "this (is)"), ("э'тот", "pron", "this"), ("тот", "pron", "that"), ("тако'й", "pron", "such"),
    ("како'й", "pron", "what kind of"), ("ка'ждый", "pron", "each"), ("весь", "pron", "all"),
    ("где", "adv", "where"), ("куда'", "adv", "where to"), ("когда'", "adv", "when"),
    ("тогда'", "adv", "then"), ("всегда'", "adv", "always"), ("никогда'", "adv", "never"), ("иногда'", "adv", "sometimes"),
    ("как", "adv", "how"), ("так", "adv", "so"), ("там", "adv", "there"), ("тут", "adv", "here"), ("здесь", "adv", "here"),
    ("сюда'", "adv", "to here"), ("пото'м", "adv", "then, afterwards"), ("потому'", "adv", "therefore"), ("почему'", "adv", "why"),
    ("ско'лько", "adv", "how many"), ("не'сколько", "adv", "several"), ("то'лько", "adv", "only"), ("о'чень", "adv", "very"),
    ("ещё", "adv", "still, more"), ("уже'", "adv", "already"), ("тепе'рь", "adv", "now"), ("пока'", "adv", "for now; bye"),
    ("вот", "part", "here is"), ("вдруг", "adv", "suddenly"), ("да'же", "part", "even"), ("на'до", "pred", "one must"),
    ("нельзя'", "pred", "one may not"), ("мо'жет", "part", "maybe"), ("то", "conj", "then"), ("то'же", "adv", "also"),
    ("по'сле", "prep", "after"), ("сего'дня", "adv", "today"),
    ("спаси'бо", "part", "thank you"), ("пожа'луйста", "part", "please"), ("приве'т", "noun", "hi"), ("мисс", "noun", "Miss"),
]:
    M(w, pos, en)

# Some of the function words above do have a visible root; override them with a segmentation.
LEMMAS[:] = [l for l in LEMMAS if l["seg"] not in ("сего'дня", "спаси'бо", "пожа'луйста", "приве'т", "о'коло", "весь", "никогда'",
                                                   "иногда'", "всегда'", "ничто'", "никто'", "не'сколько", "ме'жду", "по'дле")]
M("сего'|[дня]", "adv", "today", note="Of this day: сего (old genitive of этот) + дня.")
M("с|[пас]|и'|бо", "part", "thank you", note="Спаси Бог, God save you, worn down to one word.")
M("по|[жа'л]|уй|ста", "part", "please", note="Пожалуй (grant it) + the old polite particle -ста.")
M("при|[ве'т]", "noun", "hi, greeting", base="вет")
M("о'|[кол]|о", "prep", "around, near", note="Round the wheel.")
M("[весь]", "pron", "all", family="вс")
M("ни|когда'", "adv", "never")
M("[ин]|огда'", "adv", "sometimes", note="At other times.")
M("[вс]|егда'", "adv", "always")
M("ни|что'", "pron", "nothing")
M("ни|кто'", "pron", "nobody")
M("не'|с|колько", "adv", "several")
M("ме'жду", "prep", "between", note="From межа, a boundary strip.")
M("по'дле", "prep", "beside", note="From длина, length: along.")

# verbs
M("[зна]|ть", "verb", "to know", example=("Я зна'ю, что э'то факт.", "I know that this is a fact."))
M("у|[зна']|ть", "verb", "to find out", base="знать", example=("Сна'чала ну'жно узна'ть, что тако'е му'дрость.", "First one must find out what wisdom is."))
M("[зна'ч]|и|ть", "verb", "to mean", base="знак", example=("Что зна'чит э'то сло'во?", "What does this word mean?"))
M("[знак]", "noun", "sign", base="знать", example=("Плюс — э'то знак.", "Plus is a sign."))
M("[знач]|е'|ние", "noun", "meaning", base="значить", example=("Ка'ждое сло'во име'ет значе'ние.", "Every word has a meaning."))
M("[зна']|ние", "noun", "knowledge", base="знать", example=("Зна'ние — си'ла.", "Knowledge is power."))
M("[ду'м]|а|ть", "verb", "to think")
M("по|[ду'м]|а|ть", "verb", "to think for a moment", base="думать", example=("Он поду'мал и отве'тил.", "He thought for a moment and answered."))
M("[мы'сл]|и|ть", "verb", "to reason, to think")
M("[мысл]|ь", "noun", "thought")
M("[ум]", "noun", "mind")
M("[у'м]|н|ый", "adj", "clever", base="ум", example=("Ро'бот о'чень у'мный.", "The robot is very clever."))
M("ра'з|[ум]", "noun", "reason (the faculty)", base="ум", example=("Ра'зум и чу'вство.", "Reason and feeling."))
M("[ве'р]|и|ть", "verb", "to believe")
M("[ве'р]|н|ый", "adj", "faithful; correct", base="верить", example=("Э'то ве'рный отве'т.", "This is the correct answer."))
M("на|[ве'р]|н|ое", "adv", "probably", base="верный")
M("[пра'в]|д|а", "noun", "truth")
M("не|[пра'в]|д|а", "noun", "untruth, a lie", base="правда")
M("[пра'в]|ильн|о", "adv", "correctly", base="правда", example=("Ты всё сде'лал пра'вильно.", "You did everything correctly."))
M("[пра'в]|о", "noun", "a right; law", base="правда", example=("У меня' есть пра'во знать.", "I have the right to know."))
M("с|[прав]|ед|ли'в|ый", "adj", "just, fair", base="правда", note="Dictionaries treat справедлив- as one root today; the link to пра'вда is plain.")
M("с|[прав]|ед|ли'в|ость", "noun", "justice", base="справедливый")
M("[и'стин]|а", "noun", "truth (as a principle)", example=("Матема'тика и'щет и'стину.", "Mathematics seeks truth."))
M("[говор]|и'|ть", "verb", "to speak")
M("с|[каз]|а'|ть", "verb", "to say (once)", example=("Он сказа'л, что не зна'ет.", "He said that he doesn't know."))
M("по|[каз]|а'|ть", "verb", "to show", base="сказать", example=("Покажи' мне кни'гу.", "Show me the book."))
M("до|[каз]|а'|ть", "verb", "to prove")
M("до|[каз]|а'|тельств|о", "noun", "proof", base="доказать")
M("рас|с|[ка'з]", "noun", "a story", base="сказать", example=("Э'то расска'з Че'хова.", "This is a story by Chekhov."))
M("[каз]|а'|ть|ся", "verb", "to seem")
M("с|[прос]|и'|ть", "verb", "to ask (once)")
M("с|[пра'ш]|ива|ть", "verb", "to ask (repeatedly)", base="спросить")
M("во|[про'с]", "noun", "question", base="спросить", example=("У меня' есть вопро'с.", "I have a question."))
M("от|[ве'т]", "noun", "answer")
M("от|[веч]|а'|ть", "verb", "to answer", base="ответ")
M("со|[ве'т]", "noun", "advice; council", base="ответ", example=("Спаси'бо за сове'т.", "Thanks for the advice."))
M("[молч]|а'|ть", "verb", "to be silent")
M("[слы'ш]|а|ть", "verb", "to hear")
M("[слу'ш]|а|ть", "verb", "to listen", base="слышать")
M("[слух]", "noun", "hearing; a rumour", base="слышать")
M("[ви'д]|е|ть", "verb", "to see")
M("у|[ви'д]|е|ть", "verb", "to catch sight of", base="видеть", example=("Я уви'дел звезду'.", "I caught sight of a star."))
M("[вид]", "noun", "view; kind", base="видеть", example=("Из окна' краси'вый вид.", "There is a beautiful view from the window."))
M("[смотр]|е'|ть", "verb", "to look")
M("[чит]|а'|ть", "verb", "to read")
M("про|[чит]|а'|ть", "verb", "to read to the end", base="читать")
M("[чит]|а'|тель", "noun", "reader", base="читать", example=("Писа'тель и чита'тель.", "The writer and the reader."))
M("[чт]|е'|ние", "noun", "reading", base="читать", example=("Чте'ние — моё люби'мое де'ло.", "Reading is my favourite thing."))
M("с|[чит]|а'|ть", "verb", "to count; to consider", base="читать", example=("Ро'бот счита'ет звёзды.", "The robot is counting the stars."))
M("[чёт]|н|ый", "adj", "even (number)", base="считать")
M("не|[чёт]|н|ый", "adj", "odd (number)", base="чётный")
M("[чис]|л|о'", "noun", "number", base="считать", family="чит")
M("[пис]|а'|ть", "verb", "to write")
M("на|[пис]|а'|ть", "verb", "to write (and finish)", base="писать", example=("Она' написа'ла письмо'.", "She wrote a letter."))
M("пере|[пис]|а'|ть", "verb", "to rewrite, to copy out", base="писать", example=("Переписа'й э'то сло'во.", "Copy out this word."))
M("за|[пис]|а'|ть", "verb", "to write down, to record", base="писать", example=("Запиши' мой отве'т.", "Write down my answer."))
M("[пис]|а'|тель", "noun", "writer", base="писать", example=("Толсто'й — писа'тель.", "Tolstoy is a writer."))
M("[пис]|ьм|о'", "noun", "a letter (mail)", base="писать", example=("Э'то письмо' от бра'та.", "This is a letter from my brother."))
M("[ру'к]|о|[пис]|ь", "noun", "manuscript", base="писать", family="пис")
M("за'|[пис]|ь", "noun", "a record, a recording", base="записать", example=("Э'то за'пись уро'ка.", "This is a recording of the lesson."))
M("[уч]|и'|ть", "verb", "to teach; to learn")
M("[уч]|и'|тель", "noun", "teacher", base="учить", example=("Учи'тель чита'ет.", "The teacher is reading."))
M("[уч]|ени'к", "noun", "pupil", base="учить")
M("на|[у'к]|а", "noun", "science", base="учить", example=("Матема'тика — нау'ка.", "Mathematics is a science."))
M("[уч]|ёные", "noun", "scientists, scholars", base="учить", example=("Учёные ду'мают.", "The scientists are thinking."))
M("из|[ме'р]|и|ть", "verb", "to measure")
M("[ме'р]|а", "noun", "measure", base="измерить", example=("Всё хорошо' в ме'ру.", "Everything is good in measure."))
M("раз|[ме'р]", "noun", "size", base="мера", example=("Како'й разме'р?", "What size?"))
M("при|[ме'р]", "noun", "example", base="мера", example=("Вот приме'р.", "Here is an example."))
M("[сле'д]|ова|тельн|о", "adv", "therefore", base="следовать")
M("[сле'д]|ова|ть", "verb", "to follow", example=("Из э'того сле'дует, что два — чётное число'.", "From this it follows that two is even."))
M("[сле'д]|ующ|ий", "adj", "next, following", base="следовать", example=("Сле'дующий вопро'с!", "Next question!"))
M("[след]", "noun", "a track, a trace", example=("На снегу' след ко'та.", "On the snow, a cat's track."))
M("по|н|[им]|а'|ть", "verb", "to understand (be understanding)")
M("по|[ня']|ть", "verb", "to understand, to grasp", base="понимать")
M("[им]|е'|ть", "verb", "to have", example=("Ка'ждое число' име'ет знак.", "Every number has a sign."))
M("при|[ня']|ть", "verb", "to accept", base="понять", example=("Я принима'ю твой отве'т.", "I accept your answer."))
M("вз|[я]|ть", "verb", "to take", example=("Я взял кни'гу со стола'.", "I took the book from the table."))
M("по'|[мн]|и|ть", "verb", "to remember", example=("Я по'мню э'то сло'во.", "I remember this word."))
M("па'|[мя]|ть", "noun", "memory", base="помнить", example=("У ро'бота хоро'шая па'мять.", "The robot has a good memory."))
M("[мн]|е'|ние", "noun", "opinion", base="помнить", example=("Э'то моё мне'ние.", "This is my opinion."))
M("[тре'б]|ова|ть|ся", "verb", "to be required")
M("[сущ]|еств|ова'|ть", "verb", "to exist")
M("[су'щ]|н|ость", "noun", "essence", base="существовать", example=("В чём су'щность доказа'тельства?", "What is the essence of the proof?"))
M("[сут]|ь", "noun", "the gist, the essence", family="сущ")
M("[бы]|ть", "verb", "to be")
M("[бы]|ва'|ть", "verb", "to happen, to be (sometimes)", base="быть")
M("есть", "verb", "is, there is; to eat", note="Two words spelled alike: the surviving present of быть, and the verb to eat (root ес-/ед-: еда', обе'д, съесть).")
M("съ|[е'с]|ть", "verb", "to eat up", base="есть")
M("[ед]|а'", "noun", "food", base="есть", example=("Еда' на столе'.", "The food is on the table."))
M("об|[е'д]", "noun", "lunch", base="есть")
M("[пи]|ть", "verb", "to drink")
M("[сп]|а|ть", "verb", "to sleep")
M("[сон]", "noun", "sleep; a dream", family="сп", example=("Э'то был сон.", "It was a dream."))
M("с|[поко'й]|н|ый", "adj", "calm, quiet")
M("[поко'й]", "noun", "peace, rest", example=("Мне ну'жен поко'й.", "I need peace and quiet."))
M("[жи]|ть", "verb", "to live")
M("[жи]|знь", "noun", "life", base="жить", example=("Жизнь коротка'.", "Life is short."))
M("[жи]|в|о'й", "adj", "alive, living", base="жить", example=("Кот живо'й!", "The cat is alive!"))
M("[люб]|и'|ть", "verb", "to love")
M("[люб]|о'й", "pron", "any", base="любить")
M("[хот]|е'|ть", "verb", "to want")
M("[мо]|чь", "verb", "to be able", family="мог")
M("[мо'ж]|н|о", "pred", "one may", base="мочь")
M("по|[мо']|чь", "verb", "to help", base="мочь", family="мог", example=("Ты мо'жешь мне помо'чь?", "Can you help me?"))
M("[раб]|о'т|а", "noun", "work")
M("[раб]|о'т|а|ть", "verb", "to work", base="работа")
M("[раб]|о'т|ник", "noun", "worker", base="работа", example=("Он хоро'ший рабо'тник.", "He is a good worker."))
M("[ро'б]|от", "noun", "robot", base="работа", loan="Czech robota, forced labour (Čapek, 1920)")
M("[де'л]|а|ть", "verb", "to do")
M("с|[де'л]|а|ть", "verb", "to do (and finish)", base="делать")
M("[де'л]|о", "noun", "a matter, business", base="делать")
M("не|[де'л]|я", "noun", "week", base="дело", note="Once the day of not-doing (Sunday), then the week it begins.")
M("[дел]|и'|ть|ся", "verb", "to be divisible; to share", family="дели")
M("[дел]|и'|ть", "verb", "to divide", family="дели", example=("Де'сять де'лим на два.", "We divide ten by two."))
M("[дел]|е'|ние", "noun", "division", base="делить", family="дели", example=("Деле'ние — э'то не умноже'ние.", "Division is not multiplication."))
M("пре|[де'л]", "noun", "limit", base="делить", family="дели", example=("У э'той фу'нкции есть преде'л.", "This function has a limit."))
M("о|пре|[дел]|е'|ние", "noun", "definition", base="предел", family="дели", example=("Определе'ние: чётное число' де'лится на два.", "Definition: an even number is divisible by two."))
M("от|[де'л]", "noun", "department, section", base="делить", family="дели")
M("[да]|ть", "verb", "to give")
M("за|[да']|ч|а", "noun", "task, problem", base="дать")
M("пере|[да']|ть", "verb", "to pass, to hand over", base="дать")
M("про|[да']|ть", "verb", "to sell", base="дать", example=("Он про'дал дом.", "He sold the house."))
M("[ход]|и'|ть", "verb", "to go (regularly, on foot)")
M("[ид]|ти'", "verb", "to be going (on foot)", family="ход")
M("при|[ход]|и'|ть", "verb", "to come (regularly)", base="ходить")
M("при|[й]|ти'", "verb", "to arrive", base="идти", family="ход")
M("по|[й]|ти'", "verb", "to set off", base="идти", family="ход")
M("на|[й]|ти'", "verb", "to find", base="идти", family="ход", example=("Я не могу' найти' кни'гу.", "I can't find the book."))
M("[ход]", "noun", "a move; motion", base="ходить", example=("Твой ход!", "Your move!"))
M("в|[хо'д]", "noun", "entrance", base="ходить", example=("Где вход?", "Where is the entrance?"))
M("вы'|[ход]", "noun", "exit; a way out", base="ходить", example=("Вы'ход там.", "The exit is over there."))
M("пере|[хо'д]", "noun", "crossing; transition", base="ходить", example=("Перехо'д о'чень до'лгий.", "The crossing is very long."))
M("[е']|х|а|ть", "verb", "to be going (by transport)", family="ех", note="Here the root is е-, with -х- a stem suffix; е'здить shows the fuller езд-.")
M("[е'зд]|и|ть", "verb", "to travel (regularly)", base="ехать")
M("по'|[езд]", "noun", "train", base="ездить", example=("По'езд идёт в го'род.", "The train goes to the city."))
M("[лет]|е'|ть", "verb", "to fly")
M("[само]|[лёт]", "noun", "aeroplane", base="лететь", family="лет", note="самолёт: self-flier; лёт is лет- with the stress-shifted ё.")
M("по|[лёт]", "noun", "flight", base="лететь", family="лет", example=("Полёт на Луну'.", "A flight to the Moon."))
M("[бе'г]|а|ть", "verb", "to run (about)")
M("по|[беж]|а'|ть", "verb", "to run off", base="бегать")
M("[нес]|ти'", "verb", "to carry")
M("[нос]|и'|ть", "verb", "to wear; to carry about", base="нести")
M("[вод]|и'|ть", "verb", "to lead; to drive", family="вед")
M("пере|[во'д]", "noun", "translation", base="водить", family="вед", example=("Э'то перево'д с ру'сского.", "This is a translation from Russian."))
M("вы'|[вод]", "noun", "conclusion; output", base="водить", family="вед", example=("Вы'вод: су'мма чётная.", "Conclusion: the sum is even."))
M("[ста]|ть", "verb", "to become; to begin")
M("[сто]|я'|ть", "verb", "to stand", family="ста")
M("о|[ста']|в|и|ть", "verb", "to leave behind", base="стать", family="ста")
M("со|[сто]|я'|ние", "noun", "state, condition", base="стоять", family="ста", example=("Состоя'ние систе'мы.", "The state of the system."))
M("по|[сыл]|а'|ть", "verb", "to send (repeatedly)")
M("по|[сл]|а'|ть", "verb", "to send", base="посылать", family="сыл")
M("по|[со'л]", "noun", "ambassador (one sent)", base="послать", family="сыл")
M("[па'д]|а|ть", "verb", "to fall")
M("у|[па']|сть", "verb", "to fall down", base="падать", family="пад")
M("по|[луч]|а'|ть", "verb", "to receive")
M("по|[луч]|и'|ть", "verb", "to receive (once)", base="получать")
M("с|[лу'ч]|а|й", "noun", "a case; chance", base="получить")
M("с|[луч]|и'|ть|ся", "verb", "to happen", base="случай")
M("[служ]|и'|ть", "verb", "to serve")
M("за|[слу'ж]|ива|ть", "verb", "to deserve", base="служить")
M("с|[пас]|ти'", "verb", "to save, to rescue", example=("Соба'ка спасла' ребёнка.", "The dog saved the child."))
M("со|[гре']|ть|ся", "verb", "to get warm", family="греть")
M("[гор]|е'|ть", "verb", "to burn")
M("[гор]|я'ч|ий", "adj", "hot", base="гореть", example=("Чай горя'чий.", "The tea is hot."))
M("[ду]|ть", "verb", "to blow")
M("на|[ча']|ть", "verb", "to begin")
M("на|[ча']|л|о", "noun", "a beginning", base="начать", example=("Э'то то'лько нача'ло.", "This is only the beginning."))
M("[дви'г]|а|ть", "verb", "to move")
M("[движ]|е'|ние", "noun", "movement, motion", base="двигать")
M("[жд]|а|ть", "verb", "to wait")
M("[гул]|я'|ть", "verb", "to go for a walk")

# people and living things
M("[ма'м]|а", "noun", "mom"); M("[па'п]|а", "noun", "dad")
M("[мат]|ь", "noun", "mother"); M("[бра'т]", "noun", "brother"); M("[сестр]|а'", "noun", "sister"); M("[сын]", "noun", "son")
M("[оте'ц]", "noun", "father")
M("[де'т]|и", "noun", "children")
M("[друг]", "noun", "friend")
M("[друг]|о'й", "pron", "other", base="друг", note="The other one is, at root, the friend.")
M("[челов]|е'к", "noun", "person, human being")
M("[лю'д]|и", "noun", "people")
M("[же'н]|щин|а", "noun", "woman")
M("[жен]|а'", "noun", "wife", base="женщина", example=("Э'то моя' жена'.", "This is my wife."))
M("[де'в]|очк|а", "noun", "little girl")
M("[ма'л]|ьч|ик", "noun", "boy", family="мал")
M("[ма'л]|еньк|ий", "adj", "little", example=("Ма'ленький кот спит.", "The little cat is asleep."))
M("[ста'р]|ый", "adj", "old")
M("[стар]|и'к", "noun", "old man", base="старый")
M("[стар]|у'х|а", "noun", "old woman", base="старый")
M("[но'в]|ый", "adj", "new")
M("[хозя']|ин", "noun", "master, owner, host")
M("[кот]", "noun", "cat"); M("[кот]|ёнок", "noun", "kitten", base="кот")
M("[ко'ш]|к|а", "noun", "cat (she-cat)", base="кот", example=("Ко'шка спит на окне'.", "The cat is sleeping on the window."))
M("[соба'к]|а", "noun", "dog"); M("[соба'ч]|к|а", "noun", "little dog", base="собака")
M("[ры'б]|а", "noun", "fish"); M("[ры'б]|к|а", "noun", "little fish", base="рыба")
M("[жук]", "noun", "beetle"); M("[волк]", "noun", "wolf"); M("[конь]", "noun", "horse"); M("мураве'й", "noun", "ant")
M("[черв]|я'к", "noun", "worm"); M("лев", "noun", "lion")
# body
M("[рот]", "noun", "mouth"); M("[нос]", "noun", "nose", family="нос"); M("[рук]|а'", "noun", "hand, arm"); M("[ног]|а'", "noun", "foot, leg")
M("[глаз]", "noun", "eye"); M("[голов]|а'", "noun", "head"); M("[гла'в]|н|ый", "adj", "main", base="голова", example=("Э'то гла'вный вопро'с.", "This is the main question."))
M("[лиц]|о'", "noun", "face; person"); M("[ли'ч]|н|ый", "adj", "personal", base="лицо", example=("Э'то ли'чный вопро'с.", "This is a personal question."))
# the world
M("[мир]", "noun", "world; peace"); M("[свет]", "noun", "light; the world")
M("[звезд]|а'", "noun", "star"); M("[лун]|а'", "noun", "moon"); M("[земл]|я'", "noun", "earth, land")
M("[вод]|а'", "noun", "water", family="вод"); M("под|[во'д]|н|ый", "adj", "underwater", base="вода", family="вод", example=("Подво'дная ло'дка.", "A submarine (underwater boat)."))
M("[мо'р]|е", "noun", "sea"); M("[лёд]", "noun", "ice")
M("во'з|[дух]", "noun", "air", base="дух", example=("Во'здух здесь чи'стый.", "The air here is clean."))
M("[дух]", "noun", "spirit"); M("[душ]|а'", "noun", "soul", base="дух", example=("Душа' и те'ло.", "Soul and body."))
M("[ве'тер]", "noun", "wind", family="ветр"); M("[пар]", "noun", "steam")
M("[тёпл]|ый", "adj", "warm", family="тепл"); M("[тепл]|е'е", "adv", "warmer", base="тёплый")
M("[ночь]", "noun", "night"); M("[день]", "noun", "day")
M("[у'тр]|о", "noun", "morning"); M("за'|[втр]|а", "adv", "tomorrow", base="утро"); M("за'|[втр]|ак", "noun", "breakfast", base="завтра")
M("[ве'чер]", "noun", "evening"); M("[вчер]|а'", "adv", "yesterday", base="вечер")
M("[вре'м]|я", "noun", "time"); M("[год]", "noun", "year"); M("[ле'т]|о", "noun", "summer; (plural) years", family="лето")
M("[час]", "noun", "hour"); M("сей|[ча'с]", "adv", "now, right away", base="час", example=("Сейча'с я чита'ю.", "Right now I am reading."))
M("[доро'г]|а", "noun", "road")
M("[го'род]", "noun", "city"); M("[дом]", "noun", "house, home"); M("[дом]|о'й", "adv", "homewards", base="дом")
M("[до'лг]|о", "adv", "for a long time"); M("[до'лж]|ен", "adj", "must, obliged"); M("[долг]", "noun", "debt", family="долж", example=("Долг плати'ть ну'жно.", "A debt must be paid."))
M("в|[перед]|и'", "adv", "ahead"); M("[пре'жд]|е", "adv", "before, formerly"); M("[пе'ред]", "prep", "in front of, before")
M("[сам]", "pron", "oneself"); M("[са'м]|ый", "pron", "the very, most", base="сам")
M("[муж]", "noun", "husband"); M("[муж]|чи'н|а", "noun", "man", base="муж", example=("Э'тот мужчи'на — мой брат.", "This man is my brother."))
M("[молод]|о'й", "adj", "young", example=("Молодо'й челове'к чита'ет.", "The young man is reading."))
M("[мла'д]|ш|ий", "adj", "younger", base="молодой", family="молод", example=("Э'то мой мла'дший брат.", "This is my younger brother."))
M("кото'р|ый", "pron", "which, that"); M("пора'", "noun", "time, season (с тех пор: since then)")
M("[ме'ст]|о", "noun", "place"); M("в|[ме'ст]|е", "adv", "together", base="место")
M("[сторон]|а'", "noun", "side"); M("[стран]|а'", "noun", "country", base="сторона", example=("Росси'я — больша'я страна'.", "Russia is a big country."))
M("[стра'н]|н|ый", "adj", "strange", base="сторона")
M("[ин]|о'й", "pron", "other, different")
M("[кон]|е'ц", "noun", "end"); M("на|[кон]|е'ц", "adv", "at last", base="конец"); M("[кон]|е'ч|н|ый", "adj", "finite; final", base="конец", example=("Мно'жество коне'чное.", "The set is finite."))
M("бес|[кон]|е'ч|н|ый", "adj", "infinite", base="конечный", example=("Просты'х чи'сел бесконе'чно мно'го.", "There are infinitely many primes."))
M("за|[ко'н]", "noun", "law", base="конец", example=("Э'то зако'н приро'ды.", "This is a law of nature."))
M("на|[за'д]", "adv", "back, ago")
M("[круг]", "noun", "circle"); M("во|[кру'г]", "prep", "around", base="круг"); M("о|[кру'ж]|н|ость", "noun", "circumference", base="круг", example=("Дли'на окру'жности.", "The length of the circumference."))
M("[у'гол]", "noun", "corner, angle"); M("тре|[уго'л]|ьник", "noun", "triangle", base="угол", example=("У треуго'льника три угла'.", "A triangle has three angles."))
M("[верх]", "noun", "top"); M("[верш]|и'н|а", "noun", "summit; vertex", base="верх", example=("У треуго'льника три верши'ны.", "A triangle has three vertices."))
M("[част]|ь", "noun", "part"); M("[част]|и'ц|а", "noun", "particle", base="часть", example=("Электро'н — части'ца.", "The electron is a particle."))
M("с|[ча'ст]|ье", "noun", "happiness", base="часть", note="A good share.")
M("[ряд]", "noun", "row; series"); M("по|[ря'д]|ок", "noun", "order", base="ряд", example=("Всё в поря'дке.", "Everything is in order."))
M("[ра'вн]|о", "adv", "equals; equally"); M("[ра'вн]|ый", "adj", "equal", base="равно"); M("у|[равн]|е'|ние", "noun", "equation", base="равный", example=("Реши' уравне'ние.", "Solve the equation."))
M("[мно'г]|о", "adv", "many, much"); M("не|[мно'г]|о", "adv", "a little", base="много")
M("[мно'ж]|еств|о", "noun", "a set; a multitude", base="много", example=("Мно'жество чётных чи'сел.", "The set of even numbers."))
M("у|[мно'ж]|и|ть", "verb", "to multiply", base="много", example=("Умно'жим два на три.", "Let's multiply two by three."))
M("у|[мно'ж]|енн|ый", "adj", "multiplied", base="умножить")
M("[це'л]|ый", "adj", "whole"); M("[прост]|о'й", "adj", "simple; prime"); M("[про'ст]|о", "adv", "simply", base="простой")
M("[сло'ж]|н|ый", "adj", "complex, difficult", example=("Э'то сло'жная зада'ча.", "This is a hard problem."))
M("[то'ч]|к|а", "noun", "point, dot"); M("[то'ч]|н|о", "adv", "exactly", base="точка", example=("То'чно так.", "Exactly so."))
M("[раз]", "noun", "a time, once"); M("о'б|[раз]", "noun", "image, form", base="раз")
M("[больш]|о'й", "adj", "big", family="бол"); M("[бо'л]|ее", "adv", "more", family="бол"); M("[бо'льш]|е", "adv", "more, bigger", family="бол")
M("[до'бр]|ый", "adj", "good, kind"); M("[добр]|о'", "noun", "good (the good)", base="добрый", example=("Добро' и зло.", "Good and evil."))
M("[хорош]|о'", "adv", "well; good"); M("[хоро'ш]|ий", "adj", "good", base="хорошо")
M("[плох]|о'й", "adj", "bad"); M("[крас]|и'в|ый", "adj", "beautiful"); M("[кра'с]|н|ый", "adj", "red")
M("[ми'л]|ый", "adj", "dear, nice"); M("[ти'х]|о", "adv", "quietly"); M("ча'ст|о", "adv", "often", note="From ча'стый, dense, frequent; its kinship with часть is disputed."); M("[шум]", "noun", "noise"); M("[си'л]|а", "noun", "strength, force")
M("[си'л]|ьн|ый", "adj", "strong", base="сила", example=("Си'льный ве'тер.", "A strong wind."))
M("[вку'с]|н|о", "adv", "tasty"); M("[вкус]", "noun", "taste", base="вкусно"); M("[му'др]|ый", "adj", "wise"); M("[му'др]|ость", "noun", "wisdom", base="мудрый")
M("[ме'дл]|енн|о", "adv", "slowly"); M("по|[ме'дл]|енн|ее", "adv", "a bit slower", base="медленно"); M("[ско'р]|о", "adv", "soon"); M("[ско'р]|ость", "noun", "speed", base="скоро", example=("Ско'рость све'та.", "The speed of light."))
M("[легк]|о'", "adv", "easily", family="лёг"); M("[лёг]|к|ий", "adj", "light; easy", base="легко")
M("[то'лст]|ый", "adj", "fat, thick"); M("[то'нк]|ий", "adj", "thin"); M("[дал]|еко'", "adv", "far"); M("[пе'рв]|ый", "adj", "first")
M("[ну'ж]|н|о", "pred", "one must, it is needed"); M("[ну'ж]|н|ый", "adj", "needed", base="нужно"); M("[гото'в]", "adj", "ready"); M("[дав]|но'", "adv", "long ago"); M("[си'н]|ий", "adj", "blue")
# things
M("[кни'г]|а", "noun", "book"); M("[сло'в]|о", "noun", "word"); M("[слов]|а'рь", "noun", "dictionary", base="слово", example=("Посмотри' в словаре'.", "Look in the dictionary."))
M("[хлеб]", "noun", "bread", loan="Germanic *hlaibaz, cognate with loaf"); M("[молок]|о'", "noun", "milk"); M("[сол]|ь", "noun", "salt", family="сол"); M("[сыр]", "noun", "cheese"); M("[сок]", "noun", "juice")
M("[чай]", "noun", "tea", loan="Chinese chá via Turkic"); M("[ка'ш]|а", "noun", "porridge"); M("[стол]", "noun", "table"); M("[стол]|я'р", "noun", "joiner", base="стол")
M("[ок]|н|о'", "noun", "window", note="A little eye (о'ко) for the house."); M("[оч]|к|и'", "noun", "glasses", base="окно", family="ок", example=("Где мои' очки'?", "Where are my glasses?"))
M("[желе'з]|о", "noun", "iron"); M("[ша'п]|к|а", "noun", "cap, hat", loan="Old French chape via German, cognate with cap")
M("[и'м]|я", "noun", "name", family="им2"); M("[и'м]|енн|о", "adv", "namely, exactly", base="имя", family="им2")
# numbers
M("[оди'н]", "num", "one"); M("[оди'н]|надцать", "num", "eleven", base="один"); M("[еди'н]|ый", "adj", "single, united", family="один")
M("[два]", "num", "two"); M("[две]|на'дцать", "num", "twelve", base="два", family="дв"); M("[два']|дцать", "num", "twenty", base="два", family="дв"); M("[дв]|а'жды", "adv", "twice", base="два")
M("[три]", "num", "three"); M("[три]|на'дцать", "num", "thirteen", base="три", family="тр"); M("[три']|дцать", "num", "thirty", base="три", family="тр")
M("[четы'р]|е", "num", "four"); M("[четы'р]|надцать", "num", "fourteen", base="четыре")
M("[пят]|ь", "num", "five"); M("[пят]|на'дцать", "num", "fifteen", base="пять"); M("[пят]|ь|деся'т", "num", "fifty", base="пять")
M("[шест]|ь", "num", "six"); M("[шест]|на'дцать", "num", "sixteen", base="шесть"); M("[шест]|ь|деся'т", "num", "sixty", base="шесть")
M("[сем]|ь", "num", "seven"); M("[сем]|на'дцать", "num", "seventeen", base="семь"); M("[се'м]|ь|десят", "num", "seventy", base="семь")
M("[во'сем]|ь", "num", "eight", note="Once семь with a prefix, by analogy with other numbers."); M("[восем]|на'дцать", "num", "eighteen", base="восемь"); M("[во'сем]|ь|десят", "num", "eighty", base="восемь")
M("[де'вят]|ь", "num", "nine"); M("[девят]|на'дцать", "num", "nineteen", base="девять"); M("[девя]|но'сто", "num", "ninety", base="девять", family="девят")
M("[де'сят]|ь", "num", "ten"); M("[сто]", "num", "a hundred", family="сто"); M("[со'рок]", "num", "forty")
M("[ру'с]|ск|ий", "adj", "Russian"); M("по-|[ру'с]|ск|и", "adv", "in Russian", base="русский")
M("[англ]|и'й|ск|ий", "adj", "English"); M("по-|[англ]|и'й|ск|и", "adv", "in English", base="английский")
M("[стих]", "noun", "a verse", loan="Greek στίχος stíkhos, a row"); M("[кора'бл]|ь", "noun", "ship")
# borrowings from Greek and Latin that the course already uses (also in COGNATES)
for seg, pos, en, loan in [
    ("ко'смос", "noun", "space, cosmos", "Greek κόσμος kósmos, order, world"), ("плане'та", "noun", "planet", "Greek πλανήτης planḕtēs, wanderer"),
    ("раке'та", "noun", "rocket", "Italian rocchetta via German Rakete"), ("ста'нция", "noun", "station", "Latin statio via Polish stacja"),
    ("сигна'л", "noun", "signal", "Latin signale via German"), ("факт", "noun", "fact", "Latin factum, a thing done"),
    ("объе'кт", "noun", "object", "Latin objectum, thrown before"), ("тео'рема", "noun", "theorem", "Greek θεώρημα theṑrēma, a thing looked at"),
    ("су'мма", "noun", "sum", "Latin summa, the highest (line)"), ("плюс", "noun", "plus", "Latin plus, more"),
    ("цирк", "noun", "circus", "Latin circus, a ring"), ("норма'льно", "adv", "fine, normally", "Latin norma, a carpenter's square"),
    ("аппети'т", "noun", "appetite", "Latin appetitus"), ("ко'фе", "noun", "coffee", "Arabic qahwa via Dutch koffie"),
    ("парк", "noun", "park", "English park / French parc"), ("суп", "noun", "soup", "French soupe"),
    ("торт", "noun", "cake", "Italian torta via German Torte"), ("стул", "noun", "chair", "German Stuhl, cognate with stool"),
    ("таре'лка", "noun", "plate", "German Teller, with Russian -ка"), ("ку'хня", "noun", "kitchen", "German Küche, from Latin coquina"),
    ("са'хар", "noun", "sugar", "Greek σάκχαρον sákkharon"), ("маши'на", "noun", "machine", "French machine, from Greek μηχανή"),
    ("шко'ла", "noun", "school", "Latin schola, from Greek σχολή skholḕ, leisure"), ("магни'т", "noun", "magnet", "Greek μαγνῆτις"),
    ("мину'та", "noun", "minute", "Latin minuta, small"), ("ко'мната", "noun", "room", "Latin caminata, a room with a hearth"),
    ("артиллери'ст", "noun", "artilleryman", "French artillerie + -ист"), ("люк", "noun", "hatch", "Dutch luik"),
    ("Росси'я", "noun", "Russia", None), ("[лук]", "noun", "onion", None), ("щи", "noun", "cabbage soup", None), ("у'лица", "noun", "street", None),
    ("у'жин", "noun", "dinner, supper", None), ("у'жин|а|ть", "verb", "to have dinner", None), ("ло'жка", "noun", "spoon", None), ("ча'шка", "noun", "cup", None),
    ("прия'тн|ый", "adj", "pleasant", None), ("опя'ть", "adv", "again", None), ("ка'жется", "verb", "it seems", None), ("э'тот", "pron", "this", None),
    ("госуда'рство", "noun", "the state", None), ("ша'пка", "noun", "cap", None), ("не'вод", "noun", "fishing net", None), ("ми'лый", "adj", "dear", None),
    ("спроси'ть", "verb", "to ask", None), ("уви'деть", "verb", "to see", None), ("слу'чай", "noun", "case", None),
]:
    if any(l["seg"].replace("|", "").replace("[", "").replace("]", "") == seg.replace("|", "") for l in LEMMAS):
        continue
    M(seg, pos, en, loan=loan)
LEMMAS[:] = [l for l in LEMMAS if l is not None]
# notes for the opaque ones
for l in LEMMAS:
    h = l["seg"].replace("|", "").replace("[", "").replace("]", "")
    if h == "прия'тный":
        l["note"] = "From прия'ть, to receive kindly (cf. прия'тель, a friend); dictionaries treat приятн- as one root today."
    if h == "у'жин":
        l["note"] = "From юг, south: the meal at midday, when the sun is in the south; now the evening meal."
    if h == "у'жинать":
        l["base"] = "ужин"
    if h == "опя'ть":
        l["note"] = "From пята', the heel: turning on one's heel, back again."
    if h == "у'лица":
        l["note"] = "From an old word for a hollow or a passage, not from лицо."
    if h == "госуда'рство":
        l["note"] = "From госуда'рь, sovereign, itself worn down from господа'рь, master (cf. Го'сподь)."
    if h == "ло'жка":
        l["note"] = "A tool noun in -ка; the stem is obscure."
    if h == "ча'шка":
        l["note"] = "ча'ша, a bowl, + -ка."

# --- inflected or variant forms that the course and library use, mapped to a headword -------
FORMS = {}
for lemma, forms in [
    ("аппети'т", "аппетита"), ("де'ло", "дела"), ("молоко'", "молока молоком"), ("са'хар", "сахаром"), ("хлеб", "хлеба"), ("соль", "соли"),
    ("сыр", "сыра"), ("чай", "чая"), ("ум", "умом"), ("мир", "мира"), ("кот", "коты котенка"), ("окно'", "окна окне"), ("ло'жка", "ложки"),
    ("ча'шка", "чашки"), ("таре'лка", "тарелки"), ("стол", "столы столе"), ("но'вый", "новая новое новые"), ("умно'жить", "умноженное"),
    ("прочита'ть", "прочитал прочитала прочитаю"), ("сде'лать", "сделали"), ("ру'сский", "русски"), ("англи'йский", "английски"),
    ("тако'й", "такое такие таким"), ("кото'рый", "которое которую который"), ("ме'дленно", "помедленнее"), ("гото'в", "готов"),
    ("быть", "будет будем будешь буду был была были было есть бывает"), ("весь", "всё все всех всем"), ("мочь", "могу может мог"),
    ("идти'", "иду идёшь идём идёте идут шла пошел пошли"), ("прийти'", "пришел"), ("хоте'ть", "хочу хочешь хотел"), ("стать", "стал стала стали стало станет"),
    ("сказа'ть", "сказал сказала"), ("говори'ть", "говорит говорил"), ("ду'мать", "думал подумал"), ("знать", "знаю"), ("ви'деть", "видит видел"),
    ("они'", "их им них ними ним"), ("он", "его ему него нему"), ("она'", "её ее ей нее ней"), ("мы", "нас нам нами"), ("вы", "вас вам"), ("я", "меня мне"),
    ("ты", "тебя тебе"), ("себя'", "себе собой"), ("э'тот", "этого этом этой эту эти эта того тому том тех"), ("оди'н", "одну одной одного одно"),
    ("мой", "моей мои моя"), ("свой", "своей свою свое своего"), ("сам", "самого самое"), ("что", "чего чем"), ("тот", "того тому том тех"),
    ("вода'", "воду воды водой воде"), ("мо'ре", "моря"), ("дом", "доме дома домой"), ("друг", "друга"), ("сторона'", "стороны сторону"),
    ("соба'ка", "собаки собак"), ("соба'чка", "собачку"), ("ры'бка", "рыбку"), ("глаз", "глаза"), ("жизнь", "жизни"), ("движе'ние", "движения"),
    ("год", "лет года"), ("ко'шка", "кошки"), ("пора'", "пор"), ("де'вочка", "девочка"), ("шко'ла", "школу"), ("у'читель", "учитель"), ("воду", "воду"),
    ("согре'ться", "согреется"), ("червя'к", "червяков"), ("де'ти", "дети"), ("доро'га", "дорогу"), ("побежа'ть", "побежала"), ("вре'мя", "время"),
    ("земля'", "землю земле земли"), ("ста'ло", "стало"), ("ко'мната", "комнаты"), ("си'ла", "силы"), ("о'браз", "образом"), ("госуда'рство", "государства"),
    ("еди'ный", "единого"),     ("ста'рый", "старик старуха"), ("ещё", "еще"), ("в", "во"), ("с", "со"), ("лёд", "лед"), ("ничто'", "ничего"),
    ("стра'нный", "странно"), ("си'ний", "синее"), ("до'лжен", "должны"), ("ча'сто", "часто"), ("мальчи'к", "мальчик"), ("оте'ц", "отцу"), ("сын", "сын"), ("голова'", "голову"),
    ("взять", "взял"), ("рука'", "руки руку"), ("коне'ц", "конца"), ("конь", "коня"), ("челове'к", "человека человеку людей"),
    ("лицо'", "лицо"), ("же'нщина", "женщина"), ("стол", "столяр"), ("упа'сть", "упал"), ("ша'пка", "шапку"), ("мину'та", "минуту"),
    ("два", "две"), ("найти'", "нашел"), ("уви'деть", "увидали"), ("сло'во", "слово"), ("корабль", "корабль"),
    ("у'лица", "улице"), ("оста'вить", "оставил"), ("хозя'ин", "хозяина"), ("нача'ть", "начал"), ("спроси'ть", "спросил"), ("случи'ться", "случилось"),
    ("нога'", "ноги"), ("ду'ть", "дует"), ("магни'т", "магнит"), ("ру'копись", "рукописи"), ("горе'ть", "горят"), ("Росси'я", "россию"),
    ("справедли'вый", "справедливым"), ("му'дрый", "мудрым"), ("кни'га", "книги книге книгу"), ("су'мма", "сумму"),
    ("для", "для"), ("число'", "числа чисел"), ("чётный", "чётное чётных чётная"), ("нечётный", "нечётное"), 
    ("просто'й", "простые простое"), ("большо'й", "большее"), ("ка'ждый", "каждое"), ("любо'й", "любого"), ("це'лый", "целое целых"),
    ("существова'ть", "существует существую"), ("тре'боваться", "требовалось"), ("доказа'ть", "доказать"), ("дели'ться", "делится"),
    ("Земля'", "землю"), ("звезда'", "звёзды звезду"), ("раке'та", "ракету"), ("люби'ть", "люблю любит"), ("ста'нция", "станции"),
    ("пла'нета", "планете"), ("ко'смос", "космосе"), ("ку'хня", "кухне"), ("го'род", "городе город"), ("мысль", "мыслю"),
    ("сле'довательно", "следовательно"), ("спать", "спал"), ("у'жинать", "ужинали"), ("де'лать", "делал делали"), ("рабо'тать", "работал работала работаю работает"),
    ("гуля'ть", "гулял гуляли"), ("чита'ть", "читал читала читаю читает читаешь читаем читаете читают"), ("пи'сать", "пишет"), ("ждать", "ждёт"),
    ("молча'ть", "молчит"), ("слы'шать", "слышит"), ("слу'шать", "слушает"), ("рабо'та", "работу"), ("сигна'л", "сигнале"), ("у'тро", "утром"),
    ("ве'чер", "вечером"), ("день", "днём"), ("ночь", "ночью"), ("ходи'ть", "хожу ходит ходила"), ("е'хать", "еду едем едешь ехала"),
    ("парк", "парке"), ("зна'чить", "значит"), ("отве'т", "ответ"), ("неде'ля", "неделя"), ("но'вый", "новый"), ("ста'ть", "стать"),
    ("Ми'ра", "мира"), ("хоро'ший", "хорошая"), ("мать", "матери"), ("лев", "лев"),
]:
    head = lemma.replace("'", "").lower()
    for f in forms.split():
        FORMS.setdefault(f, head)

# --- the cognate bridge ----------------------------------------------------------------------
# (ru, en, source, rule ids, parts [(tile, gloss)] for build_word, note)
C = lambda ru, en, source, rules, parts=None, note=None: {k: v for k, v in dict(ru=ru, en=en, source=source, rules=rules.split(), parts=parts, note=note).items() if v is not None}
COGNATES = [
    C("тео'рия", "theory", "Greek θεωρία theōría, a looking at", "th-t ia-iya y-i", [("тео'р-", "look at, behold (θεωρ-)"), ("-ия", "-y: an activity or field (-ία)")]),
    C("теоре'ма", "theorem", "Greek θεώρημα theṑrēma, a thing looked at", "th-t ma-ma", [("теор-", "look at, behold (θεωρ-)"), ("-е'ма", "-em: a result (-ημα)")]),
    C("филосо'фия", "philosophy", "Greek φιλοσοφία philosophía, love of wisdom", "ph-f ia-iya", [("фило-", "love (φιλο-)"), ("-со'фия", "wisdom (σοφία)")]),
    C("фило'соф", "philosopher", "Greek φιλόσοφος", "ph-f os-drop", [("фило-", "love (φιλο-)"), ("-со'ф", "wise (σοφός)")]),
    C("ло'гика", "logic", "Greek λογική logikḕ, from λόγος, word, reason", "ics-ika", [("ло'г-", "word, reason (λόγος)"), ("-ика", "-ics: a science (-ική)")]),
    C("логи'ческий", "logical", "Greek λογικός", "ic-icheskiy", [("лог-", "word, reason (λόγος)"), ("-и'ческий", "-ical (-ικός + -ский)")]),
    C("систе'ма", "system", "Greek σύστημα sýstēma, a standing together", "y-i ma-ma", [("си-", "together (σύν-)"), ("-сте'ма", "standing (ἵστημι)")]),
    C("пробле'ма", "problem", "Greek πρόβλημα próblēma, a thing thrown forward", "ma-ma", [("про-", "forward (προ-)"), ("-бле'ма", "a throw (βλῆμα)")]),
    C("ана'лиз", "analysis", "Greek ἀνάλυσις análysis, a loosening up", "y-i is-drop", [("ана-", "up, apart (ἀνά-)"), ("-лиз", "loosen (λύσις)")]),
    C("си'нтез", "synthesis", "Greek σύνθεσις sýnthesis, a putting together", "y-i th-t is-drop", [("син-", "together (σύν-)"), ("-тез", "a placing (θέσις)")]),
    C("ме'тод", "method", "Greek μέθοδος méthodos, a way after", "th-t os-drop", [("мет-", "after (μετά-)"), ("-од", "way (ὁδός)")]),
    C("гипо'теза", "hypothesis", "Greek ὑπόθεσις hypóthesis, a placing under", "h-g y-i th-t", [("гипо-", "under (ὑπο-)"), ("-те'за", "a placing (θέσις)")]),
    C("аксио'ма", "axiom", "Greek ἀξίωμα axíōma, what is thought worthy", "x-ks ma-ma", [("акси-", "worthy (ἄξιος)"), ("-о'ма", "-om: a result (-ωμα)")]),
    C("иде'я", "idea", "Greek ἰδέα idéa, a form seen", "ia-iya"),
    C("матема'тика", "mathematics", "Greek μαθηματικά mathēmatiká, things learned", "th-t ics-ika", [("матема'т-", "learning (μάθημα)"), ("-ика", "-ics: a science (-ική)")]),
    C("геоме'трия", "geometry", "Greek γεωμετρία geōmetría, earth-measuring", "ia-iya g-g", [("гео-", "earth (γῆ)"), ("-ме'трия", "-metry: measuring (μετρία)")]),
    C("симме'трия", "symmetry", "Greek συμμετρία symmetría, measured together", "y-i ia-iya", [("сим-", "together (σύν-)"), ("-ме'трия", "-metry: measuring (μετρία)")]),
    C("арифме'тика", "arithmetic", "Greek ἀριθμητική arithmētikḕ, from ἀριθμός, number", "th-f ics-ika", None, "Greek θ became ф in the older, church-route borrowings: арифме'тика, миф, Фёдор."),
    C("а'лгебра", "algebra", "Arabic al-jabr, the restoring, via Latin", "none"),
    C("фи'зика", "physics", "Greek φυσική physikḕ, of nature", "ph-f y-i ics-ika", [("фи'з-", "nature (φύσις)"), ("-ика", "-ics: a science (-ική)")]),
    C("эне'ргия", "energy", "Greek ἐνέργεια enérgeia, at work", "e-e ia-iya", [("эн-", "in, at (ἐν-)"), ("-е'ргия", "work (ἔργον)")]),
    C("а'том", "atom", "Greek ἄτομος átomos, uncuttable", "os-drop", [("а-", "not (ἀ-)"), ("-том", "cut (τόμος)")]),
    C("микроско'п", "microscope", "Greek μικρός small + σκοπέω look", "os-drop", [("микро-", "small (μικρός)"), ("-ско'п", "look (σκοπέω)")]),
    C("телефо'н", "telephone", "Greek τῆλε far + φωνή voice", "ph-f", [("теле-", "far (τῆλε)"), ("-фо'н", "voice, sound (φωνή)")]),
    C("автома'т", "automaton", "Greek αὐτόματος autómatos, self-moving", "au-av", [("авто-", "self (αὐτός)"), ("-ма'т", "moving, thinking (-ματος)")]),
    C("изоморфи'зм", "isomorphism", "Greek ἴσος equal + μορφή form + -ism", "ism-izm ph-f", [("изо-", "equal (ἴσος)"), ("-морф-", "form (μορφή)"), ("-и'зм", "-ism (-ισμός)")]),
    C("гомоморфи'зм", "homomorphism", "Greek ὁμός same + μορφή form + -ism", "ism-izm ph-f h-g", [("гомо-", "same (ὁμός)"), ("-морф-", "form (μορφή)"), ("-и'зм", "-ism (-ισμός)")]),
    C("тополо'гия", "topology", "Greek τόπος place + λόγος", "ia-iya", [("топо-", "place (τόπος)"), ("-ло'гия", "-logy: study (λογία)")]),
    C("катего'рия", "category", "Greek κατηγορία katēgoría, a statement", "ia-iya h-g", [("кате-", "against, down (κατά-)"), ("-го'рия", "speaking in assembly (ἀγορά)")]),
    C("парадо'кс", "paradox", "Greek παράδοξος parádoxos, against expectation", "x-ks", [("пара-", "beside, against (παρά-)"), ("-до'кс", "opinion (δόξα)")]),
    C("пара'метр", "parameter", "Greek παρά beside + μέτρον measure", "os-drop", [("пара-", "beside (παρά-)"), ("-ме'тр", "measure (μέτρον)")]),
    C("хао'с", "chaos", "Greek χάος kháos", "ch-kh"),
    C("хара'ктер", "character", "Greek χαρακτήρ kharaktḕr, an engraved mark", "ch-kh"),
    C("те'хника", "technique, technology", "Greek τεχνικός tekhnikós, from τέχνη, craft", "ch-kh ics-ika"),
    C("механи'зм", "mechanism", "Greek μηχανή mēkhanḕ, device, + -ism", "ch-kh ism-izm", [("механ-", "device (μηχανή)"), ("-и'зм", "-ism (-ισμός)")]),
    C("ритм", "rhythm", "Greek ῥυθμός rhythmós", "th-t y-i os-drop"),
    C("цикл", "cycle", "Greek κύκλος kýklos, circle, via Latin cyclus", "c-ts y-i os-drop"),
    C("центр", "centre", "Greek κέντρον kéntron, a sharp point, via Latin centrum", "c-ts um-drop"),
    C("псих|оло'гия", "psychology", "Greek ψυχή psykhḕ, soul, + λόγος", "ps-ps ch-kh y-i ia-iya", [("психо-", "soul (ψυχή)"), ("-ло'гия", "-logy: study (λογία)")]),
    C("биоло'гия", "biology", "Greek βίος bíos, life, + λόγος", "ia-iya", [("био-", "life (βίος)"), ("-ло'гия", "-logy: study (λογία)")]),
    C("астроно'мия", "astronomy", "Greek ἄστρον star + νόμος law", "ia-iya", [("астро-", "star (ἄστρον)"), ("-но'мия", "-nomy: law, arrangement (νόμος)")]),
    C("демокра'тия", "democracy", "Greek δῆμος people + κράτος rule", "ia-iya", [("демо-", "people (δῆμος)"), ("-кра'тия", "-cracy: rule (κράτος)")]),
    C("иера'рхия", "hierarchy", "Greek ἱεραρχία, rule of the sacred", "h-drop ch-kh ia-iya", None, "Greek h is dropped here, as in исто'рия, not turned to г."),
    C("исто'рия", "history", "Greek ἱστορία historía, inquiry", "h-drop ia-iya"),
    C("гармо'ния", "harmony", "Greek ἁρμονία harmonía, a fitting together", "h-g ia-iya"),
    C("ми'стика", "mysticism", "Greek μυστικός mystikós, secret", "y-i ics-ika"),
    C("фу'нкция", "function", "Latin functio, a performing", "tion-tsiya", [("фу'нк-", "perform (fungi, functus)"), ("-ция", "-tion: an action (-tio)")]),
    C("структу'ра", "structure", "Latin structura, a building", "ure-ura", [("структ-", "build (struere, structus)"), ("-у'ра", "-ure: a result (-ura)")]),
    C("аргуме'нт", "argument", "Latin argumentum, proof, evidence", "ment-ment", [("аргу-", "make clear (arguere)"), ("-ме'нт", "-ment: a means (-mentum)")]),
    C("абстра'кция", "abstraction", "Latin abstractio, a drawing away", "tion-tsiya", [("абс-", "away (abs-)"), ("-тра'к-", "draw (trahere, tractus)"), ("-ция", "-tion: an action (-tio)")]),
    C("абстра'ктный", "abstract", "Latin abstractus, drawn away", "n-adj", [("абстра'кт-", "drawn away (abstractus)"), ("-ный", "makes an adjective")]),
    C("констру'кция", "construction", "Latin constructio, a putting together", "tion-tsiya", [("кон-", "together (con-)"), ("-стру'к-", "build (struere)"), ("-ция", "-tion: an action (-tio)")]),
    C("инду'кция", "induction", "Latin inductio, a leading in", "tion-tsiya", [("ин-", "in (in-)"), ("-ду'к-", "lead (ducere)"), ("-ция", "-tion: an action (-tio)")]),
    C("деду'кция", "deduction", "Latin deductio, a leading down", "tion-tsiya", [("де-", "down, from (de-)"), ("-ду'к-", "lead (ducere)"), ("-ция", "-tion: an action (-tio)")]),
    C("информа'ция", "information", "Latin informatio, a shaping", "tion-tsiya", [("ин-", "in (in-)"), ("-форма'-", "shape (forma)"), ("-ция", "-tion: an action (-tio)")]),
    C("фо'рмула", "formula", "Latin formula, a little form", "a-a"),
    C("фо'рма", "form", "Latin forma", "a-a"),
    C("элеме'нт", "element", "Latin elementum", "ment-ment e-e"),
    C("моме'нт", "moment", "Latin momentum, a movement", "ment-ment"),
    C("экспериме'нт", "experiment", "Latin experimentum, a trial", "ment-ment x-ks e-e"),
    C("докуме'нт", "document", "Latin documentum, a lesson, proof", "ment-ment"),
    C("инструме'нт", "instrument", "Latin instrumentum", "ment-ment"),
    C("ве'ктор", "vector", "Latin vector, a carrier", "or-or"),
    C("опера'тор", "operator", "Latin operator, a worker", "or-or"),
    C("фу'нктор", "functor", "Latin functor (modern coinage)", "or-or"),
    C("фа'ктор", "factor", "Latin factor, a maker", "or-or"),
    C("ма'трица", "matrix", "Latin matrix, womb, source", "x-ts", None, "Latin -ix/-ic- gives -ица: ма'трица."),
    C("ко'нус", "cone", "Greek κῶνος via Latin conus", "us-kept", None, "A few keep the Latin -us: ко'нус, ра'диус, фо'кус."),
    C("ра'диус", "radius", "Latin radius, a rod, a ray", "us-kept"),
    C("мо'дуль", "module, modulus", "Latin modulus, a small measure", "us-drop"),
    C("ска'ляр", "scalar", "Latin scalaris, of a ladder", "n-adj"),
    C("лине'йный", "linear", "Latin linea, a line, + Russian -н-ый", "n-adj", [("лине'й-", "line (linea)"), ("-ный", "makes an adjective")]),
    C("рациона'льный", "rational", "Latin rationalis, of reason", "al-alnyi", [("рацион-", "reckoning, reason (ratio)"), ("-а'льный", "-al (-alis + -н-ый)")]),
    C("реа'льный", "real", "Latin realis, of things", "al-alnyi", [("реа'-", "a thing (res)"), ("-льный", "-al (-alis + -н-ый)")]),
    C("форма'льный", "formal", "Latin formalis", "al-alnyi", [("форм-", "form (forma)"), ("-а'льный", "-al (-alis + -н-ый)")]),
    C("акти'вный", "active", "Latin activus", "ive-ivnyi", [("акт-", "do (agere, actus)"), ("-и'вный", "-ive (-ivus + -н-ый)")]),
    C("объекти'вный", "objective", "Latin objectivus", "ive-ivnyi", [("объект-", "thing thrown before (objectum)"), ("-и'вный", "-ive (-ivus + -н-ый)")]),
    C("реали'зм", "realism", "Latin realis + Greek -ism", "ism-izm", [("реал-", "a thing (res)"), ("-и'зм", "-ism (-ισμός)")]),
    C("идеали'зм", "idealism", "Greek ἰδέα + -ism", "ism-izm", [("идеал-", "idea, ideal (ἰδέα)"), ("-и'зм", "-ism (-ισμός)")]),
    C("материали'зм", "materialism", "Latin materia + -ism", "ism-izm"),
    C("эмпири'зм", "empiricism", "Greek ἐμπειρία empeiría, experience, + -ism", "ism-izm e-e"),
    C("скептици'зм", "scepticism", "Greek σκεπτικός, inquiring, + -ism", "ism-izm c-ts"),
    C("нигили'зм", "nihilism", "Latin nihil, nothing, + -ism", "ism-izm h-g"),
    C("реали'ст", "realist", "Latin realis + Greek -ist", "ist-ist"),
    C("специали'ст", "specialist", "Latin specialis + -ist", "ist-ist c-ts"),
    C("университе'т", "university", "Latin universitas via German Universität", "taet-tet"),
    C("факульте'т", "faculty", "Latin facultas via German Fakultät", "taet-tet"),
    C("профе'ссор", "professor", "Latin professor", "or-or"),
    C("студе'нт", "student", "Latin studens via German Student", "ent-ent"),
    C("ле'кция", "lecture", "Latin lectio, a reading", "tion-tsiya"),
    C("при'нцип", "principle", "Latin principium, a beginning", "c-ts um-drop"),
    C("конце'пция", "conception, concept", "Latin conceptio", "tion-tsiya c-ts"),
    C("дискре'тный", "discrete", "Latin discretus, separated", "n-adj", [("дискре'т-", "separated (discretus)"), ("-ный", "makes an adjective")]),
    C("ко'мплексный", "complex", "Latin complexus, embraced, woven together", "x-ks n-adj"),
    C("интегра'л", "integral", "Latin integralis, whole", "al-al"),
    C("дифференциа'л", "differential", "Latin differentia, difference", "al-al c-ts"),
    C("конста'нта", "constant", "Latin constans, standing firm", "a-a"),
    C("спектр", "spectrum", "Latin spectrum, an appearance", "um-drop"),
    C("квант", "quantum", "Latin quantum, how much", "qu-kv um-drop"),
    C("ква'драт", "square (quadrate)", "Latin quadratus", "qu-kv us-drop"),
    C("эквивале'нт", "equivalent", "Latin aequivalens", "qu-kv e-e ent-ent"),
    C("криста'лл", "crystal", "Greek κρύσταλλος krýstallos, ice", "y-i os-drop"),
    C("програ'мма", "programme", "Greek πρόγραμμα prógramma, a public notice", "ma-ma", [("про-", "forward, public (προ-)"), ("-гра'мма", "a writing (γράμμα)")]),
    C("грамма'тика", "grammar", "Greek γραμματική grammatikḕ, of letters", "ics-ika"),
    C("текст", "text", "Latin textus, woven", "x-ks us-drop"),
    C("конте'кст", "context", "Latin contextus, woven together", "x-ks us-drop", [("кон-", "together (con-)"), ("-те'кст", "woven (textus)")]),
    C("диало'г", "dialogue", "Greek διάλογος diálogos", "os-drop", [("диа-", "through, between (διά-)"), ("-ло'г", "word, speech (λόγος)")]),
    C("анало'гия", "analogy", "Greek ἀναλογία analogía, proportion", "ia-iya", [("ана-", "according to (ἀνά-)"), ("-ло'гия", "ratio, reasoning (λογία)")]),
    C("мета'фора", "metaphor", "Greek μεταφορά metaphorá, a carrying over", "ph-f a-a", [("мета-", "over, across (μετά-)"), ("-фо'ра", "a carrying (φορά)")]),
    C("метафи'зика", "metaphysics", "Greek τὰ μετὰ τὰ φυσικά, the books after the Physics", "ph-f y-i ics-ika", [("мета-", "after (μετά-)"), ("-фи'зика", "physics (φυσικά)")]),
    C("э'тика", "ethics", "Greek ἠθικά ēthiká, of character", "th-t ics-ika e-e"),
    C("эсте'тика", "aesthetics", "Greek αἰσθητικός, of perception", "th-t ics-ika e-e"),
    C("субста'нция", "substance", "Latin substantia, a standing under", "tion-tsiya", [("суб-", "under (sub-)"), ("-ста'н-", "stand (stare)"), ("-ция", "-tion (-tia)")]),
    C("субъе'кт", "subject", "Latin subjectum, thrown under", "um-drop", [("субъ-", "under (sub-)"), ("-е'кт", "thrown (jactus)")]),
    C("абсолю'т", "the absolute", "Latin absolutus, set free", "us-drop"),
    C("мате'рия", "matter", "Latin materia", "ia-iya"),
    C("нату'ра", "nature", "Latin natura", "ure-ura"),
    C("культу'ра", "culture", "Latin cultura, a tending", "ure-ura", [("культ-", "tend, worship (colere, cultus)"), ("-у'ра", "-ure: a result (-ura)")]),
    C("литерату'ра", "literature", "Latin litteratura, from littera, letter", "ure-ura"),
    C("архитекту'ра", "architecture", "Greek ἀρχιτέκτων chief builder, via Latin -ura", "ch-kh ure-ura"),
    C("интуи'ция", "intuition", "Latin intuitio, a looking at", "tion-tsiya"),
    C("ассоциа'ция", "association", "Latin associatio", "tion-tsiya c-ts"),
    C("револю'ция", "revolution", "Latin revolutio, a rolling back", "tion-tsiya"),
    C("компью'тер", "computer", "English computer, from Latin computare", "none"),
    C("алгори'тм", "algorithm", "Medieval Latin algorismus, from al-Khwārizmī", "th-t"),
    C("проце'сс", "process", "Latin processus, a going forward", "c-ts us-drop"),
    C("прое'кт", "project", "Latin projectum, thrown forward", "um-drop", [("про-", "forward (pro-)"), ("-е'кт", "thrown (jactus)")]),
    C("фено'мен", "phenomenon", "Greek φαινόμενον phainómenon, a thing appearing", "ph-f on-drop"),
    C("крите'рий", "criterion", "Greek κριτήριον kritḕrion, a means of judging", "on-iy"),
    C("анти|те'зис", "antithesis", "Greek ἀντίθεσις antíthesis", "th-t", [("анти-", "against (ἀντί-)"), ("-те'зис", "a placing (θέσις)")]),
    C("пери'од", "period", "Greek περίοδος períodos, a way round", "os-drop", [("пери-", "around (περί-)"), ("-о'д", "way (ὁδός)")]),
    C("сфе'ра", "sphere", "Greek σφαῖρα sphaîra, a ball", "ph-f a-a"),
    C("цили'ндр", "cylinder", "Greek κύλινδρος kýlindros, a roller", "c-ts y-i os-drop"),
    C("диагона'ль", "diagonal", "Greek διαγώνιος diagṑnios, from angle to angle", "al-al", [("диа-", "through, across (διά-)"), ("-гона'ль", "angle (γωνία)")]),
    C("паралле'льный", "parallel", "Greek παράλληλος parállēlos, beside one another", "n-adj", [("пара-", "beside (παρά-)"), ("-лле'ль-", "one another (ἀλλήλων)"), ("-ный", "makes an adjective")]),
    C("ортогона'льный", "orthogonal", "Greek ὀρθογώνιος orthogṑnios, right-angled", "th-t n-adj", [("орто-", "straight, right (ὀρθός)"), ("-гона'ль-", "angle (γωνία)"), ("-ный", "makes an adjective")]),
    C("мона'да", "monad", "Greek μονάς monás, a unit", "ad-ada", [("мон-", "one, alone (μόνος)"), ("-а'да", "-ad: a unit (-άς, -άδος)")]),
]
COGNATES = [c for c in COGNATES if c]
for c in COGNATES:
    c["ru"] = c["ru"].replace("|", "")

# Regular correspondences between the Greek/Latin (or English) shape and the Russian one.
RULE = lambda id, pattern, note, examples: dict(id=id, pattern=pattern, note=note, examples=examples.split())
RULES = [
    RULE("th-t", "th → т", "Greek θ is т in borrowings that came through Western Europe.", "тео'рия теоре'ма ме'тод гипо'теза ритм э'тика"),
    RULE("th-f", "th → ф", "Greek θ is ф in the older borrowings that came through Byzantium and the Church.", "арифме'тика миф Фёдор Афи'ны"),
    RULE("ph-f", "ph → ф", "Greek φ is always ф.", "филосо'фия фи'зика телефо'н мета'фора"),
    RULE("ch-kh", "ch → х", "Greek χ is х.", "хао'с хара'ктер те'хника механи'зм"),
    RULE("h-g", "h → г", "Greek and Latin h usually becomes г.", "гипо'теза гармо'ния гомоморфи'зм нигили'зм"),
    RULE("h-drop", "h → (nothing)", "Sometimes the h is simply dropped.", "исто'рия иера'рхия"),
    RULE("y-i", "y → и", "Greek υ (Latin y) is и.", "систе'ма фи'зика ана'лиз си'нтез цикл"),
    RULE("x-ks", "x → кс", "Latin x is spelled out as кс.", "парадо'кс текст ко'мплексный аксио'ма"),
    RULE("x-ts", "x → ц (in -ix)", "Latin -ix, stem -ic-, gives -ица.", "ма'трица"),
    RULE("c-ts", "c → ц before e, i, y", "Latin c before a front vowel is ц; before a, o, u it is к.", "центр цикл при'нцип проце'сс катего'рия культу'ра"),
    RULE("qu-kv", "qu → кв", "Latin qu is кв.", "квант ква'драт эквивале'нт"),
    RULE("au-av", "au → ав", "Greek αυ is ав.", "автома'т а'втор"),
    RULE("e-e", "e- → э-", "An initial e is spelled э, keeping it from being read as ye.", "эне'ргия э'тика экспериме'нт элеме'нт"),
    RULE("ps-ps", "ps → пс", "Greek ψ keeps both sounds.", "психоло'гия"),
    RULE("ia-iya", "-ia, -y → -ия", "Greek -ία and Latin -ia give -ия, feminine.", "филосо'фия тео'рия геоме'трия исто'рия"),
    RULE("ics-ika", "-ics, -ic → -ика", "A science in -ics is -ика, feminine singular.", "ло'гика фи'зика матема'тика э'тика"),
    RULE("ic-icheskiy", "-ic, -ical → -ический", "The adjective takes -ический.", "логи'ческий физи'ческий математи'ческий"),
    RULE("ma-ma", "-ma → -ма", "Greek -μα gives -ма, feminine in Russian though neuter in Greek.", "теоре'ма систе'ма пробле'ма аксио'ма програ'мма"),
    RULE("tion-tsiya", "-tion → -ция", "Latin -tio gives -ция.", "фу'нкция абстра'кция инду'кция информа'ция"),
    RULE("ure-ura", "-ure → -ура", "Latin -ura gives -ура.", "структу'ра культу'ра литерату'ра"),
    RULE("ment-ment", "-ment → -мент", "Latin -mentum gives -мент.", "аргуме'нт элеме'нт экспериме'нт"),
    RULE("ent-ent", "-ent → -ент", "Latin -ens, -entis gives -ент.", "студе'нт эквивале'нт"),
    RULE("or-or", "-or → -ор", "Latin -or stays -ор.", "ве'ктор опера'тор профе'ссор"),
    RULE("ism-izm", "-ism → -изм", "Greek -ισμός gives -изм.", "изоморфи'зм реали'зм механи'зм"),
    RULE("ist-ist", "-ist → -ист", "Greek -ιστής gives -ист.", "реали'ст специали'ст артиллери'ст"),
    RULE("al-alnyi", "-al → -альный", "Latin -alis takes the Russian adjective ending: -альный.", "рациона'льный форма'льный реа'льный"),
    RULE("al-al", "-al → -ал, -аль (noun)", "As a noun, -al stays -ал or -аль.", "интегра'л диагона'ль"),
    RULE("ive-ivnyi", "-ive → -ивный", "Latin -ivus takes -ивный.", "акти'вный объекти'вный"),
    RULE("n-adj", "adjective + -ный", "Any borrowed stem becomes an adjective with -н-ый.", "абстра'ктный дискре'тный лине'йный"),
    RULE("os-drop", "-os, -on → (nothing)", "Greek -ος, -ον and Latin -us, -um are usually dropped.", "а'том ме'тод ритм цикл спектр"),
    RULE("us-drop", "-us, -um → (nothing)", "Latin -us, -um are usually dropped.", "текст спектр квант мо'дуль"),
    RULE("us-kept", "-us → -ус", "A few Latin words keep -ус.", "ко'нус ра'диус фо'кус гло'бус"),
    RULE("um-drop", "-um → (nothing)", "Latin -um is dropped.", "центр субъе'кт объе'кт"),
    RULE("is-drop", "-is → (nothing)", "Greek -ις is dropped.", "ана'лиз си'нтез"),
    RULE("on-iy", "-ion → -ий", "Greek -ιον gives -ий.", "крите'рий"),
    RULE("on-drop", "-on → (nothing)", "Greek -ον is dropped.", "фено'мен"),
    RULE("ad-ada", "-ad → -ада", "Greek -άς, -άδος gives -ада.", "мона'да дека'да"),
    RULE("taet-tet", "-ty, German -tät → -тет", "Latin -tas came through German -tät as -тет.", "университе'т факульте'т"),
    RULE("a-a", "-a → -а", "Latin -a stays -а, feminine.", "фо'рмула фо'рма конста'нта"),
    RULE("g-g", "g → г", "Greek γ is г.", "геоме'трия ло'гика"),
    RULE("none", "(irregular)", "No regular correspondence; learned whole.", "а'лгебра компью'тер"),
]
RULE_IDS = {r["id"] for r in RULES}


def headword(seg):
    return seg.replace("|", "").replace("[", "").replace("]", "")


def build():
    lemmas = {}
    dups = []
    for l in LEMMAS:
        seg = l["seg"]
        head = headword(seg).replace("'", "").lower()
        segments = seg.split("|")
        roots = [x[1:-1] for x in segments if x.startswith("[")]
        root_ids = []
        for r in roots:
            plain = r.replace("'", "")
            fam = l.get("family")
            cand = [x["id"] for x in ROOTS if plain in x["forms"]]
            root_ids.append(fam if fam and (fam in cand or len(roots) > 1) else (cand[0] if cand else plain))
        family = l.get("family", root_ids[0] if root_ids else None)
        before, after = [], []
        seen_root = False
        for x in segments:
            if x.startswith("["):
                seen_root = True
            elif not seen_root:
                before.append(x.replace("'", ""))
            else:
                after.append(x.replace("'", ""))
        entry = {
            "ru": s(headword(seg)),
            "morphemes": [s(x.strip("[]")) for x in segments],
            "roots": root_ids,
            "prefixes": [p.replace("-", "") for p in before if p.replace("-", "") in PREFIX_FORMS or p.replace("-", "") in PREFIX_IDS],
            "suffixes": [x for x in after if x in SUFFIX_FORMS],
            "pos": l["pos"],
            "en": l["en"],
        }
        if family:
            entry["family"] = family
        for k in ("base", "note", "loan"):
            if k in l:
                entry[k] = s(l[k])
        if "example" in l:
            entry["example"] = {"ru": s(l["example"][0]), "en": l["example"][1]}
        if head in lemmas:
            dups.append(head)
        lemmas[head] = entry
    if dups:
        raise SystemExit(f"duplicate lemmas: {dups}")
    cognates = []
    for c in COGNATES:
        e = {"ru": s(c["ru"]), "en": c["en"], "source": c["source"], "rules": c["rules"]}
        if c.get("parts"):
            e["parts"] = [{"tile": s(t), "gloss": g} for t, g in c["parts"]]
        if c.get("note"):
            e["note"] = s(c["note"])
        cognates.append(e)
    return {
        "version": 1,
        "roots": [dict(r, label="/".join(f + "-" for f in r["forms"][:3]), note=s(r["note"])) if "note" in r
                  else dict(r, label="/".join(f + "-" for f in r["forms"][:3])) for r in ROOTS],
        "prefixes": [dict(p, examples=[s(x) for x in p["examples"]]) for p in PREFIXES],
        "suffixes": [dict(x, examples=[s(e) for e in x["examples"]]) for x in SUFFIXES],
        "lemmas": lemmas,
        "forms": FORMS,
        "cognates": cognates,
        "rules": [dict(r, examples=[s(x) for x in r["examples"]]) for r in RULES],
        "names": ["филипок", "лев", "вася", "очумелов", "каштанка", "мисс", "мэнни", "адамс", "митя", "ваня", "сократ", "мира", "т", "б"],
    }


if __name__ == "__main__":
    data = build()
    OUT.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    print(OUT, len(data["roots"]), "roots", len(data["prefixes"]), "prefixes", len(data["suffixes"]), "suffixes",
          len(data["lemmas"]), "lemmas", len(data["forms"]), "forms", len(data["cognates"]), "cognates", len(data["rules"]), "rules")
