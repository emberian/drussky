#!/usr/bin/env python3
"""Recover who to credit: Tatoeba sentence owners and Wikimedia Commons recording authors.

    python3 apple/scripts/fetch_provenance.py tatoeba fetch   # cache every sentence (≤2 req/s)
    python3 apple/scripts/fetch_provenance.py tatoeba apply   # library.json: ru_author, en_author, licences
    python3 apple/scripts/fetch_provenance.py commons fetch   # cache the page text of clips without an author
    python3 apple/scripts/fetch_provenance.py commons apply   # sources.json + human_audio.json: author, credit

fetch only writes the cache under apple/.build/{tatoeba,commons}/ and can be interrupted and
resumed; apply re-reads its targets right before writing and only fills fields the cache
supports. A sentence or clip the network could not answer for keeps its fields absent, so
build_credits.py keeps flagging it. Standard library only.
"""
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
RES = APPLE / "Drussky/Resources"
CACHE = APPLE / ".build"
UA = {"User-Agent": "DrusskyCredits/1.0 (https://ember.software; personal language-learning app) python-urllib"}
PAUSE = 0.6  # seconds between requests: under 2 per second
TATOEBA = "https://tatoeba.org/en/api_v0/sentence/"
COMMONS = "https://commons.wikimedia.org/w/api.php"
SHTOOKA = "The Shtooka Project (individual speaker not identified)"
SHTOOKA_CREDIT = "Audio sample provided by The Shtooka Project."


def get(url):
    req = urllib.request.Request(url, headers=UA)
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read().decode("utf-8")


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write(path, data):
    path.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")


def sentence_ids():
    """Every Russian id in library.json and the English id the evidence file pairs it with."""
    evidence = read(APPLE / "scripts/credits_evidence.json")["tatoeba"]
    ids = []
    for s in read(RES / "library.json")["sentences"]:
        rid = s["source"].rsplit("/", 1)[-1]
        ids.append(rid)
        en = evidence.get(rid, {}).get("en_id")
        if en:
            ids.append(en)
    return sorted(set(ids), key=int)


def tatoeba_fetch():
    cache = CACHE / "tatoeba"
    cache.mkdir(parents=True, exist_ok=True)
    ids = sentence_ids()
    todo = [i for i in ids if not (cache / f"{i}.json").exists()]
    print(f"{len(ids)} sentences, {len(todo)} to fetch")
    for n, sid in enumerate(todo, 1):
        try:
            text = get(TATOEBA + sid)
        except urllib.error.HTTPError as e:
            print(f"  {sid}: HTTP {e.code}", flush=True)
            (cache / f"{sid}.error").write_text(str(e.code))
        else:
            (cache / f"{sid}.json").write_text(text, encoding="utf-8")
        if n % 25 == 0:
            print(f"  {n}/{len(todo)}", flush=True)
        time.sleep(PAUSE)
    print("done")


def tatoeba_apply():
    cache = CACHE / "tatoeba"
    evidence = read(APPLE / "scripts/credits_evidence.json")["tatoeba"]
    lib_path = RES / "library.json"
    lib = read(lib_path)
    filled = {"ru_author": 0, "en_author": 0}
    for s in lib["sentences"]:
        rid = s["source"].rsplit("/", 1)[-1]
        en_id = evidence.get(rid, {}).get("en_id")
        for lang, sid in (("ru", rid), ("en", en_id)):
            path = cache / f"{sid}.json"
            if not sid or not path.exists():
                continue
            d = read(path)
            if str(d.get("id")) != str(sid) or d.get("text") != s[lang]:
                print(f"  {sid}: cached text differs from library {lang!r}; left unfilled")
                continue
            owner = (d.get("user") or {}).get("username")
            if owner:
                s[f"{lang}_author"] = owner
                filled[f"{lang}_author"] += 1
            if d.get("license"):
                s[f"{lang}_license"] = d["license"]
            if lang == "en":
                s["en_source"] = f"https://tatoeba.org/en/sentences/show/{sid}"
    write(lib_path, lib)
    print(f"library.json: {filled['ru_author']} Russian authors, {filled['en_author']} English authors of {len(lib['sentences'])}")


def commons_targets():
    """Human clips whose author is missing or a placeholder, by Commons file title."""
    out = {}
    for name in ("sources.json", "sources-pairs.json", "sources-narration.json"):
        for file, s in read(RES / "Audio" / name).items():
            author = s.get("reader") or s.get("author")
            if s.get("kind") == "human_recording" and (not author or author.startswith("not stated")):
                page = s.get("source_page", "")
                if "commons.wikimedia.org/wiki/File:" in page:
                    title = urllib.parse.unquote(page.split("/wiki/", 1)[1]).replace("_", " ")
                    out.setdefault(title, []).append((name, file))
    return out


def commons_fetch():
    cache = CACHE / "commons"
    cache.mkdir(parents=True, exist_ok=True)
    titles = [t for t in commons_targets() if not (cache / (re.sub(r"[^\w.-]", "_", t) + ".json")).exists()]
    print(f"{len(titles)} Commons pages to fetch")
    for i in range(0, len(titles), 10):
        batch = titles[i:i + 10]
        q = urllib.parse.urlencode({"action": "query", "prop": "revisions|imageinfo", "rvprop": "content", "rvslots": "main",
                                    "iiprop": "extmetadata|user", "titles": "|".join(batch), "format": "json", "formatversion": "2"})
        data = json.loads(get(f"{COMMONS}?{q}"))
        for page in data["query"]["pages"]:
            (cache / (re.sub(r"[^\w.-]", "_", page["title"]) + ".json")).write_text(json.dumps(page, ensure_ascii=False, indent=1))
        time.sleep(PAUSE)
    print("done")


def commons_author(page):
    """(author, credit) a Commons page supports, or (None, None)."""
    text = page.get("revisions", [{}])[0].get("slots", {}).get("main", {}).get("content", "")
    if "Shtooka" in text:
        return SHTOOKA, SHTOOKA_CREDIT
    m = re.search(r"\|\s*author\s*=\s*(.+)", text)
    if m:
        raw = m.group(1).strip()
        link = re.match(r"\[(\S+)\s+([^\]]+)\]", raw)  # [url Name]
        wiki = re.match(r"\[\[(?:User:)?([^\]|]+)(?:\|([^\]]+))?\]\]", raw)  # [[User:Name|Name]]
        name = (link.group(2) if link else wiki.group(2) or wiki.group(1) if wiki else re.sub(r"\{\{.*?\}\}", "", raw)).strip()
        if name:
            return name, None
    meta = page.get("imageinfo", [{}])[0].get("extmetadata", {})
    artist = meta.get("Artist", {}).get("value")
    if artist:
        return re.sub(r"<[^>]+>", "", artist).strip() or None, None
    return None, None


def commons_apply():
    cache = CACHE / "commons"
    targets = commons_targets()
    found, missing = {}, []
    for title, clips in targets.items():
        path = cache / (re.sub(r"[^\w.-]", "_", title) + ".json")
        if not path.exists():
            missing.append(title)
            continue
        author, credit = commons_author(read(path))
        if author:
            found[title] = (author, credit, clips)
        else:
            missing.append(title)
    for name in ("sources.json", "sources-pairs.json", "sources-narration.json"):
        path = RES / "Audio" / name
        data = read(path)
        changed = 0
        for title, (author, credit, clips) in found.items():
            for manifest, file in clips:
                if manifest == name and file in data:
                    data[file]["author"] = author
                    if credit:
                        data[file]["credit"] = credit
                    data[file]["author_evidence"] = "Commons file page text, fetched by scripts/fetch_provenance.py"
                    changed += 1
        if changed:
            write(path, dict(sorted(data.items())))
            print(f"{name}: {changed} authors recorded")
    # The chosen-takes file feeds make_audio.py; keep it in step so a rebuild does not undo this.
    human_path = APPLE / "scripts/human_audio.json"
    human = read(human_path)
    by_title = {v["title"].replace("_", " "): k for k, v in human.items() if v.get("title")}
    changed = 0
    for title, (author, credit, _) in found.items():
        k = by_title.get(title.removeprefix("File:"))
        if k:
            human[k]["author"] = author
            if credit:
                human[k]["credit"] = credit
            changed += 1
    if changed:
        write(human_path, human)
        print(f"human_audio.json: {changed} authors recorded")
    for title in missing:
        print("  still without an author:", title, [f for _, f in targets[title]])
    print(f"{len(found)} of {len(targets)} Commons pages gave an author")


def main():
    actions = {("tatoeba", "fetch"): tatoeba_fetch, ("tatoeba", "apply"): tatoeba_apply,
               ("commons", "fetch"): commons_fetch, ("commons", "apply"): commons_apply}
    try:
        actions[tuple(sys.argv[1:3])]()
    except KeyError:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
