#!/usr/bin/env python3
"""The shared cases of the imports' merge (docs/adr/0628 §5, §6): a file's rows taken into a layer that holds objects
already, written without KentOS code.

    .run/geo-py/bin/python scripts/fixtures/import_merge_cases.py --files   # writes fixtures/import-merge/v1
    .run/geo-py/bin/python scripts/fixtures/import_merge_cases.py           # writes fixtures/commands/v1/file.import.merge.json
    .run/geo-py/bin/python scripts/fixtures/import_merge_cases.py --check   # writes nothing; compares

.run/geo-py is a virtual environment over the system Python and its GDAL (uv venv --system-site-packages; pyarrow 26).

A file's rows come from the import cases' own references, which share nothing with KentOS: GDAL's OGR for GeoPackage,
CSV, FlatGeobuf and GML (gpkg_import_cases.py, csv_exchange_cases.py, fgb_exchange_cases.py, gml_exchange_cases.py),
pyarrow for GeoParquet (geoparquet_cases.py) and tools/formats/gis.py for GeoJSON and Shapefile (gis_import_cases.py).
The value a typed field takes is layer_field_cases.py's, a key Anahtarla birleştir's (join_view_cases.py). What the
merge does is written here from the ADR:
- A table's rows go onto a layer of the drawing (`layers`, else the layer of its name) and meet its objects; on a new
  layer every row is added. A row's key is its `key` column, else its lasting id (an import whose rows have none,
  GeoJSON's and Shapefile's, refuses without a key); an object's its `targetKey` attribute (else `key`'s name).
- The source giving a key more than once: `stop` refuses the import (its first such key said), `first` takes the
  first row and refuses the others, `none` refuses them all.
- Each row in order is refused when: its key is empty; the source repeats its key; its geometry is not of the layer's
  type; a value is not one its field takes; its key is on more than one object; (by lasting id) its id is another
  layer's object's. Else a row no object matches is added (its values as their fields write them) or, with
  `unmatched: skip`, left.
- A matched object: `keep` leaves it. With `geometry` (the default) it takes the row's geometry when the two are of
  one family (point, line, area; a record keeps the object's), else the row is refused. `update` writes every column
  of the source (a cell empty in the row empties the object's), `fillEmpty` only what is empty on the object. An object
  that ends as it was is unchanged.
- The rows refused are said in a rows_rejected warning, their reasons counted by code. With `rejects` (`execute`) they
  are written to a CSV: kaynak, satir, anahtar, neden, the source's columns, WKT (the row's geometry as ISO WKT, a
  number as the shortest text that reads back, a whole one without decimals); RFC 4180 quoting, CRLF lines. Without a
  row refused (and without a merge) the file is its header. The path is absolute, a .csv, in a folder that is there.
"""
import copy
import datetime
import decimal
import hashlib
import importlib.util
import json
import math
import sys
from collections import Counter
from pathlib import Path
import uuid

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/import-merge/v1"
OUT = ROOT / "fixtures/commands/v1/file.import.merge.json"


def load(name):
    spec = importlib.util.spec_from_file_location(f"merge_{name}", ROOT / f"scripts/fixtures/{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


G = load("gpkg_import_cases")
PQ = load("geoparquet_cases")
CSV = load("csv_exchange_cases")
FGB = load("fgb_exchange_cases")
GML = load("gml_exchange_cases")
GI = load("gis_import_cases")
LF = load("layer_field_cases")
JV = load("join_view_cases")

SRID = 5256
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

# ── The drawing ─────────────────────────────────────────────────────────────────


def P(p):
    return {"x": float(p[0]), "y": float(p[1])}


def square(x0, y0=4420000, s=20):
    return [(x0, y0), (x0 + s, y0), (x0 + s, y0 + s), (x0, y0 + s)]


def layer(id_, name, fields=None, locked=False, geometry=None):
    n = {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
         "style": STYLE, "children": []}
    if fields:
        n["fields"] = fields
    if geometry:
        n["geometry"] = geometry
    return n


PARSEL_FIELDS = [{"name": "parsel_no", "kind": "integer"}, {"name": "ada", "kind": "integer"},
                 {"name": "alan", "kind": "decimal", "scale": 2}, {"name": "malik", "kind": "text", "length": 40},
                 {"name": "tescil", "kind": "date"}]


def polygon(slot, pts, attrs, layer_id="parsel"):
    return {"kind": "polygon", "id": slot, "layerId": layer_id, "attrs": attrs, "pts": [P(p) for p in pts]}


# Parcels with typed fields (two objects share 104, one has no number), a register keyed by another name, a typed
# line layer and a locked one.
SETUP = {
    "format": "kentos.document", "version": 1, "name": "Artırımlı yükleme",
    "settings": {"srid": SRID, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis"},
    "origin": {"x": 0.0, "y": 0.0},
    "layers": [
        layer("cizim", "Çizim"),
        layer("parsel", "parsel", PARSEL_FIELDS),
        layer("kadastro", "Kadastro", [{"name": "no", "kind": "text"}, {"name": "malik", "kind": "text"}]),
        layer("yol", "yol", [{"name": "no", "kind": "text"}], geometry="line"),
        layer("yapi", "yapi", locked=True),
    ],
    "activeLayer": "cizim",
    "entities": [
        {"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": P((487500, 4420500))},
        polygon(2, square(487000), {"parsel_no": "101", "ada": "12", "alan": "100", "malik": "Ayşe Yılmaz",
                                    "tescil": "2020-01-15"}),
        polygon(3, square(487030), {"parsel_no": "102", "ada": "12", "alan": "100"}),
        polygon(4, square(487060), {"parsel_no": "103", "ada": "12", "alan": "100", "malik": "Mehmet Öz"}),
        polygon(5, square(487090), {"parsel_no": "104", "ada": "13", "alan": "100"}),
        polygon(6, square(487120), {"parsel_no": "104", "ada": "13", "alan": "100"}),
        polygon(7, square(487150), {"ada": "13", "alan": "100"}),
        polygon(8, square(487000, 4420100), {"no": "101", "malik": "Eski Malik"}, "kadastro"),
        {"kind": "line", "id": 9, "layerId": "yol", "attrs": {"no": "Y-1"}, "a": P((487000, 4420200)),
         "b": P((487100, 4420200))},
    ],
    "styles": {"items": [], "categories": []},
}

# ── The files ───────────────────────────────────────────────────────────────────


def wkt_polygon(pts):
    ring = list(pts) + [pts[0]]
    return "POLYGON ((" + ", ".join(f"{x} {y}" for x, y in ring) + "))"


# The parcels of 2025: 101 moved and measured again, 102 with its owner, 103 as it is, 104 (two objects), a new 105, a
# row without a number and one whose block is not a number.
PARCELS = [
    ({"parsel_no": 101, "ada": "12", "alan": 105.5, "malik": "Ayşe Yılmaz", "tescil": "2020-01-15"}, square(487001)),
    ({"parsel_no": 102, "ada": "12", "alan": 100.0, "malik": "Ali Kaya", "tescil": None}, square(487030)),
    ({"parsel_no": 103, "ada": "12", "alan": 100.0, "malik": "Mehmet Öz", "tescil": None}, square(487060)),
    ({"parsel_no": 104, "ada": "13", "alan": 100.0, "malik": None, "tescil": None}, square(487090)),
    ({"parsel_no": 105, "ada": "14", "alan": 90.25, "malik": "Zeynep Ak", "tescil": "2024-06-30"}, square(487180)),
    ({"parsel_no": None, "ada": "14", "alan": 80.0, "malik": "Adsız", "tescil": None}, square(487210)),
    ({"parsel_no": 106, "ada": "14A", "alan": 75.0, "malik": "Can Er", "tescil": None}, square(487240)),
]
# 101 twice.
REPEATED = [
    ({"parsel_no": 101, "ada": "12", "alan": 110.0, "malik": "Ayşe Yılmaz", "tescil": "2020-01-15"}, square(487000)),
    ({"parsel_no": 101, "ada": "12", "alan": 120.0, "malik": "Ayşe Yılmaz", "tescil": "2020-01-15"}, square(487000)),
    ({"parsel_no": 105, "ada": "14", "alan": 90.25, "malik": "Zeynep Ak", "tescil": "2024-06-30"}, square(487180)),
]
PARCEL_COLUMNS = [("parsel_no", "Integer"), ("ada", "String"), ("alan", "Real"), ("malik", "String"),
                  ("tescil", "Date")]


def write_files():
    from osgeo import gdal, ogr, osr
    import pyarrow as pa
    import shapely

    gdal.UseExceptions()
    # The GeoPackages' timestamps fixed: the files are the same each time they are written.
    gdal.SetConfigOption("OGR_CURRENT_DATE", "2026-10-24T00:00:00.000Z")
    for sub in ("gpkg", "parquet", "parquet/yinelenen", "csv", "csv/yinelenen", "fgb", "gml", "geojson",
                "geojson/yinelenen", "shp"):
        (DIR / sub).mkdir(parents=True, exist_ok=True)
        for p in (DIR / sub).iterdir():
            if p.is_file():
                p.unlink()
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(SRID)
    kinds = {"Integer": ogr.OFTInteger, "String": ogr.OFTString, "Real": ogr.OFTReal, "Date": ogr.OFTDate}

    def gpkg(name, table, gtype, columns, rows):
        ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(DIR / "gpkg" / name))
        lyr = ds.CreateLayer(table, srs, gtype)
        for cname, kind in columns:
            lyr.CreateField(ogr.FieldDefn(cname, kinds[kind]))
        for attrs, wkt in rows:
            f = ogr.Feature(lyr.GetLayerDefn())
            for k, v in attrs.items():
                if v is None:
                    f.SetFieldNull(k)
                else:
                    f.SetField(k, v)
            f.SetGeometry(ogr.CreateGeometryFromWkt(wkt))
            lyr.CreateFeature(f)
        ds = None

    def parcels(rows):
        return [(a, wkt_polygon(pts)) for a, pts in rows]

    gpkg("parsel-2025.gpkg", "parsel", ogr.wkbPolygon, PARCEL_COLUMNS, parcels(PARCELS))
    gpkg("parsel-yinelenen.gpkg", "parsel", ogr.wkbPolygon, PARCEL_COLUMNS, parcels(REPEATED))
    blocks = [("A blok", 3, "POINT (487300 4420000)"), ("B blok", 5, "POINT (487320 4420000)"),
              ("C blok", 2, "POINT (487340 4420000)")]
    gpkg("bina-2024.gpkg", "bina", ogr.wkbPoint, [("ad", "String"), ("kat", "Integer")],
         [({"ad": a, "kat": k}, w) for a, k, w in blocks])
    blocks_2025 = [("A blok", 4, "POINT (487300 4420000)"), ("B blok", 5, "POINT (487320 4420000)"),
                   ("C blok", 2, "POINT (487345 4420005)"), ("D blok", 1, "POINT (487360 4420000)")]
    gpkg("bina-2025.gpkg", "bina", ogr.wkbPoint, [("ad", "String"), ("kat", "Integer")],
         [({"ad": a, "kat": k}, w) for a, k, w in blocks_2025])
    gpkg("yol-2025.gpkg", "yol", ogr.wkbUnknown, [("no", "String")],
         [({"no": "Y-1"}, "LINESTRING (487000 4420200, 487100 4420210)"),
          ({"no": "Y-2"}, wkt_polygon(square(487000, 4420250))),
          ({"no": "Y-3"}, "LINESTRING (487000 4420300, 487100 4420300)")])
    gpkg("parsel-nokta.gpkg", "parsel", ogr.wkbPoint, [("parsel_no", "Integer"), ("malik", "String")],
         [({"parsel_no": 101, "malik": "Ayşe Yılmaz Kaya"}, "POINT (487010 4420010)"),
          ({"parsel_no": 107, "malik": "Can Er"}, "POINT (487270 4420010)")])
    gpkg("yapi.gpkg", "yapi", ogr.wkbPoint, [("ad", "String")], [({"ad": "Depo"}, "POINT (487400 4420000)")])

    # The parcels in the other formats: GeoParquet by pyarrow, CSV, FlatGeobuf, GML and Shapefile by GDAL from the
    # GeoPackage, GeoJSON by hand.
    def parquet(folder, rows):
        PQ.DIR = DIR / folder
        t = pa.table({
            "geometry": PQ.wkb_column([shapely.Polygon(pts) for _, pts in rows]),
            "parsel_no": pa.array([a["parsel_no"] for a, _ in rows], type=pa.int64()),
            "ada": pa.array([a["ada"] for a, _ in rows]),
            "alan": pa.array([a["alan"] for a, _ in rows], type=pa.float64()),
            "malik": pa.array([a["malik"] for a, _ in rows]),
            "tescil": pa.array([None if a["tescil"] is None else datetime.date.fromisoformat(a["tescil"])
                                for a, _ in rows], type=pa.date32()),
        })
        PQ.write_arrow("parsel.parquet", t, PQ.geo_meta(types=["Polygon"]))

    parquet("parquet", PARCELS)
    parquet("parquet/yinelenen", REPEATED)
    for src, folder in (("parsel-2025.gpkg", "csv"), ("parsel-yinelenen.gpkg", "csv/yinelenen")):
        gdal.VectorTranslate(str(DIR / folder / "parsel.csv"), str(DIR / "gpkg" / src), format="CSV",
                             layers=["parsel"],
                             layerCreationOptions=["GEOMETRY=AS_WKT", "CREATE_CSVT=YES", "LINEFORMAT=CRLF"])
    src = DIR / "gpkg" / "parsel-2025.gpkg"
    gdal.VectorTranslate(str(DIR / "fgb" / "parsel.fgb"), str(src), format="FlatGeobuf", layers=["parsel"])
    GML.translate(DIR / "gml" / "parsel.gml", src, ["parsel"])
    gdal.VectorTranslate(str(DIR / "shp" / "parsel.shp"), str(src), format="ESRI Shapefile", layers=["parsel"],
                         layerCreationOptions=["ENCODING=UTF-8"])
    for folder, rows in (("geojson", PARCELS), ("geojson/yinelenen", REPEATED)):
        doc = {"type": "FeatureCollection", "name": "parsel",
               "crs": {"type": "name", "properties": {"name": f"urn:ogc:def:crs:EPSG::{SRID}"}},
               "features": [{"type": "Feature", "properties": a,
                             "geometry": {"type": "Polygon",
                                          "coordinates": [[list(p) for p in pts + [pts[0]]]]}} for a, pts in rows]}
        (DIR / folder / "parsel.geojson").write_text(json.dumps(doc, ensure_ascii=False, indent=1) + "\n",
                                                     encoding="utf-8")
    print(f"dosyalar yazıldı: {DIR}")


# ── What the references read ────────────────────────────────────────────────────


def read(fmt, name, sub=""):
    folder = DIR / fmt / sub if sub else DIR / fmt
    if fmt == "gpkg":
        G.DIR = folder
        return G.read(name)
    if fmt == "parquet":
        PQ.DIR = folder
        return PQ.read_parquet(name)
    if fmt == "csv":
        CSV.DIR = folder
        return CSV.read_csv(name)
    if fmt == "fgb":
        FGB.DIR = folder
        return FGB.read_fgb(name)
    if fmt == "gml":
        GML.DIR = folder
        return GML.read_gml(name)
    raise ValueError(fmt)


# ── The rules ───────────────────────────────────────────────────────────────────

FAMILY = {"line": "line", "polyline": "line", "arc": "line", "spline": "line", "nurbs": "line",
          "polygon": "area", "circle": "area", "ellipse": "area"}
ADMITS = {"point": {"point"}, "line": {"line", "polyline", "arc", "spline", "nurbs", "circle", "ellipse"},
          "area": {"polygon", "circle", "ellipse"},
          "annotation": {"text", "dimension", "leader", "table", "tag", "hatch", "insert", "image", "viewmark"}}
KIND_WORDS = {"point": "nokta", "line": "çizgi", "polyline": "çoklu çizgi", "polygon": "alan", "record": "kayıt"}
LAYER_WORDS = {"point": "nokta katmanı", "line": "çizgi katmanı", "area": "alan katmanı",
               "annotation": "açıklama katmanı"}
WORDS = {"key_empty": "anahtarı boş", "key_duplicate": "anahtarı kaynakta yineleniyor",
         "target_ambiguous": "anahtarı katmanda birden çok nesnede", "field_value": "değeri alanına uymuyor",
         "geometry_kind": "geometrisi eşleşen nesneninkiyle aynı türde değil",
         "layer_type": "geometrisi katmanın türüne uymuyor",
         "uid_elsewhere": "kalıcı kimliği başka bir katmanın nesnesinde"}
GEOMETRY_KEYS = {"p", "z", "parts", "a", "b", "za", "zb", "ma", "mb", "pts", "bulges", "zs", "ms", "holes"}


def ascii_same(a, b):
    return a.lower() == b.lower() if a.isascii() and b.isascii() else a == b


def attr(attrs, name):
    if name in attrs:
        return attrs[name]
    return next((v for k, v in attrs.items() if ascii_same(k, name)), None)


def key_of(text):
    return None if text is None else JV.join_key(text)


def real_text(v):
    t = format(decimal.Decimal(repr(float(v))), "f")
    return t[:-2] if t.endswith(".0") else t


def num(v):
    return str(int(v)) if v == math.trunc(v) and abs(v) < 1e15 else real_text(v)


def wkt_of(kind, g):
    """The row's geometry as ISO WKT, rings closed (the import's write_wkt)."""
    def coords(pts):
        return "(" + ", ".join(f"{num(p['x'])} {num(p['y'])}" for p in pts) + ")"

    if kind == "record":
        return ""
    if kind == "point":
        return f"POINT ({num(g['p']['x'])} {num(g['p']['y'])})"
    if kind == "polyline":
        return "LINESTRING " + coords(g["pts"])
    if kind == "polygon":
        rings = [g["pts"]] + [h["pts"] for h in g.get("holes", [])]
        return "POLYGON (" + ", ".join(coords(r + [r[0]]) for r in rings) + ")"
    raise ValueError(kind)


def quoted(t):
    if any(c in t for c in ',"\n\r') or t.startswith(" ") or t.endswith(" "):
        return '"' + t.replace('"', '""') + '"'
    return t


def rejects_bytes(rejected, columns):
    head = ["kaynak", "satir", "anahtar", "neden", *columns, "WKT"]
    lines = [",".join(quoted(h) for h in head)]
    for r in rejected:
        cells = [r["table"], str(r["row"]), r["key"], r["detail"],
                 *[attr(r["attrs"], c) or "" for c in columns], r["wkt"]]
        lines.append(",".join(quoted(c) for c in cells))
    return ("\r\n".join(lines) + "\r\n").encode("utf-8")


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.code, self.message, self.path = code, message, path


class Doc:
    """The drawing as the cases know it: its objects by slot, the lasting ids they know, its layer nodes."""

    def __init__(self):
        self.objects = {e["id"]: {k: v for k, v in e.items() if k != "id"} for e in SETUP["entities"]}
        self.uids = {}
        self.nodes = [copy.deepcopy(n) for n in SETUP["layers"]]
        self.made = 0

    def json(self):
        return {**SETUP, "layers": self.nodes}

    def node(self, id_):
        return next((n for n in self.nodes if n["id"] == id_), None)

    def named(self, name):
        return next((n for n in self.nodes if n["type"] == "layer" and GI.fold(n["name"]) == GI.fold(name)), None)

    def slot_of(self, uid):
        return next((s for s, u in self.uids.items() if u == uid), None)

    def next_slot(self):
        return max(self.objects) + 1

    def ids(self):
        return sorted(self.objects)

    def on(self, layer_id):
        return {str(s): copy.deepcopy(e) for s, e in sorted(self.objects.items()) if e["layerId"] == layer_id}


def free_uid(doc, uid, key, taken):
    u, n = uid, 0
    while doc.slot_of(u) is not None or u in taken:
        n += 1
        u = str(uuid.uuid5(uuid.UUID(uid), f"kentos-merge:{key}:{n}"))
    return u


def checked(attrs, fields):
    out = {}
    for k, v in attrs.items():
        f = next((f for f in fields if ascii_same(f["name"], k)), None)
        if f is None:
            out[k] = v
            continue
        r = LF.check(f, v)
        if "error" in r:
            return None, r["message"]
        out[f["name"]] = r["value"]
    return out, None


def merge(doc, layer_id, table, rows, columns, m, ids):
    """What the rows of `table` do to the objects of `layer_id`: the objects to add and to change, the report and the
    rows refused; or the refusal."""
    node = doc.node(layer_id)
    fields = node.get("fields", [])
    ltype = node.get("geometry")
    key = m.get("key")
    tkey = m.get("targetKey") or key
    if key is None and not ids:
        raise Refused("merge_key_required", f"“{table}” satırlarının kalıcı kimliği yok; birleştirmek için merge.key "
                      "ile anahtar sütunu verin.", "merge.key")
    if key is not None and not any(ascii_same(c, key) for c in columns):
        raise Refused("merge_key_not_found", f"“{key}” sütunu “{table}” tablosunda yok; sütunlardan birini verin "
                      f"({', '.join(columns[:8])}).", "merge.key")
    index = {}
    if tkey is not None:
        for slot, e in sorted(doc.objects.items()):
            if e["layerId"] == layer_id:
                k = key_of(attr(e["attrs"], tkey))
                if k is not None:
                    index.setdefault(k, []).append(slot)
    keys = [key_of(attr(r["attrs"], key)) if key is not None else r["uid"] for r in rows]
    count = Counter(k for k in keys if k is not None)
    duplicates = m.get("duplicates", "stop")
    if duplicates == "stop":
        for i, k in enumerate(keys):
            if k is not None and count[k] > 1:
                shown = attr(rows[i]["attrs"], key or "") or k
                raise Refused("duplicate_key", f"“{table}” tablosunda “{shown}” anahtarı {count[k]} satırda; ilkini "
                              "almak için duplicates: first, hiçbirini almamak için duplicates: none verin.",
                              "merge.duplicates")
    out = {"add": [], "update": [], "rejected": []}
    report = {"table": table, "layer": layer_id, "added": 0, "updated": 0, "unchanged": 0, "skipped": 0,
              "rejected": 0, "reasons": {}}
    seen, taken = set(), set()

    def reject(i, reason, detail):
        r = rows[i]
        report["rejected"] += 1
        report["reasons"][reason] = report["reasons"].get(reason, 0) + 1
        out["rejected"].append({"table": table, "row": r["fid"],
                                "key": (attr(r["attrs"], key) or "") if key is not None else r["uid"],
                                "reason": reason, "detail": detail, "attrs": r["attrs"],
                                "wkt": wkt_of(r["kind"], r["geometry"])})

    for i, r in enumerate(rows):
        k = keys[i]
        if k is None:
            reject(i, "key_empty", "anahtarı boş")
            continue
        if count[k] > 1:
            first = k not in seen
            seen.add(k)
            if duplicates == "none" or not first:
                reject(i, "key_duplicate", f"anahtarı kaynakta {count[k]} satırda")
                continue
        if ltype is not None and r["kind"] != "record" and r["kind"] not in ADMITS[ltype]:
            reject(i, "layer_type", f"{KIND_WORDS[r['kind']]}, {LAYER_WORDS[ltype]}na konamaz")
            continue
        attrs, why = checked(r["attrs"], fields)
        if attrs is None:
            reject(i, "field_value", why)
            continue
        if key is not None:
            hits = index.get(k, [])
            if len(hits) > 1:
                reject(i, "target_ambiguous", f"anahtarı katmanda {len(hits)} nesnede")
                continue
            matched = hits[0] if hits else None
        else:
            slot = doc.slot_of(r["uid"])
            if slot is not None and doc.objects[slot]["layerId"] != layer_id:
                reject(i, "uid_elsewhere", "kalıcı kimliği başka bir katmanın nesnesinde")
                continue
            matched = slot
        if matched is None:
            if m.get("unmatched", "add") == "skip":
                report["skipped"] += 1
                continue
            u = free_uid(doc, r["uid"], k, taken) if r["uid"] is not None else None
            taken.add(u)
            out["add"].append((u, {"kind": r["kind"], "layerId": layer_id, "attrs": attrs, **r["geometry"]}))
            report["added"] += 1
            continue
        existing = doc.objects[matched]
        if m.get("matched", "update") == "keep":
            report["unchanged"] += 1
            continue
        if m.get("geometry", True) and r["kind"] != "record":
            if FAMILY.get(r["kind"], r["kind"]) != FAMILY.get(existing["kind"], existing["kind"]):
                reject(i, "geometry_kind", f"{KIND_WORDS[r['kind']]}, eşleşen {KIND_WORDS[existing['kind']]} "
                       "nesnesinin yerine geçemez")
                continue
            nxt = {k2: copy.deepcopy(v) for k2, v in existing.items() if k2 not in GEOMETRY_KEYS}
            nxt["kind"] = r["kind"]
            nxt.update(copy.deepcopy(r["geometry"]))
        else:
            nxt = copy.deepcopy(existing)
        target = nxt["attrs"]
        if m.get("matched", "update") == "update":
            for c in columns:
                f = next((f for f in fields if ascii_same(f["name"], c)), None)
                name = f["name"] if f else c
                v = attr(attrs, c)
                if v is not None:
                    target[name] = v
                else:
                    target.pop(name, None)
        else:
            for k2, v in attrs.items():
                if v == "":
                    continue
                have = attr(target, k2)
                if have is None or have.strip() == "":
                    target[k2] = v
        if nxt == existing:
            report["unchanged"] += 1
        else:
            out["update"].append((matched, nxt))
            report["updated"] += 1
    if not report["reasons"]:
        del report["reasons"]
    out["report"] = report
    return out


def rows_warning(reports, file):
    total = sum(r["rejected"] for r in reports)
    if not total:
        return None
    reasons = Counter()
    for r in reports:
        reasons.update(r.get("reasons", {}))
    said = ", ".join(f"{n} satırın {WORDS[k]}" for k, n in sorted(reasons.items()))
    return {"code": "rows_rejected", "message": f"{file}: {total} satır alınmadı ({said}).", "path": "merge"}


def rejects_problem(path):
    if not path.startswith("$tmp:"):
        return f"“{path}” reddedilen satırların yazılacağı bir dosyanın mutlak yolu değil; /klasör/reddedilenler.csv gibi verin."
    rest = path[len("$tmp:"):]
    if not rest.lower().endswith(".csv"):
        return f"“{rest.rsplit('/', 1)[-1]}” bir .csv dosyası değil; reddedilen satırlar CSV olarak yazılır."
    if "/" in rest:
        return f"“$tmp:{rest.rsplit('/', 1)[0]}” klasörü yok; reddedilen satırlar var olan bir klasöre yazılır."
    return None


def failed(inp, doc, code, message, path, op="execute", command=None):
    return G.step(inp, G.failed(code, message, path), {"ids": doc.ids(), "revision": "same"}, op=op, command=command)


def apply(doc, layer_ids, made, adds, updates):
    """`doc` after the import: the changes, then the new objects in their order."""
    for slot, e in updates:
        doc.objects[slot] = e
    for uid, e in adds:
        slot = doc.next_slot()
        doc.objects[slot] = e
        if uid is not None:
            doc.uids[slot] = uid
    for m in made:
        doc.nodes.append(layer(m["id"], m["name"], m.get("fields")))
    return {lid: doc.on(lid) for lid in layer_ids}


def step_expect(doc, touched, changed):
    expect = {"ids": doc.ids(), "fields": {k: v for t in touched.values() for k, v in t.items()}}
    if changed:
        expect.update({"revision": "changed", "canUndo": True})
    return expect


# ── A file read as a GeoPackage is (GeoPackage, GeoParquet, CSV, FlatGeobuf, GML) ────────────────────────────


def table_step(doc, fmt, name, *, sub="", merge_=None, routes=None, rejects=None, op="execute", command=None):
    """The step importing `name` into `doc` and what it gives (`doc` follows `execute`)."""
    r = read(fmt, name, sub)
    rel = f"import-merge/v1/{fmt}/" + (f"{sub}/" if sub else "") + name
    inp = {"path": f"$fixture:{rel}"}
    if routes:
        inp["layers"] = dict(routes)
    if merge_ is not None:
        inp["merge"] = merge_
    if rejects is not None:
        inp["rejects"] = rejects
    if rejects is not None and (why := rejects_problem(rejects)):
        return failed(inp, doc, "invalid_file_path", why, "rejects", op, command)
    tables = [t for t in r["tables"] if t["rows"]]
    srids = sorted({t["srid"] for t in tables if t["srid"]})
    warnings = []
    if not srids:
        warnings.append({"code": "crs_undeclared", "message": f"{r['file']} koordinat sistemi bildirmiyor; "
                         "koordinatlar projenin sisteminde sayılıp olduğu gibi alındı.", "path": "path"})
    used = {n["id"] for n in doc.nodes}
    made, out_tables, adds, updates, merged, rejected, columns_all, losses = [], [], [], [], [], [], [], set()
    for t in tables:
        losses.update(t["losses"])
        columns = [f["name"] for f in t["fields"]]
        for c in columns:
            if not any(ascii_same(c, x) for x in columns_all):
                columns_all.append(c)
        target = (routes or {}).get(t["name"])
        if target is None:
            node = doc.named(t["name"])
            if node is not None and node["locked"]:
                return failed(inp, doc, "layer_locked", f"Dosyanın “{t['name']}” tablosu çizimdeki aynı adlı "
                              f"“{node['name']}” katmanına gider ama o katman kilitli. Kilidi açın ya da layers ile "
                              "başka bir katmana yönlendirin.", "layers", op, command)
            target = node["id"] if node is not None else None
        rows = [{"fid": x["fid"], "uid": x["uid"], "kind": x["kind"], "geometry": x["geometry"], "attrs": x["attrs"]}
                for x in t["rows"]]
        if target is None:
            target = G.slug(t["name"])
            k = 2
            while target in used:
                target = f"{G.slug(t['name'])}-{k}"
                k += 1
            used.add(target)
            made.append({"id": target, "name": t["name"], "fields": t["fields"]})
            adds += [(x["uid"], {"kind": x["kind"], "layerId": target, "attrs": x["attrs"], **x["geometry"]})
                     for x in rows]
        elif merge_ is not None:
            try:
                m = merge(doc, target, t["name"], rows, columns, merge_, True)
            except Refused as e:
                return failed(inp, doc, e.code, e.message, e.path, op, command)
            adds += m["add"]
            updates += m["update"]
            merged.append(m["report"])
            rejected += m["rejected"]
        else:
            adds += [(x["uid"], {"kind": x["kind"], "layerId": target, "attrs": x["attrs"], **x["geometry"]})
                     for x in rows]
        out_tables.append({"table": t["name"], "layer": target, "read": len(rows), "objects": len(rows),
                           "records": sum(1 for x in rows if x["kind"] == "record"), "skipped": 0})
    if (w := rows_warning(merged, r["file"])) is not None:
        warnings.append(w)
    if losses:
        warnings.append({"code": "import_losses", "message": f"{r['file']} dosyasının bazı değerleri ya da "
                         "satırları olduğu gibi alınamadı: " + "; ".join(sorted(losses)) + ".", "path": "path"})
    output = {"file": r["file"], "bytes": r["bytes"], "sha256": r["sha256"], "objects": len(adds),
              "tables": out_tables, "layers": [{"id": m["id"], "name": m["name"]} for m in made],
              "revision": "$current"}
    if srids:
        output["srid"] = srids[0]
    if made:
        stem = Path(r["file"]).stem
        group, k = G.slug(stem), 2
        while group in used:
            group = f"{G.slug(stem)}-{k}"
            k += 1
        output["group"] = group
    if merged:
        output["merged"] = merged
    if op != "execute":
        result = None if op == "validate" else output
        return G.step(inp, G.done(result, [] if op == "validate" else warnings),
                      {"ids": doc.ids(), "revision": "same"}, op=op, command=command)
    if rejects is not None:
        data = rejects_bytes(rejected, columns_all)
        output["rejects"] = {"path": rejects, "rows": len(rejected), "bytes": len(data),
                             "sha256": hashlib.sha256(data).hexdigest()}
    changed = bool(adds or updates)
    touched = apply(doc, [o["layer"] for o in out_tables], made, adds, updates)
    return G.step(inp, G.done(output, warnings), step_expect(doc, touched, changed), command=command)


# ── A file read as GeoJSON and Shapefile are (tools/formats/gis.py) ───────────────────────────────────────────


def gis_step(doc, fmt, name, *, sub="", merge_=None, rejects=None, command):
    rel = f"import-merge/v1/{fmt}/" + (f"{sub}/" if sub else "") + name
    extra = {}
    if merge_ is not None:
        extra["merge"] = merge_
    if rejects is not None:
        extra["rejects"] = rejects
    base = GI.run(fmt, rel, doc.json(), extra_inputs=extra)
    inp = base["input"]
    if base["result"]["status"] != "completed":
        raise ValueError(base["result"])
    ref = GI.read_ref(rel)
    output = dict(base["result"]["output"])
    warnings = list(base["result"]["warnings"])
    targets = output["targets"]
    adds, updates, merged, rejected = [], [], [], []
    for source in ref["layers"]:
        target = targets[source]
        objs = [o for o in ref["objects"] if o["layer"] == source]
        fields = ref["fields"].get(source) or []
        columns = [f["name"] for f in fields] or sorted({k for o in objs for k in o.get("attrs", {})})
        rows = []
        for i, o in enumerate(objs):
            f = GI.entity_fields(o)
            rows.append({"fid": i + 1, "uid": None, "kind": f["kind"], "attrs": f["attrs"],
                         "geometry": {k: v for k, v in f.items() if k in GEOMETRY_KEYS}})
        node = doc.node(target)
        if merge_ is not None and node is not None:
            try:
                m = merge(doc, target, source, rows, columns, merge_, False)
            except Refused as e:
                return failed(inp, doc, e.code, e.message, e.path, command=command)
            adds += m["add"]
            updates += m["update"]
            merged.append(m["report"])
            rejected += m["rejected"]
        else:
            raise ValueError("a new layer: not in these cases")
    output["objects"] = len(adds)
    if merged:
        output["merged"] = merged
    if (w := rows_warning(merged, ref["file"])) is not None:
        warnings.append(w)
    changed = bool(adds or updates)
    touched = apply(doc, sorted(set(targets.values())), [], adds, updates)
    return G.step(inp, G.done(output, warnings), step_expect(doc, touched, changed), command=command)


# ── The cases ───────────────────────────────────────────────────────────────────

KEY = {"key": "parsel_no"}


def case(title, *steps):
    return {"name": title, "steps": list(steps)}


def one(title, fmt, name, **kw):
    return case(title, table_step(Doc(), fmt, name, **kw))


def bina(title, second, **kw):
    doc = Doc()
    return case(title, table_step(doc, "gpkg", "bina-2024.gpkg"), table_step(doc, "gpkg", second, **kw))


def cases():
    p25 = "parsel-2025.gpkg"
    out = [
        one("anahtarla birleştirme: eşleşen güncellenir, yenisi eklenir; anahtarı boş, katmanda iki nesnede ve alanına "
            "uymayan satır reddedilir", "gpkg", p25, merge_=KEY),
        one("yalnız boşlar doldurulur, geometri yine güncellenir", "gpkg", p25, merge_={**KEY, "matched": "fillEmpty"}),
        one("eşleşene dokunulmaz, yenisi eklenir", "gpkg", p25, merge_={**KEY, "matched": "keep"}),
        one("geometri güncellenmez, öznitelikler güncellenir", "gpkg", p25, merge_={**KEY, "geometry": False}),
        one("eşleşmeyen satır alınmaz", "gpkg", p25, merge_={**KEY, "unmatched": "skip"}),
        one("reddedilen satırlar CSV'ye yazılır", "gpkg", p25, merge_=KEY, rejects="$tmp:parsel-reddedilen.csv"),
        one("plan sayar, reddedilenleri yazmaz", "gpkg", p25, merge_=KEY, rejects="$tmp:plan-reddedilen.csv",
            op="plan"),
        one("validate yalnız denetler", "gpkg", p25, merge_=KEY, op="validate"),
        one("kaynakta yinelenen anahtar içe aktarmayı durdurur", "gpkg", "parsel-yinelenen.gpkg", merge_=KEY),
        one("yinelenen anahtarın ilk satırı alınır, ötekiler reddedilir", "gpkg", "parsel-yinelenen.gpkg",
            merge_={**KEY, "duplicates": "first"}),
        one("yinelenen anahtarın hiçbir satırı alınmaz", "gpkg", "parsel-yinelenen.gpkg",
            merge_={**KEY, "duplicates": "none"}),
        one("anahtar sütunu tabloda yok", "gpkg", p25, merge_={"key": "no"}),
        one("başka katmana yönlendirilir, katmandaki anahtarın adı başka", "gpkg", p25, routes={"parsel": "kadastro"},
            merge_={**KEY, "targetKey": "no"}),
        bina("anahtarsız: aynı tablonun yeni sürümü kalıcı kimlikle güncellenir ve eklenir", "bina-2025.gpkg",
             merge_={}),
        bina("anahtarsız: aynı dosya ikinci kez alınınca değişmeyen dokunulmaz", "bina-2024.gpkg", merge_={}),
        bina("anahtarsız: kalıcı kimlik başka katmanın nesnesinde", "bina-2024.gpkg", merge_={},
             routes={"bina": "cizim"}),
        one("çizgi katmanına alan konamaz; çizgi çoklu çizgiyle güncellenir", "gpkg", "yol-2025.gpkg",
            merge_={"key": "no"}),
        one("nokta, eşleşen alanın yerine geçemez", "gpkg", "parsel-nokta.gpkg", merge_=KEY),
        one("geometri güncellenmeyince noktanın öznitelikleri alana yazılır", "gpkg", "parsel-nokta.gpkg",
            merge_={**KEY, "geometry": False}),
        one("kilitli katmana gidemez", "gpkg", "yapi.gpkg", merge_={"key": "ad"}),
        one("reddedilenlerin yolu göreli", "gpkg", p25, merge_=KEY, rejects="reddedilenler.csv"),
        one("reddedilenlerin klasörü yok", "gpkg", p25, merge_=KEY, rejects="$tmp:yok/reddedilenler.csv"),
        one("reddedilenler .csv değil", "gpkg", p25, merge_=KEY, rejects="$tmp:reddedilenler.txt"),
        one("birleştirmesiz alımda reddedilenler dosyası yalnız başlıktır", "gpkg", "bina-2024.gpkg",
            rejects="$tmp:bina-reddedilen.csv"),
    ]
    for fmt, name, command in (("parquet", "parsel.parquet", "file.import.parquet"),
                               ("csv", "parsel.csv", "file.import.csv")):
        out += [
            one(f"{command}: anahtarla birleştirme", fmt, name, merge_=KEY, command=command),
            one(f"{command}: kaynakta yinelenen anahtar", fmt, name, sub="yinelenen", merge_=KEY, command=command),
            one(f"{command}: anahtar sütunu yok", fmt, name, merge_={"key": "no"}, command=command),
        ]
    out += [
        one("file.import.fgb: anahtarla birleştirme", "fgb", "parsel.fgb", merge_=KEY, command="file.import.fgb"),
        one("file.import.gml: anahtarla birleştirme", "gml", "parsel.gml", merge_=KEY, command="file.import.gml"),
    ]
    gj = "file.import.geojson"
    out += [
        case(f"{gj}: anahtarla birleştirme", gis_step(Doc(), "geojson", "parsel.geojson", merge_=KEY, command=gj)),
        case(f"{gj}: kaynakta yinelenen anahtar",
             gis_step(Doc(), "geojson", "parsel.geojson", sub="yinelenen", merge_=KEY, command=gj)),
        case(f"{gj}: anahtar sütunu yok",
             gis_step(Doc(), "geojson", "parsel.geojson", merge_={"key": "no"}, command=gj)),
        case(f"{gj}: kalıcı kimliği olmayan satırlar anahtarsız birleşemez",
             gis_step(Doc(), "geojson", "parsel.geojson", merge_={}, command=gj)),
        case("file.import.shp: anahtarla birleştirme",
             gis_step(Doc(), "shp", "parsel.shp", merge_=KEY, command="file.import.shp")),
    ]
    return out


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.gpkg", "commandVersion": 1,
        "title": "İçe aktarmada artırımlı yükleme",
        "note": "ADR 0628 §5, §6. Dosyaların satırları içe aktarma durumlarının kendi başvurularından (GDAL'ın OGR'ı, "
                "pyarrow, tools/formats/gis.py); birleştirme, ret nedenleri ve reddedilenlerin CSV'si ADR'nin "
                "kuralıyla burada. Bağımsız başvuru: scripts/fixtures/import_merge_cases.py; elle düzenlemeyin.",
        "setup": SETUP,
        "cases": cases(),
    }


def text(doc):
    return json.dumps(doc, ensure_ascii=False, indent=1) + "\n"


def main():
    if "--files" in sys.argv:
        write_files()
        return
    want = text(build())
    if "--check" in sys.argv:
        have = OUT.read_text(encoding="utf-8") if OUT.exists() else ""
        if have != want:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil: betiği --check'siz koşun")
        print(f"birleştirme durumları güncel: {len(json.loads(want)['cases'])} durum")
        return
    OUT.write_text(want, encoding="utf-8")
    print(f"{len(json.loads(want)['cases'])} durum yazıldı: {OUT}")


if __name__ == "__main__":
    main()
