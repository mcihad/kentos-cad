"""Birleşme çizgisi ağı and Ayak izi (`cad.orthophoto.network`, `cad.orthophoto.footprint`, docs/adr/5260; plan/map-cad.md
MAP-12), held by the test's own reckoning: three north-up rectangles' elements cover their union (its area by
inclusion and exclusion of the rectangles), their one junction is the circumcentre of the rectangles' centres; the
footprint's area is its valid pixels (read here from the TIFF's own strip) times a pixel's area, its hull's the convex
hull's (Andrew's chain on the pixel corners, written here).
"""

from __future__ import annotations

import json
import struct
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

NET = json.loads((ROOT / "fixtures/commands/v1/cad.orthophoto.network.json").read_text("utf-8"))
FOOT = json.loads((ROOT / "fixtures/commands/v1/cad.orthophoto.footprint.json").read_text("utf-8"))


def case(cases, prefix):
    return next(c for c in cases["cases"] if c["name"].startswith(prefix))


def opened(setup):
    text = json.dumps(setup).replace("$fixture:", str(ROOT / "fixtures") + "/")
    return cad.Document.from_bytes(kr.container(kr.document(json.loads(text))))


def rect(e):
    t, w, h = e["affine"], e["width"], e["height"]
    return (t[0], t[3] + h * t[5], t[0] + w * t[1], t[3])


def inter(a, b):
    return (max(a[0], b[0]), max(a[1], b[1]), min(a[2], b[2]), min(a[3], b[3]))


def size(r):
    return max(0.0, r[2] - r[0]) * max(0.0, r[3] - r[1])


def circumcentre(a, b, c):
    d = 2 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]))
    ux = sum((p[0] ** 2 + p[1] ** 2) * (q[1] - r[1]) for p, q, r in ((a, b, c), (b, c, a), (c, a, b))) / d
    uy = sum((p[0] ** 2 + p[1] ** 2) * (r[0] - q[0]) for p, q, r in ((a, b, c), (b, c, a), (c, a, b))) / d
    return ux, uy


def strip(path):
    """An uncompressed one-strip 8-bit TIFF's width, height and samples, read here."""
    b = path.read_bytes()
    at = struct.unpack("<I", b[4:8])[0]
    n = struct.unpack("<H", b[at:at + 2])[0]
    tags = {}
    for k in range(n):
        tag, kind, count, value = struct.unpack("<HHII", b[at + 2 + 12 * k:at + 14 + 12 * k])
        tags[tag] = value & 0xFFFF if kind == 3 else value
    w, h, off = tags[256], tags[257], tags[273]
    return w, h, b[off:off + w * h]


def hull(points):
    pts = sorted(set(points))
    cross = lambda o, a, b: (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])  # noqa: E731
    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


class OrthophotoNetwork(unittest.TestCase):
    def test_three_rectangles_cover_their_union_and_meet_at_the_circumcentre(self) -> None:
        c = case(NET, "üç görüntü")
        doc = opened(c["setup"])
        out = cad.command("cad.orthophoto.network").run(doc, cad.command("cad.orthophoto.network").input_type.from_json(
            c["steps"][0]["input"])).output.to_json()
        a, b, d = (rect(e) for e in c["setup"]["entities"][:3])
        union = size(a) + size(b) + size(d) - size(inter(a, b)) - size(inter(a, d)) - size(inter(b, d)) + size(
            inter(inter(a, b), d))
        self.assertAlmostEqual(sum(e["area"] for e in out["elements"]), union, places=6)
        centres = [((r[0] + r[2]) / 2, (r[1] + r[3]) / 2) for r in (a, b, d)]
        [j] = out["junctions"]
        ux, uy = circumcentre(*centres)
        self.assertAlmostEqual(j["at"][0], ux, places=6)
        self.assertAlmostEqual(j["at"][1], uy, places=6)

    def test_the_footprint_is_its_valid_pixels_and_its_hull_the_corners_hull(self) -> None:
        c = case(FOOT, "NoData'dan ayak izi")
        r = c["setup"]["entities"][0]
        w, h, data = strip(ROOT / "fixtures/orthophoto/v1/files/ayak-izi-a.tif")
        valid = [(i, j) for j in range(h) for i in range(w) if data[j * w + i] != 0]
        pixel = abs(r["affine"][1] * r["affine"][5] - r["affine"][2] * r["affine"][4])
        run = cad.command("cad.orthophoto.footprint")
        out = run.run(opened(c["setup"]), run.input_type.from_json({"raster": c["steps"][0]["input"]["raster"]})).output.to_json()
        self.assertEqual(out["valid"], len(valid))
        self.assertAlmostEqual(out["area"], len(valid) * pixel, places=6)
        corners = [(i + di, j + dj) for i, j in valid for di in (0, 1) for dj in (0, 1)]
        ring = hull(corners)
        a2 = abs(sum(ring[k][0] * ring[(k + 1) % len(ring)][1] - ring[(k + 1) % len(ring)][0] * ring[k][1] for k in range(len(ring))))
        out = run.run(opened(c["setup"]), run.input_type.from_json({"raster": c["steps"][0]["input"]["raster"], "hull": True})).output.to_json()
        self.assertAlmostEqual(out["area"], a2 / 2 * pixel, places=6)
        self.assertEqual(len(out["parts"]), 1)


if __name__ == "__main__":
    unittest.main()
