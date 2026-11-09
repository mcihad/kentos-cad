#!/usr/bin/env python3
"""Nokta bulutu araçlarının İşlemler durumları (docs/adr/0207 §7, 0652; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/pointcloud_processing_cases.py           # çizimi ve durumları yazar
    python3 scripts/fixtures/pointcloud_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Bulutlar bağımsız başvurunun dosyalarıdır (köyün LiDAR'ı fixtures/interaction/v1/pointclouds/koy.laz;
fixtures/pointcloud/v1/ops/sinir.laz ve ikinci.laz); her durumun sonucu başvurunun işlevleriyle (scripts/fixtures/
pointcloud_ops_cases.py: Seyrelt, SMRF, yüksekliğe göre sınıf, alanlar, Rasterleştir, Sınır çıkar, Birleştir) laspy ve numpy'la
yeniden hesaplanır; Dönüşüm uygula'nın (docs/adr/3100) taşınmış noktaları burada, matrisin satırları ve ızgaraya yarım
sıfırdan uzağa yuvarlamayla. Yazılan bulut dosyasının denetimi (`cloudOf`): nokta sayısı ve noktaların X, Y, Z tam sayılarının FNV-1a
64'ü dosyanın sırasıyla (COPC'de sıralanmış), sınıfların ya da kayıtların özeti; raster (`cloudRasterOf`) hücrelerinin float32
bitlerinin özeti. Sayılar özetlerin kuralıyla (binler noktayla).
"""

import json
import math
import struct
import sys
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import pointcloud_ops_cases as ref  # noqa: E402  (laspy with LASzip; runs itself with .run/pyref when it must)

import numpy as np  # noqa: E402

DIR = ROOT / "fixtures/processing/v1"
DRAWING = DIR / "pointcloud.kcad"
CASES = DIR / "pointcloud.json"
SRID = 5256
X0, Y0 = ref.X0, ref.Y0
FILES = {
    "koy.laz": "../../interaction/v1/pointclouds/koy.laz",
    "ikinci.laz": "../../pointcloud/v1/ops/ikinci.laz",
    "sinir.laz": "../../pointcloud/v1/ops/sinir.laz",
}


def words(n):
    s = str(n)
    out = []
    for i, ch in enumerate(s):
        if i > 0 and (len(s) - i) % 3 == 0:
            out.append(".")
        out.append(ch)
    return "".join(out)


def js(v):
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def layer_id(name):
    import re
    import unicodedata
    folded = name.translate(str.maketrans("çğıöşüÇĞİÖŞÜ", "cgiosuCGIOSU"))
    folded = unicodedata.normalize("NFKD", folded).encode("ascii", "ignore").decode()
    return "islem-" + re.sub(r"[^a-z0-9]+", "-", folded.lower())


# ── The clouds ───────────────────────────────────────────────────────────

def cloud(name):
    return ref.Cloud((DIR / FILES[name]).read_bytes())


KOY, IKINCI, SINIR = cloud("koy.laz"), cloud("ikinci.laz"), cloud("sinir.laz")
KOY_PLAN = [KOY.mins[0], KOY.mins[1], KOY.maxs[0], KOY.maxs[1]]


def bounds_of(c, idx):
    if len(idx) == 0:
        return None
    return [float(c.x[idx].min()), float(c.y[idx].min()), float(c.z[idx].min()),
            float(c.x[idx].max()), float(c.y[idx].max()), float(c.z[idx].max())]


def points_hash(c, idx, sort=False):
    triples = [(int(c.X[i]), int(c.Y[i]), int(c.Z[i])) for i in idx]
    if sort:
        triples.sort()
    return ref.fnv(b"".join(struct.pack("<iii", *t) for t in triples))


def header_bounds(c):
    return [*c.mins, *c.maxs]


# ── The drawing ──────────────────────────────────────────────────────────

KOY_STYLE = {"render": "rgb", "min": 842.0, "max": 858.0, "size": 3}
GREY_STYLE = {"render": "elevation", "size": 2}


def cloud_entity(file, c, srid, style):
    b = header_bounds(c)
    return {"kind": "pointcloud", "sources": [{"file": file, "format": "laz", "count": c.n, "bounds": b}],
            "bounds": b, "count": c.n, "srid": srid, "style": style}


def ring(pts):
    return [{"x": X0 + x, "y": Y0 + y} for x, y in pts]


def rect(r):
    return [(r[0], r[1]), (r[2], r[1]), (r[2], r[3]), (r[0], r[3])]


AREAS = [
    ({"kind": "polygon", "pts": ring(rect(ref.RECT))}, "Dikdörtgen"),
    ({"kind": "circle", "c": {"x": X0 + ref.DISC[0], "y": Y0 + ref.DISC[1]}, "r": ref.DISC[2]}, "Daire"),
    ({"kind": "polygon", "pts": ring(rect(ref.HOLED[0])), "holes": [{"pts": ring(rect(ref.HOLED[1]))}]}, "Delikli"),
]

LAYERS = [
    ("alanlar", "Alanlar", "#E8B04A"),
    ("koy", "Köy (LiDAR)", "#5B8FCF"),
    ("ikinci", "İkinci bulut", "#5B8FCF"),
    ("sinir", "Sınır bulutu", "#5B8FCF"),
    # docs/adr/3900: koy.laz under a history of one step and of two (the matrices as if applied before).
    ("gecmis1", "Bir adımlı geçmiş", "#5B8FCF"),
    ("gecmis2", "İki adımlı geçmiş", "#5B8FCF"),
]
# Entity ids in the drawing's order.
IDS = {"Dikdörtgen": 1, "Daire": 2, "Delikli": 3, "koy": 4, "ikinci": 5, "sinir": 6, "gecmis1": 7, "gecmis2": 8}


def drawing():
    def layer(id_, name, color):
        return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []
    for e, name in AREAS:
        entities.append({**e, "id": len(entities) + 1, "layerId": "alanlar", "attrs": {"Ad": name}})
    entities.append({**cloud_entity("koy.laz", KOY, SRID, KOY_STYLE), "id": 4, "layerId": "koy", "attrs": {}})
    entities.append({**cloud_entity("ikinci.laz", IKINCI, SRID, GREY_STYLE), "id": 5, "layerId": "ikinci", "attrs": {}})
    # Sınır çıkar's small cloud is in a local frame: no system.
    entities.append({**cloud_entity("sinir.laz", SINIR, 0, GREY_STYLE), "id": 6, "layerId": "sinir", "attrs": {}})
    # docs/adr/3900: Son dönüşümü geri al's clouds: koy.laz with a history written on it.
    entities.append({**cloud_entity("koy.laz", KOY, SRID, KOY_STYLE), "id": 7, "layerId": "gecmis1",
                     "attrs": {"Kaynak": "koy.laz", "Dönüşümler": matrix_words(ROTATED)}})
    entities.append({**cloud_entity("koy.laz", KOY, SRID, KOY_STYLE), "id": 8, "layerId": "gecmis2",
                     "attrs": {"Kaynak": "koy.laz",
                               "Dönüşümler": f"{matrix_words(ROTATED)} | {matrix_words(MOVED_ONLY)}"}})
    return {
        "format": "kentos.document",
        "version": 1,
        "name": "İşlem durumları: nokta bulutu",
        "settings": {"srid": SRID, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 500.0, "workspace": "gis"},
        "origin": {"x": X0 + 60.0, "y": Y0 + 45.0},
        "layers": [layer(*l) for l in LAYERS],
        "activeLayer": "alanlar",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }


# ── The cases ────────────────────────────────────────────────────────────

def on(lid):
    return {"scope": "layer", "layerId": lid}


def new_layer(name, color):
    return {"id": layer_id(name), "name": name, "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}}


def result_cloud(layer, path, fmt, c, idx, srid, style):
    b = bounds_of(c, idx)
    return {"kind": "pointcloud", "layerId": layer, "attrs": {},
            "sources": [{"file": path, "format": fmt, "count": len(idx), "bounds": b}],
            "bounds": b, "count": len(idx), "srid": srid, "style": style}


FORMAT = {"laz": ("laz", ".laz"), "las": ("las", ".las"), "copc": ("copc", ".copc.laz")}


def case(id_, title, tool, values, expect, selection=()):
    return {"id": id_, "title": title, "document": "pointcloud.kcad", "selection": list(selection), "run": {"tool": tool},
            "values": values, "expect": expect}


def written_cloud(id_, title, tool, values, suffix, layer_name, color, idx, summary_of, outputs, undo, *, fmt="laz",
                  output="", style=KOY_STYLE, classes=None, add=True):
    """A run that writes one cloud of koy's points `idx`: its file, its object when added, its check."""
    kind, ext = FORMAT[fmt]
    path = (output + ext) if output else f"koy{suffix}{ext}"
    values = {"input": on("koy"), **values}
    if fmt != "laz":
        values["format"] = fmt
    if output:
        values["output"] = output
    if not add:
        values["add"] = False
    check = {"count": len(idx), "points": points_hash(KOY, idx, sort=fmt == "copc")}
    if fmt == "copc":
        check["copc"] = True
    if classes is not None:
        check["classes"] = classes
    expect = {"status": "ok", "summary": summary_of(path), "outputs": outputs, "cloudOf": {path: check}}
    if add:
        layer = new_layer(layer_name, color)
        values["layer"] = {"newName": layer_name}
        expect.update({"undo": undo, "layers": [layer],
                       "added": [result_cloud(layer["id"], path, kind, KOY, idx, SRID, style)]})
    else:
        expect["undo"] = None
    return case(id_, title, tool, values, expect)


def thin_cases():
    out = []
    total = words(KOY.n)
    for id_, title, method, extra, kept, fmt, output, add in [
        ("thin-cell", "Seyrelt, Hücreyle 1 m: her küpte merkezine en yakın nokta; LAZ, çizime eklenir",
         "cell", {"size": 1.0}, ref.thin_cell(KOY, 1.0), "laz", "", True),
        ("thin-radius-las", "Seyrelt, Yarıçapla 0,6 m: dosyanın sırasıyla, yakın olan atılır; LAS, çizime eklenmez",
         "radius", {"size": 0.6}, ref.thin_radius(KOY, 0.6), "las", "", False),
        ("thin-nth-copc", "Seyrelt, Her 7'nci: ilk nokta ve her yedinci; COPC adı verilen dosyaya, dizini kurularak",
         "nth", {"n": 7}, [i for i in range(KOY.n) if i % 7 == 0], "copc", "seyrek", False),
    ]:
        values = {"method": method, **extra} if method != "cell" else dict(extra)
        n = len(kept)
        out.append(written_cloud(
            id_, title, "pointcloud.thin", values, "-seyrek", "Seyreltilmiş bulut", "#4C9A6A", kept,
            lambda p, n=n: f"{total} noktadan {words(n)} nokta kaldı; “{p}” yazıldı.", {"count": n}, "Seyrelt",
            fmt=fmt, output=output, add=add))
    return out


def classes_hash(cls, sort=False):
    """The classes' FNV-1a; a COPC's (its index orders the points) in the order of the points' X, Y, Z and class."""
    if sort:
        keyed = sorted((int(KOY.X[i]), int(KOY.Y[i]), int(KOY.Z[i]), int(v)) for i, v in enumerate(cls))
        return ref.fnv(bytes(k[3] for k in keyed))
    return ref.fnv(bytes(int(v) for v in cls))


def ground_cases():
    out = []
    every = list(range(KOY.n))
    style = {**KOY_STYLE, "render": "classification"}
    for id_, title, last, fmt, add in [
        ("ground", "Zemin süzgeci (SMRF), yalnız son dönüşler: zemin 2'ye, zemin olmayan 2'ler 1'e", True, "laz", True),
        ("ground-all-returns", "Zemin süzgeci, bütün dönüşler yüzeyi kurar; LAS, çizime eklenmez", False, "las", False),
        ("ground-copc", "Zemin süzgeci, COPC: dizini kurulur, sınıflar noktalarıyla (docs/adr/0741)", True, "copc", False),
    ]:
        n, cls = ref.smrf(KOY, KOY_PLAN, last_only=last)
        values = {} if last else {"lastOnly": False}
        out.append(written_cloud(
            id_, title, "pointcloud.ground", values, "-zemin", "Zemini ayrılmış bulut", "#8A6A3E", every,
            lambda p, n=n: f"{words(KOY.n)} noktadan {words(n)} nokta zemin (2) sınıfında; “{p}” yazıldı.",
            {"ground": n}, "Zemin süzgeci", fmt=fmt, style=style, classes=classes_hash(cls, fmt == "copc"), add=add))
    return out


def classify_cases():
    out = []
    every = list(range(KOY.n))
    style = {**KOY_STYLE, "render": "classification"}
    for id_, title, all_, fmt, add in [
        ("classify-all", "Yüksekliğe göre sınıfla, zemin olmayan bütün noktalar: zeminden yüksekliğe göre 3, 4, 5",
         True, "laz", False),
        ("classify-unclassed", "Yüksekliğe göre sınıfla, yalnız sınıfsızlar (0, 1); LAS, çizime eklenir", False, "las", True),
        ("classify-copc", "Yüksekliğe göre sınıfla, COPC (docs/adr/0741)", False, "copc", False),
    ]:
        n, cls = ref.hag(KOY, KOY_PLAN, every=all_)
        values = {"all": True} if all_ else {}
        s = sum(n)
        out.append(written_cloud(
            id_, title, "pointcloud.classify", values, "-sinifli", "Sınıflanmış bulut", "#3E8A4C", every,
            lambda p, n=n, s=s: (f"{words(s)} nokta sınıflandı: düşük bitki {words(n[0])}, orta bitki {words(n[1])}, "
                                 f"yüksek bitki {words(n[2])}; “{p}” yazıldı."),
            {"classed": s}, "Yüksekliğe göre sınıfla", fmt=fmt, style=style, classes=classes_hash(cls, fmt == "copc"), add=add))
    return out


def clip_cases():
    inside = (ref.in_rect(KOY.x, KOY.y, ref.RECT) | ref.in_disc(KOY.x, KOY.y, ref.DISC)
              | ref.in_holed(KOY.x, KOY.y, ref.HOLED))
    ins = [int(i) for i in np.nonzero(inside)[0]]
    outs = [int(i) for i in np.nonzero(~inside)[0]]
    out = []
    for id_, title, side, idx, fmt, add in [
        ("clip-inside", "Bulutu kırp, içindekiler: dikdörtgen, daire ve delikli alan birlikte; kenardaki nokta içeride",
         "inside", ins, "laz", True),
        ("clip-outside", "Bulutu kırp, dışındakiler; LAS, çizime eklenmez", "outside", outs, "las", False),
        ("clip-copc", "Bulutu kırp, içindekiler COPC olarak", "inside", ins, "copc", False),
    ]:
        values = {"areas": on("alanlar")}
        if side != "inside":
            values["side"] = side
        where = "alanların içinde" if side == "inside" else "alanların dışında"
        n = len(idx)
        out.append(written_cloud(
            id_, title, "pointcloud.clip", values, "-kirpik", "Kırpılmış bulut", "#4C7A9A", idx,
            lambda p, n=n, where=where: f"{words(KOY.n)} noktadan {words(n)} nokta {where}; “{p}” yazıldı.",
            {"count": n}, "Bulutu kırp", fmt=fmt, add=add))
    return out


def fixed(v, d):
    return "" if v is None else format(v, f".{d}f").replace(".", ",")


def area_stats_case():
    masks = [ref.in_rect(KOY.x, KOY.y, ref.RECT), ref.in_disc(KOY.x, KOY.y, ref.DISC),
             ref.in_holed(KOY.x, KOY.y, ref.HOLED)]
    sizes = [(ref.RECT[2] - ref.RECT[0]) * (ref.RECT[3] - ref.RECT[1]), math.pi * ref.DISC[2] ** 2,
             (ref.HOLED[0][2] - ref.HOLED[0][0]) * (ref.HOLED[0][3] - ref.HOLED[0][1])
             - (ref.HOLED[1][2] - ref.HOLED[1][0]) * (ref.HOLED[1][3] - ref.HOLED[1][1])]
    rows = []
    total = 0
    for (_, name), mask, size in zip(AREAS, masks, sizes):
        f = ref.figures(KOY, mask)
        total += f["count"]
        rows.append([f"#{IDS[name]}", name, str(f["count"]), fixed(size, 2), fixed(f["count"] / size, 2),
                     fixed(f.get("zMin"), 3), fixed(f.get("zMax"), 3), fixed(f.get("zMean"), 3), fixed(f.get("zStd"), 3),
                     "; ".join(f"{c}: {n}" for c, n in f.get("classes", []))])
    columns = ["Nesne", "Ad", "Nokta", "Alan (m²)", "Yoğunluk (nokta/m²)", "Z en küçük", "Z en büyük", "Z ortalama",
               "Z std. sapma", "Sınıflar"]
    return case("area-stats", "Nokta bulutu alan sorgusu: üç alanın sayısı, yoğunluğu, kotları ve sınıfları",
                "pointcloud.areaStats", {"input": on("koy"), "areas": on("alanlar")},
                {"status": "ok", "undo": None, "summary": f"3 alanın içinde {words(total)} nokta sayıldı.",
                 "outputs": {"table": {"columns": columns, "rows": rows}}})


def tile_cases():
    size = 50.0
    tiles = {}
    for i in range(KOY.n):
        t = (math.floor(KOY.x[i] / size), math.floor(KOY.y[i] / size))
        tiles.setdefault(t, []).append(i)
    out = []
    for id_, title, fmt, vpc, add in [
        ("tile", "Karola, 50 m: altı karo köşelerinin adlarıyla, sanal bulutu yanında; çizime sanal bulut olarak", "laz", True, True),
        ("tile-las", "Karola, LAS, sanal bulut yazılmaz", "las", False, False),
        ("tile-copc", "Karola, COPC istendi: karolar LAZ yazılır ve söylenir", "copc", False, False),
    ]:
        ext = ".las" if fmt == "las" else ".laz"
        kind = "las" if fmt == "las" else "laz"
        checks, sources = {}, []
        for t in sorted(tiles):
            idx = tiles[t]
            name = f"koy_{js(t[0] * size)}_{js(t[1] * size)}{ext}"
            checks[name] = {"count": len(idx), "points": points_hash(KOY, idx)}
            sources.append({"file": name, "format": kind, "count": len(idx), "bounds": bounds_of(KOY, np.asarray(idx))})
        written = sorted(checks) + (["koy.vpc"] if vpc else [])
        values = {"input": on("koy"), "size": size}
        if fmt != "laz":
            values["format"] = fmt
        if not vpc:
            values["vpc"] = False
        summary = (f"{words(KOY.n)} nokta {len(tiles)} karoya bölündü; “” klasörüne yazıldı"
                   + (", sanal bulutu “koy.vpc”" if vpc else "") + ".")
        expect = {"status": "ok", "summary": summary, "outputs": {"tiles": len(tiles)}, "cloudOf": checks,
                  "written": written}
        if fmt == "copc":
            expect["log"] = [{"level": "warn", "text": "Karolar LAZ olarak yazıldı; karoların dizini ilk gösterilişlerinde hazırlanır."}]
        if add:
            layer = new_layer("Karolar", "#9A7A4C")
            values["layer"] = {"newName": "Karolar"}
            b = header_bounds(KOY)
            expect.update({"undo": "Karola", "layers": [layer], "added": [
                {"kind": "pointcloud", "layerId": layer["id"], "attrs": {}, "sources": sources, "bounds": b,
                 "count": KOY.n, "srid": SRID, "style": KOY_STYLE}]})
        else:
            values["add"] = False
            expect["undo"] = None
        out.append(case(id_, title, "pointcloud.tile", values, expect))
    return out


def raster_of(c, cell, value, radius=None, keep=None):
    """Rasterleştir's grid over the cloud's plan, of the points `keep` holds (all when none): its check and its values'
    range (the object's style)."""
    src = c if keep is None else SimpleNamespace(n=int(keep.sum()), x=c.x[keep], y=c.y[keep], z=c.z[keep])
    values = [v for v in ref.rasterize_values(src, KOY_PLAN, cell, value, radius) if v != -9999.0]
    return ref.rasterize(src, KOY_PLAN, cell, value, radius), (min(values), max(values))


def rasterize_cases():
    out = []
    ground = KOY.cls == 2
    listed = (KOY.cls == 2) | (KOY.cls == 6)
    for id_, title, value, classes, keep, add in [
        ("rasterize-min", "Rasterleştir, en düşük, 2 m: hücreye düşen en düşük kot; çizime eklenir", "min", None, None, True),
        ("rasterize-max", "Rasterleştir, en yüksek", "max", None, None, False),
        ("rasterize-mean", "Rasterleştir, ortalama", "mean", None, None, False),
        ("rasterize-count", "Rasterleştir, nokta sayısı", "count", None, None, False),
        ("rasterize-idw", "Rasterleştir, IDW 2,5 m: hücre merkezine uzaklığın karesinin tersiyle", "idw", None, None, False),
        ("rasterize-ground", "Rasterleştir, yalnız zemin (2)", "min", "ground", ground, False),
        ("rasterize-list", "Rasterleştir, yazılan sınıflar “2, 6”", "max", "list", listed, False),
    ]:
        r, (lo, hi) = raster_of(KOY, 2.0, value, 2.5 if value == "idw" else None, keep)
        values = {"input": on("koy"), "cell": 2.0}
        if value != "min":
            values["value"] = value
        if value == "idw":
            values["radius"] = 2.5
        if classes:
            values["classes"] = classes
        if classes == "list":
            values["list"] = "2, 6"
        path = "koy-dem.tif"
        expect = {"status": "ok",
                  "summary": f"{r['cols']} × {r['rows']} hücrelik raster (hücre 2 m), {words(r['filled'])} dolu hücre; “{path}” yazıldı.",
                  "outputs": {"cells": r["filled"]}, "cloudRasterOf": {path: r}}
        if add:
            layer = new_layer("Yükseklik rasteri", "#7A6B5B")
            values["layer"] = {"newName": "Yükseklik rasteri"}
            x0, top, cols, rows = ref.frame(KOY_PLAN, 2.0)
            style = {"render": "rampShade", "bands": [1], "stretch": "minMax", "min": lo, "max": hi if hi > lo else lo + 1.0,
                     "ramp": "Arazi", "nodata": -9999.0}
            expect.update({"undo": "Rasterleştir", "layers": [layer],
                           "added": [{"kind": "raster", "layerId": layer["id"], "attrs": {}, "affine": [x0, 2.0, 0.0, top, 0.0, -2.0],
                                      "width": cols, "height": rows, "bands": 1, "sample": "f32", "file": path, "srid": SRID,
                                      "style": style}]})
        else:
            values["add"] = False
            expect["undo"] = None
        out.append(case(id_, title, "pointcloud.rasterize", values, expect))
    return out


def boundary_case():
    plan = [SINIR.mins[0], SINIR.mins[1], SINIR.maxs[0], SINIR.maxs[1]]
    parts = ref.boundary(SINIR, plan, 1.0, 2)
    f = ref.frame(plan, 1.0)
    counts = {}
    for i in range(SINIR.n):
        k = ref.frame_cell(f, 1.0, float(SINIR.x[i]), float(SINIR.y[i]))
        counts[k] = counts.get(k, 0) + 1
    cells = sum(1 for v in counts.values() if v >= 2)

    def pts(r):
        return [{"x": p[0], "y": p[1]} for p in r]

    def holes(p):
        return [{"pts": pts(h)} for h in p["holes"]]

    first = {"kind": "polygon", "pts": pts(parts[0]["outer"])}
    if parts[0]["holes"]:
        first["holes"] = holes(parts[0])
    rest = []
    for p in parts[1:]:
        part = {"pts": pts(p["outer"])}
        if p["holes"]:
            part["holes"] = holes(p)
        rest.append(part)
    if rest:
        first["parts"] = rest
    layer = new_layer("Bulut sınırı", "#5B8FCF")
    # An area layer: its fill the line's colour at 15 %.
    layer["style"] = {"color": "#5B8FCF", "fill": "#5B8FCF26", "lineType": "continuous", "lineWeight": 0.25}
    return case("boundary", "Sınır çıkar, 1 m, en az 2 nokta: delikli iki parça ve köşeden değen ayrı parça; tek çok parçalı alan",
                "pointcloud.boundary", {"input": on("sinir"), "cell": 1.0, "least": 2, "layer": {"newName": "Bulut sınırı"}},
                {"status": "ok", "undo": "Sınır çıkar",
                 "summary": f"{words(cells)} hücreden {len(parts)} parçalı sınır çizildi (1 m'lik hücreler).",
                 "outputs": {"parts": len(parts)}, "layers": [layer],
                 "added": [{**first, "layerId": layer["id"], "attrs": {"Hücre": "1", "En az nokta": "2"}}]})


def merge_case(fmt="laz"):
    m = ref.merge(KOY, IKINCI)
    path = "koy-birlesik" + FORMAT[fmt][1]
    layer = new_layer("Birleşik bulut", "#7A5C9A")
    values = {"input": {"scope": "selection"}, "add": False}
    check = {"count": m["count"], "records": m["records"]}
    if fmt != "laz":
        values["format"] = fmt
    if fmt == "copc":
        check = {"count": m["count"], "sortedRecords": m["sorted"], "copc": True}
    title = {"laz": "Bulutları birleştir, iki dosya: biçim 7'ye genişletilir, ince ızgaraya taşınır, yarım sıfırdan uzağa",
             "las": "Bulutları birleştir, LAS (docs/adr/0741)", "copc": "Bulutları birleştir, COPC: kayıtlar dizinin sırasıyla"}[fmt]
    return case("merge" if fmt == "laz" else f"merge-{fmt}", title,
                "pointcloud.merge", values,
                {"status": "ok", "undo": None,
                 "summary": f"2 dosyanın {words(m['count'])} noktası “{path}” dosyasında birleşti.",
                 "outputs": {"count": m["count"]},
                 "log": [{"level": "warn", "text": f"{words(m['rounded'])} noktanın koordinatı sonucun ölçeğine yuvarlandı "
                                                   "(kaynakların ölçek ve ötelemeleri farklı)."}],
                 "cloudOf": {path: check}},
                selection=[IDS["koy"], IDS["ikinci"]])


# ── Dönüşüm uygula (docs/adr/3100) ───────────────────────────────────────

def rz(k):
    c, s = math.cos(k), math.sin(k)
    return [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]


def ry(f):
    c, s = math.cos(f), math.sin(f)
    return [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]


def rx(w):
    c, s = math.cos(w), math.sin(w)
    return [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]


def mul(a, b):
    return [[a[i][0] * b[0][j] + a[i][1] * b[1][j] + a[i][2] * b[2][j] for j in range(3)] for i in range(3)]


def middle(c):
    return [(c.mins[k] + c.maxs[k]) / 2 for k in range(3)]


def about(r, c, t):
    """Tarama kaydı's recipe p ↦ R(p − c) + c + t as one matrix in the project's coordinates, row-major."""
    m = []
    for i in range(3):
        rc = r[i][0] * c[0] + r[i][1] * c[1] + r[i][2] * c[2]
        m += [r[i][0], r[i][1], r[i][2], c[i] + t[i] - rc]
    return m + [0.0, 0.0, 0.0, 1.0]


def axis_angle(axis, degrees, center, translation):
    """docs/adr/4300: Eksen ve açı's map in the core's fixed order: the axis over its length √(x·x + y·y + z·z); the
    angle in radians (degrees × π/180); with s, c its sine and cosine and k = 1 − c, Rodrigues' rows; the translation's
    row c_i + t_i − (R_i0·c0 + R_i1·c1 + R_i2·c2) (Tarama kaydı's recipe p ↦ R(p − c) + c + t)."""
    n = math.sqrt(axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2])
    x, y, z = axis[0] / n, axis[1] / n, axis[2] / n
    r = math.radians(degrees)
    s, c = math.sin(r), math.cos(r)
    k = 1.0 - c
    rot = [[k * x * x + c, k * x * y - s * z, k * x * z + s * y],
           [k * x * y + s * z, k * y * y + c, k * y * z - s * x],
           [k * x * z - s * y, k * y * z + s * x, k * z * z + c]]
    m = []
    for i in range(3):
        rc = rot[i][0] * center[0] + rot[i][1] * center[1] + rot[i][2] * center[2]
        m += [rot[i][0], rot[i][1], rot[i][2], center[i] + translation[i] - rc]
    return m + [0.0, 0.0, 0.0, 1.0]


def matrix_text(m):
    """Row by row, the numbers as Python writes them (they read back the same)."""
    return "\n".join(" ".join(repr(float(v)) for v in m[i * 4:i * 4 + 4]) for i in range(4))


def inverse(m):
    """docs/adr/3500: the inverse map in the core's fixed order: the cofactors over the determinant, row by row; the
    translation the negated rows of that inverse times the translation, added in order; the bottom row 0 0 0 1."""
    d = (m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
         + m[2] * (m[4] * m[9] - m[5] * m[8]))
    i = [(m[5] * m[10] - m[6] * m[9]) / d, (m[2] * m[9] - m[1] * m[10]) / d, (m[1] * m[6] - m[2] * m[5]) / d,
         (m[6] * m[8] - m[4] * m[10]) / d, (m[0] * m[10] - m[2] * m[8]) / d, (m[2] * m[4] - m[0] * m[6]) / d,
         (m[4] * m[9] - m[5] * m[8]) / d, (m[1] * m[8] - m[0] * m[9]) / d, (m[0] * m[5] - m[1] * m[4]) / d]
    t = [-(i[3 * r] * m[3] + i[3 * r + 1] * m[7] + i[3 * r + 2] * m[11]) for r in range(3)]
    return [i[0], i[1], i[2], t[0], i[3], i[4], i[5], t[1], i[6], i[7], i[8], t[2], 0.0, 0.0, 0.0, 1.0]


def rust_number(v):
    """An f64 as Rust writes it: the shortest digits that read back (Python's repr gives the same), never an exponent,
    no “.0” on a whole number; a negative zero written as 0 (the history adds 0.0)."""
    from decimal import Decimal
    v = v + 0.0
    text = format(Decimal(repr(v)), "f")
    if "." in text:
        text = text.rstrip("0").rstrip(".")
    return "0" if text in ("-0", "") else text


def matrix_words(m):
    """docs/adr/3500: the 16 numbers of the history, row by row, apart by spaces."""
    return " ".join(rust_number(v) for v in m)


def apply(m, p):
    return [m[0] * p[0] + m[1] * p[1] + m[2] * p[2] + m[3],
            m[4] * p[0] + m[5] * p[1] + m[6] * p[2] + m[7],
            m[8] * p[0] + m[9] * p[1] + m[10] * p[2] + m[11]]


def round_away(n):
    """The half away from zero, exactly (Rust's f64::round): n − trunc(n) is exact."""
    t = np.trunc(n)
    return t + np.where(np.abs(n - t) >= 0.5, np.copysign(1.0, n), 0.0)


def moved(c, m):
    """The cloud's points moved by `m`: their whole numbers on the cloud's own grid, rounded half away from zero."""
    q = apply(m, [c.x, c.y, c.z])
    ints = []
    for k in range(3):
        n = round_away((q[k] - c.offset[k]) / c.scale[k])
        assert n.min() >= -2 ** 31 and n.max() <= 2 ** 31 - 1
        ints.append(n.astype(np.int64))
    return ints


def rigid(m):
    cols = [[m[j], m[4 + j], m[8 + j]] for j in range(3)]
    for i in range(3):
        for j in range(3):
            dot = sum(cols[i][k] * cols[j][k] for k in range(3))
            if abs(dot - (1.0 if i == j else 0.0)) > 1e-9:
                return False
    det = (m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
           + m[2] * (m[4] * m[9] - m[5] * m[8]))
    return det > 0


def angle(m):
    ax, ay, az = m[9] - m[6], m[2] - m[8], m[4] - m[1]
    return math.atan2(math.sqrt(ax * ax + ay * ay + az * az), m[0] + m[5] + m[10] - 1.0) * (180.0 / math.pi)


def shift(m, c):
    q = apply(m, c)
    dx, dy, dz = q[0] - c[0], q[1] - c[1], q[2] - c[2]
    return math.sqrt(dx * dx + dy * dy + dz * dz)


def half_scales(c):
    def t(v):
        w = js(v)
        assert "e" not in w, w
        return w.replace(".", ",")
    h = [s / 2 for s in c.scale]
    return f"{t(h[0])} m" if h[0] == h[1] == h[2] else f"X {t(h[0])}, Y {t(h[1])}, Z {t(h[2])} m"


ROTATED = about(rz(0.002), middle(KOY), [0.35, -0.2, 0.05])
REGISTERED = about(mul(rz(0.0021), mul(ry(-0.0003), rx(0.0004))), middle(KOY), [-0.12, 0.08, 0.031])
MOVED_ONLY = [1.0, 0.0, 0.0, 12.5, 0.0, 1.0, 0.0, -3.25, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0]
SCALED = about([[1.0001, 0.0, 0.0], [0.0, 1.0001, 0.0], [0.0, 0.0, 1.0001]], middle(KOY), [0.0, 0.0, 0.0])
IKINCI_TURNED = about(mul(rz(-0.0015), rx(0.0002)), middle(IKINCI), [0.05, 0.02, -0.01])


def transformed_case(id_, title, c, lid, file, m, *, fmt="laz", output="", add=True, style=KOY_STYLE, srid=SRID,
                     text=None, invert=False, undo=None, axis=None):
    """`undo` (docs/adr/3900): Son dönüşümü geri al on a cloud whose history ends with `m`; the history left."""
    kind, ext = FORMAT[fmt]
    path = (output + ext) if output else f"{file}-donusmus{ext}"
    given = m
    if invert or undo is not None:
        m = inverse(m)
    X, Y, Z = moved(c, m)
    triples = list(zip(X.tolist(), Y.tolist(), Z.tolist()))
    if fmt == "copc":
        triples.sort()
    check = {"count": c.n, "points": ref.fnv(b"".join(struct.pack("<iii", *t) for t in triples))}
    if fmt == "copc":
        check["copc"] = True
    values = {"input": on(lid), "matrix": text if text is not None else matrix_text(given)}
    if axis is not None:
        # docs/adr/4300: the map as an axis and an angle; `m` is what it makes.
        values = {"input": on(lid), "form": "axisAngle", **axis}
    if invert:
        values["invert"] = True
    if undo is not None:
        values = {"input": on(lid), "undoLast": True}
    if fmt != "laz":
        values["format"] = fmt
    if output:
        values["output"] = output
    if not add:
        values["add"] = False
    turned = rigid(m)
    s = shift(m, middle(c))
    outputs = {"count": c.n, "shift": s}
    tail = ""
    if turned:
        a = angle(m)
        outputs["angle"] = a
        tail = f", dönüklük {fixed(a, 4)}°"
    log = []
    if not turned:
        log.append({"level": "warn", "text": "Matris katı bir dönüşüm değil (ölçek ya da kayma içeriyor): uzunluklar ve açılar değişir."})
    log.append({"level": "info", "text": f"Taşınan noktalar kaynağın ızgarasına yuvarlandı: bir koordinat en çok {half_scales(c)} değişir."})
    expect = {"status": "ok", "summary": f"{words(c.n)} nokta taşındı; bulutun ortası {fixed(s, 3)} m kaydı{tail}; “{path}” yazıldı.",
              "outputs": outputs, "log": log, "cloudOf": {path: check}}
    if add:
        layer = new_layer("Dönüştürülmüş bulut", "#7A5BA6")
        values["layer"] = {"newName": "Dönüştürülmüş bulut"}
        nx, ny, nz = (X * c.scale[0] + c.offset[0], Y * c.scale[1] + c.offset[1], Z * c.scale[2] + c.offset[2])
        b = [float(nx.min()), float(ny.min()), float(nz.min()), float(nx.max()), float(ny.max()), float(nz.max())]
        # docs/adr/3500: the history: the source's file and the matrix applied (the inverse when asked).
        history = {"Kaynak": f"{file}.laz", "Dönüşümler": matrix_words(m)}
        if undo is not None:
            history = {"Kaynak": f"{file}.laz", **({"Dönüşümler": undo} if undo else {})}
        expect.update({"undo": "Dönüşüm uygula", "layers": [layer], "added": [
            {"kind": "pointcloud", "layerId": layer["id"], "attrs": history,
             "sources": [{"file": path, "format": kind, "count": c.n, "bounds": b}],
             "bounds": b, "count": c.n, "srid": srid, "style": style}]})
    else:
        expect["undo"] = None
    return case(id_, title, "pointcloud.transform", values, expect)


def transform_cases():
    def refused(id_, title, text, message):
        return case(id_, title, "pointcloud.transform", {"input": on("koy"), "matrix": text},
                    {"status": "error", "message": message})

    far = list(MOVED_ONLY)
    far[3], far[7] = 0.0, 3.0e6
    return [
        transformed_case("transform-rotated", "Dönüşüm uygula, ortası çevresinde 0,002 rad dönen ve 0,35 m, −0,2 m, 5 cm "
                         "ötelenen matris: noktalar ızgaraya yarım sıfırdan uzağa; LAZ, çizime eklenir",
                         KOY, "koy", "koy", ROTATED),
        transformed_case("transform-registered", "Dönüşüm uygula, Tarama kaydı'nın tarifi Rz(κ)·Ry(φ)·Rx(ω) ortası "
                         "çevresinde, öteleme ile; çizime eklenmez", KOY, "koy", "koy", REGISTERED, add=False),
        transformed_case("transform-translation-las", "Dönüşüm uygula, yalnız öteleme (12,5 m, −3,25 m): ızgaranın katı, "
                         "tam sayılar aynen kayar; LAS, virgüllü ve köşeli yazım", KOY, "koy", "koy", MOVED_ONLY,
                         fmt="las", add=False, text="[1, 0, 0, 12.5; 0, 1, 0, -3.25; 0, 0, 1, 0; 0, 0, 0, 1]"),
        transformed_case("transform-copc", "Dönüşüm uygula, COPC adı verilen dosyaya, dizini kurularak", KOY, "koy",
                         "koy", ROTATED, fmt="copc", output="kayitli", add=False),
        transformed_case("transform-scaled", "Dönüşüm uygula, ortası çevresinde 1,0001 ölçek: katı değil, uyarılır; "
                         "dönüklük söylenmez", KOY, "koy", "koy", SCALED, add=False),
        transformed_case("transform-second-grid", "Dönüşüm uygula, X ve Y'si 0,4 mm'lik ızgaradaki ikinci bulut: kendi "
                         "ızgarasında kalır, yuvarlama eksen eksen söylenir", IKINCI, "ikinci", "ikinci", IKINCI_TURNED,
                         style=GREY_STYLE),
        # docs/adr/3500: the inverse, in the core's fixed order.
        transformed_case("transform-inverse", "Tersini uygula: dönen ve ötelenen matrisin tersi; nesnenin geçmişinde "
                         "kaynak dosya ve uygulanan ters matris", KOY, "koy", "koy", ROTATED, invert=True),
        transformed_case("transform-inverse-registered", "Tersini uygula: Tarama kaydı'nın tarifinin tersi, kayıtlı "
                         "bulut eski yerine", KOY, "koy", "koy", REGISTERED, invert=True, fmt="las"),
        transformed_case("transform-inverse-translation", "Tersini uygula: yalnız ötelemenin tersi, tam sayılar tam "
                         "geri kayar", KOY, "koy", "koy", MOVED_ONLY, invert=True, fmt="las", output="geri"),
        # docs/adr/4300: Eksen ve açı: the core's matrix from an axis, an angle, a centre and a move.
        transformed_case("transform-axis-z", "Eksen ve açı: Z ekseni çevresinde 0,5°, merkez boş (bulutun kutusunun "
                         "ortası), öteleme 1 −2 0,5 m; geçmişte yapılan matris", KOY, "koy", "koy",
                         axis_angle([0.0, 0.0, 1.0], 0.5, middle(KOY), [1.0, -2.0, 0.5]),
                         axis={"axis": "0 0 1", "angle": 0.5, "translation": "1 -2 0.5"}),
        transformed_case("transform-axis-tilted", "Eksen ve açı: eğik eksen (1, 1, 0,5; uzunluğu önemsiz) çevresinde "
                         "−0,25°, verilen merkez, öteleme yok; çizime eklenmez", KOY, "koy", "koy",
                         axis_angle([2.0, 2.0, 1.0], -0.25, [487030.0, 4420040.0, 850.0], [0.0, 0.0, 0.0]),
                         axis={"axis": "2, 2, 1", "angle": -0.25, "center": "487030 4420040 850"}, add=False),
        transformed_case("transform-axis-inverse", "Eksen ve açı, Tersini uygula: Z çevresindeki dönmenin ve "
                         "ötelemenin tersi", KOY, "koy", "koy",
                         axis_angle([0.0, 0.0, 1.0], 0.5, middle(KOY), [1.0, -2.0, 0.5]),
                         axis={"axis": "0 0 1", "angle": 0.5, "translation": "1 -2 0.5"}, invert=True, fmt="las"),
        case("transform-axis-zero", "Eksen ve açı, uzunluğu sıfır eksen: ret", "pointcloud.transform",
             {"input": on("koy"), "form": "axisAngle", "axis": "0 0 0", "angle": 10},
             {"status": "error", "message": "Dönme ekseninin uzunluğu sıfır olamaz; ekseni üç sayıyla yazın (Z için 0 0 1)."}),
        case("transform-axis-two", "Eksen ve açı, iki sayılı eksen: ret", "pointcloud.transform",
             {"input": on("koy"), "form": "axisAngle", "axis": "0 1", "angle": 10},
             {"status": "error", "message": "Eksen 3 sayıdır (X Y Z, boşlukla ayrılmış); 2 sayı verildi."}),
        case("transform-axis-centre-word", "Eksen ve açı, sayı olmayan merkez: ret", "pointcloud.transform",
             {"input": on("koy"), "form": "axisAngle", "axis": "0 0 1", "angle": 10, "center": "a 0 0"},
             {"status": "error", "message": "Merkez: “a” bir sayı değil; ondalık için nokta kullanın."}),
        # docs/adr/3900: Son dönüşümü geri al: the last matrix's inverse, the step gone from the history.
        transformed_case("transform-undo-last", "Son dönüşümü geri al: tek adımlı geçmişin dönen matrisinin tersi; "
                         "geçmiş boşalır, kaynak kalır", KOY, "gecmis1", "koy", ROTATED, undo=""),
        transformed_case("transform-undo-second", "Son dönüşümü geri al: iki adımlı geçmişte yalnız son öteleme geri "
                         "alınır, ilk adım geçmişte kalır", KOY, "gecmis2", "koy", MOVED_ONLY, undo=matrix_words(ROTATED),
                         output="geri-alinmis"),
        case("transform-undo-none", "Son dönüşümü geri al: geçmişi olmayan bulutta ret", "pointcloud.transform",
             {"input": on("koy"), "undoLast": True},
             {"status": "error", "message": "Bulutun dönüşüm geçmişi (Dönüşümler) yok; geri alınacak adım yok."}),
        refused("transform-count", "Dönüşüm uygula, 12 sayı: ret", "1 0 0 0\n0 1 0 0\n0 0 1 0",
                "Matris 16 sayıdır (4 satır, 4 sütun, satır satır); 12 sayı verildi."),
        refused("transform-word", "Dönüşüm uygula, sayı olmayan söz: ret", "1 0 0 a\n0 1 0 0\n0 0 1 0\n0 0 0 1",
                "Matriste “a” bir sayı değil; sayıları boşlukla ayırın, ondalık için nokta kullanın."),
        refused("transform-perspective", "Dönüşüm uygula, son satırı 0 0 0 1 olmayan matris: ret",
                "1 0 0 0\n0 1 0 0\n0 0 1 0\n0 0 0.001 1",
                "Matrisin son satırı 0 0 0 1 olmalı: yalnız afin dönüşüm uygulanır (perspektif yok)."),
        refused("transform-singular", "Dönüşüm uygula, kotu ezen tekil matris: ret", "1 0 0 0\n0 1 0 0\n0 0 0 850\n0 0 0 1",
                "Matrisin dönüklük kısmı tekil (determinantı sıfır): noktaları bir düzleme ya da doğruya ezer."),
        refused("transform-off-grid", "Dönüşüm uygula, 3000 km kuzeye öteleme: ızgaranın 32 bitine sığmaz, hiçbir şey "
                "yazılmadan ret", matrix_text(far),
                "Dönüşen bulut dosyanın ızgarasına sığmıyor: Y ekseninde kayıtların 32 bitlik tam sayılarına sığmayacak "
                "kadar uzağa gidiyor. Matrisin ötelemesini denetleyin."),
    ]


def refusals():
    def refused(id_, title, tool, values, message, selection=()):
        return case(id_, title, tool, values, {"status": "error", "message": message}, selection)

    return [
        refused("thin-two-clouds", "Seyrelt, iki bulut seçili: ret", "pointcloud.thin", {"input": {"scope": "selection"}},
                "2 nokta bulutu seçildi; bu araç bir bulutu işler. Bir bulut seçin (birden çok dosyayı Bulutları birleştir tek "
                "dosyada toplar).", [IDS["koy"], IDS["ikinci"]]),
        refused("merge-one", "Bulutları birleştir, tek dosya: ret", "pointcloud.merge", {"input": on("koy")},
                "Birleştirmek için en az iki dosya gerekir: birden çok bulut ya da birden çok dosyalı bir sanal bulut seçin."),
        refused("merge-systems", "Bulutları birleştir, iki koordinat sistemi: ret", "pointcloud.merge",
                {"input": {"scope": "selection"}},
                "Bulutlar farklı koordinat sistemlerinde; KentOS bulutu projenin sistemine kendiliğinden çevirmez. Aynı sistemdeki bulutları seçin.",
                [IDS["koy"], IDS["sinir"]]),
    ]


def cases():
    return (thin_cases() + ground_cases() + classify_cases() + clip_cases() + [area_stats_case()] + tile_cases()
            + rasterize_cases() + [boundary_case(), merge_case(), merge_case("las"), merge_case("copc")] + transform_cases()
            + refusals())


def build():
    return {
        "format": "kentos.processing-cases",
        "version": 1,
        "tolerance": 1e-9,
        "toleranceFields": ["bounds", "sources"],
        "rasters": dict(FILES),
        "documents": {"pointcloud.kcad": {"defaults": {"lengthDecimals": 2, "areaDecimals": 2, "angleUnit": "grad",
                                                       "plotScale": 500.0, "activeLayer": "alanlar"}}},
        "cases": cases(),
    }


def main():
    check = "--check" in sys.argv[1:]
    texts = {
        DRAWING: json.dumps(drawing(), ensure_ascii=False, indent=1) + "\n",
        CASES: json.dumps(build(), ensure_ascii=False, indent=1) + "\n",
    }
    problems = []
    for path, text in texts.items():
        if check:
            if not path.exists() or path.read_text("utf-8") != text:
                problems.append(f"{path.relative_to(ROOT)} yeniden kurulanla aynı değil (betiği --check olmadan çalıştırın)")
        else:
            path.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print(f"{CASES.relative_to(ROOT)}: {len(build()['cases'])} durum; çizim {DRAWING.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
