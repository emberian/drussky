#!/usr/bin/env python3
"""Build credits and exhaustive licence inventories, offline and with the standard library.

    python3 apple/scripts/build_credits.py
    python3 apple/scripts/build_credits.py --check

Reads provenance; never changes it. --check checks freshness, NOT legal clearance.
Missing authors/licences remain explicit. Model licences are not output licences.
credits_evidence.json holds reviewed supplements, including recovered translation IDs.
No local caches, network, clock, git, audio decoder or third-party imports are used.
"""
import argparse
import ast
from collections import Counter, defaultdict
import csv
import hashlib
import io
import json
from pathlib import Path
import sys
from urllib.parse import quote

REPO = Path(__file__).resolve().parents[2]
RES = REPO / "apple/Drussky/Resources"
SCRIPTS = REPO / "apple/scripts"
FOUNDATION = REPO / "crates/rufren-app/assets/foundations/manifest.json"
MODEL_NOTE = (
    "Drussky's original lessons, explanations and exercises were drafted with language models "
    "and edited by the project. The project reports Gemini 4 Argon and GPT (Codex) drafts for "
    "its original stories and mathematics material; those optional collections are not present "
    "in this audited resource snapshot. Claude (Anthropic) generates the live tutor's replies. "
    "This is a statement of creative process, not a claim of exclusively human authorship or "
    "a third-party content licence. Imported texts, recordings, reference material and fonts "
    "retain the separate rights and credits listed here."
)
LICENSES = {
    "CC BY 2.0 fr": ("CC-BY-2.0-FR", "https://creativecommons.org/licenses/by/2.0/fr/"),
    "CC BY 2.0 FR": ("CC-BY-2.0-FR", "https://creativecommons.org/licenses/by/2.0/fr/"),
    "CC BY 2.5": ("CC-BY-2.5", "https://creativecommons.org/licenses/by/2.5/"),
    "CC BY 3.0": ("CC-BY-3.0", "https://creativecommons.org/licenses/by/3.0/"),
    "CC BY 3.0 us": ("CC-BY-3.0-US", "https://creativecommons.org/licenses/by/3.0/us/"),
    "CC BY 3.0 US": ("CC-BY-3.0-US", "https://creativecommons.org/licenses/by/3.0/us/"),
    "CC BY 4.0": ("CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
    "CC BY-SA 2.5": ("CC-BY-SA-2.5", "https://creativecommons.org/licenses/by-sa/2.5/"),
    "CC BY-SA 3.0": ("CC-BY-SA-3.0", "https://creativecommons.org/licenses/by-sa/3.0/"),
    "CC BY-SA 4.0": ("CC-BY-SA-4.0", "https://creativecommons.org/licenses/by-sa/4.0/"),
    "CC0": ("CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/"),
    "CC0 1.0": ("CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/"),
    "Public domain": ("public-domain", None),
}


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def digest(text):
    return hashlib.sha256(text.encode()).hexdigest()


def licence(raw):
    return LICENSES.get(raw, ("unknown", None))


def csv_text(rows):
    out = io.StringIO(newline="")
    writer = csv.DictWriter(out, fieldnames=list(rows[0]), lineterminator="\n")
    writer.writeheader()
    writer.writerows(rows)
    return out.getvalue()


def reference_words():
    """Read literal reference keys without importing/executing the validator."""
    words = set()
    for node in ast.parse((SCRIPTS / "validate_paradigms.py").read_text()).body:
        if isinstance(node, ast.Assign) and any(
            isinstance(t, ast.Name) and t.id.startswith("REFERENCE_") for t in node.targets
        ) and isinstance(node.value, ast.Dict):
            words.update(ast.literal_eval(k) for k in node.value.keys)
    return sorted(words)


def build():
    evidence = read(SCRIPTS / "credits_evidence.json")
    library = read(RES / "library.json")
    foundation = {a["id"]: a.get("provenance", a.get("source_response", {}).get("provenance", {}))
                  for a in read(FOUNDATION)["assets"]}
    records, manifests, inputs = {}, {}, []
    for name in ("sources.json", "sources-pairs.json", "sources-narration.json"):
        path = RES / "Audio" / name
        data = read(path)
        inputs.append(path)
        manifests[name] = dict(sorted(Counter(v.get("license") or "MISSING" for v in data.values()).items()))
        for file, source in data.items():
            if file in records:
                raise ValueError(f"Duplicate clip across manifests: {file}")
            records[file] = (name, source)

    # Include every provenance record, even clips not reachable through an index: the
    # entire Audio folder is a bundle resource. References are checked independently.
    references = defaultdict(set)
    for path in sorted((RES / "Audio").glob("index*.json")) + sorted((RES / "Audio").glob("variants*.json")):
        inputs.append(path)
        for value in read(path).values():
            for file in value if isinstance(value, list) else [value]:
                if isinstance(file, str):
                    references[file].add(path.name)
    for s in library["sentences"]:
        references[s["audio"]].add("library.json:sentences")
    for t in library["texts"]:
        if isinstance(t.get("narration"), str):
            references[t["narration"]].add("library.json:texts")
    missing_sources = sorted(set(references) - set(records))

    groups = {kind: [] for kind in ["human_audio", "narration", "synthetic_audio", "tatoeba_text",
                                   "fonts", "reference_material", "models_and_algorithms", "foundation_audio"]}
    rows, source_groups, authors = [], {}, defaultdict(list)
    synthetic = defaultdict(list)

    def human_entry(asset, source, group):
        raw_author = source.get("reader") or source.get("author")
        credit = source.get("credit")
        supplement = evidence["commons_credit_overrides"].get(asset, {})
        author = supplement.get("author") or raw_author or credit
        credit = supplement.get("credit") or credit
        incomplete = not author or author.startswith("not stated")
        author = author or "Contributor not recorded"
        raw_license = source.get("license")
        ident, url = licence(raw_license)
        title = source.get("title") or source.get("key") or asset
        page = source.get("source_page")
        change = source.get("processing") or source.get("pipeline") or "Processing not recorded."
        required = ident.startswith("CC-BY")
        attribution = f"{title} — {author}."
        if credit:
            attribution += f" {credit}"
        attribution += f" Source: {page or 'not recorded'}. {raw_license or 'Licence not recorded'}"
        if url:
            attribution += f" ({url})"
        attribution += f". Changes by the Drussky project: {change}"
        key = (group, page, author, raw_license, title, credit, change)
        if key not in source_groups:
            entry = {"id": "recording-" + digest(json.dumps(key, ensure_ascii=False))[:16],
                     "title": title, "author": author, "recorded_author": raw_author,
                     "source_url": page, "license": ident, "recorded_license": raw_license,
                     "license_url": url, "attribution_required": required,
                     "share_alike": "-SA-" in ident, "attribution": attribution,
                     "credit_status": "incomplete" if incomplete or not page or ident == "unknown" else "ready",
                     "changes": change, "assets": []}
            if supplement:
                entry["credit_evidence"] = supplement
            source_groups[key] = entry
            groups[group].append(entry)
        source_groups[key]["assets"].append(asset)
        authors[author].append(asset)
        return author, incomplete, source_groups[key]["id"]

    for file, (manifest, source) in sorted(records.items()):
        raw = source.get("license")
        ident, url = licence(raw)
        kind = source.get("kind")
        model_license = ""
        author, incomplete, credit_id = "", False, ""
        if kind == "human_recording":
            author, incomplete, credit_id = human_entry(file, source, "narration" if manifest == "sources-narration.json" else "human_audio")
        elif kind == "ai_generated_speech":
            original = foundation.get(Path(source.get("from", "")).stem, {})
            if "Silero" in source.get("provider", "") or original.get("provider") == "local_silero_tts":
                model_license = "CC-BY-NC-SA-4.0"
                synthetic["silero"].append(file)
                credit_id = "silero-v5-ru"
            else:
                synthetic["unknown"].append(file)
        elif kind == "macos_tts":
            synthetic["apple"].append(file)
            credit_id = "apple-system-voice"
        rows.append({"file": file, "manifest": manifest, "kind": kind,
                     "recorded_license": raw or "", "normalized_license": ident,
                     "missing_license_field": not bool(raw), "model_license": model_license,
                     "attribution_required": ident.startswith("CC-BY"), "share_alike": "-SA-" in ident,
                     "model_attribution_precaution": bool(model_license),
                     "recorded_author": source.get("reader") or source.get("author") or "",
                     "credited_author": author, "incomplete_author": incomplete,
                     "source_page": source.get("source_page", ""), "credit_id": credit_id,
                     "referenced_by": ";".join(sorted(references[file])),
                     "foundation_asset": source.get("from", "")})

    silero = evidence["silero"]
    groups["synthetic_audio"].append({
        "id": "silero-v5-ru", "title": "Silero Russian speech", "author": "Silero Team",
        "source_url": "https://github.com/snakers4/silero-models", "license": "CC-BY-NC-SA-4.0",
        "license_scope": "model and upstream code; output licence is not established by the model licence alone",
        "license_url": "https://creativecommons.org/licenses/by-nc-sa/4.0/",
        "model": silero["model"], "model_sha256": silero["model_sha256"],
        "source_revision": silero["source_revision"], "credit_status": "ready",
        "attribution_required": True,
        "requirement_note": "Model use credit retained as a precaution for generated audio; output licence unresolved.",
        "output_license": None,
        "attribution": "Speech generated using Silero v5_6_ru by the Silero Team, principally voice xenia. "
                       "Model: CC BY-NC-SA 4.0 (https://creativecommons.org/licenses/by-nc-sa/4.0/). "
                       "Source: https://github.com/snakers4/silero-models. Drussky supplies the text and stress; "
                       "audio is trimmed, levelled and encoded as AAC. This model credit does not assert "
                       "that every generated recording is a derivative of the model.",
        "assets": synthetic["silero"],
        "repository_assets": [f"crates/rufren-app/assets/foundations/{fid}.wav"
                              for fid, p in sorted(foundation.items()) if p.get("provider") == "local_silero_tts"]})
    if synthetic["apple"]:
        groups["synthetic_audio"].append({
            "id": "apple-system-voice", "title": "Apple system voice", "author": "Apple",
            "license": "Apple macOS SLA, section 2F", "license_url": "https://www.apple.com/legal/sla/docs/macOSTahoe.pdf",
            "credit_status": "personal-use-only", "attribution_required": False,
            "attribution": "The е recording uses Apple's macOS Milena system voice, trimmed, levelled and AAC encoded. "
                           "For personal, non-commercial use; this credit grants no public redistribution rights.",
            "assets": synthetic["apple"]})

    for sentence in sorted(library["sentences"], key=lambda s: s["source"]):
        sid = sentence["source"].rsplit("/", 1)[-1]
        supplement = evidence["tatoeba"].get(sid, {})
        for lang in ("ru", "en"):
            if supplement and supplement[lang + "_sha256"] != digest(sentence[lang]):
                raise ValueError(f"Stale Tatoeba {lang} evidence for {sid}")
        en_id = supplement.get("en_id")
        en_url = f"https://tatoeba.org/en/sentences/show/{en_id}" if en_id else None
        ru_authors, en_authors = supplement.get("ru_contributors", []), supplement.get("en_contributors", [])
        complete = bool(ru_authors and en_authors and en_id)
        attribution = (f"Tatoeba: Russian sentence {sid}, by {', '.join(ru_authors) or 'contributors whose names are not yet recorded'} "
                       f"({sentence['source']}); English translation {en_id or 'ID not recorded'}, "
                       f"by {', '.join(en_authors) or 'contributors whose names are not yet recorded'} "
                       f"({en_url or 'source not recorded'}). CC BY 2.0 FR "
                       "(https://creativecommons.org/licenses/by/2.0/fr/). Texts selected and paired by Drussky.")
        groups["tatoeba_text"].append({
            "id": "tatoeba-" + sid, "title": sentence["ru"], "ru_id": sid, "en_id": en_id,
            "source_url": sentence["source"], "translation_source_url": en_url,
            "ru_contributors": ru_authors, "en_contributors": en_authors,
            "license": "CC-BY-2.0-FR", "license_url": "https://creativecommons.org/licenses/by/2.0/fr/",
            "attribution_required": True, "credit_status": "ready" if complete else "incomplete",
            "attribution": attribution, "audio_asset": sentence["audio"],
            "note": "Audio speaker is credited separately and is not presumed to be the text author."})

    for font in read(RES / "Fonts/fonts.json"):
        path = RES / "Fonts" / font["licenceFile"]
        full = path.read_text(encoding="utf-8")
        if "SIL OPEN FONT LICENSE Version 1.1" not in full or "DISCLAIMER" not in full:
            raise ValueError(f"Missing or incomplete OFL: {path}")
        notice = full.split("\n\n", 1)[0].replace("\n", " ")
        inputs.extend([path, RES / "Fonts" / font["file"]])
        groups["fonts"].append({
            "id": "font-" + font["postscript"], "title": font["family"], "author": notice,
            "source_url": font["source"], "license": "OFL-1.1", "license_url": "https://openfontlicense.org/open-font-license-official-text/",
            "license_file": "Fonts/" + font["licenceFile"], "license_text": full,
            "attribution_required": True, "credit_status": "ready",
            "attribution": f"{font['family']}. {notice} Licensed under the SIL Open Font License, Version 1.1. "
                           f"Source: {font['source']}. The complete licence follows.",
            "assets": ["Fonts/" + font["file"]]})

    words = reference_words()
    groups["reference_material"].append({
        "id": "wiktionary", "title": "Wiktionary", "author": "Wiktionary contributors",
        "source_url": "https://en.wiktionary.org/", "license": "CC-BY-SA-4.0",
        "license_url": "https://creativecommons.org/licenses/by-sa/4.0/", "attribution_required": True,
        "credit_status": "needs-provenance-review", "share_alike": True,
        "attribution": "Russian reference forms and word origins: Wiktionary contributors, CC BY-SA 4.0 "
                       "(https://creativecommons.org/licenses/by-sa/4.0/). Drussky's reference tables were "
                       "selected and transcribed with its stress notation; morphology was selected and rephrased "
                       "for learners. Individual entry and contributor-history links follow. Exact imported "
                       "revisions and the boundary of copied expression remain to be documented.",
        "references": [{"lemma": w, "source_url": "https://en.wiktionary.org/wiki/" + quote(w) + "#Russian",
                        "history_url": "https://en.wiktionary.org/w/index.php?title=" + quote(w) + "&action=history"}
                       for w in words],
        "assets": ["apple/scripts/validate_paradigms.py:REFERENCE_*", "paradigms.json", "morphology.json"],
        "note": "Reference URLs are reconstructed from literal validator keys, not claims of pinned revisions. "
                "morphology_sources.md also cites ru.wiktionary.org; per-entry citations are still needed."})
    groups["reference_material"].append({
        "id": "cursive-references", "title": "Russian cursive references",
        "author": "В. Г. Горецкий; Н. А. Федосова; Wikipedia contributors",
        "credit_status": "needs-provenance-review", "attribution_required": True,
        "requirement_note": "Conditional on copying protected expression; retained as a precaution.",
        "attribution": "School handwriting methodology consulted: В. Г. Горецкий and Н. А. Федосова, Прописи "
                       "(Просвещение); Wikipedia contributors, Russian cursive and Русский рукописный шрифт. "
                       "Drussky draws its own paths. Wikipedia text is CC BY-SA 4.0; the workbook is not openly licensed.",
        "source_urls": ["https://en.wikipedia.org/wiki/Russian_cursive", "https://ru.wikipedia.org/wiki/Русский_рукописный_шрифт"],
        "license_url": "https://creativecommons.org/licenses/by-sa/4.0/", "assets": ["strokes-cursive.json"]})
    groups["models_and_algorithms"].extend([
        {"id": "zipa", "title": "ZIPA small CR-CTC", "author": "Jian Zhu and ZIPA collaborators; AnySpeech",
         "license": "CC-BY-NC-SA-4.0", "license_url": "https://creativecommons.org/licenses/by-nc-sa/4.0/",
         "source_url": "https://huggingface.co/anyspeech/zipa-small-crctc-ns-700k",
         "revision": "797523ffb49e31e821a972313813cc4a26d902ef", "attribution_required": True,
         "credit_status": "ready", "applies_when": "ZipaPhones model is bundled; absent in audit checkout",
         "attribution": "ZIPA: A family of efficient models for multilingual phone recognition — Jian Zhu and "
                        "collaborators (2025). AnySpeech zipa-small-crctc-ns-700k, CC BY-NC-SA 4.0 "
                        "(https://creativecommons.org/licenses/by-nc-sa/4.0/). "
                        "Source: https://huggingface.co/anyspeech/zipa-small-crctc-ns-700k. "
                        "Drussky converts the ONNX model to Core ML with a fixed 10-second window and fp16 "
                        "computation; the default build also quantizes weights to 8 bits. Retain this licence on the model."},
        {"id": "fsrs", "title": "FSRS-5", "author": "Open Spaced Repetition / Ye and collaborators",
         "source_url": "https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Algorithm",
         "credit_status": "ready", "attribution_required": False,
         "attribution": "Memory scheduling uses FSRS-5, developed by the Open Spaced Repetition community "
                        "(Ye and collaborators), implemented in Swift by Drussky from the published equations "
                        "and default parameters. https://github.com/open-spaced-repetition/fsrs4anki/wiki/The-Algorithm"}])

    foundation_rows = []
    for fid, source in sorted(foundation.items()):
        ident, _ = licence(source.get("license"))
        asset = f"crates/rufren-app/assets/foundations/{fid}.wav"
        if source.get("kind") == "human_recording":
            human_entry(asset, {**source, "title": fid, "pipeline": "Foundation WAV; processing not itemized in this manifest."}, "foundation_audio")
        foundation_rows.append({"asset": asset, "kind": source.get("kind", ""),
                                "recorded_license": source.get("license", ""), "normalized_license": ident,
                                "missing_license_field": not bool(source.get("license")),
                                "model_license": "CC-BY-NC-SA-4.0" if source.get("provider") == "local_silero_tts" else "",
                                "attribution_required": ident.startswith("CC-BY"), "share_alike": "-SA-" in ident,
                                "credit": source.get("credit", ""), "source_page": source.get("source_page", ""),
                                "used_by_app": ";".join(r["file"] for r in rows if r["foundation_asset"] == asset)})

    inputs += [RES / "library.json", RES / "Fonts/fonts.json", RES / "morphology.json", RES / "paradigms.json",
               SCRIPTS / "validate_paradigms.py", SCRIPTS / "credits_evidence.json", FOUNDATION]
    credits = {
        "version": 1, "title": "Исто́чники · Sources", "audit_date": evidence["reviewed_on"],
        "generated_by": "apple/scripts/build_credits.py", "model_authored_content": MODEL_NOTE,
        "status": "incomplete-attribution; see research/LICENSES.md",
        "rendering_note": "Render attribution, source and licence links, changes, and font licence texts. "
                          "Do not hide incomplete credit_status entries or display this manifest as legal clearance. "
                          "foundation_audio is repository-only unless its asset is used by the app; ZIPA is conditional.",
        "input_sha256": {str(p.relative_to(REPO)): hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(set(inputs))},
        "counts": {"audio_files": len(rows), "by_manifest_recorded_license": manifests,
                   "recorded_license": dict(sorted(Counter(r["recorded_license"] or "MISSING" for r in rows).items())),
                   "attribution_required_audio": sum(r["attribution_required"] for r in rows),
                   "share_alike_audio": sum(r["share_alike"] for r in rows),
                   "missing_license_field_audio": sum(r["missing_license_field"] for r in rows),
                   "silero_audio": len(synthetic["silero"]), "macos_audio": len(synthetic["apple"]),
                   "incomplete_human_authors": sum(r["incomplete_author"] for r in rows)},
        "unresolved": {"audio_references_without_provenance": missing_sources,
                       "unidentified_synthesis": synthetic["unknown"],
                       "human_audio_without_author_credit": [r["file"] for r in rows if r["incomplete_author"]],
                       "tatoeba_text_missing_contributors": [e["id"] for e in groups["tatoeba_text"] if e["credit_status"] != "ready"]},
        "authors": [{"name": author, "assets": sorted(set(assets))} for author, assets in sorted(authors.items())],
        "groups": [{"kind": kind, "entries": sorted(entries, key=lambda e: e["id"])} for kind, entries in groups.items()],
    }
    return {RES / "credits.json": json.dumps(credits, ensure_ascii=False, indent=2) + "\n",
            REPO / "research/LICENSES-audio.csv": csv_text(rows),
            REPO / "research/LICENSES-foundation.csv": csv_text(foundation_rows)}, credits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="compare generated bytes without writing")
    args = parser.parse_args()
    outputs, credits = build()
    stale = []
    for path, text in outputs.items():
        if args.check:
            if not path.exists() or path.read_bytes() != text.encode():
                stale.append(str(path.relative_to(REPO)))
        else:
            path.write_text(text, encoding="utf-8")
    print(json.dumps(credits["counts"], ensure_ascii=False, indent=2))
    print("Unresolved: " + json.dumps({k: len(v) for k, v in credits["unresolved"].items()}))
    if stale:
        sys.exit("Stale outputs: " + ", ".join(stale))
    print("Generated bytes match." if args.check else "Wrote credits.json and both exhaustive CSV inventories.")


if __name__ == "__main__":
    main()
