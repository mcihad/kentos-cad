#!/usr/bin/env python3
"""The shared cases of `road.terrains.compare` (plan/road.md Y18.3, docs/adr/0916 §3; two surfaces' difference).

The drawing is road_terrain_analysis_cases.py's with two more lattices: a cut over the plane (a coarser lattice
shifted half off it) and a pad over the terrace. Every lattice is z = g(x) + h(y), so each square is one plane; laying
two lattices over each other gives rectangles (between the union of their x and of their y lines) each inside one
square of each, where the difference d = z_comparison − z_base is linear: its fill and cut are the fan integral
Σ A·(d₀ + d₁ + d₂)/3 of the rectangle's parts where d ≥ 0 and d ≤ 0; a range's area the rectangle clipped by d ≥ lo and
d ≤ hi. The extremes are at rectangle corners (the smallest x, then y, among equals; the place given only where it is
the one). The comparison's area off the base is uncovered.

No KentOS code; the ranges are the cases' own.

    python3 scripts/fixtures/road_terrain_compare_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_drainage_cases as drainage  # noqa: E402
import road_terrain_analysis_cases as analysis  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f5f0-7c3e-7d4a-9b1e-{:012d}"
UNCHANGED = analysis.UNCHANGED
FX, TX = drainage.FX, analysis.TX



# ADR 1520: surfaces are a Harita's and a Yol's; another project is refused.
NOT_SURFACE = drainage.failed("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden biri değil.")

def cut_z(x, y):
    # Over x 6002.5–6012.5, y 0–10 every 2.5: the plane cut down towards x′ = 7.5 and tilted the other way in y.
    xr = x - FX
    return 119.9 + 0.1 * xr - 0.15 * max(2.5 - abs(xr - 7.5), 0.0) - 0.01 * y


def pad_z(x, y):
    return analysis.terrace_z(x, y) + 0.2 - 0.05 * abs(x - TX - 5.0)


EXTRA = [("kazi", cut_z, [FX + 2.5 + 2.5 * i for i in range(5)], [2.5 * j for j in range(5)]),
         ("set", pad_z, [TX + i for i in range(11)], [float(j) for j in range(11)])]
LATTICES = {**analysis.LATTICES, "t-8": EXTRA[0], "t-9": EXTRA[1]}
NAMES = {**analysis.NAMES, "t-8": "Kazı", "t-9": "Set"}


def setup(work="road"):
    s = analysis.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for lid, name in (("kazi", "Kazı"), ("set", "Set")):
        s["layers"].append({"id": lid, "name": name, **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])
    for layer, f, xs, ys in EXTRA:
        for y in ys:
            for x in xs:
                k += 1
                s["entities"].append({"kind": "point", "id": k, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y},
                                      "z": f(x, y)})
                s["uids"].append(UID.format(k))
    if work == "road":
        s["settings"]["road"]["surfaces"] += [{"id": "t-8", "name": "Kazı", "points": {"layers": ["kazi"]}},
                                              {"id": "t-9", "name": "Set", "points": {"layers": ["set"]}}]
    return s


# Docs/adr/1029: what the comparison's heights rest on. “Kazı” made of the ground of a cloud on its own layer (its
# record's vertical system as given, none: not known); “Düz” of the drawing's points, the project's heights as given.
NOT_SHOWN = ("Karşılaştırılan kotların aynı düşey sistemde olduğu gösterilemiyor: {}. Sıfır ayrım ya da geoit tahmini "
             "yapılmaz. Projenin kotlarını Proje ayarlarında söyleyin, ya da kotları aynı saymak için assumeHeights verin "
             "(sonuç taslak olur).")
DRAWING_UNSAID = "çizimin kendi kotları (“Düz”; projenin kotlarının düşey sistemi söylenmemiş)"
CLOUD_UNSAID = ("“Kazı” yüzeyinin “kazi” katmanındaki bulut noktaları (dosya düşey sistemini söylemiyor ya da alındığında "
                "kaydedilmedi)")


def only(s, keep):
    """`s` with only the surfaces `keep` and the points of their layers, the objects numbered in their order again:
    docs/adr/1029's cases compare those alone, and the others' points would only make the file larger."""
    road = s["settings"]["road"]
    layers = lambda ts: {l for t in ts for l in t.get("points", {}).get("layers", [])}  # noqa: E731
    drop = layers(t for t in road["surfaces"] if t["id"] not in keep) - layers(t for t in road["surfaces"] if t["id"] in keep)
    pairs = [(e, u) for e, u in zip(s["entities"], s["uids"]) if e["layerId"] not in drop]
    s["entities"], s["uids"] = [dict(e, id=k + 1) for k, (e, _) in enumerate(pairs)], [u for _, u in pairs]
    road["surfaces"] = [t for t in road["surfaces"] if t["id"] in keep]
    return s


def heights_setup(vertical=None, heights=None, turned=None):
    s = setup()
    record = {"layer": "kazi", "clouds": [UID.format(1)], "classes": [2], "returns": "all", "cell": 1.0, "read": 25, "kept": 25,
              "coverage": 1.0, "density": 1.0}
    if vertical is not None:
        record["vertical"] = vertical
    if turned is not None:
        # Docs/adr/1032: the cloud's heights as the call said them, written in the project's.
        record["heights"] = {"from": turned, "to": heights}
    next(t for t in s["settings"]["road"]["surfaces"] if t["id"] == "t-8")["clouds"] = [record]
    if heights is not None:
        s["settings"]["heights"] = heights
    return only(s, {"t-5", "t-8"})


def assumed(what):
    return {"code": "heights_assumed", "message": f"Kotlar aynı düşey sistemde sayıldı (assumeHeights): {what}. Sonuç "
            "bu varsayıma dayanan bir taslaktır.", "path": "assumeHeights"}


def area2(a, b, c):
    return (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])


def split(poly, level, above):
    out = []
    for i in range(len(poly)):
        p, q = poly[i], poly[(i + 1) % len(poly)]
        dp, dq = p[1] - level, q[1] - level
        if (dp >= 0) if above else (dp <= 0):
            out.append(p)
        if (dp > 0 and dq < 0) or (dp < 0 and dq > 0):
            t = dp / (dp - dq)
            out.append(([p[0][0] + (q[0][0] - p[0][0]) * t, p[0][1] + (q[0][1] - p[0][1]) * t], level))
    return out


def integral(poly):
    a = v = 0.0
    for i in range(1, len(poly) - 1):
        s = area2(poly[0][0], poly[i][0], poly[i + 1][0]) / 2.0
        a += s
        v += s * (poly[0][1] + poly[i][1] + poly[i + 1][1]) / 3.0
    return a, v


def best(items, larger):
    if not items:
        return None
    v = max(d for d, _ in items) if larger else min(d for d, _ in items)
    near = sorted(p for d, p in items if abs(d - v) <= 1e-9)
    return {"difference": v, "at": near[0] if len(near) == 1 else ANY}


def answer(inp):
    bid = next(i for i, n in NAMES.items() if inp["base"] in (i, n))
    cid = next(i for i, n in NAMES.items() if inp["comparison"] in (i, n))
    _, fb, bxs, bys = LATTICES[bid]
    _, fc, cxs, cys = LATTICES[cid]
    xs = sorted({x for x in bxs + cxs if cxs[0] <= x <= cxs[-1]})
    ys = sorted({y for y in bys + cys if cys[0] <= y <= cys[-1]})
    breaks = inp.get("ranges")
    out = {"base": bid, "comparison": cid, "cut": 0.0, "fill": 0.0, "net": 0.0, "cutArea": 0.0, "fillArea": 0.0,
           "covered": 0.0, "uncovered": 0.0}
    bands = [[breaks[i], breaks[i + 1], 0.0] for i in range(len(breaks) - 1)] if breaks else []
    outside = 0.0
    corners = []
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            a = (x1 - x0) * (y1 - y0)
            if not (bxs[0] <= x0 and x1 <= bxs[-1] and bys[0] <= y0 and y1 <= bys[-1]):
                out["uncovered"] += a
                continue
            out["covered"] += a
            poly = [([x, y], fc(x, y) - fb(x, y)) for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]
            corners += [(d, p) for p, d in poly]
            fa, fv = integral(split(poly, 0.0, True))
            ca, cv = integral(split(poly, 0.0, False))
            out["fill"] += fv
            out["fillArea"] += fa
            out["cut"] -= cv
            out["cutArea"] += ca
            inside = 0.0
            for b in bands:
                low = split(poly, b[0], True)
                part = split(low, b[1], False) if len(low) >= 3 else []
                if len(part) >= 3:
                    pa, _ = integral(part)
                    if pa > 0:
                        b[2] += pa
                        inside += pa
            outside += max(a - inside, 0.0)
    out["net"] = out["fill"] - out["cut"]
    for key, larger in (("least", False), ("largest", True)):
        e = best(corners, larger)
        if e:
            out[key] = e
    if breaks:
        out["bands"] = [{"from": b[0], "to": b[1], "area": b[2]} for b in bands]
        out["outsideArea"] = outside
    if inp.get("method") == "grid":
        # The grid (ADR 1600, the midpoint rule of ADR 0974): cells `cell` square with corners on multiples of it, each
        # centre in the comparison's plan measured once: d = z_comparison − z_base there, its cell's volume cell²·d;
        # a centre off the base is uncovered. The exact TIN volumes stay beside it.
        cell = inp["cell"]
        exact = {"cut": out["cut"], "fill": out["fill"], "net": out["net"]}
        g = {"cut": 0.0, "fill": 0.0, "cutArea": 0.0, "fillArea": 0.0, "covered": 0.0, "uncovered": 0.0}
        a = cell * cell
        for j in range(math.ceil(cys[0] / cell - 0.5), math.floor(cys[-1] / cell - 0.5) + 1):
            for i in range(math.ceil(cxs[0] / cell - 0.5), math.floor(cxs[-1] / cell - 0.5) + 1):
                x, y = (i + 0.5) * cell, (j + 0.5) * cell
                if not (bxs[0] <= x <= bxs[-1] and bys[0] <= y <= bys[-1]):
                    g["uncovered"] += a
                    continue
                g["covered"] += a
                dz = fc(x, y) - fb(x, y)
                if dz > 0:
                    g["fill"] += dz * a
                    g["fillArea"] += a
                elif dz < 0:
                    g["cut"] -= dz * a
                    g["cutArea"] += a
        out.update(g)
        out["net"] = g["fill"] - g["cut"]
        out["cell"] = cell
        out["exact"] = exact
    if "cutFactor" in inp or "fillFactor" in inp:
        # Civil 3D's Volumes Dashboard: the cut's swell, the fill's compaction; the geometric volumes stay.
        cf, ff = inp.get("cutFactor", 1.0), inp.get("fillFactor", 1.0)
        out["adjusted"] = {"cutFactor": cf, "fillFactor": ff, "cut": out["cut"] * cf, "fill": out["fill"] * ff,
                           "net": out["fill"] * ff - out["cut"] * cf}
    warns = []
    if out["uncovered"] > 0:
        warns.append({"code": "compare_uncovered", "message": f"Karşılaştırma yüzeyinin {out['uncovered']:.2f} m²'si "
                      "tabanın dışında; o kısım hacme girmedi.", "path": "comparison"})
    return out, warns


def cases():
    out = []

    def q(name, inp, result=None, setup_=None):
        if result is None:
            o, w = answer(inp)
            result = {"status": "completed", "output": o, "warnings": w}
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED,
                                      "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    q("Kazı düzlemin üstünde, yarısı dışında", {"base": "Düz", "comparison": "Kazı"})
    q("Kazı aralıklarıyla", {"base": "Düz", "comparison": "Kazı", "ranges": [-0.5, -0.2, 0.0, 0.2, 0.5]})
    q("Set terasın üstünde", {"base": "Teras", "comparison": "Set"})
    q("Set aralıklarıyla", {"base": "Teras", "comparison": "Set", "ranges": [-0.1, 0.0, 0.1, 0.2]})
    q("Ters yönde kazı ve dolgu yer değiştirir", {"base": "Set", "comparison": "Teras", "ranges": [-0.2, 0.0, 0.2]})
    q("Düzlem kazının üstünde", {"base": "Kazı", "comparison": "Düz"})
    q("Kimlikle", {"base": "t-6", "comparison": "t-9"})
    q("Örtüşmeyen yüzeyler", {"base": "Vadi", "comparison": "Düz"})
    q("Aralıkların dışı", {"base": "Teras", "comparison": "Set", "ranges": [0.15, 0.18]})
    q("Tek aralık bütün farkı alır", {"base": "Düz", "comparison": "Kazı", "ranges": [-5.0, 5.0]})
    q("Terasın sırtla farkı", {"base": "Teras", "comparison": "Sırt"})
    q("Çukurun düzle farkı", {"base": "Düz", "comparison": "Çukur"})
    q("Teras ile set, ince aralıklar", {"base": "Teras", "comparison": "Set",
                                        "ranges": [-0.05, 0.0, 0.05, 0.1, 0.15, 0.2, 0.25]})
    q("Ters yönde ince aralıklar", {"base": "Set", "comparison": "Teras", "ranges": [-0.25, -0.1, 0.0, 0.1]})
    # The method and the factors (ADR 1600).
    q("Izgara 0,5 m: kazı düzlemin üstünde", {"base": "Düz", "comparison": "Kazı", "method": "grid", "cell": 0.5})
    q("Izgara 2 m: kaba hücre, yöntem farkı büyür", {"base": "Düz", "comparison": "Kazı", "method": "grid", "cell": 2.0})
    q("Izgara 0,25 m: set terasın üstünde", {"base": "Teras", "comparison": "Set", "method": "grid", "cell": 0.25})
    q("Izgara 0,3 m: hücre kenarı kafese denk gelmez", {"base": "Düz", "comparison": "Kazı", "method": "grid",
                                                         "cell": 0.3})
    q("Izgara 0,7 m: kırık çizgi hücrenin içinde", {"base": "Düz", "comparison": "Kazı", "method": "grid", "cell": 0.7})
    q("TIN yöntemi açıkça", {"base": "Düz", "comparison": "Kazı", "method": "tin"})
    q("Katsayılar: kazı 1,25, dolgu 1,10", {"base": "Teras", "comparison": "Set", "cutFactor": 1.25, "fillFactor": 1.1})
    q("Yalnız dolgu katsayısı", {"base": "Düz", "comparison": "Kazı", "fillFactor": 1.15})
    q("Izgara ve katsayılar birlikte", {"base": "Teras", "comparison": "Set", "method": "grid", "cell": 0.5,
                                        "cutFactor": 1.3, "fillFactor": 0.9})
    q("Izgarada hücre yok", {"base": "Düz", "comparison": "Kazı", "method": "grid"},
      analysis.failed("invalid_analysis", "Izgara yönteminde hücrenin kenarını (cell, m) verin.", "cell"))
    q("Izgarada sıfır hücre", {"base": "Düz", "comparison": "Kazı", "method": "grid", "cell": 0.0},
      analysis.failed("invalid_analysis", "Izgaranın hücresi 0'dan büyük bir uzunluk olmalı.", "cell"))
    q("Izgarada çok küçük hücre", {"base": "Düz", "comparison": "Kazı", "method": "grid", "cell": 0.001},
      analysis.failed("invalid_analysis", "Izgaranın hücresi bu alan için çok küçük: en çok 4 000 000 hücre ölçülür; "
                      "hücreyi büyütün.", "cell"))
    q("Izgarada aralık istenmez", {"base": "Düz", "comparison": "Kazı", "method": "grid", "cell": 1.0,
                                   "ranges": [-1.0, 0.0, 1.0]},
      analysis.failed("invalid_analysis", "Fark aralıkları TIN yöntemiyle verilir; ızgara yönteminde aralık istenmez.",
                      "ranges"))
    q("Sıfır kazı katsayısı", {"base": "Düz", "comparison": "Kazı", "cutFactor": 0.0},
      analysis.failed("invalid_analysis", "Kazı ve dolgu katsayısı 0'dan büyük, en çok 10 olur (1: düzeltme yok).",
                      "cutFactor"))
    q("Çok büyük dolgu katsayısı", {"base": "Düz", "comparison": "Kazı", "fillFactor": 11.0},
      analysis.failed("invalid_analysis", "Kazı ve dolgu katsayısı 0'dan büyük, en çok 10 olur (1: düzeltme yok).",
                      "fillFactor"))
    # Docs/adr/1029: the heights' vertical system.
    pair = {"base": "Düz", "comparison": "Kazı"}
    ortho = lambda srid: {"kind": "orthometric", "verticalSrid": srid}  # noqa: E731
    q("Bulutun düşey sistemi bilinmiyor: ret", pair,
      analysis.failed("unproven_heights", NOT_SHOWN.format(f"{DRAWING_UNSAID}; {CLOUD_UNSAID}"), "assumeHeights"),
      setup_=heights_setup())
    o, w = answer(pair)
    q("Varsayımla taslak", {**pair, "assumeHeights": True},
      {"status": "completed", "output": o, "warnings": [assumed(f"{DRAWING_UNSAID}; {CLOUD_UNSAID}")] + w},
      setup_=heights_setup())
    q("Bulutunki biliniyor, projeninki söylenmemiş: ret", pair,
      analysis.failed("unproven_heights", NOT_SHOWN.format(DRAWING_UNSAID), "assumeHeights"),
      setup_=heights_setup("EPSG:5775"))
    q("Projeyle bulut aynı sistemde", pair, setup_=heights_setup("Antalya height", ortho(5775)))
    q("Projenin kotlarına çevrilmiş bulut zemini", pair,
      setup_=heights_setup(None, ortho(5775), {"kind": "ellipsoidal"}))
    q("İki ayrı sistem varsayımla da ret", {**pair, "assumeHeights": True},
      analysis.failed("unproven_heights", "Karşılaştırılan kotlar iki ayrı düşey sistemde: çizimin kendi kotları "
                      "(“Düz”) Antalya height (EPSG:5775), “Kazı” yüzeyinin “kazi” katmanındaki bulut noktaları EGM2008 "
                      "height (EPSG:3855). Aradaki ayrım sıfır sayılmaz; kotları önce aynı sisteme çevirin (Koordinat "
                      "dönüştür, geoit modeliyle).", "comparison"),
      setup_=heights_setup("EGM2008 height", ortho(5775)))
    # Refusals.
    bad = "Fark aralıklarının sınırları en az iki, sonlu ve artan sayılar olmalı."
    q("Tek sınır", {"base": "Düz", "comparison": "Kazı", "ranges": [0.0]},
      analysis.failed("invalid_analysis", bad, "ranges"))
    q("Artmayan sınırlar", {"base": "Düz", "comparison": "Kazı", "ranges": [0.0, 0.0]},
      analysis.failed("invalid_analysis", bad, "ranges"))
    q("Azalan sınırlar", {"base": "Düz", "comparison": "Kazı", "ranges": [1.0, -1.0]},
      analysis.failed("invalid_analysis", bad, "ranges"))
    q("Aynı yüzey", {"base": "Düz", "comparison": "t-5"},
      analysis.failed("invalid_analysis", "Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.", "comparison"))
    q("Olmayan taban", {"base": "Tasarım", "comparison": "Düz"},
      analysis.failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "base"))
    q("Olmayan karşılaştırma", {"base": "Düz", "comparison": "Tasarım"},
      analysis.failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "comparison"))
    q("Kurulamayan yüzey", {"base": "Düz", "comparison": "Kurulamaz"},
      {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    q("Yol projesi değil", {"base": "Düz", "comparison": "Kazı"}, NOT_SURFACE, setup_="$map")
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="architecture")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.compare", "commandVersion": 1,
           "title": "Yüzeyleri karşılaştır: kazı, dolgu, fark aralıkları",
           "note": "scripts/fixtures/road_terrain_compare_cases.py'den; iki kafesin dikdörtgenlerle üst üste konuşu, her "
                   "dikdörtgende fark doğrusal, ADR 0916'nın kuralıyla, KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.compare.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.compare.json")
    else:
        path.write_text(text)
    print(f"road.terrains.compare.json: {len(found)} durum")


if __name__ == "__main__":
    main()
