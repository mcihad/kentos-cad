#!/usr/bin/env python3
"""Nokta bulutu araçlarının İşlemler durumları (docs/adr/0207 §7, 0652; biçim fixtures/processing/README.md).

    python3 scripts/fixtures/pointcloud_processing_cases.py           # çizimi ve durumları yazar
    python3 scripts/fixtures/pointcloud_processing_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Bulutlar bağımsız başvurunun dosyalarıdır (köyün LiDAR'ı fixtures/interaction/v1/pointclouds/koy.laz;
fixtures/pointcloud/v1/ops/sinir.laz ve ikinci.laz); her durumun sonucu başvurunun işlevleriyle (scripts/fixtures/
pointcloud_ops_cases.py: Seyrelt, SMRF, yüksekliğe göre sınıf, alanlar, Rasterleştir, Sınır çıkar, Birleştir) laspy ve numpy'la
yeniden hesaplanır. Yazılan bulut dosyasının denetimi (`cloudOf`): nokta sayısı ve noktaların X, Y, Z tam sayılarının FNV-1a
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
]
# Entity ids in the drawing's order.
IDS = {"Dikdörtgen": 1, "Daire": 2, "Delikli": 3, "koy": 4, "ikinci": 5, "sinir": 6}


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
                "Bulutlar farklı koordinat sistemlerinde; KentOS bulutu yeniden izdüşürmez. Aynı sistemdeki bulutları seçin.",
                [IDS["koy"], IDS["sinir"]]),
    ]


def cases():
    return (thin_cases() + ground_cases() + classify_cases() + clip_cases() + [area_stats_case()] + tile_cases()
            + rasterize_cases() + [boundary_case(), merge_case(), merge_case("las"), merge_case("copc")] + refusals())


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
