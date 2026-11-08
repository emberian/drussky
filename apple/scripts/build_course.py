#!/usr/bin/env python3
"""Author source for Drussky's course.json.

Stress is written as an apostrophe after the stressed Cyrillic vowel (ма'ма) and
converted to U+0301 on output. Run:  python3 apple/scripts/build_course.py
"""
import hashlib
import json
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
OUT = Path(__file__).resolve().parents[1] / "Drussky/Resources/course.json"
ACUTE = re.compile(r"([аеиоуыэюяАЕИОУЫЭЮЯ])'")


def s(text):
    return ACUTE.sub(lambda m: m.group(1) + "́", text)


def deep(value):
    if isinstance(value, str):
        return s(value)
    if isinstance(value, list):
        return [deep(v) for v in value]
    if isinstance(value, dict):
        return {k: deep(v) for k, v in value.items()}
    return value


# --- exercise helpers -------------------------------------------------------
def meet(letter): return {"kind": "meet", "letter": letter}
def ex(text, examples=None):
    e = {"kind": "explain", "text": text}
    if examples:
        e["examples"] = [{"ru": ru, "en": en} for ru, en in examples]
    return e
def tr(text): return {"kind": "trace", "text": text}
def cp(text, times=2): return {"kind": "copy", "text": text, "times": times}
def lp(say, options): return {"kind": "listen_pick", "say": say, "options": options}
def dc(text): return {"kind": "dictation", "text": text}
def ra(text): return {"kind": "read_aloud", "text": text}
def tl(en, answer): return {"kind": "translate", "en": en, "answer": answer}
def fl(before, answer, after, hint): return {"kind": "fill", "before": before, "after": after, "hint": hint, "answer": answer}
def rm(text, options, answer=0):
    """options[answer] is the right meaning; it lands at a position fixed by a hash of the text,
    so the right answer is not always first and a rebuild does not move it."""
    right, others = options[answer], [o for i, o in enumerate(options) if i != answer]
    pos = int(hashlib.sha1(text.encode()).hexdigest(), 16) % len(options)
    return {"kind": "read_meaning", "text": text, "options": others[:pos] + [right] + others[pos:], "answer": pos}
def ps(text, en, glosses=None):
    e = {"kind": "passage", "text": text, "en": en}
    if glosses:
        e["glosses"] = glosses
    return e
def dy(prompt): return {"kind": "diary", "prompt": prompt}


# --- letters (alphabet order) ----------------------------------------------
L = lambda up, lo, say, sound, family, tip, example: {k: v for k, v in dict(upper=up, lower=lo, say=say, sound=sound, family=family, tip=tip, example=example).items() if v is not None}
LETTERS = [
    L("А", "а", "а", "a as in father", "familiar", "Same as A. Open wide: ah.", "ма'ма"),
    L("Б", "б", "ба", "b", "new", "A 6 with a flat roof: b.", "спаси'бо"),
    L("В", "в", "ва", "v", "false_friend", "Looks like B, sounds like v.", "вы"),
    L("Г", "г", "га", "g as in go", "new", "An upside-down L: g as in go.", "где"),
    L("Д", "д", "да", "d", "new", "A little house on feet: d.", "дом"),
    L("Е", "е", "е", "ye at the start of a word; after a consonant, e that softens it", "false_friend", "Looks like E: ye at the start of a word, a softening e after a consonant.", "нет"),
    L("Ё", "ё", "ё", "yo as in yoga, always stressed", "new", "Е with two dots: yo, and always the stressed vowel.", "всё"),
    L("Ж", "ж", "жа", "zh as in pleasure", "new", "A beetle with six legs: zh, as in pleasure.", "жук"),
    L("З", "з", "за", "z", "new", "Looks like a 3: z.", "звезда'"),
    L("И", "и", "и", "ee as in meet", "new", "A backwards N: ee as in meet.", "они'"),
    L("Й", "й", "ой", "a short y gliding off a vowel, as in boy", "new", "И with a little cap: the y at the end of boy.", "мой"),
    L("К", "к", "ка", "k", "familiar", "Same as K.", "кот"),
    L("Л", "л", "ла", "l, deep, like the ll in full", "new", "A tent with one leg bent: l.", "стол"),
    L("М", "м", "ма", "m", "familiar", "Same as M, and lowercase is just a small М.", "ма'ма"),
    L("Н", "н", "на", "n", "false_friend", "Looks like H, sounds like n.", "она'"),
    L("О", "о", "о", "o as in more, when stressed", "familiar", "Same as O. A full o only when stressed.", None),
    L("П", "п", "па", "p", "new", "A doorway: p.", "па'па"),
    L("Р", "р", "ра", "a tapped r", "false_friend", "Looks like P, sounds like a tapped r.", "раке'та"),
    L("С", "с", "са", "s", "false_friend", "Looks like C, always says s.", "сок"),
    L("Т", "т", "та", "t", "familiar", "Same as T, and lowercase is just a small Т.", "там"),
    L("У", "у", "у", "oo as in moon", "false_friend", "Looks like y, sounds like oo.", "тут"),
    L("Ф", "ф", "фа", "f", "new", "A circle on a pole: f.", "факт"),
    L("Х", "х", "ха", "kh, like ch in Scottish loch", "false_friend", "Looks like X, sounds like the ch in loch.", "хлеб"),
    L("Ц", "ц", "ца", "ts as in cats", "new", "A cup with a little tail: ts.", "цирк"),
    L("Ч", "ч", "ча", "ch as in chair", "new", "An upside-down h: ch as in chair.", "чай"),
    L("Ш", "ш", "ша", "sh, always hard", "new", "Three teeth: sh.", "наш"),
    L("Щ", "щ", "ща", "a long, soft sh", "new", "Ш with a tail: a longer, softer sh.", "ещё"),
    L("Ъ", "ъ", "объе'кт", "no sound of its own: the vowel after it starts with y, kept apart from the consonant", "sign", "The hard sign: a little wall between a consonant and я, е, ё, ю.", "объе'кт"),
    L("Ы", "ы", "ты", "a deep i, between ee and oo", "new", "Say ee and pull your tongue back: ы.", "ты"),
    L("Ь", "ь", "соль", "no sound: it softens the consonant before it", "sign", "The soft sign: silent, it softens the consonant before it.", "соль"),
    L("Э", "э", "э", "e as in bet", "new", "A backwards C with a tongue: e as in bet.", "э'то"),
    L("Ю", "ю", "ю", "yu as in you", "new", "A line holding an O: yu, as in you.", "люблю'"),
    L("Я", "я", "я", "ya as in yard", "new", "A backwards R: ya. On its own it means I.", "я"),
]

# --- lexicon ----------------------------------------------------------------
W = lambda ru, en, emoji=None, note=None: {k: v for k, v in dict(ru=ru, en=en, emoji=emoji, note=note).items() if v is not None}
LEXICON = [
    # unit 1-2
    W("ма'ма", "mom", "👩"),
    W("там", "there"), W("как", "how"), W("кто", "who"),
    W("кот", "cat", "🐈"), W("он", "he; it (masculine)"), W("она'", "she; it (feminine)"),
    W("оно'", "it (neuter)"), W("я", "I"), W("ты", "you (one, familiar)"), W("мы", "we"),
    W("вы", "you (plural or polite)"), W("они'", "they"), W("и", "and"), W("вот", "here is, here are"),
    # unit 3
    W("э'то", "this is, that is"), W("не", "not"), W("нет", "no"), W("рот", "mouth", "👄"),
    W("мир", "world; peace", "🌍"), W("торт", "cake", "🎂"), W("раке'та", "rocket", "🚀"),
    W("сок", "juice", "🧃"), W("нос", "nose", "👃"),
    W("сыр", "cheese", "🧀"), W("сестра'", "sister"), W("окно'", "window", "🪟", "neuter: ends in -о"),
    # unit 4
    W("тут", "here"), W("у", "at, by (as in у меня' есть, I have)"), W("да", "yes"), W("дом", "house, home", "🏠"),
    W("до'ма", "at home"), W("вода'", "water", "💧", "feminine: ends in -а"), W("у'тро", "morning", "🌅"),
    W("где", "where"), W("кни'га", "book", "📕", "feminine: ends in -а"),
    W("го'род", "city, town", "🏙️"), W("стол", "table", None, "masculine: ends in a consonant"), W("молоко'", "milk", "🥛", "neuter: ends in -о"),
    W("сло'во", "word"), W("Как дела'?", "How are you?"),
    # unit 5
    W("па'па", "dad", "👨", "masculine, despite the -а"), W("суп", "soup", "🍲"), W("спаси'бо", "thank you"), W("брат", "brother"),
    W("ро'бот", "robot", "🤖"), W("мой", "my (masculine)"), W("моя'", "my (feminine)"), W("моё", "my (neuter)"),
    W("всё", "everything, all"), W("люблю'", "I love"), W("лю'ди", "people"), W("я ду'маю", "I think"),
    # unit 6
    W("что", "what; that", None, "said shto"), W("чай", "tea", "🍵"), W("я чита'ю", "I read, I am reading"),
    W("чита'ет", "reads, is reading (he, she)"), W("наш", "our"), W("жук", "beetle", "🪲"),
    W("ка'ша", "porridge", "🥣"), W("то'же", "also, too"), W("ты чита'ешь", "you (one person) read"),
    W("ещё", "more; still"), W("хлеб", "bread", "🍞"), W("хорошо'", "good, well"),
    # unit 7
    W("мать", "mother"), W("соль", "salt", "🧂", "feminine: many nouns in -ь are"),
    W("есть", "there is (as in у меня' есть, I have)"), W("здесь", "here (= тут)"), W("звезда'", "star", "⭐"),
    W("я зна'ю", "I know"), W("ты зна'ешь", "you (one person) know"), W("цирк", "circus", "🎪"), W("факт", "fact"),
    W("ко'фе", "coffee", "☕", "masculine, despite the -е"), W("у'лица", "street"), W("оте'ц", "father"),
    W("объе'кт", "object"), W("лук", "onion", "🧅"), W("люк", "hatch"),
    W("щи", "cabbage soup"), W("пока'", "bye (to someone you call ты)"), W("до за'втра", "see you tomorrow"),
    # unit 8
    W("ча'шка", "cup", "☕", "feminine: ends in -а"), W("ло'жка", "spoon", "🥄"), W("таре'лка", "plate", "🍽️"),
    W("твой", "your (one person; masculine)"), W("а", "and, but (setting two things side by side)"), W("стул", "chair", "🪑"),
    W("столы'", "tables"), W("ча'шки", "cups"), W("ло'жки", "spoons"), W("таре'лки", "plates"), W("о'кна", "windows"), W("коты'", "cats"),
    W("но'вая", "new (feminine)"), W("но'вое", "new (neuter)"), W("но'вые", "new (plural)"), W("ста'рый", "old (masculine)"),
    # unit 9
    W("норма'льно", "fine, okay"), W("А у тебя'?", "And you?"), W("тебя'", "you (as in у тебя' есть, you have)"),
    W("когда'", "when"), W("ско'лько", "how many, how much"), W("бу'дет", "will be (as in ско'лько бу'дет…? what is…?)"),
    # unit 10
    W("твоя'", "your (one person; feminine)"), W("твоё", "your (one person; neuter)"), W("моя' сестра'", "my sister"),
    W("на'ша", "our (feminine)"), W("на'ше окно'", "our window"), W("на'ше", "our (neuter)"), W("ваш", "your (plural or polite; masculine)"),
    W("ва'ша", "your (plural or polite; feminine)"), W("ва'ше", "your (plural or polite; neuter)"), W("наш дом", "our house"),
    W("ку'хня", "kitchen"), W("у меня' есть", "I have"), W("меня'", "me (as in у меня' есть, I have)"),
    W("у меня' нет", "I don't have", None, "the thing lacked changes its ending: нет хле'ба"), W("хле'ба", "bread (after нет: no bread)"),
    W("со'ли", "salt (after нет: no salt)"), W("ча'я", "tea (after нет: no tea)"), W("сы'ра", "cheese (after нет: no cheese)"),
    W("молока'", "milk (after нет: no milk)"),
    # unit 11
    W("чита'ть", "to read"), W("мы чита'ем", "we read"), W("вы чита'ете", "you (plural) read"), W("они' чита'ют", "they read"),
    W("ду'мать", "to think"), W("ты ду'маешь", "you (one person) think"), W("она' ду'мает", "she thinks"),
    W("мы ду'маем", "we think"), W("вы ду'маете", "you (plural) think"), W("они' ду'мают", "they think"),
    W("кни'гу", "book (as the object)", None, "книга becomes книгу as the object of a verb"),
    W("знать", "to know"), W("я не зна'ю", "I don't know"), W("зна'ет", "knows (he, she)"), W("мы зна'ем", "we know"),
    W("вы зна'ете", "you (plural) know"), W("они' зна'ют", "they know"),
    # unit 12
    W("говори'ть", "to speak"), W("я говорю'", "I speak"), W("ты говори'шь", "you (one person) speak"),
    W("говори'т", "speaks (he, she)"), W("мы говори'м", "we speak"), W("вы говори'те", "you (plural) speak"),
    W("они' говоря'т", "they speak"), W("понима'ть", "to understand"), W("я понима'ю", "I understand"),
    W("ты понима'ешь", "you (one person) understand"), W("по-ру'сски", "in Russian"),
    W("по-англи'йски", "in English"), W("немно'го", "a little"), W("Нет, спаси'бо.", "No, thank you."),
    W("ме'дленно", "slowly"), W("я не понима'ю", "I don't understand"), W("Ещё раз, пожа'луйста.", "Once more, please."), W("Поме'дленнее, пожа'луйста.", "Slower, please."),
    W("ещё раз", "once more, again"), W("Как э'то по-ру'сски?", "How do you say this in Russian?"), W("что зна'чит", "what does … mean?"),
    # unit 13
    W("до'брое у'тро", "good morning"), W("до'брый ве'чер", "good evening"), W("споко'йной но'чи", "good night"),
    W("прия'тного аппети'та", "enjoy your meal"), W("Спаси'бо, и тебе'!", "Thanks, you too!"), W("тебе'", "to you (as in и тебе', you too)"),
    W("вку'сно", "tasty, delicious"), W("о'чень", "very"), W("пожа'луйста", "please; you're welcome"),
    W("Мо'жно ещё?", "May I have more?"), W("мо'жно", "may I, it's allowed"), W("переда'й", "pass (to one person)"),
    # unit 14
    W("за'втрак", "breakfast", "🍳"), W("у'жин", "dinner", "🍽️"), W("обе'д", "lunch"), W("на", "on; for (as in Что на у'жин?)"),
    W("гото'в", "ready (masculine)"), W("я хочу'", "I want"), W("ты хо'чешь", "you (one person) want"),
    W("с", "with"), W("мне", "to me, for me"), W("са'хар", "sugar"), W("с молоко'м", "with milk"), W("с са'харом", "with sugar"),
    W("Да, пожа'луйста.", "Yes, please."), W("У'жин гото'в!", "Dinner is ready!"),
    # unit 15
    W("оди'н", "one"), W("два", "two"), W("три", "three"), W("четы'ре", "four"),
    W("пять", "five"), W("шесть", "six"), W("семь", "seven"), W("во'семь", "eight"), W("де'вять", "nine"),
    W("де'сять", "ten"), W("плюс", "plus"), W("двена'дцать", "twelve"), W("четы'рнадцать", "fourteen"),
    W("шестна'дцать", "sixteen"), W("семна'дцать", "seventeen"), W("восемна'дцать", "eighteen"), W("девятна'дцать", "nineteen"),
    # unit 16
    W("ко'смос", "space (outer space)", "🌌"), W("пла'нета", "planet", "🪐"), W("лети'т", "flies (it)"),
    W("в", "in, into"), W("число'", "number", "🔢"), W("зада'ча", "problem, task"), W("пра'вда", "truth; true"), W("непра'вда", "not true, a lie"),
    W("мысль", "thought", "💭"), W("ничего'", "nothing", None, "said nichevo"), W("мы'слю", "I think, I reason (as in Descartes)"),
    W("сле'довательно", "therefore"), W("существу'ю", "I exist"), W("ночь", "night", "🌙"),
    W("звёзды", "stars", "✨"), W("большо'й", "big"), W("одни'", "alone (plural)"),
]

# --- units ------------------------------------------------------------------
U = []

U.append(("u1", "Familiar faces", "А М О Т К Н", [
    ("First word", "Three letters that look and sound the way you expect. By the end of this page you will write a Russian word.", [
        meet("А"), meet("М"), tr("ма"), meet("О"),
        ex("Read straight through, sound by sound: м + а = ма.", [("ма", "ma"), ("мо", "mo"), ("ма'ма", "mom")]),
        lp("ма", ["ма", "мо", "ам"]), cp("ма'ма"), ra("ма'ма"), dc("ма'ма"),
    ]),
    ("Т and К", "Two more familiar shapes, and you can read real words: there, cat, how, who.", [
        meet("Т"), meet("К"), tr("там"), lp("кот", ["кот", "так", "там"]), cp("как"), ra("кто"),
        tl("cat", "кот"), dc("там"), rm("Кот там.", ["The cat is there.", "Mom is there.", "Who is there?"]),
        tl("who", "кто"),
    ]),
    ("Н: looks like H", "Н looks like H but says n. With it come he, she and it.", [
        meet("Н"),
        ex("Н looks like H and sounds like n, so он reads on. In она' and оно' the stress is on the end, and the first о relaxes toward a: ana, ano.", [("он", "he"), ("она'", "she"), ("оно'", "it")]),
        tr("он"), cp("она'"), lp("оно'", ["он", "она'", "оно'"]), ra("Он там."), dc("она'"),
        fl("", "Она'", " там.", "She is there."), tl("he", "он"),
        rm("Ма'ма там.", ["Mom is there.", "The cat is there.", "He is there."]),
    ]),
]))

U.append(("u2", "I, you, we", "я ты мы вы они́", [
    ("Я and Ы", "Я is a whole word: I. Ы is a brand-new sound, and it gives you you and we.", [
        meet("Я"), meet("Ы"),
        ex("Ы has no English twin: say ee, then pull your tongue back without moving your lips. You hear it in ты and мы.", [("ты", "you"), ("мы", "we")]),
        tr("я"), cp("ты"), lp("мы", ["мы", "ты", "я"]), ra("Я там."), dc("ты"), tl("we", "мы"),
        fl("", "Ты", " там.", "You (one, familiar) are there."),
    ]),
    ("И and В", "И looks like a backwards N and says ee. В looks like B and says v. Together: you, they, and.", [
        meet("И"), meet("В"), tr("вы"), lp("они'", ["они'", "она'", "оно'"]), cp("и"), ra("вот"),
        dc("вот"), tl("they", "они'"),
        rm("Вот кот и ма'ма.", ["Here are the cat and mom.", "The cat is there.", "Who is mom?"]),
        fl("Ма'ма ", "и", " кот.", "Mom and the cat."),
    ]),
    ("All the pronouns", "Eight little words: I, you, he, she, it, we, you, they. Write each from memory.", [
        ex("Ты is one person you know well; вы is several people, or one person you address politely.", [("ты", "you (one, familiar)"), ("вы", "you (plural or polite)")]),
        tl("I", "я"), tl("she", "она'"), tl("it (neuter)", "оно'"), tl("you (plural or polite)", "вы"),
        lp("ты", ["ты", "вы", "мы"]), dc("они'"), ra("Я, ты, он, она', оно', мы, вы, они'."),
        fl("", "Мы", " там.", "We are there."),
        rm("Они' там.", ["They are there.", "We are there.", "She is there."]),
    ]),
]))

U.append(("u3", "This is…", "э́то, не, нет · Р С", [
    ("Э and Е", "Э says e as in bet and starts the most useful word in Russian, э́то. Е says ye and gives you no.", [
        meet("Э"), meet("Е"),
        ex("Э is a plain e. Е is ye at the start of a word, and after a consonant it is e that softens the consonant, as in не and нет. Э'то means this is or that is, with no verb needed.", [("Э'то кот.", "This is a cat."), ("Нет.", "No.")]),
        tr("э'то"), cp("нет"), lp("не", ["не", "нет", "на"]), ra("Э'то ма'ма."), dc("Э'то кот."),
        tl("no", "нет"), rm("Э'то не кот.", ["This is not a cat.", "This is a cat.", "Who is this?"]),
    ]),
    ("Р: looks like P", "Р is a tapped r: the tip of the tongue flicks once behind the teeth. Now: world, cake, rocket.", [
        meet("Р"), tr("мир"), lp("рот", ["рот", "кот", "торт"]), cp("торт"), ra("Э'то раке'та."),
        dc("мир"), tl("rocket", "раке'та"), fl("Э'то ", "торт", ".", "This is a cake."),
        rm("Раке'та там.", ["The rocket is there.", "The cake is there.", "The world is there."]),
    ]),
    ("С, and the lazy о", "С looks like C and always says s. Unstressed о has a surprise too.", [
        meet("С"),
        ex("Only a stressed о is a full o. Unstressed, it relaxes to a: окно' sounds like akno, она' like ana.", [("окно'", "window"), ("она'", "she"), ("оно'", "it"), ("сестра'", "sister"), ("сыр", "cheese")]),
        tr("сок"), lp("нос", ["нос", "сок", "сон"]), cp("окно'"), ra("Кто э'то? Э'то сестра'."),
        dc("сыр"), tl("juice", "сок"), tl("window", "окно'"),
        rm("Э'то не сыр, э'то торт.", ["This is not cheese, this is cake.", "This is cheese, not cake.", "The cheese is there."]),
    ]),
]))

U.append(("u4", "Where?", "У Д Г Л · тут, там, где", [
    ("У and Д", "У looks like y and says oo. Д is a little house that says d: yes, house, water.", [
        meet("У"), meet("Д"), tr("тут"), lp("да", ["да", "дом", "тут"]), cp("до'ма"), ra("Я тут. Ты там."),
        dc("дом"), tl("yes", "да"), tl("here", "тут"), fl("Ма'ма ", "до'ма", ".", "Mom is at home."),
    ]),
    ("Г and где?", "Г is an upside-down L and says g as in go. Its best word is где, where.", [
        meet("Г"),
        ex("In the present, Russian leaves out is, and it asks questions without do: Где кот? is Where is the cat?", [("Где кот?", "Where is the cat?"), ("Кот до'ма.", "The cat is at home."), ("го'род", "city, town")]),
        tr("где"), lp("кни'га", ["кни'га", "нога'", "го'род"]), cp("Где кни'га?", 1), ra("Где ма'ма? Она' до'ма."),
        dc("го'род"), tl("where", "где"), fl("", "Где", " вода'?", "Where is the water?"),
        rm("Кни'га там.", ["The book is there.", "Where is the book?", "The leg is there."]),
    ]),
    ("Л: a tent", "Л says l and brings the table and the milk.", [
        meet("Л"), tr("стол"), lp("сло'во", ["молоко'", "сло'во", "стол"]), cp("молоко'"), ra("Молоко' тут."),
        dc("сло'во"), tl("table", "стол"), tl("milk", "молоко'"),
        rm("Где молоко'? Оно' там.", ["Where is the milk? It is there.", "Where is the table? It is here.", "The milk is at home."]),
        fl("Молоко' ", "тут", ".", "The milk is here."),
    ]),
    ("Who? Where? How?", "No new letters: questions and answers with everything so far.", [
        tl("who", "кто"), tl("how", "как"), lp("где", ["где", "кто", "как"]), dc("Где кот?"),
        fl("", "Кто", " э'то?", "Who is this?"), ra("Кто там? Э'то ма'ма."), tl("at home", "до'ма"),
        ps("Кто там? Э'то ма'ма. Ма'ма до'ма. А где кот? Кот тут.",
           "Who is there? It's mom. Mom is at home. And where is the cat? The cat is here."),
        rm("А где кот? Кот тут.", ["And where is the cat? The cat is here.", "And who is the cat? The cat is mom.", "And where is mom? Mom is at home."]),
        cp("Где вода'? Вода' тут.", 1),
    ]),
]))

U.append(("u5", "New shapes", "П Б Й Ё Ю", [
    ("П and Б", "П is a doorway and says p. Б looks like a 6 and says b. They bring dad, soup and thank you.", [
        meet("П"), meet("Б"), tr("брат"), lp("суп", ["суп", "сыр", "сок"]), cp("спаси'бо"), ra("ро'бот"),
        dc("брат"), tl("thank you", "спаси'бо"), tl("soup", "суп"),
        rm("Э'то ро'бот. Э'то не брат.", ["This is a robot. This is not a brother.", "This is a brother. This is not a robot.", "The robot is at home."]),
    ]),
    ("Й and Ё: mine", "Й is the short y at the end of boy: мой, my. Ё says yo and is always stressed.", [
        meet("Й"), meet("Ё"),
        ex("My follows the thing it owns: мой for a consonant ending, моя' for -а, моё for -о.", [("мой дом", "my house"), ("моя' кни'га", "my book"), ("моё окно'", "my window")]),
        tr("мой"), lp("моё", ["мой", "моя'", "моё"]), cp("моя' ма'ма", 1), ra("Э'то мой брат."), dc("моё окно'"),
        fl("Э'то ", "моя'", " кни'га.", "This is my book."), cp("всё", 1),
    ]),
    ("Ю and the vowel pairs", "Ю says yu, and completes a pattern: each plain vowel has a y-partner.", [
        meet("Ю"),
        ex("Five vowels come in pairs, hard and soft: а/я, у/ю, э/е, о/ё, ы/и. At the start of a word or after a vowel (моя', моё), я, ю, е and ё begin with a y. After a consonant the soft letters make the consonant itself soft (three you will meet later always stay hard).", [("да", "yes"), ("я", "I"), ("тут", "here"), ("люблю'", "I love")]),
        tr("люблю'"), lp("лю'ди", ["лю'ди", "люблю'", "ду'маю"]), cp("ду'маю"), ra("Я ду'маю."), dc("люблю'"),
        tl("I think", "я ду'маю"),
        rm("Я люблю' суп.", ["I love soup.", "I think about soup.", "This is my soup."]),
        fl("Я ", "люблю'", " молоко'.", "I love milk."),
    ]),
]))

U.append(("u6", "Hushing sounds", "Ч Ш Ж Щ Х", [
    ("Ч: what?", "Ч says ch as in chair. In что, what, it is said sh: shto.", [
        meet("Ч"),
        ex("Что is spelled with ч but said shto. Что э'то? is the question you will ask most.", [("Что э'то?", "What is this?"), ("Э'то чай.", "This is tea."), ("Я чита'ю.", "I am reading."), ("Он чита'ет.", "He is reading.")]),
        tr("что"), lp("чай", ["чай", "что", "чита'ю"]), cp("Что э'то?", 1), ra("Что э'то? Э'то чай."), dc("чай"),
        tl("what", "что"), fl("Я ", "чита'ю", ".", "I am reading."),
        rm("Ма'ма чита'ет.", ["Mom is reading.", "Mom is thinking.", "Mom is at home."]),
    ]),
    ("Ш and Ж", "Ш is three teeth: sh. Ж is a beetle: zh, as in pleasure.", [
        meet("Ш"), meet("Ж"),
        ex("То'же means too or also, and наш means our.", [("Я то'же.", "Me too."), ("наш кот", "our cat")]), lp("жук", ["жук", "шум", "суп"]), cp("ка'ша"), ra("Наш кот то'же тут."),
        dc("то'же"), tl("our", "наш"), tl("porridge", "ка'ша"), fl("Я ", "то'же", " чита'ю.", "I am reading too."),
    ]),
    ("Щ and Х", "Щ is a long soft sh and gives you ещё, more. Х is the ch of Scottish loch and gives you хорошо́, good.", [
        meet("Щ"), meet("Х"),
        ex("Х is a breathy kh from the back of the throat. Хорошо' has three о but only the last is full: kha-ra-SHO.", [("хорошо'", "good, well"), ("хлеб", "bread"), ("ещё", "more, still")]),
        tr("ещё"), lp("хлеб", ["хлеб", "хор", "ещё"]), cp("хорошо'"), ra("Как дела'? Хорошо'!"), dc("хлеб"),
        tl("good, well", "хорошо'"), tl("more", "ещё"),
    ]),
]))

U.append(("u7", "The last letters", "З Ь Ц Ф Ъ", [
    ("З and Ь", "З looks like a 3 and says z. Ь has no sound of its own: it softens the consonant before it.", [
        meet("З"), meet("Ь"),
        ex("Ь softens the consonant before it, as if a tiny y were starting. Compare брат with мать: the т of мать is soft.", [("брат", "brother"), ("мать", "mother"), ("соль", "salt"), ("Я зна'ю.", "I know.")]),
        tr("соль"), lp("здесь", ["есть", "здесь", "звезда'"]), cp("звезда'"), ra("Я зна'ю. Ты зна'ешь."),
        dc("мать"), tl("star", "звезда'"), fl("Я ", "зна'ю", ".", "I know."),
    ]),
    ("Ц and Ф", "Ц is ts as in cats. Ф is a circle on a pole: f. Now: fact, coffee, street.", [
        meet("Ц"), meet("Ф"), tr("у'лица"), lp("цирк", ["цирк", "факт", "ко'фе"]), cp("ко'фе"), ra("оте'ц"),
        dc("у'лица"), tl("coffee", "ко'фе"), tl("father", "оте'ц"),
        rm("Где оте'ц? Он до'ма.", ["Where is father? He is at home.", "Where is the street? It is there.", "Who is father? He is here."]),
    ]),
    ("Hello, and the hard sign", "Приве́т and пока́: hi and bye at home. And the last letter, Ъ, a silent wall.", [
        ex("Приве'т is hi and пока' is bye, for anyone you call ты. До за'втра is see you tomorrow.", [("Приве'т!", "Hi!"), ("Пока'!", "Bye!"), ("До за'втра!", "See you tomorrow!")]),
        tr("приве'т"), lp("пока'", ["пока'", "как", "приве'т"]), fl("", "Приве'т", "! Как дела'?", "Hi! How are you?"),
        dc("Пока'!"), tl("bye", "пока'"), ra("Пока'! До за'втра!"),
        meet("Ъ"),
        ex("Ъ makes no sound: it keeps a consonant apart from the я, е, ё or ю after it, so объе'кт is ab-YEKT. It is rare, and it makes 33.", [("объе'кт", "object")]),
        cp("объе'кт", 1),
    ]),
    ("The whole alphabet", "All 33 letters at once: words from every corner of the alphabet.", [
        ex("Лук and люк have the same oo sound; the difference you hear is the л, hard in лук and soft in люк. The vowel letter is only how the spelling shows it.", [("лук", "onion"), ("люк", "hatch")]),
        lp("люк", ["лук", "люк"]), dc("лук"), lp("щи", ["щи", "шум", "чай"]), dc("жук"), tl("salt", "соль"), tl("tea", "чай"),
        fl("Э'то ", "мой", " чай.", "This is my tea."), ra("Хлеб, сыр, чай, ко'фе."), dc("звезда'"),
    ]),
    ("By ear: hard and soft", "Pairs of words that differ by one soft consonant, and the voiced letters that go quiet at the end of a word.", [
        ex("A soft consonant is not a consonant plus y: the middle of the tongue rises toward the roof of the mouth while you say it. Брат and брать differ only in that т. At the end of a word, and before a voiceless consonant, б, в, г, д, ж, з lose their voice: хлеб ends in a p, ло'жка says losh-ka.",
           [("брат", "brother"), ("брать", "to take"), ("хлеб", "bread"), ("ло'жка", "spoon")]),
        lp("брать", ["брат", "брать"]), lp("бил", ["был", "бил"]), dc("люк"), dc("мать"), dc("хлеб"), dc("го'род"),
        tl("brother", "брат"), cp("брат — брать", 1), ra("Брат, брать. Лук, люк. Был, бил."),
    ]),
]))

U.append(("u8", "It: он, она́, оно́", "gender by ending", [
    ("He, she, it", "Every thing is он, она́ or оно́, and the last letter usually tells you which.", [
        ex("Consonant at the end: он; -а or -я: она'; -о or -е: оно'; -ь can be он or она'. Па'па is он: meaning beats ending.", [("стол — он", "the table: it (he)"), ("кни'га — она'", "the book: it (she)"), ("окно' — оно'", "the window: it")]),
        lp("она'", ["он", "она'", "оно'"]), tl("book", "кни'га"),
        fl("Где стол? Вот ", "он", ".", "Where is the table? Here it is."),
        fl("Где кни'га? Вот ", "она'", ".", "Where is the book? Here it is."),
        fl("Где окно'? Вот ", "оно'", ".", "Where is the window? Here it is."),
        ra("Где па'па? Вот он."), dc("Вот она'."),
        rm("Где ро'бот? Вот он.", ["Where is the robot? Here it is.", "Where is the robot? Here she is.", "Who is the robot?"]),
    ]),
    ("On the table", "Table things and their genders. Each one with мой, моя́ or моё.", [
        ex("Ко'фе ends in -е but is masculine: мой ко'фе. Everything else here follows its ending.", [("мой ко'фе", "my coffee"), ("моя' ча'шка", "my cup"), ("моё молоко'", "my milk"), ("твой чай", "your tea"), ("стул", "chair")]),
        tr("ча'шка"), tl("cup", "ча'шка"), lp("ло'жка", ["ло'жка", "ча'шка", "ка'ша"]), cp("таре'лка", 1),
        tl("spoon", "ло'жка"), fl("Э'то ", "моя'", " ча'шка.", "This is my cup."),
        ra("Э'то мой ко'фе, а э'то твой чай."), dc("таре'лка"),
        rm("Где моя' ло'жка?", ["Where is my spoon?", "Where is my cup?", "This is my spoon."]),
    ]),
    ("Э́то + anything", "Э́то points at anything, and it never changes.", [
        dc("Э'то моя' таре'лка."), tl("plate", "таре'лка"), lp("Э'то не чай.", ["Э'то не чай.", "Где чай?", "Что э'то?"]),
        ra("Что э'то? Э'то ло'жка."), fl("Э'то не стол, э'то ", "стул", ".", "This is not a table, it's a chair."),
        tl("chair", "стул"), cp("Э'то не стул, э'то стол.", 1),
        ps("Что э'то? Э'то стол. А э'то? Э'то не стол, э'то стул. Где моя' ча'шка? Вот она'.",
           "What is this? It's a table. And this? It's not a table, it's a chair. Where is my cup? Here it is."),
        rm("Э'то не стол, э'то стул.", ["This is not a table, it's a chair.", "This is not a chair, it's a table.", "This is not my cup."]),
    ]),
    ("Two cups: plurals", "More than one: most plurals end in -ы or -и.", [
        ex("A consonant adds -ы and -а becomes -ы: стол, столы'; -я and -ь become -и. After к, г, х, ж, ш, ч, щ it is always -и, never -ы: ча'шка, ча'шки. Neuter -о becomes -а, and the stress can move: окно', о'кна.",
           [("столы'", "tables"), ("ча'шки", "cups"), ("таре'лки", "plates"), ("кни'ги", "books"), ("о'кна", "windows")]),
        tr("ча'шки"), lp("ло'жки", ["ло'жка", "ло'жки", "ча'шки"]), tl("tables", "столы'"), tl("windows", "о'кна"),
        fl("Вот ", "таре'лки", ".", "Here are the plates."), dc("Где ло'жки?"), ra("Тут ча'шки, а там таре'лки."), cp("коты'", 1),
        rm("Ло'жки тут, а ча'шки там.", ["The spoons are here, and the cups are there.", "The spoon is here, and the cup is there.", "The cups are here, and the spoons are there."]),
    ]),
    ("A new cup: но́вый, но́вая, но́вое", "Describing words follow the noun's gender, just like мой.", [
        ex("Adjectives match the noun: но'вый стол, но'вая ча'шка, но'вое окно', and for plurals но'вые ча'шки. Ста'рый, old, works the same way.",
           [("но'вый стул", "a new chair"), ("но'вая ча'шка", "a new cup"), ("но'вое окно'", "a new window"), ("но'вые ло'жки", "new spoons"), ("ста'рый стол", "an old table")]),
        tr("но'вая ча'шка"), lp("но'вое", ["но'вый", "но'вое", "но'вые"]), tl("new (masculine)", "но'вый"),
        fl("Э'то ", "но'вая", " таре'лка.", "This is a new plate."), fl("Вот ", "но'вые", " ло'жки.", "Here are the new spoons."),
        dc("Э'то но'вый стул."), ra("Мой стол ста'рый, а стул но'вый."), tl("old (masculine)", "ста'рый"),
        rm("Э'то но'вое окно'?", ["Is this a new window?", "This is an old window.", "Is this a new plate?"]),
    ]),
]))

U.append(("u9", "Questions", "кто? что? где? как? да и'ли нет?", [
    ("Who and what", "Кто is for people and animals, что for things.", [
        ex("Кто? asks about people and animals, even a cat. Что? asks about everything else.", [("Кто э'то? Э'то кот.", "Who is this? It's the cat."), ("Что э'то? Э'то ло'жка.", "What is this? It's a spoon.")]),
        lp("кто", ["кто", "что", "как"]), fl("", "Кто", " там? Э'то мой брат.", "Who is there? It's my brother."),
        fl("", "Что", " там? Э'то суп.", "What is there? It's soup."), dc("Что э'то?"), ra("Кто э'то? Что э'то?"),
        tl("what", "что"), rm("Кто до'ма?", ["Who is at home?", "What is at home?", "Where is home?"]), cp("Кто там?", 1),
    ]),
    ("Where and how", "Где asks where; как asks how. Как дела́? is how are you.", [
        ex("Как дела'? is how are you, between people who know each other. Answer хорошо' (good) or норма'льно (fine).", [("Как дела'?", "How are you?"), ("Хорошо', спаси'бо.", "Good, thanks."), ("Норма'льно.", "Okay.")]),
        tl("How are you?", "Как дела'?"), dc("Хорошо', спаси'бо."), ra("Как дела'? Хорошо', спаси'бо. А у тебя'?"),
        fl("", "Где", " моя' ча'шка?", "Where is my cup?"), tl("okay, fine", "норма'льно"), cp("А у тебя'?", 1),
        rm("Где хлеб? Он тут.", ["Where is the bread? It's here.", "Where is the bread? It's there.", "How is the bread?"]),
    ]),
    ("Yes or no", "A yes/no question is a statement with a rising voice. Answer да or нет.", [
        ex("Э'то чай. is a statement; Э'то чай? with the voice rising on чай is a question. Nothing else changes.", [("Э'то чай?", "Is this tea?"), ("Да, э'то чай.", "Yes, this is tea."), ("Нет, э'то ко'фе.", "No, this is coffee.")]),
        lp("Да, э'то чай.", ["Да, э'то чай.", "Нет, э'то ко'фе.", "Что э'то?"]), dc("Да, э'то чай."),
        fl("Э'то суп? ", "Нет", ", э'то ка'ша.", "Is this soup? No, it's porridge."), ra("Ты до'ма? Да, я до'ма."),
        tl("yes", "да"), tl("no", "нет"), cp("Нет, спаси'бо.", 1),
        rm("Э'то твой брат? Нет.", ["Is this your brother? No.", "This is your brother. Yes.", "Where is your brother?"]),
    ]),
    ("When? How many?", "Two more question words: когда́, when, and ско́лько, how many or how much.", [
        ex("Когда'? asks when, ско'лько? how many or how much. Ско'лько бу'дет два плюс три? is what is two plus three.",
           [("Когда' за'втрак?", "When is breakfast?"), ("Ско'лько бу'дет два плюс три?", "What is two plus three?"), ("Пять.", "Five.")]),
        lp("когда'", ["когда'", "кто", "где"]), tl("when", "когда'"), tl("how many, how much", "ско'лько"),
        fl("", "Когда'", " у'жин?", "When is dinner?"), fl("", "Ско'лько", " бу'дет три плюс три?", "What is three plus three?"),
        dc("Когда' за'втрак?"), ra("Ско'лько бу'дет пять плюс пять? Де'сять."), cp("ско'лько", 1),
        rm("Ско'лько бу'дет два плюс два?", ["What is two plus two?", "When is it two o'clock?", "Who has two?"]),
    ]),
]))

U.append(("u10", "Mine and yours", "мой, твой, наш, ваш · у меня́ есть", [
    ("My and your", "Твой is your for one person you know well, and it changes exactly like мой.", [
        ex("мой / моя' / моё and твой / твоя' / твоё: the same split as он / она' / оно'.", [("твой брат", "your brother"), ("твоя' ча'шка", "your cup"), ("твоё молоко'", "your milk")]),
        lp("твоя'", ["твой", "твоя'", "твоё"]), fl("Э'то ", "твоя'", " ло'жка?", "Is this your (one, familiar) spoon?"),
        fl("Нет, э'то ", "моё", " молоко'.", "No, this is my milk."), tl("my sister", "моя' сестра'"),
        dc("твой ко'фе"), fl("Э'то ", "твоё", " окно'?", "Is this your (one, familiar) window?"), ra("Э'то твоя' кни'га? Да, моя'."), cp("моё окно'", 1),
    ]),
    ("Our and your", "Наш and ваш follow the same pattern: на́ша, на́ше, ва́ша, ва́ше.", [
        ex("Наш is our, ваш is your when you say вы. Both add -а and -е: на'ша, на'ше, ва'ша, ва'ше.", [("наш дом", "our house"), ("на'ша ку'хня", "our kitchen"), ("на'ше молоко'", "our milk"), ("ва'ша кни'га", "your book"), ("ва'ше окно'", "your window")]),
        tr("на'ша ку'хня"), lp("ваш", ["наш", "ваш", "на'ша"]), tl("our house", "наш дом"), tl("our window", "на'ше окно'"), tl("kitchen", "ку'хня"),
        fl("Где ", "ва'ша", " ку'хня?", "Where is your (plural/polite) kitchen?"), ra("На'ша ку'хня тут."), dc("ва'ше окно'"),
        rm("Э'то наш кот.", ["This is our cat.", "This is your cat.", "Where is our cat?"]),
    ]),
    ("I have: у меня́ есть", "Russian says by me there is: у меня́ есть. It is the everyday I have.", [
        ex("У меня' есть is literally by me there is. Swap in тебя' for you: у тебя' есть.", [("У меня' есть кот.", "I have a cat."), ("У тебя' есть хлеб?", "Do you have bread?"), ("Да, есть.", "Yes, I do.")]),
        tr("у меня' есть"), lp("У тебя' есть ко'фе?", ["У тебя' есть ко'фе?", "У меня' есть ко'фе.", "Где ко'фе?"]),
        tl("I have", "у меня' есть"), fl("У ", "меня'", " есть сестра'.", "I have a sister."), dc("У тебя' есть чай?"),
        ra("У меня' есть брат и сестра'."), fl("У тебя' есть соль? Да, ", "есть", ".", "Do you have salt? Yes, I do."),
        rm("У меня' есть ро'бот.", ["I have a robot.", "You have a robot.", "The robot has me."]),
    ]),
    ("I don't have: у меня́ нет", "For none, есть becomes нет, and the thing you lack changes its ending.", [
        ex("Swap есть for нет and change the ending: a consonant adds -а (хлеб, хле'ба), -й becomes -я (чай, ча'я), and -о becomes -а. -а becomes -ы, or -и after к, г, х, ж, ш, ч, щ, and -ь becomes -и (соль, со'ли). Ко'фе never changes.",
           [("У меня' нет хле'ба.", "I have no bread."), ("У тебя' нет со'ли?", "Don't you have any salt?"), ("Молока' нет.", "There's no milk."), ("Ча'я и сы'ра нет.", "There's no tea and no cheese.")]),
        tr("у меня' нет"), lp("У меня' нет хле'ба.", ["У меня' нет хле'ба.", "У меня' есть хлеб.", "У тебя' нет хле'ба?"]),
        tl("I don't have", "у меня' нет"), fl("У меня' нет ", "со'ли", ".", "I have no salt."), fl("У тебя' нет ", "ча'я", "?", "Don't you have any tea?"),
        dc("У меня' нет сы'ра."), ra("У тебя' есть молоко'? Нет, молока' нет."),
        rm("Ко'фе есть, а молока' нет.", ["There's coffee, but no milk.", "There's milk, but no coffee.", "There's coffee and milk."]),
    ]),
]))

U.append(("u11", "Doing things", "чита́ть, ду́мать, знать", [
    ("Reading: я чита́ю", "One verb, six endings. Learn them on чита́ть, to read, and they work for hundreds of verbs.", [
        ex("Drop -ть from чита'ть and add the ending for who is doing it: я -ю, ты -ешь, он and она' -ет.", [("чита'ть", "to read"), ("я чита'ю", "I read"), ("ты чита'ешь", "you read"), ("она' чита'ет", "she reads"), ("чита'ю кни'гу", "I read a book")]),
        tr("я чита'ю"), lp("ты чита'ешь", ["я чита'ю", "ты чита'ешь", "он чита'ет"]), cp("он чита'ет", 1),
        tl("I read", "я чита'ю"), tl("to read", "чита'ть"), fl("Ма'ма ", "чита'ет", ".", "Mom is reading."), dc("Ты чита'ешь."),
        ra("Я чита'ю кни'гу."),
        rm("Он чита'ет кни'гу.", ["He is reading a book.", "She is reading a book.", "He is thinking about a book."]),
    ]),
    ("We, you, they", "The plural endings: мы чита'ем, вы чита'ете, они' чита'ют.", [
        ex("мы чита'ем, вы чита'ете, они' чита'ют. Ду'мать, to think, works the same way.", [("мы чита'ем", "we read"), ("вы чита'ете", "you read"), ("ду'мать", "to think"), ("ты ду'маешь", "you think"), ("она' ду'мает", "she thinks"), ("мы ду'маем", "we think"), ("вы ду'маете", "you think"), ("они' ду'мают", "they think")]),
        lp("они' чита'ют", ["мы чита'ем", "вы чита'ете", "они' чита'ют"]), fl("Мы ", "чита'ем", ".", "We are reading."),
        fl("Они' ", "ду'мают", ".", "They are thinking."), tl("we think", "мы ду'маем"), tl("to think", "ду'мать"), dc("Вы чита'ете?"),
        ra("Мы ду'маем. Они' чита'ют."), cp("вы ду'маете", 1),
        rm("Что ты ду'маешь?", ["What do you think?", "What are you reading?", "Who is thinking?"]),
    ]),
    ("Knowing: знать", "Знать, to know, has the same endings. Я зна́ю and я не зна́ю carry a lot of conversations.", [
        ex("Не goes right before the verb: я не зна'ю, I don't know.", [("знать", "to know"), ("я зна'ю", "I know"), ("я не зна'ю", "I don't know"), ("ты зна'ешь?", "do you know?"), ("он зна'ет", "he knows"), ("мы зна'ем", "we know"), ("вы зна'ете", "you know"), ("они' зна'ют", "they know")]),
        tr("я не зна'ю"), lp("я не зна'ю", ["я зна'ю", "я не зна'ю", "ты зна'ешь"]), tl("I don't know", "я не зна'ю"),
        fl("Ты ", "зна'ешь", ", где соль?", "Do you know where the salt is?"), dc("Они' зна'ют."),
        ra("Я не зна'ю. А ты зна'ешь?"), tl("we know", "мы зна'ем"), tl("to know", "знать"),
        rm("Она' зна'ет, где кот.", ["She knows where the cat is.", "She doesn't know where the cat is.", "Where is the cat?"]),
    ]),
    ("All six, from memory", "Every form of all three verbs, recalled. Say each one as you write it.", [
        tl("you read (one person)", "ты чита'ешь"), tl("she thinks", "она' ду'мает"), tl("they know", "они' зна'ют"),
        fl("Вы ", "зна'ете", "?", "Do you know?"), lp("ду'мает", ["ду'маю", "ду'мает", "ду'маем"]),
        dc("Я ду'маю, что э'то факт."), ra("Я чита'ю. Ты ду'маешь. Мы зна'ем."),
        fl("Мы не ", "чита'ем", ", мы ду'маем.", "We are not reading, we are thinking."),
        rm("Они' не чита'ют.", ["They are not reading.", "They are reading.", "We are not reading."]),
    ]),
]))

U.append(("u12", "Speaking and understanding", "говори́ть, понима́ть, не", [
    ("Speaking: говори́ть", "Some verbs take и where чита́ть has е: говори́шь, говори́т. Same idea, different vowel.", [
        ex("я говорю', ты говори'шь, он говори'т, мы говори'м, вы говори'те, они' говоря'т. The stress stays on the end throughout.", [("говори'ть", "to speak"), ("я говорю'", "I speak"), ("ты говори'шь", "you speak"), ("мы говори'м", "we speak"), ("они' говоря'т", "they speak")]),
        tr("я говорю'"), lp("говори'т", ["говорю'", "говори'т", "говоря'т"]), cp("вы говори'те", 1),
        tl("I speak", "я говорю'"), tl("to speak", "говори'ть"), fl("Мы ", "говори'м", ".", "We are talking."), dc("Они' говоря'т."),
        ra("Ты говори'шь, а я ду'маю."), rm("Кто говори'т?", ["Who is speaking?", "Who is reading?", "Who knows?"]),
    ]),
    ("In Russian: по-ру́сски", "По-ру́сски means in Russian. Now you can say what you speak and what you understand.", [
        ex("Понима'ть, to understand, goes like чита'ть: я понима'ю, ты понима'ешь. По-ру'сски and по-англи'йски name the language.", [("Я говорю' по-ру'сски.", "I speak Russian."), ("Я понима'ю.", "I understand."), ("Я не понима'ю.", "I don't understand."), ("понима'ть", "to understand"), ("ты понима'ешь", "you understand"), ("по-англи'йски", "in English")]),
        lp("Я не понима'ю.", ["Я понима'ю.", "Я не понима'ю.", "Я не зна'ю."]), tl("I understand", "я понима'ю"),
        tl("in Russian", "по-ру'сски"), tl("to understand", "понима'ть"), fl("Я говорю' ", "по-англи'йски", ".", "I speak English."), dc("Ты понима'ешь?"),
        ra("Я немно'го говорю' по-ру'сски."), cp("по-ру'сски"),
        rm("Ты понима'ешь по-ру'сски?", ["Do you understand Russian?", "Do you speak English?", "I understand Russian."]),
    ]),
    ("Not: не and нет", "Не negates the word after it; нет is a whole no.", [
        ex("Не sits right before the word it negates. Нет answers a question all by itself.", [("Нет, я не зна'ю.", "No, I don't know."), ("Э'то не чай.", "This isn't tea."), ("Ме'дленно, пожа'луйста.", "Slowly, please.")]),
        fl("Он ", "не", " говори'т по-англи'йски.", "He doesn't speak English."),
        fl("Ты зна'ешь? ", "Нет", ", не зна'ю.", "Do you know? No, I don't."),
        lp("Нет, спаси'бо.", ["Нет, спаси'бо.", "Да, спаси'бо.", "Спаси'бо."]), tl("No, thank you.", "Нет, спаси'бо."),
        dc("Э'то не мой чай."), ra("Я не чита'ю, я ду'маю."), tl("slowly", "ме'дленно"),
        rm("Мы не зна'ем, что э'то.", ["We don't know what this is.", "We know what this is.", "They don't know who this is."]),
    ]),
]))

U.append(("u13", "At the table", "до́брое у́тро · прия́тного аппети́та · пожа́луйста", [
    ("Good morning", "The phrases that open and close the day.", [
        ex("До'брое у'тро in the morning, до'брый ве'чер in the evening, споко'йной но'чи at bedtime.", [("До'брое у'тро!", "Good morning!"), ("До'брый ве'чер!", "Good evening!"), ("Споко'йной но'чи!", "Good night!")]),
        tr("до'брое у'тро"), lp("До'брый ве'чер!", ["До'брое у'тро!", "До'брый ве'чер!", "Споко'йной но'чи!"]),
        tl("good morning", "до'брое у'тро"), cp("до'брый ве'чер", 1), dc("Споко'йной но'чи!"),
        ra("До'брое у'тро! Как дела'?"), tl("good evening", "до'брый ве'чер"),
        rm("До'брое у'тро! Ко'фе?", ["Good morning! Coffee?", "Good evening! Tea?", "Good night! Coffee?"]),
    ]),
    ("Enjoy your meal", "Прия́тного аппети́та is what you say as the food arrives. Then: is it good?", [
        ex("Прия'тного аппети'та is enjoy your meal; the reply is спаси'бо, и тебе'. Вку'сно means it's tasty.", [("Прия'тного аппети'та!", "Enjoy your meal!"), ("Спаси'бо, и тебе'!", "Thanks, you too!"), ("О'чень вку'сно!", "Very tasty!")]),
        tr("прия'тного аппети'та"), lp("вку'сно", ["вку'сно", "о'чень", "ещё"]), tl("tasty", "вку'сно"),
        dc("О'чень вку'сно!"), ra("Прия'тного аппети'та! Спаси'бо, и тебе'!"),
        fl("", "О'чень", " вку'сно!", "Very tasty!"), tl("very", "о'чень"), cp("Спаси'бо, и тебе'!", 1),
    ]),
    ("Please, and more", "Пожа́луйста is please and also you're welcome. Ещё asks for more.", [
        ex("Пожа'луйста is said pa-ZHA-lu-sta, often just pa-ZHAL-sta. It is also the answer to спаси'бо.", [("Ещё ко'фе, пожа'луйста.", "More coffee, please."), ("Спаси'бо! — Пожа'луйста.", "Thank you! — You're welcome."), ("Мо'жно ещё?", "May I have more?"), ("Переда'й соль.", "Pass the salt.")]),
        tr("пожа'луйста"), lp("Мо'жно ещё?", ["Мо'жно ещё?", "Ещё ко'фе, пожа'луйста.", "О'чень вку'сно!"]),
        tl("please", "пожа'луйста"), tl("May I have more?", "Мо'жно ещё?"), dc("Ещё ко'фе, пожа'луйста."),
        ra("Переда'й, пожа'луйста, соль."), fl("Переда'й, пожа'луйста, ", "хлеб", ".", "Pass the bread, please."),
        rm("Спаси'бо! — Пожа'луйста.", ["Thank you! — You're welcome.", "Please! — Thank you.", "More! — Please."]),
    ]),
]))

U.append(("u14", "Breakfast and dinner", "за́втрак, у́жин, хочу́", [
    ("Breakfast and dinner", "Two meals, two words, and the question what's for dinner?", [
        ex("За'втрак is breakfast, у'жин is dinner. Что на у'жин? asks what's for dinner.", [("Что на за'втрак?", "What's for breakfast?"), ("На у'жин суп.", "There's soup for dinner."), ("За'втрак гото'в.", "Breakfast is ready."), ("обе'д", "lunch")]),
        tr("за'втрак"), lp("у'жин", ["у'жин", "за'втрак", "обе'д"]), tl("breakfast", "за'втрак"), tl("dinner", "у'жин"), tl("lunch", "обе'д"),
        fl("Что на ", "у'жин", "?", "What's for dinner?"), dc("На за'втрак ка'ша."), ra("У'жин гото'в! На у'жин суп и хлеб."),
        rm("У'жин гото'в!", ["Dinner is ready!", "Breakfast is ready!", "Is dinner tasty?"]),
    ]),
    ("I want: хочу́", "Хоте́ть, to want, is irregular, but two forms do most of the work: хочу́ and хо́чешь.", [
        ex("Я хочу' is I want; хо'чешь? is do you want? What you want comes after: хочу' ко'фе.", [("Я хочу' ко'фе.", "I want coffee."), ("Хо'чешь ко'фе?", "Do you want coffee?"), ("Нет, не хочу'.", "No, I don't."), ("ты хо'чешь", "you want")]),
        tr("я хочу'"), lp("Хо'чешь ко'фе?", ["Хо'чешь ко'фе?", "Хочу' ко'фе.", "Где ко'фе?"]), tl("I want", "я хочу'"),
        fl("Я ", "хочу'", " суп.", "I want soup."), dc("Ты хо'чешь суп?"), ra("Хо'чешь ко'фе? Да, хочу', спаси'бо."),
        tl("you (one person) want", "ты хо'чешь"),
        rm("Нет, не хочу', спаси'бо.", ["No, I don't want any, thanks.", "Yes, I want some, thanks.", "No, I don't know, thanks."]),
    ]),
    ("Table talk", "A whole little meal in Russian, from good morning to thank you.", [
        dc("До'брое у'тро!"), tl("enjoy your meal", "прия'тного аппети'та"),
        fl("О'чень ", "вку'сно", ", спаси'бо!", "Very tasty, thank you!"),
        lp("Переда'й, пожа'луйста, соль.", ["Переда'й, пожа'луйста, соль.", "Переда'й, пожа'луйста, хлеб.", "Мо'жно ещё?"]),
        ra("До'брый ве'чер! Что на у'жин?"), tl("thank you", "спаси'бо"),
        fl("Спаси'бо, и ", "тебе'", "!", "Thanks, you too!"), cp("Хорошо', спаси'бо.", 1),
        ps("— До'брое у'тро! Ко'фе?\n— Да, пожа'луйста.\n— Вот ко'фе, хлеб и сыр.\n— Спаси'бо! О'чень вку'сно.\n— Ещё ко'фе?\n— Нет, спаси'бо.",
           "“Good morning! Coffee?” “Yes, please.” “Here's coffee, bread and cheese.” “Thank you! Very tasty.” “More coffee?” “No, thanks.”",
           {"доброе": "good (доброе утро: good morning)"}),
        rm("— Ещё ко'фе? — Нет, спаси'бо.", ["“More coffee?” “No, thanks.”", "“More coffee?” “Yes, please.”", "“Is the coffee tasty?” “No.”"]),
    ]),
    ("Coffee with milk", "С is with; мне is for me. Two small words for every breakfast.", [
        ex("С means with, and the word after it changes: молоко', с молоко'м; са'хар, с са'харом. Мне is for me: мне чай, пожа'луйста.",
           [("ко'фе с молоко'м", "coffee with milk"), ("чай с са'харом", "tea with sugar"), ("са'хар", "sugar"), ("Мне чай, пожа'луйста.", "Tea for me, please.")]),
        tr("с молоко'м"), lp("с са'харом", ["с са'харом", "с молоко'м", "са'хар"]), tl("with milk", "с молоко'м"), tl("sugar", "са'хар"),
        fl("Ко'фе ", "с молоко'м", "?", "Coffee with milk?"), fl("", "Мне", " чай, пожа'луйста.", "Tea for me, please."),
        dc("Мне ко'фе с молоко'м, пожа'луйста."), ra("Чай с са'харом? Да, пожа'луйста."),
        rm("Мне ко'фе, а тебе' чай?", ["Coffee for me, and tea for you?", "Tea for me, and coffee for you?", "Do you want coffee or tea?"]),
    ]),
    ("What would you say?", "Moments at the table. Each has one phrase from these pages that fits: write it, then check.", [
        tl("Your housemate offers you coffee. Say yes, please.", "Да, пожа'луйста."),
        tl("You'd like more porridge. Ask whether you may have more.", "Мо'жно ещё?"),
        lp("Хо'чешь чай?", ["Хо'чешь чай?", "Хо'чешь ко'фе?", "Хочу' чай."]),
        tl("Someone thanks you. Say you're welcome.", "пожа'луйста"),
        fl("— Прия'тного аппети'та! — Спаси'бо, и ", "тебе'", "!", "Enjoy your meal! — Thanks, you too!"),
        tl("You didn't catch that. Ask for it once more, please.", "Ещё раз, пожа'луйста."),
        ra("Хо'чешь ко'фе? Нет, спаси'бо, я хочу' чай."),
        dc("Переда'й, пожа'луйста, хлеб."),
        tl("Dinner is ready. Announce it.", "У'жин гото'в!"),
    ]),
]))

U.append(("u15", "Numbers", "1–10", [
    ("One to five", "Оди́н, два, три, четы́ре, пять. Count something on the table.", [
        ex("Оди'н changes with gender like мой: оди'н стол, одна' ча'шка.", [("оди'н", "one"), ("два", "two"), ("три", "three"), ("четы'ре", "four"), ("пять", "five"), ("два плюс три", "two plus three")]),
        tr("оди'н, два, три"), lp("три", ["два", "три", "пять"]), tl("four", "четы'ре"), dc("пять"), cp("четы'ре"),
        ra("Оди'н, два, три, четы'ре, пять."), tl("two", "два"),
        rm("Два плюс три — пять.", ["Two plus three is five.", "Two plus two is four.", "Three plus three is six."]),
    ]),
    ("Six to ten", "Шесть, семь, во́семь, де́вять, де́сять. The soft sign ends every one of them.", [
        ex("Each of these ends in a soft т or м: listen for the little y-colour at the end.", [("шесть", "six"), ("семь", "seven"), ("во'семь", "eight"), ("де'вять", "nine"), ("де'сять", "ten")]),
        tr("шесть, семь"), lp("во'семь", ["семь", "во'семь", "де'вять"]), tl("ten", "де'сять"), dc("де'вять"), cp("во'семь"),
        ra("Шесть, семь, во'семь, де'вять, де'сять."), tl("seven", "семь"),
        rm("Пять плюс пять — де'сять.", ["Five plus five is ten.", "Five plus four is nine.", "Six plus four is ten."]),
    ]),
    ("Counting from memory", "All ten, out of order. Write the Russian before you say it.", [
        tl("three", "три"), tl("eight", "во'семь"), tl("six", "шесть"), tl("one", "оди'н"),
        lp("де'сять", ["де'вять", "де'сять", "шесть"]), dc("Три плюс четы'ре — семь."),
        fl("Два плюс два — ", "четы'ре", ".", "Two plus two is four."),
        ra("Де'сять, де'вять, во'семь, семь, шесть, пять, четы'ре, три, два, оди'н!"),
        rm("Шесть плюс два — во'семь.", ["Six plus two is eight.", "Six plus two is seven.", "Six plus three is eight."]),
    ]),
    ("Eleven to twenty", "Eleven to nineteen are a number on ten: оди́ннадцать is one-on-ten.", [
        ex("Eleven to nineteen are the number plus -на'дцать; два becomes две, and a final ь drops: двена'дцать, пятна'дцать. The stress falls on that на except in оди'ннадцать and четы'рнадцать. Twenty is два'дцать.",
           [("оди'ннадцать", "eleven"), ("двена'дцать", "twelve"), ("пятна'дцать", "fifteen"), ("девятна'дцать", "nineteen"), ("два'дцать", "twenty")]),
        lp("двена'дцать", ["два", "двена'дцать", "два'дцать"]), tr("трина'дцать"),
        tl("eleven", "оди'ннадцать"), tl("twelve", "двена'дцать"), tl("twenty", "два'дцать"), dc("пятна'дцать"),
        fl("Де'сять плюс пять — ", "пятна'дцать", ".", "Ten plus five is fifteen."),
        ra("Оди'ннадцать, двена'дцать, трина'дцать, четы'рнадцать, пятна'дцать, шестна'дцать, семна'дцать, восемна'дцать, девятна'дцать, два'дцать."),
        rm("Де'сять плюс де'вять — девятна'дцать.", ["Ten plus nine is nineteen.", "Ten plus nine is ninety.", "Nine plus nine is eighteen."]),
    ]),
]))

U.append(("c1", "Keeping up", "я не понима́ю · ещё раз · как по-ру́сски?", [
    ("When you don't understand", "Phrases that keep a conversation going when you are lost. Learn them whole; the grammar inside comes later.", [
        ex("Я не понима'ю is I don't understand. Ещё раз, пожа'луйста asks for it once more, and поме'дленнее, пожа'луйста asks for it slower.",
           [("Я не понима'ю.", "I don't understand."), ("Ещё раз, пожа'луйста.", "Once more, please."), ("Поме'дленнее, пожа'луйста.", "Slower, please.")]),
        tr("я не понима'ю"), lp("Ещё раз, пожа'луйста.", ["Ещё раз, пожа'луйста.", "Поме'дленнее, пожа'луйста.", "Я не понима'ю."]),
        dc("Поме'дленнее, пожа'луйста."), tl("I don't understand", "я не понима'ю"),
        ex("Как э'то по-ру'сски? asks for a word; что зна'чит…? asks what one means.", [("Как э'то по-ру'сски?", "How do you say this in Russian?"), ("Что зна'чит «соль»?", "What does соль mean?")]),
        fl("", "Как", " э'то по-ру'сски?", "How do you say this in Russian?"), fl("Что ", "зна'чит", " «соль»?", "What does «sol'» mean?"),
        ra("Я не понима'ю. Ещё раз, пожа'луйста."), cp("Как э'то по-ру'сски?", 1),
    ]),
]))

U.append(("u16", "First readings", "ро́бот, число́, мысль, звезда́", [
    ("Robots and rockets", "Tiny readings from here on. Read each one for its meaning first; then say it and write it.", [
        ex("Science fiction is kind to beginners: many of its words are already familiar.", [("ко'смос", "space"), ("пла'нета", "planet"), ("раке'та", "rocket")]),
        rm("Ро'бот ду'мает.", ["The robot is thinking.", "The robot is reading.", "The robot knows."]),
        tl("space (outer space)", "ко'смос"), ra("Э'то ро'бот. Он ду'мает и говори'т."), dc("Ро'бот зна'ет, где раке'та."),
        lp("пла'нета", ["пла'нета", "раке'та", "звезда'"]),
        fl("Раке'та лети'т в ", "ко'смос", ".", "The rocket is flying into space."), tl("planet", "пла'нета"),
        cp("Раке'та лети'т.", 1),
    ]),
    ("Numbers and facts", "Mathematics in Russian is short and exact.", [
        ex("Число' is a number, зада'ча a problem to solve. Пра'вда is truth, or simply that's true.", [("число'", "number"), ("зада'ча", "problem, task"), ("Э'то пра'вда.", "That's true."), ("Э'то непра'вда.", "That's not true.")]),
        rm("Два плюс два — четы'ре. Э'то факт.", ["Two plus two is four. That's a fact.", "Two plus two is five. That's not true.", "Two plus four is six. That's true."]),
        tl("number", "число'"), dc("Э'то пра'вда?"), ra("Пять — э'то число'. Э'то факт."),
        fl("Два плюс два — пять? Нет, э'то ", "непра'вда", ".", "Two plus two is five? No, that's not true."),
        tl("problem, task", "зада'ча"), lp("зада'ча", ["зада'ча", "пра'вда", "число'"]), cp("Э'то не факт.", 1),
    ]),
    ("Thoughts", "Мысль is thought. Philosophy, in two famous sentences.", [
        ex("Descartes and Socrates, in Russian. Ничего' is said nichevo.", [("мысль", "thought"), ("Я мы'слю, сле'довательно, я существу'ю.", "I think, therefore I am."), ("Я зна'ю, что ничего' не зна'ю.", "I know that I know nothing.")]),
        tr("мысль"), tl("thought", "мысль"), lp("ничего'", ["ничего'", "число'", "что"]), dc("Я зна'ю, что ничего' не зна'ю."),
        ra("Я мы'слю, сле'довательно, я существу'ю."),
        fl("Я зна'ю, ", "что", " ничего' не зна'ю.", "I know that I know nothing."),
        rm("Мысль — э'то не факт.", ["A thought is not a fact.", "A fact is not a thought.", "A thought is a fact."]),
        tl("nothing", "ничего'"),
    ]),
    ("Stars", "One last reading, about the night sky.", [
        ex("Ночь is night, and большо'й is big. Звёзды is stars.", [("ночь", "night"), ("большо'й", "big"), ("звёзды", "stars")]),
        rm("Ночь. Звёзды. Ко'смос о'чень большо'й.", ["Night. Stars. Space is very big.", "Morning. The sun. Space is very small.", "Night. A rocket. The world is big."]),
        tl("night", "ночь"), tl("big", "большо'й"), dc("Звёзды там."), ra("Ночь. Звёзды. Ко'смос о'чень большо'й."),
        fl("Мир о'чень ", "большо'й", ".", "The world is very big."), lp("звёзды", ["звёзды", "звезда'", "ночь"]),
        cp("Мы не одни'.", 1),
    ]),
]))


import course_families  # noqa: E402
import course_read  # noqa: E402
import course_reading  # noqa: E402
U += course_reading.units(dict(meet=meet, ex=ex, tr=tr, cp=cp, lp=lp, dc=dc, ra=ra, tl=tl, fl=fl, rm=rm, ps=ps))
LEXICON += course_reading.LEXICON
U += course_families.units(dict(ex=ex, tr=tr, cp=cp, lp=lp, dc=dc, ra=ra, tl=tl, fl=fl, rm=rm))
LEXICON += course_families.LEXICON
U += course_read.units(dict(ex=ex, tr=tr, cp=cp, lp=lp, dc=dc, ra=ra, tl=tl, fl=fl, rm=rm, ps=ps))
LEXICON += course_read.LEXICON


# Course order. Page ids are written into learners' notebooks (sourceID), and the app walks
# pages by index, so: ids come from where a page was first authored (uN-pK, counting only
# pages without an explicit id), inserted pages carry an explicit id as a 4-tuple, and moving
# a page or a unit happens here rather than by editing its position in U. Never insert or move
# anything before the earliest page a learner may not have reached yet.
UNIT_ORDER = ["u1", "u2", "u3", "u4", "u5", "u6", "u7", "u13", "c1", "u14", "u8", "u15", "u9", "u10",
              "u11", "u18", "u12", "u16", "u17", "u19", "u20", "u21", "u22", "u23",
              "u24", "u25", "u26", "u27", "u28", "u29", "u30"]
# (page id, into unit, before page id or None for the end)
PAGE_MOVES = [
    ("u19-p1", "u17", "u17-p1"),  # today/yesterday/tomorrow before the past tense
    ("u22-p1", "u12", None),      # что-clauses and why/because as everyday tools
    ("u22-p2", "u12", None),
]

# --- the diary line -----------------------------------------------------------
# From the third unit on, every page ends with a Дневник line: a prompt in English, on a topic
# whose words the page (or an earlier one) has taught, and ruled lines for the learner's own
# Russian, which the tutor corrects in red. The prompt is never spoken, so it needs no audio.
DIARY_FROM_UNIT = 2  # index into UNIT_ORDER
DIARY_PROMPTS = {
    "u3-p1": "What is this, and what is it not? Point at two things: Э'то… Э'то не…",
    "u3-p2": "Name three things in the room with э'то.",
    "u3-p3": "Who is this? Introduce two people or animals.",
    "u4-p1": "Who is at home right now, and who is not?",
    "u4-p2": "Ask where two things are, and answer.",
    "u4-p3": "What is here and what is there? Three short sentences.",
    "u4-p4": "A tiny dialogue: who is there, where is the cat?",
    "u5-p1": "Who is in your family? Name them.",
    "u5-p2": "Three things that are yours, with мой, моя' or моё.",
    "u5-p3": "What do you love? Two or three things.",
    "u6-p1": "What is this? Ask and answer about two things, and say what you are reading.",
    "u6-p2": "What is on the table at home? Use наш and то'же.",
    "u6-p3": "How are things? Answer, and say what there is more of.",
    "u7-p1": "Who is in your family, and what do you know?",
    "u7-p2": "Coffee, a street, a fact: three sentences with the new words.",
    "u7-p3": "Say hi to someone, ask how they are, say bye.",
    "u7-p4": "Your favourite word so far, and why you like it (in Russian as far as you can).",
    "u7-p5": "Write about bread, cheese and tea on a table.",
    "u13-p1": "Good morning: what you say when you wake up, and what you say at night.",
    "u13-p2": "Dinner is on the table. What do you say, and is it good?",
    "u13-p3": "Ask for more of something, politely, and say thank you.",
    "c1-p1": "You didn't understand. Write what you would say.",
    "u14-p1": "What is for breakfast today? And for dinner?",
    "u14-p2": "What do you want right now? Two sentences with хочу'.",
    "u14-p3": "A whole little meal: from good morning to thank you.",
    "u14-p4": "Order coffee or tea the way you like it.",
    "u14-p5": "What did you eat today?",
    "u8-p1": "Three things around you: is each он, она' or оно'? Say where each is.",
    "u8-p2": "What is on your table? Each thing with мой, моя' or моё.",
    "u8-p3": "Describe the room: this is…, this is not…",
    "u8-p4": "Count what you see: cups, spoons, books, windows.",
    "u8-p5": "Describe three things as new or old.",
    "u15-p1": "How many of what is on your table? Count to five.",
    "u15-p2": "Count the things in your room, up to ten.",
    "u15-p3": "Three small sums, written out in words.",
    "u15-p4": "How old are the people and pets at home? Numbers to twenty.",
    "u9-p1": "Ask three questions with кто and что.",
    "u9-p2": "Ask someone how they are and where something is; answer both.",
    "u9-p3": "Two yes/no questions and their answers.",
    "u9-p4": "When is breakfast, and how many cups are there? Ask and answer.",
    "u10-p1": "Whose things are these? Yours and mine.",
    "u10-p2": "Describe your home with наш and на'ша.",
    "u10-p3": "What do you have at home? Three sentences with у меня' есть.",
    "u10-p4": "What is there none of today? Use у меня' нет.",
    "u11-p1": "What are you reading today? Who else is reading?",
    "u11-p2": "What do we read, and what do they think?",
    "u11-p3": "What do you know, and what don't you know?",
    "u11-p4": "Three people, three verbs: who reads, who thinks, who knows.",
    "u18-p1": "Where is everything? In the house, on the table.",
    "u18-p2": "Where are you right now? In the city, at home, in space?",
    "u18-p3": "What do you see from where you sit?",
    "u18-p4": "A robot looks out of a window. What does it see?",
    "u12-p1": "Who speaks in your home, and what do they say?",
    "u12-p2": "Which languages do you speak and understand, and how well?",
    "u12-p3": "Three things that are not so: не and нет.",
    "u22-p1": "What do you think? Two sentences with я ду'маю, что…",
    "u22-p2": "Why do you learn Russian? Answer with потому' что.",
    "u16-p1": "A robot and a rocket: a two-sentence story.",
    "u16-p2": "A fact about numbers, in Russian.",
    "u16-p3": "A thought you had today, in one sentence.",
    "u16-p4": "The night sky, in two or three sentences.",
    "u19-p1": "Your day: morning, afternoon, evening. What happens when?",
    "u17-p1": "Where were you yesterday? Who was with you?",
    "u17-p2": "What did you do yesterday? Three sentences in the past.",
    "u17-p2a": "One thing you finished yesterday, and one you only did for a while.",
    "u17-p3": "A tiny story about yesterday, in three sentences.",
    "u17-p4": "What did you do today? Ask the same of someone.",
    "u17-p5": "What will you do tomorrow, and what will you get done?",
    "u19-p2": "Where are you going right now, and where is someone else going?",
    "u19-p3": "Where do you go every day, and how do you get there?",
    "u19-p4": "Where did you go today, and how?",
    "u20-p1": "Describe a station in space in three sentences.",
    "u20-p2": "You hear numbers in the noise. Write what they are and what you think.",
    "u20-p3": "Write a short message to send into space.",
    "u20-p4": "A month later: what happened? Two or three sentences.",
    "u21-p1": "Three numbers over twenty, written out, with a sum.",
    "u21-p2": "An even number, an odd number, and an if-then sentence.",
    "u21-p3": "One sentence with для любо'го and one with существу'ет.",
    "u21-p3a": "State a tiny mathematical claim in one clause.",
    "u21-p4": "Your own two-line proof of something simple.",
    "u22-p3": "A question that is not simple, and a first attempt at an answer.",
    "u22-p4": "Something you know, and something you don't, in your own words.",
}


DIARY_PROMPTS.update(course_read.DIARY)


def add_diary_lines(units):
    for uid in UNIT_ORDER[DIARY_FROM_UNIT:]:
        for page in units[uid]["pages"]:
            if any(e["kind"] == "diary" for e in page["exercises"]):
                continue
            page["exercises"].append(dy(DIARY_PROMPTS.get(page["id"], "Two sentences of your own, with words from this page.")))


def build():
    units = {}
    for uid, title, subtitle, pages in U:
        unit = {"id": uid, "title": title, "subtitle": subtitle, "pages": []}
        n = 0
        for page in pages:
            if len(page) == 4:
                pid, ptitle, intro, exercises = page
            else:
                n += 1
                pid, (ptitle, intro, exercises) = f"{uid}-p{n}", page
            unit["pages"].append({"id": pid, "title": ptitle, "intro": intro, "exercises": exercises})
        units[uid] = unit
    assert sorted(UNIT_ORDER) == sorted(units), set(UNIT_ORDER) ^ set(units)
    for pid, into, before in PAGE_MOVES:
        src = next(u for u in units.values() if any(pg["id"] == pid for pg in u["pages"]))
        page = next(pg for pg in src["pages"] if pg["id"] == pid)
        src["pages"].remove(page)
        dest = units[into]["pages"]
        dest.insert(next(i for i, pg in enumerate(dest) if pg["id"] == before) if before else len(dest), page)
    add_diary_lines(units)
    return deep({"version": 1, "letters": LETTERS, "lexicon": LEXICON, "units": [units[u] for u in UNIT_ORDER]})


if __name__ == "__main__":
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(build(), ensure_ascii=False, indent=1) + "\n")
    print(OUT)
