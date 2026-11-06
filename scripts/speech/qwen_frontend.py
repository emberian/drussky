"""Versioned conservative input-shape policy; no pronunciation certification."""
import hashlib
import re

ACUTE = "\u0301"
AMBIGUOUS = frozenset(("замок", "мука", "плачу", "атлас", "орган", "духи", "ирис", "белки", "стрелки", "кружки", "стоит", "дороги", "вести", "пропасть", "село"))
VOWELS = frozenset("аеёиоуыэюяАЕЁИОУЫЭЮЯ")


class UnsupportedStress(ValueError):
    pass


def digest(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def synthesis_input(text):
    if ACUTE not in text:
        return {"source_text_sha256": digest(text), "synthesis_text": text,
                "synthesis_text_sha256": digest(text), "transform": "verbatim", "context": "ordinary_text"}
    words = re.findall(r"[А-Яа-яЁё\u0301]+", text)
    if len(words) < 8 or not text.rstrip().endswith((".", "?", "!", "…")):
        raise UnsupportedStress("Marked short phrases need an explicit-stress voice")
    seen = {}
    for word in words:
        if ACUTE not in word:
            continue
        base, position, previous = "", None, None
        for character in word:
            if character == ACUTE:
                if position is not None or previous not in VOWELS:
                    raise UnsupportedStress("Unsupported acute mark placement")
                position = len(base) - 1
            else:
                base += character.lower()
            previous = character
        if base in AMBIGUOUS or base in seen and seen[base] != position:
            raise UnsupportedStress("A marked stress contrast needs an explicit-stress voice")
        seen[base] = position
    actual = text.replace(ACUTE, "")
    return {"source_text_sha256": digest(text), "synthesis_text": actual,
            "synthesis_text_sha256": digest(actual), "transform": "remove_combining_acute_in_context",
            "context": "contextual_utterance"}
