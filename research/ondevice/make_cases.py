#!/usr/bin/env python3
"""Builds cases.json for the on-device tutor benchmark (research/ONDEVICE_TUTOR.md).

  python3 research/ondevice/make_cases.py   # writes research/ondevice/cases.json

Glosses come from the course's passage glosses (the reference meaning in context), lemmas from
the hand-labelled goal-lemmas.json fixture, the ending questions and diary lines are authored here
with their keys.
"""
import json
import re
import unicodedata
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
COURSE = ROOT / "apple/Drussky/Resources/course.json"
LEMMAS = ROOT / "apple/DrusskyTests/goal-lemmas.json"
OUT = Path(__file__).with_name("cases.json")

STRESS = "́"


def key(s: str) -> str:
    return unicodedata.normalize("NFC", s.replace(STRESS, "")).lower().replace("ё", "е")


def gloss_cases(limit=50):
    course = json.loads(COURSE.read_text())
    out = []
    per_passage = 5
    for unit in course["units"]:
        for page in unit.get("pages", []):
            for ex in page.get("exercises", []):
                glosses = ex.get("glosses")
                if not glosses:
                    continue
                text = ex["text"]
                sentences = [s.strip() for s in re.split(r"(?<=[.!?…])\s+|\n", text) if s.strip()]
                n = 0
                for word, meaning in glosses.items():
                    ctx = next((s for s in sentences if key(word) in key(s)), text.replace("\n", " "))
                    out.append({"id": f"gloss-{len(out) + 1}", "word": word, "context": ctx, "reference": meaning,
                                "page": page.get("id")})
                    n += 1
                    if n >= per_passage:
                        break
    return out[:limit]


ENDINGS = [
    ("Я живу́ в Москве́.", "Москве́", "Prepositional case after в for a location (where). Москва is feminine, -а becomes -е."),
    ("Я чита́ю кни́гу.", "кни́гу", "Accusative case, the direct object of читать. Feminine -а becomes -у."),
    ("У меня́ нет вре́мени.", "вре́мени", "Genitive case after нет (absence). Время is a neuter -мя noun; its genitive is времени."),
    ("Он даёт кни́гу бра́ту.", "бра́ту", "Dative case: the recipient of giving. Masculine nouns take -у."),
    ("Мы идём в шко́лу.", "шко́лу", "Accusative case after в with a verb of motion (destination, where to). Feminine -а becomes -у."),
    ("Она́ пи́шет ру́чкой.", "ру́чкой", "Instrumental case: the tool or means of doing it. Feminine -а becomes -ой."),
    ("Я ви́жу ма́му.", "ма́му", "Accusative direct object. Feminine -а becomes -у."),
    ("Я ви́жу бра́та.", "бра́та", "Accusative direct object; for animate masculine nouns the accusative looks like the genitive, so брат takes -а."),
    ("Я ви́жу стол.", "стол", "Accusative direct object; for inanimate masculine nouns the accusative equals the nominative, so no ending changes."),
    ("Э́то кни́га ма́мы.", "ма́мы", "Genitive case for possession (whose book). Feminine -а becomes -ы."),
    ("Он говори́т о Росси́и.", "Росси́и", "Prepositional case after о (about). Nouns ending in -ия take -ии, not -ие."),
    ("У меня́ пять книг.", "книг", "Genitive plural after the numbers five and up. Feminine -а nouns drop the ending (zero ending)."),
    ("На столе́ два стола́.", "стола́", "Genitive singular after the numbers два/две, три, четыре. Masculine nouns take -а."),
    ("Я рабо́таю.", "рабо́таю", "First person singular present of работать (an -ать verb, first conjugation): stem работа- plus -ю."),
    ("Ты рабо́таешь.", "рабо́таешь", "Second person singular present, first conjugation ending -ешь."),
    ("Они́ говоря́т.", "говоря́т", "Third person plural present of говорить, a second conjugation (-ить) verb, so -ят (not -ют)."),
    ("Она́ чита́ла.", "чита́ла", "Past tense: -л plus -а because the subject (она) is feminine; the past tense agrees in gender, not person."),
    ("Э́то но́вая кни́га.", "но́вая", "Adjective agreeing with the feminine singular nominative noun книга: feminine ending -ая."),
    ("Мне хо́лодно.", "Мне", "Dative case: the experiencer in an impersonal construction (it is cold to me). Dative of я is мне."),
    ("Я был в Петербу́рге.", "Петербу́рге", "Prepositional case after в for a location. Masculine nouns add -е."),
]


DIARIES = [
    ("What do you like to do?", "Я люблю читать книга.", "Я люблю́ чита́ть кни́ги.",
     [{"span": "книга", "replacement": ["книги", "книгу"], "category": "ending"}]),
    ("Tell me about your family.", "Моя папа работает дома.", "Мой па́па рабо́тает до́ма.",
     [{"span": "Моя", "replacement": ["мой"], "category": "agreement"}]),
    ("What did you do yesterday?", "Вчера я ходил в школа.", "Вчера́ я ходи́л в шко́лу.",
     [{"span": "школа", "replacement": ["школу"], "category": "ending"}]),
    ("Where do you live?", "Я живу в Москва.", "Я живу́ в Москве́.",
     [{"span": "Москва", "replacement": ["москве"], "category": "ending"}]),
    ("What is your friend doing?", "Она читает книга и пьёт чай.", "Она́ чита́ет кни́гу и пьёт чай.",
     [{"span": "книга", "replacement": ["книгу"], "category": "ending"}]),
    ("Tell me about your brother.", "У меня есть брат. Он студент. Он учится в университет.",
     "У меня́ есть брат. Он студе́нт. Он у́чится в университе́те.",
     [{"span": "университет", "replacement": ["университете"], "category": "ending"}]),
    ("What is happening at home?", "Мама и папа дома. Они смотрит телевизор.", "Ма́ма и па́па до́ма. Они́ смо́трят телеви́зор.",
     [{"span": "смотрит", "replacement": ["смотрят"], "category": "agreement"}]),
    ("What is hard today?", "Я не понимаю этот слово.", "Я не понима́ю э́то сло́во.",
     [{"span": "этот", "replacement": ["это"], "category": "agreement"}]),
    ("What are your plans?", "Сегодня хорошая погода. Я иду в парк с друг.", "Сего́дня хоро́шая пого́да. Я иду́ в парк с дру́гом.",
     [{"span": "друг", "replacement": ["другом"], "category": "ending"}]),
    ("Introduce yourself.", "Меня зовут Эмбер. Я люблю математику и кофе.", "Меня́ зову́т Э́мбер. Я люблю́ матема́тику и ко́фе.", []),
]


def lemma_cases(limit=40):
    s = json.loads(LEMMAS.read_text())
    tokens = s["tokens"]
    seen, out = set(), []
    for i, (tok, gold) in enumerate(tokens):
        if tok in seen:
            continue
        seen.add(tok)
        out.append({"id": f"lemma-{len(out) + 1}", "form": tok, "gold": gold.split("|")})
    # every k-th distinct token, so the sample spans all three texts
    step = max(1, len(out) // limit)
    return out[::step][:limit]


def main():
    cases = {
        "glosses": gloss_cases(),
        "endings": [{"id": f"ending-{i + 1}", "sentence": s, "word": w, "key": k} for i, (s, w, k) in enumerate(ENDINGS)],
        "diaries": [{"id": f"diary-{i + 1}", "prompt": p, "text": t, "gold": g, "planted": e}
                    for i, (p, t, g, e) in enumerate(DIARIES)],
        "lemmas": lemma_cases(),
    }
    OUT.write_text(json.dumps(cases, ensure_ascii=False, indent=1))
    print({k: len(v) for k, v in cases.items()}, "->", OUT)


if __name__ == "__main__":
    main()
