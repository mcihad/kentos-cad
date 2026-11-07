"""`file.exchange.check` (Devir denetimi, docs/adr/1660; plan/map-cad.md MAP-17) against an independent reader.

The drawing is fixtures/exchange/v1/devir.json (scripts/fixtures/exchange_drawing.py). For each format the file is
written by KentOS (`file.export.*`) and read by GDAL's `ogrinfo` (DXF: ezdxf), no KentOS code. What the file holds,
by that reader, is held against the check's report, which is KentOS's writer and reader together:

- the objects not written (an original with no feature near it);
- curves become chords (an original with a curve, its feature without CIRCULARSTRING / COMPOUNDCURVE / CURVEPOLYGON;
  DXF: no bulge, no CIRCLE);
- heights and measures lost (an original with Z / M, its feature's type without them);
- attributes lost (an original's attribute name not among the file's fields);
- the greatest offset of the file's vertices from the original's exact geometry (lines, arcs, circles), within 1e-6 m
  of the report's.

Skipped where GDAL or ezdxf is not installed.
"""

from __future__ import annotations

import json
import math
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

DRAWING = json.loads((ROOT / "fixtures/exchange/v1/devir.json").read_text("utf-8"))
CURVES = ("CIRCULARSTRING", "COMPOUNDCURVE", "CURVEPOLYGON", "MULTICURVE", "MULTISURFACE")


def opened() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(DRAWING)))


# ── The originals' exact geometry ──────────────────────────────────────────────────────────────────────────────────
def to_segment(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    return math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)


def to_edge(p, a, b, bulge):
    if bulge == 0:
        return to_segment(p, a, b)
    chord = math.hypot(b[0] - a[0], b[1] - a[1])
    theta = 4 * math.atan(bulge)
    r = chord / (2 * abs(math.sin(theta / 2)))
    ux, uy = (b[0] - a[0]) / chord, (b[1] - a[1]) / chord
    k = chord * (1 - bulge * bulge) / (4 * bulge)
    c = ((a[0] + b[0]) / 2 - uy * k, (a[1] + b[1]) / 2 + ux * k)
    start = a if bulge > 0 else b
    a0 = math.atan2(start[1] - c[1], start[0] - c[0])
    rel = (math.atan2(p[1] - c[1], p[0] - c[0]) - a0) % (2 * math.pi)
    if rel <= abs(theta):
        return abs(math.hypot(p[0] - c[0], p[1] - c[1]) - r)
    return min(math.hypot(p[0] - a[0], p[1] - a[1]), math.hypot(p[0] - b[0], p[1] - b[1]))


def xy(v):
    return (v["x"], v["y"])


def ring(p, pts, bulges, closed):
    n = len(pts)
    edges = n if closed else n - 1
    return min(to_edge(p, xy(pts[i]), xy(pts[(i + 1) % n]), (bulges or [0.0] * n)[i]) for i in range(edges))


def to_original(e, p):
    k = e["kind"]
    if k == "point":
        return math.hypot(p[0] - e["p"]["x"], p[1] - e["p"]["y"])
    if k == "circle":
        return abs(math.hypot(p[0] - e["c"]["x"], p[1] - e["c"]["y"]) - e["r"])
    d = ring(p, e["pts"], e.get("bulges"), k == "polygon")
    for h in e.get("holes", []):
        d = min(d, ring(p, h["pts"], h.get("bulges"), True))
    return d


def centre(e):
    if e["kind"] == "point":
        return xy(e["p"])
    if e["kind"] == "circle":
        return xy(e["c"])
    if e["kind"] == "text":
        return xy(e["p"])
    xs = [v["x"] for v in e["pts"]]
    ys = [v["y"] for v in e["pts"]]
    return ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)


def curved(e):
    return e["kind"] == "circle" or any(b != 0 for b in e.get("bulges") or [])


# ── What GDAL reads ────────────────────────────────────────────────────────────────────────────────────────────────
def ogr_features(path: Path) -> list[dict]:
    """Each feature's geometry WKT (its type and numbers) and field names, by ogrinfo's text listing."""
    name = f"/vsizip/{path}" if path.suffix == ".zip" else str(path)
    out = subprocess.run(["ogrinfo", "-ro", "-al", "-q", name], capture_output=True, text=True, check=True).stdout
    feats, cur = [], None
    for line in out.splitlines():
        if line.startswith("OGRFeature"):
            cur = {"fields": set(), "wkt": None}
            feats.append(cur)
        elif cur is not None:
            m = re.match(r"\s+(.+?) \(\w+(?:\(\d+(?:\.\d+)?\))?\) = ", line)
            if m:
                cur["fields"].add(m.group(1))
            elif re.match(r"\s+(POINT|LINESTRING|POLYGON|MULTI|CIRCULARSTRING|COMPOUNDCURVE|CURVEPOLYGON)", line):
                cur["wkt"] = line.strip()
    return [f for f in feats if f["wkt"]]


def wkt_numbers(wkt: str) -> list[float]:
    return [float(n) for n in re.findall(r"-?\d+(?:\.\d+)?(?:e[-+]?\d+)?|nan", wkt[wkt.index("("):])]


def wkt_read(wkt: str) -> tuple[list[tuple[float, float]], bool, bool]:
    """The vertices in plan, and whether any vertex has a height and a measure (FlatGeobuf writes ZM with NaN where
    there is none)."""
    head = wkt[: wkt.index("(")].upper().split()
    z, m = "ZM" in head or "Z" in head, "ZM" in head or "M" in head
    dims = 2 + z + m
    vals = wkt_numbers(wkt)
    rows = [vals[i : i + dims] for i in range(0, len(vals) - dims + 1, dims)]
    has = lambda k: any(not math.isnan(r[k]) for r in rows)
    return [(r[0], r[1]) for r in rows], z and has(2), m and has(2 + z)


def nearest(originals, at):
    return min(originals, key=lambda e: math.hypot(centre(e)[0] - at[0], centre(e)[1] - at[1]))


class ExchangeAgainstGdal(unittest.TestCase):
    def check(self, fmt: str, export: str, ext: str) -> None:
        if not shutil.which("ogrinfo"):
            self.skipTest("GDAL yok")
        folder = Path(tempfile.mkdtemp())
        try:
            doc = opened()
            w = cad.command(export)
            out = w.run(doc, w.input_type.from_json({"path": str(folder / f"devir.{ext}")}), op="execute")
            self.assertIsNone(out.error, out.error and out.error.message)
            path = folder / f"devir.{ext}"
            feats = ogr_features(path)
            report = self.report(fmt)
            originals = [e for e in DRAWING["entities"] if e["kind"] != "text"]
            # Every geometry original has a feature near it; the text has none.
            got = {}
            for f in feats:
                pts, z, m = wkt_read(f["wkt"])
                at = ((min(p[0] for p in pts) + max(p[0] for p in pts)) / 2, (min(p[1] for p in pts) + max(p[1] for p in pts)) / 2)
                e = nearest(originals, at)
                got.setdefault(id(e), (e, []))[1].append((f, pts, z, m))
            self.assertEqual(len(got), len(originals), fmt)
            self.assertEqual([k["kind"] for k in report["notWritten"]], ["text"], fmt)
            losses = {(l["kind"], l.get("detail")): l["count"] for l in report["losses"]}
            chorded = sum(1 for e, fs in got.values() if curved(e) and not any(f["wkt"].upper().startswith(CURVES) for f, *_ in fs))
            self.assertEqual(chorded, sum(c for (k, _), c in losses.items() if k == "chorded"), fmt)
            m_lost = sum(1 for e, fs in got.values() if e.get("ms") and not any(m for *_, m in fs))
            self.assertEqual(m_lost, losses.get(("mLost", None), 0), fmt)
            z_lost = sum(1 for e, fs in got.values() if (e.get("z") is not None or e.get("zs")) and not any(z for _, _, z, _ in fs))
            self.assertEqual(z_lost, losses.get(("zLost", None), 0), fmt)
            for e, fs in got.values():
                fields = set().union(*(f["fields"] for f, *_ in fs))
                for name in e["attrs"]:
                    lost = name not in fields
                    self.assertEqual(lost, ("attrLost", name) in losses, f"{fmt}: {name}")
            offset = max(to_original(e, p) for e, fs in got.values() for _, pts, _, _ in fs for p in pts)
            self.assertAlmostEqual(offset, report["numeric"]["offset"], delta=1e-6, msg=fmt)
        finally:
            shutil.rmtree(folder, ignore_errors=True)

    def report(self, fmt: str) -> dict:
        w = cad.command("file.exchange.check")
        out = w.run(opened(), w.input_type.from_json({"format": fmt}), op="execute")
        self.assertIsNone(out.error, out.error and out.error.message)
        return out.output.to_json()

    def test_geopackage(self) -> None:
        self.check("gpkg", "file.export.gpkg", "gpkg")

    def test_geoparquet(self) -> None:
        self.check("parquet", "file.export.parquet", "parquet")

    def test_flatgeobuf(self) -> None:
        self.check("fgb", "file.export.fgb", "fgb")

    def test_gml(self) -> None:
        self.check("gml", "file.export.gml", "gml")

    def test_shapefile(self) -> None:
        self.check("shp", "file.export.shp", "zip")

    def test_dxf_by_ezdxf(self) -> None:
        try:
            import ezdxf
        except ImportError:
            self.skipTest("ezdxf yok")
        folder = Path(tempfile.mkdtemp())
        try:
            w = cad.command("file.export.dxf")
            path = folder / "devir.dxf"
            out = w.run(opened(), w.input_type.from_json({"path": str(path)}), op="execute")
            self.assertIsNone(out.error, out.error and out.error.message)
            msp = ezdxf.readfile(str(path)).modelspace()
            kinds = sorted(e.dxftype() for e in msp)
            report = self.report("dxf")
            losses = {(l["kind"], l.get("detail")): l["count"] for l in report["losses"]}
            # The parcel's hole is a closed polyline of its own: the polygon becomes a polyline and one is extra.
            self.assertEqual(kinds.count("LWPOLYLINE") + kinds.count("POLYLINE"), 4)
            self.assertEqual(losses.get(("kindChanged", "polygon → polyline")), 1)
            self.assertEqual(losses.get(("extra", "polyline")), 1)
            self.assertIn("CIRCLE", kinds)
            self.assertIn("TEXT", kinds + [k.replace("MTEXT", "TEXT") for k in kinds])
            self.assertNotIn(("chorded", "polyline"), losses, "DXF keeps bulges")
            bulged = [e for e in msp if e.dxftype() == "LWPOLYLINE" and any(b for *_, b in e.get_points("xyseb"))]
            self.assertEqual(len(bulged), 1)
            self.assertEqual(losses.get(("mLost", None)), 1, "DXF has no M")
            self.assertEqual(report["notWritten"], [])
        finally:
            shutil.rmtree(folder, ignore_errors=True)


if __name__ == "__main__":
    unittest.main()
