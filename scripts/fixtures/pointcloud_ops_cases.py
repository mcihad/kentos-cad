#!/usr/bin/env python3
"""Point cloud operations (docs/adr/0207 §7): what each gives, worked out without KentOS code.

The village's LiDAR (fixtures/interaction/v1/pointclouds/koy.laz) and two small clouds written here
(fixtures/pointcloud/v1/ops/: sinir.laz, a pattern of filled cells for Sınır çıkar; ikinci.laz, a cloud at another
scale and offset for Birleştir) are read with laspy (LASzip's decoding), and every operation is computed again from
the ADR's definitions: Seyrelt (Hücreyle, Yarıçapla, Her n'inci), Zemin süzgeci (SMRF step by step, numpy's array
shifts for the discs), Yüksekliğe göre sınıfla, Kırp (rectangles, a disc and an area with a hole by their plain
geometric tests, edges inside), Alan sorgusu (the figures from the files' whole numbers), Karola, Rasterleştir (the
least, most, mean, count and IDW, cell by cell in the points' order), Sınır çıkar (the grid's edges traced with left
turns, checked against the cells' area and edges) and Birleştir (records widened by the ADR's rules, moved onto the
finer grid, the half away from zero).

fixtures/pointcloud/v1/ops.json: the expected answers (counts, FNV-1a 64 of kept numbers, classes, records and
raster values, the figures as numbers), which crates/shared/pointcloud/tests/all/ops.rs compares with the core's
machines.

Needs laspy with LASzip and numpy; when this Python lacks them the script runs itself with .run/pyref.

    python3 scripts/fixtures/pointcloud_ops_cases.py          # write
    python3 scripts/fixtures/pointcloud_ops_cases.py --check  # compare with what is on disk
"""

import argparse
import io
import json
import math
import os
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
try:
    import laspy
    import numpy as np
    from laspy import LazBackend
except ImportError:
    venv = ROOT / ".run" / "pyref"
    py = venv / "bin" / "python"
    if py.exists() and Path(sys.prefix).resolve() != venv.resolve():
        os.execv(str(py), [str(py), *sys.argv])
    sys.exit("laspy[laszip] gerekli: python3 -m venv .run/pyref && .run/pyref/bin/pip install 'laspy[laszip]==2.7.0' numpy")

KOY = ROOT / "fixtures" / "interaction" / "v1" / "pointclouds" / "koy.laz"
DIR = ROOT / "fixtures" / "pointcloud" / "v1"
OPS = DIR / "ops"
OUT = DIR / "ops.json"
X0, Y0 = 487600.0, 4420300.0


def fnv(data: bytes) -> str:
    h = 0xCBF29CE484222325
    for b in data:
        h ^= b
        h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def fnv_u64(values) -> str:
    return fnv(b"".join(struct.pack("<Q", int(v)) for v in values))


def round_away(n: float) -> float:
    """The half away from zero, exactly (Rust's f64::round)."""
    t = float(math.trunc(n))
    f = n - t
    if abs(f) >= 0.5:
        t += math.copysign(1.0, n)
    return t


class Cloud:
    def __init__(self, data: bytes):
        self.data = data
        las = laspy.read(io.BytesIO(data))
        self.las = las
        h = las.header
        self.scale = [float(v) for v in h.scales]
        self.offset = [float(v) for v in h.offsets]
        self.mins = [float(v) for v in h.mins]
        self.maxs = [float(v) for v in h.maxs]
        self.X = np.asarray(las.X, dtype=np.int64)
        self.Y = np.asarray(las.Y, dtype=np.int64)
        self.Z = np.asarray(las.Z, dtype=np.int64)
        self.x = self.X.astype(np.float64) * self.scale[0] + self.offset[0]
        self.y = self.Y.astype(np.float64) * self.scale[1] + self.offset[1]
        self.z = self.Z.astype(np.float64) * self.scale[2] + self.offset[2]
        self.cls = np.asarray(las.classification, dtype=np.int64)
        self.ret = np.asarray(las.return_number, dtype=np.int64)
        self.nret = np.asarray(las.number_of_returns, dtype=np.int64)
        self.n = len(self.x)
        self.fmt = h.point_format.id
        self.record_len = h.point_format.size

    def records(self) -> bytes:
        return bytes(self.las.points.array.tobytes())


# ---------------------------------------------------------------- Seyrelt


def thin_cell(c: Cloud, s: float):
    kx, ky, kz = np.floor(c.x / s), np.floor(c.y / s), np.floor(c.z / s)
    cx, cy, cz = (kx + 0.5) * s, (ky + 0.5) * s, (kz + 0.5) * s
    d = ((0.0 + (c.x - cx) * (c.x - cx)) + (c.y - cy) * (c.y - cy)) + (c.z - cz) * (c.z - cz)
    best = {}
    for i in range(c.n):
        k = (kx[i], ky[i], kz[i])
        b = best.get(k)
        if b is None or d[i] < b[1]:
            best[k] = (i, d[i])
    return sorted(v[0] for v in best.values())


def thin_radius(c: Cloud, r: float):
    r2 = r * r
    grid = {}
    kept = []
    for i in range(c.n):
        p = (c.x[i], c.y[i], c.z[i])
        k = (math.floor(p[0] / r), math.floor(p[1] / r), math.floor(p[2] / r))
        near = False
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    for q in grid.get((k[0] + dx, k[1] + dy, k[2] + dz), ()):
                        if (p[0] - q[0]) * (p[0] - q[0]) + (p[1] - q[1]) * (p[1] - q[1]) + (p[2] - q[2]) * (p[2] - q[2]) < r2:
                            near = True
                            break
                    if near:
                        break
                if near:
                    break
            if near:
                break
        if not near:
            grid.setdefault(k, []).append(p)
            kept.append(i)
    return kept


# ---------------------------------------------------------------- the height grid (SMRF, HAG)


class Grid:
    def __init__(self, plan, cell):
        self.x0, self.y0, self.cell = plan[0], plan[1], cell
        self.cols = int(math.floor((plan[2] - plan[0]) / cell) + 1)
        self.rows = int(math.floor((plan[3] - plan[1]) / cell) + 1)
        self.v = np.full((self.rows, self.cols), np.nan)

    def cell_of(self, x, y):
        c = np.floor((x - self.x0) / self.cell)
        r = np.floor((y - self.y0) / self.cell)
        c = np.where(c >= 0, np.minimum(c, self.cols - 1), 0).astype(np.int64)
        r = np.where(r >= 0, np.minimum(r, self.rows - 1), 0).astype(np.int64)
        return c, r

    def put_min(self, x, y, z):
        c, r = self.cell_of(x, y)
        for i in range(len(z)):
            old = self.v[r[i], c[i]]
            if math.isnan(old) or z[i] < old:
                self.v[r[i], c[i]] = z[i]


def fill(v):
    """Wave by wave: an empty cell next to filled ones (of the 8, filled before the wave) takes their least."""
    v = v.copy()
    rows, cols = v.shape
    if np.all(np.isnan(v)):
        return v
    while np.any(np.isnan(v)):
        pad = np.full((rows + 2, cols + 2), np.inf)
        pad[1:-1, 1:-1] = np.where(np.isnan(v), np.inf, v)
        least = np.full((rows, cols), np.inf)
        for dr in (-1, 0, 1):
            for dc in (-1, 0, 1):
                if dr == 0 and dc == 0:
                    continue
                least = np.minimum(least, pad[1 + dr:1 + dr + rows, 1 + dc:1 + dc + cols])
        take = np.isnan(v) & np.isfinite(least)
        if not np.any(take):
            break
        v[take] = least[take]
    return v


def morph(v, r, most):
    """The least (or most) over a disc of r cells (i² + j² ≤ r²), the grid's outside left out."""
    rows, cols = v.shape
    none = -np.inf if most else np.inf
    pad = np.full((rows + 2 * r, cols + 2 * r), none)
    pad[r:r + rows, r:r + cols] = v
    out = np.full((rows, cols), none)
    pick = np.maximum if most else np.minimum
    for dy in range(-r, r + 1):
        for dx in range(-r, r + 1):
            if dx * dx + dy * dy <= r * r:
                out = pick(out, pad[r + dy:r + dy + rows, r + dx:r + dx + cols])
    return out


def progressive(v, cell, window, slope):
    surface = v.copy()
    flags = np.zeros(v.shape, dtype=bool)
    steps = math.ceil(window / cell)
    for r in range(1, max(0, steps) + 1):
        opened = morph(morph(surface, r, False), r, True)
        flags |= (surface - opened) > slope * r * cell
        surface = opened
    return flags


def axis(f, n):
    if not (f > 0.0):
        return 0, 0, 0.0
    if f >= n - 1:
        return n - 1, n - 1, 0.0
    i = math.floor(f)
    return i, i + 1, f - i


def bilinear(g, s, x, y):
    c0, c1, tx = axis((x - g.x0) / g.cell - 0.5, g.cols)
    r0, r1, ty = axis((y - g.y0) / g.cell - 0.5, g.rows)
    a = s[r0, c0] + (s[r0, c1] - s[r0, c0]) * tx
    b = s[r1, c0] + (s[r1, c1] - s[r1, c0]) * tx
    return a + (b - a) * ty


def slope_at(g, s, x, y):
    c = math.floor((x - g.x0) / g.cell)
    r = math.floor((y - g.y0) / g.cell)
    c = min(c, g.cols - 1) if c >= 0 else 0
    r = min(r, g.rows - 1) if r >= 0 else 0
    cl, cr = max(c - 1, 0), min(c + 1, g.cols - 1)
    rb, rt = max(r - 1, 0), min(r + 1, g.rows - 1)
    gx = (s[r, cr] - s[r, cl]) / ((cr - cl) * g.cell) if cr > cl else 0.0
    gy = (s[rt, c] - s[rb, c]) / ((rt - rb) * g.cell) if rt > rb else 0.0
    return math.sqrt(gx * gx + gy * gy)


def smrf(c: Cloud, plan, cell=1.0, slope=0.15, window=18.0, threshold=0.5, scalar=1.25, last_only=True):
    g = Grid(plan, cell)
    use = (c.ret >= c.nret) if last_only else np.ones(c.n, dtype=bool)
    g.put_min(c.x[use], c.y[use], c.z[use])
    filled = fill(g.v)
    outliers = progressive(-filled, cell, 5.0, 1.0)
    cleared = g.v.copy()
    cleared[outliers] = np.nan
    again = fill(cleared)
    objects = progressive(again, cell, window, slope)
    ground = cleared.copy()
    ground[objects] = np.nan
    s = fill(ground)
    cls = c.cls.copy()
    count = 0
    for i in range(c.n):
        surf = bilinear(g, s, c.x[i], c.y[i])
        sl = slope_at(g, s, c.x[i], c.y[i])
        if abs(c.z[i] - surf) <= threshold + scalar * sl:
            cls[i] = 2
            count += 1
        elif cls[i] == 2:
            cls[i] = 1
    return count, cls


def hag(c: Cloud, plan, cell=1.0, base=0.15, low=0.5, middle=2.0, high=50.0, every=True):
    g = Grid(plan, cell)
    sel = c.cls == 2
    g.put_min(c.x[sel], c.y[sel], c.z[sel])
    s = fill(g.v)
    cls = c.cls.copy()
    n = [0, 0, 0]
    for i in range(c.n):
        k = cls[i]
        if not ((k != 2) if every else (k <= 1)):
            continue
        h = c.z[i] - bilinear(g, s, c.x[i], c.y[i])
        if h < base or h > high:
            continue
        to = 3 if h <= low else 4 if h <= middle else 5
        cls[i] = to
        n[to - 3] += 1
    return n, cls


# ---------------------------------------------------------------- Kırp and Alan sorgusu

RECT = (4.0, 54.0, 36.0, 88.0)
DISC = (90.0005, 20.0005, 12.0)
HOLED = ((40.0, 4.0, 70.0, 34.0), (50.0, 14.0, 60.0, 24.0))


def in_rect(x, y, r):
    return (x >= X0 + r[0]) & (x <= X0 + r[2]) & (y >= Y0 + r[1]) & (y <= Y0 + r[3])


def in_disc(x, y, d):
    dx, dy = x - (X0 + d[0]), y - (Y0 + d[1])
    return dx * dx + dy * dy < d[2] * d[2]


def on_disc(x, y, d):
    dx, dy = x - (X0 + d[0]), y - (Y0 + d[1])
    return dx * dx + dy * dy == d[2] * d[2]


def in_holed(x, y, h):
    outer, hole = h
    strictly = (x > X0 + hole[0]) & (x < X0 + hole[2]) & (y > Y0 + hole[1]) & (y < Y0 + hole[3])
    return in_rect(x, y, outer) & ~strictly


def figures(c: Cloud, inside):
    idx = np.nonzero(inside)[0]
    n = len(idx)
    out = {"count": n}
    if n == 0:
        return out
    zs = [int(v) for v in c.Z[idx]]
    s1 = sum(zs)
    s2 = sum(v * v for v in zs)
    k = float(n)
    mean_i = float(s1) / k
    dev = float(n * s2 - s1 * s1) / k
    file_mean = mean_i * c.scale[2] + c.offset[2]
    file_m2 = dev * c.scale[2] * c.scale[2]
    nn, mean, m2 = 0.0, 0.0, 0.0
    total = nn + k
    delta = file_mean - mean
    mean = mean + delta * k / total
    m2 = m2 + (file_m2 + delta * delta * nn * k / total)
    nn = total
    out["zMin"] = float(c.z[idx].min())
    out["zMax"] = float(c.z[idx].max())
    out["zMean"] = mean
    out["zStd"] = math.sqrt(m2 / (nn - 1.0)) if nn >= 2 else None
    cl = {}
    for v in c.cls[idx]:
        cl[int(v)] = cl.get(int(v), 0) + 1
    out["classes"] = [[k_, cl[k_]] for k_ in sorted(cl)]
    return out


# ---------------------------------------------------------------- Rasterleştir and Sınır çıkar


def frame(plan, cell):
    x0 = math.floor(plan[0] / cell) * cell
    top = math.ceil(plan[3] / cell) * cell
    cols = int(max(math.ceil((plan[2] - x0) / cell), 1.0))
    rows = int(max(math.ceil((top - plan[1]) / cell), 1.0))
    return x0, top, cols, rows


def frame_cell(f, cell, x, y):
    x0, top, cols, rows = f
    c = math.floor((x - x0) / cell)
    r = math.floor((top - y) / cell)
    c = min(c, cols - 1) if c >= 0 else 0
    r = min(r, rows - 1) if r >= 0 else 0
    return c, r


def rasterize(c: Cloud, plan, cell, value, radius=None):
    f = frame(plan, cell)
    x0, top, cols, rows = f
    k = cols * rows
    a = [math.inf if value == "min" else -math.inf if value == "max" else 0.0] * k
    b = [0.0] * k
    on = [0.0] * k
    n = [0] * k
    for i in range(c.n):
        x, y, z = float(c.x[i]), float(c.y[i]), float(c.z[i])
        if value == "idw":
            lo_c = max(math.ceil((x - radius - x0) / cell - 0.5), 0.0)
            hi_c = math.floor((x + radius - x0) / cell - 0.5)
            lo_r = max(math.ceil((top - y - radius) / cell - 0.5), 0.0)
            hi_r = math.floor((top - y + radius) / cell - 0.5)
            if not (hi_c >= lo_c and hi_r >= lo_r):
                continue
            for rr in range(int(lo_r), min(int(hi_r), rows - 1) + 1):
                for cc in range(int(lo_c), min(int(hi_c), cols - 1) + 1):
                    cx = x0 + (cc + 0.5) * cell
                    cy = top - (rr + 0.5) * cell
                    d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy)
                    if d2 > radius * radius:
                        continue
                    j = rr * cols + cc
                    if d2 == 0.0:
                        on[j] += z
                        n[j] += 1
                    else:
                        w = 1.0 / d2
                        a[j] += w * z
                        b[j] += w
            continue
        cc, rr = frame_cell(f, cell, x, y)
        j = rr * cols + cc
        if value == "min":
            a[j] = min(a[j], z)
        elif value == "max":
            a[j] = max(a[j], z)
        elif value == "mean":
            a[j] += z
        n[j] += 1
    out = []
    for j in range(k):
        if value == "idw":
            out.append(on[j] / n[j] if n[j] > 0 else a[j] / b[j] if b[j] > 0.0 else -9999.0)
        elif n[j] == 0:
            out.append(-9999.0)
        elif value in ("min", "max"):
            out.append(a[j])
        elif value == "mean":
            out.append(a[j] / n[j])
        else:
            out.append(float(n[j]))
    bits = np.asarray(out, dtype=np.float64).astype(np.float32).tobytes()
    return {"cols": cols, "rows": rows, "filled": sum(1 for v in out if v != -9999.0), "values": fnv(bits)}


def trace(filled, cols, rows):
    """The filled cells' rings: unit edges with the filled cell on the left, joined with left turns where two
    leave a corner; started from the corners in rows from the top, then left to right; straight runs merged."""
    def at(cc, rr):
        return 0 <= cc < cols and 0 <= rr < rows and filled[rr][cc]
    out_of = {}
    for rr in range(rows):
        for cc in range(cols):
            if not at(cc, rr):
                continue
            if not at(cc, rr + 1):
                out_of.setdefault((cc, rr + 1), []).append((cc + 1, rr + 1))
            if not at(cc + 1, rr):
                out_of.setdefault((cc + 1, rr + 1), []).append((cc + 1, rr))
            if not at(cc, rr - 1):
                out_of.setdefault((cc + 1, rr), []).append((cc, rr))
            if not at(cc - 1, rr):
                out_of.setdefault((cc, rr), []).append((cc, rr + 1))
    rings = []
    for s in sorted(out_of, key=lambda p: (p[1], p[0])):
        while out_of.get(s):
            first = out_of[s].pop(0)
            ring = [s]
            prev, cur = s, first
            while cur != s:
                ring.append(cur)
                d = (cur[0] - prev[0], cur[1] - prev[1])
                nxt = out_of[cur]
                left = (d[1], -d[0])
                pick = 0
                if len(nxt) > 1:
                    for i, q in enumerate(nxt):
                        if (q[0] - cur[0], q[1] - cur[1]) == left:
                            pick = i
                            break
                prev, cur = cur, nxt.pop(pick)
            rings.append(ring)
    return rings


def straight(ring):
    n = len(ring)
    out = []
    for i in range(n):
        a, b, c_ = ring[i - 1], ring[i], ring[(i + 1) % n]
        if (b[0] - a[0]) * (c_[1] - b[1]) - (b[1] - a[1]) * (c_[0] - b[0]) != 0:
            out.append(b)
    return out


def area2(ring):
    s = 0
    n = len(ring)
    for i in range(n):
        a, b = ring[i], ring[(i + 1) % n]
        s += a[0] * (-b[1]) - b[0] * (-a[1])
    return s


def inside_ring(ring, x, y):
    odd = False
    n = len(ring)
    for i in range(n):
        (ax, ay), (bx, by) = ring[i], ring[(i + 1) % n]
        if (ay > y) != (by > y) and x < ax + (y - ay) / (by - ay) * (bx - ax):
            odd = not odd
    return odd


def boundary(c: Cloud, plan, cell, least):
    f = frame(plan, cell)
    x0, top, cols, rows = f
    counts = [[0] * cols for _ in range(rows)]
    for i in range(c.n):
        cc, rr = frame_cell(f, cell, float(c.x[i]), float(c.y[i]))
        counts[rr][cc] += 1
    filled = [[counts[r][q] >= max(least, 1) for q in range(cols)] for r in range(rows)]
    raw = trace(filled, cols, rows)
    outers, holes = [], []
    for r in raw:
        s = straight(r)
        if len(s) < 4:
            continue
        a2 = area2(s)
        if a2 > 0:
            outers.append((s, a2))
        else:
            a, b = r[0], r[1]
            dc, dr = b[0] - a[0], b[1] - a[1]
            mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
            holes.append((s, (mid[0] - 0.5 * dr, mid[1] + 0.5 * dc)))
    parts = [[o, []] for o, _ in outers]
    for h, probe in holes:
        best = None
        for i, (o, a2) in enumerate(outers):
            if inside_ring(o, probe[0], probe[1]) and (best is None or a2 < outers[best][1]):
                best = i
        if best is not None:
            parts[best][1].append(h)
    # Checks: the parts' net area is the filled cells', their edges are the cells' borders.
    net = sum(area2(o) for o, _ in parts) + sum(area2(h) for _, hs in parts for h in hs)
    assert net == 2 * sum(sum(1 for v in row if v) for row in filled), "alan tutmuyor"
    world = lambda ring: [[x0 + q[0] * cell, top - q[1] * cell] for q in ring]
    return [{"outer": world(o), "holes": [world(h) for h in hs]} for o, hs in parts]


# ---------------------------------------------------------------- Birleştir


def merge(first: Cloud, second: Cloud):
    """koy (format 7, mm) then ikinci (format 6, 0.4 mm) as format 7 (the widest), at the finest scale and koy's offset:
    each record widened by the ADR's rules when its format is not 7 (colours zero), its coordinates moved onto
    the result's grid, the half away from zero; a point off the grid by more than a millionth of a step counts."""
    scale = [min(a, b) for a, b in zip(first.scale, second.scale)]
    offset = first.offset
    out = bytearray()
    rounded = 0
    for c in (first, second):
        raw = c.records()
        n = c.record_len
        for i in range(c.n):
            r = raw[i * n:(i + 1) * n]
            ints = struct.unpack_from("<iii", r, 0)
            moved = []
            off = False
            for k in range(3):
                v = float(ints[k]) * c.scale[k] + c.offset[k]
                m_ = (v - offset[k]) / scale[k]
                m = round_away(m_)
                if abs(m_ - m) > 1e-6:
                    off = True
                moved.append(int(m))
            if off:
                rounded += 1
            if c.fmt == 7:
                rec = bytearray(r)
            else:
                # Format 6 to 7: the same bytes, then three colours of zero.
                rec = bytearray(r[:30]) + bytes(6) + bytearray(r[30:])
            struct.pack_into("<iii", rec, 0, *moved)
            out += rec
    return {"count": first.n + second.n, "rounded": rounded, "scale": scale, "records": fnv(bytes(out))}


# ---------------------------------------------------------------- the small clouds


def write(points, scale, offset, fmt, version="1.4"):
    header = laspy.LasHeader(point_format=fmt, version=version)
    header.scales = scale
    header.offsets = offset
    las = laspy.LasData(header)
    las.x = np.asarray([p[0] for p in points])
    las.y = np.asarray([p[1] for p in points])
    las.z = np.asarray([p[2] for p in points])
    las.classification = np.asarray([p[3] for p in points], dtype=np.uint8)
    las.return_number = np.ones(len(points), dtype=np.uint8)
    las.number_of_returns = np.ones(len(points), dtype=np.uint8)
    las.intensity = np.full(len(points), 1000, dtype=np.uint16)
    out = io.BytesIO()
    las.write(out, do_compress=True, laz_backend=LazBackend.Laszip)
    return out.getvalue()


# A 10 × 8 pattern of 1 m cells (rows from the top): a ring of cells around a hole, a cell touching it at a corner,
# a separate bar, and cells with one point only (below the least of 2).
PATTERN = [
    "##########",
    "#..#......",
    "#..#.###..",
    "####.#.#..",
    ".....###..",
    "#.........",
    ".#..#....#",
    "........##",
]


def sinir_points():
    pts = []
    for r, line in enumerate(PATTERN):
        for q, ch in enumerate(line):
            if ch != "#":
                continue
            x = 1000.0 + q + 0.5
            y = 2000.0 + (len(PATTERN) - 1 - r) + 0.5
            pts += [(x - 0.2, y - 0.2, 10.0, 2), (x + 0.2, y + 0.2, 10.5, 2)]
    # Corners of the grid's frame: a point each, below the least, so the frame is the pattern's.
    pts += [(1000.05, 2000.05, 9.0, 1), (1009.95, 2007.95, 9.0, 1)]
    return pts


def ikinci_points():
    rng = np.random.default_rng(2071)
    pts = []
    for i in range(500):
        x = X0 + 130.0 + rng.uniform(0, 20)
        y = Y0 + rng.uniform(0, 20)
        z = 850.0 + rng.uniform(0, 2)
        pts.append((x, y, z, 2))
    return pts


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    files = {
        "sinir.laz": write(sinir_points(), [0.001, 0.001, 0.001], [1000.0, 2000.0, 0.0], 6),
        "ikinci.laz": write(ikinci_points(), [0.0004, 0.0004, 0.001], [487000.0, 4420000.0, 0.0], 6),
    }
    koy = Cloud(KOY.read_bytes())
    sinir = Cloud(files["sinir.laz"])
    ikinci = Cloud(files["ikinci.laz"])
    plan = [koy.mins[0], koy.mins[1], koy.maxs[0], koy.maxs[1]]
    assert not np.any(on_disc(koy.x, koy.y, DISC)), "bir nokta dairenin tam üstünde"

    cases = {}
    kept = thin_cell(koy, 1.0)
    cases["thinCell"] = {"size": 1.0, "count": len(kept), "kept": fnv_u64(kept)}
    kept = thin_radius(koy, 0.6)
    cases["thinRadius"] = {"size": 0.6, "count": len(kept), "kept": fnv_u64(kept)}
    kept = [i for i in range(koy.n) if i % 7 == 0]
    cases["thinNth"] = {"n": 7, "count": len(kept), "kept": fnv_u64(kept)}
    count, cls = smrf(koy, plan)
    cases["ground"] = {"ground": count, "classes": fnv(bytes(int(v) for v in cls))}
    n, cls = hag(koy, plan)
    cases["height"] = {"all": True, "classed": n, "classes": fnv(bytes(int(v) for v in cls))}
    inside = in_rect(koy.x, koy.y, RECT) | in_disc(koy.x, koy.y, DISC) | in_holed(koy.x, koy.y, HOLED)
    idx = np.nonzero(inside)[0]
    cases["clip"] = {"count": len(idx), "kept": fnv_u64(idx), "outside": int(koy.n - len(idx))}
    cases["areaStats"] = [
        figures(koy, in_rect(koy.x, koy.y, RECT)),
        figures(koy, in_disc(koy.x, koy.y, DISC)),
        figures(koy, in_holed(koy.x, koy.y, HOLED)),
    ]
    tiles = {}
    for i in range(koy.n):
        t = (math.floor(koy.x[i] / 50.0), math.floor(koy.y[i] / 50.0))
        tiles[t] = tiles.get(t, 0) + 1
    cases["tile"] = {"size": 50.0, "tiles": [[t[0], t[1], tiles[t]] for t in sorted(tiles)]}
    cases["rasterize"] = {
        v: rasterize(koy, plan, 2.0, v, 2.5 if v == "idw" else None) for v in ("min", "max", "mean", "count", "idw")
    }
    splan = [sinir.mins[0], sinir.mins[1], sinir.maxs[0], sinir.maxs[1]]
    cases["boundary"] = {"cell": 1.0, "least": 2, "parts": boundary(sinir, splan, 1.0, 2)}
    cases["merge"] = merge(koy, ikinci)

    doc = {
        "format": "kentos.pointcloud-ops",
        "version": 1,
        "about": "docs/adr/0207 §7: each operation computed again from the ADR's definitions (scripts/fixtures/pointcloud_ops_cases.py).",
        "files": {"koy": "../../interaction/v1/pointclouds/koy.laz", "sinir": "ops/sinir.laz", "ikinci": "ops/ikinci.laz"},
        "shapes": {"rect": RECT, "disc": DISC, "holed": HOLED, "origin": [X0, Y0]},
        "cases": cases,
    }
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        bad = [n_ for n_, b in files.items() if not (OPS / n_).exists() or (OPS / n_).read_bytes() != b]
        if not OUT.exists() or OUT.read_text() != text:
            bad.append("ops.json")
        if bad:
            sys.exit("farklı: " + ", ".join(bad))
        print("pointcloud_ops_cases: aynı")
        return
    OPS.mkdir(parents=True, exist_ok=True)
    for n_, b in files.items():
        (OPS / n_).write_bytes(b)
    OUT.write_text(text)
    print(json.dumps({k: (v if not isinstance(v, dict) else {kk: vv for kk, vv in v.items() if kk != "tiles"}) for k, v in cases.items() if k != "areaStats" and k != "boundary"}, indent=1)[:2000])


if __name__ == "__main__":
    main()
