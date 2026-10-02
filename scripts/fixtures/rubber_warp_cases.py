#!/usr/bin/env python3
"""Independent reference of Kauçuk levha's objects (docs/adr/0158 §3).

Writes fixtures/fit/v1/rubber-warp.json from the rules alone, with no KentOS
code: the sheet's map and derivative come from the mpmath reference
(rubber_cases.py, 50 digits), the rules every transform shares (points,
lines, a spline's fit points, xlines and rays, texts, notes, blocks,
dimensions, hatch patterns) from warp_cases.py. Both platforms put every
case's objects on the sheet with the shared core (`ops::warp::sheet_shapes`,
WASM `rubberShapes`) and must give the same geometry, the same elevations
and the same counts.

The rules a sheet has of its own:

1. Only vertices move: a path's vertices (a polygon's holes and parts too)
   by f; straight edges stay straight, an arc segment keeps its bulge;
   elevations stay with their vertices.
2. A circle, an arc and an ellipse move by the nearest similarity at their
   centre: with J there, s = √|det J|, θ the angle of J's first column, S =
   s·[[cos θ, −sin θ], [sin θ, cos θ]], or s·[[cos θ, sin θ], [sin θ, −cos θ]]
   when J mirrors. The centre goes to f(c); a circle's radius and an arc's
   radius take s; an arc's angles are those of S·(its start) and S·(its end)
   from the centre (the end's and the start's when S mirrors), from 0 up to
   2π; an ellipse's major is S·major, its ratio stays, its parameters stay
   (a part of it under a mirror: −t1, −t0, from 0 up to 2π).
3. The bend of a shape: the largest distance between f of a point and the
   kept shape's point at the same parameter. A straight edge (of a line, a
   path, a ring, a hatch ring, a leader) at t = ¼, ½, ¾: f(a + t·(b − a))
   against f(a) + t·(f(b) − f(a)); an arc segment at the same t along its
   arc and along the arc of the same bulge between f(a) and f(b). A
   circle, an arc and an ellipse at the middles of eight equal parts of
   their span: f(c + v) against f(c) + S·v, v = M·cos t + N·sin t. Other
   kinds bend 0. A shape bends when its bend is over 0.1 mm.

What is compared: every object's kind and fields (coordinates within
1e-9 m, angles within 1e-12 rad, other numbers within 1e-12 of their size,
at least 1), its paths' elevations, the counts (kept their shape, bent) and
the largest bend (within 1e-9 m).
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import rubber_cases as rc  # noqa: E402
import warp_cases as wc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "fit" / "v1" / "rubber-warp.json"
TAU = 2 * math.pi
BENT = 1e-4


class SheetMap:
    """The sheet as warp_cases' rules ask for it: f, the derivative, its kind."""

    kind = "sheet"

    def __init__(self, links):
        self.sheet = rc.solve(links)

    def f(self, p):
        image, _ = rc.evaluate(self.sheet, {"x": p[0], "y": p[1]})
        return (image["x"], image["y"])

    def jac(self, p):
        _, jac = rc.evaluate(self.sheet, {"x": p[0], "y": p[1]})
        return tuple(jac)


def similarity(j):
    """Rule 2: S as (a, b, c, d), its columns (a, b) and (c, d), and whether it mirrors."""
    s = math.sqrt(abs(wc.det(j)))
    theta = math.atan2(j[1], j[0])
    c, sn = s * math.cos(theta), s * math.sin(theta)
    if wc.det(j) < 0:
        return (c, sn, sn, -c), s, True
    return (c, sn, -sn, c), s, False


def norm(a):
    r = math.fmod(a, TAU)
    return r + TAU if r < 0 else r


def angle(v):
    return norm(math.atan2(v[1], v[0]))


def edge_bend(m, a, b, bulge):
    """Rule 3 for one edge."""
    na, nb = m.f(a), m.f(b)
    worst = 0.0
    straight = abs(bulge) <= 1e-12 or math.hypot(b[0] - a[0], b[1] - a[1]) < 1e-12
    for t in (0.25, 0.5, 0.75):
        if straight:
            real = m.f((a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t))
            here = (na[0] + (nb[0] - na[0]) * t, na[1] + (nb[1] - na[1]) * t)
        else:
            c, r, a0, sw = wc.bulge_arc(a, b, bulge)
            kc, kr, ka0, ksw = wc.bulge_arc(na, nb, bulge)
            u, v = a0 + sw * t, ka0 + ksw * t
            real = m.f((c[0] + r * math.cos(u), c[1] + r * math.sin(u)))
            here = (kc[0] + kr * math.cos(v), kc[1] + kr * math.sin(v))
        worst = max(worst, math.hypot(real[0] - here[0], real[1] - here[1]))
    return worst


def ring_bend(m, pts, bulges, closed):
    pts = [wc.xy(p) for p in pts]
    n = len(pts)
    edges = n if closed else n - 1
    worst = 0.0
    for i in range(edges):
        bg = bulges[i] if bulges and i < len(bulges) else 0.0
        worst = max(worst, edge_bend(m, pts[i], pts[(i + 1) % n], bg))
    return worst


def curve_bend(m, c, mv, nv, span):
    (sa, sb, sc, sd), _, _ = similarity(m.jac(c))
    to = m.f(c)
    worst = 0.0
    for k in range(8):
        t = span[0] + (span[1] - span[0]) * (k + 0.5) / 8
        v = (mv[0] * math.cos(t) + nv[0] * math.sin(t), mv[1] * math.cos(t) + nv[1] * math.sin(t))
        real = m.f((c[0] + v[0], c[1] + v[1]))
        here = (to[0] + sa * v[0] + sc * v[1], to[1] + sb * v[0] + sd * v[1])
        worst = max(worst, math.hypot(real[0] - here[0], real[1] - here[1]))
    return worst


def ring(m, r):
    out = {"pts": [wc.pt(m.f(wc.xy(p))) for p in r["pts"]]}
    if r.get("bulges") is not None:
        out["bulges"] = r["bulges"]
    return out


def sheet_object(m, o, zs):
    """(shape, zs, kept, bend) for one object on the sheet."""
    k = o["kind"]
    if k == "polyline":
        out = {"kind": "polyline", **ring(m, o)}
        return out, zs, 0, ring_bend(m, o["pts"], o.get("bulges"), False)
    if k == "polygon":
        out = {"kind": "polygon", **ring(m, o)}
        bend = ring_bend(m, o["pts"], o.get("bulges"), True)
        if o.get("holes"):
            out["holes"] = [ring(m, h) for h in o["holes"]]
            for h in o["holes"]:
                bend = max(bend, ring_bend(m, h["pts"], h.get("bulges"), True))
        if o.get("parts"):
            parts = []
            for part in o["parts"]:
                q = ring(m, part)
                bend = max(bend, ring_bend(m, part["pts"], part.get("bulges"), True))
                if part.get("holes"):
                    q["holes"] = [ring(m, h) for h in part["holes"]]
                    for h in part["holes"]:
                        bend = max(bend, ring_bend(m, h["pts"], h.get("bulges"), True))
                parts.append(q)
            out["parts"] = parts
        return out, zs, 0, bend
    if k in ("circle", "arc", "ellipse"):
        c = wc.xy(o["c"])
        (sa, sb, sc, sd), s, mirrors = similarity(m.jac(c))
        to = m.f(c)

        def S(v):
            return (sa * v[0] + sc * v[1], sb * v[0] + sd * v[1])

        if k == "circle":
            out = {"kind": "circle", "c": wc.pt(to), "r": o["r"] * s}
            return out, zs, 0, curve_bend(m, c, (o["r"], 0.0), (0.0, o["r"]), (0.0, TAU))
        if k == "arc":
            r = o["r"]
            start = S((r * math.cos(o["a0"]), r * math.sin(o["a0"])))
            end = S((r * math.cos(o["a1"]), r * math.sin(o["a1"])))
            a0, a1 = (angle(end), angle(start)) if mirrors else (angle(start), angle(end))
            out = {"kind": "arc", "c": wc.pt(to), "r": r * s, "a0": a0, "a1": a1}
            span = (o["a0"], o["a0"] + wc.sweep(o["a0"], o["a1"]))
            return out, zs, 0, curve_bend(m, c, (r, 0.0), (0.0, r), span)
        mv = wc.xy(o["major"])
        nv = (-mv[1] * o["ratio"], mv[0] * o["ratio"])
        t0, t1 = o["t0"], o["t1"]
        full = wc.sweep(t0, t1) >= TAU - 1e-12
        out = {"kind": "ellipse", "c": wc.pt(to), "major": wc.pt(S(mv)), "ratio": o["ratio"]}
        out["t0"], out["t1"] = (norm(-t1), norm(-t0)) if mirrors and not full else (t0, t1)
        return out, zs, 0, curve_bend(m, c, mv, nv, (t0, t0 + wc.sweep(t0, t1)))
    shape, z, _, kept = wc.warp_object(m, o, zs)
    bend = 0.0
    if k == "line":
        bend = edge_bend(m, wc.xy(o["a"]), wc.xy(o["b"]), 0.0)
    elif k == "leader":
        bend = ring_bend(m, o["pts"], None, False)
    elif k == "hatch":
        bend = ring_bend(m, o["ring"], None, True)
        for h in o.get("holes") or []:
            bend = max(bend, ring_bend(m, h, None, True))
    return shape, zs, kept, bend


def sheet_all(links, objects, zs):
    m = SheetMap(links)
    if "error" in m.sheet:
        return {"error": m.sheet["error"]}
    out, out_zs, kept, bent, bend = [], [], 0, 0, 0.0
    for o, z in zip(objects, zs):
        shape, z2, k, b = sheet_object(m, o, z)
        out.append(shape)
        out_zs.append(z2)
        kept += k
        bent += 1 if b > BENT else 0
        bend = max(bend, b)
    return {"shapes": out, "zs": out_zs, "kept": kept, "bent": bent, "bend": bend}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def objects():
    """A local survey's drawing (metres around 1000, 2000) and each object's paths' elevations."""
    o = [
        ({"kind": "point", "p": P(1012.5, 2004.25), "z": 101.25}, []),
        ({"kind": "line", "a": P(970, 1980), "b": P(1040, 2015)}, [[100.0, 102.5]]),
        ({"kind": "polyline", "pts": [P(960, 2030), P(990, 2030), P(1010, 2050)], "bulges": [0.0, 0.5, 0.0]}, [[100.0, 101.0, 103.0]]),
        (
            {
                "kind": "polygon",
                "pts": [P(1000, 2000), P(1030, 2000), P(1030, 2020), P(1000, 2020)],
                "bulges": [0.0, 0.0, -0.3, 0.0],
                "holes": [{"pts": [P(1005, 2005), P(1010, 2005), P(1010, 2010), P(1005, 2010)]}],
            },
            [[100.0, 100.5, 101.0, None], [None, None, None, None]],
        ),
        (
            {
                "kind": "polygon",
                "pts": [P(1060, 1960), P(1080, 1960), P(1080, 1975), P(1060, 1975)],
                "parts": [
                    {"pts": [P(1060, 1960), P(1080, 1960), P(1080, 1975), P(1060, 1975)]},
                    {"pts": [P(1090, 1960), P(1100, 1960), P(1095, 1970)], "holes": [{"pts": [P(1093, 1962), P(1097, 1962), P(1095, 1965)]}]},
                ],
            },
            [[None] * 4, [None] * 4, [None] * 3, [None] * 3],
        ),
        ({"kind": "circle", "c": P(980, 1990), "r": 6.0}, []),
        ({"kind": "arc", "c": P(1025, 1985), "r": 8.0, "a0": math.pi / 6, "a1": 5 * math.pi / 6}, []),
        ({"kind": "ellipse", "c": P(995, 2040), "major": P(9, 3), "ratio": 0.5, "t0": 0.0, "t1": TAU}, []),
        ({"kind": "ellipse", "c": P(1035, 2035), "major": P(-4, 7), "ratio": 0.6, "t0": 0.4, "t1": 2.6}, []),
        ({"kind": "spline", "pts": [P(965, 1965), P(975, 1972), P(988, 1967), P(1000, 1974)], "closed": False}, []),
        ({"kind": "xline", "p": P(1005, 1960), "dir": P(0.6, 0.8)}, []),
        ({"kind": "ray", "p": P(1010, 1970), "dir": P(-0.8, 0.6)}, []),
        ({"kind": "text", "p": P(1015, 2025), "text": "Ada 101", "height": 2.0, "rotation": 30.0}, []),
        ({"kind": "insert", "block": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001", "p": P(985, 2015), "scale": 1.0, "rotation": 0.25}, []),
        ({"kind": "hatch", "ring": [P(1040, 1960), P(1055, 1960), P(1055, 1975), P(1040, 1975)], "pattern": {"type": "lines", "angle": 45.0, "spacing": 1.5}}, []),
        ({"kind": "dimension", "a": P(960, 1955), "b": P(990, 1955), "offset": 4.0, "height": 2.5}, []),
        ({"kind": "leader", "pts": [P(1020, 2005), P(1028, 2012), P(1033, 2012)], "text": "Sınır taşı", "height": 1.8, "rotation": 0.0}, []),
    ]
    return [x[0] for x in o], [x[1] for x in o]


def link(fx, fy, tx, ty):
    return {"from": {"x": float(fx), "y": float(fy)}, "to": {"x": float(tx), "y": float(ty)}}


def sheets():
    """Sheets over the drawing: a grid of fixed points with moved crosses, and a strong one that bends."""
    gentle = []
    for i in range(5):
        for j in range(5):
            x, y = 950.0 + 40.0 * i, 1950.0 + 25.0 * j
            if i in (1, 2, 3) and j in (1, 2, 3):
                gentle.append(link(x, y, x + 0.012 * (i - 2) + 0.004 * j, y - 0.009 * (j - 2) + 0.003 * i))
            else:
                gentle.append(link(x, y, x, y))
    strong = [link(950, 1950, 950, 1950), link(1110, 1950, 1110.4, 1949.2), link(950, 2050, 949.1, 2050.6), link(1110, 2050, 1110, 2050), link(1030, 2000, 1031.5, 1998.7), link(1000, 1975, 999.2, 1976.1)]
    return {"pafta": gentle, "guclu": strong}


def cases():
    objs, zs = objects()
    s = sheets()
    rows = []
    for name, sheet in [
        ("Pafta ızgarası: kenarı sabit, içteki kesişimler santimetrelerle kayar; köşeler taşınır, biçimler korunur", "pafta"),
        ("Güçlü kauçuk levha: metrelerce kayma, eğriler ve kenarlar 0,1 mm'den çok sapar ve sayılır", "guclu"),
    ]:
        rows.append({"name": name, "sheet": sheet, "objects": objs, "zs": zs, "expected": sheet_all(s[sheet], objs, zs)})
    rows.append(
        {
            "name": "Bağlar bir doğru üstünde: levha yok",
            "sheet": "dogru",
            "objects": objs[:2],
            "zs": zs[:2],
            "expected": sheet_all([link(0, 0, 0, 1), link(1, 1, 1, 2), link(2, 2, 2, 3)], objs[:2], zs[:2]),
        }
    )
    return rows


def build():
    s = sheets()
    s["dogru"] = [link(0, 0, 0, 1), link(1, 1, 1, 2), link(2, 2, 2, 3)]
    return {
        "format": "kentos.fit-rubber-warp",
        "version": 1,
        "description": "Kauçuk levha'da nesneler (docs/adr/0158 §3): yalnız köşeler taşınır, daire, yay ve elips merkezlerindeki en yakın benzerlikle, öbür türler dönüşümlerin kuralıyla; gerçek görüntüden sapma. scripts/fixtures/rubber_warp_cases.py yazar.",
        "tolerance": {"metres": 1e-9, "radians": 1e-12, "relative": 1e-12},
        "sheets": s,
        "cases": cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--check", action="store_true", help="compare with the written file instead of writing it")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT} güncel değil: python3 scripts/fixtures/rubber_warp_cases.py", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT}: güncel")
        return
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    print(f"{OUT}: {len(json.loads(text)['cases'])} durum")


if __name__ == "__main__":
    main()
