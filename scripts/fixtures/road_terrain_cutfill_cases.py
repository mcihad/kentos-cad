#!/usr/bin/env python3
"""The shared cases of `road.terrains.cutfill` (docs/adr/1840; plan/map-cad.md MAP-15): two surfaces' cut and fill zones
written as areas.

The drawing is road_terrain_zeroline_cases.py's with three more lattices: a bank on the plane (z = plane + 0.1·max(0,
2 − |x − 6005|): fill on a 4 m strip, the plane itself elsewhere, so coincident there), and the plane and the mound
again at TM coordinates (x 512 000, y 4 512 000), where a project's coordinates are. Every lattice is z = g(x) + h(y):
over the rectangles between the union of two lattices' lines the difference d = comparison − base is linear. A
difference within 1 nm of zero is zero; a rectangle all zero is coincident area. Each rectangle is cut at zero and,
with an interval i, at its multiples (band k: [k·i, (k+1)·i); without one, −1 the cut and 0 the fill), a corner on a
level kept on both sides; a cut point is found from its edge's smaller end, so a rectangle and its neighbour find the
same point. A band's pieces are dissolved with shapely (GEOS), not KentOS's area algebra; a zone's area and volume are
the sums of the pieces whose corners' mean it holds. A ring's vertex within 1 µm of the line through its neighbours
(and between them) is dropped; outer rings run counterclockwise and holes clockwise, each from its smallest place
(x, then y); zones go by band and then by their ring's first place.

No KentOS code.

    uv run --no-project --python 3.11 --with shapely python3 scripts/fixtures/road_terrain_cutfill_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

from shapely.geometry import Point, Polygon
from shapely.geometry.polygon import orient
from shapely.ops import unary_union

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_drainage_cases as drainage  # noqa: E402
import road_terrain_analysis_cases as analysis  # noqa: E402
import road_terrain_zeroline_cases as zeroline  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f640-7c3e-7d4a-9b1e-{:012d}"
FX = drainage.FX
ZERO = 1e-9
STRAIGHT = 1e-6
MOST_BANDS = 1000
UNCHANGED = zeroline.UNCHANGED
STEPPED = zeroline.STEPPED
NOT_SURFACE = zeroline.NOT_SURFACE
TMX, TMY = 512000.0, 4512000.0


def bank_z(x, y):
    return drainage.plane_z(x, y) + 0.1 * max(0.0, 2.0 - abs(x - FX - 5.0))


def far_plane_z(x, y):
    return 1250.0 + 0.1 * (x - TMX) + 0.02 * (y - TMY)


def far_mound_z(x, y):
    return far_plane_z(x, y) + 0.4 - 0.1 * (abs(x - TMX - 5.0) + abs(y - TMY - 5.0))


EXTRA = [("t-11", "Bent", "bent", bank_z, [FX + i for i in range(11)], [float(j) for j in range(11)]),
         ("t-12", "Uzak düz", "uzak-duz", far_plane_z, [TMX + i for i in range(11)], [TMY + j for j in range(11)]),
         ("t-13", "Uzak tümsek", "uzak-tumsek", far_mound_z, [TMX + i for i in range(11)], [TMY + j for j in range(11)])]
LATTICES = {**zeroline.LATTICES, **{sid: (layer, f, xs, ys) for sid, _, layer, f, xs, ys in EXTRA}}
NAMES = {**zeroline.NAMES, **{sid: name for sid, name, *_ in EXTRA}}


def setup(work="road"):
    s = zeroline.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for _, name, layer, *_ in EXTRA:
        s["layers"].append({"id": layer, "name": name, **copy.deepcopy(base)})
    s["layers"].append({"id": "kazi-dolgu", "name": "Kazı ve dolgu", **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])
    for _, _, layer, f, xs, ys in EXTRA:
        for y in ys:
            for x in xs:
                k += 1
                s["entities"].append({"kind": "point", "id": k, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y},
                                      "z": f(x, y)})
                s["uids"].append(UID.format(k))
    if work in ("road", "map"):
        s["settings"]["road"]["surfaces"] += [{"id": sid, "name": name, "points": {"layers": [layer]}}
                                              for sid, name, layer, *_ in EXTRA]
    return s


def rectangles(bid, cid):
    """The overlap's rectangles, each its corners counterclockwise with d (1 nm to zero); and the comparison's area."""
    _, fb, bxs, bys = LATTICES[bid]
    _, fc, cxs, cys = LATTICES[cid]
    xs = sorted({x for x in bxs + cxs if cxs[0] <= x <= cxs[-1]})
    ys = sorted({y for y in bys + cys if cys[0] <= y <= cys[-1]})
    out = []
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            if not (bxs[0] <= x0 and x1 <= bxs[-1] and bys[0] <= y0 and y1 <= bys[-1]):
                continue
            poly = []
            for x, y in ((x0, y0), (x1, y0), (x1, y1), (x0, y1)):
                d = fc(x, y) - fb(x, y)
                poly.append(((x, y), 0.0 if abs(d) <= ZERO else d))
            out.append(poly)
    return out, (cxs[-1] - cxs[0]) * (cys[-1] - cys[0])


def cross(a, b, level):
    """Where d meets `level` on the edge a–b, found from its smaller end."""
    (p, dp), (q, dq) = sorted([a, b], key=lambda v: v[0])
    t = (dp - level) / (dp - dq)
    return (p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t)


def clip(poly, level, above):
    """The convex polygon's part with d ≥ level (`above`) or ≤ level; a corner on the level kept on both sides."""
    out = []
    n = len(poly)
    for k in range(n):
        a, b = poly[k], poly[(k + 1) % n]
        if (a[1] >= level) if above else (a[1] <= level):
            out.append(a)
        if (a[1] - level) * (b[1] - level) < 0:
            out.append((cross(a, b, level), level))
    return out


def integral(poly):
    """A convex polygon's area and the integral of the linear d over it (a fan from its first corner)."""
    area = vol = 0.0
    for i in range(1, len(poly) - 1):
        (a, da), (b, db), (c, dc) = poly[0], poly[i], poly[i + 1]
        s = ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0
        area += s
        vol += s * (da + db + dc) / 3.0
    return area, vol


def between(a, b, c):
    ux, uy = c[0] - a[0], c[1] - a[1]
    vx, vy = b[0] - a[0], b[1] - a[1]
    len2 = ux * ux + uy * uy
    if len2 == 0.0:
        return math.hypot(vx, vy) <= STRAIGHT
    t = (vx * ux + vy * uy) / len2
    return 0.0 < t < 1.0 and abs(ux * vy - uy * vx) / math.sqrt(len2) <= STRAIGHT


def simplify(pts):
    out = []
    for p in pts:
        while len(out) >= 2 and between(out[-2], out[-1], p):
            out.pop()
        out.append(p)
    while len(out) > 3:
        if between(out[-2], out[-1], out[0]):
            out.pop()
        elif between(out[-1], out[0], out[1]):
            out.pop(0)
        else:
            break
    return out


def ring(coords):
    pts = [tuple(c) for c in coords]
    if len(pts) > 1 and pts[0] == pts[-1]:
        pts.pop()
    pts = simplify(pts)
    k = min(range(len(pts)), key=lambda i: pts[i])
    return [list(p) for p in pts[k:] + pts[:k]]


def zones_of(bid, cid, interval):
    """The zones and the totals; or the band count when the interval gives too many."""
    rects, whole = rectangles(bid, cid)
    t = {"cut": 0.0, "fill": 0.0, "cutArea": 0.0, "fillArea": 0.0, "coincident": 0.0, "covered": 0.0}
    kept, ds = [], []
    for poly in rects:
        a, _ = integral(poly)
        t["covered"] += a
        if all(d == 0.0 for _, d in poly):
            t["coincident"] += a
            continue
        fa, fv = integral(clip(poly, 0.0, True))
        ca, cv = integral(clip(poly, 0.0, False))
        t["fill"] += fv
        t["fillArea"] += fa
        t["cut"] -= cv
        t["cutArea"] += ca
        ds += [d for _, d in poly]
        kept.append(poly)
    t["uncovered"] = max(whole - t["covered"], 0.0)
    if not kept:
        return None, t
    t["least"], t["largest"] = min(ds), max(ds)
    if interval is not None:
        count = math.floor(t["largest"] / interval) - math.floor(t["least"] / interval) + 1
        if count > MOST_BANDS:
            return count, t
    bands = {}
    for poly in kept:
        if interval is None:
            parts = [(-1, clip(poly, 0.0, False)), (0, clip(poly, 0.0, True))]
        else:
            lo, hi = min(d for _, d in poly), max(d for _, d in poly)
            parts = [(k, clip(clip(poly, k * interval, True), (k + 1) * interval, False))
                     for k in range(math.floor(lo / interval), math.floor(hi / interval) + 1)]
        for k, part in parts:
            if len(part) >= 3:
                a, v = integral(part)
                if a > 0.0:
                    bands.setdefault(k, []).append((part, a, v))
    zones = []
    for k in sorted(bands):
        pieces = bands[k]
        merged = unary_union([Polygon([p for p, _ in part]) for part, _, _ in pieces])
        geoms = [orient(g, sign=1.0) for g in (merged.geoms if hasattr(merged, "geoms") else [merged])]
        sums = [[0.0, 0.0] for _ in geoms]
        for part, a, v in pieces:
            n = len(part)
            c = Point(sum(p[0] for p, _ in part) / n, sum(p[1] for p, _ in part) / n)
            g = next(i for i, geom in enumerate(geoms) if geom.covers(c))
            sums[g][0] += a
            sums[g][1] += v
        for geom, (a, v) in zip(geoms, sums):
            holes = sorted((ring(h.coords) for h in geom.interiors), key=lambda h: h[0])
            zone = {"kind": "cut" if k < 0 else "fill"}
            if interval is not None:
                zone["from"], zone["to"] = k * interval, (k + 1) * interval
            zone.update({"area": a, "volume": abs(v), "ring": ring(geom.exterior.coords)})
            if holes:
                zone["holes"] = holes
            zones.append((k, zone))
    zones.sort(key=lambda kz: (kz[0], kz[1]["ring"][0]))
    return [z for _, z in zones], t


def find(name):
    # By id, else by name with case aside.
    return next(i for i, n in NAMES.items() if name == i or name.lower() == n.lower())


def answer(inp):
    bid, cid = find(inp["base"]), find(inp["comparison"])
    zones, t = zones_of(bid, cid, inp.get("interval"))
    out = {"base": bid, "comparison": cid}
    if "interval" in inp:
        out["interval"] = inp["interval"]
    out.update({"zones": zones, "cut": t["cut"], "fill": t["fill"], "net": t["fill"] - t["cut"],
                "cutArea": t["cutArea"], "fillArea": t["fillArea"], "coincident": t["coincident"],
                "covered": t["covered"], "uncovered": t["uncovered"], "least": t["least"], "largest": t["largest"]})
    return out


def failed(code, message, path=None):
    return analysis.failed(code, message, path)


def cases():
    out = []
    n = len(setup()["entities"])

    def ok(name, inp, op="execute", warns=(), note=None, setup_=None, entities=None):
        o = answer(inp)
        written = op == "execute"
        o["revision"] = "$current"
        if written:
            # Each zone a hatch (its fill) and a closed area (its data).
            o["uids"] = [ANY] * (2 * len(o["zones"]))
        result = ({"status": "completed", "output": o, "warnings": list(warns)} if op != "validate"
                  else {"status": "completed", "output": None, "warnings": list(warns)})
        count = entities if entities is not None else n
        expect = ({**STEPPED, "ids": list(range(1, count + 2 * len(o["zones"]) + 1))} if written
                  else {**UNCHANGED, "ids": list(range(1, count + 1))})
        c = {"name": name, **({"note": note} if note else {}),
             "steps": [{"op": op, "input": {"layerId": "kazi-dolgu", **inp} if "layerId" not in inp else inp,
                        "result": result, "expect": expect, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)
        return o

    def no(name, inp, result, setup_=None):
        c = {"name": name, "steps": [{"op": "execute", "input": {"layerId": "kazi-dolgu", **inp} if "layerId" not in inp
                                      else inp, "result": result, "expect": {**UNCHANGED}}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    two = ok("Tümsek düzün üstünde: dolgu eşkenar dörtgen, kazı delikli kare", {"base": "Düz", "comparison": "Tümsek"},
             note="fark 0,4 − 0,1·(|x − 6005| + |y − 5|): dolgu 32 m² ve 32·0,4/3 m³, kazı 68 m² ve 14,27 m³, deliği "
                  "dörtgen")
    assert [z["kind"] for z in two["zones"]] == ["cut", "fill"] and len(two["zones"][0].get("holes", [])) == 1
    assert abs(two["zones"][1]["area"] - 32.0) < 1e-9 and abs(two["zones"][1]["volume"] - 32.0 * 0.4 / 3.0) < 1e-9
    ok("Düz tümseğin altında: aynı bölgeler, kazı ile dolgu yer değiştirir", {"base": "Tümsek", "comparison": "Düz"})
    banded = ok("Tümsek 0,2 m'lik bantlarla: on bir bölge", {"base": "Düz", "comparison": "Tümsek", "interval": 0.2},
                note="kazıda köşelerde dörder bölge ve delikli halka; dolguda delikli halka ve tepe")
    assert len(banded["zones"]) == 11, len(banded["zones"])
    ok("Tümsek 0,25 m'lik bantlarla", {"base": "Düz", "comparison": "Tümsek", "interval": 0.25})
    ok("Büyük aralık: bir kazı, bir dolgu bandı, sınırlarıyla", {"base": "Düz", "comparison": "Tümsek", "interval": 10})
    ok("Set terasın üstünde: bir dolgu, iki kazı şeridi", {"base": "Teras", "comparison": "Set"},
       note="fark 0,2 − 0,05·|x − 8005|: x = 8001 ve 8009 kafes çizgileri bölgeleri ayırır")
    ok("Set 0,05 m'lik bantlarla", {"base": "Teras", "comparison": "Set", "interval": 0.05})
    ok("Kazı düzün hep altında: tek kazı bölgesi, bir kısmı tabanın dışında",
       {"base": "Düz", "comparison": "Kazı"}, note="karşılaştırmanın x 6010–6012,5 şeridi tabanın dışında: 25 m²")
    bank = ok("Bent düzün üstünde: dolgu şeridi, gerisi çakışık", {"base": "Düz", "comparison": "Bent"},
              note="fark 0,1·max(0, 2 − |x − 6005|): dolgu 40 m², 4 m³; 60 m² çakışık, bölge değil")
    assert len(bank["zones"]) == 1 and abs(bank["coincident"] - 60.0) < 1e-9
    ok("Bent 0,05 m'lik bantlarla: yedi şerit", {"base": "Düz", "comparison": "Bent", "interval": 0.05})
    far = ok("Proje koordinatlarında (TM): aynı eşkenar dörtgen", {"base": "Uzak düz", "comparison": "Uzak tümsek"},
             note="x 512 000, y 4 512 000; kafesin köşeleri halkadan atılır, dört köşe kalır")
    assert len(far["zones"][1]["ring"]) == 4
    ok("Proje koordinatlarında 0,2 m'lik bantlar", {"base": "Uzak düz", "comparison": "Uzak tümsek", "interval": 0.2})
    ok("Kimliklerle", {"base": "t-5", "comparison": "t-10"})
    ok("Adlar küçük harfle", {"base": "düz", "comparison": "tümsek"})
    ok("Plan yazmaz", {"base": "Düz", "comparison": "Tümsek", "interval": 0.2}, op="plan")
    ok("Doğrulama yazmaz", {"base": "Teras", "comparison": "Set"}, op="validate")
    ok("Başka katmana", {"base": "Düz", "comparison": "Tümsek", "layerId": "drenaj"})
    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    ok("Gizli katmana uyarıyla", {"base": "Teras", "comparison": "Set", "layerId": "gizli"}, warns=[hidden])
    ok("Harita projesinde de", {"base": "Düz", "comparison": "Tümsek"}, setup_="$harita",
       entities=len(setup(work="map")["entities"]))
    # Refusals.
    none = failed("no_cut_fill", "İki yüzey örtüşmüyor ya da örtüştükleri her yerde çakışıyor: kazı ya da dolgu bölgesi "
                  "yok (karşılaştırma tabanın kapladığı yerde olmalı).", "comparison")
    no("Örtüşmeyen yüzeyler", {"base": "Vadi", "comparison": "Düz"}, none)
    no("Aynı yüzey", {"base": "Düz", "comparison": "t-5"},
       failed("invalid_analysis", "Taban ile karşılaştırma aynı yüzey; iki ayrı yüzey verin.", "comparison"))
    bad_interval = failed("invalid_analysis", "Aralık 0'dan büyük bir sayı olmalı (m); bantsız kazı ve dolgu için boş "
                          "bırakın.", "interval")
    no("Aralık sıfır", {"base": "Düz", "comparison": "Tümsek", "interval": 0}, bad_interval)
    no("Aralık eksi", {"base": "Düz", "comparison": "Tümsek", "interval": -0.5}, bad_interval)
    fine, _ = zones_of("t-5", "t-10", 0.0001)
    no("Bin banttan çok", {"base": "Düz", "comparison": "Tümsek", "interval": 0.0001},
       failed("invalid_analysis", f"Bu aralık iki yüzeyin farkını {fine} banda böler; en çok {MOST_BANDS} bant olur. "
              "Daha büyük bir aralık verin.", "interval"))
    no("Olmayan taban", {"base": "Tasarım", "comparison": "Düz"},
       failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "base"))
    no("Olmayan karşılaştırma", {"base": "Düz", "comparison": "Tasarım"},
       failed("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "comparison"))
    no("Kurulamayan yüzey", {"base": "Düz", "comparison": "Kurulamaz"},
       {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    no("Kilitli katman", {"base": "Düz", "comparison": "Tümsek", "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
              "etkinleştirin.", "layerId"))
    no("Katman grubu", {"base": "Düz", "comparison": "Tümsek", "layerId": "grup"},
       {"status": "failed", "error": {"code": "not_a_layer", "message": ANY, "path": "layerId"}})
    no("Katman yok", {"base": "Düz", "comparison": "Tümsek", "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Geçersiz beklenen sürüm", {"base": "Düz", "comparison": "Tümsek", "expectedRevision": "on"}, zeroline.BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"base": "Düz", "comparison": "Tümsek", "expectedRevision": "41"},
       zeroline.CONFLICT)
    no("Mimari proje", {"base": "Düz", "comparison": "Tümsek"}, NOT_SURFACE, setup_="$mimari")
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$mimari":
            c["setup"] = setup(work="architecture")
        elif c.get("setup") == "$harita":
            c["setup"] = setup(work="map")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.cutfill", "commandVersion": 1,
           "title": "Kazı ve dolgu bölgeleri: iki yüzeyin farkı alan olarak",
           "note": "scripts/fixtures/road_terrain_cutfill_cases.py'den; kafeslerin dikdörtgenlerinde fark doğrusal, sıfırda "
                   "ve aralığın katlarında kesilir, bir bandın parçaları shapely (GEOS) ile birleşir; ADR 1840'ın halka "
                   "kuralıyla, KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.cutfill.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.cutfill.json")
    else:
        path.write_text(text)
    print(f"road.terrains.cutfill.json: {len(found)} durum")


if __name__ == "__main__":
    main()
