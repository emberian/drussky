"""Pure admission checks for NEW speech only; never rewrite archived source/proofs."""
from pathlib import Path
import re

ACUTE = "\u0301"
AMBIGUOUS = frozenset((Path(__file__).resolve().parents[2] / "crates/rufren-speech/assets/known_ambiguous_forms.txt").read_text(encoding="utf-8").splitlines())
VOWELS = frozenset("аеёиоуыэюяАЕЁИОУЫЭЮЯ")


def validate_generation_text(text):
    if re.search(r"[А-Яа-яЁё]", text) and any(c.isnumeric() for c in text):
        raise ValueError("Russian numeric glyphs need explicitly authored spoken words")
    for token in re.findall(r"[А-Яа-яЁё\u0301]+", text):
        if token.replace(ACUTE, "").lower() not in AMBIGUOUS:
            continue
        marks = 0
        previous = None
        for c in token:
            if c == ACUTE:
                marks += 1
                if marks > 1 or previous not in VOWELS:
                    raise ValueError("Unsupported acute placement on known ambiguous token")
            previous = c
        if not marks:
            raise ValueError("Every known ambiguous token needs its own intended acute")
