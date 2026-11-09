#!/usr/bin/env python3
"""The shared cases of `road.terrains.compare`'s material zones (plan/map-cad.md MAP-15, docs/adr/3540; Civil 3D's
bounded volumes in its Volumes Dashboard).

The drawing is road_terrain_compare_cases.py's with two layers of closed objects: “Malzeme bölgeleri” (an L of rock, a
holed rectangle of earth, a strip of clay touching it, the terrace's own zones) and “Bölge sınırları” (the other cases'
boundaries: a zone half off the base, a concave one across the cut's groove, one off both surfaces, the lattice's
square, a closed polyline, and the broken ones: an overlap, an open polyline, an arc).

Each zone is measured on its own here, unlike KentOS's clipping: the lattices' rectangles (road_terrain_compare_cases's
overlay, where the difference d is linear) meet the zone in GEOS (shapely), the part of each rectangle where d ≥ 0 or
d ≤ 0 meets it again, and a part's integral of the linear d is its area times d at its centroid. The rest is the whole
difference (road_terrain_compare_cases.answer) less the zones'; two zones overlap by GEOS's intersection area.

No KentOS code. Runs with shapely:

    uv run --no-project --python 3.11 --with shapely python3 scripts/fixtures/road_terrain_compare_zone_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

from shapely.geometry import Polygon, box

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_analysis_cases as analysis  # noqa: E402
import road_terrain_compare_cases as cmp  # noqa: E402

OUT = cmp.OUT
W = cmp.W
UID = cmp.UID
TX, FX = cmp.TX, cmp.FX
LAYER_STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}

# The zones: (layer, kind, corners, holes, attrs, closed polyline's bulges). The good set first, on its own layer, so a
# window reading the layer finds three zones that do not overlap.
KAYA = [(TX, 0.0), (TX + 4.0, 0.0), (TX + 4.0, 3.0), (TX + 2.0, 3.0), (TX + 2.0, 7.0), (TX, 7.0)]
TOPRAK = [(TX + 5.0, 1.0), (TX + 10.0, 1.0), (TX + 10.0, 9.0), (TX + 5.0, 9.0)]
TOPRAK_HOLE = [(TX + 6.5, 4.0), (TX + 8.0, 4.0), (TX + 8.0, 6.0), (TX + 6.5, 6.0)]
KIL = [(TX + 4.0, 3.0), (TX + 5.0, 3.0), (TX + 5.0, 9.0), (TX + 4.0, 9.0)]
HALF_OFF = [(FX + 8.0, 2.0), (FX + 12.0, 2.0), (FX + 12.0, 6.0), (FX + 8.0, 6.0)]
U = [(FX + 5.0, 1.0), (FX + 10.0, 1.0), (FX + 10.0, 9.0), (FX + 5.0, 9.0), (FX + 5.0, 7.0), (FX + 8.5, 7.0),
     (FX + 8.5, 3.0), (FX + 5.0, 3.0)]
FAR = [(FX + 20.0, 0.0), (FX + 25.0, 0.0), (FX + 25.0, 5.0), (FX + 20.0, 5.0)]
SQUARE = [(TX, 0.0), (TX + 10.0, 0.0), (TX + 10.0, 10.0), (TX, 10.0)]
TRIANGLE = [(TX + 1.0, 8.0), (TX + 9.0, 8.5), (TX + 2.0, 9.8)]
OVER = [(TX + 3.0, 2.0), (TX + 6.0, 2.0), (TX + 6.0, 5.0), (TX + 3.0, 5.0)]
OPEN = [(TX + 1.0, 1.0), (TX + 3.0, 1.0), (TX + 3.0, 2.0)]
ARC = [(TX + 6.0, 6.0), (TX + 8.0, 6.0), (TX + 8.0, 8.0)]

ZONES = [
    ("kaya", "malzeme", "polygon", KAYA, [], {"Malzeme": "Kaya", "Kazı katsayısı": "1.5"}),
    ("toprak", "malzeme", "polygon", TOPRAK, [TOPRAK_HOLE], {"Malzeme": "Toprak", "Kazı katsayısı": "1.25",
                                                              "Dolgu katsayısı": "1.1"}),
    ("kil", "malzeme", "polygon", KIL, [], {"Malzeme": "Kil"}),
    ("yarim", "sinir-b", "polygon", HALF_OFF, [], {}),
    ("u", "sinir-b", "polygon", U, [], {}),
    ("uzak", "sinir-b", "polygon", FAR, [], {}),
    ("kare", "sinir-b", "polygon", SQUARE, [], {}),
    ("ucgen", "sinir-b", "closed", TRIANGLE, [], {}),
    ("ustuste", "sinir-b", "polygon", OVER, [], {}),
    ("acik", "sinir-b", "open", OPEN, [], {}),
    ("yay", "sinir-b", "arc", ARC, [], {}),
]
FIRST = 0  # set by setup(): the first zone's object id


def pt(p):
    return {"x": p[0], "y": p[1]}


def setup(work="road"):
    global FIRST
    s = cmp.setup(work)
    for lid, name in (("malzeme", "Malzeme bölgeleri"), ("sinir-b", "Bölge sınırları")):
        s["layers"].append({"type": "layer", "id": lid, "name": name, "visible": True, "locked": False,
                            "expanded": True, "style": dict(LAYER_STYLE), "children": []})
    k = max(e["id"] for e in s["entities"])
    FIRST = k + 1
    for _, layer, kind, pts, holes, attrs in ZONES:
        k += 1
        e = {"kind": "polygon" if kind == "polygon" else "polyline", "id": k, "layerId": layer, "attrs": dict(attrs),
             "pts": [pt(p) for p in pts]}
        if holes:
            e["holes"] = [{"pts": [pt(p) for p in h]} for h in holes]
        if kind in ("closed", "arc"):
            e["closed"] = True
        if kind == "arc":
            e["bulges"] = [0.0, 0.5, 0.0]
        s["entities"].append(e)
        s["uids"].append(UID.format(k))
    return s


def uid(key):
    return UID.format(FIRST + [z[0] for z in ZONES].index(key))


def shape(key):
    z = next(z for z in ZONES if z[0] == key)
    return Polygon(z[3], z[4])


def rectangles(inp):
    """road_terrain_compare_cases.answer's overlay: each rectangle both lattices cover, with d at its corners."""
    bid = next(i for i, n in cmp.NAMES.items() if inp["base"] in (i, n))
    cid = next(i for i, n in cmp.NAMES.items() if inp["comparison"] in (i, n))
    _, fb, bxs, bys = cmp.LATTICES[bid]
    _, fc, cxs, cys = cmp.LATTICES[cid]
    xs = sorted({x for x in bxs + cxs if cxs[0] <= x <= cxs[-1]})
    ys = sorted({y for y in bys + cys if cys[0] <= y <= cys[-1]})
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            if not (bxs[0] <= x0 and x1 <= bxs[-1] and bys[0] <= y0 and y1 <= bys[-1]):
                continue
            yield x0, x1, y0, y1, [([x, y], fc(x, y) - fb(x, y)) for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1))]


def measure(inp, key):
    """A zone's part of the difference: GEOS's intersections, each part's integral its area times d at its centroid."""
    z = shape(key)
    m = {"area": z.area, "covered": 0.0, "cut": 0.0, "fill": 0.0, "cutArea": 0.0, "fillArea": 0.0}
    for x0, x1, y0, y1, poly in rectangles(inp):
        inter = z.intersection(box(x0, y0, x1, y1))
        if inter.is_empty or inter.area == 0.0:
            continue
        m["covered"] += inter.area
        d00, d10, d01 = poly[0][1], poly[1][1], poly[3][1]

        def d_at(x, y):
            return d00 + (x - x0) / (x1 - x0) * (d10 - d00) + (y - y0) / (y1 - y0) * (d01 - d00)

        for above in (True, False):
            part = cmp.split(poly, 0.0, above)
            if len(part) < 3:
                continue
            side = Polygon([p for p, _ in part])
            if side.area <= 0.0:
                continue
            g = z.intersection(side)
            if g.is_empty or g.area == 0.0:
                continue
            c = g.centroid
            v = g.area * d_at(c.x, c.y)
            if above:
                m["fill"] += v
                m["fillArea"] += g.area
            else:
                m["cut"] -= v
                m["cutArea"] += g.area
    return m


def adjusted(cut, fill, cf, ff):
    return {"cutFactor": cf, "fillFactor": ff, "cut": cut * cf, "fill": fill * ff, "net": fill * ff - cut * cf}


def answer(inp):
    out, warns = cmp.answer({k: v for k, v in inp.items() if k != "zones"})
    cf, ff = inp.get("cutFactor", 1.0), inp.get("fillFactor", 1.0)
    rows = []
    for k, z in enumerate(inp["zones"]):
        key = next(key for key, *_ in ZONES if uid(key) == z["boundary"])
        m = measure(inp, key)
        name = z["name"].strip()
        rows.append({"name": name, "boundary": z["boundary"], "area": m["area"], "covered": m["covered"],
                     "cut": m["cut"], "fill": m["fill"], "net": m["fill"] - m["cut"], "cutArea": m["cutArea"],
                     "fillArea": m["fillArea"],
                     "adjusted": adjusted(m["cut"], m["fill"], z.get("cutFactor", cf), z.get("fillFactor", ff))})
        off = m["area"] - m["covered"]
        if off > 1e-6 * max(m["area"], 1.0):
            warns.append({"code": "compare_uncovered", "message": f"“{name}” bölgesinin {off:.2f} m²'si iki yüzeyin "
                          "ortak alanının dışında; o kısım bölgenin hacmine girmedi.", "path": f"zones[{k}].boundary"})
    total = lambda key: sum(r[key] for r in rows)  # noqa: E731
    cut, fill = out["cut"] - total("cut"), out["fill"] - total("fill")
    rest = {"covered": out["covered"] - total("covered"), "cut": cut, "fill": fill, "net": fill - cut,
            "cutArea": out["cutArea"] - total("cutArea"), "fillArea": out["fillArea"] - total("fillArea"),
            "adjusted": adjusted(cut, fill, cf, ff)}
    zcut = sum(r["adjusted"]["cut"] for r in rows) + rest["adjusted"]["cut"]
    zfill = sum(r["adjusted"]["fill"] for r in rows) + rest["adjusted"]["fill"]
    out.update({"zones": rows, "rest": rest, "zoned": {"cut": zcut, "fill": zfill, "net": zfill - zcut}})
    return out, warns


def zone(name, key, cut=None, fill=None):
    z = {"name": name, "boundary": uid(key)}
    if cut is not None:
        z["cutFactor"] = cut
    if fill is not None:
        z["fillFactor"] = fill
    return z


def cases():
    out = []

    def q(name, inp, result=None):
        if result is None:
            o, w = answer(inp)
            result = {"status": "completed", "output": o, "warnings": w}
        out.append({"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": cmp.UNCHANGED,
                                             "within": W}]})

    terrace = {"base": "Teras", "comparison": "Set"}
    kaya, toprak, kil = zone("Kaya", "kaya", 1.5), zone("Toprak", "toprak", 1.25, 1.1), zone("Kil", "kil")
    q("İki malzeme bölgesi: kendi katsayılarıyla, bölge dışı karşılaştırmanınkiyle",
      {**terrace, "cutFactor": 1.2, "fillFactor": 1.05, "zones": [kaya, toprak]})
    q("Bölgenin katsayısı yoksa karşılaştırmanınki",
      {**terrace, "cutFactor": 1.3, "fillFactor": 0.95, "zones": [zone("Kaya", "kaya")]})
    q("Hiç katsayı yoksa 1", {**terrace, "zones": [zone("Kaya", "kaya"), zone("Toprak", "toprak")]})
    q("Delikli bölge: deliği hacme girmez", {**terrace, "zones": [toprak]})
    q("Kenarı ortak iki bölge örtüşmez", {**terrace, "zones": [toprak, kil]})
    # The desktop window reads these three from the layer's attributes (docs/adr/3540).
    q("Üç bölge: kaya, toprak ve katsayısız kil", {**terrace, "zones": [kaya, toprak, kil]})
    q("Yalnız kazı katsayılı bölge", {**terrace, "fillFactor": 1.2, "zones": [kaya]})
    q("Bütün alanı kaplayan bölge: bölge dışı sıfır", {**terrace, "zones": [zone("Dolgu toprağı", "kare", 1.1, 1.15)]})
    q("Kapalı çoklu çizgi bölge", {**terrace, "zones": [zone("Çakıl", "ucgen", 1.2, 1.0)]})
    q("Ters yönde: bölgelerin kazısı ve dolgusu yer değiştirir", {"base": "Set", "comparison": "Teras",
                                                                  "zones": [kaya, toprak]})
    q("Kimlikle ve fark aralıklarıyla", {"base": "t-6", "comparison": "t-9", "ranges": [-0.1, 0.0, 0.1, 0.2],
                                         "zones": [kaya]})
    q("Adı boşluklu bölge kırpılır", {**terrace, "zones": [zone("  Kaya  ", "kaya", 1.5)]})
    cut = {"base": "Düz", "comparison": "Kazı"}
    q("Yarısı tabanın dışında bölge: uyarıyla", {**cut, "zones": [zone("Kaya", "yarim", 1.4)]})
    q("İçbükey bölge kazının oluğunu keser", {**cut, "cutFactor": 1.25, "zones": [zone("Çakıl", "u", 1.1)]})
    q("Yüzeylerin dışındaki bölge: hacimsiz, uyarıyla", {**cut, "zones": [zone("Uzak", "uzak")]})
    # Refusals.
    factor = "Kazı ve dolgu katsayısı 0'dan büyük, en çok 10 olur (1: düzeltme yok)."
    fail = analysis.failed
    q("Üst üste binen bölgeler", {**terrace, "zones": [kaya, zone("Kaya 2", "ustuste")]},
      fail("invalid_analysis", f"“Kaya” ile “Kaya 2” bölgeleri üst üste biniyor ({shape('kaya').intersection(shape('ustuste')).area:.2f} m²): "
           "bir yer tek malzemenin olur; sınırları ayırın.", "zones[1].boundary"))
    q("Aynı sınır iki kez", {**terrace, "zones": [kaya, zone("Toprak", "kaya")]},
      fail("invalid_analysis", f"“Kaya” ile “Toprak” bölgeleri üst üste biniyor ({shape('kaya').area:.2f} m²): bir yer "
           "tek malzemenin olur; sınırları ayırın.", "zones[1].boundary"))
    q("Adı boş bölge", {**terrace, "zones": [zone("  ", "kaya")]},
      fail("invalid_analysis", "Malzeme bölgesinin adı boş olamaz; malzemeyi yazın (ör. Kaya).", "zones[0].name"))
    q("Sıfır kazı katsayılı bölge", {**terrace, "zones": [kil, zone("Kaya", "kaya", 0.0)]},
      fail("invalid_analysis", factor, "zones[1].cutFactor"))
    q("Çok büyük dolgu katsayılı bölge", {**terrace, "zones": [zone("Kaya", "kaya", None, 11.0)]},
      fail("invalid_analysis", factor, "zones[0].fillFactor"))
    missing = UID.format(999999)
    q("Olmayan sınır", {**terrace, "zones": [zone("Kaya", "kaya"), {"name": "Yok", "boundary": missing}]},
      fail("entity_not_found", f"“{missing}” kimlikli nesne çizimde yok.", "zones[1].boundary"))
    bad = "“{}” sınır olamaz: düz kenarlı, tek parçalı kapalı bir çoklu çizgi ya da alan olmalı."
    q("Açık çoklu çizgi sınır olamaz", {**terrace, "zones": [zone("Kaya", "acik")]},
      fail("invalid_analysis", bad.format(uid("acik")), "zones[0].boundary"))
    q("Yaylı sınır olamaz", {**terrace, "zones": [zone("Kaya", "yay")]},
      fail("invalid_analysis", bad.format(uid("yay")), "zones[0].boundary"))
    q("Izgarada bölge istenmez", {**terrace, "method": "grid", "cell": 1.0, "zones": [kaya]},
      fail("invalid_analysis", "Malzeme bölgeleri TIN yöntemiyle ölçülür; ızgara yönteminde bölge istenmez.", "zones"))
    return out


def main():
    s = setup()
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.compare", "commandVersion": 1,
           "title": "Yüzeyleri karşılaştır: malzeme bölgeleri",
           "note": "scripts/fixtures/road_terrain_compare_zone_cases.py'den; her bölge iki kafesin dikdörtgenleriyle "
                   "GEOS'ta (shapely) kesişir, parçanın integrali alanı çarpı ağırlık merkezindeki fark; bölge dışı "
                   "bütün farktan bölgelerinkinin çıkarılmasıyla, KentOS kodu olmadan.",
           "setup": s, "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.compare.zones.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.compare.zones.json")
    else:
        path.write_text(text)
    print(f"road.terrains.compare.zones.json: {len(found)} durum")


if __name__ == "__main__":
    main()
