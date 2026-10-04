#!/usr/bin/env python3
"""Independent reference of Köşe tablosu's rows and writes (docs/adr/0172 §7).

Writes fixtures/vertex-table/v1/cases.json from the ADR's rules, no KentOS code: an object's paths with their elevations
(a line's two ends, a polyline, an area's outer ring, its holes, then each other part's ring and holes), the table's rows
(each vertex, the chord and the signed radius of the edge leaving it) and its writes (a vertex moved, its elevation, the
radius of its edge, a vertex added after it, vertices removed), each the new paths or the refusal's reason.

Moves, additions and removals copy coordinates and are compared bit for bit. Radii and bulges are worked out in mpmath at
50 digits: an edge of chord c and bulge b (tan θ/4, counter-clockwise positive) has the radius c(1 + b²)/(4|b|), signed
as b; a radius R back to a bulge with s = c/(2|R|) (the sine of half the angle) is s/(1 + √(1 − s²)) for the smaller arc,
(1 + √(1 − s²))/s for the larger, signed as R. The core must agree within 1e-12 (relative).
"""

import argparse
import json
import sys
from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "vertex-table" / "v1" / "cases.json"

mp.mp.dps = 50
SAME = mp.mpf("1e-9")
SLACK = 0.0005  # half a unit of three decimals: what the table shows of a length in metres


def P(x, y):
    return {"x": x, "y": y}


def path(pts, closed, zs=None, bulges=None):
    out = {"pts": [P(x, y) for x, y in pts], "closed": closed, "zs": zs if zs is not None else [None] * len(pts)}
    if bulges is not None:
        out["bulges"] = bulges
    return out


def shape(kind, paths):
    return {"kind": kind, "paths": paths}


# ── The drawing ──────────────────────────────────────────────────────────

LINE = shape("line", [path([(486500.0, 4420100.0), (486530.0, 4420140.0)], False, [812.4, None])])
ROAD = shape("polyline", [path(
    [(486500.0, 4420100.0), (486540.0, 4420110.0), (486575.5, 4420150.25), (486610.0, 4420140.0), (486650.0, 4420180.0)],
    False, [800.0, 801.25, None, 802.5, 803.0], [0.0, 0.25, -0.4, 1.5, 0.0])])
PARCEL = shape("polygon", [
    path([(486700.0, 4420200.0), (486760.0, 4420195.0), (486780.0, 4420250.0), (486730.0, 4420280.5), (486690.0, 4420245.0)],
         True, [850.1, 850.2, 850.3, 850.4, 850.5], [0.0, 0.0, 0.3, 0.0, -0.2]),
    path([(486720.0, 4420220.0), (486740.0, 4420220.0), (486740.0, 4420240.0), (486720.0, 4420240.0)], True),
    path([(486800.0, 4420200.0), (486830.0, 4420200.0), (486830.0, 4420230.0), (486800.0, 4420230.0)], True,
         [849.0, None, 849.5, None]),
])
TRIANGLE = shape("polygon", [path([(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)], True)])
ONE_ARC = shape("polyline", [path([(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)], False, None, [0.5, 0.0, 0.0])])
SHAPES = {"çizgi": LINE, "yol": ROAD, "parsel": PARCEL, "üçgen": TRIANGLE, "tek yay": ONE_ARC}


# ── The rules ────────────────────────────────────────────────────────────

def bulge_at(p, i):
    b = p.get("bulges")
    return b[i] if b is not None and i < len(b) else 0.0


def next_of(p, i):
    n = len(p["pts"])
    if i + 1 < n:
        return i + 1
    return 0 if p["closed"] and n > 1 else None


def prev_of(p, i):
    n = len(p["pts"])
    if i > 0:
        return i - 1
    return n - 1 if p["closed"] and n > 1 else None


def chord(a, b):
    return mp.hypot(mp.mpf(b["x"]) - mp.mpf(a["x"]), mp.mpf(b["y"]) - mp.mpf(a["y"]))


def radius_of(c, b):
    if abs(b) <= 1e-12 or c <= 0:
        return None
    r = c * (1 + mp.mpf(b) ** 2) / (4 * abs(mp.mpf(b)))
    return -r if b < 0 else r


def rows(sh):
    out = []
    for k, p in enumerate(sh["paths"]):
        for i, at in enumerate(p["pts"]):
            j = next_of(p, i)
            c = chord(at, p["pts"][j]) if j is not None else None
            r = radius_of(c, bulge_at(p, i)) if c is not None else None
            out.append({"path": k, "index": i, "p": at, "z": p["zs"][i],
                        "chord": None if c is None else float(c), "radius": None if r is None else float(r)})
    return out


def tidy(bulges, closed):
    bulges = list(bulges)
    if not closed and bulges:
        bulges[-1] = 0.0
    return bulges if any(abs(b) > 1e-12 for b in bulges) else None


def with_bulges(p, bulges):
    out = {k: v for k, v in p.items() if k != "bulges"}
    if bulges is not None:
        out["bulges"] = bulges
    return out


def same(a, b):
    return chord(a, b) <= SAME


def copy(sh):
    return json.loads(json.dumps(sh))


def refused(why, **more):
    return {"refusal": {"why": why, **more}}


def edited(kind, paths):
    return {"edited": {"kind": kind, "paths": paths}}


def missing(sh, k, i):
    return k >= len(sh["paths"]) or i >= len(sh["paths"][k]["pts"])


def move(sh, k, i, to):
    if missing(sh, k, i):
        return refused("missing")
    p = sh["paths"][k]
    for n in (prev_of(p, i), next_of(p, i)):
        if n is not None and n != i and same(p["pts"][n], to):
            return refused("ontoNeighbour")
    out = copy(sh)
    out["paths"][k]["pts"][i] = to
    return edited(sh["kind"], out["paths"])


def set_z(sh, k, i, z):
    if missing(sh, k, i):
        return refused("missing")
    out = copy(sh)
    out["paths"][k]["zs"][i] = z
    return edited(sh["kind"], out["paths"])


def set_radius(sh, k, i, radius, slack=SLACK):
    if missing(sh, k, i):
        return refused("missing")
    if sh["kind"] == "line":
        return refused("lineArc")
    p = sh["paths"][k]
    j = next_of(p, i)
    if j is None:
        return refused("noEdge")
    if radius is None or radius == 0:
        b = 0.0
    else:
        c = chord(p["pts"][i], p["pts"][j])
        if c <= SAME:
            return refused("noChord")
        half = c / 2
        r = abs(mp.mpf(radius))
        if r >= half:
            s = half / r
        elif half - r <= slack:
            s = mp.mpf(1)
        else:
            return refused("radiusBelow", least=float(half))
        root = mp.sqrt(1 - s * s)
        t = (1 + root) / s if abs(bulge_at(p, i)) > 1 else s / (1 + root)
        b = float(-t if radius < 0 else t)
    bulges = [bulge_at(p, n) for n in range(len(p["pts"]))]
    bulges[i] = b
    out = copy(sh)
    out["paths"][k] = with_bulges(out["paths"][k], tidy(bulges, p["closed"]))
    return edited(sh["kind"], out["paths"])


def insert(sh, k, after, at, z):
    if missing(sh, k, after):
        return refused("missing")
    p = sh["paths"][k]
    j = next_of(p, after)
    if same(p["pts"][after], at) or (j is not None and same(p["pts"][j], at)):
        return refused("ontoNeighbour")
    pts = list(p["pts"])
    zs = list(p["zs"])
    bulges = [bulge_at(p, n) for n in range(len(pts))]
    pts.insert(after + 1, at)
    zs.insert(after + 1, z)
    bulges[after] = 0.0
    bulges.insert(after + 1, 0.0)
    out = copy(sh)
    out["paths"][k] = with_bulges({"pts": pts, "closed": p["closed"], "zs": zs}, tidy(bulges, p["closed"]))
    return edited("polyline" if sh["kind"] == "line" else sh["kind"], out["paths"])


def remove(sh, at):
    gone = [[False] * len(p["pts"]) for p in sh["paths"]]
    for k, i in at:
        if missing(sh, k, i):
            return refused("missing")
        gone[k][i] = True
    if sh["kind"] == "line" and any(any(g) for g in gone):
        return refused("lineEnds")
    out = copy(sh)
    for k, p in enumerate(sh["paths"]):
        off = gone[k]
        if not any(off):
            continue
        left = off.count(False)
        if p["closed"] and left < 3:
            return refused("ringMin")
        if not p["closed"] and left < 2:
            return refused("pathMin")
        pts, zs, bulges = [], [], []
        for i in range(len(p["pts"])):
            if off[i]:
                continue
            pts.append(p["pts"][i])
            zs.append(p["zs"][i])
            j = next_of(p, i)
            bulges.append(bulge_at(p, i) if j is not None and not off[j] else 0.0)
        out["paths"][k] = with_bulges({"pts": pts, "closed": p["closed"], "zs": zs}, tidy(bulges, p["closed"]))
    return edited(sh["kind"], out["paths"])


# ── The cases ────────────────────────────────────────────────────────────

def edge_half(sh, k, i):
    p = sh["paths"][k]
    return chord(p["pts"][i], p["pts"][next_of(p, i)]) / 2


def cases():
    out = []

    def case(name, shape_name, op, expect):
        out.append({"name": name, "shape": shape_name, "op": op, "expect": expect})

    def mv(name, sh, k, i, to):
        case(name, sh, {"kind": "move", "path": k, "index": i, "to": to}, move(SHAPES[sh], k, i, to))

    def zz(name, sh, k, i, z):
        case(name, sh, {"kind": "z", "path": k, "index": i, "z": z}, set_z(SHAPES[sh], k, i, z))

    def rr(name, sh, k, i, radius):
        case(name, sh, {"kind": "radius", "path": k, "index": i, "radius": radius, "slack": SLACK},
             set_radius(SHAPES[sh], k, i, radius))

    def ins(name, sh, k, after, at, z=None):
        case(name, sh, {"kind": "insert", "path": k, "after": after, "at": at, "z": z}, insert(SHAPES[sh], k, after, at, z))

    def rm(name, sh, at):
        case(name, sh, {"kind": "remove", "at": [list(a) for a in at]}, remove(SHAPES[sh], at))

    # Moves.
    mv("yolun iç köşesi taşınır, yaylar bükümlerini korur", "yol", 0, 2, P(486580.125, 4420155.5))
    mv("alanın son köşesi taşınır, kapanan kenar yayını korur", "parsel", 0, 4, P(486688.0, 4420240.0))
    mv("deliğin köşesi taşınır", "parsel", 1, 0, P(486721.5, 4420219.0))
    mv("çizginin ucu taşınır", "çizgi", 0, 1, P(486531.25, 4420141.75))
    mv("komşu köşenin yerine taşınamaz", "yol", 0, 2, ROAD["paths"][0]["pts"][1])
    mv("kapalı halkada sonraki komşu ilk köşedir", "parsel", 0, 4, PARCEL["paths"][0]["pts"][0])
    mv("komşu olmayan köşenin yerine taşınır", "yol", 0, 0, ROAD["paths"][0]["pts"][2])
    mv("olmayan köşe", "yol", 0, 9, P(0.0, 0.0))
    # Elevations.
    zz("kotsuz köşeye kot", "yol", 0, 2, 805.125)
    zz("kot kaldırılır", "yol", 0, 0, None)
    zz("parçanın köşesine kot", "parsel", 2, 1, 849.25)
    zz("çizginin kotsuz ucuna kot", "çizgi", 0, 1, 813.0)
    # Radii.
    rr("düz kenar sola dönen küçük yay olur", "yol", 0, 0, 40.0)
    rr("yay sağa dönen yay olur", "yol", 0, 1, -35.0)
    rr("yarım daireden büyük yay büyük kalır", "yol", 0, 3, 30.0)
    rr("büyük yay sağa döner, büyük kalır", "yol", 0, 3, -45.5)
    rr("sıfır yarıçap kenarı düz yapar", "yol", 0, 1, 0.0)
    rr("boş yarıçap kenarı düz yapar", "yol", 0, 2, None)
    half = float(edge_half(ROAD, 0, 0))
    rr("payın içinde kısa yarıçap yarım daire olur", "yol", 0, 0, half - SLACK / 2)
    rr("payın içinde eksi kısa yarıçap sağa yarım daire", "yol", 0, 0, -(half - SLACK / 2))
    rr("kirişin yarısından kısa yarıçap", "yol", 0, 0, half - 1.0)
    rr("kapanan kenarın yayı değişir", "parsel", 0, 4, -80.0)
    rr("deliğin kenarı yay olur", "parsel", 1, 1, 15.0)
    rr("bükümsüz halkaya yay: büküm listesi açılır", "parsel", 2, 0, 25.0)
    rr("tek yay düzleşince büküm listesi gider", "tek yay", 0, 0, None)
    rr("çizginin kenarı yay olamaz", "çizgi", 0, 0, 50.0)
    rr("çoklu çizginin son köşesinden kenar çıkmaz", "yol", 0, 4, 50.0)
    # Additions.
    ins("yaylı kenara köşe: iki yanı düz", "yol", 0, 1, P(486560.0, 4420125.0), 810.0)
    ins("düz kenara kotsuz köşe", "yol", 0, 0, P(486520.0, 4420098.5))
    ins("kapalı halkanın son köşesinden sonra: kapanan kenar bölünür", "parsel", 0, 4, P(486694.0, 4420221.0), 850.6)
    ins("çoklu çizginin sonuna köşe", "yol", 0, 4, P(486690.0, 4420185.0), 803.5)
    ins("çizgiye köşe: çoklu çizgi olur", "çizgi", 0, 0, P(486512.0, 4420125.0))
    ins("çizginin ucundan sonra: çoklu çizgi uzar", "çizgi", 0, 1, P(486560.0, 4420150.0), 814.0)
    ins("deliğe köşe", "parsel", 1, 3, P(486718.0, 4420230.0))
    ins("komşunun yerine köşe eklenmez", "yol", 0, 1, ROAD["paths"][0]["pts"][2])
    ins("köşenin kendi yerine eklenmez", "parsel", 0, 4, PARCEL["paths"][0]["pts"][4])
    # Removals.
    rm("iç köşe silinir, iki yayı tek düz kenar olur", "yol", [(0, 2)])
    rm("ilk köşe silinir, sonraki kenar yayını korur", "yol", [(0, 0)])
    rm("son köşe silinir", "yol", [(0, 4)])
    rm("art arda iki köşe", "yol", [(0, 1), (0, 2)])
    rm("halkalardan birlikte", "parsel", [(0, 1), (0, 2), (2, 0)])
    rm("kapanan kenarın köşesi silinir", "parsel", [(0, 0)])
    rm("aynı köşe iki kez", "parsel", [(1, 2), (1, 2)])
    rm("halkada üç köşe kalmalı", "üçgen", [(0, 1)])
    rm("delikte üç köşe kalmalı", "parsel", [(1, 0), (1, 1)])
    rm("çoklu çizgide iki köşe kalmalı", "yol", [(0, 0), (0, 1), (0, 2), (0, 3)])
    rm("çizginin ucu silinmez", "çizgi", [(0, 0)])
    rm("olmayan köşe silinmez", "parsel", [(3, 0)])
    return out


def build():
    return {
        "format": "kentos.vertex-table",
        "version": 1,
        "slack": SLACK,
        "shapes": SHAPES,
        "rows": [{"shape": name, "expect": rows(sh)} for name, sh in SHAPES.items()],
        "cases": cases(),
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail when the fixture is not what the rules give")
    args = ap.parse_args()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} kurallardan çıkan değil; yeniden yazın: "
                  f"python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} kurallardan çıkanla aynı.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, encoding="utf-8")
    d = json.loads(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(d['rows'])} nesnenin satırları, {len(d['cases'])} yazma.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
