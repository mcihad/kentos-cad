#!/usr/bin/env python3
"""The shared cases of the product command cad.sources.query (docs/adr/0581, plan/gis.md CBS-02).

    python3 scripts/fixtures/source_query_cases.py --files   # GDAL writes fixtures/sources/v1/sorgu.gpkg
    python3 scripts/fixtures/source_query_cases.py           # writes fixtures/commands/v1/cad.sources.query.json
    python3 scripts/fixtures/source_query_cases.py --check   # compares

The file is written here with GDAL's OGR: a point table with a spatial index (`nokta`, 250 rows), an area table without
one (`bina`), a point table in WGS 84 (`wgs`) and an attribute table (`kayit`); one number column then holds a text
(sqlite3), as a GeoPackage may. Its rows are read back with Python's sqlite3, not KentOS code: an integer is its digits,
a real Python's shortest text, a text itself, NULL no attribute (ADR 0502). A row's lasting id is UUIDv5 of
`gpkg:<table>:<fid>` in ADR 0502's namespace. Each case's condition is written here by hand from the language's rules
(an empty value compares false, a number-like text is a number, `arasında` inclusive, `boş` NULL or empty); what the
source is sent and how many rows it hands over follow ADR 0581 §3 (the superset rule: an order or an equality window
on a number column keeps a text stored there, a text equality only a text literal that is not a number, `ve` drops what
cannot be sent). The statistics are kentos.statistics/1 by spatial_query_cases.py's exact reference. The WGS 84 points
are moved to the project's system (EPSG:5256) by GDAL's osr; the box is half a step away from every point.
"""
import copy
import json
import math
import sqlite3
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import spatial_query_cases as sq  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures/sources/v1/sorgu.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.sources.query.json"
FX = "$fixture:sources/v1/sorgu.gpkg"
NAMESPACE = uuid.UUID(bytes=bytes([0x6b, 0x1e, 0x4f, 0x0a, 0x93, 0x27, 0x4c, 0x51, 0x8d, 0x36, 0x2f, 0x7a, 0x10, 0xc4,
                                   0x5e, 0x92]))
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
ANY = "$any"
SAME = {"revision": "same"}
CHUNK = 1000
X0, Y0 = 487000.0, 4420000.0
TYPES = ["direk", "rögar", "ağaç", None, ""]


def nokta_rows():
    out = []
    for fid in range(1, 251):
        i = fid - 1
        kot = None if fid % 17 == 0 else 100 + fid * 0.25
        out.append({"fid": fid, "x": X0 + (i % 25) * 20, "y": Y0 + (i // 25) * 20, "ad": f"N{fid}",
                    "tur": TYPES[i % 5], "kot": kot, "no": fid * 3 % 101})
    return out


def bina_rows():
    out = []
    for k in range(12):
        side = 6 + k
        x, y = X0 + k * 30, Y0 + 600
        out.append({"fid": k + 1, "ring": [(x, y), (x + side, y), (x + side, y + side), (x, y + side)],
                    "kat": k % 4 + 1, "ad": f"B{k + 1}"})
    return out


def kayit_rows():
    return [{"fid": k, "ad": f"K{k}", "tutar": None if k % 7 == 0 else round(k * 12.5 - 3.75, 2)} for k in range(1, 31)]


def write_files():
    from osgeo import ogr, osr
    osr.UseExceptions()
    ogr.UseExceptions()
    FILE.parent.mkdir(parents=True, exist_ok=True)
    if FILE.exists():
        FILE.unlink()
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(FILE))
    tm = osr.SpatialReference()
    tm.ImportFromEPSG(5256)
    geo = osr.SpatialReference()
    geo.ImportFromEPSG(4326)
    geo.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    tm.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)

    lyr = ds.CreateLayer("nokta", tm, ogr.wkbPoint, ["FID=fid"])
    for name, t in (("ad", ogr.OFTString), ("tur", ogr.OFTString), ("kot", ogr.OFTReal), ("no", ogr.OFTInteger)):
        lyr.CreateField(ogr.FieldDefn(name, t))
    for r in nokta_rows():
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(r["fid"])
        for k in ("ad", "tur", "kot", "no"):
            if r[k] is not None:
                f.SetField(k, r[k])
        g = ogr.Geometry(ogr.wkbPoint)
        g.AddPoint_2D(r["x"], r["y"])
        f.SetGeometry(g)
        lyr.CreateFeature(f)

    lyr = ds.CreateLayer("bina", tm, ogr.wkbPolygon, ["FID=fid", "SPATIAL_INDEX=NO"])
    lyr.CreateField(ogr.FieldDefn("kat", ogr.OFTInteger))
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for r in bina_rows():
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(r["fid"])
        f.SetField("kat", r["kat"])
        f.SetField("ad", r["ad"])
        ring = ogr.Geometry(ogr.wkbLinearRing)
        for p in r["ring"] + [r["ring"][0]]:
            ring.AddPoint_2D(*p)
        g = ogr.Geometry(ogr.wkbPolygon)
        g.AddGeometry(ring)
        f.SetGeometry(g)
        lyr.CreateFeature(f)

    lyr = ds.CreateLayer("wgs", geo, ogr.wkbPoint, ["FID=fid"])
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    to_geo = osr.CoordinateTransformation(tm, geo)
    for fid in range(1, 21):
        i = fid - 1
        x, y, _ = to_geo.TransformPoint(X0 + (i % 5) * 100, Y0 + 1000 + (i // 5) * 100)
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(fid)
        f.SetField("ad", f"W{fid}")
        g = ogr.Geometry(ogr.wkbPoint)
        g.AddPoint_2D(x, y)
        f.SetGeometry(g)
        lyr.CreateFeature(f)

    lyr = ds.CreateLayer("kayit", None, ogr.wkbNone, ["FID=fid"])
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    lyr.CreateField(ogr.FieldDefn("tutar", ogr.OFTReal))
    for r in kayit_rows():
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(r["fid"])
        f.SetField("ad", r["ad"])
        if r["tutar"] is not None:
            f.SetField("tutar", r["tutar"])
        lyr.CreateFeature(f)
    lyr = ds.CreateLayer("bozuk", tm, ogr.wkbPoint, ["FID=fid"])
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for fid in range(1, 4):
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(fid)
        f.SetField("ad", f"Z{fid}")
        g = ogr.Geometry(ogr.wkbPoint)
        g.AddPoint_2D(X0 + fid * 10, Y0 - 100)
        f.SetGeometry(g)
        lyr.CreateFeature(f)
    nz = osr.SpatialReference()
    nz.ImportFromEPSG(2193)
    nz.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    lyr = ds.CreateLayer("nz", nz, ogr.wkbPoint, ["FID=fid"])
    f = ogr.Feature(lyr.GetLayerDefn())
    f.SetFID(1)
    g = ogr.Geometry(ogr.wkbPoint)
    g.AddPoint_2D(1750000, 5900000)
    f.SetGeometry(g)
    lyr.CreateFeature(f)
    # A geometry that is not one (row 2 of bozuk).
    ds.ExecuteSQL("UPDATE bozuk SET geom = X'47500003' WHERE fid = 2")
    # A text in a number column (SQLite's REAL affinity keeps a text that does not read as a number); through
    # GDAL's own connection, whose triggers need its SQL functions.
    ds.ExecuteSQL("UPDATE nokta SET kot = 'ölçülmedi' WHERE fid = 60")
    ds = None


# ── Reading the file back ────────────────────────────────────────────────────

def text(v):
    if v is None:
        return None
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, int):
        return str(v)
    if isinstance(v, float):
        return sq_num(v)
    return v


def rows_of(table, columns):
    c = sqlite3.connect(f"file:{FILE}?mode=ro", uri=True)
    out = []
    for r in c.execute(f'SELECT fid, {", ".join(chr(34) + k + chr(34) for k in columns)} FROM "{table}" ORDER BY fid'):
        attrs = {k: text(v) for k, v in zip(columns, r[1:]) if v is not None}
        out.append({"fid": r[0], "uid": str(uuid.uuid5(NAMESPACE, f"gpkg:{table}:{r[0]}")), "attrs": attrs})
    c.close()
    return out


def positions():
    """Each table's rows' places in the project's system: nokta and bina as written, wgs moved back by osr."""
    from osgeo import osr
    osr.UseExceptions()
    tm = osr.SpatialReference()
    tm.ImportFromEPSG(5256)
    geo = osr.SpatialReference()
    geo.ImportFromEPSG(4326)
    for s in (tm, geo):
        s.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    c = sqlite3.connect(f"file:{FILE}?mode=ro", uri=True)
    back = osr.CoordinateTransformation(geo, tm)
    out = {"nokta": {r["fid"]: (r["x"], r["y"]) for r in nokta_rows()}, "wgs": {}}
    import struct
    for fid, blob in c.execute('SELECT fid, geom FROM "wgs" ORDER BY fid'):
        flags = blob[3]
        env = {0: 0, 1: 32, 2: 48, 3: 48, 4: 64}[(flags >> 1) & 7]
        wkb = blob[8 + env:]
        x, y = struct.unpack("<dd" if wkb[0] == 1 else ">dd", wkb[5:21])
        px, py, _ = back.TransformPoint(x, y)
        out["wgs"][fid] = (px, py)
    c.close()
    return out


# ── The language's rules, by hand ────────────────────────────────────────────

def num(v):
    """A value as the language reads a number; None: empty or not a number."""
    if v is None or v.strip() == "":
        return None
    try:
        x = float(v.strip())
    except ValueError:
        return None
    return x if math.isfinite(x) else None


def order(v, x):
    """The sign of `v` against the number `x` as the language orders them; None: `v` is empty. A text that does not
    read as a number is ordered as text against the number's text: in Turkish order a letter comes after every digit
    (the one such text here, 'ölçülmedi', starts with one)."""
    if empty(v):
        return None
    n = num(v)
    if n is not None:
        return (n > x) - (n < x)
    assert v[0].isalpha(), v
    return 1


def gt(v, x):
    o = order(v, x)
    return o is not None and o > 0


def ge(v, x):
    o = order(v, x)
    return o is not None and o >= 0


def lt(v, x):
    o = order(v, x)
    return o is not None and o < 0


def between(v, lo, hi):
    return ge(v, lo) and (lambda o: o is not None and o <= 0)(order(v, hi))


def eq_num(v, x):
    n = num(v)
    return n is not None and abs(n - x) <= 1e-9 * max(1.0, abs(n), abs(x))


def empty(v):
    return v is None or v == ""


# ── Building the answers ─────────────────────────────────────────────────────

def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, expect=None, command=None, op="execute"):
    s = {"op": op, "input": inp, "result": result, "expect": expect or SAME}
    if command:
        s["command"] = command
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def link(table, extent=None):
    edit = {"kind": "link", "path": FX, "table": table}
    if extent:
        edit["extent"] = extent
    return step({"edit": edit}, {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"},
                "cad.sources.edit")


COLUMNS = {"nokta": ["ad", "tur", "kot", "no"], "bina": ["kat", "ad"], "wgs": ["ad"], "kayit": ["ad", "tutar"],
           "bozuk": ["ad"]}
SRID = {"nokta": 5256, "bina": 5256, "wgs": 4326, "kayit": None, "bozuk": 5256}


def answer(table, *, keep=lambda r: True, pushed_keep=lambda r: True, fields=None, limit=100, after=None,
           count=False, statistics=(), box=None, plan_box=None, pushed=None, exact=True, local=(), geometry=False,
           wkt=None, layer=None, kind="geopackage", rows=None, order="key", partial=False, read=None,
           reads_shape=False):
    """The answer of a question whose rows `keep` keeps, of which the source hands over `pushed_keep`'s."""
    all_rows = rows if rows is not None else rows_of(table, COLUMNS[table])
    if kind == "drawing":
        all_rows = sorted(all_rows, key=lambda r: r["uid"])
        start = [r for r in all_rows if after is None or r["uid"] > after]
        handed = start
    else:
        start = [r for r in all_rows if after is None or r["fid"] > after]
        handed = [r for r in start if pushed_keep(r)]
    matched = [r for r in handed if keep(r)]
    page = matched[:limit]
    more = len(matched) > limit and limit > 0
    out_rows = []
    for r in page:
        attrs = {k: v for k, v in r["attrs"].items() if fields is None or k in fields}
        row = {"uid": r["uid"], "attrs": attrs}
        if kind != "drawing":
            row["key"] = r["fid"]
        if geometry:
            row["wkt"] = wkt(r)
        out_rows.append(row)
    nxt = None
    if more:
        last = page[-1]
        nxt = f"u:{last['uid']}" if kind == "drawing" else f"k:{last['fid']}"
    # What the source hands over: everything until the page and its next are known, unless counting.
    if read is None:
        if kind == "drawing":
            read = len(handed) if (count or statistics or not more) else _drawing_read(handed, keep, limit)
        else:
            read = len(handed) if (count or statistics or not more) else _chunked(handed, keep, limit)
    source = {"kind": kind}
    if layer:
        source["layer"] = layer
    if kind == "drawing":
        cols, shape = [], False
        source["srid"] = 5256
    else:
        wanted = set(_FILTER_COLUMNS) | set(statistics)
        cols = [c for c in COLUMNS[table] if fields is None or c in fields or c in wanted]
        shape = geometry or reads_shape or "box" in local or "near" in local
        source["path"] = FX
        source["table"] = table
        if SRID[table]:
            source["srid"] = SRID[table]
    plan = {"columns": cols, "geometry": shape, "exact": exact, "local": list(local), "order": order}
    if plan_box:
        plan["box"] = plan_box
    if pushed:
        plan["pushed"] = pushed
    out = {"source": source, "rows": out_rows, "plan": plan, "partial": partial, "read": read,
           "stamp": ANY if kind != "drawing" else None, "revision": "$current"}
    if out["stamp"] is None:
        del out["stamp"]
    if nxt:
        out["next"] = nxt
    if count or statistics:
        out["total"] = len(matched)
    if statistics:
        out["statistics"] = []
        for f in statistics:
            fig = sq.figures([r["attrs"].get(f) for r in matched if f in r["attrs"]])
            st = {"field": f, "read": fig["read"], "skipped": fig["skipped"]}
            for k in ("sum", "mean", "min", "max", "std"):
                if fig[k] is not None:
                    st[k] = fig[k]
            out["statistics"].append(st)
    return out


_FILTER_COLUMNS = []


def _chunked(handed, keep, limit):
    """Rows handed over in parts of CHUNK until the part holding the (limit + 1)-th match."""
    seen = 0
    for k, r in enumerate(handed):
        if keep(r):
            seen += 1
            if seen == limit + 1:
                return min(len(handed), (k // CHUNK + 1) * CHUNK)
    return len(handed)


def _drawing_read(handed, keep, limit):
    return _chunked(handed, keep, limit)


def with_columns(reads):
    global _FILTER_COLUMNS
    _FILTER_COLUMNS = list(reads)


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Kaynağı sorgula",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False,
                        "expanded": True, "style": STYLE, "children": []}],
            "activeLayer": "cizim",
            "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {"tur": "direk", "kot": "12.5"},
                          "p": {"x": 487500.0, "y": 4420500.0}}],
            "styles": {"items": [], "categories": []}}


def q(**kw):
    out = {"path": FX, "table": kw.pop("table", "nokta")}
    out.update(kw)
    return out


def cases():
    pos = positions()
    nk = lambda r: pos["nokta"][r["fid"]]  # noqa: E731
    out = []
    a = lambda r, k: r["attrs"].get(k)  # noqa: E731

    # 1. Every column, the first page.
    with_columns([])
    out.append(case("dosyanın tablosu: ilk 100 kayıt anahtar sırasıyla, sonraki sayfanın imleci",
                    step(q(), done(answer("nokta")))))
    # 2. The next page from the cursor.
    out.append(case("imleçten sonraki sayfa: 101. kayıttan",
                    step(q(cursor="k:100"), done(answer("nokta", after=100)))))
    # 3. The last page has no next.
    out.append(case("son sayfa: sonraki imleç yok",
                    step(q(cursor="k:200"), done(answer("nokta", after=200)))))
    # 4. Some columns only.
    out.append(case("yalnız istenen sütunlar okunur ve verilir",
                    step(q(fields=["ad", "no"], limit=5), done(answer("nokta", fields=["ad", "no"], limit=5)))))
    # 5. A number order sent to the source (a text kept by the window).
    with_columns(["kot"])
    out.append(case("sayı sütununda sıralama kaynağa gider; metin tutan satır da gelir, burada düşer",
                    step(q(filter="kot > 150", fields=["ad"], count=True),
                         done(answer("nokta", keep=lambda r: gt(a(r, "kot"), 150), fields=["ad"], count=True,
                                     pushed_keep=lambda r: gt(a(r, "kot"), 150) or a(r, "kot") == "ölçülmedi",
                                     pushed="(\"kot\" > 150 OR typeof(\"kot\") = 'text')", exact=False, local=["filter"])))))
    # 6. Text equality sent exactly.
    with_columns(["tur"])
    out.append(case("metin eşitliği kaynağa tam gider",
                    step(q(filter="tur = 'rögar'", fields=["ad"], count=True),
                         done(answer("nokta", keep=lambda r: a(r, "tur") == "rögar", fields=["ad"], count=True,
                                     pushed_keep=lambda r: a(r, "tur") == "rögar",
                                     pushed="\"tur\" = 'rögar'", local=["filter"])))))
    # 7. Number equality as a window, not exact.
    with_columns(["no"])
    lo, hi = 50 - 1e-8 * 50, 50 + 1e-8 * 50
    out.append(case("sayı eşitliği kaynağa pencere olarak gider, tam değil",
                    step(q(filter="no = 50", fields=["ad", "no"], limit=10),
                         done(answer("nokta", keep=lambda r: eq_num(a(r, "no"), 50), fields=["ad", "no"], limit=10,
                                     pushed_keep=lambda r: r["attrs"].get("no") is not None
                                     and lo <= float(r["attrs"]["no"]) <= hi,
                                     pushed=f"(\"no\" BETWEEN {sq_num(lo)} AND {sq_num(hi)} OR typeof(\"no\") = 'text')",
                                     exact=False, local=["filter"])))))
    # 8. `ve` with a part that cannot be sent.
    with_columns(["kot", "ad"])
    out.append(case("ve'nin gönderilemeyen yanı kalır, gönderilebilen kaynağa gider",
                    step(q(filter="kot >= 160 and ad gibi '%5'", fields=["ad"], count=True),
                         done(answer("nokta", keep=lambda r: ge(a(r, "kot"), 160) and a(r, "ad").endswith("5"), fields=["ad"], count=True,
                                     pushed_keep=lambda r: ge(a(r, "kot"), 160) or a(r, "kot") == "ölçülmedi",
                                     pushed="(\"kot\" >= 160 OR typeof(\"kot\") = 'text')", exact=False,
                                     local=["filter"])))))
    # 9. `veya` with a side that cannot be sent: nothing sent.
    out.append(case("veya'nın bir yanı gönderilemezse hiçbir şey gönderilmez",
                    step(q(filter="kot >= 160 or ad gibi '%5'", fields=["ad"], count=True, limit=3),
                         done(answer("nokta", keep=lambda r: ge(a(r, "kot"), 160) or a(r, "ad").endswith("5"), fields=["ad"], count=True, limit=3,
                                     exact=False, local=["filter"])))))
    # 10. `boş` on a text column.
    with_columns(["tur"])
    out.append(case("boş: NULL ya da boş metin, kaynağa tam gider",
                    step(q(filter="tur boş", fields=["ad"], count=True, limit=4),
                         done(answer("nokta", keep=lambda r: empty(a(r, "tur")), fields=["ad"], count=True, limit=4,
                                     pushed_keep=lambda r: empty(a(r, "tur")),
                                     pushed="(\"tur\" IS NULL OR \"tur\" = '')", local=["filter"])))))
    # 11. `içinde` of texts.
    out.append(case("içinde: metin eşitliklerinin veyası",
                    step(q(filter="tur içinde ('direk', 'ağaç')", fields=["tur"], count=True, limit=2),
                         done(answer("nokta", keep=lambda r: a(r, "tur") in ("direk", "ağaç"), fields=["tur"],
                                     count=True, limit=2, pushed_keep=lambda r: a(r, "tur") in ("direk", "ağaç"),
                                     pushed="(\"tur\" = 'direk' OR \"tur\" = 'ağaç')", local=["filter"])))))
    # 12. `arasında` on a number column.
    with_columns(["kot"])
    out.append(case("arasında: sayı sütununda kaynağa gider, uçlar dahil",
                    step(q(filter="kot arasında 110 ve 112", fields=["kot"]),
                         done(answer("nokta", keep=lambda r: between(a(r, "kot"), 110, 112), fields=["kot"],
                                     pushed_keep=lambda r: between(a(r, "kot"), 110, 112) or a(r, "kot") == "ölçülmedi",
                                     pushed="(\"kot\" BETWEEN 110 AND 112 OR typeof(\"kot\") = 'text')",
                                     exact=False, local=["filter"])))))
    # 13. A number compared with a text that reads as one.
    out.append(case("sayı gibi okunan metin sayı olarak gider",
                    step(q(filter="kot < '101'", fields=["kot"]),
                         done(answer("nokta", keep=lambda r: lt(a(r, "kot"), 101), fields=["kot"],
                                     pushed_keep=lambda r: lt(a(r, "kot"), 101) or a(r, "kot") == "ölçülmedi",
                                     pushed="(\"kot\" < 101 OR typeof(\"kot\") = 'text')", exact=False, local=["filter"])))))
    # 14. A box through the spatial index.
    box = [X0 + 90, Y0 + 30, X0 + 150, Y0 + 70]
    inbox = lambda r: box[0] <= nk(r)[0] <= box[2] and box[1] <= nk(r)[1] <= box[3]  # noqa: E731
    with_columns([])
    out.append(case("kutu kaynağın mekânsal dizininden",
                    step(q(bbox=box, fields=["ad"]),
                         done(answer("nokta", keep=inbox, pushed_keep=inbox, fields=["ad"], plan_box="index",
                                     local=["box"])))))
    # 15. Geometry out.
    out.append(case("geometri WKT olarak verilir",
                    step(q(bbox=box, fields=["ad"], geometry=True, limit=3),
                         done(answer("nokta", keep=inbox, pushed_keep=inbox, fields=["ad"], plan_box="index",
                                     local=["box"], geometry=True, limit=3,
                                     wkt=lambda r: f"POINT ({wkt_num(nk(r)[0])} {wkt_num(nk(r)[1])})")))))
    # 16. A place with a tolerance.
    at = [X0 + 100, Y0 + 100]

    def near(r, t=25.0):
        x, y = nk(r)
        return math.hypot(x - at[0], y - at[1]) <= t
    nbox = [at[0] - 25, at[1] - 25, at[0] + 25, at[1] + 25]
    out.append(case("yer ve tolerans: kutusu kaynağa, uzaklığı burada",
                    step(q(near={"at": at, "tolerance": 25.0}, fields=["ad"]),
                         done(answer("nokta", keep=near, fields=["ad"], plan_box="index", local=["near"],
                                     pushed_keep=lambda r: nbox[0] <= nk(r)[0] <= nbox[2]
                                     and nbox[1] <= nk(r)[1] <= nbox[3])))))
    # 17. The box on a table without an index: every row is handed over, its geometry read.
    bb = [X0 + 55, Y0 + 595, X0 + 125, Y0 + 640]
    brows = bina_rows()

    def bin_box(r):
        ring = brows[r["fid"] - 1]["ring"]
        xs, ys = [p[0] for p in ring], [p[1] for p in ring]
        return max(xs) >= bb[0] and min(xs) <= bb[2] and max(ys) >= bb[1] and min(ys) <= bb[3]
    out.append(case("dizinsiz tabloda kutu: her satır gelir, geometrisiyle süzülür",
                    step(q(table="bina", bbox=bb),
                         done(answer("bina", keep=bin_box, plan_box="geometry", local=["box"],
                                     read=len(brows))))))
    # 18. A filter that reads the geometry.
    with_columns([])
    side = lambda r: 6 + r["fid"] - 1  # noqa: E731
    out.append(case("geometriyi okuyan süzgeç: kaynağa bir şey gitmez, şekiller burada",
                    step(q(table="bina", filter="($alan > 150)", count=True, fields=["ad"]),
                         done(answer("bina", keep=lambda r: side(r) ** 2 > 150, count=True, fields=["ad"],
                                     exact=False, local=["filter"], reads_shape=True)))))
    # 19. Statistics, kentos.statistics/1.
    with_columns(["tutar"])
    out.append(case("istatistik: kesin toplam, ortalama, en az, en çok, sapma",
                    step(q(table="kayit", statistics=["tutar"], limit=0),
                         done(answer("kayit", statistics=["tutar"], limit=0, local=["statistics"])))))
    # 20. Statistics with a filter.
    out.append(case("süzgeçli istatistik ve sayı",
                    step(q(table="kayit", filter="tutar > 100", statistics=["tutar"], limit=2, fields=["ad"]),
                         done(answer("kayit", keep=lambda r: gt(a(r, "tutar"), 100), statistics=["tutar"], limit=2,
                                     fields=["ad"], pushed_keep=lambda r: gt(a(r, "tutar"), 100),
                                     pushed="(\"tutar\" > 100 OR typeof(\"tutar\") = 'text')", exact=False,
                                     local=["filter", "statistics"])))))
    # 21. A table in WGS 84 asked with a box in the project's system.
    wb = [X0 + 150, Y0 + 1050, X0 + 350, Y0 + 1250]
    win = lambda r: wb[0] <= pos["wgs"][r["fid"]][0] <= wb[2] and wb[1] <= pos["wgs"][r["fid"]][1] <= wb[3]  # noqa
    with_columns([])
    out.append(case("başka sistemdeki tablo: kutu tablonun sistemine çevrilip gider, satırlar projenin sisteminde denetlenir",
                    step(q(table="wgs", bbox=wb, count=True),
                         done(answer("wgs", keep=win, count=True, plan_box="index", local=["box"], read=ANY)))))
    # 22. A layer of the drawing.
    me = [{"fid": None, "uid": "$uidOf:1", "attrs": {"tur": "direk", "kot": "12.5"}}]
    out.append(case("çizimin katmanı: nesneleri kalıcı kimlik sırasıyla",
                    step({"layer": "cizim"},
                         done(answer("cizim", kind="drawing", rows=me, layer="cizim", order="uid", local=[])))))
    # 23. A bound layer with its part, loaded scope.
    ext = [X0 - 5, Y0 - 5, X0 + 105, Y0 + 45]
    part = [r for r in rows_of("nokta", COLUMNS["nokta"])
            if ext[0] <= pos["nokta"][r["fid"]][0] <= ext[2] and ext[1] <= pos["nokta"][r["fid"]][1] <= ext[3]]
    with_columns(["tur"])
    out.append(case("parçalı bağlı katman, yüklenen kapsam: parçanın nesneleri ve partial_scope",
                    link("nokta", ext),
                    step({"layer": "import-nokta", "filter": "tur = 'direk'", "fields": ["ad"]},
                         done(answer("nokta", kind="drawing", rows=part, layer="import-nokta", order="uid",
                                     keep=lambda r: a(r, "tur") == "direk", fields=["ad"], exact=False,
                                     local=["filter"], partial=True),
                              [{"code": "partial_scope",
                                "message": "“nokta” katmanı kaynağının yalnız bir parçası; yanıt o parçanın. Bütün "
                                           "tablo için kapsamı kaynak yapın (scope: source).", "path": "scope"}]))))
    # 24. The same with scope source.
    out.append(case("parçalı bağlı katman, kaynak kapsamı: bütün tablo",
                    link("nokta", ext),
                    step({"layer": "import-nokta", "scope": "source", "filter": "tur = 'direk'", "fields": ["ad"],
                          "count": True},
                         done(answer("nokta", layer="import-nokta", keep=lambda r: a(r, "tur") == "direk",
                                     pushed_keep=lambda r: a(r, "tur") == "direk", fields=["ad"], count=True,
                                     pushed="\"tur\" = 'direk'", local=["filter"])))))
    # 25. A row whose geometry does not read: left out and said.
    zrows = [r for r in rows_of("bozuk", ["ad"])]
    out.append(case("geometrisi okunamayan satır atlanır ve söylenir",
                    step(q(table="bozuk", geometry=True),
                         done(answer("bozuk", rows=zrows, keep=lambda r: r["fid"] != 2, geometry=True,
                                     wkt=lambda r: f"POINT ({wkt_num(X0 + r['fid'] * 10)} {wkt_num(Y0 - 100)})"),
                              [{"code": "rows_unreadable",
                                "message": "“bozuk” tablosunun 1 kaydının geometrisi okunamadı ya da projenin "
                                           "sistemine çevrilemedi; atlandı.", "path": "table"}]))))
    # Refusals.
    bad = [
        ("ne katman ne dosya", {}, "invalid_input",
         "Bir katman (layer) ya da bir dosya veya adres ve tablo (path, table) verin; ikisi birden ya da hiçbiri olmaz.",
         "layer"),
        ("katman ve dosya birden", {"layer": "cizim", "path": FX, "table": "nokta"}, "invalid_input",
         "Bir katman (layer) ya da bir dosya veya adres ve tablo (path, table) verin; ikisi birden ya da hiçbiri olmaz.",
         "layer"),
        ("dosyasız tablo", {"table": "nokta"}, "invalid_input",
         "Tablo (table) bir dosya ya da adresle (path) verilir.", "table"),
        ("tablosuz dosya", {"path": FX}, "invalid_input", "Dosyanın ya da veritabanının tablosunu (table) verin.",
         "table"),
        ("dosyayla kapsam", q(scope="source"), "invalid_input", "Kapsam (scope) bir katmanla (layer) verilir.",
         "scope"),
        ("sayfa sınırı aşılır", q(limit=10001), "invalid_count",
         "Bir sayfada en çok 10000 kayıt okunur; 10001 verildi.", "limit"),
        ("ters kutu", q(bbox=[10, 0, 0, 10]), "invalid_extent",
         "Kutu [batı, güney, doğu, kuzey] olmalı: sonlu sayılar, batı doğudan, güney kuzeyden büyük değil.", "bbox"),
        ("eksi tolerans", q(near={"at": [0, 0], "tolerance": -1}), "invalid_tolerance",
         "Tolerans sıfır ya da sıfırdan büyük, sonlu bir uzaklık olmalı.", "near.tolerance"),
        ("bozuk imleç", q(cursor="k:01"), "invalid_cursor",
         "“k:01” bu sorunun sayfa imleci değil; bir önceki cevabın next'ini verin.", "cursor"),
        ("çizimin imleci kaynağa", q(cursor="u:00000000-0000-0000-0000-000000000000"), "invalid_cursor",
         "Bu imleç çizimin; bir kaynağın kayıtları anahtarlarıyla sayfalanır.", "cursor"),
        ("kaynağın imleci çizime", {"layer": "cizim", "cursor": "k:5"}, "invalid_cursor",
         "Bu imleç bir kaynağın; çizimin nesneleri kalıcı kimlikleriyle sayfalanır.", "cursor"),
        ("derlenmeyen süzgeç", q(filter="kot >"), "invalid_expression", ANY, "filter"),
        ("süzgeçte $sıra", q(filter="($sıra > 3)"), "invalid_expression",
         "Süzgeç: Süzgeçte $sıra kullanılamaz: süzgeç çalıştırmaya göre değişmemeli.", "filter"),
        ("olmayan katman", {"layer": "yok"}, "layer_not_found",
         "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"),
        ("bağsız katmanda kaynak kapsamı", {"layer": "cizim", "scope": "source"}, "invalid_input",
         "“Çizim” katmanı bir kaynağa bağlı değil; kapsamı kaynak yapılamaz.", "scope"),
        ("olmayan tablo", q(table="yok"), "table_not_found",
         "“yok” tablosu dosyada yok; dosyanın tablolarından birinin adını verin (cad.sources.describe listeler).",
         "table"),
        ("olmayan sütun", q(fields=["ad", "yok"]), "field_not_found", "“yok” sütunu “nokta” tablosunda yok.",
         "fields.1"),
        ("istatistikte olmayan sütun", q(statistics=["yok"]), "field_not_found",
         "“yok” sütunu “nokta” tablosunda yok.", "statistics.0"),
    ]
    for name, inp, code, msg, path in bad:
        out.append(case(f"ret: {name}", step(inp, failed(code, msg, path))))
    # Whatever path the runner makes, the code and the place.
    for name, inp, code, path in [
        ("dosya GeoPackage değil", {"path": "$fixture:sources/v1/cases.json", "table": "nokta"}, "import_unreadable",
         "path"),
        ("parolalı adres", {"path": "postgresql://kentos:gizli@localhost/vt", "table": "nokta"}, "invalid_address",
         "path"),
        ("ulaşılamayan veritabanı", {"path": "postgresql://kentos@127.0.0.1:1/vt", "table": "public.nokta"},
         "source_unreachable", "path"),
        ("projenin sistemine çevrilemeyen tablo", q(table="nz", bbox=[0, 0, 1, 1]), "crs_mismatch", "table"),
    ]:
        out.append(case(f"ret: {name}", step(inp, {"status": "failed", "error": {"code": code, "message": ANY,
                                                                                  "path": path}})))
    # A missing file: the command's own sözü, whatever path the runner makes.
    out.append(case("ret: olmayan dosya", step({"path": "$fixture:sources/v1/yok.gpkg", "table": "nokta"},
                                                {"status": "failed", "error": {"code": "file_not_found",
                                                                               "message": ANY, "path": "path"}})))
    return out


def sq_num(x):
    """A number as the plan writes it (the language's shortest text)."""
    t = repr(float(x))
    return t[:-2] if t.endswith(".0") else t


def wkt_num(v):
    return str(int(v)) if v == int(v) and abs(v) < 1e15 else repr(v)


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.query", "commandVersion": 1,
            "title": "Kaynağı sorgula: kutu, sütunlar ve süzgeç kaynağa, satırlar burada denetlenir",
            "note": "ADR 0581. Dosyayı GDAL yazar (--files); satırlar sqlite3 ile dosyanın kendisinden okunur, kalıcı "
                    "kimlikler ADR 0502'nin UUIDv5'i, süzgeçler dilin kurallarıyla elle, gönderilen SQL ve kaynağın "
                    "verdiği satır sayısı ADR 0581 §3'ün kapsayan kuralıyla, istatistik kentos.statistics/1 "
                    "(spatial_query_cases.py). WGS 84 noktaları osr'la projenin sistemine çevrilir; kutu her noktadan "
                    "yarım adım uzakta. Bağımsız başvuru: scripts/fixtures/source_query_cases.py.",
            "setup": setup(), "cases": cases()}


def main():
    if "--files" in sys.argv:
        write_files()
        return
    data = build()
    text_out = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text_out:
            print(f"{OUT} güncel değil", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.name}: {len(data['cases'])} durum")
        return
    OUT.write_text(text_out)
    print(f"{OUT.name}: {len(data['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
