"""`file.exchange.check` (Dışa aktarma kaybı denetimi, docs/adr/1660; plan/map-cad.md MAP-17) against an independent reader.

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


BLOCK_DRAWING = json.loads((ROOT / "fixtures/exchange/v1/devir-blok.json").read_text("utf-8"))


def opened_block() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(BLOCK_DRAWING)))


def run_command(name: str, doc: cad.Document, args: dict) -> dict:
    w = cad.command(name)
    out = w.run(doc, w.input_type.from_json(args), op="execute")
    if out.error is not None:
        raise AssertionError(out.error.message)
    return out.output.to_json() if out.output is not None else {}


class BlocksTablesRelations(unittest.TestCase):
    """docs/adr/2380 on fixtures/exchange/v1/devir-blok.json: a block, a table and a relation of the project. What the
    files hold is read by GDAL (the GeoPackage's related tables) and ezdxf (the DXF's block and the table's anonymous
    block), no KentOS code; the check's rows are held against it."""

    def matrix(self) -> dict:
        return run_command("file.exchange.matrix", opened_block(), {})

    def test_the_rows_say_what_the_files_hold(self) -> None:
        m = self.matrix()
        rows = {r["aspect"]: {c["format"]: c["state"] for c in r["cells"]} for r in m["rows"]}
        for f in ("gpkg", "parquet", "fgb", "gml", "shp", "dxf"):
            self.assertEqual(rows["blocks"][f], "kept" if f == "dxf" else "lost", f)
            self.assertEqual(rows["tables"][f], "kept" if f == "dxf" else "lost", f)
            self.assertEqual(rows["relations"][f], "kept" if f == "gpkg" else "lost", f)

    def test_the_geopackage_holds_the_relation_by_gdal(self) -> None:
        if shutil.which("ogrinfo") is None:
            self.skipTest("GDAL yok")
        folder = Path(tempfile.mkdtemp())
        try:
            path = folder / "devir.gpkg"
            run_command("file.export.gpkg", opened_block(), {"path": str(path)})
            sql = ("SELECT base_table_name, related_table_name, mapping_table_name FROM gpkgext_relations")
            text = subprocess.run(["ogrinfo", "-ro", "-q", "-sql", sql, str(path)], capture_output=True, text=True,
                                  check=True).stdout
            self.assertIn("base_table_name (String) = Malik", text)
            self.assertIn("related_table_name (String) = Parsel", text)
            mapping = re.search(r"mapping_table_name \(String\) = (\S+)", text).group(1)
            # The mapping row joins Malik's point to the parcel (the only feature of each table).
            rows = subprocess.run(["ogrinfo", "-ro", "-q", "-sql", f"SELECT base_id, related_id FROM \"{mapping}\"",
                                   str(path)], capture_output=True, text=True, check=True).stdout
            self.assertEqual(len(re.findall(r"base_id \(Integer(?:64)?\) = \d+", rows)), 1, rows)
            features = sum(len(ogr_features_of(path, layer)) for layer in ("Malik", "Parsel"))
            self.assertEqual(features, 2)
            # The insert and the table are in no table of the file.
            self.assertEqual(sum(len(f) for f in [ogr_features(path)]), len(BLOCK_DRAWING["entities"]) - 3)
        finally:
            shutil.rmtree(folder, ignore_errors=True)

    def test_the_dxf_holds_the_block_and_draws_the_table_by_ezdxf(self) -> None:
        try:
            import ezdxf
        except ImportError:
            self.skipTest("ezdxf yok")
        folder = Path(tempfile.mkdtemp())
        try:
            path = folder / "devir.dxf"
            run_command("file.export.dxf", opened_block(), {"path": str(path)})
            x = ezdxf.readfile(str(path))
            inserts = [e for e in x.modelspace() if e.dxftype() == "INSERT"]
            origin = BLOCK_DRAWING["origin"]
            original = {e["kind"]: e for e in BLOCK_DRAWING["entities"] if e["kind"] in ("insert", "table")}
            rogar = [e for e in inserts if e.dxf.name == "Rögar"]
            self.assertEqual(len(rogar), 1)
            p = original["insert"]["p"]
            self.assertAlmostEqual(rogar[0].dxf.insert.x, p["x"], delta=1e-9)
            self.assertAlmostEqual(rogar[0].dxf.insert.y, p["y"], delta=1e-9)
            block = [(e.dxftype(), round(getattr(e.dxf, "radius", 0.0), 9), getattr(e.dxf, "text", None))
                     for e in x.blocks.get("Rögar")]
            self.assertEqual(block, [("CIRCLE", 0.75, None), ("TEXT", 0.0, "R")])
            # The table: an anonymous block drawing its lines and cells, its cells also in KentOS's data; another
            # program sees lines and texts, KentOS reads a table back.
            drawn = [e for e in inserts if e.dxf.name.startswith("*U")]
            self.assertEqual(len(drawn), 1)
            texts = [e.dxf.text for e in x.blocks.get(drawn[0].dxf.name) if e.dxftype() == "TEXT"]
            cells = original["table"]["cells"]
            self.assertEqual(texts, [c for row in cells for c in row])
            lines = [e for e in x.blocks.get(drawn[0].dxf.name) if e.dxftype() == "LINE"]
            self.assertEqual(len(lines), (len(cells) + 1) + (len(cells[0]) + 1))
            data = [t.value for t in drawn[0].get_xdata("KENTOS") if t.code == 1000]
            self.assertEqual(json.loads(data[data.index("table") + 1])["cells"], cells)
            t = original["table"]["p"]
            self.assertAlmostEqual(drawn[0].dxf.insert.x, t["x"], delta=1e-9)
            self.assertAlmostEqual(drawn[0].dxf.insert.y, t["y"], delta=1e-9)
            self.assertEqual(origin, BLOCK_DRAWING["origin"])
        finally:
            shutil.rmtree(folder, ignore_errors=True)


def ogr_features_of(path: Path, layer: str) -> list[str]:
    text = subprocess.run(["ogrinfo", "-ro", "-q", str(path), layer], capture_output=True, text=True).stdout
    return re.findall(r"^OGRFeature\(", text, flags=re.M)


# Docs/adr/1880: a matrix cell's rules, written here from the ADR, not from KentOS's code.
FIRST = {
    "kinds": [("kindChanged", "converted")],
    "curves": [("chorded", "approximated")],
    "z": [("zLost", "lost"), ("zAdded", "added")],
    "m": [("mLost", "lost")],
    "attributes": [("attrLost", "lost"), ("attrChanged", "converted"), ("attrAdded", "added")],
    "style": [("styleLost", "lost")],
    "identity": [("identityLost", "lost")],
    "extra": [("extra", "added")],
    "relations": [("relationLost", "lost")],
}
# Docs/adr/2380: an object kind's row (inserts, tables): lost when not written, else converted when read back as
# another kind or changed.
PARTS = {"blocks": ("insert", "blockChanged"), "tables": ("table", "tableChanged")}


def expected_cell(check: dict, aspect: str) -> tuple[str, int]:
    if aspect == "objects":
        n = sum(k["count"] for k in check["notWritten"])
        return ("lost" if n else "kept"), n
    if aspect == "position":
        above = check["numeric"]["above"]
        return ("approximated" if above else "kept"), above
    if aspect in PARTS:
        kind, changed = PARTS[aspect]
        n = sum(k["count"] for k in check["notWritten"] if k["kind"] == kind)
        if n:
            return "lost", n
        n = sum(l["count"] for l in check["losses"] if l["kind"] == "kindChanged"
                and (l.get("detail") or "").startswith(f"{kind} → "))
        if n:
            return "converted", n
        n = sum(l["count"] for l in check["losses"] if l["kind"] == changed)
        return ("converted" if n else "kept"), n
    for kind, state in FIRST[aspect]:
        n = sum(l["count"] for l in check["losses"] if l["kind"] == kind)
        if n:
            return state, n
    return "kept", 0


class MatrixFromTheChecks(unittest.TestCase):
    """`file.exchange.matrix` (docs/adr/1880): its checks are `file.exchange.check`'s, format by format (which the GDAL
    tests above hold to the files), and every cell follows the ADR's rules from its format's check."""

    def test_the_matrix_reads_each_format_s_check_by_the_rules(self) -> None:
        for drawing in (opened, opened_block):
            w = cad.command("file.exchange.matrix")
            out = w.run(drawing(), w.input_type.from_json({}), op="execute")
            self.assertIsNone(out.error, out.error and out.error.message)
            m = out.output.to_json()
            formats = [c["format"] for c in m["checks"]]
            self.assertEqual(formats, ["gpkg", "parquet", "fgb", "gml", "shp", "dxf"])
            check = cad.command("file.exchange.check")
            for c in m["checks"]:
                alone = check.run(drawing(), check.input_type.from_json({"format": c["format"]}),
                                  op="execute").output.to_json()
                self.assertEqual(c, alone, c["format"])
            aspects = ["objects", "kinds", "curves", "z", "m", "attributes", "style", "identity", "position", "extra",
                       "blocks", "tables", "relations"]
            self.assertEqual([r["aspect"] for r in m["rows"]], aspects)
            for row in m["rows"]:
                for cell, c in zip(row["cells"], m["checks"]):
                    self.assertEqual(cell["format"], c["format"])
                    self.assertEqual((cell["state"], cell["count"]), expected_cell(c, row["aspect"]),
                                     f"{drawing.__name__} {row['aspect']} {c['format']}")


if __name__ == "__main__":
    unittest.main()
