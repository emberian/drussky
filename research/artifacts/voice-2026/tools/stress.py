"""Stress-notation conversions for the voice audition (mirrors apple/scripts/audio_store.py).

The corpus authors stress as Silero's '+' before the stressed vowel. Each model gets the notation
it understands:
  plus   '+' before the vowel            Silero, ESpeech (F5-TTS Russian)
  acute  U+0301 after the vowel          OmniVoice, VoxCPM2 (honoured only partly; measured here)
  plain  marks removed                   the "no stress control" condition
"""
import re
import unicodedata

VOWELS = "аеёиоуыэюяАЕЁИОУЫЭЮЯ"
ACUTE = "́"


def plus_to_acute(text):
    return re.sub(rf"\+([{VOWELS}])", lambda m: m.group(1) + ACUTE, text)


def plus_to_plain(text):
    return re.sub(rf"\+([{VOWELS}])", r"\1", text)


def acute_to_plus(text):
    return re.sub(rf"([{VOWELS}]){ACUTE}", r"+\1", text)


def spoken(text):
    """What a transcript should contain: no marks, no punctuation, lower case, ё folded to е."""
    text = unicodedata.normalize("NFC", plus_to_plain(text).replace(ACUTE, ""))
    text = text.lower().replace("ё", "е")
    return " ".join(re.sub(r"[^\w\s]", " ", text).split())


def convert(text, notation):
    return {"plus": text, "acute": plus_to_acute, "plain": plus_to_plain}[notation](text) if notation != "plus" else text
