#!/usr/bin/env python3
"""The derived GeoPackage of the CBS-16 cases read with OGR, without KentOS code (docs/adr/0530), and the edited copies
of it the shared cases read.

    <QGIS 4.2's python3.12> scripts/fixtures/gis_derivation_files.py            # reads ada.gpkg and its copies, checks them
    <QGIS 4.2's python3.12> scripts/fixtures/gis_derivation_files.py --files    # makes the copies from ada.gpkg, then checks

(GDAL 3.13 and numpy come with QGIS: /Applications/QGIS-final-4_2_2.app/Contents/MacOS/python3.12 with PYTHONHOME and
PYTHONPATH of its Resources.) `ada.gpkg` is what `file.export.gis` writes of `drawing.json`
(`KENTOS_WRITE_GPKG=1 cargo test -p kentos-headless --test gis`); the copies are `ada-duzenli` (an attribute, a geometry and
a text changed as a GIS would), `ada-eksik` (a row deleted) and `ada-yabanci` (a row's id changed).

What is checked, each from the drawing's own numbers and the curves' own definitions:

- the tables, their columns, row counts, fids and ids (`kentos_uid` and `kentos_uids` agree, none twice);
- each row's kind, and its deviation: an arc's exactly (the sagitta 2·r·sin²(φ/4) of its chords' angle, φ = θ/n, n the
  least count that holds the tolerance with no chord over a quarter turn), an ellipse's, a curve's and a NURBS's the
  tolerance (a bound);
- an arc's, circle's and bulged edge's chords: every one of its vertices on the circle, the count n + 1, the ends the
  source's, a circle's and an arc's chords of one angle;
- every curved row sampled along the true curve (circle, arc, bulged edge, ellipse, the NURBS quarter circle) 20 000
  times: the farthest a sample is from the written chords is within the deviation, and for an arc it is the deviation
  (the recorded value is no larger than the truth);
- Z and M carried: a vertex of the source has its own, a chord's vertex the value at its fraction of the arc;
- rings closed and valid, the parcel's area: OGR's area of the written polygon against the shoelace of its corners with
  each arc's circular segment taken away by its chords' slivers, and against the analytic area (the difference is within
  the arcs' length times their deviation);
- the metadata: one derivation document a table, its fields as the ADR gives them;
- GDAL's `validate_gpkg.py` over the dataset and its copies: no finding QGIS's own GeoPackage lacks;
- the edited copies differ from `ada.gpkg` in exactly what was changed.
"""
import json
import math
import shutil
import sqlite3
import sys
import uuid
from pathlib import Path

import numpy as np
from osgeo import gdal, ogr

gdal.UseExceptions()
ogr.UseExceptions()
sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/gis-derivation/v1"
DRAWING = json.loads((DIR / "drawing.json").read_text())
ENTS = {e["id"]: e for e in DRAWING["entities"]}
E, N = DRAWING["origin"]["x"], DRAWING["origin"]["y"]
TOL = 1e-4

# Each table, its role and the source of each row (a row's fid is its place).
TABLES = {
    "Parsel": ("layer", [1, 2, 3, 4]),
    "blok_Bina": ("blocks", [15, 16]),
    "Bina": ("layer", [17]),
    "Yol": ("layer", [5, 6, 7, 8]),
    "kentos_annotations": ("annotations", [14]),
    "Çizim": ("layer", [9, 10, 11, 12, 13]),
}


def fail(message):
    sys.exit(f"gis_derivation_files: {message}")


def check(ok, message):
    if not ok:
        fail(message)


# ── The curves' own definitions ─────────────────────────────────────────────

def sagitta(r, step):
    return 2 * r * math.sin(abs(step) / 4) ** 2


def least_count(r, theta, tol):
    """The least n with sagitta(r, θ/n) ≤ tol and θ/n ≤ π/2."""
    n = max(1, math.ceil(theta / (math.pi / 2) - 1e-12))
    while sagitta(r, theta / n) > tol:
        n += 1
    return n


def segment_area(r, sweep):
    """The area between an arc of radius r and sweep and its chord."""
    return r * r / 2 * (abs(sweep) - math.sin(abs(sweep)))


def rel(p):
    return (p["x"] - E, p["y"] - N)


def bulge_circle(a, b, bulge):
    """The centre, radius, start angle and signed sweep of a bulged edge (DXF's bulge is tan(θ/4))."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    k = (1 - bulge * bulge) / (4 * bulge)
    c = ((a[0] + b[0]) / 2 - dy * k, (a[1] + b[1]) / 2 + dx * k)
    r = chord * (1 + bulge * bulge) / (4 * abs(bulge))
    return c, r, math.atan2(a[1] - c[1], a[0] - c[0]), 4 * math.atan(bulge)


class Arc:
    def __init__(self, c, r, a0, sweep, start=None, end=None):
        self.c, self.r, self.a0, self.sweep, self.start, self.end = np.array(c), r, a0, sweep, start, end
        self.n = least_count(r, abs(sweep), TOL)
        self.deviation = sagitta(r, abs(sweep) / self.n)

    def samples(self, count=20000):
        t = self.a0 + self.sweep * np.linspace(0.0, 1.0, count)
        return np.column_stack([self.c[0] + self.r * np.cos(t), self.c[1] + self.r * np.sin(t)])

    def holds(self, p):
        """Whether a vertex is on the circle and within the sweep."""
        v = p - self.c
        if abs(math.hypot(*v) - self.r) > 1e-7 * max(1.0, self.r):
            return False
        t = ((math.atan2(v[1], v[0]) - self.a0) * (1 if self.sweep > 0 else -1)) % (2 * math.pi)
        return t <= abs(self.sweep) + 1e-9 or t >= 2 * math.pi - 1e-9


def rings_of(e):
    """A source's paths as (points, bulges, closed): the outer ring and the holes of an area, an open or closed line."""
    k = e["kind"]
    out = [(e["pts"], e.get("bulges"), k == "polygon" or e.get("closed", False))]
    for h in e.get("holes", []):
        out.append((h["pts"], h.get("bulges"), True))
    return out


def arcs_of(e):
    k = e["kind"]
    if k == "circle":
        return [Arc(rel(e["c"]), e["r"], 0.0, 2 * math.pi)]
    if k == "arc":
        return [Arc(rel(e["c"]), e["r"], e["a0"], e["a1"] - e["a0"])]
    out = []
    if k in ("polygon", "polyline"):
        for pts, bulges, closed in rings_of(e):
            n = len(pts)
            for i in range(n if closed else n - 1):
                bl = bulges[i] if bulges and i < len(bulges) else 0
                if abs(bl) > 1e-12:
                    a, b = rel(pts[i]), rel(pts[(i + 1) % n])
                    c, r, a0, sweep = bulge_circle(a, b, bl)
                    out.append(Arc(c, r, a0, sweep, a, b))
    return out


def to_segment(points, a, b):
    d = b - a
    len2 = float(d @ d)
    if len2 == 0.0:
        return np.hypot(*(points - a).T)
    t = np.clip(((points - a) @ d) / len2, 0.0, 1.0)
    return np.hypot(*(points - (a + np.outer(t, d))).T)


def far_from(samples, rings):
    """The farthest a sample is from the nearest chord of any of `rings` (arrays of points)."""
    best = np.full(len(samples), np.inf)
    for pts in rings:
        for a, b in zip(pts[:-1], pts[1:]):
            best = np.minimum(best, to_segment(samples, a, b))
    return float(best.max())


def ellipse_samples(e, count=20000):
    major = np.array([e["major"]["x"], e["major"]["y"]])
    minor = np.array([-major[1], major[0]]) * e["ratio"]
    t = np.linspace(e["t0"], e["t1"], count)
    return np.array(rel(e["c"])) + np.outer(np.cos(t), major) + np.outer(np.sin(t), minor)


# ── Reading the dataset ─────────────────────────────────────────────────────

def vertex_rings(geom):
    """A polygon's rings or a line's points about the origin, as arrays of (x, y)."""
    t = ogr.GT_Flatten(geom.GetGeometryType())
    if t == ogr.wkbPolygon:
        return [np.array([(x - E, y - N) for x, y, *_ in geom.GetGeometryRef(i).GetPoints()])
                for i in range(geom.GetGeometryCount())]
    if t == ogr.wkbLineString:
        return [np.array([(x - E, y - N) for x, y, *_ in geom.GetPoints()])]
    fail(f"unexpected geometry type {t}")


def read_rows(ds, table):
    layer = ds.GetLayerByName(table)
    defn = layer.GetLayerDefn()
    names = [defn.GetFieldDefn(i).GetName() for i in range(defn.GetFieldCount())]
    rows = []
    for f in layer:
        g = f.GetGeometryRef()
        rows.append({"fid": f.GetFID(), "fields": {n: f.GetField(n) for n in names},
                     "geom": g.Clone() if g is not None else None})
    return rows, names


def read_uids(path):
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    out = {(t, fid): uid for t, fid, uid in c.execute("SELECT table_name, fid, uid FROM kentos_uids")}
    c.close()
    return out


def read_records(path):
    c = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    ext = {r[0] for r in c.execute("SELECT table_name FROM gpkg_extensions WHERE extension_name = 'gpkg_metadata'")}
    check({"gpkg_metadata", "gpkg_metadata_reference"} <= ext, "the metadata extension is not registered")
    rows = c.execute(
        "SELECT r.table_name, r.reference_scope, m.md_scope, m.md_standard_uri, m.mime_type, m.metadata "
        "FROM gpkg_metadata_reference r JOIN gpkg_metadata m ON m.id = r.md_file_id ORDER BY m.id").fetchall()
    c.close()
    out = {}
    for table, scope, md_scope, uri, mime, text in rows:
        check((scope, md_scope, uri, mime) == ("table", "dataset", "https://kentos.app/derivation/1", "application/json"),
              f"{table}: the metadata row is {(scope, md_scope, uri, mime)}")
        out[table] = json.loads(text)
    return out


def check_dataset(path, rows_too=True):
    ds = ogr.Open(str(path))
    uids = read_uids(path)
    records = read_records(path)
    check(set(records) == set(TABLES), f"records for {sorted(records)}")
    seen = set()
    for table, (role, slots) in TABLES.items():
        layer = ds.GetLayerByName(table)
        check(layer is not None, f"{table} is not in the file")
        rows, names = read_rows(ds, table)
        if rows_too:
            check(len(rows) == len(slots), f"{table}: {len(rows)} rows")
        head = ["kentos_uid", "kentos_kind", "kentos_deviation", "kentos_digest"]
        check(names[:4] == head, f"{table}: columns {names}")
        if role != "layer":
            check(names[4] == "kentos_layer", f"{table}: columns {names}")
        rec = records[table]
        check(rec["format"] == "kentos.derivation" and rec["version"] == 1, f"{table}: record {rec}")
        check(rec["algorithm"] == "kentos.chords/1" and rec["tolerance"] == TOL and rec["unit"] == "m", f"{table}: {rec}")
        check(rec["role"] == role and rec["rows"] == len(slots), f"{table}: {rec}")
        check(rec["source"]["name"] == DRAWING["name"] and rec["source"]["revision"] == "0"
              and rec["source"]["srid"] == 5256, f"{table}: source {rec['source']}")
        check(rec["digest"] == "kentos.derivation-digest/1" and rec["made"].endswith("Z"), f"{table}: {rec}")
        check(rec["layers"] == [lay["id"] for lay in DRAWING["layers"]], f"{table}: layers {rec['layers']}")
        kinds = {ENTS[s]["kind"] for s in slots}
        check(set(rec["kinds"]) == kinds, f"{table}: kinds {rec['kinds']} against {kinds}")
        check(all(v in ("exact", "bound", "sampled", "none") for v in rec["kinds"].values()), f"{table}: {rec['kinds']}")
        if role == "layer":
            check(rec["layer"]["id"] == ENTS[slots[0]]["layerId"], f"{table}: layer {rec['layer']}")
        if role == "blocks":
            check(rec["block"] == "Bina", f"{table}: block {rec.get('block')}")
        if not rows_too:
            continue
        devs = []
        for fid, (row, slot) in enumerate(zip(rows, slots), start=1):
            e, f = ENTS[slot], row["fields"]
            where = f"{table} {fid} ({e['kind']})"
            check(row["fid"] == fid, f"{where}: fid {row['fid']}")
            check(f["kentos_uid"] == uids[(table, fid)], f"{where}: kentos_uid differs from kentos_uids")
            check(f["kentos_uid"] not in seen, f"{where}: id twice")
            seen.add(f["kentos_uid"])
            uuid.UUID(f["kentos_uid"])
            check(f["kentos_kind"] == e["kind"], f"{where}: kind {f['kentos_kind']}")
            check(len(f["kentos_digest"]) == 64 and int(f["kentos_digest"], 16) >= 0, f"{where}: digest")
            devs.append(f["kentos_deviation"])
            check_row(where, e, f, row["geom"])
        check(rec["maxDeviation"] == max(devs), f"{table}: record maxDeviation {rec['maxDeviation']} against {max(devs)}")
    if rows_too:
        check(len(seen) == 17, f"{len(seen)} ids")


def check_row(where, e, f, geom):
    k = e["kind"]
    if k == "record":
        check(geom is None, f"{where}: a record has no geometry")
        return
    check(geom is not None and not geom.IsEmpty(), f"{where}: no geometry")
    # The attributes a layer's fields hold, as written.
    for name, value in e["attrs"].items():
        if name in f:
            check(str(f[name]) == value, f"{where}: {name} is {f[name]}, not {value}")
    if k in ("text", "insert", "point"):
        check(ogr.GT_Flatten(geom.GetGeometryType()) == ogr.wkbPoint, f"{where}: a point")
        p = rel(e["p"])
        check(abs(geom.GetX() - E - p[0]) < 1e-9 and abs(geom.GetY() - N - p[1]) < 1e-9, f"{where}: elsewhere")
        check(f["kentos_deviation"] == 0.0, f"{where}: deviation {f['kentos_deviation']}")
        if k == "text":
            check(f["text"] == e["text"] and f["height"] == e["height"] and f["rotation_deg"] == e["rotation"], f"{where}: {f}")
        if k == "insert":
            check(f["scale"] == e["scale"] and abs(f["rotation_deg"] - math.degrees(e["rotation"])) < 1e-12, f"{where}: {f}")
            for a in DRAWING["blocks"][0]["attributes"]:
                want = e["attrs"].get(a["tag"], a.get("value"))
                check(f[a["tag"]] == want, f"{where}: {a['tag']} is {f[a['tag']]}, not {want}")
            check(f["kentos_layer"] == "Bina", f"{where}: layer {f['kentos_layer']}")
        if k == "point":
            check(geom.Is3D() and geom.GetZ() == e["z"], f"{where}: z {geom.GetZ()}")
        return
    rings = vertex_rings(geom)
    arcs = arcs_of(e)
    flat = [p for ring in rings for p in ring]
    # The deviation: exact for an arc, the tolerance for the bounded kinds, none for straight edges.
    if k in ("ellipse", "spline", "nurbs"):
        check(f["kentos_deviation"] == TOL, f"{where}: a bound, not {f['kentos_deviation']}")
    elif arcs:
        want = max(a.deviation for a in arcs)
        check(abs(f["kentos_deviation"] - want) <= 1e-12 * max(1.0, want), f"{where}: deviation {f['kentos_deviation']}, not {want}")
        check(f["kentos_deviation"] <= TOL, f"{where}: over the tolerance")
    else:
        check(f["kentos_deviation"] == 0.0, f"{where}: straight, deviation {f['kentos_deviation']}")
    if ogr.GT_Flatten(geom.GetGeometryType()) == ogr.wkbPolygon:
        for ring in rings:
            check(np.allclose(ring[0], ring[-1], atol=1e-12), f"{where}: a ring is not closed")
        check(geom.IsValid(), f"{where}: not a valid polygon")
    # Chords of an arc: on the circle, n of them, the ends the source's.
    for a in arcs:
        on = [p for p in flat if a.holds(p)]
        if k in ("circle", "arc"):
            check(len(on) == a.n + 1, f"{where}: {len(on)} points on the circle for {a.n} chords")
            ang = sorted(((math.atan2(p[1] - a.c[1], p[0] - a.c[0]) - a.a0) * (1 if a.sweep > 0 else -1)) % (2 * math.pi)
                         for p in on[:-1] if True)
            ang = [t if t < 2 * math.pi - 1e-9 else t - 2 * math.pi for t in ang]
            ang = sorted(ang)
            steps = np.diff(ang + [ang[0] + abs(a.sweep)]) if k == "circle" else np.diff(ang)
            check(np.allclose(steps, abs(a.sweep) / a.n, atol=1e-9), f"{where}: the chords are not of one angle")
        else:
            check(len(on) >= a.n + 1, f"{where}: {len(on)} points on an arc of {a.n} chords")
            for end in (a.start, a.end):
                check(any(np.allclose(p, end, atol=1e-9) for p in flat), f"{where}: an arc edge's end {end} is not a vertex")
    if k == "arc":
        a = arcs[0]
        check(np.allclose(rings[0][0], a.c + a.r * np.array([math.cos(a.a0), math.sin(a.a0)]), atol=1e-9), f"{where}: start")
        check(np.allclose(rings[0][-1], a.c + a.r * np.array([math.cos(a.a0 + a.sweep), math.sin(a.a0 + a.sweep)]), atol=1e-9), f"{where}: end")
    # Sampled along the true curve.
    for a in arcs:
        far = far_from(a.samples(), rings)
        check(far <= a.deviation * (1 + 1e-6) + 1e-9, f"{where}: a point of the arc is {far} from the chords, deviation {a.deviation}")
        check(far >= 0.99 * a.deviation, f"{where}: the chords come only {far} from the arc, the row says {a.deviation}")
    if k == "ellipse":
        far = far_from(ellipse_samples(e), rings)
        check(far <= TOL * (1 + 1e-6) + 1e-9, f"{where}: an ellipse point is {far} from the chords")
    if k == "nurbs":
        # The rational quarter circle of radius 8 about (300, 0).
        t = np.linspace(0.0, math.pi / 2, 20000)
        s = np.column_stack([300.0 + 8.0 * np.cos(t), 8.0 * np.sin(t)])
        far = far_from(s, rings)
        check(far <= TOL * (1 + 1e-6) + 1e-9, f"{where}: a NURBS point is {far} from the chords")
        check(all(abs(math.hypot(p[0] - 300.0, p[1]) - 8.0) < 1e-4 for p in rings[0]), f"{where}: a vertex off the circle")
    if k == "spline":
        for p in e["pts"]:
            q = np.array(rel(p))
            check(any(np.allclose(v, q, atol=1e-9) for v in rings[0]), f"{where}: fit point {q} is not a vertex")
    check_zm(where, e, geom)
    if k == "polygon" and e["attrs"].get("parsel_no") == "7":
        check_area(where, e, geom)


def check_zm(where, e, geom):
    k = e["kind"]
    if k == "line":
        pts = geom.GetPoints()
        check(pts[0][2] == e["za"] and pts[1][2] == e["zb"], f"{where}: z {pts}")
        return
    zs, ms = e.get("zs"), e.get("ms")
    if k == "polyline" and not e.get("closed") and zs and ms:
        pts = np.array(geom.GetPoints(), dtype=float)
        # OGR's points carry x, y and z; the measure is read apart.
        pts = np.column_stack([pts[:, :3], [geom.GetM(i) for i in range(geom.GetPointCount())]])
        verts = [(p["x"], p["y"]) for p in e["pts"]]
        at = 0
        for i in range(len(verts) - 1):
            check(np.allclose(pts[at][:2], verts[i], atol=1e-9), f"{where}: edge {i} starts elsewhere")
            check(abs(pts[at][2] - zs[i]) < 1e-12 and abs(pts[at][3] - ms[i]) < 1e-9, f"{where}: vertex {i}: {pts[at]}")
            bl = e["bulges"][i]
            n = 1
            if abs(bl) > 1e-12:
                a, b = rel(e["pts"][i]), rel(e["pts"][i + 1])
                c, r, a0, sweep = bulge_circle(a, b, bl)
                n = least_count(r, abs(sweep), TOL)
                for j in range(1, n):
                    fr = j / n
                    p = pts[at + j]
                    check(abs(p[2] - (zs[i] + (zs[i + 1] - zs[i]) * fr)) < 1e-9, f"{where}: z at {fr}: {p[2]}")
                    check(abs(p[3] - (ms[i] + (ms[i + 1] - ms[i]) * fr)) < 1e-6, f"{where}: m at {fr}: {p[3]}")
            at += n
        check(at == len(pts) - 1 and np.allclose(pts[at][:2], verts[-1], atol=1e-9), f"{where}: the last vertex")
        check(abs(pts[at][2] - zs[-1]) < 1e-12 and abs(pts[at][3] - ms[-1]) < 1e-9, f"{where}: the last vertex's z and m")
    if k == "polygon" and zs:
        ring = geom.GetGeometryRef(0).GetPoints()
        a, b = rel(e["pts"][1]), rel(e["pts"][2])
        c, r, a0, sweep = bulge_circle(a, b, e["bulges"][1])
        n = least_count(r, abs(sweep), TOL)
        check(ring[0][2] == zs[0] and ring[1][2] == zs[1], f"{where}: the first corners' z")
        for j in range(1, n):
            check(abs(ring[1 + j][2] - (zs[1] + (zs[2] - zs[1]) * j / n)) < 1e-9, f"{where}: z along the bulged edge")
        check(ring[1 + n][2] == zs[2] and ring[2 + n][2] == zs[3] and ring[-1][2] == zs[0], f"{where}: the last corners' z")
        # The hole has no elevations: a vertex without one is written as not a number.
        hole = geom.GetGeometryRef(1).GetPoints()
        check(all(p[2] != p[2] for p in hole), f"{where}: the hole's z")


def check_area(where, e, geom):
    """The written polygon's area against the shoelace of its corners with the arcs' slivers, and the analytic one."""
    def ring(pts, bulges):
        p = [rel(q) for q in pts]
        n = len(p)
        shoelace = sum(p[i][0] * p[(i + 1) % n][1] - p[(i + 1) % n][0] * p[i][1] for i in range(n)) / 2
        analytic = derived = shoelace
        length = 0.0
        for i in range(n):
            bl = bulges[i] if bulges else 0
            if abs(bl) > 1e-12:
                c, r, a0, sweep = bulge_circle(p[i], p[(i + 1) % n], bl)
                m = least_count(r, abs(sweep), TOL)
                seg = math.copysign(segment_area(r, sweep), bl)
                sliver = math.copysign(m * segment_area(r, abs(sweep) / m), bl)
                analytic += seg
                derived += seg - sliver
                length += r * abs(sweep)
        return abs(analytic), abs(derived), length

    oa, od, ol = ring(e["pts"], e.get("bulges"))
    holes = [ring(h["pts"], h.get("bulges")) for h in e.get("holes", [])]
    analytic = oa - sum(h[0] for h in holes)
    derived = od - sum(h[1] for h in holes)
    length = ol + sum(h[2] for h in holes)
    check(abs(geom.GetArea() - derived) <= 1e-6, f"{where}: written area {geom.GetArea()}, from its slivers {derived}")
    check(0 <= analytic - derived <= length * TOL, f"{where}: the chords' area is off the analytic by {analytic - derived}")
    print(f"  {where}: analytic {analytic:.6f} m², derived {derived:.6f} m², "
          f"difference {analytic - derived:.6f} m² (bound {length * TOL:.6f})")


# ── The edited copies ───────────────────────────────────────────────────────

def copy_of(name):
    dst = DIR / name
    shutil.copyfile(DIR / "ada.gpkg", dst)
    return dst


def update(path, table, fid, field=None, value=None, move=None):
    ds = ogr.Open(str(path), 1)
    layer = ds.GetLayerByName(table)
    f = layer.GetFeature(fid)
    if field:
        f.SetField(field, value)
    if move:
        g = f.GetGeometryRef().Clone()
        x, y = g.GetPoint_2D(0)
        g.SetPoint_2D(0, x + move[0], y + move[1])
        f.SetGeometry(g)
    layer.SetFeature(f)
    ds = None


def make_files():
    p = copy_of("ada-duzenli.gpkg")
    update(p, "Parsel", 2, field="ada", value="999")
    update(p, "Yol", 1, move=(0.5, 0.0))
    update(p, "kentos_annotations", 1, field="text", value="Ada 102")
    p = copy_of("ada-eksik.gpkg")
    ds = ogr.Open(str(p), 1)
    ds.GetLayerByName("Parsel").DeleteFeature(3)
    ds = None
    p = copy_of("ada-yabanci.gpkg")
    foreign = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999"
    update(p, "Yol", 4, field="kentos_uid", value=foreign)
    c = sqlite3.connect(p)
    c.execute("UPDATE kentos_uids SET uid = ? WHERE table_name = 'Yol' AND fid = 4", (foreign,))
    c.commit()
    c.close()


def check_copies():
    def rows(name, table):
        ds = ogr.Open(str(DIR / name))
        out = {}
        for f in ds.GetLayerByName(table):
            g = f.GetGeometryRef()
            out[f.GetFID()] = (f.items(), g.ExportToWkt() if g is not None else None)
        return out

    base = {t: rows("ada.gpkg", t) for t in TABLES}
    for name, changes in {
        "ada-duzenli.gpkg": {("Parsel", 2), ("Yol", 1), ("kentos_annotations", 1)},
        "ada-eksik.gpkg": {("Parsel", 3)},
        "ada-yabanci.gpkg": {("Yol", 4)},
    }.items():
        check((DIR / name).exists(), f"{name} is not there; run with --files")
        for t in TABLES:
            now = rows(name, t)
            for fid in set(base[t]) | set(now):
                same = base[t].get(fid) == now.get(fid)
                check(same != ((t, fid) in changes), f"{name}: {t} {fid} {'unchanged' if same else 'changed'}")
    check(rows("ada-duzenli.gpkg", "Parsel")[2][0]["ada"] == "999", "ada-duzenli: the attribute")
    check(rows("ada-duzenli.gpkg", "kentos_annotations")[1][0]["text"] == "Ada 102", "ada-duzenli: the text")
    now = np.array(ogr.CreateGeometryFromWkt(rows("ada-duzenli.gpkg", "Yol")[1][1]).GetPoint_2D(0))
    was = np.array(ogr.CreateGeometryFromWkt(base["Yol"][1][1]).GetPoint_2D(0))
    check(np.allclose(now - was, (0.5, 0.0), atol=1e-9), f"ada-duzenli: the geometry moved by {now - was}")
    check(read_uids(DIR / "ada-yabanci.gpkg")[("Yol", 4)] == "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d9999", "ada-yabanci: the id")
    for name in ("ada-duzenli.gpkg", "ada-yabanci.gpkg"):
        check_dataset(DIR / name, rows_too=False)


def findings(path):
    """GDAL's validator's findings of a file."""
    from osgeo_utils.samples import validate_gpkg

    try:
        return validate_gpkg.check(str(path), abort_at_first_error=False) or []
    except Exception as e:  # noqa: BLE001  (the validator raises its finding)
        return [str(e)]


def check_validator():
    """The derived file may have no finding QGIS's own GeoPackage lacks (GDAL 3.12's validator reads the empty flag from
    another bit than the specification: every empty geometry is its finding 152, as in QGIS's file)."""
    qgis = findings(ROOT / "fixtures/geopackage/v1/qgis.gpkg")
    for name in ("ada.gpkg", "ada-duzenli.gpkg", "ada-eksik.gpkg", "ada-yabanci.gpkg"):
        extra = [f for f in findings(DIR / name) if f not in qgis]
        check(not extra, f"{name}: GDAL's validator: {extra}")


def main():
    check((DIR / "ada.gpkg").exists(), "ada.gpkg is not there: KENTOS_WRITE_GPKG=1 cargo test -p kentos-headless --test gis")
    if "--files" in sys.argv:
        make_files()
    check_dataset(DIR / "ada.gpkg")
    check_copies()
    check_validator()
    print("gis_derivation_files ok")


if __name__ == "__main__":
    main()
