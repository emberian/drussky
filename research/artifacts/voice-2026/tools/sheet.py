#!/usr/bin/env python3
"""Build the blind listening sheet (index.html) from audition/*/ and shadow/*/ clips.

  python3 tools/sheet.py          # writes research/artifacts/voice-2026/index.html

Per item, the conditions are shuffled (seeded by item id) and labelled A/B/C/D; the model name, the
ASR transcript and the stress measurement appear only after "reveal". Open the file directly
(file://) from the repo checkout: all paths are relative.
"""
import html
import json
import random
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ITEMS = json.loads((ROOT / "items.json").read_text())
CONDITIONS = ["silero", "voxcpm2", "omnivoice", "espeech"]
NAMES = {"silero": "Silero v5 xenia (current)", "voxcpm2": "VoxCPM2 (MLX, cloned narrator)",
         "omnivoice": "OmniVoice (MLX, cloned narrator)", "espeech": "ESpeech-TTS-1 RL-V2 (F5-TTS, cloned narrator)"}


def load_json(p):
    return json.loads(p.read_text()) if p.exists() else {}


def clip_block(rel, meta, asr, ac, cond, label):
    heard = asr.get("heard_norm")
    err = asr.get("word_errors")
    stress = ac.get("stress") or {}
    details = [f"<b>{html.escape(NAMES[cond])}</b>"]
    if meta.get("generation_seconds") is not None:
        details.append(f"{meta['audio_seconds']}s audio, {meta['generation_seconds']}s to generate")
    if heard is not None:
        details.append(f"ASR heard: <i>{html.escape(heard)}</i>" + (f" ({err} word error{'s' if err != 1 else ''})" if err else " (exact)"))
    if stress.get("stress_measurable"):
        v1, v2 = stress["v1"], stress["v2"]
        details.append(f"vowel durations {v1['duration_ms']} / {v2['duration_ms']} ms, heard stress on syllable {stress['heard_stress']} "
                       f"({'as authored' if stress['correct'] else 'WRONG'}, cue: {stress['cue']})")
    elif "stress" in ac:
        details.append("stress not measurable automatically")
    if ac.get("pitch"):
        p = ac["pitch"]
        details.append(f"F0 {p['f0_min']}–{p['f0_max']} Hz ({p['f0_range_semitones']} st), peak at {int(p['peak_position'] * 100)}% of the clip")
    if meta.get("error"):
        details.append(f"generation failed: {html.escape(meta['error'])}")
    audio = f'<audio controls preload="none" src="{html.escape(rel)}"></audio>' if not meta.get("error") else "<em>no clip</em>"
    return (f'<div class="clip"><div class="label">{label}</div>{audio}'
            f'<div class="reveal hidden">{"<br>".join(details)}</div></div>')


def section(title, items, base, note=""):
    out = [f"<h2>{html.escape(title)}</h2>"]
    if note:
        out.append(f"<p class='note'>{note}</p>")
    data = {c: (load_json(base / c / "asr.json"), load_json(base / c / "acoustics.json")) for c in CONDITIONS}
    for it in items:
        conds = [c for c in CONDITIONS if (base / c / f"{it['id']}.wav").exists() or (base / c / f"{it['id']}.json").exists()]
        if not conds:
            continue
        rng = random.Random(it["id"])
        rng.shuffle(conds)
        blocks = []
        for i, c in enumerate(conds):
            meta = load_json(base / c / f"{it['id']}.json")
            rel = str((base / c / f"{it['id']}.wav").relative_to(ROOT))
            blocks.append(clip_block(rel, meta, data[c][0].get(it["id"], {}), data[c][1].get(it["id"], {}), c, "ABCD"[i]))
        group = it.get("group") or it.get("kind") or ""
        out.append(f'<div class="item"><div class="text"><span class="id">{it["id"]}</span> '
                   f'<span class="group">{html.escape(str(group))}</span><br><span class="ru">{html.escape(it["text"])}</span></div>'
                   f'<div class="clips">{"".join(blocks)}</div>'
                   f'<button onclick="reveal(this)">reveal</button></div>')
    return "\n".join(out)


def main():
    narr = []
    for p in sorted((ROOT / "narrator").glob("*.wav")):
        m = load_json(p.with_suffix(".json"))
        narr.append(f'<div class="clip"><div class="label">{p.stem}</div><audio controls preload="none" src="narrator/{p.name}"></audio>'
                    f'<div class="small">{html.escape(m.get("instruct", ""))}</div></div>')
    body = [
        "<h1>Drussky narrator audition, 2026-11-07</h1>",
        "<p>Each row: the same text through four systems in a hidden, per-row-shuffled order. Listen, pick, then <b>reveal</b>. "
        "Three of the four are the <em>same designed narrator</em> (VoxCPM2 voice design, then cloned by each model), so you are judging "
        "the models' Russian, not different speakers. Silero xenia is today's production voice. Details in "
        "<a href='../../VOICE_2026.md'>research/VOICE_2026.md</a>.</p>",
        "<h2>Narrator candidates (VoxCPM2 voice design, Russian prompt text)</h2>",
        "<p class='note'>Candidate 1 was used as the cloning reference for everything below. If you prefer 2 or 3, say so: re-running "
        "the audition with another reference is one command per model.</p>",
        f'<div class="clips">{"".join(narr)}</div>',
        section("Core items", ITEMS["core"], ROOT / "audition"),
        section("Stress pairs (bare words)", ITEMS["stress_words"], ROOT / "audition",
                "Pairs: the same spelling, stress authored on syllable 1 then 2. The reveal shows which vowel was measured longer."),
    ]
    shadow = ROOT / "shadow" / "shadow.json"
    if shadow.exists():
        body.append(section("Shadow set: 60 corpus sentences", json.loads(shadow.read_text()), ROOT / "shadow",
                            "Representative course sentences, today's clips' exact inputs. Not in Resources/Audio."))
    page = f"""<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Narrator audition</title>
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
:root{{--bg:#fbfaf7;--fg:#222;--muted:#666;--card:#fff;--line:#ddd;--accent:#4b6cb7}}
@media (prefers-color-scheme: dark){{:root{{--bg:#16181d;--fg:#eee;--muted:#aaa;--card:#1f2228;--line:#333;--accent:#8fb0ff}}}}
body{{background:var(--bg);color:var(--fg);font:16px/1.45 -apple-system,system-ui,sans-serif;margin:0;padding:16px;max-width:1100px;margin-inline:auto}}
h1{{font-size:1.5rem}} h2{{font-size:1.15rem;margin-top:2rem;border-bottom:1px solid var(--line)}}
.item{{background:var(--card);border:1px solid var(--line);border-radius:10px;padding:12px;margin:12px 0}}
.text .id{{color:var(--muted);font-size:.85rem}} .text .group{{color:var(--muted);font-size:.85rem}} .ru{{font-size:1.1rem}}
.clips{{display:flex;flex-wrap:wrap;gap:10px;margin:8px 0}}
.clip{{flex:1 1 220px;min-width:200px}} .clip .label{{font-weight:600;color:var(--accent)}}
audio{{width:100%;max-width:320px}} .hidden{{display:none}} .reveal{{font-size:.85rem;color:var(--muted);margin-top:4px}}
.small{{font-size:.8rem;color:var(--muted)}} .note{{color:var(--muted)}}
button{{background:var(--accent);color:#fff;border:0;border-radius:6px;padding:6px 12px;cursor:pointer}}
</style></head><body>
{"".join(body)}
<script>
function reveal(b){{b.parentElement.querySelectorAll('.reveal').forEach(e=>e.classList.remove('hidden'));b.remove();}}
</script></body></html>"""
    (ROOT / "index.html").write_text(page)
    print("wrote", ROOT / "index.html")


if __name__ == "__main__":
    main()
