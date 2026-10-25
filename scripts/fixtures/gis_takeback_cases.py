#!/usr/bin/env python3
"""The shared cases of a GIS edit's two ways out (docs/adr/0634, plan/gis.md CBS-16), written without KentOS code:

  fixtures/gis-derivation/v1/ada-geri.gpkg     the derived dataset `ada.gpkg` edited in a GIS, made with OGR (--files)
  fixtures/gis-derivation/v1/ada-shp-geri.zip, ada-yol-geri.geojson, mimari-geri.gpkg
                                               the Shapefile, GeoJSON and architectural derivatives KentOS wrote
                                               (`cargo test -p kentos-headless --test gis derived_` with
                                               KENTOS_WRITE_GPKG=1) edited in a GIS, made with OGR and json (--files)
  fixtures/commands/v1/cad.gis.takeback.json   `cad.gis.takeback`: the edits written back to their sources
  fixtures/commands/v1/cad.gis.detach.json     `cad.gis.detach`: the derived tables made the GIS's own data

    /usr/bin/python3 scripts/fixtures/gis_takeback_cases.py --files     # makes ada-geri.gpkg again with OGR
    /usr/bin/python3 scripts/fixtures/gis_takeback_cases.py             # writes the cases
    /usr/bin/python3 scripts/fixtures/gis_takeback_cases.py --check     # checks the file's edits, compares the cases
    /usr/bin/python3 scripts/fixtures/gis_takeback_cases.py --written   # reads the files the Rust tests wrote with OGR

The edits (ADR 0634 §1's examples): a straight parcel's `ada` changed and the parcel moved, a parcel's number emptied,
the record's number set, the well (a circle) moved, the road's arc moved, a road renamed, a line moved with its Z, the
fit-point curve made a point, the survey point moved, the text's words and height changed and the text moved, a block
insertion's floor count and turn changed. What a case expects of a source is ADR 0634's rule applied to the drawing
(`gis_derivation_drawing.py`) and to the GIS's geometry as OGR reads it: straight geometry taken as it is, a curve's
chords taken and its analytic definition left (`link_broken`), a value read by its column's type and checked by the
layer's field. A row's lasting id is the file's (`kentos_uids`, read with sqlite3).
"""
import json
import math
import shutil
import sqlite3
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import cad_gis_derivation_cases as cg  # noqa: E402
import gis_derivation_drawing as dd  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/gis-derivation/v1"
EDITED = DIR / "ada-geri.gpkg"
MIMARI_EDITED = DIR / "mimari-geri.gpkg"
# The architectural derivative's edits: a wall's state (an attribute), width (a computed column) and place; the
# dimension's note (an attribute) and value (computed); the space's name (computed).
ARCH_EDITS = {
    "mimari_duvar": {"set": {"durum": "yikilacak", "width": 0.3}, "shift": (1.0, 0.0)},
    "kentos_dimensions": {"set": {"not": "arka", "value": 12.0}},
    "mimari_mahal": {"set": {"name": "Oturma"}},
}
WRITTEN = DIR / "out/ayrik.gpkg"
TAKEBACK = ROOT / "fixtures/commands/v1/cad.gis.takeback.json"
DETACH = ROOT / "fixtures/commands/v1/cad.gis.detach.json"
ANY = "$any"
LABEL = "GIS'ten geri al"

# The edits: (table, fid) → what the GIS did. A shift moves every vertex.
EDITS = [
    ("Parsel", 2, {"set": {"ada": "999"}, "shift": (1.0, 2.0)}),
    ("Parsel", 1, {"set": {"parsel_no": None}}),
    ("Parsel", 3, {"set": {"parsel_no": 12}}),
    ("Parsel", 4, {"shift": (1.0, 0.0)}),
    ("Yol", 1, {"shift": (0.5, 0.0)}),
    ("Yol", 2, {"set": {"ad": "E-200"}}),
    ("Yol", 3, {"shift": (0.5, 0.0)}),
    ("Çizim", 3, {"point": (dd.E + 270.0, dd.N)}),
    ("Çizim", 5, {"shift": (1.0, 1.0)}),
    ("kentos_annotations", 1, {"set": {"text": "Ada 102", "height": 2.5}, "shift": (1.0, 0.0)}),
    ("blok_Bina", 1, {"set": {"KAT": "7", "rotation_deg": 90.0}}),
]
# The source slot of each table's row.
SLOT = {(t["table"], fid): slot for t in cg.TABLES for fid, slot in enumerate(t["slots"], start=1)}


# ── The edited file ─────────────────────────────────────────────────────────

def ogr():
    from osgeo import ogr as o
    o.UseExceptions()
    return o


def shifted(g, dx, dy):
    if g.GetGeometryCount() > 0:
        for i in range(g.GetGeometryCount()):
            shifted(g.GetGeometryRef(i), dx, dy)
        return g
    for i in range(g.GetPointCount()):
        if g.Is3D():
            x, y, z = g.GetPoint(i)
            g.SetPoint(i, x + dx, y + dy, z)
        else:
            x, y = g.GetPoint_2D(i)
            g.SetPoint_2D(i, x + dx, y + dy)
    return g


def make_file():
    make_main()
    make_formats()


def make_main():
    o = ogr()
    shutil.copyfile(DIR / "ada.gpkg", EDITED)
    ds = o.Open(str(EDITED), 1)
    for table, fid, edit in EDITS:
        layer = ds.GetLayerByName(table)
        f = layer.GetFeature(fid)
        for k, v in edit.get("set", {}).items():
            if v is None:
                f.SetFieldNull(k)
            else:
                f.SetField(k, v)
        if "shift" in edit:
            f.SetGeometry(shifted(f.GetGeometryRef().Clone(), *edit["shift"]))
        if "point" in edit:
            p = o.Geometry(o.wkbPoint)
            p.AddPoint_2D(*edit["point"])
            f.SetGeometry(p)
        layer.SetFeature(f)
    ds = None
    print(f"wrote {EDITED}")


def uid_rows(layer, uid):
    layer.ResetReading()
    for f in layer:
        if f.GetField("kentos_uid") == uid:
            return f
    raise SystemExit(f"{uid} not in {layer.GetName()}")


def make_formats():
    """The Shapefile, GeoJSON and architectural derivatives edited as a GIS would."""
    import tempfile
    import zipfile
    o = ogr()
    # The Shapefile zip: the second parcel's ada, the line moved; the archive's other files as they were.
    with tempfile.TemporaryDirectory() as tmp:
        with zipfile.ZipFile(DIR / "ada.zip") as z:
            names = z.namelist()
            z.extractall(tmp)
        ds = o.Open(str(Path(tmp) / "Parsel.shp"), 1)
        layer = ds.GetLayer(0)
        f = uid_rows(layer, uid("Parsel", 2))
        f.SetField("ada", "999")
        layer.SetFeature(f)
        ds = None
        ds = o.Open(str(Path(tmp) / "Yol.shp"), 1)
        layer = ds.GetLayer(0)
        f = uid_rows(layer, uid("Yol", 3))
        f.SetGeometry(shifted(f.GetGeometryRef().Clone(), 0.5, 0.0))
        layer.SetFeature(f)
        ds = None
        with zipfile.ZipFile(DIR / "ada-shp-geri.zip", "w", zipfile.ZIP_DEFLATED) as z:
            for n in names:
                z.write(Path(tmp) / n, n)
    # The GeoJSON file: the polyline renamed, the line moved.
    fc = json.loads((DIR / "ada-yol.geojson").read_text())
    for f in fc["features"]:
        u = f["properties"]["kentos_uid"]
        if u == uid("Yol", 2):
            f["properties"]["ad"] = "E-200"
        if u == uid("Yol", 3):
            f["geometry"]["coordinates"] = [[x + 0.5, y, *z] for x, y, *z in f["geometry"]["coordinates"]]
    (DIR / "ada-yol-geri.geojson").write_text(json.dumps(fc, ensure_ascii=False))
    # The architectural GeoPackage: the wall's state, width and place; the dimension's note and value; the space's name.
    shutil.copyfile(DIR / "mimari.gpkg", MIMARI_EDITED)
    ds = o.Open(str(MIMARI_EDITED), 1)
    for table, edit in ARCH_EDITS.items():
        layer = ds.GetLayerByName(table)
        f = layer.GetFeature(1)
        for k, v in edit.get("set", {}).items():
            f.SetField(k, v)
        if "shift" in edit:
            f.SetGeometry(shifted(f.GetGeometryRef().Clone(), *edit["shift"]))
        layer.SetFeature(f)
    ds = None
    print("wrote ada-shp-geri.zip, ada-yol-geri.geojson, mimari-geri.gpkg")


def read_row(path, table, fid):
    """A row of `path` as OGR reads it: its geometry's vertices (x, y, z) by ring or part, its fields."""
    o = ogr()
    ds = o.Open(str(path))
    f = ds.GetLayerByName(table).GetFeature(fid)
    g = f.GetGeometryRef()

    def flat(g):
        if g is None:
            return None
        if g.GetGeometryCount() > 0:
            return [flat(g.GetGeometryRef(i)) for i in range(g.GetGeometryCount())]
        z = lambda i: g.GetZ(i) if g.Is3D() and not math.isnan(g.GetZ(i)) else None  # noqa: E731
        return [(g.GetX(i), g.GetY(i), z(i)) for i in range(g.GetPointCount())]

    out = {"geometry": flat(g), "type": g.GetGeometryName() if g else None,
           "fields": {k: f.GetField(k) for k in f.keys()}}
    ds = None
    return out


def check_file():
    """Every edit is in ada-geri.gpkg and nothing else differs from ada.gpkg."""
    edited = {(t, fid): e for t, fid, e in EDITS}
    for t in cg.TABLES:
        for fid, _slot in enumerate(t["slots"], start=1):
            a, b = read_row(DIR / "ada.gpkg", t["table"], fid), read_row(EDITED, t["table"], fid)
            e = edited.get((t["table"], fid), {})
            want_fields = dict(a["fields"])
            want_fields.update({k: v for k, v in e.get("set", {}).items()})
            if b["fields"] != want_fields:
                sys.exit(f"ada-geri.gpkg {t['table']} {fid}: fields {b['fields']} ≠ {want_fields}")
            want = a["geometry"]
            if "shift" in e:
                dx, dy = e["shift"]
                want = move(want, dx, dy)
            if "point" in e:
                want = [(e["point"][0], e["point"][1], None)]
            if b["geometry"] != want:
                sys.exit(f"ada-geri.gpkg {t['table']} {fid}: the geometry is not the edit's")


def move(g, dx, dy):
    if g is None:
        return None
    if g and isinstance(g[0], list):
        return [move(p, dx, dy) for p in g]
    return [(x + dx, y + dy, z) for x, y, z in g]


# ── What the cases expect ───────────────────────────────────────────────────

UIDS = cg.file_uids()


def uid(table, fid):
    return UIDS[(table, fid)]


def xy(x, y):
    return {"x": x, "y": y}


def ring_pts(ring):
    """A GIS ring's vertices without its closing one."""
    pts = list(ring)
    if len(pts) > 1 and pts[0][:2] == pts[-1][:2]:
        pts.pop()
    return pts


def fx(name):
    return f"$fixture:gis-derivation/v1/{name}"


def row(table, fid, take=None):
    r = {"table": table, "uid": uid(table, fid)}
    if take:
        r["take"] = take
    return r


def taken(table, fid, fields=(), geometry=False, broken=None):
    r = {"table": table, "uid": uid(table, fid)}
    if fields:
        r["fields"] = list(fields)
    if geometry:
        r["geometry"] = True
    if broken:
        r["broken"] = broken
    return r


def output(rows, changed, name="ada-geri.gpkg"):
    return {"file": name, "rows": rows, "changed": changed, "revision": "$current"}


def broken_warning(items):
    listed = ", ".join(f"{uid(t, fid)} ({kind})" for t, fid, kind in items)
    return {"code": "link_broken",
            "message": f"{len(items)} nesnenin kaynağı bir eğriydi; GIS'teki kirişleri aldı ve analitik tanımı "
                       f"bırakıldı: {listed}.", "path": "rows"}


def left_warning(n):
    return {"code": "geometry_left",
            "message": f"{n} satırın geometrisi GIS'te değişmiş; yalnız öznitelikleri alındı. Geometriyi almak için "
                       "take: geometry verin.", "path": "rows"}


def take(rows, result, expect, within=None, op="execute"):
    s = {"op": op, "input": {"path": fx("ada-geri.gpkg"), "rows": rows}, "result": result, "expect": expect}
    if within is not None:
        s["within"] = within
    return s


def done(out, warnings=()):
    return {"status": "completed", "output": out, "warnings": list(warnings)}


def refused(code, path, inp=None, status="failed", undo=False):
    """A refusal: nothing written; `undo` what the undo stack holds (none: earlier steps' own, not asked)."""
    err = {"code": code, "message": ANY, "path": path}
    if status == "conflict":
        err["revision"] = "$current"
    expect = {"revision": "same"}
    if undo is not None:
        expect["canUndo"] = undo
    return {"op": "execute", "input": inp, "result": {"status": status, "error": err}, "expect": expect}


def fields_step(changes):
    """The parcel layer's fields with `changes` (a field's name → its new rules), as cad.layers.edit writes them."""
    fields = json.loads(json.dumps(next(n for n in cg.DRAWING["layers"] if n["id"] == "parsel")["fields"]))
    for f in fields:
        f.update(changes.get(f["name"], {}))
    return cg.run("cad.layers.edit", {"operation": "fields", "layer": "parsel", "fields": fields})


def wrote(fields):
    return {"revision": "changed", "canUndo": True, "fields": fields}


ENTS = cg.ENTS


def geri(table, fid):
    return read_row(EDITED, table, fid)


def parsel2_moved():
    return [xy(p["x"] + 1.0, p["y"] + 2.0) for p in ENTS[2]["pts"]]


def circle_pts():
    ring = geri("Parsel", 4)["geometry"][0]
    return [xy(x, y) for x, y, _ in ring_pts(ring)]


def arc_pts():
    return [xy(x, y) for x, y, _ in geri("Yol", 1)["geometry"]]


def case(name, steps, setup=None):
    c = {"name": name, "steps": steps}
    if setup is not None:
        c["setup"] = setup
    return c


def takeback_cases():
    p1 = ENTS[1]
    c = []
    c.append(case("öznitelik: GIS'te değişen ada kaynağa yazılır, taşınan geometri kalır ve söylenir", [
        take([row("Parsel", 2)], done(output([taken("Parsel", 2, ["ada"])], 1), [left_warning(1)]),
             wrote({"2": {"attrs": {"ada": "999", "parsel_no": "8"}, "pts": ENTS[2]["pts"]}}))]))
    c.append(case("geometri: düz kenarlı parsel GIS'teki yerini olduğu gibi alır, öznitelikleriyle", [
        take([row("Parsel", 2, "geometry")], done(output([taken("Parsel", 2, ["ada"], geometry=True)], 1)),
             wrote({"2": {"kind": "polygon", "attrs": {"ada": "999", "parsel_no": "8"}, "pts": parsel2_moved(),
                          "bulges": None}}))]))
    c.append(case("GIS'te boşaltılan değer kaynağın özniteliğini siler; yaylı parselin geometrisi değişmez", [
        take([row("Parsel", 1)], done(output([taken("Parsel", 1, ["parsel_no"])], 1)),
             wrote({"1": {"attrs": {"ada": "101"}, "pts": p1["pts"], "bulges": p1["bulges"], "holes": p1["holes"]}}))]))
    c.append(case("zorunlu alan GIS'te boşaltılmışsa geri alınmaz", [
        fields_step({"parsel_no": {"required": True}}),
        refused("attribute_required", "rows.0.attrs.parsel_no",
                {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 1)]}, undo=None)]))
    c.append(case("alanın kuralına uymayan değer geri alınmaz: ada en çok iki karakter", [
        fields_step({"ada": {"length": 2}}),
        refused("invalid_attribute", "rows.0.attrs.ada", {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2)]},
                undo=None)]))
    c.append(case("geometrisiz kayıt: GIS'te yazılan numara alınır, geometri istense de yoktur", [
        take([row("Parsel", 3, "geometry")], done(output([taken("Parsel", 3, ["parsel_no"])], 1)),
             wrote({"3": {"kind": "record", "attrs": {"ada": "102", "parsel_no": "12"}}}))]))
    c.append(case("daire: GIS'te taşınan kuyunun kirişleri alınır, dairenin tanımı bırakılır (bağ koptu)", [
        take([row("Parsel", 4, "geometry")],
             done(output([taken("Parsel", 4, geometry=True, broken="circle")], 1),
                  [broken_warning([("Parsel", 4, "circle")])]),
             wrote({"4": {"kind": "polygon", "attrs": {"ada": "kuyu"}, "pts": circle_pts(), "c": None, "r": None}}))]))
    c.append(case("yay: taşınan yolun kirişleri çoklu çizgi olur, yayın tanımı bırakılır", [
        take([row("Yol", 1, "geometry")],
             done(output([taken("Yol", 1, geometry=True, broken="arc")], 1), [broken_warning([("Yol", 1, "arc")])]),
             wrote({"5": {"kind": "polyline", "attrs": {"ad": "D-100"}, "pts": arc_pts(), "c": None, "a0": None}}))]))
    c.append(case("yalnız geometrisi değişen satırın öznitelikleri istenirse hiçbir şey yazılmaz, söylenir", [
        take([row("Yol", 1)], done(output([taken("Yol", 1)], 0), [left_warning(1)]),
             {"revision": "same", "canUndo": False, "fields": {"5": {"kind": "arc", "r": 60.0}}})]))
    c.append(case("bükümlü çoklu çizginin yalnız adı değişti: ad alınır, yayları ve kotları kalır", [
        take([row("Yol", 2)], done(output([taken("Yol", 2, ["ad"])], 1)),
             wrote({"6": {"attrs": {"ad": "E-200"}, "bulges": ENTS[6]["bulges"], "zs": ENTS[6]["zs"],
                          "ms": ENTS[6]["ms"]}}))]))
    a, b = ENTS[7]["a"], ENTS[7]["b"]
    c.append(case("çizgi: GIS'te taşınan çizgi uçlarını kotlarıyla alır", [
        take([row("Yol", 3, "geometry")], done(output([taken("Yol", 3, geometry=True)], 1)),
             wrote({"7": {"kind": "line", "a": xy(a["x"] + 0.5, a["y"]), "b": xy(b["x"] + 0.5, b["y"]),
                          "za": 5.0, "zb": 6.0}}))]))
    c.append(case("eğrinin GIS'te noktaya çevrilen geometrisi türüne uymaz", [
        refused("geometry_kind_mismatch", "rows.0",
                {"path": fx("ada-geri.gpkg"), "rows": [row("Çizim", 3, "geometry")]})]))
    c.append(case("aynı eğrinin öznitelikleri istenirse geometrisi bırakılır, söylenir", [
        take([row("Çizim", 3)], done(output([taken("Çizim", 3)], 0), [left_warning(1)]),
             {"revision": "same", "fields": {"11": {"kind": "spline"}}})]))
    p13 = ENTS[13]["p"]
    c.append(case("nokta: GIS'te taşınan ölçü noktası yerini kotuyla alır", [
        take([row("Çizim", 5, "geometry")], done(output([taken("Çizim", 5, geometry=True)], 1)),
             wrote({"13": {"kind": "point", "p": xy(p13["x"] + 1.0, p13["y"] + 1.0), "z": 7.5,
                           "attrs": {"ad": "P-1"}}}))]))
    p14 = ENTS[14]["p"]
    c.append(case("yazı: GIS'te değişen metin ve yükseklik alınır, yeri kalır", [
        take([row("kentos_annotations", 1)],
             done(output([taken("kentos_annotations", 1, ["text", "height"])], 1), [left_warning(1)]),
             wrote({"14": {"text": "Ada 102", "height": 2.5, "rotation": 15.0, "p": p14}}))]))
    c.append(case("yazı geometrisiyle: yeri de GIS'teki olur", [
        take([row("kentos_annotations", 1, "geometry")],
             done(output([taken("kentos_annotations", 1, ["text", "height"], geometry=True)], 1)),
             wrote({"14": {"text": "Ada 102", "height": 2.5, "p": xy(p14["x"] + 1.0, p14["y"])}}))]))
    c.append(case("blok yerleştirmesi: GIS'te değişen dönüş (derece) ve kat sayısı alınır", [
        take([row("blok_Bina", 1)], done(output([taken("blok_Bina", 1, ["rotation_deg", "KAT"])], 1)),
             wrote({"15": {"rotation": math.radians(90.0), "attrs": {"NO": "B-1", "KAT": "7"}, "p": ENTS[15]["p"]}}),
             within=1e-12)]))
    many = [row("Parsel", 2, "geometry"), row("Parsel", 4, "geometry"), row("kentos_annotations", 1),
            row("blok_Bina", 1)]
    many_out = output([taken("Parsel", 2, ["ada"], geometry=True), taken("Parsel", 4, geometry=True, broken="circle"),
                       taken("kentos_annotations", 1, ["text", "height"]),
                       taken("blok_Bina", 1, ["rotation_deg", "KAT"])], 4)
    many_warn = [broken_warning([("Parsel", 4, "circle")]), left_warning(1)]
    c.append(case("birkaç tablonun satırları tek geri alma adımı; geri alınca kaynaklar eski hâlinde", [
        take(many, done(many_out, many_warn), {"revision": "changed", "canUndo": True}),
        {"op": "undo", "returns": LABEL, "expect": {"fields": {
            "2": {"kind": "polygon", "pts": ENTS[2]["pts"], "attrs": ENTS[2]["attrs"]},
            "4": {"kind": "circle", "r": 2.5}, "14": {"text": "Ada 101", "height": 2.0},
            "15": {"rotation": 0, "attrs": ENTS[15]["attrs"]}}}}]))
    c.append(case("plan yazmadan aynı cevabı verir", [
        take(many, done(many_out, many_warn), {"revision": "same", "canUndo": False}, op="plan")]))
    c.append(case("doğrulama yazmadan denetler", [
        take(many, {"status": "completed", "output": None, "warnings": []}, {"revision": "same"}, op="validate")]))
    c.append(case("geri alınan satır yeniden alınamaz: kaynağı artık türevdekinden farklı", [
        take([row("Parsel", 2)], done(output([taken("Parsel", 2, ["ada"])], 1), [left_warning(1)]),
             {"revision": "changed"}),
        refused("row_stale", "rows.0", {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2)]}, undo=None)]))
    c.append(case("kaynağı çizimde de değişen satır alınmaz: iki yan da değişti", [
        cg.run("cad.entities.set", {"uids": ["$uidOf:2"], "attrs": {"parsel_no": "9"}, "operation": "attributes"}),
        refused("row_stale", "rows.0", {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2)]}, undo=None)]))
    c.append(case("kaynağı silinmiş satırın yazılacağı nesne yok", [
        cg.run("cad.entities.delete", {"uids": ["$uidOf:2"]}),
        refused("row_source_gone", "rows.0", {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2)]}, undo=None)]))
    c.append(case("GIS'te değişmemiş satır: alınacak bir şey yok", [
        refused("row_not_edited", "rows.0", {"path": fx("ada-geri.gpkg"), "rows": [row("Bina", 1)]})]))
    c.append(case("bir satır reddedilirse hiçbiri yazılmaz", [
        refused("row_not_edited", "rows.1",
                {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2), row("Bina", 1)]})]))
    c.append(case("kilitli katmandaki kaynağa yazılmaz", [
        cg.run("cad.layers.set", {"operation": "locked", "layer": "parsel", "value": True}),
        refused("layer_locked", "rows.0", {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2)]}, undo=None)]))
    c.append(case("dosyada olmayan kimlik", [
        refused("derivation_row_not_found", "rows.0.uid",
                {"path": fx("ada-geri.gpkg"), "rows": [{"table": "Parsel", "uid": uid("Yol", 1)}]})]))
    c.append(case("dosyada olmayan tablo", [
        refused("derivation_row_not_found", "rows.0.table",
                {"path": fx("ada-geri.gpkg"), "rows": [{"table": "Yok", "uid": uid("Yol", 1)}]})]))
    c.append(case("satırsız istek", [refused("invalid_count", "rows", {"path": fx("ada-geri.gpkg"), "rows": []})]))
    c.append(case("aynı satır iki kez", [
        refused("invalid_count", "rows.1", {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2), row("Parsel", 2)]})]))
    c.append(case("biçimsiz beklenen sürüm", [
        refused("invalid_revision", "expectedRevision",
                {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2)], "expectedRevision": "07"})]))
    c.append(case("çizim beklenen sürümde değil", [
        refused("revision_conflict", "expectedRevision",
                {"path": fx("ada-geri.gpkg"), "rows": [row("Parsel", 2)], "expectedRevision": "999"},
                status="conflict")]))
    c.append(case("yol mutlak değil", [refused("invalid_file_path", "path", {"path": "ada-geri.gpkg", "rows": [row("Parsel", 2)]})]))
    c.append(case("dosya yok", [refused("file_not_found", "path", {"path": fx("yok.gpkg"), "rows": [row("Parsel", 2)]})]))
    c.append(case("GeoPackage olmayan dosya", [
        refused("import_unreadable", "path", {"path": fx("drawing.json"), "rows": [row("Parsel", 2)]})]))
    c.append(case("türetim kaydı olmayan GeoPackage", [
        refused("not_a_derivation", "path", {"path": "$fixture:geopackage/v1/qgis.gpkg", "rows": [row("Parsel", 2)]})]))
    return c + format_cases()


# ── Other containers and architectural objects ─────────────────────────────

ARCH = json.loads((DIR / "mimari.json").read_text())
AENTS = {e["id"]: e for e in ARCH["entities"]}


def arch_uid(table):
    c = sqlite3.connect(f"file:{DIR / 'mimari.gpkg'}?mode=ro", uri=True)
    u = c.execute("SELECT uid FROM kentos_uids WHERE table_name = ? AND fid = 1", (table,)).fetchone()[0]
    c.close()
    return u


def facts(name):
    import hashlib
    data = (DIR / name).read_bytes()
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def derived_out(name, tables, source=None, entries=()):
    """`cad.gis.derivation`'s answer for `tables` (table, role, layer or None, rows, max deviation) all current but
    for `entries` (table, fid, uid, kind, fields): rows edited in the GIS."""
    rows = []
    edited = {}
    for t, fid, u, kind, fields in entries:
        edited[t] = edited.get(t, 0) + 1
    for table, role, layer, n, dev in tables:
        r = {"table": table, "role": role}
        if layer:
            r["layer"] = layer
        e = edited.get(table, 0)
        r.update({"rows": n, "current": n - e, "stale": 0, "edited": e, "sourceGone": 0, "notExported": 0,
                  "overTolerance": 0, "tolerance": 1e-4, "maxDeviation": dev})
        rows.append(r)
    listed = [{"table": t, "fid": fid, "uid": u, "status": "edited", "kind": kind, "fields": fields,
               "reason": cg.edited_reason(fields)} for t, fid, u, kind, fields in entries]
    n_all = sum(t[3] for t in tables)
    return {**facts(name), "algorithm": "kentos.chords/1", "tolerance": 1e-4,
            "source": source or cg.SOURCE, "sameProject": True, "tables": rows,
            "current": n_all - len(entries), "stale": 0, "edited": len(entries), "sourceGone": 0, "notExported": 0,
            "entries": listed, "unlisted": 0, "revision": "$current"}


def dev(slots):
    return max(cg.deviation_of(cg.ENTS[s]) for s in slots)


# The Shapefile's sets: a table a set of each geometry family; a record in its table's first family's set.
SHP_SETS = [("Parsel", "layer", "parsel", [1, 2, 3, 4]), ("blok_Bina", "blocks", None, [15, 16]),
            ("Bina", "layer", "bina", [17]), ("Yol", "layer", "yol", [5, 6, 7, 8]),
            ("kentos_annotations", "annotations", None, [14]), ("Çizim_nokta", "layer", "cizim", [13]),
            ("Çizim_cizgi", "layer", "cizim", [10, 11, 12]), ("Çizim_alan", "layer", "cizim", [9])]
GPKG_TABLES = [(t["table"], t["role"], t.get("layer"), t["slots"]) for t in cg.TABLES]
ARCH_TABLES = [("mimari_duvar", 1), ("mimari_mahal", 3), ("mimari_aks", 4), ("mimari_kolon", 5),
               ("mimari_merdiven", 6), ("mimari_rampa", 7), ("mimari_aciklik", 8)]
ARCH_SOURCE = {"name": "Mimari türev", "project": ANY, "revision": "0", "srid": 5256}


def tables_of(sets):
    return [(t, role, layer, len(slots), dev(slots)) for t, role, layer, slots in sets]


def check(name, out, warnings=(), setup_note=None):
    return {"op": "execute", "command": "cad.gis.derivation", "input": {"path": fx(name)},
            "result": {"status": "completed", "output": out, "warnings": list(warnings)}, "within": 1e-12,
            "expect": {"revision": "same"}}


def edited_warning(n):
    return {"code": "derivation_edited",
            "message": f"{n} satır GIS'te değiştirilmiş. Kaynak olan çizimdeki nesneler değişmedi: değişikliği "
                       "cad.gis.takeback ile kaynağa alın ya da cad.gis.detach ile tablonun bağını koparın.",
            "path": "path"}


def fields_left_warning(items):
    listed = "; ".join(f"{u} ({', '.join(f)})" for u, f in items)
    return {"code": "fields_left",
            "message": f"{len(items)} satırın GIS'te değişen hesaplanmış sütunları kaynağa yazılmadı (mimari nesnenin "
                       f"ve ölçünün değerleri çizimde değiştirilir): {listed}.", "path": "rows"}


def format_cases():
    c = []
    c.append(case("Shapefile türevi: her tablo geometri türü başına bir set, bütün satırlar güncel", [
        check("ada.zip", derived_out("ada.zip", tables_of(SHP_SETS)))]))
    c.append(case("GeoJSON türevi (zip): her tablo bir dosya, bütün satırlar güncel", [
        check("ada-geojson.zip", derived_out("ada-geojson.zip", tables_of(GPKG_TABLES)))]))
    c.append(case("tek tablolu GeoJSON türevi: bütün satırlar güncel, M'si olmayan kap farklı saymaz", [
        check("ada-yol.geojson", derived_out("ada-yol.geojson", tables_of([("Yol", "layer", "yol", [5, 6, 7, 8])])))]))
    arch = [(t, "arch", None, 1, 0.0) for t, _ in ARCH_TABLES] + [("kentos_dimensions", "dimensions", None, 1, 0.0)]
    c.append(case("mimari türev: her nesne türü ve ölçüler kendi tablosunda, hepsi güncel", [
        check("mimari.gpkg", derived_out("mimari.gpkg", arch, ARCH_SOURCE))], setup=ARCH))
    shp_edits = [("Parsel", 2, uid("Parsel", 2), "polygon", ["ada"]), ("Yol", 3, uid("Yol", 3), "line", ["geometry"])]
    c.append(case("GIS'te düzenlenen Shapefile: değişen ad ve taşınan çizgi", [
        check("ada-shp-geri.zip", derived_out("ada-shp-geri.zip", tables_of(SHP_SETS), entries=shp_edits),
              [edited_warning(2)])]))
    shp = lambda rows: {"path": fx("ada-shp-geri.zip"), "rows": rows}  # noqa: E731
    c.append(case("Shapefile'dan öznitelik: setin satırındaki ada kaynağa yazılır", [
        {**take([row("Parsel", 2)], done(output([taken("Parsel", 2, ["ada"])], 1, "ada-shp-geri.zip")),
                wrote({"2": {"attrs": {"ada": "999", "parsel_no": "8"}, "pts": ENTS[2]["pts"]}})),
         "input": shp([row("Parsel", 2)])}]))
    a, b = ENTS[7]["a"], ENTS[7]["b"]
    c.append(case("Shapefile'dan geometri: taşınan çizgi kotlarıyla", [
        {**take([row("Yol", 3, "geometry")], done(output([taken("Yol", 3, geometry=True)], 1, "ada-shp-geri.zip")),
                wrote({"7": {"kind": "line", "a": xy(a["x"] + 0.5, a["y"]), "b": xy(b["x"] + 0.5, b["y"]),
                             "za": 5.0, "zb": 6.0}})),
         "input": shp([row("Yol", 3, "geometry")])}]))
    gj = lambda rows: {"path": fx("ada-yol-geri.geojson"), "rows": rows}  # noqa: E731
    c.append(case("GeoJSON'dan öznitelik: yeniden adlandırılan yol, yayları ve ölçüleri kalır", [
        {**take([row("Yol", 2)], done(output([taken("Yol", 2, ["ad"])], 1, "ada-yol-geri.geojson")),
                wrote({"6": {"attrs": {"ad": "E-200"}, "bulges": ENTS[6]["bulges"], "ms": ENTS[6]["ms"]}})),
         "input": gj([row("Yol", 2)])}]))
    c.append(case("GeoJSON'dan geometri: taşınan çizgi kotlarıyla", [
        {**take([row("Yol", 3, "geometry")], done(output([taken("Yol", 3, geometry=True)], 1, "ada-yol-geri.geojson")),
                wrote({"7": {"kind": "line", "a": xy(a["x"] + 0.5, a["y"]), "za": 5.0, "zb": 6.0}})),
         "input": gj([row("Yol", 3, "geometry")])}]))
    m = lambda rows: {"path": fx("mimari-geri.gpkg"), "rows": rows}  # noqa: E731
    wall, dim, space = arch_uid("mimari_duvar"), arch_uid("kentos_dimensions"), arch_uid("mimari_mahal")
    arow = lambda table, u, take=None: {"table": table, "uid": u, **({"take": take} if take else {})}  # noqa: E731
    atake = lambda table, u, fields=(): {"table": table, "uid": u, **({"fields": list(fields)} if fields else {})}  # noqa: E731
    c.append(case("duvar: durum kaynağa yazılır; genişlik hesaplanmış sütun, yer geometri: ikisi de söylenir", [
        {"op": "execute", "input": m([arow("mimari_duvar", wall)]),
         "result": done({"file": "mimari-geri.gpkg", "rows": [atake("mimari_duvar", wall, ["durum"])], "changed": 1,
                         "revision": "$current"},
                        [fields_left_warning([(wall, ["width"])]), left_warning(1)]),
         "expect": wrote({"1": {"attrs": {"durum": "yikilacak"}, "width": 0.2, "pts": AENTS[1]["pts"]}})}],
        setup=ARCH))
    c.append(case("duvarın GIS'teki geometrisi alınmaz: değerlerinden çizilir", [
        refused("geometry_parametric", "rows.0", m([arow("mimari_duvar", wall, "geometry")]))], setup=ARCH))
    c.append(case("ölçü: notu kaynağa yazılır, ölçülen değer çizimdendir", [
        {"op": "execute", "input": m([arow("kentos_dimensions", dim)]),
         "result": done({"file": "mimari-geri.gpkg", "rows": [atake("kentos_dimensions", dim, ["not"])], "changed": 1,
                         "revision": "$current"}, [fields_left_warning([(dim, ["value"])])]),
         "expect": wrote({"9": {"attrs": {"not": "arka"}, "a": AENTS[9]["a"], "b": AENTS[9]["b"]}})}], setup=ARCH))
    c.append(case("mahalin GIS'te değişen adı hesaplanmış sütundur: hiçbir şey yazılmaz, söylenir", [
        {"op": "execute", "input": m([arow("mimari_mahal", space)]),
         "result": done({"file": "mimari-geri.gpkg", "rows": [atake("mimari_mahal", space)], "changed": 0,
                         "revision": "$current"}, [fields_left_warning([(space, ["name"])])]),
         "expect": {"revision": "same", "canUndo": False, "fields": {"3": {"name": "Salon"}}}}], setup=ARCH))
    c.append(case("veritabanı adresi parolayla verilmez", [
        refused("invalid_address", "path", {"path": "postgresql://kentos:gizli@127.0.0.1/turev",
                                             "rows": [row("Parsel", 2)]})]))
    c.append(case("PostGIS sağlayıcısı olmayan ortamda veritabanı okunamaz", [
        refused("source_unreachable", "path", {"path": "postgresql://kentos@127.0.0.1:1/turev",
                                                "rows": [row("Parsel", 2)]})]))
    c.append(case("türev dizini olmayan zip türev değildir", [
        refused("not_a_derivation", "path", {"path": "$fixture:gis-import/v1/parsel.zip", "rows": [row("Parsel", 2)]})]))
    c.append(case("türetim kaydı olmayan GeoJSON türev değildir", [
        refused("not_a_derivation", "path", {"path": "$fixture:gis-import/v1/katmanli.geojson",
                                              "rows": [row("Parsel", 2)]})]))
    return c


# ── cad.gis.detach ──────────────────────────────────────────────────────────

ORDER = [t["table"] for t in cg.TABLES]
ROWS = {t["table"]: len(t["slots"]) for t in cg.TABLES}
SOURCE = cg.DRAWING["name"]


def detached(tables, linked, name):
    return {"file": name, "tables": [{"table": t, "source": SOURCE, "rows": ROWS[t]} for t in tables],
            "linked": linked}


def detach(path, tables=None, reason=None):
    inp = {"path": path}
    if tables is not None:
        inp["tables"] = tables
    if reason is not None:
        inp["reason"] = reason
    return inp


def copy(name, to):
    return {"op": "copyFile", "from": fx(name), "to": f"$tmp:{to}"}


def det(inp, out, expect=None, op="execute"):
    s = {"op": op, "command": "cad.gis.detach", "input": inp,
         "result": {"status": "completed", "output": out, "warnings": []},
         "expect": expect or {"revision": "same", "canUndo": False}}
    return s


def other(command, inp, result, expect):
    return {"op": "execute", "command": command, "input": inp, "result": result, "expect": expect}


def drefused(code, path, inp):
    return {"op": "execute", "command": "cad.gis.detach", "input": inp,
            "result": {"status": "failed", "error": {"code": code, "message": ANY, "path": path}},
            "expect": {"revision": "same", "canUndo": False}}


def detach_cases():
    c = []
    c.append(case("plan: bütün türev tablolar, satırlarıyla; dosya da çizim de değişmez", [
        det(detach(fx("ada-geri.gpkg")), detached(ORDER, 0, "ada-geri.gpkg"), op="plan")]))
    c.append(case("doğrulama yazmadan denetler", [
        {"op": "validate", "command": "cad.gis.detach", "input": detach(fx("ada-geri.gpkg"), ["Parsel"]),
         "result": {"status": "completed", "output": None, "warnings": []}, "expect": {"revision": "same"}}]))
    c.append(case("bir tablonun bağı kopar, öbürleri türev kalır; çizim değişmez", [
        copy("ada-geri.gpkg", "ayrik-1.gpkg"),
        det(detach("$tmp:ayrik-1.gpkg", ["Parsel"], "Kadastro güncellemesi GIS'te sürecek"),
            detached(["Parsel"], 5, "ayrik-1.gpkg")),
        det(detach("$tmp:ayrik-1.gpkg"), detached([t for t in ORDER if t != "Parsel"], 0, "ayrik-1.gpkg"),
            op="plan")]))
    c.append(case("bağı kopan tablonun satırı artık geri alınmaz; türev kalan tablonunki alınır", [
        copy("ada-geri.gpkg", "ayrik-2.gpkg"),
        det(detach("$tmp:ayrik-2.gpkg", ["Parsel"]), detached(["Parsel"], 5, "ayrik-2.gpkg")),
        refused("derivation_row_not_found", "rows.0.table",
                {"path": "$tmp:ayrik-2.gpkg", "rows": [row("Parsel", 2)]}) | {"command": "cad.gis.takeback"},
        other("cad.gis.takeback", {"path": "$tmp:ayrik-2.gpkg", "rows": [row("Yol", 2)]},
              done(output([taken("Yol", 2, ["ad"])], 1, "ayrik-2.gpkg")), {"revision": "changed"})]))
    c.append(case("bütün tabloların bağı kopunca dosya türev değildir", [
        copy("ada-geri.gpkg", "ayrik-3.gpkg"),
        det(detach("$tmp:ayrik-3.gpkg"), detached(ORDER, 0, "ayrik-3.gpkg")),
        other("cad.gis.derivation", {"path": "$tmp:ayrik-3.gpkg"},
              {"status": "failed", "error": {"code": "not_a_derivation", "message": ANY, "path": "path"}},
              {"revision": "same"}),
        drefused("not_a_derivation", "path", detach("$tmp:ayrik-3.gpkg"))]))
    c.append(case("bağı kopan tablo katmana bağlanır: artık türev değil", [
        copy("ada.gpkg", "ayrik-4.gpkg"),
        det(detach("$tmp:ayrik-4.gpkg", ["Yol"]), detached(["Yol"], 5, "ayrik-4.gpkg")),
        other("cad.sources.edit", {"edit": {"kind": "link", "path": "$tmp:ayrik-4.gpkg", "table": "Yol"}},
              {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"})]))
    c.append(case("bağı kopan dosya aynı çizime alınır: satırlar çizimin kimliklerini bıraktı, yeni nesnelerdir", [
        copy("ada.gpkg", "ayrik-5.gpkg"),
        det(detach("$tmp:ayrik-5.gpkg"), detached(ORDER, 0, "ayrik-5.gpkg")),
        other("file.import.gpkg", {"path": "$tmp:ayrik-5.gpkg", "tables": ["Bina"]},
              {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"})]))
    c.append(case("bağı kopmamış türev aynı çizime alınamaz", [
        other("file.import.gpkg", {"path": fx("ada.gpkg"), "tables": ["Bina"]},
              {"status": "failed", "error": {"code": "entity_exists", "message": ANY, "path": "path"}},
              {"revision": "same"})]))
    c.append(case("yol mutlak değil", [drefused("invalid_file_path", "path", detach("ada.gpkg"))]))
    c.append(case("yol klasör", [drefused("invalid_file_path", "path", detach(fx("")))]))
    c.append(case("dosya yok", [drefused("file_not_found", "path", detach(fx("yok.gpkg")))]))
    c.append(case("GeoPackage olmayan dosya", [drefused("import_unreadable", "path", detach(fx("drawing.json")))]))
    c.append(case("türetim kaydı olmayan GeoPackage", [
        drefused("not_a_derivation", "path", detach("$fixture:geopackage/v1/qgis.gpkg"))]))
    c.append(case("dosyada olmayan tablo", [
        drefused("derivation_row_not_found", "tables.1", detach(fx("ada.gpkg"), ["Parsel", "Yok"]))]))
    c.append(case("aynı tablo iki kez", [
        drefused("invalid_count", "tables.1", detach(fx("ada.gpkg"), ["Yol", "Yol"]))]))
    c.append(case("kayıtsız tablo adı: büyük küçük harf ayrıdır", [
        drefused("derivation_row_not_found", "tables.0", detach(fx("ada.gpkg"), ["parsel"]))]))
    c.append(case("blok ve yazı tabloları da türevdir, bağları ayrı koparılır", [
        copy("ada.gpkg", "ayrik-6.gpkg"),
        det(detach("$tmp:ayrik-6.gpkg", ["blok_Bina", "kentos_annotations"]),
            detached(["blok_Bina", "kentos_annotations"], 4, "ayrik-6.gpkg"))]))
    c.append(case("tablolar dosyadaki sıralarıyla söylenir, verilen sırayla değil", [
        det(detach(fx("ada.gpkg"), ["Çizim", "Parsel"]), detached(["Parsel", "Çizim"], 4, "ada.gpkg"), op="plan")]))
    c.append(case("boş neden yazılmaz, bağ yine kopar", [
        copy("ada.gpkg", "ayrik-7.gpkg"),
        det(detach("$tmp:ayrik-7.gpkg", ["Bina"], "  "), detached(["Bina"], 5, "ayrik-7.gpkg"))]))
    sets = [t for t, *_ in SHP_SETS]
    shp_rows = {t: len(slots) for t, _, _, slots in SHP_SETS}

    def shp_detached(tables, linked, name):
        return {"file": name, "tables": [{"table": t, "source": SOURCE, "rows": shp_rows[t]} for t in tables],
                "linked": linked}
    c.append(case("Shapefile türevinin bir setinin bağı kopar; o setin satırı artık geri alınmaz", [
        copy("ada-shp-geri.zip", "ayrik-shp-1.zip"),
        det(detach("$tmp:ayrik-shp-1.zip", ["Parsel"]), shp_detached(["Parsel"], 7, "ayrik-shp-1.zip")),
        refused("derivation_row_not_found", "rows.0.table",
                {"path": "$tmp:ayrik-shp-1.zip", "rows": [row("Parsel", 2)]}) | {"command": "cad.gis.takeback"},
        other("cad.gis.takeback", {"path": "$tmp:ayrik-shp-1.zip", "rows": [row("Yol", 3, "geometry")]},
              done(output([taken("Yol", 3, geometry=True)], 1, "ayrik-shp-1.zip")), {"revision": "changed"})]))
    c.append(case("Shapefile türevinin bütün setlerinin bağı kopunca zip türev değildir", [
        copy("ada.zip", "ayrik-shp-2.zip"),
        det(detach("$tmp:ayrik-shp-2.zip"), shp_detached(sets, 0, "ayrik-shp-2.zip")),
        other("cad.gis.derivation", {"path": "$tmp:ayrik-shp-2.zip"},
              {"status": "failed", "error": {"code": "not_a_derivation", "message": ANY, "path": "path"}},
              {"revision": "same"})]))
    c.append(case("GeoJSON türevinin bağı kopar; dosya türev değildir, geri alma reddedilir", [
        copy("ada-yol-geri.geojson", "ayrik-yol.geojson"),
        det(detach("$tmp:ayrik-yol.geojson"),
            {"file": "ayrik-yol.geojson", "tables": [{"table": "Yol", "source": SOURCE, "rows": 4}], "linked": 0}),
        refused("not_a_derivation", "path",
                {"path": "$tmp:ayrik-yol.geojson", "rows": [row("Yol", 2)]}) | {"command": "cad.gis.takeback"}]))
    c.append(case("GeoJSON zip'inde bir tablonun bağı kopar, öbürleri türev kalır", [
        copy("ada-geojson.zip", "ayrik-gj.zip"),
        det(detach("$tmp:ayrik-gj.zip", ["Yol"]), detached(["Yol"], 5, "ayrik-gj.zip")),
        det(detach("$tmp:ayrik-gj.zip"), detached([t for t in ORDER if t != "Yol"], 0, "ayrik-gj.zip"), op="plan")]))
    c.append(case("mimari türevin tablolarının bağı kopar", [
        copy("mimari.gpkg", "ayrik-mimari.gpkg"),
        det(detach("$tmp:ayrik-mimari.gpkg", ["mimari_duvar", "kentos_dimensions"]),
            {"file": "ayrik-mimari.gpkg", "tables": [
                {"table": "mimari_duvar", "source": "Mimari türev", "rows": 1},
                {"table": "kentos_dimensions", "source": "Mimari türev", "rows": 1}], "linked": 6})], setup=ARCH))
    c.append(case("veritabanı adresi parolayla verilmez", [
        drefused("invalid_address", "path", detach("postgresql://kentos:gizli@127.0.0.1/turev"))]))
    c.append(case("kopan bağ geri gelmez: aynı tablo yeniden koparılamaz", [
        copy("ada.gpkg", "ayrik-8.gpkg"),
        det(detach("$tmp:ayrik-8.gpkg", ["Bina"]), detached(["Bina"], 5, "ayrik-8.gpkg")),
        drefused("derivation_row_not_found", "tables.0", detach("$tmp:ayrik-8.gpkg", ["Bina"]))]))
    return c


def setup():
    return cg.DRAWING


def files():
    common = {"format": "kentos.command-cases", "version": 1, "commandVersion": 1, "setup": setup()}
    note = ("ADR 0634. Çizim fixtures/gis-derivation/v1/drawing.json; türev ada.gpkg ve GIS'te düzenlenmiş kopyası "
            "ada-geri.gpkg (scripts/fixtures/gis_takeback_cases.py OGR'la yapar ve denetler). Beklenenler ADR 0634'ün "
            "kuralı: düz geometri olduğu gibi, eğrinin kirişleri ve kopan bağ, değer sütunun türüyle ve katmanın "
            "alanının kuralıyla. Satırların kimlikleri dosyanın kentos_uids'i (sqlite3).")
    return {
        TAKEBACK: {**common, "command": "cad.gis.takeback",
                   "title": "GIS'ten geri al: türevde GIS'te değiştirilmiş satırlar kaynaklarına",
                   "note": note, "cases": takeback_cases()},
        DETACH: {**common, "command": "cad.gis.detach",
                 "title": "GIS türevinin bağını kopar: türev tablolar GIS'in kendi verisi",
                 "note": note, "cases": detach_cases()},
    }


def check_written():
    """The file a Rust test detached (KENTOS_WRITE_GPKG=1): the note in place of the record, no lasting ids, valid."""
    c = sqlite3.connect(f"file:{WRITTEN}?mode=ro", uri=True)
    docs = list(c.execute("SELECT r.table_name, m.md_standard_uri, m.metadata FROM gpkg_metadata m "
                          "JOIN gpkg_metadata_reference r ON r.md_file_id = m.id ORDER BY m.id"))
    notes = {t: json.loads(text) for t, uri, text in docs if uri == "https://kentos.app/derivation-detached/1"}
    records = {t for t, uri, _ in docs if uri == "https://kentos.app/derivation/1"}
    if "Parsel" not in notes or "Parsel" in records:
        sys.exit("ayrik.gpkg: Parsel's record is not a note")
    n = notes["Parsel"]
    if n["format"] != "kentos.derivation-detached" or n["version"] != 1 or n["reason"] != "Kadastro GIS'te sürecek":
        sys.exit(f"ayrik.gpkg: the note {n}")
    if n["derivation"]["table"] != "Parsel" or n["derivation"]["source"]["name"] != SOURCE:
        sys.exit("ayrik.gpkg: the note does not keep the record")
    if records != set(ORDER) - {"Parsel"}:
        sys.exit(f"ayrik.gpkg: the other tables' records {records}")
    left = c.execute("SELECT count(*) FROM kentos_uids WHERE table_name = 'Parsel'").fetchone()[0]
    kept = c.execute("SELECT count(*) FROM kentos_uids WHERE table_name = 'Yol'").fetchone()[0]
    if left != 0 or kept != 4:
        sys.exit(f"ayrik.gpkg: kentos_uids Parsel {left}, Yol {kept}")
    import subprocess
    gdal = Path("/usr/share/gdal/validate_gpkg.py")
    if gdal.exists():
        r = subprocess.run([sys.executable, str(gdal), str(WRITTEN)], capture_output=True, text=True)
        if r.returncode != 0:
            sys.exit(f"validate_gpkg: {r.stdout}{r.stderr}")
    print("ayrik.gpkg ok: Parsel's record a note with its reason, its ids left, the others linked")
    check_formats()
    check_arch()


def canonical_rings(geom):
    """A geometry's rings or lines as sets of rounded vertices, whatever way and from wherever they run."""
    out = []

    def flat(g):
        if g.GetGeometryCount() > 0:
            for i in range(g.GetGeometryCount()):
                flat(g.GetGeometryRef(i))
            return
        pts = [(round(g.GetX(i), 9), round(g.GetY(i), 9),
                round(g.GetZ(i), 9) if g.Is3D() and not math.isnan(g.GetZ(i)) else None)
               for i in range(g.GetPointCount())]
        if len(pts) > 1 and pts[0][:2] == pts[-1][:2]:
            pts = pts[:-1]
        out.append(tuple(sorted(pts, key=lambda p: (p[0], p[1]))))

    flat(geom)
    return sorted(out)


def gpkg_rows():
    """ada.gpkg's rows by lasting id: geometry rings, fields."""
    o = ogr()
    ds = o.Open(str(DIR / "ada.gpkg"))
    rows = {}
    for t in cg.TABLES:
        layer = ds.GetLayerByName(t["table"])
        for f in layer:
            rows[f.GetField("kentos_uid")] = (f.GetGeometryRef().Clone() if f.GetGeometryRef() else None,
                                             {k: f.GetField(k) for k in f.keys()})
    return rows


def flat_z(rings, zero):
    """The rings' Z as a container holds them: none dropped, a missing one 0 (`zero`) or left."""
    return [tuple((x, y, (0.0 if z is None else z) if zero else z) for x, y, z in r) for r in rings]


def check_formats():
    """KentOS's Shapefile zip and GeoJSON files hold ada.gpkg's rows: each by its lasting id, the same vertices (a
    Shapefile's missing Z as 0, GeoJSON without M), the same values under the index's column names."""
    import zipfile
    o = ogr()
    want = gpkg_rows()
    with zipfile.ZipFile(DIR / "ada.zip") as z:
        manifest = json.loads(z.read("kentos-turev.json"))
    if manifest["format"] != "kentos.derivation-set" or manifest["container"] != "shapefile":
        sys.exit(f"ada.zip: the index {manifest['format']} {manifest['container']}")
    seen = set()
    for info in manifest["tables"]:
        ds = o.Open(f"/vsizip/{DIR / 'ada.zip'}/{info['file']}")
        layer = ds.GetLayer(0)
        names = {c.get("file") or c["name"]: c["name"] for c in info["columns"]}
        zero = info.get("geometry", {}).get("z") == 3
        for f in layer:
            fields = {names.get(k, k): f.GetField(k) for k in f.keys()}
            u = fields["kentos_uid"]
            g, wf = want[u]
            seen.add(u)
            if g is not None:
                have = canonical_rings(f.GetGeometryRef())
                ref = flat_z(canonical_rings(g), zero)
                ref = [tuple(sorted(r, key=lambda p: (p[0], p[1]))) for r in ref]
                if sorted(ref) != have:
                    sys.exit(f"ada.zip {info['table']} {u}: vertices differ")
            for k, v in wf.items():
                if k in ("kentos_deviation",):
                    if abs(float(fields[k]) - v) > 1e-15:
                        sys.exit(f"ada.zip {u} {k}")
                elif isinstance(v, (int, float)) and fields.get(k) not in (None, ""):
                    # A number the .dbf could not hold to its 15 digits is written as text (ADR 0514).
                    if abs(float(fields[k]) - v) > 1e-12:
                        sys.exit(f"ada.zip {info['table']} {u} {k}: {fields.get(k)!r} ≠ {v!r}")
                elif fields.get(k) != v and not (v is None and fields.get(k) in (None, "")):
                    if str(fields.get(k)) != str(v):
                        sys.exit(f"ada.zip {info['table']} {u} {k}: {fields.get(k)!r} ≠ {v!r}")
        if info["record"]["rows"] != layer.GetFeatureCount():
            sys.exit(f"ada.zip {info['table']}: rows")
    if seen != set(want):
        sys.exit("ada.zip: not every row of ada.gpkg")
    for name, tables in (("ada-geojson.zip", None), ("ada-yol.geojson", {"Yol"})):
        if name.endswith(".zip"):
            with zipfile.ZipFile(DIR / name) as z:
                files = [t["file"] for t in json.loads(z.read("kentos-turev.json"))["tables"]]
            sources = [f"/vsizip/{DIR / name}/{f}" for f in files]
        else:
            sources = [str(DIR / name)]
        for src in sources:
            ds = o.Open(src)
            layer = ds.GetLayer(0)
            for f in layer:
                u = f.GetField("kentos_uid")
                g, wf = want[u]
                # OGR reads a position without Z beside ones with as 0: GeoJSON holds it as two numbers.
                if g is not None and canonical_rings(f.GetGeometryRef()) != flat_z(canonical_rings(g), True) \
                        and canonical_rings(f.GetGeometryRef()) != canonical_rings(g):
                    sys.exit(f"{name} {u}: vertices differ")
                for k, v in wf.items():
                    have = f.GetField(k)
                    if have != v and str(have) != str(v) and not (v is None and have in (None, "")):
                        if isinstance(v, float) and abs(float(have) - v) < 1e-15:
                            continue
                        sys.exit(f"{name} {u} {k}: {have!r} ≠ {v!r}")
    print("ada.zip, ada-geojson.zip, ada-yol.geojson ok: ada.gpkg's rows, vertices and values (OGR)")


def check_arch():
    """mimari.gpkg's rows as ADR 0634 §4 makes them from mimari.json, computed here: a straight wall's band, the
    space's area, the grid's path, the column's and the stair's and ramp's rectangles, the opening's jambs' footprint,
    the dimension's measured points and value; the arced wall left out."""
    o = ogr()
    ds = o.Open(str(DIR / "mimari.gpkg"))
    e = AENTS
    w = e[1]
    half = w["width"] / 2
    (x0, y0), (x1, y1) = (w["pts"][0]["x"], w["pts"][0]["y"]), (w["pts"][1]["x"], w["pts"][1]["y"])
    rect = lambda xa, ya, xb, yb: [(xa, ya), (xb, ya), (xb, yb), (xa, yb)]  # noqa: E731
    sp = e[3]["parts"][0]["outer"]["pts"]
    col = e[5]
    st, rp, op, dm = e[6], e[7], e[8], e[9]
    want = {
        "mimari_duvar": (rect(x0, y0 - half, x1, y1 + half), {"width": 0.2, "justify": "center", "durum": "mevcut"}),
        "mimari_mahal": ([(p["x"], p["y"]) for p in sp], {"number": "101", "name": "Salon"}),
        "mimari_aks": ([(e[4]["pts"][0]["x"], e[4]["pts"][0]["y"]), (e[4]["pts"][1]["x"], e[4]["pts"][1]["y"])],
                       {"name": "A"}),
        "mimari_kolon": (rect(col["at"]["x"] - col["width"] / 2, col["at"]["y"] - col["depth"] / 2,
                              col["at"]["x"] + col["width"] / 2, col["at"]["y"] + col["depth"] / 2),
                         {"type": "C30", "width": 0.4, "depth": 0.5}),
        "mimari_merdiven": (rect(st["pts"][0]["x"], st["pts"][0]["y"] - st["width"] / 2, st["pts"][1]["x"],
                                 st["pts"][1]["y"] + st["width"] / 2), {"risers": 16, "width": 1.2}),
        "mimari_rampa": (rect(rp["pts"][0]["x"], rp["pts"][0]["y"] - rp["width"] / 2, rp["pts"][1]["x"],
                              rp["pts"][1]["y"] + rp["width"] / 2), {"slope": 5.0, "width": 1.5}),
        "mimari_aciklik": ([(op["place"][i]["x"], op["place"][i]["y"]) for i in (0, 2, 3, 1)],
                           {"number": "B1", "host": op["host"], "width": 0.6}),
        "kentos_dimensions": ([(dm["a"]["x"], dm["a"]["y"]), (dm["b"]["x"], dm["b"]["y"])],
                              {"dim_type": "aligned", "value": math.dist((dm["a"]["x"], dm["a"]["y"]),
                                                                         (dm["b"]["x"], dm["b"]["y"])),
                               "unit": "length", "not": "cephe"}),
    }
    for table, (pts, fields) in want.items():
        layer = ds.GetLayerByName(table)
        if layer is None or layer.GetFeatureCount() != 1:
            sys.exit(f"mimari.gpkg: {table} is not one row")
        f = layer.GetFeature(1)
        have = canonical_rings(f.GetGeometryRef())
        ref = [tuple(sorted(((round(x, 9), round(y, 9), None) for x, y in pts), key=lambda p: (p[0], p[1])))]
        if have != ref:
            sys.exit(f"mimari.gpkg {table}: {have} ≠ {ref}")
        for k, v in fields.items():
            got = f.GetField(k)
            if (abs(got - v) > 1e-9) if isinstance(v, (int, float)) else got != v:
                sys.exit(f"mimari.gpkg {table} {k}: {got!r} ≠ {v!r}")
    if ds.GetLayerByName("mimari_duvar").GetFeatureCount() != 1:
        sys.exit("mimari.gpkg: the arced wall is not left out")
    print("mimari.gpkg ok: walls, spaces, grids, columns, stairs, ramps, openings and the dimension as computed (OGR)")


def main():
    if "--files" in sys.argv:
        make_file()
        return
    if "--written" in sys.argv:
        check_written()
        return
    check_file()
    out = {p: json.dumps(v, ensure_ascii=False, indent=1) + "\n" for p, v in files().items()}
    if "--check" in sys.argv:
        for p, text in out.items():
            if not p.exists() or p.read_text() != text:
                sys.exit(f"{p} is not what the script writes; run it without --check and read the difference")
        print(f"{len(takeback_cases())} takeback cases, {len(detach_cases())} detach cases, ada-geri.gpkg's edits ok")
        return
    for p, text in out.items():
        p.write_text(text)
        print(f"wrote {p}")


if __name__ == "__main__":
    main()
