#!/usr/bin/env python3
"""The shared cases of `cad.rasters.georef` (docs/adr/0518; plan/gis.md CBS-29).

    python3 scripts/fixtures/raster_georef_command_cases.py           # writes the file
    python3 scripts/fixtures/raster_georef_command_cases.py --check   # writes nothing; compares

Writes fixtures/commands/v1/cad.rasters.georef.json without KentOS code, from ADR 0518 and ADR 0204 §6:

- the transforms fitted here with numpy: the affine and the polynomials of order 2 and 3 by least squares, the thin plate
  by its interpolating system (φ(r) = r² ln r); Helmert and the projective as scripts/fixtures/raster_georef_cases.py
  fits them (Vektör oturtma's frames, docs/adr/0156 §2). Each is held to GDAL: the affine to `GCPsToGeoTransform`
  (pixel corners, as GDAL's GCPs: no half-pixel shift), the polynomials and the thin plate to GDAL's GCP transformer;
- every point's residual (the transformed pixel less its target), m0 = √(Σ v² / (2n − u)) of the control points, the
  check points' root mean square and largest error; a check point and a left-out point are not fitted to;
- targets given in EPSG:4326 carried to the project's TUREF / TM33 by PROJ's own transverse Mercator on GRS80 (KentOS's
  TUREF–WGS 84 step is null);
- a resampling's grid: the forward of 64 points a side of the source's border, its corner on a multiple of the pixel
  (by default the mean of √|area| of a pixel at the control points), a sliver under a millionth of a pixel making no
  row or column; an 8-bit source's output is RGBA;
- the source's SHA-256 read here (`$fixture:` files, the library's embedded raster);
- the refusals in the contract's order, in the command's words.

Numbers are rounded to 9 decimals here (GDAL and numpy differ in the last bits between versions); the steps compare
within 10⁻⁶ (a grid by the default pixel within 10⁻³ m).
"""
import base64
import hashlib
import json
import math
import os
import sys
from fractions import Fraction
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.rasters.georef.json"
sys.path.insert(0, str(Path(__file__).resolve().parent))
os.environ["KENTOS_NO_MAIN"] = "1"
from assets_check_cases import fx, real  # noqa: E402
import raster_georef_cases as ref  # noqa: E402

SRID = 5255
SLIVER = 1e-6
NEED = {"helmert": 2, "affine": 3, "projective": 4, "poly2": 6, "poly3": 10, "thinPlate": 3}
UNKNOWNS = {"helmert": 4, "affine": 6, "projective": 8, "poly2": 12, "poly3": 20, "thinPlate": 0}
TITLE = {"helmert": "Helmert", "affine": "Afin", "projective": "Projektif", "poly2": "Polinom 2", "poly3": "Polinom 3",
         "thinPlate": "İnce plaka"}
GRAY = fx("raster/v1/files/gray-lzw-strips.tif")  # 333 × 257, u8
DEM = fx("raster/v1/files/i16-dem-tiled128.tif")  # 400 × 300, i16
I8 = fx("raster/v1/files/i8.tif")  # 50 × 40, i8
WITHIN = 1e-6
# A grid by the default pixel: its corner is a multiple of a pixel worked out from the thin plate's differences, some
# million pixels from the origin; numpy's and KentOS's last bits move it by a fraction of a millimetre. The record keeps
# the pixel, and the same pixel given makes the same grid (docs/adr/0518 §2).
PIXEL_WITHIN = 1e-3


def r9(x):
    return round(float(x), 9)


def sha(path):
    return hashlib.sha256(Path(real(path)).read_bytes()).hexdigest()


# ── Transforms ────────────────────────────────────────────────────────────

TERMS = {1: 3, 2: 6, 3: 10}


def terms(x, y, order):
    t = [1.0, x, y, x * x, x * y, y * y, x ** 3, x * x * y, x * y * y, y ** 3]
    return t[:TERMS[order]]


def poly_fit(order, used):
    src = np.array([p[0] for p in used], float)
    dst = np.array([p[1] for p in used], float)
    c, t0 = src.mean(axis=0), dst.mean(axis=0)
    s = np.abs(src - c).max()
    A = np.array([terms((x - c[0]) / s, (y - c[1]) / s, order) for x, y in src])
    cx = np.linalg.lstsq(A, dst[:, 0] - t0[0], rcond=None)[0]
    cy = np.linalg.lstsq(A, dst[:, 1] - t0[1], rcond=None)[0]

    def fwd(col, row):
        v = np.array(terms((col - c[0]) / s, (row - c[1]) / s, order))
        return (t0[0] + float(v @ cx), t0[1] + float(v @ cy))

    return fwd


def tps_fit(used):
    src = np.array([p[0] for p in used], float)
    dst = np.array([p[1] for p in used], float)
    c, t0 = src.mean(axis=0), dst.mean(axis=0)
    s = np.abs(src - c).max()
    q = (src - c) / s
    n = len(q)

    def phi(r):
        return 0.0 if r == 0.0 else r * r * math.log(r)

    K = np.array([[phi(math.hypot(*(q[i] - q[j]))) for j in range(n)] for i in range(n)])
    P = np.hstack([np.ones((n, 1)), q])
    M = np.block([[K, P], [P.T, np.zeros((3, 3))]])
    wx = np.linalg.solve(M, np.concatenate([dst[:, 0] - t0[0], np.zeros(3)]))
    wy = np.linalg.solve(M, np.concatenate([dst[:, 1] - t0[1], np.zeros(3)]))

    def fwd(col, row):
        p = (np.array([col, row]) - c) / s
        k = np.array([phi(math.hypot(*(p - q[i]))) for i in range(n)] + [1.0, p[0], p[1]])
        return (t0[0] + float(k @ wx), t0[1] + float(k @ wy))

    return fwd


def fit(method, used):
    g = [{"pixel": list(p[0]), "target": list(p[1]), "used": True} for p in used]
    if method == "helmert":
        return ref.helmert(g)[0]
    if method == "projective":
        return ref.projective(g)[0]
    if method == "affine":
        fwd = poly_fit(1, used)
        # GDAL's geotransform of the same points: pixel corners, no half-pixel shift.
        gt = gdal.GCPsToGeoTransform([gdal.GCP(t[0], t[1], 0.0, px[0], px[1]) for px, t in used])
        for cr in ((0.0, 0.0), (333.0, 0.0), (0.0, 257.0), (166.5, 128.25)):
            a = fwd(*cr)
            b = (gt[0] + gt[1] * cr[0] + gt[2] * cr[1], gt[3] + gt[4] * cr[0] + gt[5] * cr[1])
            assert abs(a[0] - b[0]) < 1e-6 and abs(a[1] - b[1]) < 1e-6, (a, b)
        return fwd
    if method in ("poly2", "poly3"):
        fwd = poly_fit(2 if method == "poly2" else 3, used)
    else:
        fwd = tps_fit(used)
    _, tr = ref.gdal_transformer(g, method)
    for cr in ((0.0, 0.0), (300.0, 0.0), (150.0, 200.0), (40.5, 33.25)):
        a = fwd(*cr)
        b = tr.TransformPoint(0, cr[0], cr[1], 0.0)[1][:2]
        assert abs(a[0] - b[0]) < 1e-4 and abs(a[1] - b[1]) < 1e-4, (method, a, b)
    return fwd


def affine_of(fwd):
    o, ex, ey = fwd(0.0, 0.0), fwd(1000.0, 0.0), fwd(0.0, 1000.0)
    return [o[0], (ex[0] - o[0]) / 1000.0, (ey[0] - o[0]) / 1000.0, o[1], (ex[1] - o[1]) / 1000.0, (ey[1] - o[1]) / 1000.0]


def border(w, h, n=64):
    out = []
    for k in range(n):
        t = k / n
        out += [(t * w, 0.0), (w, t * h), (w - t * w, h), (0.0, h - t * h)]
    return out


def covering(points, size):
    x0, y0 = min(p[0] for p in points), min(p[1] for p in points)
    x1, y1 = max(p[0] for p in points), max(p[1] for p in points)
    left = math.floor(x0 / size + SLIVER) * size
    top = math.ceil(y1 / size - SLIVER) * size
    width = math.ceil((x1 - left) / size - SLIVER)
    height = math.ceil((top - y0) / size - SLIVER)
    if width > 4_000_000 or height > 4_000_000:
        return None
    return [left, size, 0.0, top, 0.0, -size], max(width, 1), max(height, 1)


def default_pixel(fwd, pixels):
    sizes = []
    for c, r in pixels:
        a, b, d = fwd(c, r), fwd(c + 1.0, r), fwd(c, r + 1.0)
        area = abs((b[0] - a[0]) * (d[1] - a[1]) - (b[1] - a[1]) * (d[0] - a[0]))
        sizes.append(math.sqrt(area))
    return sum(sizes) / len(sizes)


# ── Targets in the project's system ───────────────────────────────────────

_WGS = osr.SpatialReference()
_WGS.ImportFromEPSG(4326)
_WGS.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
_TM33 = osr.SpatialReference()
_TM33.ImportFromProj4("+proj=tmerc +lat_0=0 +lon_0=33 +k=1 +x_0=500000 +y_0=0 +ellps=GRS80 +units=m +no_defs")
_TM33.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
_LONLAT = osr.SpatialReference()
_LONLAT.ImportFromProj4("+proj=longlat +ellps=GRS80 +no_defs")
_LONLAT.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
_TO_TM33 = osr.CoordinateTransformation(_LONLAT, _TM33)


def tm33(lon, lat):
    x, y, _ = _TO_TM33.TransformPoint(lon, lat)
    return (x, y)


# ── Drawings ──────────────────────────────────────────────────────────────

SETTINGS = {"srid": SRID, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 1000,
            "workspace": "gis"}


def layer(i, name, locked=False):
    return {"id": i, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


LAYERS = [layer("pafta", "Pafta"), layer("kilitli", "Kilitli", locked=True)]
GRAY_STYLE = {"render": "gray", "bands": [1]}
RAMP_STYLE = {"render": "rampShade", "bands": [1], "stretch": "minMax", "ramp": "Arazi"}


def library_item(path, name):
    b = Path(real(path)).read_bytes()
    ident = "raster-" + hashlib.sha256(b).hexdigest()[:16]
    return ident, {"kind": "asset", "id": ident, "name": name, "path": ["Rasterler"], "format": "tiff",
                   "data": "data:image/tiff;base64," + base64.b64encode(b).decode(), "width": 50, "height": 40}


I8_ID, I8_ITEM = library_item(I8, "i8")


def drawing(entities=(), settings=None, items=()):
    return {"format": "kentos.document", "version": 1, "name": "Oturtma",
            "settings": {**SETTINGS, **(settings or {})}, "origin": {"x": 0.0, "y": 0.0}, "layers": LAYERS,
            "activeLayer": "pafta", "entities": [{**e, "id": k + 1} for k, e in enumerate(entities)],
            "styles": {"items": list(items), "categories": []}}


def raster(file=None, asset=None, w=333, h=257, bands=1, sample="u8", style=None, affine=None, layer_id="pafta",
           georef=None, url=None):
    e = {"kind": "raster", "layerId": layer_id, "attrs": {},
         "affine": affine or [487000.0, 0.25, 0.0, 4420100.0, 0.0, -0.25], "width": w, "height": h, "bands": bands,
         "sample": sample}
    if asset is not None:
        e["asset"] = asset
    if file is not None:
        e["file"] = file
    if url is not None:
        e["url"] = url
    e.update({"srid": 0, "style": style or GRAY_STYLE})
    if georef is not None:
        e["georef"] = georef
    return e


def fields_of(e):
    return {k: v for k, v in e.items() if k not in ("kind", "layerId", "attrs", "id")}


def point():
    return {"kind": "point", "layerId": "pafta", "attrs": {}, "p": {"x": 1.0, "y": 2.0}}


# ── The points ────────────────────────────────────────────────────────────

def sheet(kind, w=333, h=257, noise=True):
    """A sheet's pixels and where a known placement carries them (a few centimetres of clicking when `noise`)."""
    out = []
    grid = [(c, r) for r in (0.1 * h, 0.5 * h, 0.9 * h) for c in (0.1 * w, 0.5 * w, 0.9 * w)]
    grid += [(0.3 * w, 0.3 * h), (0.7 * w, 0.7 * h), (0.3 * w, 0.75 * h)]
    for n, (c, r) in enumerate(grid):
        c, r = round(c, 2), round(r, 2)
        u, v = (c - w / 2) / w, (r - h / 2) / h
        x = 487100 + 0.5 * c + 0.03 * r
        y = 4420300 - 0.5 * r + 0.02 * c
        if kind == "bent":
            x += 4 * u * v + 3 * u * u - 2 * v * v * v
            y += 3 * u * u * v - 2 * v * v + 1.5 * u * u * u
        if noise:
            x += ((n * 37) % 11 - 5) * 0.004
            y += ((n * 53) % 13 - 6) * 0.004
        out.append({"pixel": [c, r], "target": [round(x, 3), round(y, 3)]})
    return out


def halton(i, b):
    f, r = 1.0, 0.0
    while i > 0:
        f /= b
        r += f * (i % b)
        i //= b
    return r


def spread(w, h, n):
    """`n` pixels spread over a `w` × `h` source (Halton's 2 and 3), their targets bent as `sheet`'s."""
    out = []
    for k in range(1, n + 1):
        c, r = round(halton(k, 2) * w, 2), round(halton(k, 3) * h, 2)
        u, v = (c - w / 2) / w, (r - h / 2) / h
        x = 487100 + 0.5 * c + 0.03 * r + 4 * u * v + 3 * u * u - 2 * v * v * v
        y = 4420300 - 0.5 * r + 0.02 * c + 3 * u * u * v - 2 * v * v + 1.5 * u * u * u
        out.append({"pixel": [c, r], "target": [round(x, 3), round(y, 3)]})
    return out


def with_roles(points, checks=(), off=(), names=False):
    out = []
    for i, p in enumerate(points):
        q = dict(p)
        if names:
            q = {"name": f"N{i + 1}", **q}
        if i in checks:
            q["check"] = True
        if i in off:
            q["off"] = True
        out.append(q)
    return out


# ── The answer ────────────────────────────────────────────────────────────

def solved(method, points, *, srid=None, src=(333, 257), carried=None):
    """Residuals, counts, m0, check errors and the forward of the control points; `carried`: the targets already in
    the project's system (another script's, docs/adr/0594)."""
    if carried is not None:
        targets = carried
    elif srid == 4326:
        targets = [tm33(*p["target"]) for p in points]
    else:
        targets = [tuple(p["target"]) for p in points]
    used = [(tuple(p["pixel"]), t) for p, t in zip(points, targets) if not p.get("check") and not p.get("off")]
    fwd = fit(method, used)
    residuals, vv, checks = [], 0.0, []
    for p, t in zip(points, targets):
        x, y = fwd(*p["pixel"])
        vx, vy = x - t[0], y - t[1]
        d = math.hypot(vx, vy)
        residuals.append({"dx": r9(vx), "dy": r9(vy), "d": r9(d), "target": [r9(t[0]), r9(t[1])]})
        if not p.get("check") and not p.get("off"):
            vv += vx * vx + vy * vy
        if p.get("check"):
            checks.append(d)
    dof = 2 * len(used) - UNKNOWNS[method]
    m0 = math.sqrt(vv / dof) if method != "thinPlate" and dof > 0 else None
    rmse = math.sqrt(sum(d * d for d in checks) / len(checks)) if checks else None
    most = max(checks) if checks else None
    return {"fwd": fwd, "residuals": residuals, "control": len(used), "checks": len(checks), "m0": m0, "rmse": rmse,
            "max": most, "used": [u[0] for u in used], "src": src}


NO_CHECKS = {"code": "no_check_points",
             "message": "Denetim noktası yok: m0 yalnız kontrol noktalarının uyumunu söyler; dönüşüme girmeyen, rastere yayılmış denetim noktaları ekleyin.",
             "path": "points"}
NOT_READ = {"code": "source_not_read", "message": "Rasterin kaynağı okunamadı; kayıt kaynağın özetini taşımıyor.",
            "path": "uid"}


def record(method, points, s, source, *, srid=None, output=None):
    g = {"method": method, "points": points}
    if srid is not None:
        g["srid"] = srid
    g["source"] = source
    if s["m0"] is not None:
        g["m0"] = r9(s["m0"])
    if s["rmse"] is not None:
        g["checkRmse"] = r9(s["rmse"])
        g["checkMax"] = r9(s["max"])
    if output is not None:
        g["output"] = output
    return g


def grid_of(s, pixel=None, nearest=False):
    size = pixel if pixel is not None else default_pixel(s["fwd"], s["used"])
    edge = [s["fwd"](*p) for p in border(*s["src"])]
    got = covering(edge, size)
    assert got is not None
    affine, w, h = got
    out = {"affine": [r9(v) for v in affine], "width": w, "height": h, "pixel": r9(affine[1])}
    if nearest:
        out["resampling"] = "nearest"
    return out


def completed(raster_fields, s, *, written=None, warnings=()):
    out = {"uid": "$uidOf:1", "residuals": s["residuals"], "control": s["control"], "checks": s["checks"],
           "raster": raster_fields}
    if written is not None:
        out["written"] = written
    out["revision"] = "$current"
    return {"status": "completed", "output": out, "warnings": list(warnings)}


def refused(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


cases = []


def add(name, setup, input_, result, expect=None, op="execute", within=None):
    step = {"op": op, "input": input_, "result": result}
    if within is not None:
        step["within"] = within
    if expect is not None:
        step["expect"] = expect
    elif result["status"] != "completed" or op != "execute":
        step["expect"] = {"canUndo": False, "revision": "same"}
    cases.append({"name": name, "setup": setup, "steps": [step]})


def placed_case(name, method, points, *, srid=None, file=GRAY, warnings=None, op="execute", digest=True):
    e = raster(file=file)
    s = solved(method, points, srid=srid)
    source = {"width": 333, "height": 257, "file": file}
    if digest:
        source["digest"] = sha(file)
    f = fields_of(e)
    f.update({"affine": [r9(v) for v in affine_of(s["fwd"])], "srid": SRID,
              "georef": record(method, points, s, source, srid=srid)})
    warns = list(warnings if warnings is not None else ([] if s["checks"] else [NO_CHECKS]))
    expect = None
    if op == "execute":
        expect = {"canUndo": True, "revision": "changed", "fields": {"1": {"width": 333, "height": 257, "file": file}}}
    inp = {"uid": "$uidOf:1", "method": method, "points": points}
    if srid is not None:
        inp["srid"] = srid
    add(name, drawing([e]), inp, completed(f, s, warnings=warns), expect=expect, op=op,
        within=1e-5 if srid else WITHIN)


STRAIGHT = sheet("straight")
BENT = sheet("bent")

# Placed: the affine and Helmert set the raster's affine and keep the record.
placed_case("Afin: dört kontrol ve iki denetim noktası; dönüşüm, m0 ve denetimin hataları kayda",
            "affine", with_roles(STRAIGHT[:4] + STRAIGHT[9:11], checks=(4, 5), names=True))
placed_case("Helmert: denetim noktasıyla; döndürür, ölçekler", "helmert",
            with_roles(STRAIGHT[:5] + STRAIGHT[10:11], checks=(5,)))
placed_case("Afin: denetim noktası yoksa uyarılır", "affine", with_roles(STRAIGHT[:6]))
placed_case("Afin: kullanılmayan noktanın artığı söylenir, dönüşüme girmez", "affine",
            with_roles(STRAIGHT[:7], checks=(6,), off=(5,)))
placed_case("Afin: tam üç kontrol noktasında m0 yok", "affine", with_roles([STRAIGHT[0], STRAIGHT[2], STRAIGHT[7], STRAIGHT[9]], checks=(3,)))
placed_case("Plan: afin, yazmadan", "affine", with_roles(STRAIGHT[:8], checks=(6, 7)), op="plan")
# The rotated sheet: pixel corners (GDAL's GCPs), the affine exactly the placement, no half-pixel shift.
ANG = math.radians(30.0)
ROT = [487000.0, 0.25 * math.cos(ANG), 0.25 * math.sin(ANG), 4420300.0, 0.25 * math.sin(ANG), -0.25 * math.cos(ANG)]
ROTATED = [{"pixel": [c, r], "target": [ROT[0] + ROT[1] * c + ROT[2] * r, ROT[3] + ROT[4] * c + ROT[5] * r]}
           for c, r in ((0.0, 0.0), (333.0, 0.0), (333.0, 257.0), (0.0, 257.0), (166.5, 128.5))]
ROTATED[4]["check"] = True
placed_case("Afin: döndürülmüş pafta, piksel köşeleri; dönüşüm yerleşimin kendisi, yarım piksel kayma yok", "affine",
            ROTATED)
# Targets in WGS 84 carried to TUREF / TM33.
GEO = []
for p in STRAIGHT[:5]:
    c, r = p["pixel"]
    lon, lat = 33.36 + c * 0.0000058, 39.93 - r * 0.0000045 + c * 0.0000001
    GEO.append({"pixel": [c, r], "target": [round(lon, 9), round(lat, 9)]})
GEO[4]["check"] = True
placed_case("Afin: hedefler EPSG:4326'da; projenin TUREF / TM33'üne çevrilir, kayıtta sistemi", "affine", GEO,
            srid=4326)
placed_case("Afin: hedeflerin sistemi projeninki verilirse çevrilmez, kayda yazılmaz", "helmert",
            with_roles(STRAIGHT[:4], checks=(3,)), srid=None)
cases[-1]["steps"][0]["input"]["srid"] = SRID
placed_case("Afin: bağlı kaynak bulunamadı; yazılır, kaynağın özeti yok", "affine",
            with_roles(STRAIGHT[:5], checks=(4,)), file="/yok/pafta.tif",
            warnings=[NOT_READ], digest=False)


def resampled_plan(name, method, points, *, file=GRAY, w=333, h=257, sample="u8", pixel=None, nearest=False,
                   style=None, out_bands=None, out_sample=None, out_style=None, within=WITHIN):
    e = raster(file=file, w=w, h=h, sample=sample, style=style)
    s = solved(method, points, src=(w, h))
    output = grid_of(s, pixel, nearest)
    source = {"width": w, "height": h, "file": file, "digest": sha(file)}
    stem = Path(real(file)).stem
    kept = file.rsplit("/", 1)[0] + f"/{stem}-oturtulmus.tif"
    f = fields_of(e)
    f.update({"affine": output["affine"], "width": output["width"], "height": output["height"],
              "bands": out_bands or 1, "sample": out_sample or sample, "file": kept, "srid": SRID,
              "style": out_style or e["style"], "georef": record(method, points, s, source, output=output)})
    inp = {"uid": "$uidOf:1", "method": method, "points": points}
    if pixel is not None:
        inp["pixel"] = pixel
    if nearest:
        inp["resampling"] = "nearest"
    warns = [] if s["checks"] else [NO_CHECKS]
    add(name, drawing([e]), inp, completed(f, s, written=kept, warnings=warns), op="plan", within=within)


RGBA = {"render": "rgb", "bands": [1, 2, 3, 4]}
resampled_plan("Plan: ince plaka, 8 bitlik kaynak RGBA'ya; ızgara kaynağın sınırından, piksel ortalama ölçekten",
               "thinPlate", with_roles(BENT[:9] + BENT[10:12], checks=(9, 10)), out_bands=4, out_style=RGBA,
               within=PIXEL_WITHIN)
resampled_plan("Plan: polinom 2, 0,6 m piksel, en yakın örnekleme", "poly2",
               with_roles(BENT[:9] + BENT[9:10], checks=(9,)), pixel=0.6, nearest=True, out_bands=4, out_style=RGBA)
resampled_plan("Plan: projektif, denetim noktasız uyarılır", "projective", with_roles(BENT[:8]), out_bands=4,
               out_style=RGBA)
DEM_PTS = spread(400, 300, 16)
resampled_plan("Plan: polinom 3, 16 bitlik yükseklik modeli bandı ve türüyle kalır", "poly3",
               with_roles(DEM_PTS, checks=(13, 14, 15)), file=DEM, w=400, h=300, sample="i16",
               style=RAMP_STYLE)

# Resampled into the library: an embedded source's output is embedded; its id and digest are its bytes'.
I8_PTS = [{"pixel": [c, r], "target": [487000.0 + 0.25 * c + 0.01 * r + 0.002 * c * r / 50, 4420100.0 - 0.25 * r + 0.01 * c]}
          for c, r in ((0.0, 0.0), (50.0, 0.0), (50.0, 40.0), (0.0, 40.0), (25.0, 20.0), (10.0, 30.0))]
I8_PTS[5]["check"] = True
e = raster(asset=I8_ID, w=50, h=40, sample="i8", style=RAMP_STYLE)
s = solved("thinPlate", I8_PTS, src=(50, 40))
output = grid_of(s, None, True)
f = fields_of(e)
f.pop("asset")
f.update({"affine": output["affine"], "width": output["width"], "height": output["height"], "asset": "$any",
          "srid": SRID, "georef": record("thinPlate", I8_PTS, s,
                                         {"width": 50, "height": 40, "asset": I8_ID, "digest": sha(I8)},
                                         output={**output, "digest": "$any"})})
add("İnce plaka: gömülü kaynak yeniden örneklenir, çıktı kitaplığa gömülür, raster onu gösterir", drawing([e], items=[I8_ITEM]),
    {"uid": "$uidOf:1", "method": "thinPlate", "points": I8_PTS, "resampling": "nearest"}, completed(f, s),
    expect={"canUndo": True, "revision": "changed",
            "fields": {"1": {"width": output["width"], "height": output["height"], "bands": 1, "sample": "i8"}}},
    within=PIXEL_WITHIN)

# A resampled raster put back on its source by the affine: its record's source, its pixels.
WARPED_OUT = {"affine": [486999.5, 0.3, 0.0, 4420101.0, 0.0, -0.3], "width": 45, "height": 38, "pixel": 0.3,
              "resampling": "nearest"}
warped_record = {"method": "thinPlate", "points": [{"pixel": p["pixel"], "target": p["target"]} for p in I8_PTS[:4]],
                 "source": {"width": 50, "height": 40, "asset": I8_ID, "digest": sha(I8)}, "output": WARPED_OUT}
e = raster(file=fx("raster/v1/files/i8-oturtulmus.tif"), w=45, h=38, sample="i8", style=RAMP_STYLE,
           affine=WARPED_OUT["affine"], georef=warped_record)
pts = with_roles(I8_PTS[:5], checks=(4,))
s = solved("affine", pts, src=(50, 40))
f = fields_of(e)
f.pop("file")
f.update({"affine": [r9(v) for v in affine_of(s["fwd"])], "width": 50, "height": 40, "asset": I8_ID, "srid": SRID,
          "georef": record("affine", pts, s, {"width": 50, "height": 40, "asset": I8_ID, "digest": sha(I8)})})
add("Afin: yeniden örneklenmiş raster kaydının kaynağına döner; noktalar kaynağın pikselleri",
    drawing([e], items=[I8_ITEM]), {"uid": "$uidOf:1", "method": "affine", "points": pts}, completed(f, s),
    expect={"canUndo": True, "revision": "changed",
            "fields": {"1": {"width": 50, "height": 40, "asset": I8_ID, "file": None, "bands": 1}}},
    within=WITHIN)

# Refusals, in the contract's order.
OK_PTS = with_roles(STRAIGHT[:5], checks=(4,))
base = drawing([raster(file=GRAY)])
add("Ret: kimlik UUID değil", base, {"uid": "1", "method": "affine", "points": OK_PTS},
    refused("invalid_uid", "“1” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "uid"))
add("Ret: nesne yok", base, {"uid": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7699", "method": "affine", "points": OK_PTS},
    refused("entity_not_found", "“0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7699” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir.", "uid"))
add("Ret: nesne raster değil", drawing([point()]), {"uid": "$uidOf:1", "method": "affine", "points": OK_PTS},
    refused("not_a_raster", "Verilen nesne bir raster değil; yalnız raster oturtulur.", "uid"))
add("Ret: raster kilitli katmanda", drawing([raster(file=GRAY, layer_id="kilitli")]),
    {"uid": "$uidOf:1", "method": "affine", "points": OK_PTS},
    refused("layer_locked", "“Kilitli” katmanı kilitli; üzerindeki raster oturtulamaz. Kilidi Katmanlar panelinden açın.", "uid"))
bad = [dict(p) for p in OK_PTS]
bad[4]["off"] = True
add("Ret: kullanılmayan nokta denetim noktası yapılmış", base, {"uid": "$uidOf:1", "method": "affine", "points": bad},
    refused("invalid_control_point", "Kullanılmayan nokta denetim noktası olamaz; denetim noktası Kullan'da kalır.", "points[4]"))
bad = [dict(p) for p in OK_PTS]
bad[1] = {"name": " K2", **bad[1]}
add("Ret: noktanın adı kırpılmamış", base, {"uid": "$uidOf:1", "method": "affine", "points": bad},
    refused("invalid_control_point", "Noktanın adı kırpılmış, boş olmayan ve en çok 64 harf olmalı.", "points[1]"))
add("Ret: hedeflerin sistemi kayıtta yok", base, {"uid": "$uidOf:1", "method": "affine", "points": OK_PTS, "srid": 99999},
    refused("unknown_srid", "EPSG:99999 KentOS'un kayıtlı koordinat sistemlerinde yok; hedefleri kayıtlı bir sistemde verin.", "srid"))
add("Ret: yerel projede hedefler başka sistemde", drawing([raster(file=GRAY)], settings={"srid": 0}),
    {"uid": "$uidOf:1", "method": "affine", "points": GEO, "srid": 4326},
    refused("target_not_converted", "Hedefler EPSG:4326 sisteminde; projenin koordinat sistemi yok (yerel proje). Hedefleri projenin koordinatlarıyla verin.", "srid"))
few = with_roles(STRAIGHT[:4], checks=(2, 3))
add("Ret: denetim noktaları sayılmaz; afin üç kontrol noktası ister", base,
    {"uid": "$uidOf:1", "method": "affine", "points": few},
    refused("too_few_control_points", "Afin en az 3 kontrol noktası ister; 2 verildi (denetim ve kullanılmayan noktalar dönüşüme girmez).", "points"))
twice = [dict(p) for p in STRAIGHT[:4]]
twice[3] = {"pixel": twice[0]["pixel"], "target": [487300.0, 4420100.0]}
add("Ret: iki kontrol noktası aynı pikselde", base, {"uid": "$uidOf:1", "method": "affine", "points": twice},
    refused("control_points_duplicate", "İki kontrol noktası aynı pikselde; birini düzeltin ya da kullanmayın.", "points"))
line = [{"pixel": [10.0 * k, 5.0 * k], "target": [487000.0 + k, 4420000.0 + 2.0 * k]} for k in range(1, 5)]
assert all(Fraction(10 * k) * 5 - Fraction(5 * k) * 10 == 0 for k in range(1, 5))
add("Ret: kontrol noktalarının pikselleri bir doğruda", base, {"uid": "$uidOf:1", "method": "affine", "points": line},
    refused("control_points_collinear", "Kontrol noktalarının pikselleri bir doğru üzerinde; rasterin iki yönüne yayılmış noktalar verin.", "points"))
CIRCLE = [{"pixel": [float(c), float(r)], "target": [487000.0 + 0.5 * c + k * 0.01, 4420000.0 - 0.5 * r]}
          for k, (c, r) in enumerate(((150, 100), (100, 150), (50, 100), (100, 50), (130, 140), (70, 60)))]
# Six pixels on one circle: x² + y² is the others' combination, the second order has no single fit.
assert np.linalg.matrix_rank(np.array([terms((p["pixel"][0] - 100) / 50, (p["pixel"][1] - 100) / 50, 2) for p in CIRCLE]), tol=1e-9) == 5
add("Ret: polinom 2'nin altı kontrol noktası bir çemberde; tek çözüm yok", base,
    {"uid": "$uidOf:1", "method": "poly2", "points": CIRCLE},
    refused("georef_singular", "Bu kontrol noktalarıyla dönüşüm çözülemiyor; noktaları rasterin geneline yayın.", "points"))
BENT_PTS = with_roles(BENT[:9] + BENT[10:12], checks=(9, 10))
add("Ret: çıktının piksel boyu eksi", base,
    {"uid": "$uidOf:1", "method": "thinPlate", "points": BENT_PTS, "pixel": -2.5},
    refused("invalid_pixel_size", "Çıktının piksel boyu sıfırdan büyük, sonlu bir uzunluk olmalı; -2.5 verildi.", "pixel"))
add("Ret: çıktının ızgarası çok büyük", base,
    {"uid": "$uidOf:1", "method": "thinPlate", "points": BENT_PTS, "pixel": 0.00001},
    refused("georef_grid", "Çıktının ızgarası kurulamadı: dönüşüm rasteri çok büyütüyor ya da bozuyor; noktaları ya da piksel boyunu denetleyin.", "pixel"))
add("Ret: göreli kaynak, çizimin dosyası yok", drawing([raster(file="tarama/pafta.tif")]),
    {"uid": "$uidOf:1", "method": "thinPlate", "points": BENT_PTS},
    refused("invalid_file_path", "Rasterin kaynağı “tarama/pafta.tif” göreli bir yol, ama çizimin dosyası yok; çizimi kaydedin ya da drawing'i verin.", "drawing"))
MISSING = "/yok/pafta.tif"
add("Ret: yeniden örneklenecek kaynak bulunamadı", drawing([raster(file=MISSING)]),
    {"uid": "$uidOf:1", "method": "thinPlate", "points": BENT_PTS},
    refused("file_not_found", f"Rasterin kaynağı pafta.tif bulunamadı. Bakılan yerler: {MISSING}.", "uid"))
add("Ret: adresten okunan raster yeniden örneklenmez", drawing([raster(url="https://ornek.gov.tr/pafta.tif")]),
    {"uid": "$uidOf:1", "method": "thinPlate", "points": BENT_PTS},
    refused("import_unreadable", "Adresten okunan raster bu ortamda okunmaz; dosyasını indirip bağlayın ya da gömün.", "uid"))
add("Ret: ev sahibinin çıktısı kitaplıkta yok", base,
    {"uid": "$uidOf:1", "method": "thinPlate", "points": BENT_PTS, "made": {"asset": "raster-0000000000000000"}},
    refused("unknown_asset", "“raster-0000000000000000” kimlikli raster projenin kitaplığında yok.", "made.asset"))
add("Ret: ev sahibinin çıktısı planın ızgarası değil", base,
    {"uid": "$uidOf:1", "method": "thinPlate", "points": BENT_PTS, "made": {"file": I8}},
    refused("georef_output_mismatch", "Verilen çıktının ızgarası (konumu, piksel boyu ya da boyu) planınki değil; hiçbir şey yazılmadı.", "made"))
add("Ret: afin yeniden örneklemez; made verilmez", base,
    {"uid": "$uidOf:1", "method": "affine", "points": OK_PTS, "made": {"asset": I8_ID}},
    refused("georef_output_mismatch", "Afin rasteri yeniden örneklemez; made verilmez.", "made"))
add("Ret: beklenen sürüm geçersiz", base, {"uid": "$uidOf:1", "method": "affine", "points": OK_PTS, "expectedRevision": "dün"},
    refused("invalid_revision", "Beklenen sürüm “dün” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
add("Ret: çizim komut hazırlandıktan sonra değişmiş", base,
    {"uid": "$uidOf:1", "method": "affine", "points": OK_PTS, "expectedRevision": "999"},
    {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}})


def compact(v):
    return json.dumps(v, ensure_ascii=False, separators=(", ", ": "))


def build():
    out = ["{", '  "format": "kentos.command-cases",', '  "version": 1,', '  "command": "cad.rasters.georef",',
           '  "commandVersion": 1,', '  "title": "Raster oturt: kontrol ve denetim noktaları, kalıcı kayıt",',
           f'  "note": {compact("Written by scripts/fixtures/raster_georef_command_cases.py from docs/adr/0518 and docs/adr/0204 §6; do not edit by hand.")},',
           f'  "setup": {compact(drawing())},', '  "cases": [']
    blocks = []
    for c in cases:
        lines = ["    {", f'      "name": {compact(c["name"])},', f'      "setup": {compact(c["setup"])},', '      "steps": [']
        lines.append(",\n".join(f"        {compact(s)}" for s in c["steps"]))
        lines.append("      ]")
        lines.append("    }")
        blocks.append("\n".join(lines))
    out.append(",\n".join(blocks))
    out.append("  ]")
    out.append("}")
    return "\n".join(out) + "\n"


def numbers_close(a, b):
    """The file as written and as on disk, numbers within 10⁻⁸ of each other (GDAL's and numpy's last bits)."""
    if isinstance(a, float) or isinstance(b, float):
        return isinstance(a, (int, float)) and isinstance(b, (int, float)) and abs(a - b) <= 1e-8 * max(1.0, abs(a))
    if isinstance(a, list):
        return isinstance(b, list) and len(a) == len(b) and all(numbers_close(x, y) for x, y in zip(a, b))
    if isinstance(a, dict):
        return isinstance(b, dict) and a.keys() == b.keys() and all(numbers_close(a[k], b[k]) for k in a)
    return a == b


def main():
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or not numbers_close(json.loads(OUT.read_text(encoding="utf-8")), json.loads(text)):
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil")
        print(f"{len(cases)} durum tutarlı")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{len(cases)} durum yazıldı")


if __name__ == "__main__":
    main()
