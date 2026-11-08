#!/usr/bin/env python3
"""The bundled audio is not in git: the manifests are, and this rebuilds the clips from them.

  python3 apple/scripts/audio.py status  [--plan]   # missing / extra clips; --plan: how each would be restored
  .local/silero-tts/venv/bin/python apple/scripts/audio.py restore [--dry-run] [--offline] [--only F ...]
  python3 apple/scripts/audio.py verify             # every referenced clip exists, decodes, has provenance
  python3 apple/scripts/audio.py annotate           # backfill restore fields into sources*.json (append-only)
  python3 apple/scripts/audio.py revoice [--engine voxcpm2|espeech] [--dest DIR] [--list L.json] [--only F ...]
                                                     # re-render the authored clips with the 2026 narrator (research/VOICE_2026.md)

Referenced clips are every file named in Audio/index*.json and Audio/variants*.json. Each has a
sources*.json entry (see audio_store.py for the restore fields). `restore` rebuilds each missing clip
from the first source that works:
  1. the original in the local cache (`orig`, checked against `orig_sha1`)
  2. a download of `orig_url` (Wikimedia Commons / Tatoeba), saved into the cache, SHA-1 checked
  3. re-synthesis of `input` with the recorded voice (Silero; a 2026 narrator engine in its own venv,
     via audio_store.Narrator, when `synth` names one; macOS `say` for one clip)
then trims, levels and encodes it exactly as the generators did (audio_store.encode). Downloaded and
human clips come out with identical audio; re-synthesized ones say the same thing but need not match
byte for byte. Files are written under the names the manifests reference.

Options: --audio-dir DIR (default Drussky/Resources/Audio), --cache-root DIR (where `.local/...` paths
live, default <repo>/.local), --offline, --max-fetches N, --max-synth N, --only FILE..., --limit N,
--assume-no-cache (with --dry-run/--plan: count what a fresh clone without caches would need).
Re-synthesis needs the Silero venv (.local/silero-tts, set up by scripts/speech/setup-silero.py).
"""
import argparse
import hashlib
import json
import sys
import tempfile
from collections import Counter
from pathlib import Path

APPLE = Path(__file__).resolve().parents[1]
REPO = APPLE.parent
sys.path.insert(0, str(APPLE / "scripts"))
import audio_store as store  # noqa: E402

DEFAULT_AUDIO = APPLE / "Drussky/Resources/Audio"


def load(audio, pattern):
    return {p.name: json.loads(p.read_text()) for p in sorted(audio.glob(pattern))}


def referenced(audio):
    """file -> key, for every clip a manifest points at."""
    out = {}
    for d in load(audio, "index*.json").values():
        out.update({f: k for k, f in d.items()})
    for d in load(audio, "variants*.json").values():
        out.update({f: k for k, fs in d.items() for f in fs})
    return out


def sources(audio):
    """file -> (sources file name, entry)."""
    return {f: (name, e) for name, d in load(audio, "sources*.json").items() for f, e in d.items()}


def local_path(orig, cache_root):
    p = Path(orig)
    return cache_root / Path(*p.parts[1:]) if p.parts[0] == ".local" else REPO / p


def method(entry, cache_root, args, assume_no_cache=False):
    """How a clip would be rebuilt: cache, network, synth, or None (with a reason)."""
    if "pipeline" not in entry or "orig" not in entry:
        return None, "no restore fields (run annotate)"
    synthetic = "input" in entry and "voice" in entry
    p = local_path(entry["orig"], cache_root)
    in_git = not entry["orig"].startswith(".local/")
    if (in_git or not assume_no_cache) and p.is_file():
        if synthetic or entry.get("orig_sha1") in (None, store.sha1(p)):
            return "cache", p
    if entry.get("orig_url") and not args.offline:
        return "network", p
    if synthetic:
        return "synth", p
    return None, f"original {entry['orig']} not cached" + (" (offline)" if entry.get("orig_url") else "")


def cmd_status(args):
    audio = args.audio_dir
    refs, src = referenced(audio), sources(audio)
    present = {p.name for p in audio.glob("*.m4a")}
    missing = sorted(set(refs) - present)
    extra = sorted(present - set(refs))
    print(f"{len(refs)} referenced clips: {len(refs) - len(missing)} present, {len(missing)} missing; {len(extra)} unreferenced .m4a")
    for f in extra:
        print("  extra", f)
    if args.plan:
        for label, assume in (("with the local caches", False), ("from a clone without caches", True)):
            counts = Counter(method(src[f][1], args.cache_root, args, assume)[0] if f in src else None for f in refs)
            print(f"restoring all {len(refs)} {label}: " + ", ".join(f"{k or 'impossible'} {n}" for k, n in sorted(counts.items(), key=str)))
    elif missing:
        print("  missing:", " ".join(missing[:20]) + (" ..." if len(missing) > 20 else ""))
    return 0


def cmd_restore(args):
    audio = args.audio_dir
    refs, src = referenced(audio), sources(audio)
    todo = sorted(args.only) if args.only else sorted(f for f in refs if not (audio / f).is_file())
    todo = todo[: args.limit] if args.limit else todo
    done, failed = Counter(), []
    fetches = synths = 0
    silero, narrators = None, {}
    with tempfile.TemporaryDirectory(prefix="drussky-level-") as level_dir:
        for n, f in enumerate(todo, 1):
            if f not in src:
                failed.append((f, "no sources entry"))
                continue
            entry = src[f][1]
            how, p = method(entry, args.cache_root, args, args.assume_no_cache)
            if how == "network" and args.max_fetches is not None and fetches >= args.max_fetches:
                how, p = ("synth", p) if "input" in entry else (None, "fetch budget spent")
            if how == "synth" and args.max_synth is not None and synths >= args.max_synth:
                how, p = None, "synthesis budget spent"
            if how is None:
                failed.append((f, p))
                continue
            done[how] += 1
            print(f"[{n}/{len(todo)}] {how:7} {f}", flush=True)
            if args.dry_run:
                continue
            try:
                if how == "network":
                    fetches += 1
                    store.fetch(entry["orig_url"], p, entry.get("orig_sha1"))
                    src_path = p
                elif how == "synth":
                    synths += 1
                    engine = store.narrator_engine(entry["synth"])
                    if entry["synth"] == store.MACOS_SETTINGS:
                        src_path = Path(level_dir) / (Path(f).stem + ".aiff")
                        store.macos_say(entry["input"], entry["voice"], src_path)
                    elif engine:
                        narrators[engine] = narrators.get(engine) or store.Narrator(engine)
                        narrators[engine].wav(entry["input"], p)
                        src_path = p
                    else:
                        silero = silero or store.Silero(threads=4)
                        data, _ = silero.wav(entry["input"], entry["voice"])
                        p.parent.mkdir(parents=True, exist_ok=True)
                        p.write_bytes(data)
                        src_path = p
                else:
                    src_path = p
                if "cut" in entry:  # a narration excerpt: the recorded stretch of the chapter, not the whole file
                    src_path = store.cut(src_path, entry["cut"]["start"], entry["cut"]["end"], Path(level_dir) / (Path(f).stem + "-cut.wav"))
                store.encode(src_path, audio / f, level_dir)
            except Exception as e:  # noqa: BLE001 -- report every clip, keep going
                done[how] -= 1
                failed.append((f, f"{how}: {e}"))
    for n in narrators.values():
        n.close()
    verb = "would restore" if args.dry_run else "restored"
    print(f"{verb} {sum(done.values())} of {len(todo)}: " + ", ".join(f"{k} {v}" for k, v in sorted(done.items())))
    for f, why in failed:
        print("  cannot restore", f, "-", why)
    return 1 if failed else 0


def cmd_revoice(args):
    """Re-render the authored (Silero xenia / narrator) clips with a 2026 narrator engine.

    Without --dest the clips in the audio dir are replaced and their sources*.json entries rewritten
    (provider/voice/synth/orig/orig_sha1), so `restore` regenerates them with the same engine. With
    --dest DIR nothing in the audio dir or the manifests changes: the levelled .m4a files and a
    sources-revoice.json go to DIR (a shadow set to judge by ear). Originals (48/24 kHz WAV) are kept
    under .local/tts-2026/originals/<engine>/.
    """
    audio = args.audio_dir
    src = sources(audio)
    todo = sorted(args.only) if args.only else sorted(f for f, (_, e) in src.items()
                                                      if "input" in e and e.get("voice") in ("xenia", store.NARRATOR_VOICE))
    if args.list:
        wanted = {x["file"] for x in json.loads(Path(args.list).read_text())}
        todo = [f for f in todo if f in wanted]
    todo = todo[: args.limit] if args.limit else todo
    dest = args.dest.resolve() if args.dest else None
    if dest:
        dest.mkdir(parents=True, exist_ok=True)
    originals = args.cache_root / "tts-2026/originals" / args.engine
    narrator = store.Narrator(args.engine)
    changed, failed, out_entries = Counter(), [], {}
    with tempfile.TemporaryDirectory(prefix="drussky-level-") as level_dir:
        for n, f in enumerate(todo, 1):
            name, entry = src[f]
            text = entry["input"]
            wav = originals / f"{Path(f).stem}-{hashlib.sha1(text.encode()).hexdigest()[:6]}.wav"
            print(f"[{n}/{len(todo)}] {args.engine:8} {f}  {text[:70]}", flush=True)
            if args.dry_run:
                continue
            try:
                if not wav.exists():
                    narrator.wav(text, wav)
                store.encode(wav, (dest or audio) / f, level_dir)
            except Exception as e:  # noqa: BLE001
                failed.append((f, str(e)))
                continue
            new = {**entry, "provider": store.NARRATOR_PROVIDER[args.engine]}
            for k in ("orig", "orig_sha1", "voice", "synth", "input", "pipeline"):
                new.pop(k, None)
            new.update(store.restore_fields(wav, synth=store.narrator_fields(text, args.engine)))
            if dest:
                out_entries[f] = new
            else:
                src[f] = (name, new)
                changed[name] += 1
    narrator.close()
    if args.dry_run:
        print(f"would re-voice {len(todo)} clips with {args.engine}")
        return 0
    if dest:
        (dest / "sources-revoice.json").write_text(json.dumps(out_entries, ensure_ascii=False, indent=1) + "\n")
        print(f"wrote {len(out_entries)} shadow clips to {dest}")
    else:
        for name in changed:
            d = {f: e for f, (nm, e) in src.items() if nm == name}
            (audio / name).write_text(json.dumps(d, ensure_ascii=False, indent=1) + "\n")
        print(f"re-voiced {sum(changed.values())} clips with {args.engine}: " + ", ".join(f"{k} {v}" for k, v in changed.items()))
    for f, why in failed:
        print("  failed", f, "-", why)
    return 1 if failed else 0


def cmd_verify(args):
    audio = args.audio_dir
    refs, src = referenced(audio), sources(audio)
    errors = []
    for f, k in sorted(refs.items()):
        if not (audio / f).is_file():
            errors.append(f"{f}: missing")
        else:
            try:
                if len(store.decode(audio / f)) < 1200:
                    errors.append(f"{f}: decodes to under 50 ms")
            except Exception as e:  # noqa: BLE001
                errors.append(f"{f}: does not decode ({e})")
        if f not in src:
            errors.append(f"{f}: no sources entry")
            continue
        e = src[f][1]
        if e.get("key") != k:
            errors.append(f"{f}: sources key {e.get('key')!r} but referenced for {k!r}")
        if not e.get("kind"):
            errors.append(f"{f}: sources entry has no kind")
        if e.get("kind") == "human_recording" and not (e.get("license") and (e.get("source_page") or e.get("from"))):
            errors.append(f"{f}: human recording without license/source_page")
        if "pipeline" not in e or "orig" not in e:
            errors.append(f"{f}: no restore fields")
        elif e["orig"].startswith(".local/") and not (e.get("orig_url") or e.get("input")):
            errors.append(f"{f}: original only in a local cache, with no URL or synthesis input")
    for msg in errors:
        print(msg)
    print(f"{len(refs)} referenced clips, {len(errors)} problems")
    return 1 if errors else 0


def cmd_annotate(args):
    """Backfill restore fields for entries written before the generators recorded them."""
    import make_pair_audio as pairs
    import make_variants
    from make_audio import human_restore, slug
    cache = args.cache_root
    human = json.loads((APPLE / "scripts/human_audio.json").read_text())
    variant = {make_variants.name(k, t): t for k, ts in json.loads((APPLE / "scripts/variant_audio.json").read_text()).items() for t in ts}
    added = Counter()
    for name, d in load(args.audio_dir, "sources*.json").items():
        for f, e in d.items():
            if "pipeline" in e:
                continue
            synth = store.silero_fields(e["input"], e.get("voice") or e["provider"].split("voice ")[1].split()[0]) if "input" in e else None
            url = None
            if e.get("set") == "library-sentences":
                orig, url = cache / "drussky-library/tatoeba/public/raw" / (e["audio_url"].rsplit("/", 1)[1] + ".mp3"), e["audio_url"]
            elif "from" in e:
                orig = REPO / e["from"]
            elif synth and name == "sources-pairs.json":
                orig = cache / "drussky-audio/pairs/wav" / (f[:-4] + ".wav")
            elif synth:
                orig = cache / "drussky-audio/wav" / f"{f[:-4]}-{hashlib.sha1(e['input'].encode()).hexdigest()[:6]}.wav"
            elif name == "sources-pairs.json":
                orig, url = pairs.find_orig("File:" + e["title"]), store.commons_url(e["title"])
            elif e["key"] in human and slug(e["key"]) + ".m4a" == f:
                h = human[e["key"]]
                orig = REPO / h["orig"] if (REPO / h["orig"]).exists() else cache / Path(h["orig"]).relative_to(".local")
                url, synth = human_restore(e["key"], h).get("url"), human_restore(e["key"], h).get("synth")
            elif f in variant:
                t = variant[f]
                orig = REPO / t["orig"] if (REPO / t["orig"]).exists() else cache / Path(t["orig"]).relative_to(".local")
                url = store.commons_url(t["title"])
            else:
                print("  no original known for", f)
                continue
            if orig is None or not (Path(orig).exists() or synth):
                print("  original missing for", f, orig)
                continue
            fields = store.restore_fields(orig, url=url, synth=synth)
            e.update({k: v for k, v in fields.items() if k not in e})
            added[name] += 1
        if added[name] and not args.dry_run:
            (args.audio_dir / name).write_text(json.dumps(d, ensure_ascii=False, indent=1) + "\n")
    print("annotated:", dict(added) or "nothing to do")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("command", choices=["status", "restore", "verify", "annotate", "revoice"])
    ap.add_argument("--engine", choices=sorted(store.NARRATOR_SETTINGS), default="voxcpm2", help="revoice: narrator engine")
    ap.add_argument("--dest", type=Path, help="revoice: write a shadow set here instead of the audio dir")
    ap.add_argument("--list", help="revoice: JSON list of {file: ...} restricting the clips (e.g. the shadow set)")
    ap.add_argument("--audio-dir", type=Path, default=DEFAULT_AUDIO)
    ap.add_argument("--cache-root", type=Path, default=REPO / ".local")
    ap.add_argument("--plan", action="store_true")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--offline", action="store_true")
    ap.add_argument("--assume-no-cache", action="store_true")
    ap.add_argument("--max-fetches", type=int)
    ap.add_argument("--max-synth", type=int)
    ap.add_argument("--limit", type=int)
    ap.add_argument("--only", nargs="+")
    args = ap.parse_args()
    args.audio_dir, args.cache_root = args.audio_dir.resolve(), args.cache_root.resolve()
    return {"status": cmd_status, "restore": cmd_restore, "verify": cmd_verify, "annotate": cmd_annotate, "revoice": cmd_revoice}[args.command](args)


if __name__ == "__main__":
    sys.exit(main())
