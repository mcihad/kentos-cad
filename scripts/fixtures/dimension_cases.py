#!/usr/bin/env python3
"""Independent reference of the new dimensions' layout (docs/adr/0147 §2).

Writes fixtures/dimension/v1/layout.json from the rules alone, with Python's
standard library and no KentOS code: for each dimension, its lines, where its
value stands and how it turns, the value, its unit and prefix, and the grip
that moves it. The geometry core (`geom::dimension::layout_dimension`) must
give the same within 1e-9 m on both platforms
(crates/shared/geometry-core/tests/dimension_layout.rs; the web through its
WASM).

Every length is in the value's height h; g = h/2 is the gap. As every
dimension:

- a tick at p along a unit direction u is the line p ∓ t, t = ((u.x − u.y),
  (u.y + u.x)) · √½ · 0.6h / 2;
- the value along u through a point m: its turn is u's angle in degrees,
  turned a half turn when it is over 90 or not over −90 (so it reads left
  to right; then its "above" is the other side), and it stands 0.35h above m
  on its reading side.

Which of the lines are extension lines (docs/adr/0205 §6), by their place in
the list: an arc length's two radial lines; none of an ordinate's, a jogged
radius's, an azimuth's or a slope's.

The new kinds:

- ordinate (Koordinat): e is the measured axis (x for Y, `angle` 0 or
  none; y for X, `angle` 90), n the other; the line runs along u = ±n
  towards b. l = (b − a)·u ≥ 0, s = (b − a)·e. Nothing when l ≤ g. It starts
  g from a; when s ≠ 0 and l ≥ g + 2h it goes h along u, then diagonally to
  the line through b along u, 2h + g from a, then to b; otherwise straight to
  b. The value is on the last part's middle along it (u when that part has
  no length); it is a's x (Y) or y (X), prefix "Y=" or "X=", a coordinate
  (written as a length is, but no length: it is not summed as one).
- the value's box (as a text's mask measures it) runs from 0.23h under its
  baseline to 1.15h over it. A value on the side of its line away from what
  it measures stands as above; when its "above" faces what it measures it
  goes under the line instead, its box (with an arc length's symbol) as far
  under it as it would be over it: its top 0.12h under the line, its
  baseline 1.27h under it (1.72h with the symbol).
- arcLength (Yay uzunluğu): the arc about c from a, r = |a − c|,
  counter-clockwise to b's direction (sweep in (0, 2π)); the dimension arc
  at R = r + offset (nothing when r, the sweep or R is 0), as chords, 8 at
  least and one per 5°; its ends' ticks along the arc. When |offset| > g, a
  radial extension line at each end from r + k·g to R + k·g (k the offset's
  sign: 1 for 0). The value along the arc's tangent at its middle, away from
  the arc (outwards for k = 1, inwards for −1); above it a small arc, the
  symbol: half a circle of radius 0.3h about the point 1.3h above the
  value's baseline (clear of its mask), 12 chords from its right end over
  to its left. The value is r · sweep, a length, no prefix.
- jogged (Kırıklı yarıçap): r = |b − a| about the true centre a, u =
  (b − a)/r, n = u turned a quarter counter-clockwise; c is the centre shown.
  s = (c − b)·n, t = (b − c)·u; nothing when t ≤ 0 or |s| > t. The line
  goes from c along u for tA = the offset kept within [0, t − |s|], then at
  45° (|s| along u, −s along n) onto b's radius, then to b, which has a tick
  along u; a part without length is left out. The value is on the last part
  when it is 3h long or more, else on the first; r, a length, prefix "R ".
- azimuth (Semt) and slope (Eğim): d = b − a, u = d/|d| (nothing when
  |d| is under 1e-9 m), n = u turned a quarter; m = (a + b)/2 + n·offset. An arrow 3h
  long centred on m along w, its head two lines from its tip back 0.5h and
  0.2h aside. Azimuth: w = u, the value atan2(d.x, d.y) in [0, 2π), an angle,
  prefix "t=". Slope: w points downhill (u when zb < za, −u when zb > za;
  when they are equal, u and no head), the value |zb − za| / |d| · 100, a
  percentage, prefix "%". The value along u through m, away from the edge
  (on the offset's side; above when the offset is 0).

    python3 scripts/fixtures/dimension_cases.py           # writes the cases
    python3 scripts/fixtures/dimension_cases.py --check   # compares with the file
"""

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/dimension/v1/layout.json"
SQRT1_2 = math.sqrt(0.5)


def add(p, v, k):
    return (p[0] + v[0] * k, p[1] + v[1] * k)


def sub(p, q):
    return (p[0] - q[0], p[1] - q[1])


def dot(p, q):
    return p[0] * q[0] + p[1] * q[1]


def unit(v):
    length = math.hypot(v[0], v[1])
    return (v[0] / length, v[1] / length) if length > 0 else None


def tick(p, u, h):
    size = 0.6 * h
    t = ((u[0] - u[1]) * SQRT1_2 * size / 2, (u[1] + u[0]) * SQRT1_2 * size / 2)
    return [(p[0] - t[0], p[1] - t[1]), (p[0] + t[0], p[1] + t[1])]


def text_along(m, u, h):
    rotation = math.degrees(math.atan2(u[1], u[0]))
    side = 1.0
    if rotation > 90 or rotation <= -90:
        rotation += -180 if rotation > 0 else 180
        side = -1.0
    return add(m, (-u[1], u[0]), side * h * 0.35), rotation


def frame(rotation):
    """The value's reading direction and its up, from its turn in degrees."""
    turn = math.radians(rotation)
    return (math.cos(turn), math.sin(turn)), (-math.sin(turn), math.cos(turn))


def away_from(m, at, rotation, away, h, top):
    """The value at its place `at` (over its line), or under the line through m when its up faces from `away`."""
    _, up = frame(rotation)
    if dot(up, away) >= 0:
        return at
    return add(m, up, -(0.12 * h + top))


def norm_angle(t):
    t = math.fmod(t, 2 * math.pi)
    return t + 2 * math.pi if t < 0 else t


def ordinate(d):
    a, b, h = d["a"], d["b"], d["height"]
    g = h / 2
    y = d.get("angle", 0.0) == 0.0
    e, n = ((1.0, 0.0), (0.0, 1.0)) if y else ((0.0, 1.0), (1.0, 0.0))
    l = dot(sub(b, a), n)
    u = n if l >= 0 else (-n[0], -n[1])
    l, s = abs(l), dot(sub(b, a), e)
    if l <= g:
        return None
    start = add(a, u, g)
    if abs(s) > 1e-9 and l >= g + 2 * h:
        j1 = add(a, u, g + h)
        j2 = add(add(a, e, s), u, g + 2 * h)
        lines = [[start, j1], [j1, j2], [j2, b]]
        tail = (j2, b)
    else:
        lines = [[start, b]]
        tail = (start, b)
    v = unit(sub(tail[1], tail[0])) or u
    m = ((tail[0][0] + tail[1][0]) / 2, (tail[0][1] + tail[1][1]) / 2)
    at, rotation = text_along(m, v, h)
    return {
        "lines": lines,
        "textAt": at,
        "rotation": rotation,
        "value": a[0] if y else a[1],
        "unit": "coordinate",
        "prefix": "Y=" if y else "X=",
        "handle": b,
    }


def arc_length(d):
    a, b, c, h, offset = d["a"], d["b"], d["c"], d["height"], d["offset"]
    r = math.hypot(a[0] - c[0], a[1] - c[1])
    if r < 1e-9:
        return None
    t0 = math.atan2(a[1] - c[1], a[0] - c[0])
    sweep = norm_angle(math.atan2(b[1] - c[1], b[0] - c[0]) - t0)
    big = r + offset
    if sweep < 1e-9 or big < 1e-9:
        return None

    def at(t, rho):
        return (c[0] + math.cos(t) * rho, c[1] + math.sin(t) * rho)

    g = h / 2
    k = 1.0 if offset >= 0 else -1.0
    lines = []
    ext = []
    if abs(offset) > g:
        for t in (t0, t0 + sweep):
            ext.append(len(lines))
            lines.append([at(t, r + k * g), at(t, big + k * g)])
    steps = max(8, math.ceil(sweep / (math.pi / 36)))
    for i in range(steps):
        lines.append([at(t0 + sweep * i / steps, big), at(t0 + sweep * (i + 1) / steps, big)])

    def tangent(t):
        return (-math.sin(t), math.cos(t))

    lines.append(tick(at(t0, big), tangent(t0), h))
    lines.append(tick(at(t0 + sweep, big), tangent(t0 + sweep), h))
    tm = t0 + sweep / 2
    mid = at(tm, big)
    text_at, rotation = text_along(mid, tangent(tm), h)
    outwards = ((mid[0] - c[0]) / big * k, (mid[1] - c[1]) / big * k)
    text_at = away_from(mid, text_at, rotation, outwards, h, 1.6 * h)
    along, up = frame(rotation)
    centre = add(text_at, up, 1.3 * h)
    rho = 0.3 * h

    def symbol(phi):
        return add(add(centre, along, rho * math.cos(phi)), up, rho * math.sin(phi))

    for i in range(12):
        lines.append([symbol(math.pi * i / 12), symbol(math.pi * (i + 1) / 12)])
    return {
        "lines": lines,
        "ext": ext,
        "textAt": text_at,
        "rotation": rotation,
        "value": r * sweep,
        "unit": "length",
        "prefix": "",
        "handle": mid,
    }


def jogged(d):
    a, b, c, h, offset = d["a"], d["b"], d["c"], d["height"], d["offset"]
    r = math.hypot(b[0] - a[0], b[1] - a[1])
    if r < 1e-9:
        return None
    u = ((b[0] - a[0]) / r, (b[1] - a[1]) / r)
    n = (-u[1], u[0])
    s = dot(sub(c, b), n)
    t = dot(sub(b, c), u)
    jog = abs(s)
    if t <= 1e-9 or jog > t:
        return None
    ta = min(max(offset, 0.0), t - jog)
    p1 = add(c, u, ta)
    p2 = add(add(p1, u, jog), n, -s)
    lines = []
    if ta > 1e-9:
        lines.append([c, p1])
    if jog > 1e-9:
        lines.append([p1, p2])
    last = math.hypot(b[0] - p2[0], b[1] - p2[1])
    if last > 1e-9:
        lines.append([p2, b])
    lines.append(tick(b, u, h))
    part = (p2, b) if last >= 3 * h else (c, p1)
    m = ((part[0][0] + part[1][0]) / 2, (part[0][1] + part[1][1]) / 2)
    text_at, rotation = text_along(m, u, h)
    return {
        "lines": lines,
        "textAt": text_at,
        "rotation": rotation,
        "value": r,
        "unit": "length",
        "prefix": "R ",
        "handle": p1,
    }


def arrowed(d, slope):
    a, b, h, offset = d["a"], d["b"], d["height"], d["offset"]
    dd = sub(b, a)
    if math.hypot(dd[0], dd[1]) < 1e-9:
        return None
    u = unit(dd)
    n = (-u[1], u[0])
    m = add(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2), n, offset)
    head = True
    w = u
    if slope:
        za, zb = d["za"], d["zb"]
        if zb > za:
            w = (-u[0], -u[1])
        head = za != zb
    tail, tip = add(m, w, -1.5 * h), add(m, w, 1.5 * h)
    wn = (-w[1], w[0])
    lines = [[tail, tip]]
    if head:
        back = add(tip, w, -0.5 * h)
        lines.append([tip, add(back, wn, 0.2 * h)])
        lines.append([tip, add(back, wn, -0.2 * h)])
    text_at, rotation = text_along(m, u, h)
    if offset != 0:
        side = 1.0 if offset > 0 else -1.0
        text_at = away_from(m, text_at, rotation, (n[0] * side, n[1] * side), h, 1.15 * h)
    if slope:
        value, unit_name, prefix = abs(d["zb"] - d["za"]) / math.hypot(dd[0], dd[1]) * 100, "percent", "%"
    else:
        t = math.atan2(dd[0], dd[1])
        value, unit_name, prefix = (t + 2 * math.pi if t < 0 else t), "angle", "t="
    return {
        "lines": lines,
        "textAt": text_at,
        "rotation": rotation,
        "value": value,
        "unit": unit_name,
        "prefix": prefix,
        "handle": m,
    }


def layout(d):
    style = d["style"]
    if style == "ordinate":
        return ordinate(d)
    if style == "arcLength":
        return arc_length(d)
    if style == "jogged":
        return jogged(d)
    return arrowed(d, style == "slope")


def P(x, y):
    return (float(x), float(y))


CASES = [
    ("koordinat Y, düz yukarı", {"style": "ordinate", "a": P(10, 20), "b": P(10, 35), "height": 2.5}),
    ("koordinat Y, kırık sağa", {"style": "ordinate", "a": P(10, 20), "b": P(16, 40), "height": 2.5, "angle": 0.0}),
    ("koordinat Y, aşağı kırık sola", {"style": "ordinate", "a": P(10, 20), "b": P(4, 2), "height": 2.0}),
    ("koordinat X, sağa düz", {"style": "ordinate", "a": P(452345.123, 4412345.678), "b": P(452365.123, 4412345.678), "height": 2.5, "angle": 90.0}),
    ("koordinat X, sola kırık", {"style": "ordinate", "a": P(0, 0), "b": P(-20, 6), "height": 2.5, "angle": 90.0}),
    ("koordinat Y, kırığa yer yok: eğik düz", {"style": "ordinate", "a": P(0, 0), "b": P(3, 4), "height": 2.5}),
    ("koordinat Y, çizgiye yer yok", {"style": "ordinate", "a": P(0, 0), "b": P(5, 1), "height": 2.5}),
    ("yay uzunluğu, çeyrek, dışarı", {"style": "arcLength", "a": P(60, 0), "b": P(50, 10), "c": P(50, 0), "offset": 2.0, "height": 2.0}),
    ("yay uzunluğu, büyük yay, içeri", {"style": "arcLength", "a": P(0, 10), "b": P(10, 0), "c": P(0, 0), "offset": -3.0, "height": 1.5}),
    ("yay uzunluğu, uzatmasız yakın", {"style": "arcLength", "a": P(5, 0), "b": P(0, 5), "c": P(0, 0), "offset": 0.5, "height": 2.0}),
    ("yay uzunluğu, alt yarı", {"style": "arcLength", "a": P(-10, 0), "b": P(10, 0), "c": P(0, 0), "offset": 2.0, "height": 2.0}),
    ("yay uzunluğu, ölçü yayı sıfır", {"style": "arcLength", "a": P(5, 0), "b": P(0, 5), "c": P(0, 0), "offset": -5.0, "height": 2.0}),
    ("kırıklı yarıçap, uzak merkez", {"style": "jogged", "a": P(380, -260), "b": P(100, 40), "c": P(110, 30), "offset": 3.0, "height": 2.0}),
    ("kırıklı yarıçap, kırık sona sıkışır", {"style": "jogged", "a": P(0, 0), "b": P(300, 0), "c": P(280, 4), "offset": 50.0, "height": 2.0}),
    ("kırıklı yarıçap, gösterilen merkez yarıçapta", {"style": "jogged", "a": P(0, 0), "b": P(300, 0), "c": P(270, 0), "offset": 10.0, "height": 2.0}),
    ("kırıklı yarıçap, merkez yayın ötesinde", {"style": "jogged", "a": P(0, 0), "b": P(300, 0), "c": P(310, 2), "offset": 1.0, "height": 2.0}),
    ("semt, kuzeydoğu", {"style": "azimuth", "a": P(0, 50), "b": P(30, 70), "offset": 2.0, "height": 2.5}),
    ("semt, güneybatı, sağda", {"style": "azimuth", "a": P(30, 70), "b": P(0, 50), "offset": -2.0, "height": 2.5}),
    ("semt, tam kuzey", {"style": "azimuth", "a": P(0, 0), "b": P(0, 10), "offset": 1.0, "height": 2.0}),
    ("eğim, iniş a'dan b'ye", {"style": "slope", "a": P(0, 80), "b": P(40, 80), "offset": 1.5, "height": 2.0, "za": 105.25, "zb": 104.75}),
    ("eğim, iniş b'den a'ya", {"style": "slope", "a": P(0, 0), "b": P(30, 40), "offset": 2.0, "height": 2.0, "za": 10.0, "zb": 12.5}),
    ("eğim, düz", {"style": "slope", "a": P(0, 0), "b": P(20, 0), "offset": 1.0, "height": 2.0, "za": 7.0, "zb": 7.0}),
    ("eğim, kenarın altında", {"style": "slope", "a": P(0, 80), "b": P(40, 80), "offset": -1.5, "height": 2.0, "za": 105.25, "zb": 104.75}),
    ("semt, kenarın üstünde", {"style": "azimuth", "a": P(30, 70), "b": P(0, 50), "offset": 2.0, "height": 2.5}),
    ("yay uzunluğu, sağ yan, dışarı", {"style": "arcLength", "a": P(10, -10), "b": P(10, 10), "c": P(0, 0), "offset": 3.0, "height": 2.0}),
]


def pt(p):
    return {"x": p[0], "y": p[1]}


def case(name, d):
    d = {"offset": 0.0, **d}
    out = layout(d)
    dim = {k: (pt(v) if isinstance(v, tuple) else v) for k, v in d.items()}
    want = None
    if out is not None:
        want = {
            "lines": [[pt(p), pt(q)] for p, q in out["lines"]],
            "ext": out.get("ext", []),
            "textAt": pt(out["textAt"]),
            "rotation": out["rotation"],
            "value": out["value"],
            "unit": out["unit"],
            "prefix": out["prefix"],
            "handle": pt(out["handle"]),
        }
    return {"name": name, "dimension": dim, "want": want}


def document():
    return {
        "format": "kentos.dimension-cases",
        "version": 1,
        "note": "Written by scripts/fixtures/dimension_cases.py from docs/adr/0147 §2 alone; the geometry core gives the same within 1e-9 m.",
        "cases": [case(n, d) for n, d in CASES],
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)")
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(CASES)} durum kurallarla tutarlı")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(CASES)} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
