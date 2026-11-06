"""Auditable authored-stress to official Silero '+' input syntax; no model imports."""
import hashlib
import re
import sys
import unicodedata
sys.dont_write_bytecode = True

from silero_model import MAX_TEXT_BYTES, MAX_TEXT_CHARS

VOWELS = frozenset("аеёиоуыэюяАЕЁИОУЫЭЮЯ")
BINDING = re.compile(r"[A-Za-z0-9_.:-]{1,128}\Z")


def sha256(text):
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def validate_request(request):
    if not isinstance(request, dict) or set(request) - {"text", "request_id", "revision_id"}:
        raise ValueError("Invalid source-bound speech request")
    text = request.get("text")
    request_id = request.get("request_id")
    revision_id = request.get("revision_id")
    if (not isinstance(text, str) or not text.strip() or len(text) > MAX_TEXT_CHARS
            or len(text.encode("utf-8")) > MAX_TEXT_BYTES or not any(c.isalpha() for c in text)
            or any(unicodedata.category(c) == "Cc" and c not in "\n\t" for c in text)
            or any(c in text for c in "<>+") or "[[" in text
            or not isinstance(request_id, str) or not BINDING.fullmatch(request_id)
            or revision_id is not None and (not isinstance(revision_id, str) or not BINDING.fullmatch(revision_id))):
        raise ValueError("Invalid short text or exact request/revision binding")
    return {"text": text, "request_id": request_id, "revision_id": revision_id}


def synthesis_input(text):
    validate_request({"text": text, "request_id": "frontend-validation", "revision_id": None})
    output = []
    converted = 0
    for index, character in enumerate(text):
        if character == "\u0301":
            # No normalization of ё or other characters: remove only the authored
            # acute and add Silero's control immediately BEFORE that exact vowel.
            if index == 0 or text[index - 1] not in VOWELS or not output or output[-1] != text[index - 1]:
                raise ValueError("Place one U+0301 acute directly after a Russian vowel")
            vowel = output.pop(); output.extend(("+", vowel)); converted += 1
        elif unicodedata.category(character).startswith("M"):
            raise ValueError("Unsupported combining mark in explicit stress input")
        else:
            output.append(character)
    synthesis_text = "".join(output)
    return {
        "source_text_sha256": sha256(text),
        "synthesis_text": synthesis_text,
        "synthesis_text_sha256": sha256(synthesis_text),
        "transform": "pedagogical_acute_to_silero_plus" if converted else "verbatim",
        "context": "explicit_stress" if converted else "ordinary_text",
    }
