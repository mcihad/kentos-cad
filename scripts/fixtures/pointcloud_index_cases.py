#!/usr/bin/env python3
"""The COPC index KentOS builds (docs/adr/0207 §4), worked out without KentOS code.

From a source cloud (read by laspy, LAZ through LASzip) every record is moved to format 6, 7 or 8 (the legacy rules of
pointcloud_cases.py), put in its bin at depth d₀, and each bin's subtree is built from its leaves up: a node holding
more points than the leaf limit splits (octants by `≥` the node's middle, in order 0–7); every node gives its parent,
for each filled cell of the parent's 128³ grid, the point nearest that cell's centre (ties to the earlier in the
source; in the cells' order), and keeps the rest; the nodes above the bins are built from what the bins' roots gave,
deepest first. The result per node: its key, its points and the FNV-1a 64 hash of its records in order.

fixtures/pointcloud/v1/index.json: those nodes for files/many-chunks.laz with the ADR's limits and for
files/index-sample.laz (12 000 clustered points with duplicates, written here by LASzip) with small limits, so that
splits and bins show on a small file. `--verify FILE` reads a COPC KentOS wrote with LASzip and checks it point by
point against the same build (the header, COPC's info VLR first, the hierarchy, each node's chunk).

    python3 scripts/fixtures/pointcloud_index_cases.py          # write
    python3 scripts/fixtures/pointcloud_index_cases.py --check  # compare with what is on disk
    python3 scripts/fixtures/pointcloud_index_cases.py --verify fixtures/pointcloud/v1/files/index-sample.copc.laz
        (KentOS's index of index-sample.laz with a leaf limit of 400 and bins at depth 2; --source, --leaf-most and
        --bin-depth name another)
"""

import argparse
import json
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import pointcloud_cases as pc  # noqa: E402  (re-runs itself with .run/pyref when laspy is missing)
import laspy  # noqa: E402
import numpy as np  # noqa: E402
from laspy import LazBackend  # noqa: E402

ROOT = pc.ROOT
FILES = pc.FILES
OUT = pc.DIR / "index.json"
GRID = 128
MAX_DEPTH = 16


def cube_of(b):
    center = [(b[0] + b[3]) / 2.0, (b[1] + b[4]) / 2.0, (b[2] + b[5]) / 2.0]
    half = max((b[3] - b[0]) / 2.0, (b[4] - b[1]) / 2.0, (b[5] - b[2]) / 2.0)
    if not (math.isfinite(half) and half > 0.0):
        half = 1.0
    return center, half


def cell_of(v, low, cell, n):
    t = math.floor((v - low) / cell)
    return min(max(t, 0), n - 1)


class Tree:
    def __init__(self, scale, offset, bounds, leaf_most, bin_depth):
        self.scale = scale
        self.offset = offset
        self.center, self.half = cube_of(bounds)
        self.low = [c - self.half for c in self.center]
        self.leaf_most = leaf_most
        self.bin_depth = bin_depth
        self.nodes = []

    def world(self, rec):
        x, y, z = struct.unpack_from("<iii", rec, 0)
        return (x * self.scale[0] + self.offset[0], y * self.scale[1] + self.offset[1], z * self.scale[2] + self.offset[2])

    def node(self, key):
        d, x, y, z = key
        side = 2.0 * self.half / float(1 << d)
        return [self.low[0] + x * side, self.low[1] + y * side, self.low[2] + z * side], side

    def key_at(self, p, d):
        n = 1 << d
        side = 2.0 * self.half / float(n)
        return (d, cell_of(p[0], self.low[0], side, n), cell_of(p[1], self.low[1], side, n), cell_of(p[2], self.low[2], side, n))

    def give(self, key, pts):
        """pts: [(rec, src, world)]; (up, own)."""
        if key[0] == 0:
            return [], list(pts)
        parent = (key[0] - 1, key[1] >> 1, key[2] >> 1, key[3] >> 1)
        low, side = self.node(parent)
        cell = side / float(GRID)
        best = {}
        for at, (rec, src, p) in enumerate(pts):
            i = cell_of(p[0], low[0], cell, GRID)
            j = cell_of(p[1], low[1], cell, GRID)
            k = cell_of(p[2], low[2], cell, GRID)
            c = (low[0] + (i + 0.5) * cell, low[1] + (j + 0.5) * cell, low[2] + (k + 0.5) * cell)
            dd = (p[0] - c[0]) * (p[0] - c[0]) + (p[1] - c[1]) * (p[1] - c[1]) + (p[2] - c[2]) * (p[2] - c[2])
            cid = i + GRID * (j + GRID * k)
            b = best.get(cid)
            if b is None or dd < b[0] or (dd == b[0] and src < b[1]):
                best[cid] = (dd, src, at)
        taken = set()
        up = []
        for cid in sorted(best):
            at = best[cid][2]
            taken.add(at)
            up.append(pts[at])
        own = [q for at, q in enumerate(pts) if at not in taken]
        return up, own

    def subtree(self, key, pts):
        if len(pts) > self.leaf_most and key[0] < MAX_DEPTH:
            low, side = self.node(key)
            mid = [low[0] + side / 2.0, low[1] + side / 2.0, low[2] + side / 2.0]
            parts = [[] for _ in range(8)]
            for q in pts:
                p = q[2]
                o = (p[0] >= mid[0]) | ((p[1] >= mid[1]) << 1) | ((p[2] >= mid[2]) << 2)
                parts[o].append(q)
            kept = []
            for o, part in enumerate(parts):
                if not part:
                    continue
                child = (key[0] + 1, key[1] * 2 + (o & 1), key[2] * 2 + ((o >> 1) & 1), key[3] * 2 + ((o >> 2) & 1))
                kept.extend(self.subtree(child, part))
        else:
            kept = pts
        up, own = self.give(key, kept)
        self.nodes.append((key, own))
        return up

    def build(self, records):
        bins = {}
        for src, rec in enumerate(records):
            p = self.world(rec)
            bins.setdefault(self.key_at(p, self.bin_depth), []).append((rec, src, p))
        gifts = {}
        for key in sorted(bins):
            gifts[key] = self.subtree(key, bins[key])
        upper = set()
        for k in gifts:
            d, x, y, z = k
            while d > 0:
                d, x, y, z = d - 1, x >> 1, y >> 1, z >> 1
                upper.add((d, x, y, z))
        for key in sorted(upper, key=lambda k: (-k[0], k)):
            kept = []
            for o in range(8):
                child = (key[0] + 1, key[1] * 2 + (o & 1), key[2] * 2 + ((o >> 1) & 1), key[3] * 2 + ((o >> 2) & 1))
                kept.extend(gifts.pop(child, []))
            up, own = self.give(key, kept)
            self.nodes.append((key, own))
            gifts[key] = up
        wanted = set()
        for key, own in self.nodes:
            if own:
                k = key
                while True:
                    if k in wanted:
                        break
                    wanted.add(k)
                    if k[0] == 0:
                        break
                    k = (k[0] - 1, k[1] >> 1, k[2] >> 1, k[3] >> 1)
        return sorted([(k, own) for k, own in self.nodes if k in wanted], key=lambda t: t[0])


def wide_records(path):
    las = laspy.read(str(path))
    fmt = las.header.point_format.id
    raw = las.points.array.tobytes()
    n = las.header.point_format.size
    return las, [pc.widen(fmt, raw[i:i + n]) for i in range(0, len(raw), n)]


def build_sample():
    """12 000 points in three clusters, a hundred exact duplicates and points on the cube's faces."""
    rng = np.random.default_rng(2077)
    header = laspy.LasHeader(point_format=3, version="1.2")
    header.scales = np.array([0.001, 0.001, 0.001])
    header.offsets = np.array([400000.0, 4500000.0, 0.0])
    header.system_identifier = "KentOS fixtures"
    header.generating_software = "laspy 2.7.0 + LASzip"
    las = laspy.LasData(header)
    parts = []
    for cx, cy, n, spread in [(10_000, 20_000, 7_200, 4_000), (80_000, 70_000, 3_600, 15_000), (50_000, 50_000, 1_100, 50_000)]:
        parts.append(np.column_stack([cx + rng.normal(0, spread, n), cy + rng.normal(0, spread, n), rng.normal(20_000, 3_000, n)]))
    # Centimetres (multiples of ten of the scale): the file compresses, the ties stay.
    pts = (np.concatenate(parts) / 10.0).round().astype(np.int64) * 10
    pts = np.clip(pts, 0, 100_000)
    dup = pts[rng.integers(0, len(pts), 100)]
    pts = np.concatenate([pts, dup])
    n = len(pts)
    las.X, las.Y, las.Z = pts[:, 0], pts[:, 1], pts[:, 2]
    las.intensity = (pts[:, 0] // 50 + pts[:, 1] // 70) % 4096
    cls = rng.choice([1, 2, 5, 6], n)
    las.classification = cls
    las.return_number = np.ones(n, dtype=np.int64)
    las.number_of_returns = np.ones(n, dtype=np.int64)
    las.gps_time = np.arange(n) * 0.5
    palette = {1: (40000, 40000, 40000), 2: (35000, 25000, 15000), 5: (10000, 50000, 12000), 6: (55000, 12000, 9000)}
    las.red = np.array([palette[c][0] for c in cls])
    las.green = np.array([palette[c][1] for c in cls])
    las.blue = np.array([palette[c][2] for c in cls])
    las.write(str(FILES / "index-sample.laz"), do_compress=True, laz_backend=LazBackend.Laszip)


CASES = [
    ("many-chunks.laz", 100_000, 0),
    ("index-sample.laz", 400, 0),
    ("index-sample.laz", 400, 2),
]


def case(name, leaf_most, bin_depth):
    las, recs = wide_records(FILES / name)
    h = las.header
    bounds = [float(h.mins[0]), float(h.mins[1]), float(h.mins[2]), float(h.maxs[0]), float(h.maxs[1]), float(h.maxs[2])]
    t = Tree([float(v) for v in h.scales], [float(v) for v in h.offsets], bounds, leaf_most, bin_depth)
    nodes = t.build(recs)
    return {
        "file": name,
        "leafMost": leaf_most,
        "binDepth": bin_depth,
        "center": t.center,
        "half": t.half,
        "spacing": 2.0 * t.half / GRID,
        "count": sum(len(own) for _, own in nodes),
        "nodes": [{"key": list(k), "count": len(own), "hash": pc.fnv(b"".join(r for r, _, _ in own))} for k, own in nodes],
    }


def verify(path: Path):
    """A COPC KentOS wrote, read by LASzip (as the LAZ it also is) and by its own records, against the reference build."""
    b = path.read_bytes()
    assert b[:4] == b"LASF" and b[24:26] == bytes([1, 4]), "LAS 1.4 değil"
    fmt = b[104] & 0x3F
    assert b[104] & 0x80 and fmt in (6, 7, 8), "COPC 6, 7 ya da 8 biçiminde sıkıştırılmış olmalı"
    header_size = struct.unpack_from("<H", b, 94)[0]
    assert header_size == 375
    user = b[375 + 2: 375 + 18].split(b"\0")[0].decode()
    record = struct.unpack_from("<H", b, 375 + 18)[0]
    assert (user, record) == ("copc", 1), "ilk VLR COPC'nin bilgi VLR'si olmalı"
    info = struct.unpack_from("<5d2Q2d", b, 375 + 54)
    cx, cy, cz, half, spacing, root_off, root_size, gmin, gmax = info
    entries = []
    for i in range(0, root_size, 32):
        d, x, y, z, off, size, count = struct.unpack_from("<4iQii", b, root_off + i)
        assert count >= 0, "KentOS hiyerarşiyi tek sayfa yazar"
        entries.append(((d, x, y, z), off, size, count))
    las = laspy.read(str(path), laz_backend=LazBackend.Laszip)
    raw = las.points.array.tobytes()
    n = las.header.point_format.size
    assert len(raw) // n == sum(e[3] for e in entries), "hiyerarşinin noktaları dosyanınkine eşit değil"
    # Chunks follow each other in the file: the points in file order split by the entries sorted by offset.
    by_offset = sorted([e for e in entries if e[3] > 0], key=lambda e: e[1])
    at = 0
    got = {}
    for key, off, size, count in by_offset:
        got[key] = raw[at * n:(at + count) * n]
        at += count
    return {"cube": (cx, cy, cz, half), "spacing": spacing, "nodes": got, "entries": entries, "gps": (gmin, gmax)}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--verify", type=Path)
    ap.add_argument("--source", default="index-sample.laz")
    ap.add_argument("--leaf-most", type=int, default=400)
    ap.add_argument("--bin-depth", type=int, default=2)
    args = ap.parse_args()
    if args.verify:
        v = verify(args.verify)
        ref = case(args.source, args.leaf_most, args.bin_depth)
        expect = {tuple(nd["key"]): nd for nd in ref["nodes"]}
        assert set(expect) == {e[0] for e in v["entries"]}, "düğümler başvurununkilerle aynı değil"
        for key, nd in expect.items():
            recs = v["nodes"].get(key, b"")
            if nd["count"]:
                assert pc.fnv(recs) == nd["hash"], f"{key}: kayıtlar farklı"
            else:
                assert recs == b"", f"{key}: noktası olmamalı"
        assert (v["cube"][0], v["cube"][1], v["cube"][2], v["cube"][3]) == (ref["center"][0], ref["center"][1], ref["center"][2], ref["half"])
        assert v["spacing"] == ref["spacing"]
        print(f"{args.verify}: {len(expect)} düğüm, {ref['count']} nokta LASzip'in okuduğuyla ve başvurunun ağacıyla aynı")
        return
    if not args.check:
        build_sample()
    got = {"about": "docs/adr/0207 §4: written by scripts/fixtures/pointcloud_index_cases.py, not by KentOS", "cases": [case(*c) for c in CASES]}
    text = json.dumps(got, ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text() != text:
            sys.exit("pointcloud index.json farklı")
        print(f"{OUT.relative_to(ROOT)}: {len(got['cases'])} dizin aynı")
    else:
        OUT.write_text(text)
        print(f"wrote {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
