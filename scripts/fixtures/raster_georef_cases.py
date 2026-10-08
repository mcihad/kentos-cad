#!/usr/bin/env python3
"""Raster oturt (docs/adr/0204 §6): the transforms and the resampling, written without KentOS code.

fixtures/raster/v1/georef.json:
- control point sets (pixels of a scanned sheet, targets in TUREF metres) and, for each method, every point's residual,
  m0, where probe pixels land and which pixels probe points come from. The affine and the polynomials of order 2 and 3
  and the thin plate are GDAL's GCP transformer forward (METHOD=GCP_POLYNOMIAL with MAX_GCP_ORDER 1–3,
  METHOD=GCP_TPS); Helmert's least squares and the projective's linearised least squares in the points' centred
  frames scaled by their largest coordinate (docs/adr/0156 §2) are numpy here. The inverse is the forward's: closed
  for Helmert, the affine and the projective; for the polynomials and the thin plate Newton's method on GDAL's forward
  (a central-difference derivative) from GDAL's reverse (fitted apart; GDAL refines the thin plate's itself, not the
  polynomials').
- a resampled output: fixtures/raster/v1/files/gray-lzw-strips.tif carried by a thin plate onto a north-up grid,
  nearest neighbour, every output pixel's centre brought back by the forward's inverse here; the same grid by gdalwarp
  (exact transformer, -et 0, with an alpha band) must be the same sample for sample. Given as an FNV-1a hash of each
  band and the grid. KentOS's output is RGBA (grey spread to red, green and blue).

    python3 scripts/fixtures/raster_georef_cases.py          # write
    python3 scripts/fixtures/raster_georef_cases.py --check  # compare
"""

import argparse
import json
import math
import sys
import tempfile
from pathlib import Path

import numpy as np
from osgeo import gdal, osr

gdal.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "raster" / "v1"
OUT = DIR / "georef.json"


def fnv(data: bytes) -> str:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def points(kind):
    """Control points of a sheet 2000 × 1500 pixels: where a known distortion carries them, rounded as one clicks."""
    out = []
    grid = [(c, r) for r in (60, 520, 980, 1440) for c in (80, 700, 1320, 1940)]
    for n, (c, r) in enumerate(grid):
        u, v = (c - 1000) / 1000, (r - 750) / 750
        x = 487200 + 0.5 * c + 0.03 * r + (12 * u * v + 7 * u * u - 5 * v * v * v if kind == "bent" else 0.0)
        y = 4420800 - 0.5 * r + 0.02 * c + (9 * u * u * v - 6 * v * v + 4 * u * u * u if kind == "bent" else 0.0)
        # A few centimetres of clicking.
        jx = ((n * 37) % 11 - 5) * 0.004
        jy = ((n * 53) % 13 - 6) * 0.004
        out.append({"pixel": [float(c) + 0.5, float(r) + 0.5], "target": [round(x + jx, 3), round(y + jy, 3)], "used": n != 5})
    return out


def mem_with_gcps(gcps):
    ds = gdal.GetDriverByName("MEM").Create("", 2000, 1500, 1, gdal.GDT_Byte)
    sr = osr.SpatialReference()
    sr.ImportFromEPSG(5256)
    ds.SetGCPs([gdal.GCP(p["target"][0], p["target"][1], 0.0, p["pixel"][0], p["pixel"][1]) for p in gcps if p["used"]], sr.ExportToWkt())
    return ds


def gdal_transformer(gcps, method):
    ds = mem_with_gcps(gcps)
    opts = {"affine": ["METHOD=GCP_POLYNOMIAL", "MAX_GCP_ORDER=1"], "poly2": ["METHOD=GCP_POLYNOMIAL", "MAX_GCP_ORDER=2"],
            "poly3": ["METHOD=GCP_POLYNOMIAL", "MAX_GCP_ORDER=3"], "thinPlate": ["METHOD=GCP_TPS"]}[method]
    return ds, gdal.Transformer(ds, None, opts)


def centred(gcps):
    used = [p for p in gcps if p["used"]]
    s0, t0 = used[0]["pixel"], used[0]["target"]
    n = len(used)
    ds = [((p["pixel"][0] - s0[0], p["pixel"][1] - s0[1]), (p["target"][0] - t0[0], p["target"][1] - t0[1])) for p in used]
    ms = (sum(d[0][0] for d in ds) / n, sum(d[0][1] for d in ds) / n)
    mt = (sum(d[1][0] for d in ds) / n, sum(d[1][1] for d in ds) / n)
    src = [(d[0][0] - ms[0], d[0][1] - ms[1]) for d in ds]
    dst = [(d[1][0] - mt[0], d[1][1] - mt[1]) for d in ds]
    frm = (s0[0] + ms[0], s0[1] + ms[1])
    to = (t0[0] + mt[0], t0[1] + mt[1])
    return frm, to, src, dst


def helmert(gcps):
    """A similarity from (column, −row): the raster's rows go down, the drawing's north up (a mirror)."""
    flipped = [{**p, "pixel": [p["pixel"][0], -p["pixel"][1]]} for p in gcps]
    frm, to, src, dst = centred(flipped)
    s = sum(x * x + y * y for x, y in src)
    pa = sum(x * tx + y * ty for (x, y), (tx, ty) in zip(src, dst))
    pb = sum(x * ty - y * tx for (x, y), (tx, ty) in zip(src, dst))
    a, b = pa / s, pb / s
    fwd = lambda c, r: (to[0] + a * (c - frm[0]) - b * (-r - frm[1]), to[1] + b * (c - frm[0]) + a * (-r - frm[1]))

    def inv(x, y):
        tx, ty = x - to[0], y - to[1]
        det = a * a + b * b
        return (frm[0] + (a * tx + b * ty) / det, -(frm[1] + (a * ty - b * tx) / det))

    return fwd, inv, 4


def projective(gcps):
    frm, to, src, dst = centred(gcps)
    s = max(max(abs(x), abs(y)) for x, y in src)
    t = max(max(abs(x), abs(y)) for x, y in dst)
    A, B = [], []
    for (x, y), (tx, ty) in zip(src, dst):
        x, y, tx, ty = x / s, y / s, tx / t, ty / t
        A.append([x, y, 1, 0, 0, 0, -x * tx, -y * tx])
        B.append(tx)
        A.append([0, 0, 0, x, y, 1, -x * ty, -y * ty])
        B.append(ty)
    A, B = np.array(A), np.array(B)
    h = np.linalg.solve(A.T @ A, A.T @ B)
    p = [h[0] * t / s, h[1] * t / s, h[2] * t, h[3] * t / s, h[4] * t / s, h[5] * t, h[6] / s, h[7] / s]

    def fwd(c, r):
        x, y = c - frm[0], r - frm[1]
        d = p[6] * x + p[7] * y + 1
        return (to[0] + (p[0] * x + p[1] * y + p[2]) / d, to[1] + (p[3] * x + p[4] * y + p[5]) / d)

    def inv(X, Y):
        tx, ty = X - to[0], Y - to[1]
        m00, m01, r0 = p[0] - tx * p[6], p[1] - tx * p[7], tx - p[2]
        m10, m11, r1 = p[3] - ty * p[6], p[4] - ty * p[7], ty - p[5]
        det = m00 * m11 - m01 * m10
        return (frm[0] + (m11 * r0 - m01 * r1) / det, frm[1] + (m00 * r1 - m10 * r0) / det)

    return fwd, inv, 8


UNKNOWNS = {"helmert": 4, "affine": 6, "projective": 8, "poly2": 12, "poly3": 20, "thinPlate": 0}
PROBES = [[0.0, 0.0], [2000.0, 0.0], [2000.0, 1500.0], [0.0, 1500.0], [1000.0, 750.0], [333.25, 1211.5], [1777.0, 222.75]]


def newton(tr, targets):
    """The pixels GDAL's forward sends to `targets`, by Newton's method from GDAL's reverse, all at once."""
    t = np.array(targets, dtype=float)
    start, _ = tr.TransformPoints(1, [(float(x), float(y), 0.0) for x, y in t])
    p = np.array([q[:2] for q in start], dtype=float)
    h = 0.5

    def fwd(q):
        out, _ = tr.TransformPoints(0, [(float(c), float(r), 0.0) for c, r in q])
        return np.array([o[:2] for o in out], dtype=float)

    for _ in range(30):
        f = fwd(p)
        dc = (fwd(p + [h, 0.0]) - fwd(p - [h, 0.0])) / (2 * h)
        dr = (fwd(p + [0.0, h]) - fwd(p - [0.0, h])) / (2 * h)
        a, b, c, d = dc[:, 0], dc[:, 1], dr[:, 0], dr[:, 1]
        rx, ry = f[:, 0] - t[:, 0], f[:, 1] - t[:, 1]
        det = a * d - b * c
        step = np.stack([(d * rx - c * ry) / det, (a * ry - b * rx) / det], axis=1)
        p = p - step
        if np.max(np.hypot(step[:, 0], step[:, 1])) < 1e-8:
            break
    miss = np.max(np.hypot(*(fwd(p) - t).T))
    if not miss < 1e-6:
        raise SystemExit(f"Newton's method did not settle: {miss} m")
    return [tuple(q) for q in p]


def solve(gcps, method):
    keep = None
    if method in ("helmert", "projective"):
        fwd, inv, _ = helmert(gcps) if method == "helmert" else projective(gcps)
    elif method == "affine":
        # GDAL's first-order fit forward; its inverse in closed form.
        # The differences over 10 000 pixels: a linear map's are exact, and the long base keeps the digits.
        keep, tr = gdal_transformer(gcps, "affine")
        o = tr.TransformPoint(0, 0.0, 0.0, 0.0)[1]
        ex = tr.TransformPoint(0, 10000.0, 0.0, 0.0)[1]
        ey = tr.TransformPoint(0, 0.0, 10000.0, 0.0)[1]
        A, C = (ex[0] - o[0]) / 10000.0, (ex[1] - o[1]) / 10000.0
        B, D = (ey[0] - o[0]) / 10000.0, (ey[1] - o[1]) / 10000.0

        def fwd(c, r, tr=tr):
            return tuple(tr.TransformPoint(0, c, r, 0.0)[1][:2])

        def inv(x, y, o=o, A=A, B=B, C=C, D=D):
            dx, dy = x - o[0], y - o[1]
            det = A * D - B * C
            return ((D * dx - B * dy) / det, (A * dy - C * dx) / det)
    else:
        keep, tr = gdal_transformer(gcps, method)

        def fwd(c, r, tr=tr):
            ok, p = tr.TransformPoint(0, c, r, 0.0)
            return (p[0], p[1])

        def inv(x, y, tr=tr):
            return newton(tr, [(x, y)])[0]

    res = []
    vv = 0.0
    n = 0
    for p in gcps:
        x, y = fwd(*p["pixel"])
        vx, vy = x - p["target"][0], y - p["target"][1]
        res.append([vx, vy, math.hypot(vx, vy)])
        if p["used"]:
            vv += vx * vx + vy * vy
            n += 1
    dof = 2 * n - UNKNOWNS[method]
    m0 = math.sqrt(vv / dof) if method != "thinPlate" and dof > 0 else None
    fw = [list(fwd(*q)) for q in PROBES]
    back = [list(inv(*f)) for f in fw]
    return {"method": method, "residuals": res, "m0": m0, "forward": fw, "back": back}


def warp_case():
    """gray-lzw-strips.tif carried by a thin plate through a bent sheet's points, nearest, the exact inverse."""
    src = DIR / "files" / "gray-lzw-strips.tif"
    sds = gdal.Open(str(src))
    w, h = sds.RasterXSize, sds.RasterYSize
    gcps = []
    for n, (c, r) in enumerate([(0, 0), (w, 0), (w, h), (0, h), (w / 2, h / 2), (w / 4, h * 0.7), (w * 0.8, h * 0.3), (w * 0.3, h * 0.2)]):
        u, v = (c - w / 2) / w, (r - h / 2) / h
        x = 487100 + 0.5 * c + 0.05 * r + 6 * u * v + 3 * u * u
        y = 4420300 - 0.5 * r + 0.04 * c + 4 * v * v - 2 * u * v
        gcps.append({"pixel": [float(c), float(r)], "target": [x, y], "used": True})
    with tempfile.TemporaryDirectory() as tmp:
        withg = Path(tmp) / "g.tif"
        gdal.Translate(str(withg), str(src), GCPs=[gdal.GCP(p["target"][0], p["target"][1], 0, p["pixel"][0], p["pixel"][1]) for p in gcps], outputSRS="EPSG:5256")
        # The grid KentOS makes: the forward map of the border, every 16 pixels, on multiples of the pixel size; a
        # sliver under a millionth of a pixel past an edge makes no row or column.
        ds, tr = gdal_transformer(gcps, "thinPlate")
        border = []
        for i in range(0, w + 1, 16):
            border += [(i, 0), (i, h)]
        for j in range(0, h + 1, 16):
            border += [(0, j), (w, j)]
        border += [(w, 0), (w, h), (0, h)]
        pts = [tr.TransformPoint(0, float(c), float(r), 0.0)[1][:2] for c, r in border]
        size = 0.6
        sliver = 1e-6
        left = math.floor(min(p[0] for p in pts) / size + sliver) * size
        top = math.ceil(max(p[1] for p in pts) / size - sliver) * size
        width = math.ceil((max(p[0] for p in pts) - left) / size - sliver)
        height = math.ceil((top - min(p[1] for p in pts)) / size - sliver)
        # Every output pixel's centre back to the source; nearest is the pixel the point falls in.
        centres = [(left + (i + 0.5) * size, top - (j + 0.5) * size) for j in range(height) for i in range(width)]
        back = np.array(newton(tr, centres))
        col, row = np.floor(back[:, 0]), np.floor(back[:, 1])
        inside = (col >= 0) & (col < w) & (row >= 0) & (row < h)
        src_pixels = sds.GetRasterBand(1).ReadAsArray()
        gray = np.zeros(len(centres), dtype=np.uint8)
        gray[inside] = src_pixels[row[inside].astype(int), col[inside].astype(int)]
        alpha = np.where(inside, 255, 0).astype(np.uint8)
        gray, alpha = gray.reshape(height, width), alpha.reshape(height, width)
        out = Path(tmp) / "w.tif"
        gdal.Warp(str(out), str(withg), tps=True, errorThreshold=0, resampleAlg="near", dstAlpha=True,
                  outputBounds=(left, top - height * size, left + width * size, top), xRes=size, yRes=size, outputType=gdal.GDT_Byte)
        ods = gdal.Open(str(out))
        wg, wa = ods.GetRasterBand(1).ReadAsArray(), ods.GetRasterBand(2).ReadAsArray()
        differ = int(((wg != gray) | (wa != alpha)).sum())
        if differ:
            raise SystemExit(f"gdalwarp differs from the exact inverse at {differ} pixels")
        return {
            "source": "gray-lzw-strips.tif",
            "points": gcps,
            "method": "thinPlate",
            "pixel": size,
            "border": [[float(c), float(r)] for c, r in border],
            "grid": {"affine": [left, size, 0.0, top, 0.0, -size], "width": width, "height": height},
            "gray": fnv(gray.astype(np.uint8).tobytes()),
            "alpha": fnv(alpha.astype(np.uint8).tobytes()),
            "inside": int((alpha > 0).sum()),
        }


def build():
    sets = []
    for kind in ("straight", "bent"):
        gcps = points(kind)
        sets.append({"name": kind, "points": gcps, "solutions": [solve(gcps, m) for m in ("helmert", "affine", "projective", "poly2", "poly3", "thinPlate")]})
    return {"format": "kentos.raster-georef-cases", "version": 1, "probes": PROBES, "sets": sets, "warp": warp_case()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} farklı; yeniden yazın.", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.relative_to(ROOT)} güncel.")
        return
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)} yazıldı ({len(data['sets'])} nokta takımı, {data['warp']['inside']} içeride kalan çıktı pikseli).")


if __name__ == "__main__":
    main()
