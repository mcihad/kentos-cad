#!/usr/bin/env python3
"""The shared cases of the bound layers' commands cad.sources.describe, .edit and .sync (docs/adr/0503, plan/gis.md
CBS-01, CBS-02, CBS-11, CBS-36).

    /usr/bin/python3 scripts/fixtures/bound_sources_cases.py           # writes fixtures/commands/v1/cad.sources.*.json
    /usr/bin/python3 scripts/fixtures/bound_sources_cases.py --check   # compares
    python3 scripts/fixtures/bound_sources_cases.py --files            # GDAL writes komsu.gpkg (neighbours, ADR 0531)

A file's tables are read here with Python's sqlite3 from the GeoPackage tables themselves (gpkg_contents,
gpkg_geometry_columns, gpkg_spatial_ref_sys, PRAGMA table_info), not with KentOS code; a column's field kind is ADR
0502's (BOOLEAN boolean, TINYINT…INTEGER integer, FLOAT/DOUBLE/REAL decimal, DATE, DATETIME, BLOB none, else text). A
table's rows become the objects ADR 0502 says by gpkg_import_cases.py's reader (GDAL's OGR). A file's revision is its
SHA-256 by hashlib. How a layer differs from its table follows from how each case builds the layer: an object changed by
a later step differs, one deleted is missing, a row the layer lacks is added back by a reload. The layer's digest is
the SHA-256 of a canonical JSON KentOS writes; its value is not written here (`$any`), its meaning is: right after a
link, a write or a reload `edited` is false, after an edit true.
"""
import copy
import hashlib
import json
import sqlite3
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import gpkg_import_cases as gi  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1"
Q = "qgis.gpkg"
STYLE = gi.STYLE
ANY = "$any"


def fx(name):
    return f"$fixture:geopackage/v1/{name}"


def sha(name):
    return hashlib.sha256((DIR / name).read_bytes()).hexdigest()


def kind_of(decl):
    up = decl.strip().upper()
    base = up.split("(")[0].strip()
    return {"BOOLEAN": "boolean", "TINYINT": "integer", "SMALLINT": "integer", "MEDIUMINT": "integer", "INT": "integer",
            "INTEGER": "integer", "FLOAT": "decimal", "DOUBLE": "decimal", "REAL": "decimal", "DATE": "date",
            "DATETIME": "datetime", "BLOB": None}.get(base, "text")


def tables_of(name):
    """The file's feature and attribute tables as cad.sources.describe says them."""
    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    out = []
    for table, data, srs in c.execute("SELECT table_name, data_type, srs_id FROM gpkg_contents ORDER BY rowid"):
        if data not in ("features", "attributes"):
            continue
        g = c.execute("SELECT column_name, geometry_type_name, z, m FROM gpkg_geometry_columns WHERE table_name = ?",
                      (table,)).fetchone()
        srid = None
        if srs and srs > 0:
            r = c.execute("SELECT organization, organization_coordsys_id, definition FROM gpkg_spatial_ref_sys "
                          "WHERE srs_id = ?", (srs,)).fetchone()
            if r and r[0].upper() == "EPSG" and r[2] != "undefined":
                srid = r[1]
        info = c.execute(f'PRAGMA table_info("{table}")').fetchall()
        columns = []
        for _, cname, decl, _, _, pk in info:
            if pk or (g and cname == g[0]):
                continue
            col = {"name": cname, "declared": decl}
            k = kind_of(decl)
            if k:
                col["kind"] = k
            columns.append(col)
        t = {"name": table, "data": data, "rows": c.execute(f'SELECT COUNT(*) FROM "{table}"').fetchone()[0]}
        if g:
            t["geometry"] = g[1]
        if srid is not None:
            t["srid"] = srid
        t["columns"] = columns
        t["capabilities"] = {"read": True, "insert": True, "update": True, "delete": True, "transaction": True,
                             "z": bool(g and g[2]), "m": bool(g and g[3]), "curves": True}
        out.append(t)
    return out


def rows_of(name, table):
    return next(t for t in gi.read(name)["tables"] if t["name"] == table)["rows"]


def link(table, layer, n=1, name=Q, srid=5256):
    s = {"id": f"kaynak-{n}", "kind": "geopackage", "path": fx(name), "table": table, "layer": layer,
         "revision": sha(name), "digest": ANY}
    if srid is not None:
        s["srid"] = srid
    s["writable"] = True
    return s


def setup(layers=(), entities=(), sources=(), uids=None):
    doc = {
        "format": "kentos.document", "version": 1, "name": "Bağlı katmanlar",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis"},
        "origin": gi.P(0, 0),
        "layers": [
            {"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
             "style": STYLE, "children": []},
            {"id": "kilitli", "name": "Kilitli", "type": "layer", "visible": True, "locked": True, "expanded": True,
             "style": STYLE, "children": []},
            {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
             "style": STYLE, "children": []},
            *layers,
        ],
        "activeLayer": "cizim",
        "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": gi.P(487500, 4420500)},
                     *entities],
        "styles": {"items": [], "categories": []},
    }
    if sources:
        doc["settings"]["sources"] = list(sources)
    if uids:
        doc["uids"] = uids
    return doc


def layer(id_, locked=False):
    return {"id": id_, "name": id_.capitalize(), "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": STYLE, "children": []}


SETUP = setup()
SAME = {"revision": "same"}
POINT_UID = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7abc"


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def conflict(code, message, path):
    return {"status": "conflict", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, expect=None, op="execute", command=None):
    s = {"op": op, "input": inp, "result": result, "expect": expect or SAME}
    if command:
        s["command"] = command
    return s


def case(name, *steps, setup_=None):
    c = {"name": name, "steps": list(steps)}
    if setup_ is not None:
        c["setup"] = setup_
    return c


def other(command, inp, expect=None):
    """A step of another command whose answer is not this file's matter."""
    return step(inp, {"status": "completed", "output": ANY, "warnings": ANY}, expect or {"revision": "changed"},
                command=command)


# ── cad.sources.edit ───────────────────────────────────────────────────────

W = "wgs84.gpkg"
WB = link("yer", "import-yer", name=W, srid=4326)


def wgs_link():
    """The WGS 84 table bound with reproject: the point moved into TM36 (its coordinates the core's, PROJ's within a
    micrometre: crs_transform_cases.py), the table's system kept in the binding."""
    row = rows_of(W, "yer")[0]
    answer = {"link": WB, "objects": 1, "layers": [{"id": "import-yer", "name": "yer"}], "revision": "$current"}
    return step({"edit": {"kind": "link", "path": fx(W), "table": "yer", "reproject": True}}, done(answer),
                {"ids": [1, 2], "fields": {"2": {"kind": "point", "layerId": "import-yer", "attrs": row["attrs"]}},
                 "revision": "changed", "canUndo": True, "settings": {"sources": [WB]}}, command="cad.sources.edit")

def linked(table, name=Q, start=2, objects_layer=None):
    """`link` onto a new layer: the answer, the objects (their fields by slot) and the binding."""
    out, warnings, fields, ids, r = gi.imported(name, [table], start=start)
    layer_id = gi.slug(table)
    t = next(t for t in r["tables"] if t["name"] == table)
    srid = t["srid"]
    b = link(table, layer_id, name=name, srid=srid)
    if not any(x.get("geometry") for x in tables_of(name) if x["name"] == table):
        warnings = [w for w in warnings if w["code"] != "crs_undeclared"]
    answer = {"link": b, "objects": len(ids), "layers": [{"id": layer_id, "name": table}], "revision": "$current"}
    if not ids:
        answer["objects"] = 0
    return answer, warnings, fields, ids, b


def link_step(table, name=Q, start=2, cmd=None, more_ids=()):
    answer, warnings, fields, ids, b = linked(table, name, start)
    expect = {"ids": [1, *more_ids, *ids], "fields": fields, "revision": "changed", "canUndo": True,
              "settings": {"sources": [b]}}
    return step({"edit": {"kind": "link", "path": fx(name), "table": table}}, done(answer, warnings), expect,
                command=cmd), b, ids


def in_area(name, table, area):
    """The fids of a table's rows whose geometry's box meets `area` (sides included), by OGR's envelopes; a row without
    geometry is in no area (docs/adr/0510)."""
    from osgeo import ogr
    ds = ogr.Open(str(DIR / name))
    out = set()
    for f in ds.GetLayerByName(table):
        g = f.GetGeometryRef()
        if g is None or g.IsEmpty():
            continue
        x0, x1, y0, y1 = g.GetEnvelope()
        if x1 >= area[0] and x0 <= area[2] and y1 >= area[1] and y0 <= area[3]:
            out.add(f.GetFID())
    return out


AREA = [486990.0, 4419990.0, 487050.0, 4420030.0]


def area_link_step(table, area, name=Q, start=2, cmd=None):
    """`link` with an area onto a new layer: only the rows in the area, the area kept in the binding."""
    keep = {table: in_area(name, table, area)}
    out, warnings, fields, ids, r = gi.imported(name, [table], start=start, keep=keep)
    t = next(t for t in r["tables"] if t["name"] == table)
    b = {**link(table, gi.slug(table), name=name, srid=t["srid"]), "extent": area}
    b = {k: b[k] for k in ("id", "kind", "path", "table", "layer", "revision", "digest", "srid", "extent", "writable")}
    answer = {"link": b, "objects": len(ids), "layers": [{"id": gi.slug(table), "name": table}], "revision": "$current"}
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True, "settings": {"sources": [b]}}
    return step({"edit": {"kind": "link", "path": fx(name), "table": table, "extent": area}}, done(answer, warnings),
                expect, command=cmd), b, ids


def area_cases():
    a_link, _, ids = area_link_step("parsel", AREA)
    parsel = rows_of(Q, "parsel")
    inside = in_area(Q, "parsel", AREA)
    assert inside and len(inside) < len(parsel), "the area takes a part of the table"
    first = next(p for p in parsel if p["fid"] in inside)
    out = [
        case("alan sınırıyla bağlanır: yalnız alandaki satırlar, alan bağda", a_link),
        case("ters alan sınırı reddedilir",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "extent": [487050.0, 4419990.0, 486990.0, 4420030.0]}},
                  failed("invalid_extent", "Alan sınırının en küçüğü en büyüğünden küçük olmalı.", "edit.extent"))),
        case("var olan katmana alan sınırıyla bağlanmaz",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "layer": "cizim", "extent": AREA}},
                  failed("invalid_extent", "Var olan katmana bağlamada alan sınırı olmaz: katman tablonun bütün "
                         "nesnelerini taşır. Yeni katmana bağlayın (layer vermeden).", "edit.extent"))),
        case("geometrisiz tabloda alan sınırı olmaz",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "malik", "extent": AREA}},
                  failed("invalid_extent", "“malik” tablosunun geometrisi yok; alan sınırı geometrili tabloda olur.",
                         "edit.extent"))),
    ]
    sync_out = [
        case("alan sınırlı katmanın yazılacak bir şeyi yok: alanın dışındaki satırlar silinecek sayılmaz",
             area_link_step("parsel", AREA, cmd="cad.sources.edit")[0],
             step({"layer": "import-parsel", "direction": "write"}, done(synced()), op="plan",
                  command="cad.sources.sync")),
        case("alan sınırlı katmanda silinen nesne yalnız kendisi silinecek",
             area_link_step("parsel", AREA, cmd="cad.sources.edit")[0],
             other("cad.entities.delete", {"uids": [first["uid"]]}),
             step({"layer": "import-parsel", "direction": "write"}, done(synced(removed=1)), op="plan",
                  command="cad.sources.sync")),
        case("alanın dışına taşınan nesne yeni satır değil, kendi satırına yazılacak",
             area_link_step("parsel", AREA, cmd="cad.sources.edit")[0],
             other("cad.entities.transform", {"uids": [first["uid"]], "transform": {"kind": "move", "dx": 500, "dy": 0}}),
             step({"layer": "import-parsel", "direction": "write"}, done(synced(modified=1)), op="plan",
                  command="cad.sources.sync")),
    ]
    return out, sync_out


# ── neighbours outside the area (docs/adr/0531) ────────────────────────────

K = "komsu.gpkg"
# A parcel with a bump (A), its right neighbour sharing the bump's edges (B) and a triangle over the bump (C); only A's
# box meets the area, B and C lie outside it.
BUMP = (487012.0, 4420005.0)
K_ROWS = [
    ("A", [(487000, 4420000), (487010, 4420000), BUMP, (487010, 4420010), (487000, 4420010)]),
    ("B", [(487010, 4420000), (487020, 4420000), (487020, 4420010), (487010, 4420010), BUMP]),
    ("C", [(487010, 4420000), BUMP, (487010, 4420010)]),
]
K_AREA = [486990.0, 4419990.0, 487009.0, 4420030.0]


def write_files():
    """komsu.gpkg by GDAL's OGR: the three parcels in TUREF / TM33 (EPSG 5256)."""
    from osgeo import ogr, osr
    path = DIR / K
    if path.exists():
        path.unlink()
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
    lyr = ds.CreateLayer("parsel", srs=srs, geom_type=ogr.wkbPolygon)
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for ad, ring in K_ROWS:
        f = ogr.Feature(lyr.GetLayerDefn())
        wkt = "POLYGON ((" + ", ".join(f"{x} {y}" for x, y in [*ring, ring[0]]) + "))"
        f.SetGeometry(ogr.CreateGeometryFromWkt(wkt))
        f.SetField("ad", ad)
        lyr.CreateFeature(f)
    ds = None
    print(f"yazıldı: {K}")


def check_written():
    """kentos-komsu.gpkg (crates/native/application/tests/all/sources.rs with KENTOS_WRITE_GPKG) by GDAL's OGR: A's bump
    moved one metre east, B and C followed it, nothing else changed. None when it holds."""
    path = DIR / "kentos-komsu.gpkg"
    if not path.exists():
        return "kentos-komsu.gpkg yok: KENTOS_WRITE_GPKG=1 cargo test -p kentos-native-application --test all a_part_writes"
    from osgeo import ogr
    ds = ogr.Open(str(path))
    moved = (BUMP[0] + 1, BUMP[1])
    want = {ad: [moved if p == BUMP else (float(p[0]), float(p[1])) for p in ring] for ad, ring in K_ROWS}
    for f in ds.GetLayerByName("parsel"):
        r = f.GetGeometryRef().GetGeometryRef(0)
        got = [(r.GetX(i), r.GetY(i)) for i in range(r.GetPointCount() - 1)]
        ring = want.pop(f.GetField("ad"), None)
        n = len(got)
        if ring is None or n != len(ring) or not any(
                all(got[(i + k) % n] == ring[k] for k in range(n)) or all(got[(i - k) % n] == ring[k] for k in range(n))
                for i in range(n)):
            return f"kentos-komsu.gpkg'de {f.GetField('ad')} beklenen halka değil: {got}"
    return f"kentos-komsu.gpkg'de eksik satır: {sorted(want)}" if want else None


def follow(neighbours, change):
    """ADR 0160's rule on rings outside the area: a move takes every corner at its place; a removal takes the corner
    whose ring runs prev–at–next through it, and leaves a ring of fewer than three corners invalid. (edited, invalid)."""
    same = lambda p, q: abs(p[0] - q[0]) <= 1e-9 and abs(p[1] - q[1]) <= 1e-9
    edited = invalid = 0
    for ring in neighbours:
        n = len(ring)
        if change[0] == "move":
            if any(same(v, change[1]) for v in ring):
                edited += 1
            continue
        _, at, prev, nxt = change
        hit = [k for k in range(n) if same(ring[k], at)
               and ((same(ring[k - 1], prev) and same(ring[(k + 1) % n], nxt))
                    or (same(ring[k - 1], nxt) and same(ring[(k + 1) % n], prev)))]
        if hit:
            if n - 1 < 3:
                invalid += 1
            else:
                edited += 1
    return edited, invalid


def neighbour_cases():
    """The write's topology option on a part (docs/adr/0531)."""
    k_link, _, _ = area_link_step("parsel", K_AREA, name=K, cmd="cad.sources.edit")
    rows = rows_of(K, "parsel")
    inside = in_area(K, "parsel", K_AREA)
    a = next(r for r in rows if r["fid"] in inside)
    outside = [[(p["x"], p["y"]) for p in r["geometry"]["pts"]] for r in rows if r["fid"] not in inside]
    assert len(outside) == 2, "B and C lie outside the area"
    pts = [(p["x"], p["y"]) for p in a["geometry"]["pts"]]
    k = pts.index(BUMP)
    moved = follow(outside, ("move", BUMP))
    removed = follow(outside, ("remove", BUMP, pts[k - 1], pts[(k + 1) % len(pts)]))
    assert moved == (2, 0) and removed == (1, 1), (moved, removed)

    def out(followed=0, left=0, modified=1):
        o = synced(modified=modified, name=K)
        if followed:
            o["followed"] = followed
        if left:
            o["left"] = left
        return o

    stretch = other("cad.entities.stretch", {"window": {"min": gi.P(BUMP[0] - 0.5, BUMP[1] - 0.5),
                                                        "max": gi.P(BUMP[0] + 0.5, BUMP[1] + 0.5)},
                                             "dx": 1, "dy": 0, "uids": [a["uid"]]})
    remove = other("cad.entities.vertex", {"uid": a["uid"], "action": "remove",
                                           "place": {"kind": "vertex", "part": 0, "ring": 0, "index": k}})
    left_warning = {"code": "neighbours_left", "message": f"Alanın dışındaki {removed[1]} komşu kayıt ortak köşeyi "
                    "izleyince geçersiz kalacağı için olduğu gibi kaldı.", "path": "topology"}

    def sync(topology=None, **kw):
        i = {"layer": "import-parsel", "direction": kw.pop("direction", "write")}
        if topology:
            i["topology"] = topology
        return i

    return [
        case("alanın dışındaki komşular izlenmeden yazılırsa yalnız değişen nesne yazılacak",
             k_link, stretch, step(sync(), done(out()), op="plan", command="cad.sources.sync")),
        case("strict: ortak köşesi taşınan alanın dışındaki iki komşu da yazılacak",
             k_link, stretch, step(sync("strict"), done(out(followed=moved[0])), op="plan",
                                   command="cad.sources.sync")),
        case("loose: ortak köşesi taşınan alanın dışındaki iki komşu da yazılacak",
             k_link, stretch, step(sync("loose"), done(out(followed=moved[0])), op="plan",
                                   command="cad.sources.sync")),
        case("loose: köşe silinince geçersiz kalacak üçgen olduğu gibi kalır, söylenir",
             k_link, remove, step(sync("loose"), done(out(followed=removed[0], left=removed[1]), [left_warning]),
                                  op="plan", command="cad.sources.sync")),
        case("strict: köşe silinince geçersiz kalacak komşu varsa hiçbir şey yazılmaz",
             k_link, remove, step(sync("strict"), failed(
                 "outside_neighbour_invalid", f"Alanın dışındaki {removed[1]} komşu kayıt ortak köşeyi izleyince "
                 "geçersiz kalırdı; hiçbir şey yazılmadı. Alanı genişletip komşuları da çalışma kopyasına alın ya da "
                 "gevşek kipte (loose) yazın.", "topology"), op="plan", command="cad.sources.sync")),
        case("strict: hiçbir şey değişmediyse izlenen komşu da yok",
             k_link, step(sync("strict"), done(out(modified=0)), op="plan", command="cad.sources.sync")),
        case("okumada komşuların izlenmesi istenmez",
             k_link, step(sync("strict", direction="read"), failed(
                 "topology_on_read", "Komşuların izlenmesi (topology) yalnız yazarken olur; okumada tablonun satırları "
                 "olduğu gibi gelir.", "topology"), op="plan", command="cad.sources.sync")),
        case("bütün tablo bağlıyken alanın dışında komşu yoktur",
             link_step("parsel", name=K, cmd="cad.sources.edit")[0], stretch,
             step(sync("strict"), done(out()), op="plan", command="cad.sources.sync")),
    ]


def edit_cases():
    parsel = rows_of(Q, "parsel")
    s_link, b, _ = link_step("parsel")
    n_link, nb, _ = link_step("nokta")
    m_link, mb, _ = link_step("malik")
    empty_answer = {"link": link("yer", "import-yer", name="bos.gpkg", srid=None), "objects": 0,
                    "layers": [{"id": "import-yer", "name": "yer"}], "revision": "$current"}
    _, lw, _, _, _ = gi.imported("sistemsiz.gpkg", ["yer"])
    loose_answer, loose_w, loose_f, loose_ids, loose_b = linked("yer", "sistemsiz.gpkg")
    return [
        case("parsel tablosu yeni katmana bağlanır: satırları kalıcı kimlikleriyle, bağ dosyanın SHA-256'sıyla", s_link),
        case("Z'li noktalar bağlanır", n_link),
        case("geometrisiz öznitelik tablosu kayıtlarıyla bağlanır", m_link),
        case("sistem bildirmeyen dosya bağlanır, srid yazılmaz",
             step({"edit": {"kind": "link", "path": fx("sistemsiz.gpkg"), "table": "yer"}}, done(loose_answer, loose_w),
                  {"ids": [1, *loose_ids], "fields": loose_f, "revision": "changed", "canUndo": True,
                   "settings": {"sources": [loose_b]}})),
        case("boş tablo boş bir katmana bağlanır",
             step({"edit": {"kind": "link", "path": fx("bos.gpkg"), "table": "yer"}},
                  done(empty_answer, gi.imported("bos.gpkg", ["yer"])[1] or [
                      {"code": "crs_undeclared", "message": "bos.gpkg koordinat sistemi bildirmiyor; koordinatlar "
                       "projenin sisteminde sayılıp olduğu gibi alındı.", "path": "path"}]),
                  {"ids": [1], "revision": "changed", "settings": {"sources": [empty_answer["link"]]}})),
        case("WGS 84'teki tablo çevirme istenmeden bağlanmaz",
             step({"edit": {"kind": "link", "path": fx("wgs84.gpkg"), "table": "yer"}},
                  failed("crs_mismatch", "“yer” tablosu EPSG:4326 sisteminde, proje EPSG:5256. Katmanın nesneleri "
                         "projenin sisteminde, tablonunkiler kendi sisteminde tutulsun diye reproject verin ya da "
                         "projeyi EPSG:4326'de açın.", "edit.reproject"))),
        case("WGS 84'teki tablo reproject ile bağlanır: katman projenin, tablo kendi sisteminde", wgs_link()),
        case("aynı tablo ikinci kez bağlanmaz", s_link,
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel"}},
                  failed("source_bound", "“parsel” tablosu zaten “import-parsel” katmanına bağlı (kaynak-1).",
                         "edit.table"))),
        case("içe aktarılmış katman olduğu gibi bağlanır",
             other("file.import.gpkg", {"path": fx(Q), "tables": ["parsel"]}),
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "layer": "import-parsel"}},
                  done({"link": link("parsel", "import-parsel"), "objects": 0, "revision": "$current"}),
                  {"revision": "changed", "settings": {"sources": [link("parsel", "import-parsel")]}})),
        case("başka nesneler taşıyan katman bağlanmaz",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "layer": "cizim"}},
                  failed("source_differs", "“cizim” katmanı “parsel” tablosunun nesnelerini taşımıyor: tabloda olmayan "
                         "0, farklı 0, katmanda olmayan 3 nesne. Tabloyu yeni bir katmana bağlayın (layer vermeden).",
                         "edit.layer"))),
        case("olmayan katman",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "layer": "yok"}},
                  failed("layer_not_found", "“yok” kimlikli katman çizimde yok; var olan bir katmanın kimliğini verin.",
                         "edit.layer"))),
        case("grup bağlanmaz",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "layer": "grup"}},
                  failed("not_a_layer", "“Grup” bir grup; yalnız katman bağlanır.", "edit.layer"))),
        case("kilitli katman bağlanmaz",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "layer": "kilitli"}},
                  failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın.", "edit.layer"))),
        case("bir katman tek bağa", s_link,
             step({"edit": {"kind": "link", "path": fx(Q), "table": "yol", "layer": "import-parsel"}},
                  failed("source_bound", "“import-parsel” katmanı zaten “parsel” tablosuna bağlı (kaynak-1).",
                         "edit.layer"))),
        case("dosyada olmayan tablo",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "bina"}},
                  failed("table_not_found", "“bina” tablosu dosyada yok; dosyanın tablolarından birinin adını verin.",
                         "edit.table"))),
        case("GeoPackage olmayan dosya",
             step({"edit": {"kind": "link", "path": fx("bozuk.gpkg"), "table": "parsel"}},
                  failed("import_unreadable", f"{fx('bozuk.gpkg')} GeoPackage olarak okunamadı: “{fx('bozuk.gpkg')}” "
                         "bir SQLite dosyası değil", "edit.table"))),
        case("göreli yol",
             step({"edit": {"kind": "link", "path": "veri/kadastro.gpkg", "table": "parsel"}},
                  failed("invalid_file_path", "“veri/kadastro.gpkg” mutlak bir dosya yolu değil; GeoPackage dosyasının "
                         "tam yolunu verin (/klasör/ad.gpkg).", "edit.path"))),
        case("olmayan dosya",
             step({"edit": {"kind": "link", "path": fx("yok.gpkg"), "table": "parsel"}},
                  failed("file_not_found", f"{fx('yok.gpkg')} yok; dosyanın yolunu denetleyin.", "edit.path"))),
        case("içe aktarılmış satırlar ikinci kez alınmaz",
             other("file.import.gpkg", {"path": fx(Q), "tables": ["parsel"]}),
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel"}},
                  failed("entity_exists", f"Dosyadaki bir kaydın kalıcı kimliği ({parsel[0]['uid']}) çizimde zaten bir "
                         f"nesnenin: {Q} daha önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.",
                         "path"))),
        case("bağ çözülür, nesneler kalır", s_link,
             step({"edit": {"kind": "unlink", "id": "kaynak-1"}},
                  done({"objects": 0, "revision": "$current"}),
                  {"ids": [1, 2, 3, 4], "revision": "changed", "settings": {"sources": None}})),
        case("olmayan bağ çözülmez",
             step({"edit": {"kind": "unlink", "id": "kaynak-9"}},
                  failed("source_not_found", "“kaynak-9” kimlikli bir bağ yok; cad.sources.describe bağları listeler.",
                         "edit.id"))),
        case("taşınan dosya yeni yeriyle bağlanır", s_link,
             step({"edit": {"kind": "relink", "id": "kaynak-1", "path": "$fixture:geopackage/v1/../v1/qgis.gpkg"}},
                  done({"link": {**b, "path": "$fixture:geopackage/v1/../v1/qgis.gpkg"}, "objects": 0,
                        "revision": "$current"}),
                  {"revision": "changed",
                   "settings": {"sources": [{**b, "path": "$fixture:geopackage/v1/../v1/qgis.gpkg"}]}})),
        case("başka dosya yeniden bağlanmaz", s_link,
             step({"edit": {"kind": "relink", "id": "kaynak-1", "path": fx("kentos.gpkg")}},
                  conflict("source_changed", f"{fx('kentos.gpkg')} katmanın son eşleştiği dosya değil (içeriği farklı). "
                           "Aynı dosyanın yeni yerini verin; başka bir dosyayı bağlamak için bağı çözüp yeniden "
                           "bağlayın.", "edit.path"))),
        case("olmayan bağ yeniden bağlanmaz",
             step({"edit": {"kind": "relink", "id": "kaynak-2", "path": fx(Q)}},
                  failed("source_not_found", "“kaynak-2” kimlikli bir bağ yok; cad.sources.describe bağları listeler.",
                         "edit.id"))),
        *[case(f"PostGIS: {n}", step({"edit": {"kind": "link", "path": a, "table": "kadastro.parsel"}},
                                     bad_address(a, why, "edit.path"))) for n, a, why in BAD_ADDRESSES],
        case("PostGIS: dosyanın bağı veritabanına yeniden bağlanmaz", s_link,
             step({"edit": {"kind": "relink", "id": "kaynak-1", "path": PG}},
                  failed("invalid_address", f"{PG} bağın kaynağıyla aynı türden değil (dosya ya da veritabanı); başka "
                         "türden bir kaynak için bağı çözüp yeniden bağlayın.", "edit.path"))),
        case("PostGIS: yeniden bağlamada da adreste parola olmaz", s_link,
             step({"edit": {"kind": "relink", "id": "kaynak-1", "path": BAD_ADDRESSES[0][1]}},
                  bad_address(BAD_ADDRESSES[0][1], PASSWORD, "edit.path"))),
        case("PostGIS: veritabanının bağı veritabanına ulaşmadan çözülür",
             step({"edit": {"kind": "unlink", "id": "kaynak-1"}}, done({"objects": 0, "revision": "$current"}),
                  {"revision": "changed", "settings": {"sources": None}}),
             setup_=setup([layer("bagli")], sources=[{"id": "kaynak-1", "kind": "postgis", "path": PG,
                                                      "table": "kadastro.parsel", "layer": "bagli", "revision": FAKE,
                                                      "digest": FAKE, "srid": 5256, "writable": True}])),
        case("iki tablo iki bağ", s_link,
             step({"edit": {"kind": "link", "path": fx(Q), "table": "nokta"}},
                  done({**linked("nokta", start=5)[0], "link": {**nb, "id": "kaynak-2"}},
                       linked("nokta", start=5)[1]),
                  {"revision": "changed", "settings": {"sources": [b, {**nb, "id": "kaynak-2"}]}})),
        case("plan hiçbir şey yazmaz",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel"}}, done(linked("parsel")[0],
                                                                                    linked("parsel")[1]),
                  {"ids": [1], "revision": "same", "canUndo": False, "dirty": False}, op="plan")),
        case("validate yalnız denetler",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel"}}, done(None), op="validate")),
    ]


# ── cad.sources.describe ───────────────────────────────────────────────────

def state(b, found=True, changed=False, edited=False, added=0, modified=0, removed=0):
    return {"link": b, "found": found, "changed": changed, "edited": edited, "added": added, "modified": modified,
            "removed": removed}


FAKE = "f" * 64


PG = "postgresql://cbs@veri.belediye.local:5432/kent"
PASSWORD = "adreste parola var; parola projeye yazılmaz: PGPASSWORD ortam değişkenine ya da ~/.pgpass dosyasına koyun"
FORM = "(postgresql://kullanıcı@sunucu:5432/veritabanı)"
# docs/adr/0505: a database's address is refused before anything is reached when it carries a password or lacks a host
# or a database. Whether a database answers depends on the host's provider (Python registers one, the application's
# tests do not), so those refusals are tested in sources.rs and kentos-postgis's tests, not here.
BAD_ADDRESSES = [
    ("veritabanı adresinde parola", "postgresql://cbs:gizli@veri.belediye.local/kent", PASSWORD),
    ("adresin password parametresi", "postgresql://cbs@veri.belediye.local/kent?sslmode=require&password=gizli", PASSWORD),
    ("adreste sunucu yok", "postgresql://cbs@/kent", f"adreste sunucu yok {FORM}"),
    ("adreste veritabanı yok", "postgres://cbs@veri.belediye.local:5432/", f"adreste veritabanı yok {FORM}"),
]


def bad_address(address, why, at):
    return failed("invalid_address", f"“{address}”: {why}.", at)


def describe_cases():
    s_link, b, ids = link_step("parsel", cmd="cad.sources.edit")
    n_link, nb, _ = link_step("nokta", cmd="cad.sources.edit")
    parsel = rows_of(Q, "parsel")
    files = [Q, "kentos.gpkg", "kentos-export.gpkg", "kentos-edited.gpkg", "wgs84.gpkg", "sistemsiz.gpkg", "bos.gpkg"]
    cs = [case(f"{n} dosyasının tabloları",
               step({"path": fx(n)}, done({"tables": tables_of(n), "revision": "$current"})))
          for n in files]
    bound = layer("bagli")
    cs += [
        case("GeoPackage olmayan dosya",
             step({"path": fx("bozuk.gpkg")},
                  failed("import_unreadable", f"{fx('bozuk.gpkg')} GeoPackage olarak okunamadı: “{fx('bozuk.gpkg')}” "
                         "bir SQLite dosyası değil", "path"))),
        case("göreli yol",
             step({"path": "veri/kadastro.gpkg"},
                  failed("invalid_file_path", "“veri/kadastro.gpkg” mutlak bir dosya yolu değil; GeoPackage dosyasının "
                         "tam yolunu verin (/klasör/ad.gpkg).", "path"))),
        case("klasör",
             step({"path": "$fixture:geopackage/v1"},
                  failed("invalid_file_path", "$fixture:geopackage/v1 bir klasör; GeoPackage dosyasının adını da verin.",
                         "path"))),
        case("olmayan dosya",
             step({"path": fx("yok.gpkg")},
                  failed("file_not_found", f"{fx('yok.gpkg')} yok; dosyanın yolunu denetleyin.", "path"))),
        case("plan aynısını söyler", step({"path": fx(Q)}, done({"tables": tables_of(Q), "revision": "$current"}),
                                          op="plan")),
        case("validate yalnız denetler", step({"path": fx(Q)}, done(None), op="validate")),
        case("bağ yokken liste boş", step({}, done({"revision": "$current"}))),
        case("bağlanan katman tablosuyla aynı", s_link, step({}, done({"links": [state(b)], "revision": "$current"}))),
        case("değişen nesne: katman değişti, bir nesne farklı", s_link,
             other("cad.entities.set", {"uids": [parsel[0]["uid"]], "attrs": {"ada": "Yeni Mahalle"}, "operation": "attributes"}),
             step({}, done({"links": [state(b, edited=True, modified=1)], "revision": "$current"}))),
        case("silinen nesne tabloda kalır", s_link,
             other("cad.entities.delete", {"uids": [parsel[1]["uid"]]}),
             step({}, done({"links": [state(b, edited=True, removed=1)], "revision": "$current"}))),
        case("katmana eklenen nokta tabloda yok", n_link,
             other("cad.point.create", {"layerId": "import-nokta", "p": gi.P(487600, 4420600)}),
             step({}, done({"links": [state(nb, edited=True, added=1)], "revision": "$current"}))),
        case("alanı olmayan öznitelik katmanı değiştirir ama satırı değiştirmez", s_link,
             other("cad.entities.set", {"uids": [parsel[0]["uid"]], "attrs": {"durum": "x"}, "operation": "attributes"}),
             step({}, done({"links": [state(b, edited=True)], "revision": "$current"}))),
        case("dosya değişmiş, katman boş: bütün satırlar katmanda yok",
             step({}, done({"links": [state({**link("parsel", "bagli"), "revision": FAKE, "digest": FAKE},
                                            changed=True, edited=True, removed=3)], "revision": "$current"})),
             setup_=setup([bound], sources=[{**link("parsel", "bagli"), "revision": FAKE, "digest": FAKE}])),
        case("dosya yerinde değil",
             step({}, done({"links": [state({**link("parsel", "bagli"), "path": fx("tasindi.gpkg"), "digest": FAKE},
                                            found=False, edited=True)], "revision": "$current"})),
             setup_=setup([bound], sources=[{**link("parsel", "bagli"), "path": fx("tasindi.gpkg"), "digest": FAKE}])),
        case("tablo dosyada yok",
             step({}, done({"links": [state({**link("bina", "bagli"), "digest": FAKE}, edited=True)],
                            "revision": "$current"})),
             setup_=setup([bound], sources=[{**link("bina", "bagli"), "digest": FAKE}])),
        case("tablonun tutmadığı nesne sayılmaz",
             step({}, done({"links": [state({**link("parsel", "bagli"), "digest": FAKE}, edited=True, removed=3)],
                            "revision": "$current"})),
             setup_=setup([bound], [{"kind": "point", "id": 2, "layerId": "bagli", "attrs": {}, "p": gi.P(1, 2)}],
                          [{**link("parsel", "bagli"), "digest": FAKE}])),
        case("başka sistemdeki tablo: gidip gelen koordinatlar aynı sayılır", wgs_link(),
             step({}, done({"links": [state(WB)], "revision": "$current"}))),
        case("başka sistemdeki tablo: taşınan nokta farklı", wgs_link(),
             other("cad.entities.transform", {"uids": ["$uidOf:2"], "transform": {"kind": "move", "dx": 10, "dy": 0}}),
             step({}, done({"links": [state(WB, edited=True, modified=1)], "revision": "$current"}))),
        *[case(f"PostGIS: {n}", step({"path": a}, bad_address(a, why, "path"))) for n, a, why in BAD_ADDRESSES],
        case("iki bağ sırasıyla", s_link, n_link,
             step({}, done({"links": [state(b), state({**nb, "id": "kaynak-2"})], "revision": "$current"}))),
    ]
    # The second link's objects come after the first's.
    cs[-1]["steps"] = copy.deepcopy(cs[-1]["steps"])
    second = cs[-1]["steps"][1]
    second["expect"] = {"revision": "changed", "settings": {"sources": [b, {**nb, "id": "kaynak-2"}]}}
    second["result"] = done({**linked("nokta", start=5)[0], "link": {**nb, "id": "kaynak-2"}},
                            linked("nokta", start=5)[1])
    return cs


# ── cad.sources.sync ───────────────────────────────────────────────────────

def synced(added=0, modified=0, removed=0, columns=(), name=Q, pins=None):
    out = {"added": added, "modified": modified, "removed": removed, "source": sha(name), "revision": "$current"}
    if columns:
        out["columns"] = list(columns)
    if pins is not None:
        out["pins"] = pins
    return out


def sync_cases():
    s_link, b, ids = link_step("parsel", cmd="cad.sources.edit")
    parsel = rows_of(Q, "parsel")
    _, _, fields, _, _ = gi.imported(Q, ["parsel"], start=2)
    bound = layer("bagli")
    good = {**link("parsel", "bagli"), "digest": FAKE}
    w = {"layer": "import-parsel", "direction": "write"}
    r = {"layer": "import-parsel", "direction": "read"}
    point = {"kind": "point", "id": 2, "layerId": "bagli", "attrs": {}, "p": gi.P(487000, 4420000)}
    read_fields = {str(k): {**v, "layerId": "bagli"} for k, v in gi.imported(Q, ["parsel"], start=2)[2].items()}
    read_fields3 = {str(int(k) + 1): v for k, v in read_fields.items()}
    return [
        case("bağlanan katmanın yazılacak bir şeyi yok", s_link, step(w, done(synced()), op="plan")),
        case("değişen nesne yazılacak", s_link,
             other("cad.entities.set", {"uids": [parsel[0]["uid"]], "attrs": {"ada": "Yeni Mahalle"}, "operation": "attributes"}),
             step(w, done(synced(modified=1)), op="plan")),
        case("silinen nesne tablodan silinecek", s_link,
             other("cad.entities.delete", {"uids": [parsel[2]["uid"]]}),
             step(w, done(synced(removed=1)), op="plan")),
        case("alanı olmayan öznitelik yazılmaz, söylenir", s_link,
             other("cad.entities.set", {"uids": [parsel[0]["uid"]], "attrs": {"durum": "x"}, "operation": "attributes"}),
             step(w, done(synced(), [{"code": "export_losses", "message": "parsel: katmanın alanı olmayan öznitelikler "
                                      "yazılmadı (durum); Alanlar penceresinden alan yapın.", "path": "layer"}]),
                  op="plan")),
        case("noktası tablonun alan sütununa uymaz",
             {"op": "captureUid", "id": 2, "as": "nokta"},
             step({"layer": "bagli", "direction": "write"},
                  failed("geometry_mismatch", "$uid:nokta kimlikli nesne tabloya yazılamaz: POINT geometrisi "
                         "MULTIPOLYGON sütununa yazılamaz.", "layer"), op="plan"),
             setup_=setup([bound], [point], [good])),
        case("dosya değiştiyse yazılmaz",
             step({"layer": "bagli", "direction": "write"},
                  conflict("source_changed", f"{fx(Q)} katman onunla son eşleştiğinden beri değişti; üzerine yazılmadı. "
                           "Önce tablodan okuyun (direction read) ya da dosyanın kopyasını bağlayın.", "layer"),
                  op="plan"),
             setup_=setup([bound], sources=[{**good, "revision": FAKE}])),
        case("salt okunur tabloya yazılmaz",
             step({"layer": "bagli", "direction": "write"},
                  failed("source_readonly", f"{fx(Q)} dosyasının “parsel” tablosuna yazılamaz (salt okunur).", "layer"),
                  op="plan"),
             setup_=setup([bound], sources=[{**good, "writable": False}])),
        case("bağlı olmayan katman",
             step({"layer": "cizim", "direction": "write"},
                  failed("source_not_found", "“cizim” katmanı bir kaynağa bağlı değil; önce cad.sources.edit ile "
                         "bağlayın.", "layer"))),
        case("bağlı katman çizimde yok",
             step({"layer": "silindi", "direction": "read"},
                  failed("layer_not_found", "Bağlı “silindi” katmanı çizimde yok; bağı çözün (cad.sources.edit "
                         "unlink).", "layer")),
             setup_=setup(sources=[{**good, "layer": "silindi"}])),
        case("dosya yerinde yok",
             step({"layer": "bagli", "direction": "read"},
                  failed("source_gone", f"{fx('tasindi.gpkg')} yerinde yok; dosyanın yeni yerini cad.sources.edit "
                         "relink ile verin.", "layer")),
             setup_=setup([bound], sources=[{**good, "path": fx("tasindi.gpkg")}])),
        case("tablo dosyada yok",
             step({"layer": "bagli", "direction": "read"},
                  failed("table_not_found", "“bina” tablosu dosyada yok; dosyanın tablolarından birinin adını verin.",
                         "layer")),
             setup_=setup([bound], sources=[{**good, "table": "bina"}])),
        case("GeoPackage olmayan dosya",
             step({"layer": "bagli", "direction": "read"},
                  failed("import_unreadable", f"{fx('bozuk.gpkg')} GeoPackage olarak okunamadı: “{fx('bozuk.gpkg')}” "
                         "bir SQLite dosyası değil", "layer")),
             setup_=setup([bound], sources=[{**good, "path": fx("bozuk.gpkg"), "revision": sha("bozuk.gpkg")}])),
        case("kilitli katman okunmaz",
             step({"layer": "kilitli", "direction": "read", "discard": True},
                  failed("layer_locked", "“kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın.", "layer")),
             setup_=setup(sources=[{**good, "layer": "kilitli"}])),
        case("değişen katman discard olmadan okunmaz", s_link,
             other("cad.entities.set", {"uids": [parsel[0]["uid"]], "attrs": {"ada": "Yeni Mahalle"}, "operation": "attributes"}),
             step(r, conflict("pending_changes", "“import-parsel” katmanı dosyayla son eşleştiğinden beri değişti; "
                              "okumak bu değişiklikleri götürür. Önce yazın (direction write) ya da discard ile tablonun "
                              "hâlini alın.", "layer"))),
        case("discard ile tablonun değeri geri gelir", s_link,
             other("cad.entities.set", {"uids": [parsel[0]["uid"]], "attrs": {"ada": "Yeni Mahalle"}, "operation": "attributes"}),
             step({**r, "discard": True}, done(synced(modified=1)),
                  {"ids": [1, 2, 3, 4], "fields": {"2": {"attrs": fields["2"]["attrs"]}}, "revision": "changed",
                   "canUndo": True})),
        case("silinen nesne okununca geri gelir", s_link,
             other("cad.entities.delete", {"uids": [parsel[2]["uid"]]}),
             step({**r, "discard": True}, done(synced(added=1)),
                  {"ids": [1, 2, 3, 5], "fields": {"5": {**fields["4"]}}, "revision": "changed", "canUndo": True})),
        case("eşleşen katman okununca değişmez", s_link, step(r, done(synced()), {"ids": [1, 2, 3, 4],
                                                                               "revision": "same"})),
        case("boş katman tablonun satırlarını alır",
             step({"layer": "bagli", "direction": "read", "discard": True}, done(synced(added=3)),
                  {"ids": [1, 2, 3, 4], "fields": read_fields, "revision": "changed", "canUndo": True}),
             setup_=setup([bound], sources=[{**good, "revision": FAKE}])),
        case("tabloda olmayan nesne okununca silinir",
             step({"layer": "bagli", "direction": "read", "discard": True}, done(synced(added=3, removed=1)),
                  {"ids": [1, 3, 4, 5], "fields": read_fields3, "revision": "changed", "canUndo": True}),
             setup_=setup([bound], [point], [good])),
        case("plan okunacakları söyler, yazmaz",
             step({"layer": "bagli", "direction": "read", "discard": True}, done(synced(added=3)), op="plan"),
             setup_=setup([bound], sources=[good])),
        case("başka sistemdeki tablo: bağlanınca yazılacak bir şey yok", wgs_link(),
             step({"layer": "import-yer", "direction": "write"}, done(synced(name=W)), op="plan")),
        case("başka sistemdeki tablo: taşınan nokta tablonun sistemine çevrilip yazılacak", wgs_link(),
             other("cad.entities.transform", {"uids": ["$uidOf:2"], "transform": {"kind": "move", "dx": 10, "dy": 0}}),
             step({"layer": "import-yer", "direction": "write"}, done(synced(modified=1, name=W)), op="plan")),
        case("başka sistemdeki tablo: okunca nokta yerine döner", wgs_link(),
             other("cad.entities.transform", {"uids": ["$uidOf:2"], "transform": {"kind": "move", "dx": 10, "dy": 0}}),
             step({"layer": "import-yer", "direction": "read", "discard": True}, done(synced(modified=1, name=W)),
                  {"ids": [1, 2], "revision": "changed", "canUndo": True})),
        case("validate yalnız denetler", s_link, step(w, done(None), op="validate")),
    ]



# ── Label pins of bound layers (docs/adr/0533 §1, §2): the key, the area change, what a reload does to the pins ──

KEY = "ada"
PIN_A = {"at": {"x": 1.0, "y": 2.0}, "rotation": 10.0}
PIN_B = {"at": {"x": -3.0, "y": 4.0}}
LOST_WORDS = [("deleted", "satırı tabloda yok"), ("outside", "satırı katmanın alanının dışında"),
              ("rebound", "satır kimliği artık başka bir kayıt (anahtar değeri değişmiş)"),
              ("taken", "anahtarının satırında zaten iğneler var")]


def pins_after(bearers, rows, key, outside=()):
    """What a reload does to the pins of a layer's objects, by the rule of the ADR (§1). `bearers`: the objects that
    hold pins, as {uid, value (the key's value they hold now, None without), pins}; `rows`: the table's rows as read, by
    uid, each its key's value (None when it has none); `key`: the binding's key or None; `outside`: uids of rows the
    layer's area leaves out. Returns the report and which pins each object holds after (the objects named only)."""
    by_value = {}
    for uid, value in rows.items():
        if key and value:
            by_value.setdefault(value, []).append(uid)
    kept, moves, lost = 0, [], []
    for b in bearers:
        here = b["uid"] in rows
        if here and (not key or b["value"] is None or rows[b["uid"]] == b["value"]):
            kept += 1
            continue
        target = by_value.get(b["value"]) if b["value"] is not None else None
        if target and len(target) == 1 and target[0] != b["uid"]:
            moves.append((b, target[0]))
        else:
            reason = "rebound" if here else ("outside" if b["uid"] in outside else "deleted")
            lost.append((b["uid"], reason))
    leaving = {b["uid"] for b, _ in moves}
    staying = {b["uid"] for b in bearers} - leaving
    moved, after = [], {}
    for b, to in moves:
        if to in staying and to in {x["uid"] for x in bearers}:
            lost.append((b["uid"], "taken"))
        else:
            after[to] = b["pins"]
            moved.append({"from": b["uid"], "to": to, "key": b["value"]})
    for uid in list(leaving) + [u for u, _ in lost]:
        if uid in rows:
            after.setdefault(uid, [])
    moved.sort(key=lambda m: m["from"])
    lost.sort()
    return {"kept": kept, **({"moved": moved} if moved else {}),
            **({"lost": [{"uid": u, "reason": r} for u, r in lost]} if lost else {})}, after


def pin_warnings(table, key, report):
    """The warnings a pins report comes with: lost pins, moved pins."""
    out = []
    lost = report.get("lost", [])
    if lost:
        parts = [f"{sum(1 for l in lost if l['reason'] == r)} nesnenin {w}" for r, w in LOST_WORDS
                 if any(l["reason"] == r for l in lost)]
        out.append({"code": "label_pins_lost",
                    "message": f"{table}: etiket iğneleri korunamadı ({'; '.join(parts)}); okumayı geri alırsanız dönerler.",
                    "path": "layer"})
    moved = report.get("moved", [])
    if moved:
        out.append({"code": "label_pins_moved",
                    "message": f"{table}: {len(moved)} nesnenin etiket iğneleri “{key}” anahtarının yeni satırına taşındı.",
                    "path": "layer"})
    return out


def pin_of(uid, pin):
    return other("cad.labels.pin", {"pins": [{"uid": uid, "pin": pin}]})


def set_ada(uid, value):
    return other("cad.entities.set", {"uids": [uid], "attrs": {KEY: value}, "operation": "attributes"})


def keyed_link(table="parsel", key=KEY, name=Q):
    """`link` onto a new layer with a key: the answer and the objects as ever, the key in the binding."""
    s, b, ids = link_step(table, name, cmd="cad.sources.edit")
    s = copy.deepcopy(s)
    s["input"]["edit"]["key"] = key
    b = {**b, "key": key}
    s["result"]["output"]["link"] = b
    s["expect"]["settings"] = {"sources": [b]}
    return s, b, ids


def pin_edit_cases():
    parsel = rows_of(Q, "parsel")
    ua, ub = parsel[0]["uid"], parsel[1]["uid"]
    mal = rows_of(Q, "malik")
    s_link, b, ids = link_step("parsel", cmd="cad.sources.edit")
    k_link, kb, _ = keyed_link()
    inside = in_area(Q, "parsel", AREA)
    assert inside and len(inside) < len(parsel)
    slot_of = {r["fid"]: 2 + i for i, r in enumerate(parsel)}
    geometric = [r for r in parsel if r["kind"] != "record"]
    keep_row = next(r for r in geometric if r["fid"] in inside)
    out_row = next(r for r in geometric if r["fid"] not in inside)
    # The whole table onto a new layer, a pin on an object inside the area and one outside, then the area.
    rows_in = {r["uid"]: r["attrs"].get(KEY) for r in parsel if r["fid"] in inside}
    outside_uids = {r["uid"] for r in parsel if r["fid"] not in inside}
    bearers = [{"uid": keep_row["uid"], "value": keep_row["attrs"].get(KEY), "pins": [PIN_A]},
               {"uid": out_row["uid"], "value": out_row["attrs"].get(KEY), "pins": [PIN_B]}]
    report, _ = pins_after(bearers, rows_in, None, outside_uids)
    narrowed = {**b, "extent": AREA}
    gone_slots = [slot_of[r["fid"]] for r in parsel if r["fid"] not in inside]
    remaining = [1] + [slot_of[r["fid"]] for r in parsel if r["fid"] in inside]
    extent_out = {"link": narrowed, "objects": 0, "pins": report, "revision": "$current"}
    # An area on a table linked with the area, taken away: the rows outside come in, with no pins to give.
    a_link, ab, _ = area_link_step("parsel", AREA, cmd="cad.sources.edit")
    widened = {k: v for k, v in ab.items() if k != "extent"}
    added = len(parsel) - len(inside)
    cs = [
        case("anahtarla bağlanır: ada alanı, katmanın bağında", k_link),
        case("anahtar tabloda olmayan bir alan: invalid_key",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "key": "yok"}},
                  failed("invalid_key", f"“yok” alanı 3 nesnede yok ya da boş (örneğin {ua}); anahtar her nesnede bir "
                         "değer taşımalı.", "edit.key"))),
        case("anahtar bir nesnede boşsa invalid_key",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "key": "parsel_no"}},
                  failed("invalid_key", f"“parsel_no” alanı 1 nesnede yok ya da boş (örneğin {ub}); anahtar her nesnede "
                         "bir değer taşımalı.", "edit.key"))),
        case("anahtar iki nesnede aynıysa invalid_key",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "malik", "key": "pay"}},
                  failed("invalid_key", f"“pay” alanının “1/2” değeri iki nesnede ({mal[0]['uid']} ve {mal[1]['uid']}); "
                         "anahtar iki nesnede aynı olamaz.", "edit.key"))),
        case("anahtarın adı boş olamaz",
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "key": " "}},
                  failed("invalid_key", "Anahtar alanı boş, çok uzun ya da denetim karakteri taşıyor.", "edit.key"))),
        case("var olan katmana anahtarla bağlanır; anahtar katmanın nesnelerinde aranır",
             other("file.import.gpkg", {"path": fx(Q), "tables": ["parsel"]}),
             step({"edit": {"kind": "link", "path": fx(Q), "table": "parsel", "layer": "import-parsel", "key": KEY}},
                  done({"link": {**link("parsel", "import-parsel"), "key": KEY}, "objects": 0, "revision": "$current"}),
                  {"revision": "changed", "settings": {"sources": [{**link("parsel", "import-parsel"), "key": KEY}]}})),
        case("key işlemi: bağlı katmana anahtar konur", s_link,
             step({"edit": {"kind": "key", "id": "kaynak-1", "key": KEY}},
                  done({"link": {**b, "key": KEY}, "objects": 0, "revision": "$current"}),
                  {"revision": "changed", "settings": {"sources": [{**b, "key": KEY}]}})),
        case("key işlemi: key verilmezse anahtar kalkar", k_link,
             step({"edit": {"kind": "key", "id": "kaynak-1"}},
                  done({"link": b, "objects": 0, "revision": "$current"}),
                  {"revision": "changed", "settings": {"sources": [b]}})),
        case("key işlemi: olmayan alan reddedilir", s_link,
             step({"edit": {"kind": "key", "id": "kaynak-1", "key": "yok"}},
                  failed("invalid_key", f"“yok” alanı 3 nesnede yok ya da boş (örneğin {ua}); anahtar her nesnede bir "
                         "değer taşımalı.", "edit.key"))),
        case("key işlemi: iki nesnede aynı değer reddedilir",
             link_step("malik", cmd="cad.sources.edit")[0],
             step({"edit": {"kind": "key", "id": "kaynak-1", "key": "pay"}},
                  failed("invalid_key", f"“pay” alanının “1/2” değeri iki nesnede ({mal[0]['uid']} ve {mal[1]['uid']}); "
                         "anahtar iki nesnede aynı olamaz.", "edit.key"))),
        case("key işlemi: olmayan bağ",
             step({"edit": {"kind": "key", "id": "kaynak-9", "key": KEY}},
                  failed("source_not_found", "“kaynak-9” kimlikli bir bağ yok; cad.sources.describe bağları listeler.",
                         "edit.id"))),
        case("alan sınırı değişir: alanın dışına çıkan nesnenin iğnesi söylenir", s_link,
             pin_of(keep_row["uid"], PIN_A), pin_of(out_row["uid"], PIN_B),
             step({"edit": {"kind": "extent", "id": "kaynak-1", "extent": AREA}}, done(extent_out, pin_warnings("parsel", None, report)),
                  {"ids": remaining, "fields": {str(slot_of[keep_row["fid"]]): {"labelPins": [PIN_A]}},
                   "revision": "changed", "canUndo": True, "settings": {"sources": [narrowed]}}),
             {"op": "undo", "returns": "Alan sınırı: parsel",
              "expect": {"ids": [1, 2, 3, 4], "settings": {"sources": [narrowed]},
                         "fields": {str(slot_of[out_row["fid"]]): {"labelPins": [PIN_B]}}}}),
        case("alan sınırı kalkar: dışarıdakiler iğnesiz geri gelir", a_link,
             step({"edit": {"kind": "extent", "id": "kaynak-1"}},
                  done({"link": widened, "objects": added, "revision": "$current"}),
                  {"ids": [1, *range(2, 2 + len(parsel))],
                   "revision": "changed", "canUndo": True, "settings": {"sources": [widened]}})),
        case("aynı alan: değişen bir şey yok", a_link,
             step({"edit": {"kind": "extent", "id": "kaynak-1", "extent": AREA}},
                  done({"link": ab, "objects": 0, "revision": "$current"}), {"revision": "same"})),
        case("alan sınırı ters verilirse invalid_extent", s_link,
             step({"edit": {"kind": "extent", "id": "kaynak-1", "extent": [487050.0, 4419990.0, 486990.0, 4420030.0]}},
                  failed("invalid_extent", "Alan sınırının en küçüğü en büyüğünden küçük olmalı.", "edit.extent"))),
        case("geometrisiz tablonun alan sınırı olmaz", link_step("malik", cmd="cad.sources.edit")[0],
             step({"edit": {"kind": "extent", "id": "kaynak-1", "extent": AREA}},
                  failed("invalid_extent", "“malik” tablosunun geometrisi yok; alan sınırı geometrili tabloda olur.",
                         "edit.extent"))),
        case("katman değişmişse alan değişmez: pending_changes", s_link,
             other("cad.entities.set", {"uids": [ua], "attrs": {"ada": "Yeni Mahalle"}, "operation": "attributes"}),
             step({"edit": {"kind": "extent", "id": "kaynak-1", "extent": AREA}},
                  conflict("pending_changes", "“import-parsel” katmanı dosyayla son eşleştiğinden beri değişti; alanı "
                           "değiştirmek bu değişiklikleri götürür. Önce yazın (cad.sources.sync write).", "edit.id"))),
        case("olmayan bağın alanı değişmez",
             step({"edit": {"kind": "extent", "id": "kaynak-9", "extent": AREA}},
                  failed("source_not_found", "“kaynak-9” kimlikli bir bağ yok; cad.sources.describe bağları listeler.",
                         "edit.id"))),
        case("plan alan değişince ne olacağını söyler, yazmaz", s_link,
             pin_of(keep_row["uid"], PIN_A), pin_of(out_row["uid"], PIN_B),
             step({"edit": {"kind": "extent", "id": "kaynak-1", "extent": AREA}},
                  done(extent_out, pin_warnings("parsel", None, report)),
                  {"ids": [1, 2, 3, 4], "revision": "same", "settings": {"sources": [b]}}, op="plan")),
    ]
    return cs


def pin_sync_cases():
    parsel = rows_of(Q, "parsel")
    ua, ub = parsel[0]["uid"], parsel[1]["uid"]
    ka, kb = parsel[0]["attrs"][KEY], parsel[1]["attrs"][KEY]
    s_link, b, ids = link_step("parsel", cmd="cad.sources.edit")
    k_link, kb_link, _ = keyed_link()
    rows = {r["uid"]: r["attrs"].get(KEY) for r in parsel}
    r = {"layer": "import-parsel", "direction": "read", "discard": True}

    def pins_of(slot, pins):
        return {str(slot): {"labelPins": pins}}

    out = []
    # Without a key the row id is the record: a pin stays where the row is.
    out.append(case("iğne satırı yerinde olan nesnede kalır (anahtarsız)", s_link, pin_of(ua, PIN_A),
                    step({**r, "discard": False}, done(synced(pins={"kept": 1})),
                         {"fields": pins_of(2, [PIN_A]), "revision": "same"})))
    out.append(case("iğnesiz katmanın okuması iğne bilgisi taşımaz", s_link,
                    step({**r, "discard": False}, done(synced()), {"revision": "same"})))
    # An object the table has no row of: it goes, its pins are said.
    report = {"kept": 0, "lost": [{"uid": "$uid:fazla", "reason": "deleted"}]}
    out.append(case("satırı tabloda olmayan nesne gider, iğnesi söylenir", s_link,
                    other("cad.point.create", {"layerId": "import-parsel", "p": gi.P(487600, 4420600)}),
                    {"op": "captureUid", "id": 5, "as": "fazla"},
                    pin_of("$uid:fazla", PIN_A),
                    step(r, done(synced(removed=1, pins=report), pin_warnings("parsel", None, report)),
                         {"ids": [1, 2, 3, 4], "revision": "changed", "canUndo": True})))
    # The key: two objects that swapped records (the table was written again with other row ids).
    swap = [{"uid": ua, "value": kb, "pins": [PIN_A]}, {"uid": ub, "value": ka, "pins": [PIN_B]}]
    report, after = pins_after(swap, rows, KEY)
    assert len(report["moved"]) == 2 and "lost" not in report
    swapped = [k_link, pin_of(ua, PIN_A), pin_of(ub, PIN_B), set_ada(ua, kb), set_ada(ub, ka)]
    out.append(case("anahtarla: satırı değişen nesnelerin iğneleri anahtarın yeni satırına taşınır", *swapped,
                    step(r, done(synced(modified=2, pins=report), pin_warnings("parsel", KEY, report)),
                         {"fields": {**pins_of(2, after[ua]), **pins_of(3, after[ub])}, "revision": "changed",
                          "canUndo": True})))
    out.append(case("anahtarsız: satırı değişen nesnenin iğnesi satır kimliğinde kalır (izlenemez)",
                    s_link, pin_of(ua, PIN_A), pin_of(ub, PIN_B), set_ada(ua, kb), set_ada(ub, ka),
                    step(r, done(synced(modified=2, pins={"kept": 2})),
                         {"fields": {**pins_of(2, [PIN_A]), **pins_of(3, [PIN_B])}, "revision": "changed",
                          "canUndo": True})))
    # The old value is nowhere in the table: the pin is the old record's, which is gone.
    lone = [{"uid": ua, "value": "Eski Mahalle", "pins": [PIN_A]}]
    report, after = pins_after(lone, rows, KEY)
    assert report["lost"][0]["reason"] == "rebound"
    out.append(case("anahtarla: eski değerin satırı yoksa iğne kaybolur (rebound), nesnenin iğnesi boşalır",
                    k_link, pin_of(ua, PIN_A), set_ada(ua, "Eski Mahalle"),
                    step(r, done(synced(modified=1, pins=report), pin_warnings("parsel", KEY, report)),
                         {"fields": {**pins_of(2, None)}, "revision": "changed", "canUndo": True})))
    # The row of the key has pins of its own: its pins are kept, the others' are lost.
    taken = [{"uid": ua, "value": kb, "pins": [PIN_A]}, {"uid": ub, "value": kb, "pins": [PIN_B]}]
    report, after = pins_after(taken, rows, KEY)
    assert report["lost"] == [{"uid": ua, "reason": "taken"}] and report["kept"] == 1
    out.append(case("anahtarla: anahtarın satırında iğne varsa o korunur, öbürü kaybolur (taken)",
                    k_link, pin_of(ua, PIN_A), pin_of(ub, PIN_B), set_ada(ua, kb),
                    step(r, done(synced(modified=1, pins=report), pin_warnings("parsel", KEY, report)),
                         {"fields": {**pins_of(2, None), **pins_of(3, [PIN_B])}, "revision": "changed",
                          "canUndo": True})))
    # Nothing moved: the pins stay and are counted.
    out.append(case("anahtarla: satırları yerinde olan nesnelerin iğneleri kalır", k_link, pin_of(ua, PIN_A),
                    pin_of(ub, PIN_B),
                    step({**r, "discard": False}, done(synced(pins={"kept": 2})),
                         {"fields": {**pins_of(2, [PIN_A]), **pins_of(3, [PIN_B])}, "revision": "same"})))
    # Geri alma: okumanın adımı iğneleri de geri getirir.
    out.append(case("anahtarla taşınan iğneler okumayı geri alınca döner", *swapped,
                    step(r, done(synced(modified=2, pins=pins_after(swap, rows, KEY)[0]),
                                 pin_warnings("parsel", KEY, pins_after(swap, rows, KEY)[0])),
                         {"revision": "changed", "canUndo": True}),
                    {"op": "undo", "returns": "Kaynaktan oku: parsel",
                     "expect": {"fields": {**pins_of(2, [PIN_A]), **pins_of(3, [PIN_B])}}}))
    # The plan says it too.
    report, _ = pins_after(swap, rows, KEY)
    out.append(case("plan iğnelerin ne olacağını söyler, yazmaz", *swapped,
                    step(r, done(synced(modified=2, pins=report), pin_warnings("parsel", KEY, report)),
                         {"fields": {**pins_of(2, [PIN_A]), **pins_of(3, [PIN_B])}, "revision": "same"}, op="plan")))
    return out


FILES = {
    "cad.sources.describe": ("Kaynakları tanı: dosyanın tabloları ve bağlı katmanların durumu", describe_cases),
    "cad.sources.edit": ("Kaynağa bağla: tablo katmana, bağı çöz, yeniden bağla",
                         lambda: edit_cases() + area_cases()[0] + pin_edit_cases()),
    "cad.sources.sync": ("Kaynakla eşitle: katmanın değişiklikleri tabloya, tablonun satırları katmana",
                         lambda: sync_cases() + area_cases()[1] + pin_sync_cases() + neighbour_cases()),
}
NOTE = ("ADR 0503. Tablolar sqlite3 ile dosyanın kendi tablolarından okunur (gpkg_contents, gpkg_geometry_columns, "
        "gpkg_spatial_ref_sys, PRAGMA table_info); satırlar gpkg_import_cases.py'nin OGR okuyucusuyla ADR 0502'nin "
        "nesneleri. Dosyanın sürümü hashlib'in SHA-256'sı. Katmanın tablodan farkı durumun kuruluşundan gelir; özetin "
        "değeri $any, anlamı edited'dır. Dosyaya yazan execute ortak durumlarda yoktur (fixture dosyası değişirdi): "
        "geçici kopyada crates/native/application/tests/all/sources.rs sınar, sonucu geopackage_cases.py OGR'la okur. "
        "Bağımsız başvuru: scripts/fixtures/bound_sources_cases.py.")


def build(command):
    title, make = FILES[command]
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": SETUP, "cases": make()}


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    bad = 0
    for command in FILES:
        out = OUT / f"{command}.json"
        data = build(command)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv:
            if not out.exists() or out.read_text("utf-8") != text:
                print(f"{out.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
                bad += 1
            continue
        out.write_text(text, "utf-8")
        print(f"yazıldı: {out.relative_to(ROOT)} ({len(data['cases'])} durum)")
    if "--check" in sys.argv:
        problem = check_written()
        if problem:
            print(problem, file=sys.stderr)
            bad += 1
    if "--check" in sys.argv and not bad:
        print("Bağlı katman durumları tutarlı: " + ", ".join(f"{c} {len(build(c)['cases'])}" for c in FILES))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
