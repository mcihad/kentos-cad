#!/usr/bin/env python3
"""The shared cases of cad.layers.crs and cad.layers.reproject (docs/adr/0583 §2, plan/gis.md CBS-07).

    python3 scripts/fixtures/layer_crs_cases.py           # writes fixtures/commands/v1/cad.layers.{crs,reproject}.json
    python3 scripts/fixtures/layer_crs_cases.py --check   # compares

No KentOS code. A layer's record is written here from the registry (fixtures/crs/v1/registry.json): its name, EPSG's
axis order, unit and area; its WKT2 from EPSG's definition (datum, ellipsoid, method and parameters with their EPSG
codes) and checked against PROJ (pyproj): the text read by PROJ moves points across Türkiye exactly as the EPSG code
does, and has the code's axis order. The moved coordinates are PROJ's, along the EPSG path ADR 0167 names for each
pair of datums (crs_transform_cases.py's `transformer`); the cases compare them within 1 mm.

The preview of Koordinat sistemi ▸ Dönüştür (docs/adr/2500) is written here too: each object's first point now and moved (PROJ's), its
points' shifts, the shifts summed up; the notes from EPSG's areas of use as PROJ's database gives them (the source
system's and the project's, `CRS.area_of_use`; the datum transformation's, `CoordinateOperation.area_of_use` of the
path's operations), a point read at its place on the source's datum (PROJ's inverse) and where it lands; the chain's
verdict by docs/adr/1230 §3's rules (crs_chain_rules.py's order and texts, the ensembles from PROJ's database).
"""

import copy
import json
import math
import sys
from pathlib import Path

from pyproj import CRS, Transformer
from pyproj.crs import CoordinateOperation

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_area_test as AT  # noqa: E402
import crs_chain_rules as rules  # noqa: E402
import crs_transform_cases as ct  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
REGISTRY = {s["srid"]: s for s in json.loads((ROOT / "fixtures/crs/v1/registry.json").read_text("utf-8"))["systems"]}
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
PROJECT = 5256
WITHIN = 0.001

DEGREE = 'ANGLEUNIT["degree",0.0174532925199433]'
METRE = 'LENGTHUNIT["metre",1]'
DATUMS = {
    "TUREF": ("TUREF", "Turkish National Reference Frame", "GRS 1980", 6378137, 298.257222101),
    "ED50": ("ED50", "European Datum 1950", "International 1924", 6378388, 297),
    "WGS84": ("WGS 84", "World Geodetic System 1984", "WGS 84", 6378137, 298.257223563),
}


def num(v):
    return str(int(v)) if float(v) == int(v) else repr(float(v))


def wkt2(s):
    """EPSG's definition of a registry system as WKT2 (ISO 19162:2019)."""
    base, datum, ellipsoid, a, rf = DATUMS[s["datum"]]
    d = f'DATUM["{datum}",ELLIPSOID["{ellipsoid}",{num(a)},{num(rf)},{METRE}]],PRIMEM["Greenwich",0,{DEGREE}]'
    code = f'ID["EPSG",{s["srid"]}]'
    if s["kind"] == "geographic":
        return (f'GEOGCRS["{s["name"]}",{d},CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],{DEGREE}],'
                f'AXIS["geodetic longitude (Lon)",east,ORDER[2],{DEGREE}],{code}]')
    if s["projection"] == "Pseudo-Mercator":
        method = 'METHOD["Popular Visualisation Pseudo Mercator",ID["EPSG",1024]]'
        params = (f'PARAMETER["Latitude of natural origin",0,{DEGREE},ID["EPSG",8801]],'
                  f'PARAMETER["Longitude of natural origin",0,{DEGREE},ID["EPSG",8802]],'
                  f'PARAMETER["False easting",0,{METRE},ID["EPSG",8806]],PARAMETER["False northing",0,{METRE},ID["EPSG",8807]]')
    else:
        method = 'METHOD["Transverse Mercator",ID["EPSG",9807]]'
        params = (f'PARAMETER["Latitude of natural origin",0,{DEGREE},ID["EPSG",8801]],'
                  f'PARAMETER["Longitude of natural origin",{num(s["centralMeridian"])},{DEGREE},ID["EPSG",8802]],'
                  f'PARAMETER["Scale factor at natural origin",{num(s["scaleFactor"])},SCALEUNIT["unity",1],ID["EPSG",8805]],'
                  f'PARAMETER["False easting",{num(s["falseEasting"])},{METRE},ID["EPSG",8806]],'
                  f'PARAMETER["False northing",{num(s["falseNorthing"])},{METRE},ID["EPSG",8807]]')
    if s["axisOrder"] == "ne":
        axes = f'AXIS["northing (X)",north,ORDER[1],{METRE}],AXIS["easting (Y)",east,ORDER[2],{METRE}]'
    else:
        axes = f'AXIS["easting (X)",east,ORDER[1],{METRE}],AXIS["northing (Y)",north,ORDER[2],{METRE}]'
    conversion = s["name"].rsplit(" / ", 1)[-1]
    return (f'PROJCRS["{s["name"]}",BASEGEOGCRS["{base}",{d}],CONVERSION["{conversion}",{method},{params}],'
            f'CS[Cartesian,2],{axes},{code}]')


def record(srid):
    s = REGISTRY[srid]
    out = {"srid": srid, "name": s["name"], "wkt": wkt2(s), "axisOrder": s.get("axisOrder") or "en", "unit": s["unit"]}
    if s.get("area"):
        out["area"] = s["area"]
    return out


def check_wkt():
    """Every registry system's WKT2, read by PROJ, is its EPSG code: the same axis order, the same points."""
    problems = []
    for srid, s in REGISTRY.items():
        if s["kind"] == "local":
            continue
        text = wkt2(s)
        mine, theirs = CRS.from_wkt(text), CRS.from_epsg(srid)
        axes = [a.direction for a in mine.axis_info]
        if axes != [a.direction for a in theirs.axis_info]:
            problems.append(f"EPSG:{srid}: eksenler {axes}")
        a = Transformer.from_crs("EPSG:4326", mine, always_xy=True)
        b = Transformer.from_crs("EPSG:4326", theirs, always_xy=True)
        for _, lat, lon in ct.PLACES:
            if s.get("projection") in ("Transverse Mercator", "UTM") and abs(lon - s["centralMeridian"]) > 6:
                continue
            p, q = a.transform(lon, lat), b.transform(lon, lat)
            if max(abs(p[0] - q[0]), abs(p[1] - q[1])) > 1e-6:
                problems.append(f"EPSG:{srid}: {lat},{lon} → {p} ≠ {q}")
    if problems:
        raise SystemExit("; ".join(problems))


def moved(src, x, y):
    """(x, y) of the registry system `src` in the project's (TUREF / TM36), along ADR 0167's EPSG path."""
    t, _, _ = ct.transformer(REGISTRY[src], REGISTRY[PROJECT])
    p = t.transform(x, y)
    return {"x": p[0], "y": p[1]}


def layer(id_, name, **extra):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": [], **extra}


# Places: a point of the project's own; an old sheet in ED50 / TM36 (its numbers as ED50's); GPS fixes in
# longitude, latitude; a point whose numbers are no longitude.
ED50 = [(512345.678, 4420123.456), (512400.0, 4420150.0), (512380.0, 4420200.0)]
GPS = [(32.8597, 39.9334), (32.8640, 39.9370)]
# One point of Ankara in other systems' numbers (as their own): ED50 and WGS 84 UTM 36N, TUREF's latitude and
# longitude, Web Mercator.
OTHERS = {"utm": (23036, (488000.0, 4420000.0)), "wgsutm": (32636, (488000.0, 4420000.0)),
          "geo": (5252, (32.86, 39.93)), "merc": (3857, (3657946.0, 4855000.0)), "tm33": (2321, (757000.0, 4425000.0))}
RING = [(512000.0, 4421000.0), (512100.0, 4421000.0), (512100.0, 4421100.0), (512000.0, 4421100.0)]
HOLE = [(512030.0, 4421030.0), (512060.0, 4421030.0), (512060.0, 4421060.0), (512030.0, 4421060.0)]
# ED50 / TM36 numbers: a point of the old sheet's and one at 44.3° N, north of every Türkiye box (docs/adr/2500 §3).
NEAR, FAR = (512200.0, 4420200.0), (512000.0, 4900000.0)


def setup():
    ents = [
        {"kind": "point", "id": 1, "layerId": "tum", "attrs": {}, "p": {"x": 512000.0, "y": 4420000.0}},
        {"kind": "point", "id": 2, "layerId": "eski", "attrs": {"ad": "S1"}, "p": {"x": ED50[0][0], "y": ED50[0][1]},
         "z": 875.25},
        {"kind": "polyline", "id": 3, "layerId": "eski", "attrs": {},
         "pts": [{"x": x, "y": y} for x, y in ED50]},
        {"kind": "point", "id": 4, "layerId": "gps", "attrs": {"ad": "G1"}, "p": {"x": GPS[0][0], "y": GPS[0][1]}},
        {"kind": "point", "id": 5, "layerId": "gps", "attrs": {"ad": "G2"}, "p": {"x": GPS[1][0], "y": GPS[1][1]}},
        {"kind": "point", "id": 6, "layerId": "bozuk", "attrs": {}, "p": {"x": 512000.0, "y": 4420000.0}},
        {"kind": "point", "id": 7, "layerId": "kilitli", "attrs": {}, "p": {"x": 32.86, "y": 39.93}},
        {"kind": "polygon", "id": 8, "layerId": "parsel", "attrs": {"ada": "101"},
         "pts": [{"x": x, "y": y} for x, y in RING], "holes": [{"pts": [{"x": x, "y": y} for x, y in HOLE]}]},
        {"kind": "polyline", "id": 9, "layerId": "kotlu", "attrs": {},
         "pts": [{"x": x, "y": y} for x, y in ED50[:2]], "zs": [870.5, 871.25]},
    ] + [{"kind": "point", "id": 10 + i, "layerId": lid, "attrs": {}, "p": {"x": xy[0], "y": xy[1]}}
         for i, (lid, (_, xy)) in enumerate(OTHERS.items())] + [
        # An old sheet's point and one far north of Türkiye in its numbers (docs/adr/2500 §3).
        {"kind": "point", "id": 15, "layerId": "uzak", "attrs": {}, "p": {"x": NEAR[0], "y": NEAR[1]}},
        {"kind": "point", "id": 16, "layerId": "uzak", "attrs": {"ad": "U1"}, "p": {"x": FAR[0], "y": FAR[1]}},
        # A point, a text and a circle: only points and straight paths move (docs/adr/2500 §5).
        {"kind": "point", "id": 17, "layerId": "yazilar", "attrs": {}, "p": {"x": 512000.0, "y": 4420000.0}},
        {"kind": "text", "id": 18, "layerId": "yazilar", "attrs": {}, "p": {"x": 512010.0, "y": 4420010.0},
         "text": "S2", "height": 2.5, "rotation": 0},
        {"kind": "circle", "id": 19, "layerId": "yazilar", "attrs": {}, "c": {"x": 512020.0, "y": 4420020.0}, "r": 5},
    ]
    layers = [layer("0", "0"), layer("tum", "Tüm"), layer("eski", "Eski pafta"), layer("gps", "GPS"),
              layer("bozuk", "Bozuk"), layer("kilitli", "Kilitli"), layer("parsel", "Parsel"), layer("kotlu", "Kotlu"),
              layer("bos", "Boş"), *[layer(lid, lid.upper()) for lid in OTHERS], layer("uzak", "Uzak"),
              layer("yazilar", "Yazılar"),
              {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
               "style": STYLE, "children": []}]
    next(n for n in layers if n["id"] == "kilitli")["locked"] = True
    return {"format": "kentos.document", "version": 1, "name": "Kaynak sistemleri",
            "settings": {"srid": PROJECT, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 500000, "y": 4400000}, "layers": layers, "activeLayer": "0", "entities": ents,
            "styles": {"items": [], "categories": []}}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path, uids=None):
    e = {"code": code, "message": message, "path": path}
    if uids:
        e["uids"] = uids
    return {"status": "failed", "error": e}


SAME = {"revision": "same"}
CHANGED = {"revision": "changed"}


def step(inp, result, expect=None, command=None, within=None):
    s = {"op": "execute", "input": inp, "result": result, "expect": expect or SAME}
    if command:
        s["command"] = command
    if within:
        s["within"] = within
    return s


CONFLICT = ("Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
            "yeniden hazırlayın.")


def stale(inp):
    """A command prepared before the drawing changed: another layer's record is written in between."""
    return [{"op": "captureRevision", "as": "once"},
            step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": record(4326), "changed": True,
                                                        "revision": "$current"}), CHANGED, command="cad.layers.crs"),
            {"op": "execute", "input": {**inp, "expectedRevision": "$once"},
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT,
                                                       "path": "expectedRevision", "revision": "$current"}},
             "expect": SAME}]


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def crs_cases():
    gps = record(4326)
    out = [
        case("GPS katmanının kaynağı WGS 84: kayıt adı, WKT2'si, eksen sırası ve kullanım alanıyla",
             step({"layer": "gps", "srid": 4326},
                  done({"layer": "gps", "crs": gps, "changed": True, "revision": "$current"}), CHANGED)),
        case("aynı kayıt ikinci kez yazılmaz",
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": gps, "changed": True,
                                                         "revision": "$current"}), CHANGED),
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": gps, "changed": False,
                                                         "revision": "$current"}))),
        case("kayıt silinir",
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": gps, "changed": True,
                                                         "revision": "$current"}), CHANGED),
             step({"layer": "gps"}, done({"layer": "gps", "changed": True, "revision": "$current"}), CHANGED)),
        case("kaydı olmayanın silinmesi bir şey yazmaz",
             step({"layer": "tum"}, done({"layer": "tum", "changed": False, "revision": "$current"}))),
        case("kilitli katmana da atanır: veri değişmez",
             step({"layer": "kilitli", "srid": 4326}, done({"layer": "kilitli", "crs": gps, "changed": True,
                                                             "revision": "$current"}), CHANGED)),
    ]
    for srid in (5256, 2322, 23036, 32636, 3857, 5252, 2319, 5259, 23038, 32635, 2325):
        out.append(case(f"{REGISTRY[srid]['name']} kaydı (EPSG:{srid})",
                        step({"layer": "tum", "srid": srid},
                             done({"layer": "tum", "crs": record(srid), "changed": True, "revision": "$current"}),
                             CHANGED)))
    out += [
        case("ret: kayıtta olmayan kod",
             step({"layer": "gps", "srid": 9999},
                  failed("unknown_srid", "EPSG:9999 koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin.",
                         "srid"))),
        case("ret: yerel sistem kaynak sistemi değildir",
             step({"layer": "gps", "srid": 0},
                  failed("unknown_srid", "Yerel sistem bir kaynak sistemi değildir; kaydı silmek için srid vermeyin.",
                         "srid"))),
        case("ret: grup",
             step({"layer": "grup", "srid": 4326},
                  failed("not_a_layer",
                         "“Grup” bir katman grubu; kaynak sistemi yalnız katmanın olur. Grubun bir katmanını verin.",
                         "layer"))),
        case("sürüm çakışması: hazırlandıktan sonra çizim değişmişse hiçbir şey yazılmaz",
             *stale({"layer": "tum", "srid": 4326})),
        case("ret: sürüm metni değil",
             step({"layer": "gps", "srid": 4326, "expectedRevision": "x"}, failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))),
        case("ret: olmayan katman",
             step({"layer": "yok", "srid": 4326},
                  failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                         "layer"))),
    ]
    return out


def ent(id_, kind, layer_id, attrs, **geom):
    return {"kind": kind, "id": id_, "layerId": layer_id, "attrs": attrs, **geom}



# ── Koordinat sistemi ▸ Dönüştür's preview (docs/adr/2500) ────────────────────────────────────────────────────────────────

NOTES = ["outside_source_area", "outside_operation_area", "outside_project_area"]
NOTE_TEXT = {
    "outside_source_area": "kaynak sistemin kullanım alanının dışında (nesneler başka bir sistemde olabilir)",
    "outside_operation_area": "datum dönüşümünün kullanım alanının dışında (dönüşümün doğruluğu orada geçerli değil)",
    "outside_project_area": "projenin sisteminin kullanım alanının dışında",
}
LISTED = 1000


def area(srid):
    """EPSG's area of use of a registry system as PROJ's database gives it: (west, south, east, north)."""
    return tuple(CRS.from_epsg(srid).area_of_use.bounds)


def operation_area(src):
    """The box of the datum transformation's EPSG operations from `src` into the project's system (all of the path's
    operations', which agree); none when the datum stays."""
    a, b = ct.datum(REGISTRY[src]), ct.datum(REGISTRY[PROJECT])
    if a == b:
        return None
    codes = [int(c.strip().split(":")[1]) for c in ct.PATHS[frozenset([a, b])][2].split("+")]
    boxes = {tuple(CoordinateOperation.from_epsg(c).area_of_use.bounds) for c in codes}
    assert len(boxes) == 1, (src, boxes)
    return boxes.pop()


def lonlat(srid, x, y):
    """The longitude and latitude of a point of `srid` on its own datum, through PROJ."""
    crs = CRS.from_epsg(srid)
    return Transformer.from_crs(crs, crs.geodetic_crs, always_xy=True).transform(x, y)


# docs/adr/3700: a point within 1e-9° of an edge is on it (the noise of two engines' inverse projections on an edge).
EDGE = 1e-9


def holds(box, lon, lat):
    """docs/adr/4900: a registry system's area is its polygon where carried (crs_area_test.py), else its box."""
    if box[0] == "rings" or box[0] == "box":
        return AT.holds(box, lon, lat)
    west, south, east, north = box[0] - EDGE, box[1] - EDGE, box[2] + EDGE, box[3] + EDGE
    along = west <= lon <= east if west <= east else (lon >= west or lon <= east)
    return along and south <= lat <= north


def vertices(e):
    """An object's points in the order KentOS moves them (its ring, then its holes); none for another kind."""
    if e["kind"] == "point":
        return [(e["p"]["x"], e["p"]["y"])]
    if e["kind"] in ("polyline", "polygon"):
        pts = [(p["x"], p["y"]) for p in e["pts"]]
        for h in e.get("holes", []):
            pts += [(p["x"], p["y"]) for p in h["pts"]]
        return pts
    return None


def verdict(src):
    """The chain's verdict for the way from `src` into the project's system (docs/adr/1230 §3 in crs_chain_rules'
    order): the horizontal way alone (heights stay), the project states no epoch, every registry way states its
    accuracy (ct.PATHS; 0 within a datum)."""
    a, b = ct.datum(REGISTRY[src]), ct.datum(REGISTRY[PROJECT])
    ensemble = any(rules._proj_side(s, REGISTRY[s])[3] is not None for s in (src, PROJECT))
    found = {"datum_ensemble": a != b and ensemble, "unofficial_operation": a != b and "ED50" in (a, b)}
    reasons = [r for r in rules.REASONS if found.get(r)]
    return ("limited" if reasons else "complete"), reasons


def reprojected(layer_id, src):
    """`cad.layers.reproject`'s answer for a layer of the setup moved from `src`, and its warnings."""
    ents = [e for e in setup()["entities"] if e["layerId"] == layer_id]
    boxes = (AT.area_of(src, area(src)), operation_area(src), AT.area_of(PROJECT, area(PROJECT)))
    noted, plain = [], []
    total, sx, sy, least, most = 0, 0.0, 0.0, None, None
    for e in ents:
        before = vertices(e)
        after = [moved(src, x, y) for x, y in before]
        out, ds = [False, False, False], []
        for (x, y), q in zip(before, after):
            dx, dy = q["x"] - x, q["y"] - y
            d = math.hypot(dx, dy)
            sx, sy = sx + dx, sy + dy
            ds.append(d)
            here = lonlat(src, x, y)
            out[0] |= not holds(boxes[0], *here)
            out[1] |= boxes[1] is not None and not holds(boxes[1], *here)
            out[2] |= not holds(boxes[2], *lonlat(PROJECT, q["x"], q["y"]))
        total += len(before)
        least = min([d for d in ds] + ([least] if least is not None else []))
        most = max([d for d in ds] + ([most] if most is not None else []))
        p, q = before[0], after[0]
        row = {"uid": f"$uidOf:{e['id']}", "kind": e["kind"], "points": len(before), "before": {"x": p[0], "y": p[1]},
               "after": q, "dx": q["x"] - p[0], "dy": q["y"] - p[1], "shift": math.hypot(q["x"] - p[0], q["y"] - p[1]),
               "least": min(ds), "most": max(ds)}
        notes = [n for n, o in zip(NOTES, out) if o]
        if notes:
            row["notes"] = notes
            noted.append(row)
        else:
            plain.append(row)
    out = {"layer": layer_id, "from": src, "to": PROJECT, "moved": len(ents), "via": "$any" if ents else "",
           "crs": record(src), "revision": "$current", "points": total}
    warnings = []
    if total:
        out.update({"meanDx": sx / total, "meanDy": sy / total, "least": least, "most": most})
    out["flagged"] = len(noted)
    if ents:
        status, reasons = verdict(src)
        out["status"] = status
        out["reasons"] = reasons
        if reasons:
            warnings.append({"code": "chain_limited",
                             "message": "Dönüşüm zinciri sınırlı, hassas dönüşüm sayılmaz: "
                                        + "; ".join(rules.TEXT[r] for r in reasons) + ".",
                             "path": "from"})
    else:
        out["reasons"] = []
    out["shifts"] = (noted + plain)[:LISTED]
    if noted:
        counts = [(n, sum(1 for r in noted if n in r["notes"])) for n in NOTES]
        warnings.append({"code": "outside_area",
                         "message": f"{len(noted)} nesnenin noktaları EPSG'nin kullanım alanlarından birinin dışında: "
                                    + "; ".join(f"{k} nesne {NOTE_TEXT[n]}" for n, k in counts if k)
                                    + ". Önizlemedeki kaymaları denetleyin.",
                         "path": "from"})
    return out, warnings


def answered(layer_id, src):
    """A completed step's result for the layer moved from `src`."""
    out, warnings = reprojected(layer_id, src)
    return done(out, warnings)


def reproject_cases():
    p0 = moved(2322, *ED50[0])
    line = [moved(2322, x, y) for x, y in ED50]
    g = [moved(4326, x, y) for x, y in GPS]
    old_entities = {"2": ent(2, "point", "eski", {"ad": "S1"}, p={"x": ED50[0][0], "y": ED50[0][1]}, z=875.25),
                    "3": ent(3, "polyline", "eski", {}, pts=[{"x": x, "y": y} for x, y in ED50])}
    new_entities = {"2": ent(2, "point", "eski", {"ad": "S1"}, p=p0, z=875.25),
                    "3": ent(3, "polyline", "eski", {}, pts=line)}
    out = [
        case("ED50 / TM36'da sanılan eski pafta projenin sistemine: noktalar EPSG yoluyla, kot kalır, kayıt ED50",
             step({"layer": "eski", "from": 2322}, answered("eski", 2322),
                  {"revision": "changed", "entities": new_entities}, within=WITHIN)),
        case("geri alma eski koordinatları ve kaydın yokluğunu geri getirir",
             step({"layer": "eski", "from": 2322}, answered("eski", 2322), CHANGED, within=WITHIN),
             {"op": "undo", "returns": "Katmanın koordinatlarını dönüştür", "expect": {"entities": old_entities}}),
        case("plan hiçbir şey yazmaz",
             {"op": "plan", "input": {"layer": "eski", "from": 2322},
              "result": answered("eski", 2322), "expect": SAME, "within": WITHIN}),
        case("kaydı olan katman from'suz: önce Tanımla, sonra Dönüştür",
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": record(4326), "changed": True,
                                                         "revision": "$current"}), CHANGED, command="cad.layers.crs"),
             step({"layer": "gps"}, answered("gps", 4326),
                  {"revision": "changed", "entities": {
                      "4": ent(4, "point", "gps", {"ad": "G1"}, p=g[0]),
                      "5": ent(5, "point", "gps", {"ad": "G2"}, p=g[1])}}, within=WITHIN)),
        *[case(f"{REGISTRY[srid]['name']} numaraları projenin sistemine (EPSG:{srid} → {PROJECT})",
               step({"layer": lid, "from": srid}, answered(lid, srid),
                    {"revision": "changed", "entities": {str(10 + i): ent(10 + i, "point", lid, {}, p=moved(srid, *xy))}},
                    within=WITHIN))
          for i, (lid, (srid, xy)) in enumerate(OTHERS.items())],
        case("delikli alan: dış halka ve delik birlikte",
             step({"layer": "parsel", "from": 2322}, answered("parsel", 2322),
                  {"revision": "changed", "entities": {"8": ent(8, "polygon", "parsel", {"ada": "101"},
                                                              pts=[moved(2322, x, y) for x, y in RING],
                                                              holes=[{"pts": [moved(2322, x, y) for x, y in HOLE]}])}},
                  within=WITHIN)),
        case("kotlu çizgi: köşelerin kotları kalır",
             step({"layer": "kotlu", "from": 2322}, answered("kotlu", 2322),
                  {"revision": "changed", "entities": {"9": ent(9, "polyline", "kotlu", {},
                                                              pts=[moved(2322, x, y) for x, y in ED50[:2]],
                                                              zs=[870.5, 871.25])}},
                  within=WITHIN)),
        case("boş katman: hiçbir nesne çevrilmez, kayıt yazılır",
             step({"layer": "bos", "from": 2322}, answered("bos", 2322), CHANGED)),
        case("önizleme: kullanım alanlarının dışındaki nokta notlanır, uyarı verir ve önce listelenir; öbürü çizim "
             "sırasıyla ardından",
             {"op": "plan", "input": {"layer": "uzak", "from": 2322}, "result": answered("uzak", 2322),
              "expect": SAME, "within": WITHIN},
             step({"layer": "uzak", "from": 2322}, answered("uzak", 2322),
                  {"revision": "changed", "entities": {
                      "15": ent(15, "point", "uzak", {}, p=moved(2322, *NEAR)),
                      "16": ent(16, "point", "uzak", {"ad": "U1"}, p=moved(2322, *FAR))}}, within=WITHIN)),
        case("sürüm çakışması: hazırlandıktan sonra çizim değişmişse hiçbir şey yazılmaz",
             *stale({"layer": "eski", "from": 2322})),
        case("ret: sürüm metni değil",
             step({"layer": "eski", "from": 2322, "expectedRevision": "x"}, failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))),
        case("ret: olmayan katman",
             step({"layer": "yok", "from": 2322},
                  failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                         "layer"))),
        case("ret: kaydı olmayan katman from'suz",
             step({"layer": "eski"},
                  failed("invalid_input",
                         "“Eski pafta” katmanının kaynak sistemi kaydı yok; nesnelerin şimdiki sistemini from ile verin.",
                         "from"))),
        case("ret: nesneler zaten projenin sisteminde",
             step({"layer": "tum", "from": PROJECT},
                  failed("crs_mismatch",
                         f"Nesneler zaten projenin sisteminde (EPSG:{PROJECT}); çevrilecek bir şey yok. Yalnız kayıt için cad.layers.crs.",
                         "from"))),
        case("ret: kayıtta olmayan kod",
             step({"layer": "eski", "from": 9999},
                  failed("unknown_srid", "EPSG:9999 koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin.",
                         "from"))),
        case("ret: kilitli katman",
             step({"layer": "kilitli", "from": 4326},
                  failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın.", "layer"))),
        case("ret: grup",
             step({"layer": "grup", "from": 4326},
                  failed("not_a_layer",
                         "“Grup” bir katman grubu; kaynak sistemi yalnız katmanın olur. Grubun bir katmanını verin.",
                         "layer"))),
        case("ret: yazı ve daire tam çevrilemez, nedeni türleriyle söylenir; nokta da yazılmaz",
             step({"layer": "yazilar", "from": 2322},
                  failed("crs_unreached",
                         "“Yazılar” katmanının 2 nesnesi EPSG:2322'den projenin sistemine çevrilemiyor (yay ya da eğrili "
                         "çizgi, daire, yazı, blok gibi tam çevrilemeyen türden); hiçbir şey yazılmadı. Yalnız noktalar "
                         "ve düz kenarlı çizgiler çevrilir; bu nesneleri başka bir katmana alın.",
                         "from", ["$uidOf:18", "$uidOf:19"]))),
        case("ret: nedenler karışıkken her biri sayısıyla: biri sistemin dışında, ikisi türünden",
             step({"layer": "yazilar", "from": 4326},
                  failed("crs_unreached",
                         "“Yazılar” katmanının 3 nesnesi EPSG:4326'den projenin sistemine çevrilemiyor (1 nesne sistemin "
                         "dışında, 2 nesne yay ya da eğrili çizgi, daire, yazı, blok gibi tam çevrilemeyen türden); "
                         "hiçbir şey yazılmadı. Doğru sistemi from ile verin; yalnız noktalar ve düz kenarlı çizgiler "
                         "çevrilir, öbürlerini başka bir katmana alın.",
                         "from", ["$uidOf:17", "$uidOf:18", "$uidOf:19"]))),
        case("ret: boylam olamayacak sayılar WGS 84'ten çevrilemez, hiçbir şey yazılmaz",
             step({"layer": "bozuk", "from": 4326},
                  failed("crs_unreached",
                         "“Bozuk” katmanının 1 nesnesi EPSG:4326'den projenin sistemine çevrilemiyor (sistemin dışında); hiçbir şey yazılmadı. Doğru sistemi from ile verin.",
                         "from", ["$uidOf:6"]))),
    ]
    return out


def build(name, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": name, "commandVersion": 1, "title": title,
            "note": note, "setup": setup(), "cases": cases}


def main():
    check_wkt()
    note = ("ADR 0583. Kayıt koordinat sistemleri kaydından; WKT2 EPSG'nin tanımından yazılır ve PROJ'da (pyproj) kodun "
            "kendisiyle aynı eksen sırası ve aynı noktalar olarak okunur. Çevrilen koordinatlar PROJ'un, ADR 0167'nin "
            "her datum çifti için adını verdiği EPSG yoluyla; 1 mm içinde. Bağımsız başvuru: "
            "scripts/fixtures/layer_crs_cases.py.")
    files = {
        "cad.layers.crs.json": build("cad.layers.crs", "Koordinat sistemi ▸ Tanımla: katmanın kaynak sistemi kaydı", note, crs_cases()),
        "cad.layers.reproject.json": build("cad.layers.reproject",
                                           "Koordinat sistemi ▸ Dönüştür: katmanın nesneleri bir sistemden projeninkine",
                                           note + " Önizleme (ADR 2500): nesne başına kayma PROJ'un noktalarından; "
                                           "notlar PROJ'un veritabanındaki EPSG kullanım alanlarından (nokta kaynağın "
                                           "datumundaki yerinde ve vardığı yerde okunur); zincirin kararı ADR 1230 "
                                           "§3'ün kurallarıyla.",
                                           reproject_cases()),
    }
    bad = False
    for name, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if path.read_text() != text:
                print(f"{name} güncel değil", file=sys.stderr)
                bad = True
            else:
                print(f"{name}: {len(data['cases'])} durum")
        else:
            path.write_text(text)
            print(f"{name}: {len(data['cases'])} durum yazıldı")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
