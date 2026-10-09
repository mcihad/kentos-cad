#!/usr/bin/env python3
"""Yüzey analizinin bağımsız başvurusu (docs/adr/0231 §2–§8).

    python3 scripts/fixtures/terrain_cases.py           # DEM'leri ve durumları yazar
    python3 scripts/fixtures/terrain_cases.py --check   # hiçbir şey yazmaz; karşılaştırır

KentOS kodu kullanılmaz. Örnek DEM'ler numpy ile tanımlanır ve GDAL'la GeoTIFF olarak
yazılır (fixtures/terrain/v1/*.tif: düz, dönük, coğrafi, int16 şeritli, çok şeritli). Beklenen
sonuçlar ADR'nin tanımlarıyla, ADR'nin işlem sırasıyla Python'un float'larında (aşkın işlevler
Python'un math'i), f32'ye yuvarlanarak hesaplanır. Tanımların kendisi gdaldem'le çapraz denetlenir:
eğim (Horn ve Zevenbergen-Thorne), bakı, gölgeli kabartma, TRI (Riley, Wilson), TPI, engebe ve
renkli kabartma iç hücrelerde gdaldem'in verdiğiyle aynı olmalı (gölgeli kabartmada en çok bir adım).
Güneşlenmenin satır enlemi pyproj'dan (sistemin kendi coğrafi sistemi).

--check durum dosyasını bellekte yeniden kurar ve diskteki fixtures/terrain/v1/cases.json'la
karşılaştırır; GeoTIFF'lerin GDAL'la okununca tanımlı dizileri verdiğini de denetler.
"""

import json
import math
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from osgeo import gdal, osr
import pyproj

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/terrain/v1"
CASES = DIR / "cases.json"

NODATA = -9999.0

TUREF_TM30 = {"kind": "tm", "datum": "TUREF", "centralMeridian": 30, "scaleFactor": 1, "falseEasting": 500000, "falseNorthing": 0}
WGS84 = {"kind": "geographic", "datum": "WGS84"}


# ── The DEMs ─────────────────────────────────────────────────────────────

def surface(w, h, scale=1.0):
    """A hill on a tilted plane with ripples: values every cell differs (no ties with levels)."""
    j, i = np.mgrid[0:h, 0:w].astype(np.float64)
    z = (120.0 + 35.0 * np.exp(-((i - w * 0.45) ** 2 + (j - h * 0.42) ** 2) / (0.05 * w * h))
         + 0.8 * i - 0.5 * j + 2.0 * np.sin(i / 3.1) * np.cos(j / 4.3)) * scale
    return z


def dem_tepe():
    z = surface(41, 33)
    z[20:24, 6:10] = 150.0  # a flat patch: slope 0, aspect −1
    z[5:8, 30:33] = NODATA  # a hole
    z[25, 5] = np.nan
    return z.astype(np.float32)


def dem_donuk():
    return surface(23, 19).astype(np.float32)


def dem_cografi():
    z = surface(25, 21)
    z[10, 12] = NODATA
    return z.astype(np.float32)


def dem_int16():
    z = np.floor(surface(30, 26) * 10.0) - 1000.0
    z[0:2, 0:3] = -32768
    return z.astype(np.int16)


def dem_genis():
    j, i = np.mgrid[0:530, 0:600].astype(np.float64)
    z = 300.0 + 80.0 * np.sin(i / 47.0) * np.cos(j / 61.0) + 0.15 * i + 25.0 * np.exp(-((i - 410) ** 2 + (j - 180) ** 2) / 4000.0)
    z[300:304, 100:420] = NODATA  # a band of holes across the strips' seam
    return z.astype(np.float32)


DEMS = [
    # name, array, geotransform [x0, a, b, y0, c, d], EPSG, nodata, system, creation options
    ("tepe", dem_tepe, [500100.0, 10.0, 0.0, 4420330.0, 0.0, -10.0], 5254, NODATA, TUREF_TM30, ["TILED=YES", "BLOCKXSIZE=16", "BLOCKYSIZE=16", "COMPRESS=DEFLATE"]),
    ("donuk", dem_donuk, [501000.0, 4.330127018922193, 2.5, 4421000.0, 2.5, -4.330127018922193], 5254, None, TUREF_TM30, ["COMPRESS=LZW"]),
    ("cografi", dem_cografi, [32.8, 0.0005, 0.0, 39.95, 0.0, -0.0005], 4326, NODATA, WGS84, ["TILED=YES", "BLOCKXSIZE=16", "BLOCKYSIZE=16"]),
    ("int16", dem_int16, [400000.0, 2.0, 0.0, 4100000.0, 0.0, -2.0], 5254, -32768.0, TUREF_TM30, ["BLOCKYSIZE=7"]),
    ("genis", dem_genis, [502000.0, 5.0, 0.0, 4425000.0, 0.0, -5.0], 5254, NODATA, TUREF_TM30, ["TILED=YES", "COMPRESS=DEFLATE", "PREDICTOR=3"]),
]


def write_tif(path, arr, gt, epsg, nodata, options):
    kind = gdal.GDT_Int16 if arr.dtype == np.int16 else gdal.GDT_Float32
    ds = gdal.GetDriverByName("GTiff").Create(str(path), arr.shape[1], arr.shape[0], 1, kind, options=options)
    ds.SetGeoTransform(gt)
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(epsg)
    ds.SetProjection(srs.ExportToWkt())
    band = ds.GetRasterBand(1)
    if nodata is not None:
        band.SetNoDataValue(nodata)
    band.WriteArray(arr)
    ds.FlushCache()
    ds = None


def read_tif(path):
    ds = gdal.Open(str(path))
    return ds.GetRasterBand(1).ReadAsArray()


# ── The ADR's rules ──────────────────────────────────────────────────────

GRS80_A = 6378137.0
GRS80_F = 1.0 / 298.257222101


def degree_metres(phi):
    e2 = GRS80_F * (2.0 - GRS80_F)
    r = phi * math.pi / 180.0
    s = math.sin(r)
    w = 1.0 - e2 * s * s
    n = GRS80_A / math.sqrt(w)
    m = GRS80_A * (1.0 - e2) / (w * math.sqrt(w))
    k = math.pi / 180.0
    return n * math.cos(r) * k, m * k


def axes(gt, geographic, j, width):
    """A row's pixel axes in metres (§2): [a, b, c, d]."""
    x0, a, b, y0, c, d = gt
    if not geographic:
        return a, b, c, d
    phi = y0 + c * (width / 2.0) + d * (j + 0.5)
    ke, kn = degree_metres(phi)
    return a * ke, b * ke, c * kn, d * kn


def gradient_k(ax, z):
    a, b, c, d = ax
    det = a * d - b * c
    return z * d / det, -z * c / det, -z * b / det, z * a / det


def heights(arr, nodata):
    """The band as float64, nodata and NaN as NaN, padded by its edge (§2)."""
    v = arr.astype(np.float64)
    if nodata is not None:
        v[v == nodata] = np.nan
    return np.pad(v, 1, mode="edge")


def windows(p, i, j):
    """z1 … z9 of cell (i, j) of the padded heights; none when the centre is nodata."""
    w = [p[j + dj, i + di] for dj in (0, 1, 2) for di in (0, 1, 2)]
    c = w[4]
    if math.isnan(c):
        return None
    return [c if math.isnan(x) else float(x) for x in w]


def derivatives(w, method):
    if method == "horn":
        return (((w[2] + 2.0 * w[5] + w[8]) - (w[0] + 2.0 * w[3] + w[6])) / 8.0,
                ((w[6] + 2.0 * w[7] + w[8]) - (w[0] + 2.0 * w[1] + w[2])) / 8.0)
    return (w[5] - w[3]) / 2.0, (w[7] - w[1]) / 2.0


def f32(v):
    """The 32-bit value, written in the fewest digits that give it back."""
    return float(str(np.float32(v)))


def kernel_grid(dem, fn):
    """fn(window, row j) for every cell; NaN where the centre is nodata."""
    arr, gt, geographic, nodata = dem["array"], dem["gt"], dem["geographic"], dem["nodata"]
    h, w = arr.shape
    p = heights(arr, nodata)
    out = np.full((h, w), np.nan)
    for j in range(h):
        for i in range(w):
            win = windows(p, i, j)
            if win is not None:
                out[j, i] = fn(win, j)
    return out


def slope_of(dem, method, percent, z):
    def fn(win, j):
        k1, k2, k3, k4 = gradient_k(axes(dem["gt"], dem["geographic"], j, dem["array"].shape[1]), z)
        pi, pj = derivatives(win, method)
        gx, gy = k1 * pi + k2 * pj, k3 * pi + k4 * pj
        m = math.sqrt(gx * gx + gy * gy)
        return 100.0 * m if percent else math.atan(m) * (180.0 / math.pi)
    return kernel_grid(dem, fn)


def aspect_of(dem, method, z):
    def fn(win, j):
        k1, k2, k3, k4 = gradient_k(axes(dem["gt"], dem["geographic"], j, dem["array"].shape[1]), z)
        pi, pj = derivatives(win, method)
        gx, gy = k1 * pi + k2 * pj, k3 * pi + k4 * pj
        if gx == 0.0 and gy == 0.0:
            return -1.0
        a = math.atan2(-gx, -gy) * (180.0 / math.pi)
        return (a + 360.0 if a < 0.0 else a) + 0.0
    return kernel_grid(dem, fn)


def hillshade_of(dem, azimuth, altitude, z):
    rad = math.pi / 180.0
    az, alt = azimuth * rad, altitude * rad
    sin_alt, cs, cc = math.sin(alt), math.cos(alt) * math.sin(az), math.cos(alt) * math.cos(az)

    def fn(win, j):
        k1, k2, k3, k4 = gradient_k(axes(dem["gt"], dem["geographic"], j, dem["array"].shape[1]), z)
        pi, pj = derivatives(win, "horn")
        gx, gy = k1 * pi + k2 * pj, k3 * pi + k4 * pj
        c = (sin_alt - (gx * cs + gy * cc)) / math.sqrt(1.0 + gx * gx + gy * gy)
        v = 1.0 if c <= 0.0 else 1.0 + 254.0 * c
        return math.floor(v + 0.5)
    g = kernel_grid(dem, fn)
    return np.where(np.isnan(g), 0, g)


def curvature_of(dem, kind, z):
    def fn(win, j):
        a, b, c, d = axes(dem["gt"], dem["geographic"], j, dem["array"].shape[1])
        lx, ly = math.sqrt(a * a + c * c), math.sqrt(b * b + d * d)
        z1, z2, z3, z4, z5, z6, z7, z8, z9 = [v * z for v in win]
        D = ((z4 + z6) / 2.0 - z5) / (lx * lx)
        E = ((z2 + z8) / 2.0 - z5) / (ly * ly)
        if kind == "total":
            return -200.0 * (D + E)
        F = (z3 - z1 + z7 - z9) / (4.0 * lx * ly)
        G = (z6 - z4) / (2.0 * lx)
        H = (z2 - z8) / (2.0 * ly)
        gh = G * G + H * H
        if gh == 0.0:
            return 0.0
        if kind == "profile":
            return 200.0 * (D * G * G + E * H * H + F * G * H) / gh
        return -200.0 * (D * H * H + E * G * G - F * G * H) / gh
    return kernel_grid(dem, fn)


def ruggedness_of(dem, index):
    def fn(win, j):
        c = win[4]
        n = win[:4] + win[5:]
        if index == "triRiley":
            s = 0.0
            for v in n:
                s += (v - c) * (v - c)
            return math.sqrt(s)
        if index == "triWilson":
            s = 0.0
            for v in n:
                s += abs(v - c)
            return s / 8.0
        if index == "tpi":
            s = 0.0
            for v in n:
                s += v
            return c - s / 8.0
        return max(win) - min(win)
    return kernel_grid(dem, fn)


# ADR 0204's ramps (docs/adr/0204 §4): the same table as its independent reference's
# (scripts/fixtures/raster_cases.py), not KentOS's.
RAMP_TABLE = {
    "Gri": [(0, 0, 0), (255, 255, 255)],
    "Arazi": [(0x2E, 0x7D, 0x32), (0x9C, 0xCC, 0x65), (0xFF, 0xF5, 0x9D), (0xA1, 0x88, 0x7F), (0xFA, 0xFA, 0xFA)],
    "Spektral": [(0x2B, 0x83, 0xBA), (0xAB, 0xDD, 0xA4), (0xFF, 0xFF, 0xBF), (0xFD, 0xAE, 0x61), (0xD7, 0x19, 0x1C)],
    "Viridis": [(0x44, 0x01, 0x54), (0x3B, 0x52, 0x8B), (0x21, 0x91, 0x8C), (0x5E, 0xC9, 0x62), (0xFD, 0xE7, 0x25)],
}


def table_parse(text):
    rows = []
    # Rows by line or by ";" (ADR 0231 §5: a one-line field writes "100 #2E7D32; 500 #FFF59D").
    for line in text.replace(";", "\n").splitlines():
        line = line.strip()
        if not line:
            continue
        v, c = line.split()
        c = c[1:]
        rgba = [int(c[k:k + 2], 16) for k in (0, 2, 4)] + [int(c[6:8], 16) if len(c) == 8 else 255]
        rows.append((float(v), rgba))
    rows.sort(key=lambda r: r[0])  # stable: the first of equal values stays first
    out = []
    for r in rows:
        if out and out[-1][0] == r[0]:
            continue
        out.append(r)
    return out


def table_ramp(stops, invert, lo, hi):
    colors = [list(s) + [255] for s in stops]
    if invert:
        colors.reverse()
    if not hi > lo:
        return [(lo, colors[0]), (lo, colors[0])]
    n = len(colors) - 1
    return [(lo + (hi - lo) * k / n, colors[k]) for k in range(n + 1)]


def color_of(table, v, interp):
    if v <= table[0][0]:
        return table[0][1]
    if v >= table[-1][0]:
        return table[-1][1]
    k = max(i for i in range(len(table)) if table[i][0] <= v)
    (a, ca), (b, cb) = table[k], table[k + 1]
    if interp == "nearest":
        return ca if v - a <= b - v else cb
    t = (v - a) / (b - a)
    return [min(255, max(0, math.floor(ca[c] + t * (cb[c] - ca[c]) + 0.5))) for c in range(4)]


def relief_of(dem, spec):
    arr, nodata = dem["array"], dem["nodata"]
    v = arr.astype(np.float64)
    if nodata is not None:
        v[v == nodata] = np.nan
    if spec.get("table"):
        table = table_parse(spec["table"])
    else:
        lo, hi = spec.get("min"), spec.get("max")
        if lo is None or hi is None:
            lo, hi = float(np.nanmin(v)), float(np.nanmax(v))
        table = table_ramp(RAMP_TABLE[spec["ramp"]], spec.get("invert", False), lo, hi)
    h, w = v.shape
    out = np.zeros((h, w, 4), dtype=np.uint8)
    for j in range(h):
        for i in range(w):
            if not math.isnan(v[j, i]):
                out[j, i] = color_of(table, float(v[j, i]), spec["interp"])
    return out


# ── Güneşlenme (§8) ──────────────────────────────────────────────────────

MONTHS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]


def day_blocks(first, last, step):
    out, start = [], first
    while start <= last:
        end = min(start + step - 1, last)
        out.append((start + (end - start) // 2, float(end - start + 1)))
        start = end + 1
    return out


def suns(phi, days, hour_step, tau):
    rad = math.pi / 180.0
    sp, cp = math.sin(phi * rad), math.cos(phi * rad)
    out = []
    for n, weight in days:
        g = 2.0 * math.pi * (n - 1) / 365.0
        decl = (0.006918 - 0.399912 * math.cos(g) + 0.070257 * math.sin(g) - 0.006758 * math.cos(2.0 * g)
                + 0.000907 * math.sin(2.0 * g) - 0.002697 * math.cos(3.0 * g) + 0.00148 * math.sin(3.0 * g))
        e0 = 1.000110 + 0.034221 * math.cos(g) + 0.001280 * math.sin(g) + 0.000719 * math.cos(2.0 * g) + 0.000077 * math.sin(2.0 * g)
        sd, cd = math.sin(decl), math.cos(decl)
        for k in range(round(24.0 / hour_step)):
            t = (k + 0.5) * hour_step
            w = 15.0 * (t - 12.0) * rad
            sw, cw = math.sin(w), math.cos(w)
            up = sp * sd + cp * cd * cw
            if up <= 0.0:
                continue
            east = -cd * sw
            north = sd * cp - cd * sp * cw
            hdeg = math.asin(up) / rad
            m = 1.0 / (up + 0.50572 * math.pow(hdeg + 6.07995, -1.6364))
            k_sun = 1367.0 * e0 * math.pow(tau, m) * hour_step * weight / 1000.0
            out.append((east, north, up, k_sun))
    return out


def row_latitudes(dem):
    arr, gt = dem["array"], dem["gt"]
    h, w = arr.shape
    x0, a, b, y0, c, d = gt
    pts = [(x0 + a * (w / 2.0) + b * (j + 0.5), y0 + c * (w / 2.0) + d * (j + 0.5)) for j in range(h)]
    if dem["geographic"]:
        return [y for _, y in pts]
    # The system's own geographic system (TUREF / TM30 → TUREF).
    tr = pyproj.Transformer.from_crs(dem["epsg"], 5252, always_xy=True)
    return [tr.transform(x, y)[1] for x, y in pts]


def insolation_of(dem, spec):
    days = day_blocks(spec["firstDay"], spec["lastDay"], spec["dayStep"])
    lats = row_latitudes(dem)
    rows = {}

    def fn(win, j):
        if j not in rows:
            rows[j] = suns(lats[j], days, spec["hourStep"], spec["transmissivity"])
        k1, k2, k3, k4 = gradient_k(axes(dem["gt"], dem["geographic"], j, dem["array"].shape[1]), spec["zFactor"])
        pi, pj = derivatives(win, "horn")
        gx, gy = k1 * pi + k2 * pj, k3 * pi + k4 * pj
        s = 0.0
        for e, nn, u, k in rows[j]:
            dot = -gx * e - gy * nn + u
            if dot > 0.0:
                s += k * dot
        return s / math.sqrt(1.0 + gx * gx + gy * gy)
    return kernel_grid(dem, fn)


# ── gdaldem cross-checks ─────────────────────────────────────────────────

def gdaldem(mode, src, *args, color_text=None):
    with tempfile.TemporaryDirectory(dir=ROOT / ".run") as tmp:
        out = Path(tmp) / "out.tif"
        cmd = ["gdaldem", mode, str(src)]
        if color_text is not None:
            ct = Path(tmp) / "colors.txt"
            ct.write_text(color_text)
            cmd.append(str(ct))
        cmd += [str(out), "-q", *args]
        subprocess.run(cmd, check=True)
        ds = gdal.Open(str(out))
        return np.stack([ds.GetRasterBand(k + 1).ReadAsArray() for k in range(ds.RasterCount)], axis=-1).squeeze()


def interior(arr, dem):
    """The cells whose whole window is inside the raster and has no nodata."""
    v = dem["array"].astype(np.float64)
    if dem["nodata"] is not None:
        v[v == dem["nodata"]] = np.nan
    ok = ~np.isnan(v)
    m = np.zeros_like(ok)
    m[1:-1, 1:-1] = ok[1:-1, 1:-1]
    for dj in (-1, 0, 1):
        for di in (-1, 0, 1):
            m[1:-1, 1:-1] &= ok[1 + dj:v.shape[0] - 1 + dj, 1 + di:v.shape[1] - 1 + di]
    return m


def cross_check(dem, path):
    problems = []
    inside = interior(None, dem)

    def same(name, mine, theirs, tol):
        d = np.abs(mine[inside] - theirs[inside].astype(np.float64))
        bad = d > tol
        if bad.any():
            problems.append(f"{dem['name']} {name}: gdaldem'den {int(bad.sum())} iç hücrede farklı (en çok {float(d.max()):.3g})")

    # gdaldem works in 32-bit floats: on a nearly flat cell its angle wanders (an aspect most), so
    # the definitions are held to a tolerance a wrong rule would pass by far, the aspect where the
    # slope is over 1°.
    for method, flag in (("horn", []), ("zevenbergenThorne", ["-alg", "ZevenbergenThorne"])):
        slope = slope_of(dem, method, False, 1.0)
        same(f"eğim {method}", slope, gdaldem("slope", path, *flag), 1e-3)
        same(f"eğim yüzde {method}", slope_of(dem, method, True, 1.0), gdaldem("slope", path, "-p", *flag), 1e-3)
        mine = aspect_of(dem, method, 1.0)
        theirs = gdaldem("aspect", path, *flag)
        steep = slope > 1.0
        mine = np.where(steep, mine, theirs)
        same(f"bakı {method}", mine, theirs, 5e-2)
    same("gölgeli kabartma", hillshade_of(dem, 315.0, 45.0, 1.0), gdaldem("hillshade", path), 1.0)
    same("gölgeli kabartma 135/30/2", hillshade_of(dem, 135.0, 30.0, 2.0), gdaldem("hillshade", path, "-az", "135", "-alt", "30", "-z", "2"), 1.0)
    same("TRI Riley", ruggedness_of(dem, "triRiley"), gdaldem("TRI", path, "-alg", "Riley"), 1e-3)
    same("TRI Wilson", ruggedness_of(dem, "triWilson"), gdaldem("TRI", path, "-alg", "Wilson"), 1e-3)
    same("TPI", ruggedness_of(dem, "tpi"), gdaldem("TPI", path), 1e-3)
    same("engebe", ruggedness_of(dem, "roughness"), gdaldem("roughness", path), 1e-3)
    text = "100 #2E7D32\n140 #FFF59D\n180 #A1887F\n"
    # gdaldem's colour file names a colour by its channels.
    theirs_text = "100 46 125 50 255\n140 255 245 157 255\n180 161 136 127 255\n"
    mine = relief_of(dem, {"table": text, "interp": "linear"}).astype(np.float64)
    theirs = gdaldem("color-relief", path, "-alpha", color_text=theirs_text).astype(np.float64)
    for c in range(4):
        d = np.abs(mine[..., c][inside] - theirs[..., c][inside])
        if (d > 1.0).any():
            problems.append(f"{dem['name']} renkli kabartma kanal {c}: gdaldem'den {int((d > 1).sum())} hücrede farklı")
    return problems


# ── Cases ────────────────────────────────────────────────────────────────

def numbers(grid):
    return [None if math.isnan(v) else f32(v) for v in grid.ravel()]


def probe(grid, every=97):
    h, w = grid.shape[:2]
    out = []
    for j in range(h):
        for i in range(w):
            if (i * 31 + j * 17) % every == 0:
                v = grid[j, i]
                out.append([i, j, None if isinstance(v, float) and math.isnan(v) else (f32(v) if isinstance(v, float) else v)])
    return out


def build():
    dems = []
    for name, make, gt, epsg, nodata, system, options in DEMS:
        arr = make()
        dems.append({"name": name, "array": arr, "gt": gt, "epsg": epsg, "nodata": nodata, "system": system,
                     "geographic": system["kind"] == "geographic", "options": options})
    cases = []

    def add(dem, title, tool, grid, sample="f32", bands=1, full=True):
        spec = {"tool": tool, "band": 1, "affine": dem["gt"], "epsg": dem["epsg"], "system": dem["system"]}
        case = {"dem": dem["name"], "name": title, "spec": spec, "sample": sample, "bands": bands}
        if sample == "f32":
            vals = grid
            if full:
                case["values"] = numbers(vals)
            else:
                case["probe"] = [[i, j, None if math.isnan(v) else f32(v)] for i, j, v in
                                 [(i, j, float(vals[j, i])) for j in range(vals.shape[0]) for i in range(vals.shape[1]) if (i * 31 + j * 17) % 97 == 0]]
        else:
            flat = grid.reshape(-1).astype(int).tolist() if full else None
            if full:
                case["values"] = flat
            else:
                case["probe"] = [[i, j, grid[j, i].astype(int).tolist() if bands > 1 else int(grid[j, i])] for j in range(grid.shape[0]) for i in range(grid.shape[1]) if (i * 31 + j * 17) % 97 == 0]
        cases.append(case)

    # Each DEM what it tests: tepe every tool; int16 the sample type and its nodata; donuk the
    # turned affine (J⁻ᵀ, curvature's sides, insolation's rows); cografi the degrees; genis
    # (probes) the strips' seams and the reduced levels.
    every = {"tepe": None,
             "int16": {"slope", "aspect", "hillshade", "tri", "relief-table"},
             "donuk": {"slope", "slope-zt", "aspect", "hillshade", "curvature", "insolation-year"},
             "cografi": {"slope", "aspect", "hillshade", "curvature-total", "insolation-year"},
             "genis": {"slope", "aspect", "hillshade", "curvature-profile", "tri"}}
    for dem in dems:
        full = dem["name"] != "genis"
        wants = every[dem["name"]]

        def want(*keys):
            return wants is None or any(k in wants for k in keys)

        if want("slope"):
            add(dem, "Eğim (derece, Horn)", {"kind": "slope", "method": "horn", "unit": "degrees", "zFactor": 1.0}, slope_of(dem, "horn", False, 1.0), full=full)
        if want("slope-zt"):
            add(dem, "Eğim (yüzde, Zevenbergen-Thorne, z 1,5)", {"kind": "slope", "method": "zevenbergenThorne", "unit": "percent", "zFactor": 1.5},
                slope_of(dem, "zevenbergenThorne", True, 1.5), full=full)
        if want("aspect"):
            add(dem, "Bakı (Horn)", {"kind": "aspect", "method": "horn", "zFactor": 1.0}, aspect_of(dem, "horn", 1.0), full=full)
        if want("aspect-zt"):
            add(dem, "Bakı (Zevenbergen-Thorne)", {"kind": "aspect", "method": "zevenbergenThorne", "zFactor": 1.0}, aspect_of(dem, "zevenbergenThorne", 1.0), full=full)
        if want("hillshade"):
            add(dem, "Gölgeli kabartma (315, 45, 1)", {"kind": "hillshade", "azimuth": 315.0, "altitude": 45.0, "zFactor": 1.0},
                hillshade_of(dem, 315.0, 45.0, 1.0).astype(np.uint8), sample="u8", full=full)
        if want("hillshade-2"):
            add(dem, "Gölgeli kabartma (135, 30, 2)", {"kind": "hillshade", "azimuth": 135.0, "altitude": 30.0, "zFactor": 2.0},
                hillshade_of(dem, 135.0, 30.0, 2.0).astype(np.uint8), sample="u8", full=full)
        if want("curvature", "curvature-profile"):
            add(dem, "Eğrilik (profil)", {"kind": "curvature", "curvature": "profile", "zFactor": 1.0}, curvature_of(dem, "profile", 1.0), full=full)
        if want("curvature", "curvature-total"):
            add(dem, "Eğrilik (toplam, z 2)", {"kind": "curvature", "curvature": "total", "zFactor": 2.0}, curvature_of(dem, "total", 2.0), full=full)
        if want("curvature"):
            add(dem, "Eğrilik (plan)", {"kind": "curvature", "curvature": "plan", "zFactor": 1.0}, curvature_of(dem, "plan", 1.0), full=full)
        if want("tri"):
            add(dem, "TRI (Riley)", {"kind": "ruggedness", "index": "triRiley"}, ruggedness_of(dem, "triRiley"), full=full)
        if want("ruggedness"):
            add(dem, "TRI (Wilson)", {"kind": "ruggedness", "index": "triWilson"}, ruggedness_of(dem, "triWilson"), full=full)
            add(dem, "TPI", {"kind": "ruggedness", "index": "tpi"}, ruggedness_of(dem, "tpi"), full=full)
            add(dem, "Engebe", {"kind": "ruggedness", "index": "roughness"}, ruggedness_of(dem, "roughness"), full=full)
        for key, title, rspec in [
            ("relief", "Renkli kabartma (Arazi, en küçük–en büyük)", {"ramp": "Arazi", "invert": False, "interp": "linear"}),
            ("relief", "Renkli kabartma (Viridis ters, 100–200, en yakın)", {"ramp": "Viridis", "invert": True, "min": 100.0, "max": 200.0, "interp": "nearest"}),
            ("relief-table", "Renkli kabartma (tablo, alfa)", {"table": "180 #A1887F\n100 #2E7D32\n140 #FFF59D80\n140 #000000\n", "interp": "linear"}),
            ("relief", "Renkli kabartma (tablo, en yakın)", {"table": "100 #2E7D32; 140 #FFF59D;180 #A1887F", "interp": "nearest"}),
        ]:
            if want(key):
                add(dem, title, {"kind": "colorRelief", **rspec}, relief_of(dem, rspec), sample="u8", bands=4, full=full)
        for key, title, ispec in [
            ("insolation-year", "Güneşlenme (yıl, 14 gün, 0,5 saat, 0,5)", {"firstDay": 1, "lastDay": 365, "dayStep": 14, "hourStep": 0.5, "transmissivity": 0.5, "zFactor": 1.0}),
            ("insolation", "Güneşlenme (1 Haziran–31 Ağustos, 7 gün, 1 saat, 0,6)", {"firstDay": 152, "lastDay": 243, "dayStep": 7, "hourStep": 1.0, "transmissivity": 0.6, "zFactor": 1.0}),
            ("insolation", "Güneşlenme (21 Aralık, 0,25 saat)", {"firstDay": 355, "lastDay": 355, "dayStep": 1, "hourStep": 0.25, "transmissivity": 0.5, "zFactor": 2.0}),
        ]:
            if want(key, "insolation"):
                add(dem, title, {"kind": "insolation", **ispec}, insolation_of(dem, ispec), full=full)
    return dems, {"format": "kentos.terrain-cases", "version": 1,
                  "dems": [{"name": d["name"], "file": f"{d['name']}.tif", "width": int(d["array"].shape[1]), "height": int(d["array"].shape[0]),
                            "affine": d["gt"], "epsg": d["epsg"], "nodata": d["nodata"], "system": d["system"]} for d in dems],
                  "cases": cases}


def main():
    check = "--check" in sys.argv[1:]
    (ROOT / ".run").mkdir(exist_ok=True)
    dems, doc = build()
    problems = []
    for d in dems:
        path = DIR / f"{d['name']}.tif"
        if check:
            if not path.exists():
                problems.append(f"{path.relative_to(ROOT)} yok (betiği --check olmadan çalıştırın)")
                continue
            back = read_tif(path)
            if back.dtype != d["array"].dtype or not np.array_equal(back, d["array"], equal_nan=True):
                problems.append(f"{path.relative_to(ROOT)} tanımlı diziyi vermiyor")
        else:
            DIR.mkdir(parents=True, exist_ok=True)
            write_tif(path, d["array"], d["gt"], d["epsg"], d["nodata"], d["options"])
        # gdaldem reads a north-up projected raster only as the ADR does (a turned one it warps
        # first; a geographic one takes one scale for the whole raster).
        if d["name"] in ("tepe", "int16", "genis"):
            problems += cross_check(d, path)
    text = json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n"
    if check:
        if not CASES.exists() or CASES.read_text("utf-8") != text:
            problems.append(f"{CASES.relative_to(ROOT)} yeniden kurulanla aynı değil (betiği --check olmadan çalıştırın)")
    else:
        CASES.write_text(text, "utf-8")
    if problems:
        print("\n".join(problems), file=sys.stderr)
        sys.exit(1)
    print(f"{CASES.relative_to(ROOT)}: {len(doc['cases'])} durum, {len(dems)} DEM; gdaldem'le çapraz denetim geçti")


if __name__ == "__main__":
    main()
