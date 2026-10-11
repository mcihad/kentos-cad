#!/usr/bin/env python3
"""Independent reference of the writers that move into WGS 84: GPX and RFC 7946 GeoJSON (docs/adr/0222 §5, §7).

Writes the writers' inputs, fixtures/formats/v1/wgs84/<name>.case.json; with --check, reads the file the writer wrote
for each (<name>.gpx or <name>.geojson, written by `KENTOS_WRITE_WGS84=1 cargo test -p kentos-formats wgs84`) with
Python's expat and json and holds it to its input with PROJ (pyproj), no KentOS code.

GPX (§5): a GPX 1.1 file (version 1.1, creator “KentOS CAD”, the topografix namespace), its metadata named by the
drawing; in the schema's order its waypoints, then routes, then tracks. A point (each point of a multi-point object)
is a waypoint, a text a waypoint named by its text; a line, a polyline's paths and a circle (72 points a turn from
angle 0, closed) are, in the objects' order, a route each when routes are asked for, else one track of the object
with a segment each; an area's outline (counter-clockwise) and holes (clockwise), closed, part after part, are one
track's segments whichever lines become. A waypoint, route or track is named by its object's label, else its Ad
attribute, described by its Açıklama and typed by its Kod; a point's elevation is its ele, as it is; latitude and
longitude are the vertex moved from the project's system into WGS 84 by the ADR's EPSG path
(scripts/fixtures/crs_transform_cases.py), within 1.5e-10 degrees (ten decimals).

GeoJSON (§7): a FeatureCollection named by the drawing, without a crs member; a Feature per object the writer writes,
in order, its geometry's coordinates the vertices moved likewise (within 1.5e-10 degrees), its elevations as they
are, its properties its attributes, its layer and label in the kentos member.
"""

import argparse
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from crs_transform_cases import system, transformer  # noqa: E402
from kml_cases import DEGREES, parse, registry, text, write_inputs, written_geometry  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures" / "formats" / "v1" / "wgs84"
GPX_NS = "http://www.topografix.com/GPX/1/1"


def inputs():
    """The cases: GPX as tracks and as routes from TUREF / TM33, GeoJSON from ED50 / TM33."""
    base = dict(write_inputs())["tm33"]["input"]
    reg = registry()
    out = []
    for name, srid, extra in (("gpx-iz", 5255, {"routes": False}), ("gpx-rota", 5255, {"routes": True})):
        out.append((name, {"format": "gpx", "srid": srid, "system": system(reg[srid]), "choices": [],
                           "input": {"name": base["name"], "entities": base["entities"], **extra}}))
    layers = [{"id": l["id"], "name": l["name"]} for l in base["layers"]]
    out.append(("geojson", {"format": "geojson", "srid": 2321, "system": system(reg[2321]), "choices": [],
                            "input": {"name": base["name"], "srid": 2321, "layers": layers, "entities": base["entities"]}}))
    return out


def circle(data):
    (cx, cy), r = data
    ring = [(cx + math.cos(k / 72 * 2 * math.pi) * r, cy + math.sin(k / 72 * 2 * math.pi) * r, None) for k in range(72)]
    return ring + [ring[0]]


def pieces(geom):
    """A written geometry's points, paths and area rings, in order."""
    kind, data = geom
    if kind == "Point":
        return [data], [], []
    if kind == "LineString":
        return [], [data], []
    if kind == "Circle":
        return [], [circle(data)], []
    if kind == "Polygon":
        return [], [], list(data)
    points, paths, rings = [], [], []
    for m in data:
        a, b, c = pieces(m)
        points += a
        paths += b
        rings += c
    return points, paths, rings


def said(e):
    name = e["text"] if e["kind"] == "text" else (e.get("label") or e["attrs"].get("Ad") or None)
    return {"name": name, "desc": e["attrs"].get("Açıklama") or None, "type": e["attrs"].get("Kod") or None}


def close(a, b):
    return abs(a - b) <= DEGREES


def check_point(node, v, t, where, problems):
    lon, lat = t.transform(v[0], v[1])
    try:
        glat, glon = float(node.attrs["lat"]), float(node.attrs["lon"])
    except (KeyError, ValueError):
        problems.append(f"{where}: lat ya da lon yok")
        return
    if not (close(glat, lat) and close(glon, lon)):
        problems.append(f"{where}: ({glat}, {glon}), PROJ ({lat}, {lon})")
    ele = text(node.child("ele"))
    if (ele is None) != (v[2] is None) or (ele is not None and float(ele) != v[2]):
        problems.append(f"{where}: ele {ele}, beklenen {v[2]}")


def check_said(node, s, where, problems):
    for tag in ("name", "desc", "type"):
        if text(node.child(tag)) != s[tag]:
            problems.append(f"{where}: {tag} {text(node.child(tag))!r}, beklenen {s[tag]!r}")


def check_gpx(name, case, data):
    problems = []
    reg = registry()
    t, _, _ = transformer(reg[case["srid"]], reg[4326])
    spec = case["input"]
    if not data.decode("utf-8").startswith('<?xml version="1.0" encoding="UTF-8"?>\n<gpx version="1.1" creator="KentOS CAD" xmlns="' + GPX_NS + '">'):
        return [f"{name}: başlık ya da kök GPX 1.1'inki değil"]
    root = parse(data)
    if text(root.child("metadata").child("name")) != spec["name"]:
        problems.append(f"{name}: metadata'nın adı beklenen değil")
    order = [c.tag for c in root.children if c.tag != "metadata"]
    if order != sorted(order, key=["wpt", "rte", "trk"].index):
        problems.append(f"{name}: öğeler wpt, rte, trk sırasında değil")
    waypoints, routes, tracks = [], [], []
    for e in spec["entities"]:
        geom = written_geometry(e)
        if geom is None:
            continue
        points, paths, rings = pieces(geom)
        s = said(e)
        waypoints += [(e, s, p) for p in points]
        if spec.get("routes"):
            routes += [(e, s, p) for p in paths]
            if rings:
                tracks.append((e, s, rings))
        elif paths or rings:
            tracks.append((e, s, paths + rings))
    got_w = [c for c in root.children if c.tag == "wpt"]
    got_r = [c for c in root.children if c.tag == "rte"]
    got_t = [c for c in root.children if c.tag == "trk"]
    if (len(got_w), len(got_r), len(got_t)) != (len(waypoints), len(routes), len(tracks)):
        return problems + [f"{name}: {len(got_w)} wpt, {len(got_r)} rte, {len(got_t)} trk; beklenen {len(waypoints)}, {len(routes)}, {len(tracks)}"]
    for i, (node, (e, s, p)) in enumerate(zip(got_w, waypoints)):
        where = f"{name} wpt {i} ({e['id']})"
        check_point(node, p, t, where, problems)
        check_said(node, s, where, problems)
    for i, (node, (e, s, path)) in enumerate(zip(got_r, routes)):
        where = f"{name} rte {i} ({e['id']})"
        check_said(node, s, where, problems)
        pts = [c for c in node.children if c.tag == "rtept"]
        if len(pts) != len(path):
            problems.append(f"{where}: {len(pts)} nokta, beklenen {len(path)}")
            continue
        for k, (n, v) in enumerate(zip(pts, path)):
            check_point(n, v, t, f"{where} {k}", problems)
    for i, (node, (e, s, segments)) in enumerate(zip(got_t, tracks)):
        where = f"{name} trk {i} ({e['id']})"
        check_said(node, s, where, problems)
        segs = [c for c in node.children if c.tag == "trkseg"]
        if len(segs) != len(segments):
            problems.append(f"{where}: {len(segs)} parça, beklenen {len(segments)}")
            continue
        for j, (seg, path) in enumerate(zip(segs, segments)):
            pts = [c for c in seg.children if c.tag == "trkpt"]
            if len(pts) != len(path):
                problems.append(f"{where} parça {j}: {len(pts)} nokta, beklenen {len(path)}")
                continue
            for k, (n, v) in enumerate(zip(pts, path)):
                check_point(n, v, t, f"{where} parça {j} {k}", problems)
    return problems


def check_positions(got, want, t, where, problems):
    if len(got) != len(want):
        problems.append(f"{where}: {len(got)} konum, beklenen {len(want)}")
        return
    for k, (g, v) in enumerate(zip(got, want)):
        lon, lat = t.transform(v[0], v[1])
        if not (close(g[0], lon) and close(g[1], lat)):
            problems.append(f"{where} {k}: {g[:2]}, PROJ {[lon, lat]}")
        if (len(g) == 3) != (v[2] is not None) or (len(g) == 3 and g[2] != v[2]):
            problems.append(f"{where} {k}: kot {g[2:]}, beklenen {v[2]}")


def check_geojson_geometry(g, geom, t, where, problems):
    kind, data = geom
    types = {"Point": "Point", "LineString": "LineString", "Circle": "LineString", "Polygon": "Polygon"}
    if kind == "Multi":
        member = data[0][0]
        want = {"Point": "MultiPoint", "LineString": "MultiLineString", "Polygon": "MultiPolygon"}[member]
        if g["type"] != want or len(g["coordinates"]) != len(data):
            problems.append(f"{where}: {g['type']}, beklenen {want} ({len(data)} üye)")
            return
        for k, (c, m) in enumerate(zip(g["coordinates"], data)):
            check_geojson_geometry({"type": types[m[0]], "coordinates": c}, m, t, f"{where} {k}. üye", problems)
        return
    if g["type"] != types[kind]:
        problems.append(f"{where}: {g['type']}, beklenen {types[kind]}")
        return
    if kind == "Point":
        check_positions([g["coordinates"]], [data], t, where, problems)
    elif kind == "LineString":
        check_positions(g["coordinates"], data, t, where, problems)
    elif kind == "Circle":
        check_positions(g["coordinates"], circle(data), t, where, problems)
    else:
        if len(g["coordinates"]) != len(data):
            problems.append(f"{where}: {len(g['coordinates'])} halka, beklenen {len(data)}")
            return
        for k, (c, ring) in enumerate(zip(g["coordinates"], data)):
            check_positions(c, ring, t, f"{where} {k}. halka", problems)


def check_geojson(name, case, data):
    problems = []
    reg = registry()
    t, _, _ = transformer(reg[case["srid"]], reg[4326])
    spec = case["input"]
    doc = json.loads(data.decode("utf-8"))
    if doc.get("type") != "FeatureCollection" or doc.get("name") != spec["name"] or "crs" in doc:
        return [f"{name}: adı girdininki olan, crs üyesiz bir FeatureCollection değil"]
    layers = {l["id"]: l["name"] for l in spec["layers"]}
    written = [(e, written_geometry(e)) for e in spec["entities"] if e["kind"] != "text" and written_geometry(e) is not None]
    if len(doc["features"]) != len(written):
        return [f"{name}: {len(doc['features'])} nesne, beklenen {len(written)}"]
    for f, (e, geom) in zip(doc["features"], written):
        where = f"{name} {e['id']}"
        if f["properties"] != e["attrs"]:
            problems.append(f"{where}: özellikler {f['properties']}, beklenen {e['attrs']}")
        kentos = {k: v for k, v in (("layer", layers.get(e["layerId"])), ("label", e.get("label"))) if v is not None}
        if f.get("kentos", {}) != kentos:
            problems.append(f"{where}: kentos üyesi {f.get('kentos')}, beklenen {kentos}")
        check_geojson_geometry(f["geometry"], geom, t, where, problems)
    return problems


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--check", action="store_true", help="compare with the files instead of writing them")
    args = parser.parse_args()
    files = {DIR / f"{name}.case.json": (json.dumps(case, ensure_ascii=False, indent=2) + "\n").encode("utf-8") for name, case in inputs()}
    if args.check:
        problems = [f"{p.relative_to(ROOT)} kurallardan çıkanla aynı değil" for p, data in files.items() if not p.exists() or p.read_bytes() != data]
        for name, case in inputs():
            out = DIR / f"{name}.{case['format']}"
            if not out.exists():
                problems.append(f"{out.relative_to(ROOT)} yok (KENTOS_WRITE_WGS84=1 cargo test -p kentos-formats wgs84)")
                continue
            check = check_gpx if case["format"] == "gpx" else check_geojson
            problems += check(name, case, out.read_bytes())
        if problems:
            print("\n".join(problems))
            print("Betiği --check olmadan çalıştırıp farkı okuyun.")
            sys.exit(1)
        print(f"{DIR.relative_to(ROOT)}: yazıcıların {len(inputs())} dosyası kurallarla aynı.")
        return
    DIR.mkdir(parents=True, exist_ok=True)
    for path, data in files.items():
        path.write_bytes(data)
    print(f"{DIR.relative_to(ROOT)} yazıldı: {len(inputs())} girdi.")


if __name__ == "__main__":
    main()
