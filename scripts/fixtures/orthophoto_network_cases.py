#!/usr/bin/env python3
"""The shared cases of `cad.orthophoto.network` (Birleşme çizgisi ağı) and `cad.orthophoto.footprint` (Ayak izi;
plan/map-cad.md MAP-12, docs/adr/5260), without KentOS code.

    uv run --no-project --python 3.11 --with shapely python3 scripts/fixtures/orthophoto_network_cases.py
    uv run --no-project --python 3.11 --with shapely python3 scripts/fixtures/orthophoto_network_cases.py --check

The rules, by hand:
- network: footprints are the rasters' four corners (GDAL's affine, pixel corners (0, h), (w, h), (w, 0), (0, 0)) or
  the areas given, convex; an image's centre is its footprint's area centroid; every place of the footprints' union
  goes to the covering image whose centre is nearest (the first given of equals). Here an image's element is found
  with shapely: its footprint less the union, over the other images, of the part of each one's footprint nearer its
  centre (a half-plane by the bisector, made as a large polygon and intersected). Rings: counter-clockwise outer
  rings, clockwise holes, the closing repeat and collinear corners dropped (a corner whose turn is within 10⁻¹² of the
  coordinates' size), started at the southernmost corner (westernmost of equals); parts and holes ordered by their
  starts. A junction is a corner three or more elements share; where three centres' cells meet it is their
  circumcentre (asserted here);
- footprint: a pixel is valid when its value is not the GeoTIFF's NoData (GDAL_NODATA, tag 42113); the valid pixels'
  outline is the union of their squares (pixel corners, 4-connectivity: squares meeting only at a corner are apart),
  its rings tidied as above on the ground (the affine); `hull` the convex hull of the valid squares. The images are
  written here byte by byte (a baseline 8-bit TIFF, one strip, GeoTIFF keys, the NoData tag).
"""

import copy
import json
import math
import struct
import sys
from pathlib import Path

from shapely.affinity import translate
from shapely.geometry import MultiPolygon, Polygon, box
from shapely.ops import unary_union

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
NET_OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.orthophoto.network.json"
FOOT_OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.orthophoto.footprint.json"
FILES = ROOT / "fixtures" / "orthophoto" / "v1" / "files"
X0, Y0 = 487000.0, 4420000.0
SAME = {"revision": "same", "canUndo": False}
W = 1e-6


def uid(n):
    return f"0192f5c0-5260-7000-8000-{n:012x}"


def layer(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def raster_ent(x0, y0, w, h, pixel=1.0, file="ag-a.tif", srid=5254, affine=None):
    return {"kind": "raster", "layerId": "altlik", "attrs": {},
            "affine": affine or [X0 + x0, pixel, 0.0, Y0 + y0 + h * pixel, 0.0, -pixel],
            "width": w, "height": h, "bands": 1, "sample": "u8", "srid": srid,
            "style": {"render": "gray", "bands": [1]}, "file": f"$fixture:orthophoto/v1/files/{file}"}


def area_ent(ring, layer_="ayak-izi"):
    return {"kind": "polygon", "layerId": layer_, "attrs": {}, "pts": [{"x": x, "y": y} for x, y in ring]}


def doc(entities, layers=(), name="Birleşme çizgisi ağı"):
    ents = [dict(e, id=k + 1) for k, e in enumerate(entities)]
    return {"format": "kentos.document", "version": 2, "name": name,
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("altlik", "Altlık"), layer("ayak-izi", "Ayak izleri (çizilen)")] + list(layers),
            "activeLayer": "0", "entities": ents, "uids": [uid(k + 1) for k in range(len(ents))],
            "styles": {"items": [], "categories": []}}


def corners(r):
    t, w, h = r["affine"], r["width"], r["height"]
    at = lambda i, j: (t[0] + i * t[1] + j * t[2], t[3] + i * t[4] + j * t[5])  # noqa: E731
    return [at(0, h), at(w, h), at(w, 0), at(0, 0)]


def signed2(r):
    return sum(r[k][0] * r[(k + 1) % len(r)][1] - r[(k + 1) % len(r)][0] * r[k][1] for k in range(len(r)))


def tidy(ring, ccw, tol):
    """The module's ring rule, written here: repeats and collinear corners dropped, turned, started south-west."""
    r = [tuple(p) for p in ring]
    if len(r) > 1 and r[0] == r[-1]:
        r.pop()
    out = []
    for p in r:
        if not out or abs(out[-1][0] - p[0]) > tol or abs(out[-1][1] - p[1]) > tol:
            out.append(p)
    r = out
    while len(r) >= 3:
        n = len(r)
        drop = None
        for k in range(n):
            a, b, c = r[k - 1], r[k], r[(k + 1) % n]
            u, v = (b[0] - a[0], b[1] - a[1]), (c[0] - b[0], c[1] - b[1])
            cr = u[0] * v[1] - u[1] * v[0]
            ln = max(math.hypot(*u), math.hypot(*v))
            if abs(cr) <= tol * ln or math.hypot(*u) <= tol:
                drop = k
                break
        if drop is None:
            break
        r.pop(drop)
    if (signed2(r) > 0) != ccw:
        r.reverse()
    s = min(range(len(r)), key=lambda k: (r[k][1], r[k][0]))
    r = r[s:] + r[:s]
    return [[x, y] for x, y in r]


def polys(g):
    if g.is_empty:
        return []
    if isinstance(g, Polygon):
        return [g]
    if isinstance(g, MultiPolygon):
        return list(g.geoms)
    return [p for p in getattr(g, "geoms", []) if isinstance(p, Polygon)]


def parts_of(g, tol):
    out = []
    for p in polys(g):
        if p.area <= tol * tol:
            continue
        ring = tidy(list(p.exterior.coords), True, tol)
        holes = [tidy(list(h.coords), False, tol) for h in p.interiors]
        holes = sorted([h for h in holes if len(h) >= 3], key=lambda h: (h[0][1], h[0][0]))
        if len(ring) >= 3:
            out.append({"ring": ring, **({"holes": holes} if holes else {})})
    return sorted(out, key=lambda p: (p["ring"][0][1], p["ring"][0][0]))


def area_of(parts, o):
    rel = lambda r: signed2([[q[0] - o[0], q[1] - o[1]] for q in r]) / 2.0  # noqa: E731
    return sum(rel(p["ring"]) + sum(rel(h) for h in p.get("holes", [])) for p in parts)


def centroid(r):
    o = r[0]
    a2 = cx = cy = 0.0
    for k in range(len(r)):
        p = (r[k][0] - o[0], r[k][1] - o[1])
        q = (r[(k + 1) % len(r)][0] - o[0], r[(k + 1) % len(r)][1] - o[1])
        c = p[0] * q[1] - q[0] * p[1]
        a2 += c
        cx += (p[0] + q[0]) * c
        cy += (p[1] + q[1]) * c
    return [o[0] + cx / (3 * a2), o[1] + cy / (3 * a2)]


def ccw_ring(r):
    r = [list(p) for p in r]
    return r if signed2(r) > 0 else r[::-1]


def network(rings):
    """The elements (parts, area) and the junctions of the footprints `rings` (the module's rule, by shapely)."""
    o = rings[0][0]
    size = max(max(abs(p[0]), abs(p[1])) for r in rings for p in r)
    tol = size * 1e-12 + 1e-9
    loc = [[(p[0] - o[0], p[1] - o[1]) for p in r] for r in rings]
    cs = [centroid(r) for r in loc]
    big = 1e7
    elements = []
    for i, fi in enumerate(loc):
        cut = []
        for j, fj in enumerate(loc):
            if i == j:
                continue
            ci, cj = cs[i], cs[j]
            n = (2 * (ci[0] - cj[0]), 2 * (ci[1] - cj[1]))
            c = ci[0] ** 2 + ci[1] ** 2 - cj[0] ** 2 - cj[1] ** 2
            if n == (0.0, 0.0):
                if j < i:
                    cut.append(Polygon(fj))
                continue
            # The half-plane n · p ≤ c as a large square cut by the line.
            ln = math.hypot(*n)
            u = (n[0] / ln, n[1] / ln)
            base = (u[0] * c / ln, u[1] * c / ln)
            t = (-u[1], u[0])
            half = Polygon([(base[0] + t[0] * big, base[1] + t[1] * big), (base[0] - t[0] * big, base[1] - t[1] * big),
                            (base[0] - t[0] * big - u[0] * big, base[1] - t[1] * big - u[1] * big),
                            (base[0] + t[0] * big - u[0] * big, base[1] + t[1] * big - u[1] * big)])
            cut.append(Polygon(fj).intersection(half))
        rest = Polygon(fi).difference(unary_union(cut)) if cut else Polygon(fi)
        parts = parts_of(translate(rest, o[0], o[1]), tol)
        elements.append(parts)
    corners_ = []
    for i, parts in enumerate(elements):
        for p in parts:
            for q in p["ring"] + [v for h in p.get("holes", []) for v in h]:
                hit = next((c for c in corners_ if abs(c[0][0] - q[0]) <= tol and abs(c[0][1] - q[1]) <= tol), None)
                if hit is None:
                    corners_.append((q, [i]))
                elif i not in hit[1]:
                    hit[1].append(i)
    junctions = sorted([(at, sorted(who)) for at, who in corners_ if len(who) >= 3], key=lambda j: (j[0][1], j[0][0]))
    centres = [[c[0] + o[0], c[1] + o[1]] for c in cs]
    return elements, junctions, centres


def circumcentre(a, b, c):
    ax, ay = a
    bx, by = b
    cx, cy = c
    d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by))
    ux = ((ax * ax + ay * ay) * (by - cy) + (bx * bx + by * by) * (cy - ay) + (cx * cx + cy * cy) * (ay - by)) / d
    uy = ((ax * ax + ay * ay) * (cx - bx) + (bx * bx + by * by) * (ax - cx) + (cx * cx + cy * cy) * (bx - ax)) / d
    return [ux, uy]


def net_answer(s, inp, layer_="mozaik-elemanlari", opened=True):
    ents = {uid(k + 1): e for k, e in enumerate(s["entities"])}
    if inp.get("footprints"):
        rings = [ccw_ring([(p["x"], p["y"]) for p in ents[f]["pts"]]) for f in inp["footprints"]]
    else:
        rings = [ccw_ring(corners(ents[i])) for i in inp["images"]]
    elements, junctions, centres = network(rings)
    out_el = []
    o = rings[0][0]
    for k, parts in enumerate(elements):
        out_el.append({"image": inp["images"][k], "parts": parts, "area": area_of(parts, o), "centre": centres[k]})
    covered = sum(e["area"] for e in out_el)
    notes = [f"{len(rings)} görüntünün mozaik elemanları: her yer, onu kaplayan görüntülerden ayak izinin merkezi en yakın "
             f"olanın; kaplanan alan {covered:.2f} m²."]
    notes.append("Üç ya da daha çok elemanın buluştuğu düğüm yok." if not junctions else
                 f"{len(junctions)} düğümde üç ya da daha çok eleman buluşuyor (üçlü düğüm).")
    if any(len(e["parts"]) > 1 for e in out_el):
        notes.append("Bir elemanın birden çok parçası var: görüntü mozaikte ayrık yerlerde kullanılır.")
    notes.append("Birleşme çizgileri elemanların ortak kenarlarıdır. BÖHHBÜY birleşme çizgisinin yerine kural koymaz; "
                 "kayıklık m. 67(6) ile ayrıca denetlenir (Ortofoto karşılaştırması).")
    out = {"elements": out_el, "junctions": [{"at": at, "images": [inp["images"][k] for k in who]} for at, who in junctions],
           "covered": covered, "layer": layer_, "notes": notes, "revision": "$current"}
    if opened:
        out["opened"] = True
    return out, junctions, centres


def written_case(name, s, inp, out, layer_, step, uids_key, note=None):
    written = copy.deepcopy(out)
    if uids_key == "elements":
        for e in written["elements"]:
            e["uids"] = ["$any"]
    else:
        written["uid"] = "$any"
    case = {"name": name, "setup": s, "steps": [
        {"op": "execute", "input": inp, "result": {"status": "completed", "output": written, "warnings": []},
         "expect": {"revision": "changed", "canUndo": True}, "within": W},
        {"op": "undo", "returns": step, "expect": {"revision": "changed"}}]}
    if note:
        case["note"] = note
    return case


def plan_case(name, s, inp, out, op="plan", note=None):
    result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
    case = {"name": name, "setup": s, "steps": [{"op": op, "input": inp, "result": result, "expect": SAME, "within": W}]}
    if note:
        case["note"] = note
    return case


def refused(name, s, inp, code, message, path_):
    err = {"code": code, "message": message}
    if path_:
        err["path"] = path_
    return {"name": name, "setup": s, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": err},
                                                 "expect": SAME}]}


# Network setups: three strips meeting (A west, B east, C north over both), four in a 2 × 2 grid, two only.
A = raster_ent(0, 0, 100, 60)
B = raster_ent(40, 0, 100, 60)
C = raster_ent(20, 30, 100, 60)
GRID = [raster_ent(0, 0, 60, 60), raster_ent(40, 0, 60, 60), raster_ent(0, 40, 60, 60), raster_ent(40, 40, 60, 60)]
FAR = raster_ent(400, 0, 50, 50)
SMALL = raster_ent(40, 20, 20, 20)
BIG = raster_ent(0, 0, 100, 60)


def net_cases():
    out = []
    three = doc([A, B, C])
    ids3 = [uid(1), uid(2), uid(3)]
    o3, j3, c3 = net_answer(three, {"images": ids3})
    assert len(j3) == 1, j3
    cc = circumcentre(*c3)
    assert abs(j3[0][0][0] - cc[0]) < 1e-6 and abs(j3[0][0][1] - cc[1]) < 1e-6, (j3, cc)
    out.append(plan_case("üç görüntü: elemanlar ve tek üçlü düğüm (merkezlerin çevrel merkezi)", three, {"images": ids3}, o3))
    out.append(written_case("yazılır: üç eleman Mozaik elemanları katmanına, katman aynı adımda açılır; geri alınır", three,
                            {"images": ids3}, o3, "mozaik-elemanlari", "Mozaik elemanları ağı", "elements"))
    out.append(plan_case("doğrulama", three, {"images": ids3}, o3, op="validate"))
    rev = [uid(3), uid(1), uid(2)]
    out.append(plan_case("görüntülerin sırası değişince elemanlar aynı, düğümün görüntüleri girdinin sırasıyla", three,
                         {"images": rev}, net_answer(three, {"images": rev})[0]))
    two = doc([A, B])
    o2, j2, _ = net_answer(two, {"images": [uid(1), uid(2)]})
    assert not j2
    out.append(plan_case("iki görüntü: düğüm yok, birleşme çizgisi ortak sınır", two, {"images": [uid(1), uid(2)]}, o2))
    grid = doc(GRID)
    idsg = [uid(k + 1) for k in range(4)]
    og, jg, _ = net_answer(grid, {"images": idsg})
    assert len(jg) == 1 and len(jg[0][1]) == 4, jg
    out.append(plan_case("dört görüntü 2 × 2: dört eleman bir düğümde (merkezler aynı çemberde)", grid, {"images": idsg}, og))
    out.append(written_case("dört görüntü yazılır", grid, {"images": idsg}, og, "mozaik-elemanlari", "Mozaik elemanları ağı",
                            "elements"))
    far = doc([A, B, C, FAR])
    of, _, _ = net_answer(far, {"images": [uid(k + 1) for k in range(4)]})
    out.append(plan_case("örtüşmeyen görüntü: ayak izinin hepsi kendi elemanı", far, {"images": [uid(k + 1) for k in range(4)]}, of))
    holed = doc([SMALL, BIG])
    oh, _, _ = net_answer(holed, {"images": [uid(1), uid(2)]})
    assert any(p.get("holes") for p in oh["elements"][1]["parts"]), oh
    out.append(plan_case("küçük görüntü büyüğün içinde ve önce verilmiş: büyüğün elemanı delikli", holed,
                         {"images": [uid(1), uid(2)]}, oh,
                         note="merkezler aynı yerde: eşitlikte önce verilen kazanır"))
    fps = [[(X0 + x, Y0 + y) for x, y in r] for r in ([(0, 0), (90, 0), (95, 55), (0, 60)], [(70, 0), (170, 5), (170, 60),
                                                                                             (75, 60)],
                                                      [(40, 40), (130, 45), (135, 100), (35, 100)])]
    given = doc([A, B, C] + [area_ent(r) for r in fps])
    inp_g = {"images": ids3, "footprints": [uid(4), uid(5), uid(6)]}
    out.append(plan_case("verilen ayak izleriyle: dörtgenler, merkezleri alan ağırlık merkezleri", given, inp_g,
                         net_answer(given, inp_g)[0]))
    lay = doc([A, B, C], layers=[layer("mozaik", "Ortofoto mozaiği")])
    out.append(written_case("verilen katmana yazılır, katman açılmaz", lay, {"images": ids3, "layer": "mozaik"},
                            net_answer(lay, {"images": ids3}, layer_="mozaik", opened=False)[0], "mozaik",
                            "Mozaik elemanları ağı", "elements"))
    out.append(plan_case("Yol projesinde", dict(three, settings=dict(three["settings"], cadWork="road")), {"images": ids3}, o3))
    bad = "invalid_input"
    out.append(refused("tek görüntü", three, {"images": [uid(1)]}, bad, "Ağ 2 ile 50 arasında ortofotoyla kurulur; 1 verildi.",
                       "images"))
    out.append(refused("aynı görüntü iki kez", three, {"images": [uid(1), uid(2), uid(1)]}, bad,
                       "Aynı ortofoto iki kez verildi; her görüntü ağda bir kez yer alır.", "images[2]"))
    out.append(refused("ayak izleri her görüntüye bir değil", given, {"images": ids3, "footprints": [uid(4)]}, bad,
                       "Ayak izleri görüntülerle aynı sırada, her görüntüye bir tane verilir (3 görüntü, 1 ayak izi).",
                       "footprints"))
    concave = doc([A, B, area_ent([(X0, Y0), (X0 + 100, Y0), (X0 + 50, Y0 + 20), (X0 + 100, Y0 + 60), (X0, Y0 + 60)]),
                   area_ent([(X0 + 70, Y0), (X0 + 170, Y0), (X0 + 170, Y0 + 60), (X0 + 70, Y0 + 60)])])
    out.append(refused("içbükey ayak izi", concave, {"images": [uid(1), uid(2)], "footprints": [uid(3), uid(4)]}, bad,
                       "1. ayak izi dışbükey değil; ağ dışbükey ayak izleriyle kurulur. İçbükey ayak izini parçalara bölün ya "
                       "da Ayak izi'nin zarfını kullanın.", "footprints[0]"))
    twin = doc([A, dict(A)])
    out.append(refused("aynı yerdeki ikinci görüntüye yer kalmaz", twin, {"images": [uid(1), uid(2)]}, bad,
                       "2. görüntünün ayak izinin her yeri başka bir görüntünün merkezine daha yakın; mozaikte ona yer "
                       "kalmıyor. Onu ağdan çıkarın.", "images[1]"))
    out.append(refused("raster değil", given, {"images": [uid(1), uid(4)]}, "not_a_raster",
                       f"“{uid(4)}” bir raster değil; ortofotoyu seçin.", "images[1]"))
    out.append(refused("geçersiz kimlik", three, {"images": [uid(1), "x"]}, "invalid_uid",
                       "“x” geçerli bir nesne kimliği değil.", "images[1]"))
    out.append(refused("çizimde olmayan nesne", three, {"images": [uid(1), uid(99)]}, "entity_not_found",
                       f"“{uid(99)}” kimlikli nesne çizimde yok.", "images[1]"))
    other = doc([A, dict(B, srid=5256)])
    out.append(refused("başka sistemde görüntü", other, {"images": [uid(1), uid(2)]}, "crs_mismatch",
                       "Ortofotonun koordinat sistemi EPSG:5256, projeninki EPSG:5254; birleşme çizgisi aynı sistemdeki "
                       "görüntüler arasında kurulur. Önce projeye çevirin (Raster ekle › Projenin sistemine çevir ve ekle).",
                       "images[1]"))
    out.append(refused("olmayan katman", three, {"images": ids3, "layer": "yok"}, "layer_not_found", "“yok” katmanı çizimde yok.",
                       "layer"))
    locked = doc([A, B, C], layers=[layer("kilitli", "Kilitli", locked=True)])
    out.append(refused("kilitli katman", locked, {"images": ids3, "layer": "kilitli"}, "layer_locked",
                       "“Kilitli” katmanı kilitli; mozaik elemanları bu katmana yazılır. Kilidi Katmanlar panelinden açın.",
                       "layer"))
    out.append({"name": "eski sürüme göre hazırlanmış", "setup": three, "steps": [{
        "op": "execute", "input": {"images": ids3, "expectedRevision": "7"},
        "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan "
                   "sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                   "path": "expectedRevision", "revision": "$current"}}, "expect": SAME}]})
    out.append(refused("sürüm yazısı geçersiz", three, {"images": ids3, "expectedRevision": "abc"}, "invalid_revision",
                       "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                       "ya da komutun planından alın.", "expectedRevision"))
    return out


# Footprint images: 8-bit, NoData 0 (or none). 'a': 14 × 10, a valid block with a hole of 2 × 1 and an island pixel
# touching the block only at a corner; 'b' the same without the NoData tag; 'c' all NoData.
def image_a():
    w, h = 14, 10
    rows = [[0] * w for _ in range(h)]
    for j in range(1, 8):
        for i in range(2, 11):
            rows[j][i] = 40 + i + j
    rows[4][5] = rows[4][6] = 0          # a hole
    rows[8][11] = 200                    # touches (10, 7) only at a corner
    rows[2][2] = 0                       # a notch on the block's edge
    return rows


IMAGES = {"ayak-izi-a.tif": (image_a(), 0), "ayak-izi-b.tif": (image_a(), None),
          "ayak-izi-c.tif": ([[0] * 6 for _ in range(4)], 0)}


def geotiff(rows, nodata):
    """A baseline little-endian TIFF: 8-bit grey, one strip, uncompressed; GeoTIFF's scale, tie point (1 m pixels at the
    origin; the raster's affine is the drawing's) and keys; GDAL's NoData as ASCII when given."""
    h, w = len(rows), len(rows[0])
    data = bytes(v for row in rows for v in row)
    keys = [1, 1, 0, 3, 1024, 0, 1, 1, 1025, 0, 1, 1, 3072, 0, 1, 5254]
    geokeys = struct.pack(f"<{len(keys)}H", *keys)
    scale = struct.pack("<3d", 1.0, 1.0, 0.0)
    tie = struct.pack("<6d", 0.0, 0.0, 0.0, X0, Y0 + h, 0.0)
    entries = [(256, 4, 1, w), (257, 4, 1, h), (258, 3, 1, 8), (259, 3, 1, 1), (262, 3, 1, 1), (273, 4, 1, None),
               (277, 3, 1, 1), (278, 4, 1, h), (279, 4, 1, len(data)), (284, 3, 1, 1), (33550, 12, 3, scale),
               (33922, 12, 6, tie), (34735, 3, len(keys), geokeys)]
    if nodata is not None:
        text = f"{nodata}".encode() + b"\x00"
        entries.append((42113, 2, len(text), text if len(text) > 4 else text.ljust(4, b"\x00")))
    ifd_at = 8
    extra_at = ifd_at + 2 + 12 * len(entries) + 4
    blobs, packed = b"", []
    for tag, kind, count, value in entries:
        if isinstance(value, bytes) and len(value) > 4:
            packed.append((tag, kind, count, struct.pack("<I", extra_at + len(blobs))))
            blobs += value
        else:
            packed.append((tag, kind, count, value))
    data_at = extra_at + len(blobs)
    out = b"II*\x00" + struct.pack("<I", ifd_at) + struct.pack("<H", len(packed))
    for tag, kind, count, value in packed:
        if tag == 273:
            value = data_at
        field = value if isinstance(value, bytes) else (struct.pack("<HH", value, 0) if kind == 3 else struct.pack("<I", value))
        out += struct.pack("<HHI", tag, kind, count) + field
    return out + struct.pack("<I", 0) + blobs + data


def foot_answer(r, rows, nodata, hull=False, layer_="ayak-izleri", opened=True):
    t = r["affine"]
    ground = lambda i, j: (t[0] + i * t[1] + j * t[2], t[3] + i * t[4] + j * t[5])  # noqa: E731
    h, w = len(rows), len(rows[0])
    valid = [(i, j) for j in range(h) for i in range(w) if nodata is None or rows[j][i] != nodata]
    squares = unary_union([box(i, j, i + 1, j + 1) for i, j in valid])
    if hull:
        squares = squares.convex_hull
    g = Polygon([ground(*p) for p in squares.exterior.coords], [[ground(*p) for p in hole.coords] for hole in squares.interiors]) \
        if isinstance(squares, Polygon) else MultiPolygon([Polygon([ground(*p) for p in q.exterior.coords],
                                                                    [[ground(*p) for p in hole.coords] for hole in q.interiors])
                                                           for q in squares.geoms])
    cs = corners(r)
    size = max(max(abs(x), abs(y)) for x, y in cs)
    tol = size * 1e-12 + 1e-9
    parts = parts_of(g, tol)
    area = area_of(parts, parts[0]["ring"][0])
    holes = sum(len(p.get("holes", [])) for p in parts)
    count = len(valid)
    notes = [f"Ayak izi: NoData olmayan {count} pikselin dışbükey zarfı, {area:.2f} m²; birleşme çizgisi ve ağ bunu ayak izi "
             "olarak alır." if hull else
             f"Ayak izi: NoData olmayan {count} pikselin dış sınırı, piksellerin kenarlarından; {len(parts)} parça, {holes} "
             f"delik, {area:.2f} m²."]
    nd = f"{float(nodata):g}" if nodata is not None else ""
    notes.append(f"NoData değeri {nd}: bir piksel, bantlarından biri bu değerde değilse geçerlidir. Köşeden değen "
                 "pikseller ayrı parçadır." if nodata is not None else
                 "Rasterin NoData değeri yok: her piksel geçerli, ayak izi rasterin sınırıdır.")
    out = {"parts": parts, "area": area, "pixels": w * h, "valid": count, "layer": layer_, "notes": notes,
           "revision": "$current"}
    if nodata is not None:
        out["nodata"] = float(nodata)
    if hull:
        out["hull"] = True
    if opened:
        out["opened"] = True
    return out


def foot_cases():
    out = []
    ra = raster_ent(0, 0, 14, 10, pixel=0.5, file="ayak-izi-a.tif")
    rows, nd = IMAGES["ayak-izi-a.tif"]
    s = doc([ra], name="Ayak izi")
    o = foot_answer(ra, rows, nd)
    assert len(o["parts"]) == 2 and sum(len(p.get("holes", [])) for p in o["parts"]) == 1, o["parts"]
    out.append(plan_case("NoData'dan ayak izi: bir delik, köşeden değen ayrı parça, kenardaki çentik", s, {"raster": uid(1)}, o))
    out.append(written_case("yazılır: Ayak izleri katmanına, katman aynı adımda açılır; geri alınır", s, {"raster": uid(1)}, o,
                            "ayak-izleri", "Ayak izi", "uid"))
    out.append(plan_case("doğrulama", s, {"raster": uid(1)}, o, op="validate"))
    oh = foot_answer(ra, rows, nd, hull=True)
    assert len(oh["parts"]) == 1 and "holes" not in oh["parts"][0]
    out.append(plan_case("dışbükey zarf: tek parça, deliksiz", s, {"raster": uid(1), "hull": True}, oh))
    out.append(written_case("zarf yazılır", s, {"raster": uid(1), "hull": True}, oh, "ayak-izleri", "Ayak izi", "uid"))
    rb = raster_ent(0, 0, 14, 10, pixel=0.5, file="ayak-izi-b.tif")
    sb = doc([rb], name="Ayak izi")
    out.append(plan_case("NoData'sı olmayan raster: ayak izi rasterin sınırı", sb, {"raster": uid(1)},
                         foot_answer(rb, IMAGES["ayak-izi-b.tif"][0], None)))
    turn = math.radians(20.0)
    t = [X0 + 30.0, 0.5 * math.cos(turn), 0.5 * math.sin(turn), Y0 + 40.0, 0.5 * math.sin(turn), -0.5 * math.cos(turn)]
    rt = raster_ent(0, 0, 14, 10, file="ayak-izi-a.tif", affine=t)
    st = doc([rt], name="Ayak izi")
    out.append(plan_case("döndürülmüş raster: pikseller afinle yere", st, {"raster": uid(1)}, foot_answer(rt, rows, nd)))
    out.append(plan_case("döndürülmüş rasterin zarfı", st, {"raster": uid(1), "hull": True}, foot_answer(rt, rows, nd, hull=True)))
    sl = doc([ra], layers=[layer("izler", "İzler")], name="Ayak izi")
    out.append(written_case("verilen katmana yazılır, katman açılmaz", sl, {"raster": uid(1), "layer": "izler"},
                            foot_answer(ra, rows, nd, layer_="izler", opened=False), "izler", "Ayak izi", "uid"))
    out.append(plan_case("Yol projesinde", dict(s, settings=dict(s["settings"], cadWork="road")), {"raster": uid(1)}, o))
    bad = "invalid_input"
    rc = raster_ent(0, 0, 6, 4, pixel=0.5, file="ayak-izi-c.tif")
    out.append(refused("her pikseli NoData", doc([rc], name="Ayak izi"), {"raster": uid(1)}, bad,
                       "Rasterin her pikseli NoData; ayak izi yok.", "raster"))
    huge = raster_ent(0, 0, 6000, 5000, pixel=0.5, file="ayak-izi-a.tif")
    out.append(refused("çok piksel", doc([huge], name="Ayak izi"), {"raster": uid(1)}, bad,
                       "Raster 30000000 piksel; ayak izi en çok 25000000 pikselden çıkarılır. Daha küçük bir kopyasını "
                       "kullanın.", "raster"))
    missing = raster_ent(0, 0, 14, 10, pixel=0.5, file="yok.tif")
    out.append(refused("dosyası yok", doc([missing], name="Ayak izi"), {"raster": uid(1)}, "file_not_found",
                       "Ortofotonun dosyası “$fixture:orthophoto/v1/files/yok.tif” bulunamadı.", "raster"))
    sa = doc([ra, area_ent([(X0, Y0), (X0 + 1, Y0), (X0 + 1, Y0 + 1)])], name="Ayak izi")
    out.append(refused("raster değil", sa, {"raster": uid(2)}, "not_a_raster", f"“{uid(2)}” bir raster değil; ortofotoyu seçin.",
                       "raster"))
    out.append(refused("geçersiz kimlik", s, {"raster": "x"}, "invalid_uid", "“x” geçerli bir nesne kimliği değil.", "raster"))
    out.append(refused("çizimde olmayan nesne", s, {"raster": uid(9)}, "entity_not_found", f"“{uid(9)}” kimlikli nesne çizimde yok.",
                       "raster"))
    so = doc([dict(ra, srid=5256)], name="Ayak izi")
    out.append(refused("başka sistemde raster", so, {"raster": uid(1)}, "crs_mismatch",
                       "Ortofotonun koordinat sistemi EPSG:5256, projeninki EPSG:5254; birleşme çizgisi aynı sistemdeki "
                       "görüntüler arasında kurulur. Önce projeye çevirin (Raster ekle › Projenin sistemine çevir ve ekle).",
                       "raster"))
    out.append(refused("olmayan katman", s, {"raster": uid(1), "layer": "yok"}, "layer_not_found", "“yok” katmanı çizimde yok.",
                       "layer"))
    sk = doc([ra], layers=[layer("kilitli", "Kilitli", locked=True)], name="Ayak izi")
    out.append(refused("kilitli katman", sk, {"raster": uid(1), "layer": "kilitli"}, "layer_locked",
                       "“Kilitli” katmanı kilitli; ayak izi bu katmana yazılır. Kilidi Katmanlar panelinden açın.", "layer"))
    out.append({"name": "eski sürüme göre hazırlanmış", "setup": s, "steps": [{
        "op": "execute", "input": {"raster": uid(1), "expectedRevision": "7"},
        "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan "
                   "sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
                   "path": "expectedRevision", "revision": "$current"}}, "expect": SAME}]})
    out.append(refused("sürüm yazısı geçersiz", s, {"raster": uid(1), "expectedRevision": "abc"}, "invalid_revision",
                       "Beklenen sürüm “abc” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden "
                       "ya da komutun planından alın.", "expectedRevision"))
    return out


def main():
    check = "--check" in sys.argv[1:]
    files = {n: geotiff(rows, nd) for n, (rows, nd) in IMAGES.items()}
    docs = {
        NET_OUT: {"format": "kentos.command-cases", "version": 1, "command": "cad.orthophoto.network", "commandVersion": 1,
                  "title": "Birleşme çizgisi ağı",
                  "note": "ADR 5260. Bağımsız başvuru scripts/fixtures/orthophoto_network_cases.py: elemanlar shapely'nin "
                          "farkıyla, düğümler merkezlerin çevrel merkeziyle denetlenir; KentOS kodu yok.",
                  "setup": doc([A, B, C]), "cases": net_cases()},
        FOOT_OUT: {"format": "kentos.command-cases", "version": 1, "command": "cad.orthophoto.footprint", "commandVersion": 1,
                   "title": "Ayak izi",
                   "note": "ADR 5260. Bağımsız başvuru scripts/fixtures/orthophoto_network_cases.py: görüntüler bayt bayt "
                           "yazılır, ayak izi geçerli piksellerin karelerinin shapely birleşimiyle; KentOS kodu yok.",
                   "setup": doc([raster_ent(0, 0, 14, 10, pixel=0.5, file="ayak-izi-a.tif")], name="Ayak izi"),
                   "cases": foot_cases()},
    }
    texts = {p: json.dumps(d, ensure_ascii=False, indent=1) + "\n" for p, d in docs.items()}
    if check:
        good = all(p.exists() and p.read_text("utf-8") == t for p, t in texts.items()) and all(
            (FILES / n).exists() and (FILES / n).read_bytes() == b for n, b in files.items())
        print("Ağ ve ayak izi ortak durumları güncel." if good else "Ağ ya da ayak izi ortak durumları güncel değil.")
        return 0 if good else 1
    FILES.mkdir(parents=True, exist_ok=True)
    for n, b in files.items():
        (FILES / n).write_bytes(b)
    for p, t in texts.items():
        p.write_text(t, "utf-8")
    print(f"yazıldı: {NET_OUT.name} ({len(docs[NET_OUT]['cases'])} durum), {FOOT_OUT.name} ({len(docs[FOOT_OUT]['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
