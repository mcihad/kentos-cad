#!/usr/bin/env python3
"""The shared cases of `road.terrains.coverage` (plan/road.md Y19.1, docs/adr/0920; a surface's coverage of a project
boundary, its no-data rest and its estimated part).

The drawing is road_terrain_quality_cases.py's with boundaries of this script. The surfaces are lattices: their
triangles' union is the lattice's squares, and both triangles of a square have its diagonal as their longest plan edge,
whichever diagonal they take, so a square is wholly estimated or wholly measured. The boundaries cut squares only
across their insides (never along a lattice line, a corner never on a diagonal), so both triangles of a cut square reach into the
boundary. A
square's area inside a boundary is the boundary's rings clipped against the square (Sutherland–Hodgman, the square
the convex window), holes subtracted. No KentOS code. The limits are the cases' own.

    python3 scripts/fixtures/road_terrain_coverage_cases.py [--check]
"""

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_quality_cases as quality  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
UID = quality.UID
UNCHANGED = quality.UNCHANGED
SX, TX = quality.SX, quality.TX
PX = 6000.0
HX = 12000.0
# A lattice with wide and narrow columns and rows: its 1 × 1 middle square is a hole among the wide ones.
HOLED_LATTICE = [(HX + x, float(y), 50.0 + 0.01 * x + 0.02 * y) for y in (0, 4, 5, 9) for x in (0, 4, 5, 9)]
UIDS = {}


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = quality.analysis.failed(code, message, path)

# Boundaries: (name, kind, outer, holes, bulges)
RECT = [(SX - 3, -1.0), (SX + 6.5, -1.0), (SX + 6.5, 2.6), (SX - 3, 2.6)]
ELL = [(TX - 2, -2.0), (TX + 5.5, -2.0), (TX + 5.5, 4.5), (TX + 12, 4.5), (TX + 12, 12.0), (TX - 2, 12.0)]
HOLED = [(PX + 2.5, 1.5), (PX + 8.5, 1.5), (PX + 8.5, 8.5), (PX + 2.5, 8.5)]
HOLE = [(PX + 4.5, 3.5), (PX + 6.5, 3.5), (PX + 6.5, 6.5), (PX + 4.5, 6.5)]
FAR = [(TX + 50, 50.0), (TX + 60, 50.0), (TX + 60, 60.0)]
WIDE = [(SX - 5, -5.0), (SX + 20, -5.0), (SX + 20, 8.0), (SX - 5, 8.0)]


def pts(ring):
    return [{"x": float(x), "y": float(y)} for x, y in ring]


def setup(work="road"):
    s = quality.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    s["layers"].append({"id": "sinir", "name": "Sınır", **copy.deepcopy(base)})
    s["layers"].append({"id": "delikli", "name": "Delikli", **copy.deepcopy(base)})
    s["layers"].append({"id": "kapsam", "name": "Kapsam", **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])

    def add(e, name):
        nonlocal k
        k += 1
        s["entities"].append({**e, "id": k, "layerId": "sinir", "attrs": {}})
        s["uids"].append(UID.format(k))
        UIDS[name] = UID.format(k)

    add({"kind": "polyline", "pts": pts(RECT), "bulges": [0.0] * 4, "closed": True}, "rect")
    add({"kind": "polyline", "pts": pts(list(reversed(ELL))), "closed": True}, "ell")
    add({"kind": "polygon", "pts": pts(HOLED), "holes": [{"pts": pts(HOLE)}]}, "holed")
    add({"kind": "polygon", "pts": pts(FAR)}, "far")
    add({"kind": "polygon", "pts": pts(WIDE)}, "wide")
    add({"kind": "polyline", "pts": pts(RECT)}, "open")
    add({"kind": "polyline", "pts": pts(RECT), "bulges": [0.0, 0.3, 0.0, 0.0], "closed": True}, "bulged")
    add({"kind": "polygon", "pts": pts([(SX, 0.0), (SX + 1, 0.0)])}, "flat")
    for n, (x, y, z) in enumerate(HOLED_LATTICE):
        k += 1
        s["entities"].append({"kind": "point", "id": k, "layerId": "delikli", "attrs": {}, "p": {"x": x, "y": y}, "z": z})
        s["uids"].append(UID.format(k))
    if work == "road":
        s["settings"]["road"]["surfaces"].append({"id": "t-12", "name": "Delikli", "points": {"layers": ["delikli"]}})
    return s


setup()
RINGS = {"rect": (RECT, []), "ell": (list(reversed(ELL)), []), "holed": (HOLED, [HOLE]), "far": (FAR, []),
         "wide": (WIDE, [])}


def shoelace(p):
    return sum(p[i][0] * p[(i + 1) % len(p)][1] - p[(i + 1) % len(p)][0] * p[i][1] for i in range(len(p))) / 2.0


def clip_box(ring, x0, x1, y0, y1):
    """The ring clipped against the box, side by side (Sutherland–Hodgman); its area."""
    poly = [tuple(p) for p in ring]
    # Each side: which coordinate (0 x, 1 y), its value, and whether inside is above it.
    for axis, v, above in ((0, x0, True), (0, x1, False), (1, y0, True), (1, y1, False)):
        def inside(p, axis=axis, v=v, above=above):
            return p[axis] >= v if above else p[axis] <= v
        out = []
        for i in range(len(poly)):
            p, q = poly[i], poly[(i + 1) % len(poly)]
            if inside(p):
                out.append(p)
            if inside(p) != inside(q):
                t = (v - p[axis]) / (q[axis] - p[axis])
                c = (v, p[1] + t * (q[1] - p[1])) if axis == 0 else (p[0] + t * (q[0] - p[0]), v)
                out.append(c)
        poly = out
        if not poly:
            return 0.0
    return abs(shoelace(poly)) if len(poly) >= 3 else 0.0


CORNERS = {**quality.CORNERS, "t-12": HOLED_LATTICE}
NAMES = {**quality.NAMES, "t-12": "Delikli"}


def lattice(sid):
    xs = sorted({p[0] for p in CORNERS[sid]})
    ys = sorted({p[1] for p in CORNERS[sid]})
    return [(xs[i], xs[i + 1], ys[j], ys[j + 1]) for j in range(len(ys) - 1) for i in range(len(xs) - 1)]


def answer(inp):
    longest = inp.get("longest")
    if longest is not None and not longest > 0:
        raise Refused("invalid_coverage", "Tahmin sayılan kenar boyu (longest) sıfırdan büyük olmalı.", "longest")
    sid = next((k for k, v in NAMES.items() if inp["surface"] in (k, v)), inp["surface"])
    region = RINGS[next(k for k, u in UIDS.items() if u == inp["boundary"])] if "boundary" in inp else None
    surface_area = covered = estimated = longest_edge = 0.0
    tri = 0
    for x0, x1, y0, y1 in lattice(sid):
        whole = (x1 - x0) * (y1 - y0)
        surface_area += whole
        if region:
            inside = clip_box(region[0], x0, x1, y0, y1) - sum(clip_box(h, x0, x1, y0, y1) for h in region[1])
        else:
            inside = whole
        if inside <= 0:
            continue
        covered += inside
        diag = ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5
        longest_edge = max(longest_edge, diag)
        if longest is not None and diag > longest:
            estimated += inside
            tri += 2
    out = {"surface": sid, "surfaceArea": surface_area}
    warns = []
    if region:
        b = abs(shoelace(region[0])) - sum(abs(shoelace(h)) for h in region[1])
        gap = max(b - covered, 0.0)
        out.update(boundaryArea=b, covered=covered, noData=gap, outside=surface_area - covered,
                   share=covered / b if b > 0 else None)
        if out["share"] is None:
            del out["share"]
        if gap > 1e-6 * max(b, 1.0):
            warns.append({"code": "terrain_no_data",
                          "message": f"“{NAMES[sid]}” yüzeyi sınırın {gap:.1f} m²'sini örtmüyor "
                                     f"(%{100 * gap / b:.1f}); orada veri yok, arazisi bilinmiyor.", "path": "boundary"})
    else:
        out["covered"] = covered
    out.update(estimated=estimated, estimatedShare=estimated / covered if covered > 0 else 0.0,
               estimatedTriangles=tri, longestEdge=longest_edge)
    return out, warns


def cases():
    out = []
    refused = quality.analysis.failed

    def q(name, inp, result=None, setup_=None):
        if result is None:
            try:
                o, w = answer(inp)
                result = {"status": "completed", "output": o, "warnings": w}
            except Refused as r:
                result = r.result
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    def b(name):
        return UIDS[name]

    q("Seyrek yüzeyin tamamı", {"surface": "t-10"})
    q("Seyrek yüzey, tahminle", {"surface": "t-10", "longest": 3.0})
    q("Seyrek yüzey proje sınırında", {"surface": "t-10", "boundary": b("rect"), "longest": 3.0})
    q("Sınır yüzeyi aşıyor", {"surface": "t-10", "boundary": b("wide"), "longest": 3.0})
    q("Sınır içinde hepsi ölçülmüş", {"surface": "t-10", "boundary": b("rect"), "longest": 9.0})
    q("Sınır kısmen ölçülmüş, sınır adıyla yüzey", {"surface": "Seyrek", "boundary": b("rect"), "longest": 1.5})
    q("L biçimli sınır teras üstünde", {"surface": "t-6", "boundary": b("ell")})
    q("L biçimli sınır, her üçgen tahmin", {"surface": "t-6", "boundary": b("ell"), "longest": 1.2})
    q("Delikli alan sınırı", {"surface": "t-5", "boundary": b("holed"), "longest": 2.0})
    q("Yüzeye değmeyen sınır", {"surface": "t-6", "boundary": b("far")})
    q("Terasın tamamı", {"surface": "Teras", "longest": 1.5})
    q("Delikli yüzey, ortadaki kare ölçülmüş", {"surface": "t-12", "longest": 3.0})
    # Refusals.
    bad = "Tahmin sayılan kenar boyu (longest) sıfırdan büyük olmalı."
    q("Kenar boyu sıfır", {"surface": "t-10", "longest": 0.0}, refused("invalid_coverage", bad, "longest"))
    q("Kenar boyu eksi", {"surface": "t-10", "longest": -2.0}, refused("invalid_coverage", bad, "longest"))
    q("Olmayan yüzey", {"surface": "t-99"}, refused("unknown_terrain", "“t-99” kimlikli ya da adlı yüzey projede yok.",
                                                     "surface"))
    q("Olmayan sınır", {"surface": "t-10", "boundary": "0192f600-7c3e-7d4a-9b1e-999999999999"},
      refused("entity_not_found", "“0192f600-7c3e-7d4a-9b1e-999999999999” kimlikli nesne çizimde yok.", "boundary"))
    for name, key in (("Açık çoklu çizgi sınır olamaz", "open"), ("Yaylı sınır", "bulged"), ("İki köşeli alan", "flat"),
                      ("Nokta sınır olamaz", "point"), ("Çizgi sınır olamaz", "line")):
        uid = quality.UIDS["kotsuz"] if key == "point" else quality.UIDS["cizgi"] if key == "line" else b(key)
        q(name, {"surface": "t-10", "boundary": uid},
          refused("invalid_coverage", f"“{uid}” sınır olamaz: düz kenarlı, tek parçalı kapalı bir çoklu çizgi ya da alan "
                                      "olmalı.", "boundary"))
    q("Yol projesi değil", {"surface": "t-10"},
      refused("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden biri değil."),
      setup_="$map")
    return out


# ── road.terrains.lowconfidence ───────────────────────────────────────────────────────────────────────────────────
def outlines(squares):
    """The union's outlines of axis-aligned squares (each a triangle pair: the diagonal cancels inside): edges used
    once, directed with the square on the left, chained end to start (rightmost turn at a shared corner), straight-on
    corners dropped; each from its least corner, in that order."""
    count, directed = {}, []
    for x0, x1, y0, y1 in squares:
        ring = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
        for k in range(4):
            a, b = ring[k], ring[(k + 1) % 4]
            key = (min(a, b), max(a, b))
            count[key] = count.get(key, 0) + 1
            directed.append((a, b))
    nxt = {}
    for a, b in directed:
        if count[(min(a, b), max(a, b))] == 1:
            nxt.setdefault(a, []).append(b)
    import math
    loops = []
    while any(nxt.values()):
        start = min(v for v, outs in nxt.items() if outs)
        ring, prev, cur = [start], start, nxt[start].pop()
        while cur != start:
            ring.append(cur)
            outs = nxt[cur]
            din = (cur[0] - prev[0], cur[1] - prev[1])

            def turn(o, din=din, cur=cur):
                d = (o[0] - cur[0], o[1] - cur[1])
                return math.atan2(din[0] * d[1] - din[1] * d[0], din[0] * d[0] + din[1] * d[1])
            k = min(range(len(outs)), key=lambda i: turn(outs[i]))
            prev, cur = cur, outs.pop(k)
        n = len(ring)
        kept = [ring[i] for i in range(n)
                if abs((ring[i][0] - ring[i - 1][0]) * (ring[(i + 1) % n][1] - ring[i][1])
                       - (ring[i][1] - ring[i - 1][1]) * (ring[(i + 1) % n][0] - ring[i][0])) > 1e-12]
        if len(kept) < 3:
            continue
        first = min(range(len(kept)), key=lambda i: kept[i])
        loops.append(kept[first:] + kept[:first])
    loops.sort(key=lambda r: r[0])
    return loops


def inside(ring, p):
    hit = False
    for i in range(len(ring)):
        a, b = ring[i], ring[(i + 1) % len(ring)]
        if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
            hit = not hit
    return hit


def centre(ring):
    a = cx = cy = 0.0
    for i in range(len(ring)):
        p, q = ring[i], ring[(i + 1) % len(ring)]
        c = p[0] * q[1] - q[0] * p[1]
        a += c
        cx += (p[0] + q[0]) * c
        cy += (p[1] + q[1]) * c
    return [cx / (3 * a), cy / (3 * a)]


def drawing(inp):
    sid = next((k for k, v in NAMES.items() if inp["surface"] in (k, v)), inp["surface"])
    squares = lattice(sid)
    est = [q for q in squares if ((q[1] - q[0]) ** 2 + (q[3] - q[2]) ** 2) ** 0.5 > inp["longest"]]
    loops = outlines(est)
    outers = [r for r in loops if shoelace(r) > 0]
    holes = [r for r in loops if shoelace(r) < 0]
    objects = [{"kind": "polyline", "pts": pts(r), "closed": True} for r in loops]
    total = 0.0
    for o in outers:
        own = sum(-shoelace(h) for h in holes
                  if inside(o, h[0]) and not any(x is not o and inside(x, h[0]) and shoelace(x) < shoelace(o)
                                                 for x in outers))
        a = shoelace(o) - own
        total += a
        if inp.get("labels"):
            c = centre(o)
            objects.append({"kind": "text", "p": {"x": c[0], "y": c[1]}, "text": f"Düşük güven · {a:.1f} m²".replace(".", ","),
                            "height": 2.5})
    if inp.get("outline"):
        objects += [{"kind": "polyline", "pts": pts(r), "closed": True} for r in outlines(squares)]
    return sid, objects, len(outers), len(holes), total


def draw_cases():
    out = []
    base = setup()
    first = max(e["id"] for e in base["entities"]) + 1
    refused = quality.analysis.failed
    stepped = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}

    def ok(name, inp, plan=False, undo=False):
        sid, objects, areas, holes, area = drawing(inp)
        result = {"status": "completed", "output": {"surface": sid, "areas": areas, "holes": holes, "area": area,
                                                    "uids": ["$any"] * len(objects), "revision": "$any" if plan else "$current"},
                  "warnings": []}
        if inp["layerId"] == "gizli":
            result["warnings"] = [{"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.",
                                   "path": "layerId"}]
        expect = dict(UNCHANGED) if plan or not objects else dict(stepped)
        if not plan and objects:
            expect["fields"] = {str(first + i): o for i, o in enumerate(objects)}
        steps = [{"op": "plan" if plan else "execute", "input": inp, "result": result, "expect": expect, "within": W}]
        if undo:
            steps.append({"op": "undo", "returns": "Düşük güvenli alanları çiz",
                          "expect": {"canUndo": False, "canRedo": True, "ids": [e["id"] for e in base["entities"]]}})
        out.append({"name": name, "steps": steps})

    def no(name, inp, result, setup_=None):
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    ok("Seyrek yüzeyin tahmin edilen alanı", {"surface": "t-10", "longest": 3.0, "layerId": "kapsam"}, undo=True)
    ok("Alanıyla", {"surface": "Seyrek", "longest": 3.0, "layerId": "kapsam", "labels": True})
    ok("Yüzeyin dış çizgisiyle", {"surface": "t-10", "longest": 3.0, "layerId": "kapsam", "outline": True, "labels": True})
    ok("Delikli tahmin alanı", {"surface": "t-12", "longest": 3.0, "layerId": "kapsam", "labels": True})
    ok("Yalnız iri kareler, dört ayrı alan", {"surface": "t-12", "longest": 4.5, "layerId": "kapsam", "labels": True})
    ok("Terasın her yeri", {"surface": "t-6", "longest": 1.2, "layerId": "kapsam", "labels": True})
    ok("Tahmin yok, yalnız dış çizgi", {"surface": "t-6", "longest": 2.0, "layerId": "kapsam", "outline": True})
    ok("Planı yazmaz", {"surface": "t-12", "longest": 3.0, "layerId": "kapsam", "labels": True}, plan=True)
    ok("Gizli katmana uyarıyla", {"surface": "t-10", "longest": 3.0, "layerId": "gizli"})
    ok("Seyrek ve dar sınır", {"surface": "t-10", "longest": 1.0, "layerId": "kapsam", "labels": True})
    ok("Delikli, dış çizgisiyle", {"surface": "t-12", "longest": 3.0, "layerId": "kapsam", "outline": True, "labels": True})
    ok("Tahmin yok, hiçbir şey çizilmez", {"surface": "t-6", "longest": 2.0, "layerId": "kapsam"})
    # Refusals.
    bad = "Tahmin sayılan kenar boyu (longest) sıfırdan büyük olmalı."
    no("Kenar boyu sıfır", {"surface": "t-10", "longest": 0.0, "layerId": "kapsam"}, refused("invalid_coverage", bad, "longest"))
    no("Olmayan yüzey", {"surface": "yok", "longest": 3.0, "layerId": "kapsam"},
       refused("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Olmayan katman", {"surface": "t-10", "longest": 3.0, "layerId": "yok"},
       refused("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Kilitli katman", {"surface": "t-10", "longest": 3.0, "layerId": "kilitli"},
       refused("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı "
               "etkinleştirin.", "layerId"))
    no("Katman grubu", {"surface": "t-10", "longest": 3.0, "layerId": "grup"},
       {"status": "failed", "error": {"code": "not_a_layer", "message": "$any", "path": "layerId"}})
    no("Geçersiz beklenen sürüm", {"surface": "t-10", "longest": 3.0, "layerId": "kapsam", "expectedRevision": "on"},
       refused("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. "
               "Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    no("Eski sürüme göre", {"surface": "t-10", "longest": 3.0, "layerId": "kapsam", "expectedRevision": "41"},
       {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra "
        "değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision",
        "revision": "$current"}})
    no("Yol projesi değil", {"surface": "t-10", "longest": 3.0, "layerId": "kapsam"},
       refused("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim ikisinden biri değil."),
       setup_="$map")
    return out


def main():
    docs = {
        "road.terrains.coverage.json": ("road.terrains.coverage",
                                        "Yüzeyin kapsamı: proje sınırında örtülen, verisiz ve tahmin edilen alan", cases()),
        "road.terrains.lowconfidence.json": ("road.terrains.lowconfidence",
                                             "Düşük güvenli alanları çiz: tahmin edilen üçgenlerin dış çizgileri",
                                             draw_cases()),
    }
    stale = []
    for name, (command, title, found) in docs.items():
        for c in found:
            if c.get("setup") == "$map":
                c["setup"] = setup(work="architecture")
        doc = {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
               "note": "scripts/fixtures/road_terrain_coverage_cases.py'den; kafes yüzeylerin kareleri, sınırın halkaları "
                       "karelerle Sutherland–Hodgman ile kırpılarak, tahmin edilen karelerin birleşiminin dış çizgileri, "
                       "ADR 0920'nin kuralıyla; KentOS kodu olmadan.",
               "setup": setup(), "cases": found}
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if not path.exists() or path.read_text() != text:
                stale.append(name)
        else:
            path.write_text(text)
        print(f"{name}: {len(found)} durum")
    if stale:
        sys.exit(f"güncel değil: {stale}")


if __name__ == "__main__":
    main()
