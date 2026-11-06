"""Pure lexical screening after independent ASR; no acoustic judgments.

The target text is deliberately outside the recognition worker. This helper can
drive a bounded reference replacement or a learner listen/confirm/retry action.
"""
import re
import unicodedata


def lexical_tokens(text):
    # Remove pedagogical acute accents, but preserve й and the lexical ё/е distinction.
    text = unicodedata.normalize("NFC", unicodedata.normalize("NFD", text).replace("\u0301", ""))
    return re.findall(r"[^\W_]+", text.casefold(), flags=re.UNICODE)


def token_differences(expected, heard):
    """Minimum-edit token alignment; repeated words retain sequence positions."""
    a, b = lexical_tokens(expected), lexical_tokens(heard)
    if len(a) > 256 or len(b) > 256:
        raise ValueError("Too many words for a short-utterance screen")
    costs = [[0] * (len(b) + 1) for _ in range(len(a) + 1)]
    for i in range(len(a) + 1): costs[i][0] = i
    for j in range(len(b) + 1): costs[0][j] = j
    for i in range(1, len(a) + 1):
        for j in range(1, len(b) + 1):
            costs[i][j] = min(costs[i - 1][j] + 1, costs[i][j - 1] + 1,
                              costs[i - 1][j - 1] + (a[i - 1] != b[j - 1]))
    differences, i, j = [], len(a), len(b)
    while i or j:
        if i and j and costs[i][j] == costs[i - 1][j - 1] + (a[i - 1] != b[j - 1]):
            if a[i - 1] != b[j - 1]:
                differences.append({"kind": "substitution", "expected_index": i - 1, "heard_index": j - 1,
                                    "expected": a[i - 1], "heard": b[j - 1],
                                    "yo_spelling_ambiguity": a[i - 1].replace("ё", "е") == b[j - 1].replace("ё", "е")})
            i -= 1; j -= 1
        elif i and costs[i][j] == costs[i - 1][j] + 1:
            differences.append({"kind": "omission", "expected_index": i - 1, "heard_index": None,
                                "expected": a[i - 1], "heard": None}); i -= 1
        else:
            differences.append({"kind": "addition", "expected_index": None, "heard_index": j - 1,
                                "expected": None, "heard": b[j - 1]}); j -= 1
    return list(reversed(differences))


def plan_next_action(expected_text, evidence, *, mode, replacement_candidates_used=0):
    if (mode not in ("generated_reference", "learner_attempt")
            or type(replacement_candidates_used) is not int or replacement_candidates_used not in (0, 1)
            or not isinstance(expected_text, str) or not expected_text.strip() or len(expected_text) > 8192):
        raise ValueError("Invalid short lexical review")
    if (evidence.get("schema_version") != "rufren-speech-transcript-v1"
            or evidence.get("target_conditioned") is not False
            or evidence.get("evidence_kind") != "transcript_hypothesis"
            or evidence.get("language") != "ru"
            or not isinstance(evidence.get("transcript"), str)):
        raise ValueError("Requires independent Russian transcript evidence")
    # A missing/bad recording binding cannot be repaired by a model's prose.
    if (not re.fullmatch(r"[0-9a-f]{64}", evidence.get("audio_sha256", ""))
            or not isinstance(evidence.get("asset_id"), str) or not evidence["asset_id"]):
        raise ValueError("Missing exact audio binding")
    span, shape = evidence.get("span"), evidence.get("original_audio", {})
    if (not isinstance(span, dict) or set(span) != {"start_frame", "end_frame"}
            or any(type(span[k]) is not int for k in span)
            or type(shape.get("frames")) is not int
            or not 0 <= span["start_frame"] < span["end_frame"] <= shape["frames"]):
        raise ValueError("Invalid replay span")
    differences = token_differences(expected_text, evidence["transcript"])
    if not differences:
        action = "continue_with_new_meaningful_phrase"
    elif (mode == "generated_reference" and replacement_candidates_used == 0
          and any(not d.get("yo_spelling_ambiguity", False) for d in differences)):
        action = "request_one_alternative_reference_candidate"
    else:
        action = "listen_confirm_meaning_and_retry"
    return {"schema_version": "rufren-lexical-review-v1", "mode": mode,
            "asset_id": evidence["asset_id"], "audio_sha256": evidence["audio_sha256"],
            "independent_transcript": evidence["transcript"], "lexical_differences": differences,
            "lexical_screen": "mismatch_needs_review" if differences else "no_lexical_mismatch",
            "primary_action": action, "automatic_replacement_budget_remaining": max(0, 1 - replacement_candidates_used),
            "replay": {"span": dict(span), "coordinate_system": "original_frames_half_open",
                       "localization": "selected_crop_not_word_alignment"},
            "pronunciation_verdict": None, "reference_phonetics_verified": False,
            "next_step": ("Compare the displayed words, replay this phrase, and try once more."
                          if differences else "The words were recovered. Try the next phrase with a new meaning.")}
