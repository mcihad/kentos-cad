#!/usr/bin/env python3
"""Eş yükselti eğrilerinin bağımsız başvurusu (docs/adr/0231 §9).

    python3 scripts/fixtures/contour_cases.py           # durumları yazar
    python3 scripts/fixtures/contour_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Eğriler ADR'nin kurallarıyla çıkarılır: hücre merkezleri arasında kareler,
z ≥ L içeride, kenardaki kesişme kenarın sol (ya da üst) ucundan doğrusal, eyerde dört köşenin ortalaması,
yüksek taraf solda, parçalar kenarlarının kimlikleriyle zincirlenir (önce açık eğriler, sonra kapalılar),
art arda aynı noktalar teke iner, isteğe bağlı Douglas-Peucker. Her düzeyin Kot yazısı ADR 0149'un gösterim
kuralıyla (decimal), basamağı aralığın ve tabanın tam yazıldığı en az basamak (en çok 6). DEM'ler yüzey analizinin örnekleri
(fixtures/terrain/v1) ve eyerleri, düzlüğü ve düzeye eşit değerleri olan küçük bir DEM
(fixtures/contours/v1/eyer.tif, GDAL'la yazılır). gdal_contour'la çapraz denetim: geniş DEM'de (düzeye
eşit değer yok) her düzeyin eğrilerinin toplam boyu gdal_contour'unkiyle aynı olmalı (göreli 1e-9).
"""

import json
import math
import subprocess
import sys
import tempfile
from decimal import ROUND_HALF_UP, Decimal, getcontext
from pathlib import Path

import numpy as np
from osgeo import gdal, ogr, osr

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/contours/v1"
CASES = DIR / "cases.json"
TERRAIN = ROOT / "fixtures/terrain/v1"


def eyer():
    """Saddles both ways, a flat patch at a level, values equal to levels, a nodata cell."""
    z = np.array([
        [10, 10, 10, 10, 10, 10, 10, 10],
        [10, 20, 10, 20, 10, 15, 15, 10],
        [10, 10, 20, 10, 10, 15, 15, 10],
        [10, 20, 10, 20, 12, 15, 15, 10],
        [10, 10, 10, 10, 10, 10, -9999, 10],
        [10, 11, 13, 17, 19, 14, 12, 10],
    ], dtype=np.float32)
    return z


def write_eyer(path):
    z = eyer()
    ds = gdal.GetDriverByName("GTiff").Create(str(path), z.shape[1], z.shape[0], 1, gdal.GDT_Float32)
    ds.SetGeoTransform([400000.0, 3.0, 0.0, 4100030.0, 0.0, -3.0])
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5254)
    ds.SetProjection(srs.ExportToWkt())
    band = ds.GetRasterBand(1)
    band.SetNoDataValue(-9999.0)
    band.WriteArray(z)
    ds = None


def read(path):
    ds = gdal.Open(str(path))
    b = ds.GetRasterBand(1)
    return b.ReadAsArray().astype(np.float64), list(ds.GetGeoTransform()), b.GetNoDataValue()


# ── The ADR's rules ──────────────────────────────────────────────────────

def contours(z, gt, nodata, interval, base, index_every, simplify_tol):
    h, w = z.shape
    v = z.copy()
    if nodata is not None:
        v[v == nodata] = np.nan
    x0, a, b, y0, c, d = gt
    flip = a * d - b * c > 0.0

    def point(px, py):
        return [x0 + a * px + b * py, y0 + c * px + d * py]

    def level(k):
        return base + k * interval

    def h_edge(i, j):
        return 2 * (j * w + i)

    def v_edge(i, j):
        return 2 * (j * w + i) + 1

    segs = []  # (k, start edge, end edge, sp, ep)
    for j in range(h - 1):
        for i in range(w - 1):
            zz = [float(v[j, i]), float(v[j, i + 1]), float(v[j + 1, i + 1]), float(v[j + 1, i])]
            if any(math.isnan(t) for t in zz):
                continue
            lo, hi = min(zz), max(zz)
            below = math.floor((lo - base) / interval)
            k = int(below) - 1
            while level(k) <= hi:
                L = level(k)
                if L > lo:
                    inside = [t >= L for t in zz]

                    def crossing(e):
                        fi, fj = i + 0.5, j + 0.5
                        if e == 0:
                            t = (L - zz[0]) / (zz[1] - zz[0])
                            return h_edge(i, j), point(fi + t, fj)
                        if e == 1:
                            t = (L - zz[1]) / (zz[2] - zz[1])
                            return v_edge(i + 1, j), point(fi + 1.0, fj + t)
                        if e == 2:
                            t = (L - zz[3]) / (zz[2] - zz[3])
                            return h_edge(i, j + 1), point(fi + t, fj + 1.0)
                        t = (L - zz[0]) / (zz[3] - zz[0])
                        return v_edge(i, j), point(fi, fj + t)

                    if inside[0] == inside[2] and inside[1] == inside[3] and inside[0] != inside[1]:
                        joined = (zz[0] + zz[1] + zz[2] + zz[3]) / 4.0 >= L
                        if inside[0]:
                            pieces = [(1, 0), (3, 2)] if joined else [(1, 2), (3, 0)]
                        else:
                            pieces = [(0, 3), (2, 1)] if joined else [(0, 1), (2, 3)]
                    else:
                        start = end = None
                        for e in range(4):
                            if inside[e] != inside[(e + 1) % 4]:
                                if inside[(e + 1) % 4]:
                                    start = e
                                else:
                                    end = e
                        pieces = [(start, end)]
                    if flip:
                        pieces = sorted([(q, p) for p, q in pieces])
                    for p, q in pieces:
                        s_id, sp = crossing(p)
                        e_id, ep = crossing(q)
                        segs.append((k, s_id, e_id, sp, ep))
                k += 1
    levels = sorted({s[0] for s in segs})
    lines = []
    for k in levels:
        lst = [n for n, s in enumerate(segs) if s[0] == k]
        starts = {segs[n][1]: n for n in lst}
        ends = {segs[n][2] for n in lst}
        used = set()

        def chain(first):
            pts = [segs[first][3]]
            cur = first
            used.add(cur)
            while True:
                pts.append(segs[cur][4])
                nxt = starts.get(segs[cur][2])
                if nxt is None or nxt == first or nxt in used:
                    break
                used.add(nxt)
                cur = nxt
            return pts

        chains = []
        for n in lst:
            if n not in used and segs[n][1] not in ends:
                chains.append(chain(n))
        for n in lst:
            if n not in used:
                chains.append(chain(n))
        for pts in chains:
            out = []
            for p in pts:
                if not out or out[-1] != p:
                    out.append(p)
            if len(out) < 2:
                continue
            if simplify_tol > 0.0:
                out = douglas_peucker(out, simplify_tol)
            lines.append({"k": k, "value": level(k), "index": k % index_every == 0, "pts": out})
    return lines


def to_segment(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    len2 = dx * dx + dy * dy
    if len2 == 0.0:
        qx, qy = a
    else:
        t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2
        t = min(1.0, max(0.0, t))
        qx, qy = a[0] + t * dx, a[1] + t * dy
    return math.sqrt((p[0] - qx) * (p[0] - qx) + (p[1] - qy) * (p[1] - qy))


def douglas_peucker(pts, tol):
    n = len(pts)
    if n < 3:
        return pts
    keep = [False] * n
    keep[0] = keep[-1] = True
    stack = [(0, n - 1)]
    while stack:
        a, b = stack.pop()
        far, at = 0.0, 0
        for k in range(a + 1, b):
            dd = to_segment(pts[k], pts[a], pts[b])
            if dd > far:
                far, at = dd, k
        if at != 0 and far > tol:
            keep[at] = True
            stack.append((at, b))
            stack.append((a, at))
    return [p for p, k in zip(pts, keep) if k]


def length(pts):
    return sum(math.hypot(q[0] - p[0], q[1] - p[1]) for p, q in zip(pts, pts[1:]))


# ── gdal_contour cross-check ─────────────────────────────────────────────

def gdal_lengths(path, interval, base, clip):
    """gdal_contour's lines' lengths by level, clipped to `clip` (gdal_contour runs its lines on to
    the raster's outer edge; the ADR's squares end at the outermost cell centres)."""
    with tempfile.TemporaryDirectory(dir=ROOT / ".run") as tmp:
        out = Path(tmp) / "c.gpkg"
        subprocess.run(["gdal_contour", "-q", "-a", "kot", "-i", str(interval), "-off", str(base), str(path), str(out)], check=True, capture_output=True)
        ds = ogr.Open(str(out))
        layer = ds.GetLayer(0)
        totals = {}
        for f in layer:
            g = f.GetGeometryRef().Intersection(clip)
            totals[f.GetField("kot")] = totals.get(f.GetField("kot"), 0.0) + g.Length()
        return totals


def cross_check(name, z, gt, spec):
    full = z.copy()
    hole = full == -9999.0
    if hole.any():
        j, i = np.mgrid[0:full.shape[0], 0:full.shape[1]].astype(np.float64)
        # genis's own surface (fixtures/terrain_cases.py's dem_genis) where the band was.
        surf = (300.0 + 80.0 * np.sin(i / 47.0) * np.cos(j / 61.0) + 0.15 * i
                + 25.0 * np.exp(-((i - 410) ** 2 + (j - 180) ** 2) / 4000.0)).astype(np.float32).astype(np.float64)
        full[hole] = surf[hole]
    problems = []
    with tempfile.TemporaryDirectory(dir=ROOT / ".run") as tmp:
        path = Path(tmp) / "full.tif"
        ds = gdal.GetDriverByName("GTiff").Create(str(path), full.shape[1], full.shape[0], 1, gdal.GDT_Float32)
        ds.SetGeoTransform(gt)
        ds.GetRasterBand(1).WriteArray(full.astype(np.float32))
        ds = None
        x0, a, _, y0, _, d = gt
        h, w = full.shape
        ring = ogr.Geometry(ogr.wkbLinearRing)
        for px, py in ((0.5, 0.5), (w - 0.5, 0.5), (w - 0.5, h - 0.5), (0.5, h - 0.5), (0.5, 0.5)):
            ring.AddPoint_2D(x0 + a * px, y0 + d * py)
        clip = ogr.Geometry(ogr.wkbPolygon)
        clip.AddGeometry(ring)
        theirs = gdal_lengths(path, spec["interval"], spec["base"], clip)
        mine = {}
        for ln in contours(full, gt, None, spec["interval"], spec["base"], spec["indexEvery"], 0.0):
            mine[ln["value"]] = mine.get(ln["value"], 0.0) + length(ln["pts"])
    if set(theirs) != set(mine):
        problems.append(f"{name}: gdal_contour'un düzeyleri farklı ({sorted(set(theirs) ^ set(mine))})")
    # gdal_contour nudges a cell exactly at a level off it; the ADR keeps z ≥ L (its own cases: eyer, int16).
    exact = {float(v) for v in np.unique(full)}
    for val, t in theirs.items():
        if val in exact:
            continue
        m = mine.get(val)
        if m is None or abs(t - m) > 1e-9 * max(1.0, t):
            problems.append(f"{name} düzey {val}: toplam boy {m!r}, gdal_contour {t!r}")
    return problems


# ── Kot (ADR 0231 §9, ADR 0149) ──────────────────────────────────────────

getcontext().prec = 200


def shown(v, d):
    """ADR 0149's display rule: the exact value to 7 decimals, then to d, halves away from zero."""
    x = Decimal(abs(v))
    if d >= 7:
        y = x.quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    else:
        y = x.quantize(Decimal(1).scaleb(-7), rounding=ROUND_HALF_UP).quantize(Decimal(1).scaleb(-d), rounding=ROUND_HALF_UP)
    text = format(y, "f")
    return "-" + text if v < 0 and any(c in "123456789" for c in text) else text


def decimals_of(x):
    """The fewest decimals (at most 6) that write x whole, to 1e-9 of it."""
    scale = 1.0
    for d in range(7):
        v = x * scale
        if abs(v - round(v)) <= 1e-9 * max(abs(v), 1.0):
            return d
        scale *= 10.0
    return 6


def texts(lines, spec):
    d = max(decimals_of(spec["interval"]), decimals_of(spec["base"]))
    return {str(ln["k"]): shown(ln["value"], d) for ln in lines}


# ── Cases ────────────────────────────────────────────────────────────────

def build():
    cases = []
    problems = []
    specs = [
        ("eyer", DIR / "eyer.tif", {"interval": 5.0, "base": 0.0, "indexEvery": 2, "simplify": 0.0}, True),
        ("eyer", DIR / "eyer.tif", {"interval": 2.5, "base": 1.25, "indexEvery": 4, "simplify": 0.0}, True),
        ("tepe", TERRAIN / "tepe.tif", {"interval": 5.0, "base": 0.0, "indexEvery": 5, "simplify": 0.0}, True),
        ("tepe", TERRAIN / "tepe.tif", {"interval": 2.0, "base": 1.0, "indexEvery": 5, "simplify": 2.0}, True),
        ("donuk", TERRAIN / "donuk.tif", {"interval": 3.0, "base": 0.0, "indexEvery": 5, "simplify": 0.0}, True),
        ("int16", TERRAIN / "int16.tif", {"interval": 50.0, "base": 0.0, "indexEvery": 5, "simplify": 0.0}, True),
        ("genis", TERRAIN / "genis.tif", {"interval": 10.0, "base": 0.0, "indexEvery": 5, "simplify": 0.0}, False),
    ]
    for name, path, spec, full in specs:
        z, gt, nodata = read(path)
        lines = contours(z, gt, nodata, spec["interval"], spec["base"], spec["indexEvery"], spec["simplify"])
        case = {"dem": name, "file": str(path.relative_to(ROOT / "fixtures")), "spec": {"tool": {"kind": "contours", **spec}, "band": 1, "affine": gt, "epsg": 5254}}
        case["texts"] = texts(lines, spec)
        if full:
            case["lines"] = lines
        else:
            per = {}
            for ln in lines:
                s = per.setdefault(ln["k"], {"k": ln["k"], "lines": 0, "vertices": 0, "length": 0.0})
                s["lines"] += 1
                s["vertices"] += len(ln["pts"])
                s["length"] += length(ln["pts"])
            case["levels"] = sorted(per.values(), key=lambda s: s["k"])
            case["first"] = lines[:3]
            # gdal_contour's totals, level by level, on the DEM without its nodata band (gdal_contour
            # runs a line on along a hole's edge; the ADR ends it there): the squares, the crossings
            # and the chains held to GDAL's.
            problems += cross_check(name, z, gt, spec)
        cases.append(case)
    return {"format": "kentos.contour-cases", "version": 1, "cases": cases}, problems


def main():
    check = "--check" in sys.argv[1:]
    (ROOT / ".run").mkdir(exist_ok=True)
    eyer_path = DIR / "eyer.tif"
    problems = []
    if check:
        if not eyer_path.exists():
            problems.append("fixtures/contours/v1/eyer.tif yok (betiği --check olmadan çalıştırın)")
        elif not np.array_equal(read(eyer_path)[0], eyer().astype(np.float64)):
            problems.append("fixtures/contours/v1/eyer.tif tanımlı diziyi vermiyor")
    else:
        DIR.mkdir(parents=True, exist_ok=True)
        write_eyer(eyer_path)
    doc, cross = build()
    problems += cross
    text = json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n"
    if check:
        if not CASES.exists() or CASES.read_text("utf-8") != text:
            problems.append(f"{CASES.relative_to(ROOT)} yeniden kurulanla aynı değil (betiği --check olmadan çalıştırın)")
    else:
        CASES.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print(f"{CASES.relative_to(ROOT)}: {len(doc['cases'])} durum; gdal_contour'la çapraz denetim geçti")


if __name__ == "__main__":
    main()
