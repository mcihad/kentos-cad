#!/usr/bin/env python3
"""The shared cases of `road.terrains.stockpile` (Stok hacmi; plan/map-cad.md MAP-15, docs/adr/3140), without KentOS
code.

    uv run --no-project --python 3.11 --with triangle --with numpy python3 scripts/fixtures/road_terrain_stockpile_cases.py
    ... --check   # compares the file with the one on disk

The surface is Shewchuk's Triangle's (road_terrain_command_cases.py's build) of 256 points on a 4 m grid shaken by up to
1 m, at map coordinates (x from 487 000, y from 4 420 000): ground rising 1 cm a metre east and 0.5 cm north, and a pile
6 m high (a cone of 14 m radius, rounded) on it. The boundaries: a pentagon around the pile (a polygon), an L (a closed
polyline, concave), a ring with a hole (a polygon), one reaching beyond the surface, an open polyline, an arc and three
corners on a line.

The rules, by hand (Pix4D's volume base surfaces, docs/adr/3140):
- the outer corners' heights on the surface: the triangle's plane (exact fractions, road_surface_cases.py);
- the base: level at the lowest, highest or average height, the plane z = a·x + b·y + c of least vertical squares
  through them (exact normal equations, centred on the corners' mean), or level at a height given;
- inside the boundary (outer ring counterclockwise, holes clockwise) each triangle of the surface clips each ring
  (Sutherland–Hodgman, the triangle the window), the part is split where surface − base is zero, and the linear
  difference is integrated from the parts' areas and first moments, all in exact fractions; above the base is cut,
  below it fill, total their sum.
"""

import copy
import json
import math
import random
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_command_cases as rt  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.terrains.stockpile.json"
W = 1e-6
X0, Y0 = 487000.0, 4420000.0


def U(n):
    return f"0192f5c0-3140-7000-8000-{n:012x}"


def ground(x, y):
    """The ground's height at local (x, y): the slope and the pile (a cone of 14 m, its top rounded within 3 m)."""
    z = 100.0 + 0.01 * x + 0.005 * y
    r = math.hypot(x - 30.0, y - 30.0)
    if r < 14.0:
        z += 6.0 * (1.0 - r / 14.0) if r > 3.0 else 6.0 * (1.0 - r / 14.0) + 0.2 * (1.0 - (r / 3.0) ** 2)
    return z


def shaken_points():
    rnd = random.Random(3140)
    out = []
    for j in range(16):
        for i in range(16):
            x = min(60.0, max(0.0, i * 4.0 + rnd.uniform(-1.0, 1.0)))
            y = min(60.0, max(0.0, j * 4.0 + rnd.uniform(-1.0, 1.0)))
            x, y = round(x, 3), round(y, 3)
            out.append((round(X0 + x, 3), round(Y0 + y, 3), round(ground(x, y), 3)))
    return out


POINTS = shaken_points()
TWO = [(X0 + 500.0, Y0 + 500.0, 100.0), (X0 + 510.0, Y0 + 500.0, 101.0)]


def L(pts):
    return [(round(X0 + x, 3), round(Y0 + y, 3)) for x, y in pts]


PENTAGON = L([(14.0, 16.0), (46.0, 14.0), (50.0, 34.0), (31.0, 49.0), (12.0, 36.0)])
ELL = L([(12.0, 12.0), (48.0, 12.0), (48.0, 26.0), (26.0, 26.0), (26.0, 48.0), (12.0, 48.0)])
HOLED = (L([(10.0, 10.0), (50.0, 10.0), (50.0, 50.0), (10.0, 50.0)]), L([(25.0, 25.0), (25.0, 35.0), (35.0, 35.0), (35.0, 25.0)]))
BEYOND = L([(40.0, 40.0), (70.0, 40.0), (70.0, 58.0), (40.0, 58.0)])
OPEN = L([(5.0, 5.0), (20.0, 5.0), (20.0, 20.0)])
ARC = L([(5.0, 40.0), (15.0, 40.0), (15.0, 50.0), (5.0, 50.0)])
LINE = L([(5.0, 55.0), (15.0, 55.0), (25.0, 55.0)])


def P(p):
    return {"x": float(p[0]), "y": float(p[1])}


def entities():
    ents = [("point", "zemin", {"p": P((x, y)), "z": z}) for x, y, z in POINTS]
    ents.append(("polygon", "sinir", {"pts": [P(p) for p in PENTAGON]}))
    ents.append(("polyline", "sinir", {"pts": [P(p) for p in ELL], "closed": True}))
    ents.append(("polygon", "sinir", {"pts": [P(p) for p in HOLED[0]], "holes": [{"pts": [P(p) for p in HOLED[1]]}]}))
    ents.append(("polygon", "sinir", {"pts": [P(p) for p in BEYOND]}))
    ents.append(("polyline", "sinir", {"pts": [P(p) for p in OPEN]}))
    ents.append(("polyline", "sinir", {"pts": [P(p) for p in ARC], "closed": True, "bulges": [0.0, 0.4, 0.0, 0.0]}))
    ents.append(("polyline", "sinir", {"pts": [P(p) for p in LINE], "closed": True}))
    ents += [("point", "az", {"p": P((x, y)), "z": z}) for x, y, z in TWO]
    return ents


ENTS = entities()
FIRST = len(POINTS) + 1  # the pentagon's slot; the others follow
B = {name: U(FIRST + k) for k, name in enumerate(["pentagon", "ell", "holed", "beyond", "open", "arc", "line"])}
RINGS = {"pentagon": (PENTAGON, []), "ell": (ELL, []), "holed": (HOLED[0], [HOLED[1]]), "beyond": (BEYOND, []),
         "line": (LINE, [])}
SURFACES = [{"id": "t-1", "name": "Yığın", "points": {"layers": ["zemin"]}},
            {"id": "t-9", "name": "Az nokta", "points": {"layers": ["az"]}}]
LAYERS = [rt.node("0", "0"), rt.node("zemin", "Zemin"), rt.node("sinir", "Sınır"), rt.node("az", "Az nokta")]


def setup(work="map"):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 500.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work in ("map", "road"):
        settings["road"] = {"surfaces": copy.deepcopy(SURFACES)}
    return {"format": "kentos.document", "version": 2, "name": "Stok hacmi", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "0",
            "entities": [{"kind": k, "id": i + 1, "layerId": l, "attrs": {}, **copy.deepcopy(g)}
                         for i, (k, l, g) in enumerate(ENTS)],
            "uids": [U(i + 1) for i in range(len(ENTS))], "styles": {"items": [], "categories": []}}


S = rt.build({"points": [(x, y, z, U(i + 1)) for i, (x, y, z) in enumerate(POINTS)], "breaklines": [], "boundary": None,
              "voids": [], "tolerance": None, "maxEdge": None, "edges": [], "removed": []})


# ── The reference ───────────────────────────────────────────────────────────────────────────────────────────────────

def signed_area(ring):
    n = len(ring)
    return sum(ring[i][0] * ring[(i + 1) % n][1] - ring[(i + 1) % n][0] * ring[i][1] for i in range(n)) / 2


def oriented(ring, ccw):
    return ring if (signed_area(ring) > 0) == ccw else list(reversed(ring))


def keep(poly, f):
    """Sutherland–Hodgman against the half-plane f ≥ 0 (f linear), exact."""
    out = []
    n = len(poly)
    for i in range(n):
        p, q = poly[i], poly[(i + 1) % n]
        fp, fq = f(p), f(q)
        if fp >= 0:
            out.append(p)
        if (fp >= 0) != (fq >= 0):
            t = fp / (fp - fq)
            out.append((p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t))
    return out


def moments(poly):
    a = sx = sy = F(0)
    n = len(poly)
    for i in range(n):
        p, q = poly[i], poly[(i + 1) % n]
        w = p[0] * q[1] - q[0] * p[1]
        a += w
        sx += (p[0] + q[0]) * w
        sy += (p[1] + q[1]) * w
    return a / 2, sx / 6, sy / 6


def corner_heights(outer):
    out = []
    for k, p in enumerate(outer):
        h = rt.height(S, p)
        if h["place"] != "inside":
            return k
        out.append(h["z"])
    return out


def base_plane(kind, outer, heights, elevation=None):
    if kind == "elevation":
        return [0.0, 0.0, float(elevation)]
    if kind == "lowest":
        return [0.0, 0.0, min(heights)]
    if kind == "highest":
        return [0.0, 0.0, max(heights)]
    if kind == "average":
        return [0.0, 0.0, float(sum(F(h) for h in heights) / len(heights))]
    # fit: exact least squares of the vertical differences
    pts = [(F(p[0]), F(p[1]), F(h)) for p, h in zip(outer, heights)]
    n = len(pts)
    mx, my, mz = sum(p[0] for p in pts) / n, sum(p[1] for p in pts) / n, sum(p[2] for p in pts) / n
    suu = sum((p[0] - mx) ** 2 for p in pts)
    svv = sum((p[1] - my) ** 2 for p in pts)
    suv = sum((p[0] - mx) * (p[1] - my) for p in pts)
    suz = sum((p[0] - mx) * (p[2] - mz) for p in pts)
    svz = sum((p[1] - my) * (p[2] - mz) for p in pts)
    det = suu * svv - suv * suv
    if not det > F(1, 10 ** 12) * (suu + svv) ** 2:
        return None
    a = (suz * svv - svz * suv) / det
    b = (svz * suu - suz * suv) / det
    return [float(a), float(b), float(mz - a * mx - b * my)]


def measure(outer, holes, plane):
    """The cut, fill and areas inside the rings against the plane, exact."""
    o = (F(outer[0][0]), F(outer[0][1]))
    sh = lambda ring: [(F(p[0]) - o[0], F(p[1]) - o[1]) for p in ring]
    rings = [oriented(sh(outer), True)] + [oriented(sh(h), False) for h in holes]
    pa, pb, pc = F(plane[0]), F(plane[1]), F(plane[2])
    base_c = pa * o[0] + pb * o[1] + pc
    out = {"area": sum(signed_area(r) for r in rings), "covered": F(0), "cut": F(0), "fill": F(0), "cutArea": F(0),
           "fillArea": F(0)}
    for t in S["tris"]:
        v = [((F(S["xy"][i][0]) - o[0], F(S["xy"][i][1]) - o[1]), F(S["z"][i])) for i in t]
        turn = lambda v: (v[1][0][0] - v[0][0][0]) * (v[2][0][1] - v[0][0][1]) - (v[1][0][1] - v[0][0][1]) * (v[2][0][0] - v[0][0][0])
        if turn(v) < 0:
            v[1], v[2] = v[2], v[1]
        det = turn(v)
        if det <= 0:
            continue
        e1 = (v[1][0][0] - v[0][0][0], v[1][0][1] - v[0][0][1])
        e2 = (v[2][0][0] - v[0][0][0], v[2][0][1] - v[0][0][1])
        dz1, dz2 = v[1][1] - v[0][1], v[2][1] - v[0][1]
        sa = (dz1 * e2[1] - dz2 * e1[1]) / det
        sb = (e1[0] * dz2 - e2[0] * dz1) / det
        sc = v[0][1] - sa * v[0][0][0] - sb * v[0][0][1]
        da, db, dc = sa - pa, sb - pb, sc - base_c
        for ring in rings:
            poly = ring
            for k in range(3):
                a, b = v[k][0], v[(k + 1) % 3][0]
                poly = keep(poly, lambda p, a=a, b=b: (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]))
                if len(poly) < 3:
                    break
            if len(poly) < 3:
                continue
            out["covered"] += moments(poly)[0]
            for above in (True, False):
                part = keep(poly, (lambda p: da * p[0] + db * p[1] + dc) if above else (lambda p: -(da * p[0] + db * p[1] + dc)))
                if len(part) < 3:
                    continue
                a_, sx, sy = moments(part)
                integral = da * sx + db * sy + dc * a_
                if above:
                    out["cut"] += integral
                    out["cutArea"] += a_
                else:
                    out["fill"] -= integral
                    out["fillArea"] += a_
    return {k: float(v) for k, v in out.items()}


SAID = {"lowest": "köşelerin en alçak kotunda yatay düzlem", "highest": "köşelerin en yüksek kotunda yatay düzlem",
        "average": "köşelerin ortalama kotunda yatay düzlem", "fit": "köşelere en küçük kareler düzlemi (düşey farklar)",
        "elevation": "verilen kotta yatay düzlem"}


def answer(boundary, base, elevation=None, surface="t-1"):
    outer, holes = RINGS[boundary]
    heights = corner_heights(outer)
    if isinstance(heights, int):
        heights = []
    plane = base_plane(base, outer, heights, elevation)
    m = measure(outer, holes, plane)
    uncovered = max(m["area"] - m["covered"], 0.0)
    out = {"surface": "t-1", "base": base, "plane": plane}
    if heights:
        out["corners"] = heights
    out.update({"area": m["area"], "covered": m["covered"], "uncovered": uncovered, "cut": m["cut"], "fill": m["fill"],
                "total": m["cut"] + m["fill"], "cutArea": m["cutArea"], "fillArea": m["fillArea"],
                "notes": [f"Taban: {SAID[base]}; tabanın üstü kazı (yığın), altı dolgu (Pix4D'nin hacim tabanları). "
                          f"Sınırın {len(outer)} köşesi.",
                          "Hacim yüzeyin üçgenleriyle sınırın kesişiminde kesin tümlenir: doğrusal fark, parçanın alanı ve "
                          "momentleriyle."]})
    warns = []
    if uncovered > 1e-6 * max(m["area"], 1.0):
        warns.append({"code": "terrain_no_data",
                      "message": f"“Yığın” yüzeyi sınırın {uncovered:.1f} m²'sini örtmüyor (%{100 * uncovered / m['area']:.1f}); "
                                 "orada hacim ölçülmedi.", "path": "boundary"})
    return out, warns


# ── The cases ───────────────────────────────────────────────────────────────────────────────────────────────────────

N = len(ENTS)


def read(cases, name, input_, op="plan", setup_=None, **kw):
    out, warns = answer(**kw)
    result = rt.completed(out if op != "validate" else None, warns)
    c = {"name": name, "steps": [rt.step(op, input_, result, {**rt.UNCHANGED, "ids": list(range(1, N + 1))}, W)]}
    if setup_ is not None:
        c["setup"] = setup_
    cases.append(c)
    return out


def refused(cases, name, input_, result, setup_=None):
    c = {"name": name, "steps": [rt.step("execute", input_, result, {**rt.UNCHANGED, "ids": list(range(1, N + 1))})]}
    if setup_ is not None:
        c["setup"] = setup_
    cases.append(c)


def cases():
    out = []
    pent = {"surface": "t-1", "boundary": B["pentagon"]}
    low = read(out, "Beşgen, en alçak köşe: yatay taban köşelerin en alçağında, yığın kazı", {**pent, "base": "lowest"},
               boundary="pentagon", base="lowest")
    assert low["cut"] > 0 and low["fill"] == 0.0, low
    read(out, "Beşgen, en yüksek köşe: tabanın altı dolgu", {**pent, "base": "highest"}, boundary="pentagon", base="highest")
    read(out, "Beşgen, köşelerin ortalaması", {**pent, "base": "average"}, boundary="pentagon", base="average")
    fit = read(out, "Beşgen, köşelere oturan eğimli düzlem (en küçük kareler)", {**pent, "base": "fit"}, boundary="pentagon",
               base="fit")
    assert abs(fit["plane"][0] - 0.01) < 0.01 and fit["cut"] > 0, fit
    read(out, "Beşgen, verilen kot 101,5", {**pent, "base": "elevation", "elevation": 101.5}, boundary="pentagon",
         base="elevation", elevation=101.5)
    below = read(out, "Beşgen, her yerin altındaki kot: hepsi kazı", {**pent, "base": "elevation", "elevation": 95.0},
                 boundary="pentagon", base="elevation", elevation=95.0)
    assert below["fill"] == 0.0 and abs(below["cutArea"] - below["area"]) < 1e-6, below
    above = read(out, "Beşgen, her yerin üstündeki kot: hepsi dolgu", {**pent, "base": "elevation", "elevation": 120.0},
                 boundary="pentagon", base="elevation", elevation=120.0)
    assert above["cut"] == 0.0 and abs(above["fillArea"] - above["area"]) < 1e-6, above
    read(out, "Yüzeyin adıyla", {"surface": "Yığın", "boundary": B["pentagon"], "base": "fit"}, boundary="pentagon", base="fit")
    read(out, "Doğrulama", {**pent, "base": "lowest"}, op="validate", boundary="pentagon", base="lowest")
    read(out, "Yazma da okur: hiçbir şey yazılmaz", {**pent, "base": "lowest"}, op="execute", boundary="pentagon",
         base="lowest")
    ell = {"surface": "t-1", "boundary": B["ell"]}
    read(out, "L (kapalı çoklu çizgi, iç bükey), eğimli düzlem", {**ell, "base": "fit"}, boundary="ell", base="fit")
    read(out, "L, en alçak köşe", {**ell, "base": "lowest"}, boundary="ell", base="lowest")
    holed = {"surface": "t-1", "boundary": B["holed"]}
    h = read(out, "Delikli alan, köşelerin ortalaması: deliğin hacmi sayılmaz", {**holed, "base": "average"},
             boundary="holed", base="average")
    assert abs(h["area"] - 1500.0) < 1e-9, h
    read(out, "Delikli alan, verilen kot", {**holed, "base": "elevation", "elevation": 101.0}, boundary="holed",
         base="elevation", elevation=101.0)
    beyond = {"surface": "t-1", "boundary": B["beyond"]}
    over = read(out, "Yüzeyi aşan sınır, verilen kot: örtülmeyen yer uyarıyla, köşe kotları yok",
                {**beyond, "base": "elevation", "elevation": 100.5}, boundary="beyond", base="elevation", elevation=100.5)
    assert over["uncovered"] > 0 and "corners" not in over, over
    refused(out, "Yüzeyi aşan sınır, köşeden taban: dışarıdaki köşe ret", {**beyond, "base": "lowest"},
            rt.failed("invalid_stockpile", f"Sınırın 2. köşesi ({BEYOND[1][0]:.3f}, {BEYOND[1][1]:.3f}) yüzeyin dışında; "
                      "taban köşelerin yüzeydeki kotlarından çizilir. Sınırı yüzeyin içine çizin ya da kotla taban verin.",
                      "boundary"))
    refused(out, "Bilinmeyen taban", {**pent, "base": "plane"},
            rt.failed("invalid_stockpile", "“plane” bir taban değil: lowest, highest, average, fit ya da elevation olmalı.",
                      "base"))
    refused(out, "Kotla taban, kot yok", {**pent, "base": "elevation"},
            rt.failed("invalid_stockpile", "Kotla taban (elevation) için sonlu bir kot verin.", "elevation"))
    refused(out, "Yol ya da Harita projesi değil", {**pent, "base": "lowest"}, rt.NOT_ROAD, setup("architecture"))
    refused(out, "Olmayan yüzey", {"surface": "yok", "boundary": B["pentagon"], "base": "lowest"},
            rt.failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    refused(out, "Kurulamayan yüzey", {"surface": "t-9", "boundary": B["pentagon"], "base": "lowest"},
            rt.failed("terrain_unbuildable", f"Kaynaklardan yüzey kurulamıyor: {rt.PROBLEM['too_few_points']}."))
    missing = U(99999)
    refused(out, "Olmayan sınır", {**pent, "boundary": missing, "base": "lowest"},
            rt.failed("entity_not_found", f"“{missing}” kimlikli nesne çizimde yok.", "boundary"))
    for key, title in (("open", "Açık çoklu çizgi sınır olamaz"), ("arc", "Yaylı kenar sınır olamaz")):
        refused(out, title, {"surface": "t-1", "boundary": B[key], "base": "lowest"},
                rt.failed("invalid_coverage", f"“{B[key]}” sınır olamaz: düz kenarlı, tek parçalı kapalı bir çoklu çizgi ya "
                          "da alan olmalı.", "boundary"))
    refused(out, "Köşeleri bir doğru üstünde: eğimli düzlem çizilemez", {"surface": "t-1", "boundary": B["line"], "base": "fit"},
            rt.failed("invalid_stockpile", "Sınırın köşeleri bir doğru üstünde; eğimli taban düzlemi çizilemez.", "base"))
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.stockpile", "commandVersion": 1,
           "title": "Stok hacmi (Pix4D'nin hacim tabanları)",
           "note": "ADR 3140. Bağımsız başvuru scripts/fixtures/road_terrain_stockpile_cases.py: yüzey Triangle'ın; köşe "
                   "kotları, taban düzlemi, kırpma ve tümleme kesin kesirlerle elle; KentOS kodu yok. Okur, yazmaz.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("Stok hacmi ortak durumları güncel." if ok else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum, {len(text) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
