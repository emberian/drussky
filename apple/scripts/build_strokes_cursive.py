#!/usr/bin/env python3
"""Builds Drussky/Resources/strokes-cursive.json: Russian school cursive (рукописный шрифт,
прописи) stroke models for the 66 letter forms, with the entry and exit of every letter so whole
words can be drawn joined up.

Same design space as strokes.json / StripMetrics: a 132-pt strip, y absolute (cap line 22.4,
baseline 92.4, descender 110.2), x from the letter's own origin. The small letters' top is the
cursive x-line (54), lower than the print models' (42): in прописи a small letter is about half a
capital. Points run in writing direction, and the forms are drawn in one stroke wherever the
school hand does (т is three humps, ш three cups, л and м start with a hook on the baseline,
д has its loop below the line, б rises to a flag, в loops above the line).

Every letter carries `entry` (where a join from the letter before arrives) and `exit` (where the
join to the next letter leaves; absent on letters the school hand does not join onward: э, б's
flag…), classified `top` / `mid` / `base`. `connections` holds the join models by
"<exitKind>-<entryKind>": a polyline in a unit frame, (0,0) at the exit and (1,1) at the entry,
with an optional `lift` (an arch, in design points). CursiveLayout.swift and the proof sheet
below place letters the same way: x advances by `advance`, a space adds 18 and breaks the join.

The letters are authored upright and sheared right by SLANT about the baseline at the end
(прописи slant the hand about 65°; this is gentler so models stay legible at strip size).

Sources checked against (shapes, stroke order, what joins and what does not):
  - В. Г. Горецкий, Н. А. Федосова, «Прописи» 1–4 к «Русской азбуке» (Просвещение) — the
    standard first-grade workbooks; the letter descriptions («начинаем чуть ниже верхней
    линии рабочей строки…») follow their methodology.
  - Wikipedia, "Russian cursive" (https://en.wikipedia.org/wiki/Russian_cursive) and the
    table of handwritten forms at https://ru.wikipedia.org/wiki/Русский_рукописный_шрифт.
  - The "лишишься" problem (и ш л м т as identical zigzags) as described there; our т keeps
    its humps and л/м their hooks, as the прописи do.

Run: python3 scripts/build_strokes_cursive.py [--proof out.png]   (--proof needs Pillow)
"""
import json
import math
import sys
from pathlib import Path

CAP, X, BASE, DESC = 22.4, 54.0, 92.4, 110.2
XMID = (X + BASE) / 2          # 73.2, the waist of a small letter
MID = (CAP + BASE) / 2         # 57.4, the waist of a capital
H = BASE - X                   # 38.4, small-letter height
R = 6                          # the cup at the foot of a stem
U = 20                         # distance between the stems of и
SLANT = 0.27                   # shear: x += (BASE - y) * SLANT


def arc(cx, cy, rx, ry, a0, a1, step=6):
    """Ellipse arc from angle a0 to a1 in degrees, screen coordinates (y down): increasing
    angle turns clockwise on screen, decreasing anticlockwise."""
    n = max(2, int(abs(a1 - a0) / step) + 1)
    return [(cx + rx * math.cos(math.radians(a0 + (a1 - a0) * i / n)),
             cy + ry * math.sin(math.radians(a0 + (a1 - a0) * i / n))) for i in range(n + 1)]


def line(*pts, step=4.0):
    out = [pts[0]]
    for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
        n = max(1, int(math.hypot(x1 - x0, y1 - y0) / step))
        out += [(x0 + (x1 - x0) * i / n, y0 + (y1 - y0) * i / n) for i in range(1, n + 1)]
    return out


def cubic(p0, c1, c2, p1, n=14):
    out = []
    for i in range(n + 1):
        t = i / n
        u = 1 - t
        out.append((u**3 * p0[0] + 3 * u * u * t * c1[0] + 3 * u * t * t * c2[0] + t**3 * p1[0],
                    u**3 * p0[1] + 3 * u * u * t * c1[1] + 3 * u * t * t * c2[1] + t**3 * p1[1]))
    return out


def join(*parts):
    out = []
    for p in parts:
        out += p if not out else (p[1:] if math.dist(out[-1], p[0]) < 0.6 else p)
    return out


def dot(x, y):
    return line((x - 0.8, y - 0.8), (x + 0.8, y + 0.8), step=1)


def s(name, pts, free=False):
    d = {"name": name, "points": [[round(x, 1), round(y, 1)] for x, y in pts]}
    if free:
        d["free"] = True
    return d


# --- cursive building blocks (upright frame) ---------------------------------------------

def stem_cup(x, top=X):
    """A stem from `top` down to the baseline, rounding into a cup; ends at the cup's bottom."""
    return join(line((x, top), (x, BASE - R)), arc(x + R, BASE - R, R, R, 180, 90))


def cup_rise(x, nx, top=X, ntop=X):
    """Stem, cup, then the straight rise to the top of the next stem at nx."""
    return join(line((x, top), (x, BASE - R)), arc(x + R, BASE - R, R, R, 180, 60), line((x + R + 3, BASE - 0.8), (nx, ntop)))


def hook_from(x, y=BASE):
    """The exit hook: a short rise off the baseline at x towards the next letter."""
    return line((x, y), (x + 6, y - 12))


def cup_hook(x, top=X):
    """Stem, cup and the exit hook. Exit at exit_of(x)."""
    return join(line((x, top), (x, BASE - R)), arc(x + R, BASE - R, R, R, 180, 30), line((x + R + 5.2, BASE - 3), exit_of(x)))


def exit_of(x):
    return (x + 16, BASE - 12)


def hook_start(x, y=BASE - 9):
    """The little hook л, м, я begin with: a curl on the baseline before the rise."""
    return cubic((x, y), (x - 0.5, y + 5), (x + 1.5, BASE + 0.3), (x + 4, BASE - 0.5), n=8)


def oval(cx, cy, rx, ry, a0=-60):
    """A full anticlockwise oval from angle a0 back to itself (how о, а, б, д begin)."""
    return arc(cx, cy, rx, ry, a0, a0 - 360)


def tail_loop(x, y_top):
    """The descender loop of у, д, з: down from (x, y_top), round to the left below the line,
    back up across the stem and on towards the next letter."""
    return join(line((x, y_top), (x - 1, DESC - 3)),
                arc(x - 6, DESC - 3, 5, 6.5, 0, 180),
                cubic((x - 11, DESC - 3), (x - 12, DESC - 9), (x - 1, BASE + 2), (x + 9, BASE - 11), n=10))


def small_tail(x):
    """The little tail of ц and щ below the baseline, ending in the exit hook."""
    return join(line((x, BASE), (x + 4, BASE)),
                cubic((x + 4, BASE), (x + 8, BASE + 9), (x - 1, BASE + 9), (x + 2, BASE + 1), n=8),
                line((x + 2, BASE + 1), (x + 12, BASE - 11)))


def bowl(x, top, bottom=BASE):
    """The small bowl of ь ы ъ: from the stem foot, out right and back to the stem at `top`."""
    r = (bottom - top) / 2
    return arc(x, bottom - r, r * 0.95, r, 90, -90)


L = {}
HINT = {}


def letter(glyph, advance, strokes, entry, exit, hint, **flags):
    L[glyph] = (advance, strokes, entry, exit, flags)
    HINT[glyph] = hint


# --- small letters ------------------------------------------------------------------------
letter("и", U + 22, [s("down, cup, up, down, cup", join(cup_rise(0, U), cup_hook(U)))],
       (0, X), exit_of(U), "и starts at the top: down, cup, up, then down and cup again. (л and м start with a hook on the line.)")
letter("й", U + 22, L["и"][1] + [s("breve", cubic((U / 2 - 5, X - 13), (U / 2 - 3, X - 7), (U / 2 + 3, X - 7), (U / 2 + 5, X - 13), n=8))],
       (0, X), exit_of(U), "й is и with a little cup above it, added after the letter.")
letter("ш", 2 * U + 22, [s("three cups", join(cup_rise(0, U), cup_rise(U, 2 * U), cup_hook(2 * U)))],
       (0, X), exit_of(2 * U), "ш is three cups in one stroke, rounded at the bottom; т is rounded at the top.")
letter("щ", 2 * U + 24, [s("three cups and tail", join(cup_rise(0, U), cup_rise(U, 2 * U), stem_cup(2 * U), small_tail(2 * U + R)))],
       (0, X), (2 * U + R + 12, BASE - 11), "щ is ш with a small tail below the line at the end.")
letter("ц", U + 24, [s("two cups and tail", join(cup_rise(0, U), stem_cup(U), small_tail(U + R)))],
       (0, X), (U + R + 12, BASE - 11), "ц is и with a small tail below the line at the end.")
letter("л", U + 22, [s("hook, rise, down, cup", join(hook_start(1), line((5, BASE - 0.5), (U, X)), cup_hook(U)))],
       (1, BASE - 9), exit_of(U), "л starts with a little hook on the baseline, climbs to the top, then comes down into its cup.")
UM = 16
letter("м", 3 * UM + 22, [s("hook, up, down, up, down, cup", join(hook_start(1), line((5, BASE - 0.5), (UM, X)),
                                                                   cubic((UM, X), (UM + 4, XMID), (2 * UM - 4, BASE - 6), (2 * UM, BASE - 1), n=10),
                                                                   cubic((2 * UM, BASE - 1), (2 * UM + 4, BASE - 6), (3 * UM - 4, XMID), (3 * UM, X), n=10),
                                                                   cup_hook(3 * UM)))],
       (1, BASE - 9), exit_of(3 * UM), "м starts with the hook on the baseline, climbs, dips to the line and climbs again; и has no hook.")
letter("т", 2 * U + 22, [s("down, three humps", join(line((0, X), (0, BASE), (0, X + 5)),
                                                     arc(U / 2, X + 5, U / 2, 5, 180, 360),
                                                     line((U, X + 5), (U, BASE), (U, X + 5)),
                                                     arc(1.5 * U, X + 5, U / 2, 5, 180, 360),
                                                     cup_hook(2 * U, X + 5)))],
       (0, X), exit_of(2 * U), "т is three humps in one stroke, rounded at the top (ш is rounded at the bottom).")
letter("п", U + 22, [s("down, hump, down, cup", join(line((0, X), (0, BASE), (0, X + 5)), arc(U / 2, X + 5, U / 2, 5, 180, 360), cup_hook(U, X + 5)))],
       (0, X), exit_of(U), "п: down, back up the same line, over the hump, then down into the cup.")
letter("н", U + 22, [s("down, up to the middle, loop, bar, up, down, cup",
                       join(line((0, X), (0, BASE), (0, XMID)),
                            cubic((0, XMID), (1, XMID - 7), (7, XMID - 6), (6, XMID), n=8),
                            line((6, XMID), (U, XMID), (U, X)), cup_hook(U)))],
       (0, X), exit_of(U), "н: down, back up to the middle, a tiny loop, the bar across, up to the top and down into the cup.")
letter("г", 29, [s("curl over, down, cup", join(cubic((0, X + 7), (1, X + 1), (5, X - 0.5), (7, X + 2), n=8), cup_hook(7, X + 2)))],
       (0, X + 7), exit_of(7), "г curls over at the top, then one stem down into the cup.")
letter("ч", U + 22, [s("down, bowl, up, down, cup", join(cubic((0, X), (0, XMID + 5), (8, XMID + 3), (U, XMID - 4), n=10), line((U, XMID - 4), (U, X)), cup_hook(U)))],
       (0, X), exit_of(U), "ч: a shallow bowl from the top to the middle, up to the top, then down into the cup.")
letter("к", 32, [s("down, up to the middle, arm, leg", join(line((0, X), (0, BASE), (0, XMID)),
                                                        cubic((0, XMID), (4, XMID - 8), (8, X + 1), (10, X + 3), n=8),
                                                        line((10, X + 3), (2, XMID + 1)),
                                                        cubic((2, XMID + 1), (8, XMID + 2), (10, BASE - 10), (10, BASE - R), n=6),
                                                        arc(10 + R, BASE - R, R, R, 180, 30), line((10 + R + 5.2, BASE - 3), exit_of(10))))],
       (0, X), exit_of(10), "к: down, back up to the middle, a short arm up, then the leg down into its cup.")
letter("р", 32, [s("long stem, up, bowl", join(line((0, X), (0, DESC), (0, X)), arc(0, XMID, 11, H / 2, -90, 90), hook_from(0)))],
       (0, X), (6, BASE - 12), "р: the stem goes below the line, back up to the top, then the bowl round to the right.")
letter("у", U + 20, [s("down, cup, up, long loop", join(cup_rise(0, U), tail_loop(U, X)))],
       (0, X), (U + 9, BASE - 11), "у: down, cup, up, then the long stroke below the line with a loop to the left.")
letter("о", 30, [s("oval", oval(10, XMID, 10, H / 2))],
       (15, XMID - H / 2 * 0.866), (15, XMID - H / 2 * 0.866), "о starts near the top right and goes round anticlockwise, closing where it began.")
letter("а", 42, [s("oval, stem, cup", join(oval(10, XMID, 10, H / 2), line((15, XMID - H / 2 * 0.866), (20, X + 2)), cup_hook(20, X + 2)))],
       (15, XMID - H / 2 * 0.866), exit_of(20), "а is an oval round anticlockwise, then a stem down its right side into the cup.")
letter("д", 40, [s("oval, stem, loop below", join(oval(10, XMID, 10, H / 2), line((15, XMID - H / 2 * 0.866), (20, X + 2)), tail_loop(20, X + 2)))],
       (15, XMID - H / 2 * 0.866), (29, BASE - 11), "д is an oval whose stem drops below the line and loops back to the left (not the print д with feet).")
letter("б", 36, [s("oval, stem up, flag", join(oval(10, XMID, 10, H / 2), line((15, XMID - H / 2 * 0.866), (12, CAP + 8)),
                                             cubic((12, CAP + 8), (12, CAP + 1), (19, CAP + 1), (25, CAP + 5), n=8)))],
       (15, XMID - H / 2 * 0.866), None, "б is an oval, then its stem climbs above the small letters and flags to the right.")
letter("в", 32, [s("rise, loop above, stem, bowl", join(line((5, X + 8), (10, CAP + 6)),
                                                  cubic((10, CAP + 6), (9, CAP - 1), (1, CAP + 2), (5, CAP + 12), n=8),
                                                  line((5, CAP + 12), (5, BASE - R)), arc(5 + R, BASE - R, R, R, 180, 90),
                                                  arc(5, XMID, 12, H / 2 - 0.2, 90, -90)))],
       (5, X + 8), (5, X), "в climbs to a loop above the small letters, drops down the stem, and closes its bowl at the top.")
letter("е", 32, [s("rise, loop, round, hook", join(line((0, BASE - 10), (13, X + 6)), arc(8, XMID, 8.5, H / 2, -25, -300), hook_from(8 + 8.5 * math.cos(math.radians(60)), XMID + H / 2 * math.sin(math.radians(60)))))],
       (0, BASE - 10), (8 + 8.5 * 0.5 + 6, XMID + H / 2 * 0.866 - 12), "е rises from the line, loops over the top and round, and leaves at the bottom right.")
letter("ё", 32, L["е"][1] + [s("dots", dot(5, X - 9)), s("second dot", dot(12, X - 9))],
       (0, BASE - 10), L["е"][3], "ё is е with two dots above, added after the letter.")
letter("с", 30, [s("round", join(arc(9, XMID, 9, H / 2, -45, -315), hook_from(9 + 9 * math.cos(math.radians(45)), XMID + H / 2 * math.sin(math.radians(45)))))],
       (9 + 9 * 0.707, XMID - H / 2 * 0.707), (9 + 9 * 0.707 + 6, XMID + H / 2 * 0.707 - 12), "с starts at the top right and goes round anticlockwise, leaving at the bottom right.")
letter("э", 30, [s("round", arc(9, XMID, 9, H / 2, -135, 120)), s("bar", line((5, XMID), (16, XMID)))],
       (9 + 9 * math.cos(math.radians(-135)), XMID + H / 2 * math.sin(math.radians(-135))), None, "э goes round clockwise from the top left; the bar across the middle comes second.")
letter("з", 34, [s("top bump, waist, long loop", join(cubic((1, X + 7), (3, X - 1), (14, X - 1), (14, X + 7), n=8),
                                                     cubic((14, X + 7), (14, XMID - 3), (10, XMID - 1), (6, XMID), n=6),
                                                     cubic((6, XMID), (17, XMID + 1), (19, BASE - 2), (16, DESC - 6), n=8),
                                                     tail_loop(16, DESC - 6)))],
       (1, X + 7), (25, BASE - 11), "з: a small bump at the top, in to the waist, then a bigger curve that loops below the line.")
def half_pair(cl, cr, stem, rx, ry, cy, top):
    """ж's three parts in one stroke: a left half-oval open to the right (-60° to -300°), a rise
    from its lower end to the stem's top, the stem, a rise to the right half-oval, which goes
    round clockwise (-120° to 120°) and leaves in a hook."""
    a0, a1 = math.radians(-60), math.radians(60)
    left_end = (cl + rx * math.cos(a1), cy + ry * math.sin(a1))
    right_start = (cr + rx * math.cos(math.radians(-120)), cy + ry * math.sin(math.radians(-120)))
    right_end = (cr + rx * math.cos(math.radians(120)), cy + ry * math.sin(math.radians(120)))
    pts = join(arc(cl, cy, rx, ry, -60, -300), line(left_end, (stem, top)), line((stem, top), (stem, BASE)),
               line((stem, BASE), right_start), arc(cr, cy, rx, ry, -120, 120), hook_from(right_end[0], right_end[1]))
    entry = (cl + rx * math.cos(a0), cy + ry * math.sin(a0))
    return pts, entry, (right_end[0] + 6, right_end[1] - 12)


_zh, _zh_in, _zh_out = half_pair(9, 39, 24, 9, H / 2 - 1, XMID, X)
letter("ж", 60, [s("left half, up, stem, up, right half", _zh)], _zh_in, _zh_out,
       "ж: the left half-oval like с, up to the stem, down it, up again and the right half-oval the other way.")
letter("х", 34, [s("left curve", arc(0, XMID, 10, H / 2, -90, 90)), s("right curve", join(arc(20, XMID, 10, H / 2, -90, -270), hook_from(20, BASE)))],
       (0, X), (26, BASE - 12), "х is two curves back to back, touching in the middle: the left one first (bulging right), then the right one.")
letter("ф", 40, [s("left oval", oval(7, XMID, 7, H / 2, -90)), s("stem", line((14, X), (14, DESC))), s("right oval", arc(21, XMID, 7, H / 2, -90, 360))],
       (7, X), (28, XMID), "ф: the left oval, the long stem down below the line, then the right oval.")
letter("ы", 36, [s("down, bowl, up, down, cup", join(line((0, X), (0, BASE)), bowl(0, BASE - 18), line((0, BASE - 18), (14, X)), cup_hook(14)))],
       (0, X), exit_of(14), "ы: down, a small bowl back to the stem, then up to the top and down into the cup.")
letter("ь", 22, [s("down, bowl", join(line((0, X), (0, BASE)), bowl(0, BASE - 18)))],
       (0, X), (1, BASE - 18), "ь: one stem down, then a small bowl that closes on the stem halfway up.")
letter("ъ", 28, [s("flag, down, bowl", join(cubic((0, X + 5), (1, X - 1), (5, X - 1), (6, X), n=6), line((6, X), (6, BASE)), bowl(6, BASE - 18)))],
       (0, X + 5), (7, BASE - 18), "ъ is ь with a little flag at the top that the stem hangs from.")
letter("ю", 40, [s("down, up to the middle, bar, oval", join(line((0, X), (0, BASE), (0, XMID)), line((0, XMID), (9, XMID)), arc(17, XMID, 8, H / 2, 180, 450), hook_from(17, BASE)))],
       (0, X), (23, BASE - 12), "ю: down, back up to the middle, the short bar, then the oval round from its left side.")
letter("я", 36, [s("hook, rise, head, down, cup", join(hook_start(1), line((5, BASE - 0.5), (15, X)),
                                                   cubic((15, X), (5, X - 1), (0, XMID - 3), (15, XMID + 1), n=10), cup_hook(15, XMID + 1)))],
       (1, BASE - 9), exit_of(15), "я starts with the hook, climbs, loops its head to the left down to the middle, then the stem into the cup.")

# --- capitals -----------------------------------------------------------------------------
CU = 28  # stem spacing in a capital
TOPC = CAP + 2


def cap_cup_hook(x, top=TOPC):
    return cup_hook(x, top)


def cap_cup_rise(x, nx, top=TOPC):
    return cup_rise(x, nx, top, TOPC)


letter("А", 50, [s("hook, rise, down, cup", join(hook_start(2), line((6, BASE - 0.5), (26, CAP)), cubic((26, CAP), (28, MID), (30, BASE - 14), (30, BASE - R), n=8),
                                                 arc(30 + R, BASE - R, R, R, 180, 30), line((30 + R + 5.2, BASE - 3), exit_of(30)))),
                 s("crossbar", line((9, MID + 8), (36, MID + 8)))],
       (2, BASE - 9), exit_of(30), "А starts with a hook on the line, climbs to the top and comes down into the cup; the bar comes last.")
letter("Б", 48, [s("stem, bowl", join(line((8, CAP), (8, BASE)), bowl(8, MID + 2))), s("roof", line((8, CAP), (44, CAP)))],
       (8, CAP), None, "Б: the stem down, a bowl from its foot back to the middle, then the roof to the right.")
letter("В", 46, [s("stem", line((8, CAP), (8, BASE))), s("two bowls", join(arc(8, (CAP + MID) / 2, 15, (MID - CAP) / 2, -90, 90), arc(8, (MID + BASE) / 2, 18, (BASE - MID) / 2, -90, 90)))],
       (8, CAP), None, "В: the stem down, then two bowls off it, the lower one a little bigger.")
letter("Г", 44, [s("roof, stem, cup", join(line((40, CAP + 3), (10, CAP)), cubic((10, CAP), (6, CAP), (6, CAP + 4), (6, CAP + 8), n=4), cup_hook(6, CAP + 8)))],
       (40, CAP + 3), None, "Г: the roof from right to left, round the corner and down the stem into the cup.")
letter("Д", 54, [s("down, loop, base, round", join(line((30, CAP + 6), (10, BASE - 8)),
                                                   cubic((10, BASE - 8), (6, BASE + 2), (0, BASE - 2), (6, BASE - 10), n=8),
                                                   cubic((6, BASE - 10), (10, BASE - 2), (20, BASE), (32, BASE), n=6),
                                                   cubic((32, BASE), (50, BASE - 6), (50, CAP), (30, CAP + 6), n=12)))],
       (30, CAP + 6), None, "Д: down to the left, a loop at the foot, along the line and round up over the top.")
letter("Е", 44, [s("two rounds", join(arc(22, (CAP + MID) / 2, 18, (MID - CAP) / 2, -30, -270), arc(22, (MID + BASE) / 2, 20, (BASE - MID) / 2, -90, -330)))],
       (22 + 18 * math.cos(math.radians(-30)), (CAP + MID) / 2 + (MID - CAP) / 2 * math.sin(math.radians(-30))), None, "Е is two rounds open to the right, the top one first, both anticlockwise.")
letter("Ё", 44, L["Е"][1] + [s("dots", dot(14, 10)), s("second dot", dot(30, 10))], L["Е"][2], None, "Ё is Е with two dots above.")
_ZH, _ZH_in, _ZH_out = half_pair(15, 65, 40, 15, (BASE - CAP) / 2 - 1, MID, CAP)
letter("Ж", 90, [s("left half, up, stem, up, right half", _ZH)], _ZH_in, _ZH_out,
       "Ж: the left half-oval like С, up to the stem, down it, up again and the right half-oval the other way.")
letter("З", 44, [s("two rounds", join(arc(20, (CAP + MID) / 2, 16, (MID - CAP) / 2, -150, 90), arc(20, (MID + BASE) / 2, 20, (BASE - MID) / 2, -90, 150)))],
       (20 + 16 * math.cos(math.radians(-150)), (CAP + MID) / 2 + (MID - CAP) / 2 * math.sin(math.radians(-150))), None, "З is two rounds to the right, clockwise, the lower one a little bigger.")
letter("И", CU + 24, [s("down, cup, up, down, cup", join(cubic((0, CAP + 10), (0, CAP + 4), (3, TOPC), (6, TOPC), n=4), cap_cup_rise(6, 6 + CU), cap_cup_hook(6 + CU)))],
       (0, CAP + 10), exit_of(6 + CU), "И: down, cup, up, then down and cup again — the same as the small и, taller.")
letter("Й", CU + 24, L["И"][1] + [s("breve", cubic((12, 4), (15, 11), (23, 11), (26, 4), n=8))], (0, CAP + 10), exit_of(6 + CU), "Й is И with a little cup above.")
letter("К", 46, [s("stem", cup_hook(8, CAP)), s("arm, loop, leg", join(line((38, CAP + 2), (14, MID - 2)), cubic((14, MID - 2), (10, MID - 2), (10, MID + 4), (15, MID + 3), n=6), line((15, MID + 3), (36, BASE))))],
       (8, CAP), None, "К: the stem down into its cup, then the arm from the top right to a little loop in the middle and the leg out.")
letter("Л", CU + 24, [s("hook, rise, down, cup", join(hook_start(2), line((6, BASE - 0.5), (6 + CU, CAP)), cap_cup_hook(6 + CU, CAP)))],
       (2, BASE - 9), exit_of(6 + CU), "Л starts with a hook on the line, climbs to the top and comes down into the cup.")
CM = 22
letter("М", 3 * CM + 24, [s("hook, up, down, up, down, cup", join(hook_start(2), line((6, BASE - 0.5), (CM + 2, CAP)),
                                                                  cubic((CM + 2, CAP), (CM + 6, MID), (2 * CM - 2, BASE - 8), (2 * CM + 2, BASE - 1), n=10),
                                                                  cubic((2 * CM + 2, BASE - 1), (2 * CM + 6, BASE - 8), (3 * CM - 2, MID), (3 * CM + 2, CAP), n=10),
                                                                  cap_cup_hook(3 * CM + 2, CAP)))],
       (2, BASE - 9), exit_of(3 * CM + 2), "М starts with the hook, climbs, dips to the line and climbs again, then down into the cup.")
letter("Н", CU + 26, [s("left stem", line((6, CAP), (6, BASE))),
                      s("loop, bar, up, down, cup", join(cubic((6, MID), (7, MID - 9), (14, MID - 8), (12, MID), n=6), line((12, MID), (6 + CU, MID), (6 + CU, CAP)), cap_cup_hook(6 + CU, CAP)))],
       (6, CAP), exit_of(6 + CU), "Н: the left stem; then from its middle a tiny loop, the bar across, up to the top and down into the cup.")
letter("О", 50, [s("oval", oval(24, MID, 23, (BASE - CAP) / 2, -60))],
       (24 + 23 * 0.5, MID - (BASE - CAP) / 2 * 0.866), (24 + 23 * 0.5, MID - (BASE - CAP) / 2 * 0.866), "О goes round anticlockwise from the top right, closing where it began.")
letter("П", 48, [s("left stem", join(cubic((2, CAP + 4), (3, CAP), (6, CAP), (8, CAP), n=3), line((8, CAP), (8, BASE)))), s("roof, down, cup", join(line((8, CAP), (8 + CU + 4, CAP)), cap_cup_hook(8 + CU + 4, CAP)))],
       (2, CAP + 4), None, "П: the left stem down, then the roof across and the right stem down into its cup.")
letter("Р", 44, [s("stem", cup_hook(8, CAP)), s("bowl", arc(8, (CAP + MID) / 2 + 2, 20, (MID - CAP) / 2 + 2, -90, 90))],
       (8, CAP), None, "Р: the stem down into its cup, then the bowl off the top.")
letter("С", 46, [s("round", arc(23, MID, 22, (BASE - CAP) / 2, -40, -320))],
       (23 + 22 * math.cos(math.radians(-40)), MID + (BASE - CAP) / 2 * math.sin(math.radians(-40))), None, "С starts at the top right and goes round anticlockwise.")
letter("Т", 50, [s("roof", join(cubic((2, CAP + 6), (2, CAP), (8, CAP), (12, CAP), n=4), line((12, CAP), (44, CAP)), cubic((44, CAP), (48, CAP), (48, CAP + 4), (46, CAP + 7), n=4))),
                 s("stem, cup", cup_hook(24, CAP))],
       (2, CAP + 6), None, "Т: the wavy roof first, then the stem down the middle into its cup.")
letter("У", 48, [s("curve, up, long down, curl", join(cubic((4, CAP + 10), (2, CAP + 2), (12, CAP - 2), (14, CAP + 8), n=6), cubic((14, CAP + 8), (14, MID), (22, MID + 2), (30, MID - 6), n=6),
                                                      line((30, MID - 6), (40, CAP + 2)), line((40, CAP + 2), (14, BASE - 6)),
                                                      cubic((14, BASE - 6), (8, BASE + 2), (2, BASE - 2), (4, BASE - 10), n=6)))],
       (4, CAP + 10), None, "У: a short curve from the top left down to the middle, up to the top right, then the long stroke down to a curl on the line.")
letter("Ф", 58, [s("left oval", oval(14, MID + 2, 14, (BASE - CAP) / 2 - 8, -90)), s("stem", line((28, CAP), (28, BASE))), s("right oval", arc(42, MID + 2, 14, (BASE - CAP) / 2 - 8, -90, 270))],
       (12, CAP + 8), None, "Ф: the left oval, the stem down the middle, then the right oval.")
letter("Х", 52, [s("left curve", arc(0, MID, 16, (BASE - CAP) / 2, -90, 90)), s("right curve", join(arc(32, MID, 16, (BASE - CAP) / 2, -90, -270), hook_from(32, BASE)))],
       (0, CAP), (38, BASE - 12), "Х is two curves back to back, touching in the middle: the left one first, then the right.")
letter("Ц", CU + 28, [s("down, cup, up, down, tail", join(cubic((0, CAP + 10), (0, CAP + 4), (3, TOPC), (6, TOPC), n=4), cap_cup_rise(6, 6 + CU), stem_cup(6 + CU, TOPC), small_tail(6 + CU + R)))],
       (0, CAP + 10), (6 + CU + R + 12, BASE - 11), "Ц is И with a small tail below the line at the end.")
letter("Ч", CU + 24, [s("down, bowl, up, down, cup", join(cubic((6, CAP), (6, MID + 6), (16, MID + 4), (6 + CU, MID - 4), n=10), line((6 + CU, MID - 4), (6 + CU, CAP)), cap_cup_hook(6 + CU, CAP)))],
       (6, CAP), exit_of(6 + CU), "Ч: a bowl from the top down to the middle, up to the top right, then down into the cup.")
letter("Ш", 2 * CU + 24, [s("three cups", join(cubic((0, CAP + 10), (0, CAP + 4), (3, TOPC), (6, TOPC), n=4), cap_cup_rise(6, 6 + CU), cap_cup_rise(6 + CU, 6 + 2 * CU), cap_cup_hook(6 + 2 * CU)))],
       (0, CAP + 10), exit_of(6 + 2 * CU), "Ш is three cups in one stroke, rounded at the bottom.")
letter("Щ", 2 * CU + 28, [s("three cups and tail", join(cubic((0, CAP + 10), (0, CAP + 4), (3, TOPC), (6, TOPC), n=4), cap_cup_rise(6, 6 + CU), cap_cup_rise(6 + CU, 6 + 2 * CU), stem_cup(6 + 2 * CU, TOPC), small_tail(6 + 2 * CU + R)))],
       (0, CAP + 10), (6 + 2 * CU + R + 12, BASE - 11), "Щ is Ш with a small tail below the line at the end.")
letter("Ъ", 46, [s("flag, stem, bowl", join(line((2, CAP), (14, CAP)), line((14, CAP), (14, BASE)), bowl(14, MID + 2)))],
       (2, CAP), None, "Ъ: the flag along the top, the stem down from its end, then the bowl back to the middle.")
letter("Ы", 58, [s("stem, bowl, up, down, cup", join(line((6, CAP), (6, BASE)), bowl(6, MID + 2), line((6, MID + 2), (6 + CU + 6, CAP)), cap_cup_hook(6 + CU + 6, CAP)))],
       (6, CAP), exit_of(6 + CU + 6), "Ы: the stem down, a bowl back to the middle, then up to the top and down into the cup.")
letter("Ь", 42, [s("stem, bowl", join(line((6, CAP), (6, BASE)), bowl(6, MID + 2)))],
       (6, CAP), None, "Ь: the stem down, then the bowl from its foot back to the middle.")
letter("Э", 46, [s("round", arc(23, MID, 22, (BASE - CAP) / 2, -140, 125)), s("bar", line((14, MID), (44, MID)))],
       (23 + 22 * math.cos(math.radians(-140)), MID + (BASE - CAP) / 2 * math.sin(math.radians(-140))), None, "Э goes round clockwise from the top left; the bar comes second.")
letter("Ю", 68, [s("stem, bar, oval", join(line((6, CAP), (6, BASE), (6, MID)), line((6, MID), (20, MID)), arc(40, MID, 20, (BASE - CAP) / 2, 180, 450), hook_from(40, BASE)))],
       (6, CAP), (46, BASE - 12), "Ю: the stem down, back up to the middle, the bar, then the oval round from its left side.")
letter("Я", 48, [s("hook, rise, head, down, cup", join(hook_start(2), line((6, BASE - 0.5), (26, CAP)), cubic((26, CAP), (8, CAP - 1), (0, MID - 4), (26, MID + 2), n=10), cap_cup_hook(26, MID + 2)))],
       (2, BASE - 9), exit_of(26), "Я starts with the hook, climbs, loops its head to the left down to the middle, then the stem into the cup.")


# --- shear, classify, write ----------------------------------------------------------------

def shear(p):
    return (p[0] + (BASE - p[1]) * SLANT, p[1])


def kind(p):
    if p[1] <= X + 8:
        return "top"
    if p[1] >= BASE - 16:
        return "base"
    return "mid"


CONNECTIONS = {f"{a}-{b}": {"points": [[0, 0], [1, 1]]} for a in ("top", "mid", "base") for b in ("top", "mid", "base")}
CONNECTIONS["top-top"]["lift"] = 1.5     # a slight arch along the top (ои, он)
CONNECTIONS["base-top"]["points"] = [[0, 0], [0.45, 0.4], [1, 1]]   # the long rise steepens as it nears the stem
CONNECTIONS["top-base"]["points"] = [[0, 0], [0.5, 0.55], [1, 1]]   # the dip of ол, ом


def build():
    assert len(L) == 66, len(L)
    letters = {}
    for g, (adv, strokes, entry, exit, flags) in L.items():
        d = {"advance": adv,
             "strokes": [{**st, "points": [[round(x, 1), round(y, 1)] for x, y in map(shear, st["points"])]} for st in strokes],
             "entry": [round(v, 1) for v in shear(entry)], "entryKind": kind(entry), "hint": HINT[g], **flags}
        if exit is not None:
            d["exit"] = [round(v, 1) for v in shear(exit)]
            d["exitKind"] = kind(exit)
        letters[g] = d
    return {
        "version": 1, "designHeight": 132, "style": "cursive", "slant": SLANT, "space": 18,
        "lines": {"cap": CAP, "xHeight": X, "baseline": BASE, "descender": DESC},
        "note": "School cursive models (прописи); points run in writing direction, y absolute on the 132-pt strip, x from the letter's origin; entry/exit are where joins arrive and leave.",
        "letters": letters, "connections": CONNECTIONS,
    }


# --- layout shared with CursiveLayout.swift ------------------------------------------------

def layout(data, text, x0=16.0):
    """[(kind, payload, x)]: ("letter", glyph, x) and ("join", [(x, y)…], 0) in writing order."""
    out, x, prev = [], x0, None
    for ch in text:
        if ch == " ":
            x += data["space"]
            prev = None
            continue
        l = data["letters"].get(ch)
        if l is None:
            x += 20
            prev = None
            continue
        if prev is not None and "exit" in prev[0]:
            pl, px = prev
            a = (px + pl["exit"][0], pl["exit"][1])
            b = (x + l["entry"][0], l["entry"][1])
            model = data["connections"][f"{pl['exitKind']}-{l['entryKind']}"]
            pts = [(a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * v - model.get("lift", 0) * math.sin(math.pi * u)) for u, v in model["points"]]
            out.append(("join", pts, 0))
        out.append(("letter", ch, x))
        x += l["advance"]
        prev = (l, x - l["advance"])
    return out, x


PROOF_WORDS = ["мама", "молоко", "шишка", "лимон", "тишина", "дом", "будет", "вода", "где", "жук", "зима", "кот", "окно",
               "папа", "ракета", "сестра", "утро", "кофе", "хлеб", "цирк", "чашка", "щи", "объект", "сыр", "мать", "это",
               "юг", "я тут", "Анна и Том", "Это Москва."]


def proof(data, path):
    from PIL import Image, ImageDraw
    k = 3.0  # pixels per design point
    cols, cell_w, row_h = 6, 150, 132
    rows = []
    for i in range(0, 66, 11):
        rows.append([("letter", g) for g in list(L)[i:i + 11]])
    word_rows = [PROOF_WORDS[i:i + 3] for i in range(0, len(PROOF_WORDS), 3)]
    width = int(k * 1240)
    height = int(k * (row_h * (len(rows) + len(word_rows)) + 20))
    im = Image.new("RGB", (width, height), (251, 248, 240))
    d = ImageDraw.Draw(im)

    def rules(oy):
        for y, col in ((CAP, (160, 180, 210)), (X, (160, 180, 210)), (BASE, (90, 110, 150)), (DESC, (200, 210, 225))):
            d.line([(0, (oy + y) * k), (width, (oy + y) * k)], fill=col, width=1 if y != BASE else 2)

    def draw_text(text, ox, oy):
        items, _ = layout(data, text, x0=0)
        for kind_, payload, x in items:
            if kind_ == "join":
                pts = [((ox + px) * k, (oy + py) * k) for px, py in payload]
                d.line(pts, fill=(200, 60, 50), width=int(2.2 * k), joint="curve")
            else:
                for st in data["letters"][payload]["strokes"]:
                    pts = [((ox + x + px) * k, (oy + py) * k) for px, py in st["points"]]
                    if len(pts) > 1:
                        d.line(pts, fill=(27, 43, 75), width=int(3.2 * k), joint="curve")
                    d.ellipse([pts[0][0] - 2.2 * k, pts[0][1] - 2.2 * k, pts[0][0] + 2.2 * k, pts[0][1] + 2.2 * k], fill=(200, 60, 50))

    oy = 10
    for row in rows:
        rules(oy)
        x = 10
        for _, g in row:
            draw_text(g, x, oy)
            x += 100
        oy += row_h
    for row in word_rows:
        rules(oy)
        x = 10
        for w in row:
            draw_text(w, x, oy)
            x += 410
        oy += row_h
    im.save(path)
    print(path, im.size)


def main():
    data = build()
    path = Path(__file__).resolve().parent.parent / "Drussky/Resources/strokes-cursive.json"
    path.write_text(json.dumps(data, ensure_ascii=False, separators=(",", ":")) + "\n")
    print(path, sum(len(st["points"]) for l in data["letters"].values() for st in l["strokes"]), "points")
    if "--proof" in sys.argv:
        proof(data, sys.argv[sys.argv.index("--proof") + 1])


if __name__ == "__main__":
    main()
