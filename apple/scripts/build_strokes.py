#!/usr/bin/env python3
"""Builds Drussky/Resources/strokes.json: hand-printed stroke models for the 33 letters.

Coordinates are the writing strip's 132-pt design space (see StripMetrics): y is absolute on
the strip, x is relative to the letter's pen origin in the model font (SF Rounded), so a model
sits exactly on the ghost letter the strip draws. Points run in writing direction.

The forms are the ones Russians print by hand: single-storey а, a ring-and-stem б, д with
feet below the baseline. Uprights go top to bottom, bars left to right, rounds anticlockwise
from the top (о, с, the bowls of а and ю), the bowls of б в р ь ъ ы clockwise off the stem.

Per stroke, "free": true means either direction is fine (the diagonal of и, for one).
Per letter, "anyOrder": true means the first stroke may be any of them (т, н, х, ф).
Run: python3 scripts/build_strokes.py
"""
import json
import math
from pathlib import Path

# Model-font lines on a 132-pt strip (SF Rounded scaled so cap height = baseline - cap line).
CAP, XH, BASE = 22.4, 42.0, 92.4
DESC = 110.2      # р у ф
TAIL = 106.7      # the feet of д ц щ
MID = (CAP + BASE) / 2
XMID = (XH + BASE) / 2


def arc(cx, cy, rx, ry, a0, a1, step=6):
    """Ellipse arc from angle a0 to a1 in degrees, screen coordinates (y down): increasing
    angle turns clockwise on screen, decreasing anticlockwise."""
    n = max(2, int(abs(a1 - a0) / step) + 1)
    out = []
    for i in range(n + 1):
        a = math.radians(a0 + (a1 - a0) * i / n)
        out.append((cx + rx * math.cos(a), cy + ry * math.sin(a)))
    return out


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


def bowl_right(x0, top, bottom, x1, name="bowl", rx=None):
    """A bowl off a stem at x0: across the top to x1, round clockwise, back along the bottom."""
    r = (bottom - top) / 2
    rx = rx or r * 1.2
    return s(name, join(line((x0, top), (x1, top)), arc(x1, top + r, rx, r, -90, 90), line((x1, bottom), (x0, bottom))))


def oval(cx, cy, rx, ry, name="round"):
    return s(name, arc(cx, cy, rx, ry, -90, -455))


def upright(x, top, bottom, name="upright"):
    return s(name, line((x, top), (x, bottom)))


L = {}

# --- capitals -----------------------------------------------------------------------------
L["А"] = (63.6, [s("left slant", line((31.8, CAP), (4, BASE))), s("right slant", line((31.8, CAP), (60, BASE))),
                 s("crossbar", line((15, 70), (49, 70)))], {})
L["Б"] = (60.9, [upright(9, CAP, BASE), s("roof", line((9, CAP), (52, CAP))), bowl_right(9, 54, BASE, 32)], {})
L["В"] = (61.0, [upright(9, CAP, BASE), bowl_right(9, CAP, 56.6, 30, "top bowl"), bowl_right(9, 56.6, BASE, 33, "bottom bowl")], {})
L["Г"] = (51.9, [upright(9, CAP, BASE), s("roof", line((9, CAP), (49, CAP)))], {})
L["Д"] = (71.8, [s("left side", cubic((17, CAP), (16, 60), (12, 82), (6, BASE))),
                 s("roof and right side", line((17, CAP), (59, CAP), (59, BASE))),
                 s("base and feet", line((3, TAIL), (3, BASE), (67, BASE), (67, TAIL)))], {})
L["Е"] = (55.8, [upright(9, CAP, BASE), s("top bar", line((9, CAP), (50, CAP))), s("middle bar", line((9, MID), (45, MID))),
                 s("bottom bar", line((9, BASE), (51, BASE)))], {})
L["Ё"] = (55.8, L["Е"][1] + [s("dots", dot(20, 9)), s("second dot", dot(38, 9))], {})
L["Ж"] = (95.4, [s("left arms", line((8, CAP), (44, MID), (5, BASE))), upright(47.7, CAP, BASE),
                 s("right arms", line((88, CAP), (51.4, MID), (91, BASE)))], {})
L["З"] = (57.5, [s("two bumps", join(arc(28, 39.6, 21, 17.4, -155, 90), arc(28, 75.4, 25, 17.0, -90, 150)))], {})
L["И"] = (70.4, [upright(9, CAP, BASE, "left upright"), s("diagonal", line((9, BASE), (61, CAP)), free=True),
                 upright(61, CAP, BASE, "right upright")], {})
L["Й"] = (70.4, L["И"][1] + [s("breve", cubic((22, 4), (27, 13), (43, 13), (48, 4), n=10))], {})
L["К"] = (61.8, [upright(9, CAP, BASE), s("arm and leg", line((55, CAP), (11, 59), (57, BASE)))], {})
L["Л"] = (67.4, [s("left leg", cubic((20, CAP), (20, 62), (14, 86), (3, BASE))), s("roof and right side", line((20, CAP), (59, CAP), (59, BASE)))], {})
L["М"] = (83.5, [upright(9, CAP, BASE, "left upright"), s("middle V", line((9, CAP), (41.7, 74), (74, CAP))),
                 upright(74, CAP, BASE, "right upright")], {})
L["Н"] = (70.4, [upright(9, CAP, BASE, "left upright"), upright(61, CAP, BASE, "right upright"),
                 s("crossbar", line((9, MID), (61, MID)))], {"anyOrder": True})
L["О"] = (73.6, [oval(36.8, 57.45, 32.2, 36.1)], {})
L["П"] = (69.4, [upright(9, CAP, BASE, "left upright"), s("roof and right side", line((9, CAP), (60.5, CAP), (60.5, BASE)))], {})
L["Р"] = (58.3, [upright(9, CAP, BASE), bowl_right(9, CAP, 61, 31, rx=22)], {})
L["С"] = (68.9, [s("round", arc(35, 57.45, 30.4, 36.1, -40, -320))], {})
L["Т"] = (58.7, [s("roof", line((3, CAP), (56, CAP))), upright(29.4, CAP, BASE)], {"anyOrder": True})
L["У"] = (59.1, [s("short arm", line((5, CAP), (30.6, 64))), s("long stroke", line((55, CAP), (14, BASE)))], {})
L["Ф"] = (78.7, [upright(39.4, 20.8, 94), s("oval", arc(39.4, 51, 35, 20.5, -90, -455))], {"anyOrder": True})
L["Х"] = (64.0, [s("first diagonal", line((6, CAP), (58, BASE))), s("second diagonal", line((58, CAP), (6, BASE)))], {"anyOrder": True})
L["Ц"] = (73.8, [upright(9, CAP, BASE, "left upright"), upright(61, CAP, BASE, "right upright"),
                 s("base and tail", line((9, BASE), (69, BASE), (69, TAIL)))], {})
L["Ч"] = (66.3, [s("hook", join(line((8, CAP), (8, 44)), cubic((8, 44), (8, 60), (30, 64), (57, 56)))), upright(58, CAP, BASE)], {})
L["Ш"] = (91.3, [upright(9, CAP, BASE, "left upright"), upright(45.7, CAP, BASE, "middle upright"),
                 upright(82, CAP, BASE, "right upright"), s("base", line((9, BASE), (82, BASE)))], {})
L["Щ"] = (95.7, [upright(9, CAP, BASE, "left upright"), upright(45, CAP, BASE, "middle upright"),
                 upright(81, CAP, BASE, "right upright"), s("base and tail", line((9, BASE), (90, BASE), (90, TAIL)))], {})
L["Ъ"] = (70.9, [s("flag and upright", line((3, CAP), (19, CAP), (19, BASE))), bowl_right(19, 55, BASE, 43)], {})
L["Ы"] = (81.0, [upright(9, CAP, BASE), bowl_right(9, 55, BASE, 28), upright(72, CAP, BASE, "separate upright")], {})
L["Ь"] = (59.2, [upright(9, CAP, BASE), bowl_right(9, 55, BASE, 31)], {})
L["Э"] = (68.9, [s("round", arc(34, 57.45, 30.4, 36.1, -140, 140)), s("bar", line((24, MID), (63, MID)))], {})
L["Ю"] = (96.5, [upright(9, CAP, BASE), s("short bar", line((9, MID), (31, MID))), oval(61.5, 57.45, 30.5, 36.1, "oval")], {})
L["Я"] = (62.4, [s("bowl", join(line((53, CAP), (29, CAP)), arc(29, 40.8, 22, 18.4, -90, -270), line((29, 59.2), (53, 59.2)))),
                 upright(53, CAP, BASE, "right upright"), s("leg", line((31, 59.2), (6, BASE)))], {})

# --- small letters ------------------------------------------------------------------------
L["а"] = (50.8, [s("round", arc(24, XMID, 20, 25.2, -20, -380)), upright(44, XH, BASE, "stem")], {})
L["б"] = (54.6, [s("flag and ring", join(cubic((47, 22), (30, 26), (8, 32), (7, 70.4)), arc(27.5, 70.4, 20.5, 22, 180, -180)))], {})
L["в"] = (48.7, [upright(8, XH, BASE), bowl_right(8, XH, 67, 25, "top bowl"), bowl_right(8, 67, BASE, 27, "bottom bowl")], {})
L["г"] = (41.6, [upright(8, XH, BASE), s("roof", line((8, XH), (39, XH)))], {})
L["д"] = (57.4, [s("left side", cubic((14, XH), (13, 70), (10, 85), (5, BASE))),
                 s("roof and right side", line((14, XH), (46, XH), (46, BASE))),
                 s("base and feet", line((2, TAIL), (2, BASE), (54, BASE), (54, TAIL)))], {})
L["е"] = (51.8, [s("bar and round", join(line((6, XMID), (47, XMID)), arc(26.2, XMID, 21, 25.6, 0, -315)))], {})
L["ё"] = (51.8, L["е"][1] + [s("dots", dot(17, 27)), s("second dot", dot(35, 27))], {})
L["ж"] = (73.5, [s("left arms", line((6, XH), (34, XMID), (5, BASE))), upright(36.7, XH, BASE),
                 s("right arms", line((67, XH), (39.4, XMID), (69, BASE)))], {})
L["з"] = (46.2, [s("two bumps", join(arc(22.5, 54.5, 16, 12.5, -150, 90), arc(23, 79.7, 19, 12.7, -90, 150)))], {})
L["и"] = (54.1, [upright(8, XH, BASE, "left upright"), s("diagonal", line((8, BASE), (46, XH)), free=True),
                 upright(46, XH, BASE, "right upright")], {})
L["й"] = (54.1, L["и"][1] + [s("breve", cubic((15, 22), (19, 31), (35, 31), (39, 22), n=10))], {})
L["к"] = (48.4, [upright(8, XH, BASE), s("arm and leg", line((43, XH), (9, 68), (44, BASE)))], {})
L["л"] = (52.4, [s("left leg", cubic((15, XH), (15, 72), (10, 88), (2, BASE))), s("roof and right side", line((15, XH), (45, XH), (45, BASE)))], {})
L["м"] = (68.3, [upright(8, XH, BASE, "left upright"), s("middle V", line((8, XH), (34, 80), (60, XH))), upright(60, XH, BASE, "right upright")], {})
L["н"] = (53.4, [upright(8, XH, BASE, "left upright"), upright(46, XH, BASE, "right upright"),
                 s("crossbar", line((8, XMID), (46, XMID)))], {"anyOrder": True})
L["о"] = (53.6, [oval(26.8, XMID, 23.3, 25.4)], {})
L["п"] = (52.9, [upright(8, XH, BASE, "left upright"), s("roof and right side", line((8, XH), (45, XH), (45, BASE)))], {})
L["р"] = (55.5, [upright(8, XH, DESC, "stem"), bowl_right(8, XH, BASE, 28, rx=22)], {})
L["с"] = (50.6, [s("round", arc(25.4, XMID, 21.6, 25.4, -35, -325))], {})
L["т"] = (44.7, [s("roof", line((2, XH), (42, XH))), upright(22, XH, BASE)], {"anyOrder": True})
L["у"] = (49.2, [s("short arm", line((4, XH), (25, 91))), s("long stroke", join(line((45, XH), (21, 102)), cubic((21, 102), (18, 109), (12, 111), (6, 109))))], {})
L["ф"] = (67.2, [upright(33.6, 24.4, DESC), s("oval", arc(33.6, XMID, 29.5, 23.5, -90, -455))], {"anyOrder": True})
L["х"] = (47.5, [s("first diagonal", line((5, XH), (43, BASE))), s("second diagonal", line((43, XH), (5, BASE)))], {"anyOrder": True})
L["ц"] = (56.2, [upright(8, XH, BASE, "left upright"), upright(46, XH, BASE, "right upright"),
                 s("base and tail", line((8, BASE), (53, BASE), (53, TAIL)))], {})
L["ч"] = (50.3, [s("hook", join(line((7, XH), (7, 57)), cubic((7, 57), (7, 72), (26, 74), (42, 67)))), upright(43, XH, BASE)], {})
L["ш"] = (74.1, [upright(8, XH, BASE, "left upright"), upright(37, XH, BASE, "middle upright"),
                 upright(66, XH, BASE, "right upright"), s("base", line((8, BASE), (66, BASE)))], {})
L["щ"] = (77.8, [upright(8, XH, BASE, "left upright"), upright(36, XH, BASE, "middle upright"),
                 upright(64, XH, BASE, "right upright"), s("base and tail", line((8, BASE), (74, BASE), (74, TAIL)))], {})
L["ъ"] = (55.5, [s("flag and upright", line((2, XH), (15, XH), (15, BASE))), bowl_right(15, 65, BASE, 33)], {})
L["ы"] = (63.3, [upright(8, XH, BASE), bowl_right(8, 65, BASE, 24), upright(56, XH, BASE, "separate upright")], {})
L["ь"] = (46.0, [upright(8, XH, BASE), bowl_right(8, 65, BASE, 25)], {})
L["э"] = (50.6, [s("round", arc(25, XMID, 21.6, 25.6, -145, 145)), s("bar", line((15, XMID), (46, XMID)))], {})
L["ю"] = (72.5, [upright(8, XH, BASE), s("short bar", line((8, XMID), (23, XMID))), oval(46.3, XMID, 22.5, 25.4, "oval")], {})
L["я"] = (47.9, [s("bowl", join(line((41, XH), (24, XH)), arc(24, 54.6, 16.5, 12.6, -90, -270), line((24, 67.2), (41, 67.2)))),
                 upright(41, XH, BASE, "right upright"), s("leg", line((25, 67.2), (6, BASE)))], {})


def main():
    assert len(L) == 66, len(L)
    out = {
        "version": 1,
        "designHeight": 132,
        "lines": {"cap": CAP, "xHeight": XH, "baseline": BASE, "descender": DESC},
        "note": "Hand-printed models; points run in writing direction, y absolute on the 132-pt strip, x from the letter's pen origin in the model font.",
        "letters": {k: {"advance": adv, "strokes": strokes, **flags} for k, (adv, strokes, flags) in L.items()},
    }
    path = Path(__file__).resolve().parent.parent / "Drussky/Resources/strokes.json"
    path.write_text(json.dumps(out, ensure_ascii=False, separators=(",", ":")) + "\n")
    print(path, sum(len(st["points"]) for _, sts, _ in L.values() for st in sts), "points")


if __name__ == "__main__":
    main()
