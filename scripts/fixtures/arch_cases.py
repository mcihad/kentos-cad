#!/usr/bin/env python3
"""Grids, columns, stairs and ramps drawn (docs/adr/0343 §2), from the ADR's rules, without KentOS code.

Writes fixtures/arch/v1/cases.json; `--check` compares and writes nothing. Paths run along the axes or 3-4-5 directions
so every length is an exact fraction; a turn's angle is checked as floats within 1e-9 by the core's test.

- A grid's bubble: radius size/2 × scale/1000 (size 8 mm unless said), its centre past the end along the outward
  tangent by the radius; an arc grid's tangents are its chord turned by ∓2·atan(bulge).
- A column's rectangle: (±w/2, ±d/2) turned by its rotation about its centre.
- A stair's band: the walking line offset half its width each side, mitred at the turns; the landings: at each turn,
  its length along the walking line, half each side; the risers: from 0 a tread apart, one that falls strictly inside a
  landing moved to its end, one past the walking line's end not fitting; two landings leaving less than a tread
  between them are one (docs/adr/0704 §2). Checks: stair_short (risers not fitting), stair_long (length past the last
  riser), stair_step_rule (2 × riser + tread outside 0.60–0.65 m). Fitted (ADR 0704 §1): the last point moved along
  the last segment to the last riser of a line long enough for all, none when that riser is not past the last turn.
  Past its cut riser a stair's risers are drawn dashed (ADR 0704 §3).
- A ramp's sloped run: its length less its landings; its slope rise / run × 100; ramp_slope_mismatch beyond 0.1 points;
  its levels “+base” and “+base+rise” with two decimals.
"""

import argparse
import json
import math
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/arch/v1/cases.json"


def length(a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    s = dx * dx + dy * dy
    r = F(math.isqrt(s.numerator), math.isqrt(s.denominator))
    assert r * r == s, "lengths must be exact"
    return r


def unit(a, b):
    l = length(a, b)
    return ((b[0] - a[0]) / l, (b[1] - a[1]) / l)


def left(d):
    return (-d[1], d[0])


def walk(pts):
    at = [F(0)]
    for a, b in zip(pts, pts[1:]):
        at.append(at[-1] + length(a, b))
    return at


def landings(pts, at, land):
    s = at[-1]
    return [(max(at[k] - land / 2, F(0)), min(at[k] + land / 2, s)) for k in range(1, len(pts) - 1)]


# The core's tolerance (docs/adr/0343 §2): a riser within it of a landing's ends is not inside it, one within it past
# the walking line's end still fits; two landings within it of a tread apart stay two.
EPS = F(1, 10**9)


def stair_landings(pts, at, land, tread):
    """A stair's: consecutive landings with less than a tread between them make one."""
    out = []
    for a, b in landings(pts, at, land):
        if out and a - out[-1][1] < tread - EPS:
            out[-1] = (out[-1][0], max(out[-1][1], b))
        else:
            out.append((a, b))
    return out


def risers(pts, width, count, tread, land):
    at = walk(pts)
    lands = stair_landings(pts, at, land, tread)
    out, missing, s = [], 0, F(0)
    for _ in range(count):
        for a, b in lands:
            if a + EPS < s < b - EPS:
                s = b
                break
        if s > at[-1] + EPS:
            missing += 1
        else:
            out.append(s)
        s += tread
    return out, missing, at


def band(pts, half):
    n = len(pts)
    dirs = [unit(a, b) for a, b in zip(pts, pts[1:])]

    def side(sign):
        out = []
        for i, p in enumerate(pts):
            a, b = left(dirs[min(max(i - 1, 0), n - 2)]), left(dirs[min(i, n - 2)])
            if i == 0:
                m = b
            elif i == n - 1:
                m = a
            else:
                dot = a[0] * b[0] + a[1] * b[1]
                m = ((a[0] + b[0]) / (1 + dot), (a[1] + b[1]) / (1 + dot))
            out.append((p[0] + m[0] * half * sign, p[1] + m[1] * half * sign))
        return out

    return side(1) + side(-1)[::-1]


def fl(p):
    return [float(p[0]), float(p[1])]


def grid_case(name, pts, bulge, label, bubbles, size, scale):
    a, b = pts
    d = unit(a, b)
    half = 2 * math.atan(float(bulge))
    turn = lambda v, t: (v[0] * math.cos(t) - v[1] * math.sin(t), v[0] * math.sin(t) + v[1] * math.cos(t))
    start = turn((float(d[0]), float(d[1])), -half)
    end = turn((float(d[0]), float(d[1])), half)
    r = float(F(size if size else 8) * scale / 2000)
    centres = []
    if bubbles in ("both", "start"):
        centres.append([float(a[0]) - start[0] * r, float(a[1]) - start[1] * r])
    if bubbles in ("both", "end"):
        centres.append([float(b[0]) + end[0] * r, float(b[1]) + end[1] * r])
    shape = {"kind": "grid", "pts": [{"x": float(p[0]), "y": float(p[1])} for p in pts], "name": label}
    if bulge:
        shape["bulges"] = [float(bulge), 0.0]
    if bubbles != "both":
        shape["bubbles"] = bubbles
    if size:
        shape["bubbleSize"] = float(size)
    return {"name": name, "shape": shape, "scale": float(scale), "expect": {"bubbles": centres, "radius": r}}


def column_case(name, at, width, depth, rotation):
    c, s = math.cos(rotation), math.sin(rotation)
    w, d = float(width) / 2, float(depth if depth else width) / 2
    corners = [[float(at[0]) + x * c - y * s, float(at[1]) + x * s + y * c] for x, y in ((-w, -d), (w, -d), (w, d), (-w, d))]
    shape = {"kind": "column", "at": {"x": float(at[0]), "y": float(at[1])}, "width": float(width)}
    if depth:
        shape["depth"] = float(depth)
    if rotation:
        shape["rotation"] = rotation
    return {"name": name, "shape": shape, "scale": 100.0, "expect": {"corners": corners}}


def fit_end(pts, width, count, tread, land):
    """The last point moved along the last segment to the last riser of a line long enough for every riser."""
    prev = pts[-2]
    d = unit(prev, pts[-1])
    start = walk(pts[:-1])[-1]
    reach = (count + 1) * tread + len(pts) * land
    far = pts[:-1] + [(prev[0] + d[0] * reach, prev[1] + d[1] * reach)]
    st, missing, _ = risers(far, width, count, tread, land)
    if missing or not st or st[-1] - start <= EPS:
        return None
    t = st[-1] - start
    return (prev[0] + d[0] * t, prev[1] + d[1] * t)


def stair_case(name, pts, width, count, riser, tread, land=None, cut=None):
    land_len = land if land is not None else width
    st, missing, at = risers(pts, width, count, tread, land_len)
    codes = []
    if missing:
        codes.append("stair_short")
    elif st and at[-1] - st[-1] > F(1, 10**6):
        codes.append("stair_long")
    rule = 2 * riser + tread
    if rule < F(60, 100) or rule > F(65, 100):
        codes.append("stair_step_rule")
    shape = {"kind": "stair", "pts": [{"x": float(p[0]), "y": float(p[1])} for p in pts], "width": float(width),
             "risers": count, "riserHeight": float(riser), "tread": float(tread)}
    if land is not None:
        shape["landing"] = float(land)
    if cut is not None:
        shape["cut"] = cut
    fit = fit_end(pts, width, count, tread, land_len)
    shown = len(st) if cut is None else min(max(cut, 1), len(st))
    return {"name": name, "shape": shape, "scale": 50.0,
            "expect": {"stations": [float(s) for s in st], "missing": missing, "codes": codes,
                       "band": [fl(p) for p in band(pts, width / 2)],
                       "landings": [[float(a), float(b)] for a, b in stair_landings(pts, at, land_len, tread)],
                       "fit": fl(fit) if fit else None, "dashed": len(st) - shown}}


def level(v):
    t = f"{abs(v):.2f}"
    return f"-{t}" if v < 0 and t != "0.00" else f"+{t}"


def ramp_case(name, pts, width, rise, slope=None, base=None, land=None):
    at = walk(pts)
    land_len = land if land is not None else width
    run = at[-1] - sum(b - a for a, b in landings(pts, at, land_len))
    actual = rise / run * 100
    codes = []
    if slope is not None and abs(F(slope) - actual) > F(1, 10):
        codes.append("ramp_slope_mismatch")
    b0 = base if base is not None else F(0)
    shape = {"kind": "ramp", "pts": [{"x": float(p[0]), "y": float(p[1])} for p in pts], "width": float(width),
             "rise": float(rise)}
    if slope is not None:
        shape["slope"] = float(slope)
    if base is not None:
        shape["base"] = float(base)
    if land is not None:
        shape["landing"] = float(land)
    return {"name": name, "shape": shape, "scale": 50.0,
            "expect": {"run": float(run), "slope": float(actual), "codes": codes,
                       "levels": [level(float(b0)), level(float(b0 + rise))],
                       "band": [fl(p) for p in band(pts, width / 2)]}}


def P(x, y):
    return (F(x), F(y))


def cases():
    c = [
        grid_case("Yatay aks, iki uçta balon, 1:100", [P(0, 0), P(10, 0)], 0, "A", "both", None, 100),
        grid_case("Düşey aks, yalnız başta, 10 mm balon, 1:50", [P(0, 0), P(0, 8)], 0, "1", "start", 10, 50),
        grid_case("Yalnız sonda balon, eğik aks (3-4-5)", [P(0, 0), P(6, 8)], 0, "B", "end", None, 100),
        grid_case("Yaylı aks: yarım çember", [P(-5, 0), P(5, 0)], 1, "C", "both", None, 100),
        grid_case("Balonsuz aks", [P(0, 0), P(4, 0)], 0, "D", "none", None, 100),
        column_case("Dikdörtgen kolon", P(2, 3), F(3, 10), F(1, 2), 0.0),
        column_case("Kare kolon, boyu verilmemiş", P(-1, 1), F(2, 5), None, 0.0),
        column_case("Döndürülmüş kolon (30°)", P(5, 5), F(3, 10), F(6, 10), math.pi / 6),
        stair_case("Düz merdiven, tam sığar", [P(0, 0), P(F(15, 4), 0)], F(6, 5), 16, F(7, 40), F(1, 4)),
        stair_case("Düz merdiven, yol uzun", [P(0, 0), P(4, 0)], F(6, 5), 16, F(7, 40), F(1, 4)),
        stair_case("Düz merdiven, yol kısa", [P(0, 0), P(3, 0)], F(6, 5), 16, F(7, 40), F(1, 4)),
        stair_case("L merdiven, sahanlık genişlik kadar", [P(0, 0), P(3, 0), P(3, 3)], F(1), 10, F(18, 100), F(3, 10)),
        stair_case("U merdiven, iki dönüş", [P(0, 0), P(3, 0), P(3, 2), P(0, 2)], F(1), 18, F(17, 100), F(28, 100)),
        stair_case("Eğik düz merdiven (3-4-5)", [P(0, 0), P(F(12, 5), F(16, 5))], F(1), 13, F(17, 100), F(3, 10)),
        stair_case("L merdiven, kısa sahanlık", [P(0, 0), P(2, 0), P(2, 2)], F(1), 12, F(16, 100), F(3, 10), F(1, 2)),
        # ADR 0704: a U-stair's flights a width apart (1.2 + 0.1 gap): one landing across both, no riser between.
        stair_case("U merdiven, kollar arası dar: tek sahanlık", [P(0, 0), P(3, 0), P(3, F(13, 10)), P(0, F(13, 10))],
                   F(6, 5), 18, F(1, 6), F(7, 25)),
        stair_case("U merdiven, sahanlıklar üst üste: tek sahanlık", [P(0, 0), P(3, 0), P(3, 1), P(0, 1)],
                   F(6, 5), 18, F(1, 6), F(7, 25)),
        stair_case("U merdiven, aralarında tam bir basamak: iki sahanlık", [P(0, 0), P(3, 0), P(3, F(37, 25)), P(0, F(37, 25))],
                   F(6, 5), 18, F(1, 6), F(7, 25)),
        stair_case("Düz merdiven, kesmeden sonra kesikli rıhtlar", [P(0, 0), P(F(119, 25), 0)], F(6, 5), 18, F(1, 6),
                   F(7, 25), cut=6),
        stair_case("L merdiven, yola sığmayan uç (son kol kısa)", [P(0, 0), P(F(14, 5), 0), P(F(14, 5), 1)], F(1), 18,
                   F(1, 6), F(7, 25)),
        stair_case("L merdiven, rıhtlar dönüşten önce biter: sığdırılamaz", [P(0, 0), P(6, 0), P(6, 2)], F(1), 12,
                   F(1, 6), F(7, 25)),
        stair_case("Eğik (3-4-5) düz merdiven, uzun yol", [P(0, 0), P(F(24, 5), F(32, 5))], F(1), 10, F(17, 100),
                   F(3, 10)),
        ramp_case("Düz rampa, eğim verilmiş ve uyuyor", [P(0, 0), P(10, 0)], F(3, 2), F(1, 2), slope=5),
        ramp_case("Düz rampa, eğimsiz", [P(0, 0), P(8, 0)], F(3, 2), F(2, 5)),
        ramp_case("L rampa, eğim uymuyor", [P(0, 0), P(6, 0), P(6, 6)], F(3, 2), F(3, 5), slope=6),
        ramp_case("Başlangıç kotu eksi, inen rampa", [P(0, 0), P(0, 10)], F(6, 5), F(-1, 2), base=F(-3, 10)),
        ramp_case("U rampa, kısa sahanlık", [P(0, 0), P(5, 0), P(5, 3), P(0, 3)], F(3, 2), F(1, 2), land=F(1)),
    ]
    return c


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    text = json.dumps({"format": "kentos.arch-cases", "version": 1, "cases": cases()}, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{OUT.relative_to(ROOT)}: güncel")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(cases())} durum yazıldı.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
