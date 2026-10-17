#!/usr/bin/env python3
"""The shared cases of the product command file.import.gpkg (docs/adr/0502, plan/gis.md CBS-03).

    /usr/bin/python3 scripts/fixtures/gpkg_import_cases.py --files   # GDAL writes the small GeoPackages beside qgis.gpkg
    /usr/bin/python3 scripts/fixtures/gpkg_import_cases.py           # writes fixtures/commands/v1/file.import.gpkg.json
    /usr/bin/python3 scripts/fixtures/gpkg_import_cases.py --check   # compares

Each file is read with GDAL's OGR, not KentOS code, and turned into the objects ADR 0502 says: a point (Z its
elevation), a polyline (each arc of a circular string its edge's bulge tan(θ/4), the sweep found from the arc's three
points here; Z elevations, M measures), an area (the closing vertex dropped; holes; parts), a record for a row without
geometry or with an empty one; NULL no attribute, a boolean `true`/`false`, a real its shortest text, a UTC time stamp
without its zone, an integer column with a value past 2⁵³ − 1 as text. A row's lasting id is `kentos_uids`'s, else
UUIDv5 of `gpkg:<table>:<fid>` in ADR 0502's namespace (checked here by a later step that names it).
"""
import hashlib
import json
import math
import sys
import uuid
from pathlib import Path

from osgeo import ogr, osr

ogr.UseExceptions()
sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/file.import.gpkg.json"
NAMESPACE = uuid.UUID(bytes=bytes([0x6b, 0x1e, 0x4f, 0x0a, 0x93, 0x27, 0x4c, 0x51, 0x8d, 0x36, 0x2f, 0x7a, 0x10, 0xc4,
                                   0x5e, 0x92]))
MAX_INTEGER = 2 ** 53 - 1
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def write_files():
    """The small GeoPackages: a point in WGS 84, a table without a system, an empty table, a broken file."""
    drv = ogr.GetDriverByName("GPKG")

    def make(name, srs, rows, gtype=ogr.wkbPoint):
        path = DIR / name
        if path.exists():
            path.unlink()
        ds = drv.CreateDataSource(str(path))
        lyr = ds.CreateLayer("yer", srs=srs, geom_type=gtype)
        lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
        for wkt, ad in rows:
            f = ogr.Feature(lyr.GetLayerDefn())
            f.SetGeometry(ogr.CreateGeometryFromWkt(wkt))
            f.SetField("ad", ad)
            lyr.CreateFeature(f)
        ds = None

    wgs = osr.SpatialReference()
    wgs.ImportFromEPSG(4326)
    wgs.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    make("wgs84.gpkg", wgs, [("POINT (32.8541 39.9208)", "Kızılay")])
    make("sistemsiz.gpkg", None, [("POINT (12.5 7.25)", "yerel")])
    make("bos.gpkg", None, [])
    (DIR / "bozuk.gpkg").write_bytes(b"Bu bir GeoPackage degil, duz metin.\n")
    print("yazıldı: wgs84.gpkg, sistemsiz.gpkg, bos.gpkg, bozuk.gpkg")


def P(x, y):
    return {"x": float(x), "y": float(y)}


def sweep(a, m, b):
    """The signed sweep from a through m to b (counter-clockwise positive), from the circle through the three."""
    (ax, ay), (mx, my), (bx, by) = a, m, b
    d = 2 * (ax * (my - by) + mx * (by - ay) + bx * (ay - my))
    ux = ((ax * ax + ay * ay) * (my - by) + (mx * mx + my * my) * (by - ay) + (bx * bx + by * by) * (ay - my)) / d
    uy = ((ax * ax + ay * ay) * (bx - mx) + (mx * mx + my * my) * (ax - bx) + (bx * bx + by * by) * (mx - ax)) / d
    t0, t2 = math.atan2(ay - uy, ax - ux), math.atan2(by - uy, bx - ux)
    ccw = (mx - ax) * (by - ay) - (my - ay) * (bx - ax) > 0
    tau = 2 * math.pi
    return (t2 - t0) % tau if ccw else -((t0 - t2) % tau)


def points(g):
    out = []
    for i in range(g.GetPointCount()):
        p = g.GetPoint(i)
        z = p[2] if g.Is3D() else None
        m = g.GetM(i) if g.IsMeasured() else None
        out.append((p[0], p[1], z, m))
    return out


def walk(g):
    """A curve as vertices (x, y, z, m) and each vertex's edge bulge (the last 0)."""
    name = g.GetGeometryName()
    pts, bul = [], []

    def push(p, before=None):
        if before is not None and bul:
            bul[-1] = before
        if pts and pts[-1][0] == p[0] and pts[-1][1] == p[1]:
            return
        pts.append(p)
        bul.append(0.0)

    def add(c):
        n = c.GetGeometryName()
        if n == "LINESTRING" or n == "LINEARRING":
            for p in points(c):
                push(p, 0.0 if pts else None)
        elif n == "CIRCULARSTRING":
            ps = points(c)
            push(ps[0])
            for i in range(0, len(ps) - 2, 2):
                s = sweep(ps[i][:2], ps[i + 1][:2], ps[i + 2][:2])
                push(ps[i + 2], math.tan(s / 4))
        elif n == "COMPOUNDCURVE":
            for k in range(c.GetGeometryCount()):
                add(c.GetGeometryRef(k))
        else:
            raise ValueError(n)

    add(g)
    return pts, bul, name


def fields_of(pts, bul, measures):
    out = {"pts": [P(p[0], p[1]) for p in pts]}
    if any(b != 0 for b in bul):
        out["bulges"] = bul
    if any(p[2] is not None for p in pts):
        out["zs"] = [p[2] for p in pts]
    if measures and any(p[3] is not None for p in pts):
        out["ms"] = [p[3] for p in pts]
    return out


def ring(g):
    pts, bul, _ = walk(g)
    if len(pts) > 1 and pts[0][:2] == pts[-1][:2]:
        pts, bul = pts[:-1], bul[:-1]
    return pts, bul


def surface(g):
    rings = [ring(g.GetGeometryRef(i)) for i in range(g.GetGeometryCount())]
    out = fields_of(*rings[0], False)
    if len(rings) > 1:
        out["holes"] = [fields_of(*r, False) for r in rings[1:]]
    return out


def geometry(g):
    """The object's kind and geometry fields; none for a record."""
    if g is None or g.IsEmpty():
        return "record", {}
    n = g.GetGeometryName()
    if n == "POINT":
        p = points(g)[0]
        out = {"p": P(p[0], p[1])}
        if p[2] is not None:
            out["z"] = p[2]
        return "point", out
    if n in ("LINESTRING", "CIRCULARSTRING", "COMPOUNDCURVE"):
        pts, bul, _ = walk(g)
        return "polyline", fields_of(pts, bul, True)
    if n in ("POLYGON", "CURVEPOLYGON"):
        return "polygon", surface(g)
    if n in ("MULTIPOLYGON", "MULTISURFACE"):
        parts = [surface(g.GetGeometryRef(i)) for i in range(g.GetGeometryCount())]
        out = dict(parts[0])
        if len(parts) > 1:
            out["parts"] = parts[1:]
        return "polygon", out
    raise ValueError(n)


def text(v, kind, wide):
    if v is None:
        return None
    if kind == "Boolean":
        return "true" if v else "false"
    if kind in ("Integer", "Integer64"):
        return str(v)
    if kind == "Real":
        return repr(float(v))
    if kind == "DateTime":
        return v[:19].replace(" ", "T")
    return v


def read(name):
    """Every table: its name, fields (as layer fields), rows as (fid, uid, kind, fields, attrs), losses."""
    path = DIR / name
    ds = ogr.Open(str(path))
    uids = {}
    if ds.GetLayerByName("kentos_uids") is not None:
        rs = ds.ExecuteSQL("SELECT table_name, fid, uid FROM kentos_uids")
        for r in rs:
            uids[(r.GetField(0), r.GetField(1))] = r.GetField(2)
        ds.ReleaseResultSet(rs)
    tables = []
    for i in range(ds.GetLayerCount()):
        lyr = ds.GetLayerByIndex(i)
        defn = lyr.GetLayerDefn()
        cols = []
        for j in range(defn.GetFieldCount()):
            fd = defn.GetFieldDefn(j)
            kind = "Boolean" if fd.GetSubType() == ogr.OFSTBoolean else fd.GetTypeName()
            cols.append((fd.GetName(), kind, fd.GetWidth()))
        feats = list(lyr)
        wide = {c[0] for j, c in enumerate(cols) if c[1] in ("Integer", "Integer64")
                and any(f.IsFieldSetAndNotNull(j) and abs(f.GetFieldAsInteger64(j)) > MAX_INTEGER for f in feats)}
        fields = []
        for cname, kind, width in cols:
            k = {"String": "text", "Integer": "integer", "Integer64": "integer", "Real": "decimal", "Date": "date",
                 "DateTime": "datetime", "Boolean": "boolean"}[kind]
            if cname in wide:
                k = "text"
            f = {"name": cname, "kind": k}
            if kind == "String" and width:
                f["length"] = width
            fields.append(f)
        losses = [f"“{c}” sütunu 2⁵³'ten büyük sayılar tutuyor: metin alanı olarak, her basamağıyla alındı"
                  for c in sorted(wide)]
        rows = []
        for f in feats:
            attrs = {}
            for j, (cname, kind, _) in enumerate(cols):
                if not f.IsFieldSetAndNotNull(j):
                    continue
                if kind == "Integer64" or (kind == "Integer" and cname in wide):
                    v = f.GetFieldAsInteger64(j)
                elif kind == "Integer" or kind == "Boolean":
                    v = f.GetFieldAsInteger(j)
                elif kind == "Real":
                    v = f.GetFieldAsDouble(j)
                elif kind == "Date":
                    y, mo, d = f.GetFieldAsDateTime(j)[:3]
                    v = f"{y:04d}-{mo:02d}-{d:02d}"
                elif kind == "DateTime":
                    v = f.GetFieldAsISO8601DateTime(j)
                else:
                    v = f.GetFieldAsString(j)
                attrs[cname] = text(v, kind, cname in wide)
            kind, geo = geometry(f.GetGeometryRef())
            fid = f.GetFID()
            uid = uids.get((lyr.GetName(), fid)) or str(uuid.uuid5(NAMESPACE, f"gpkg:{lyr.GetName()}:{fid}"))
            rows.append({"fid": fid, "uid": uid, "kind": kind, "geometry": geo, "attrs": attrs})
        srs = lyr.GetSpatialRef()
        srid = int(srs.GetAuthorityCode(None)) if srs is not None and srs.GetAuthorityName(None) == "EPSG" else None
        tables.append({"name": lyr.GetName(), "fields": fields, "rows": rows, "losses": losses, "srid": srid})
    data = path.read_bytes()
    return {"file": name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "tables": tables}


def slug(name):
    folded = name.translate(str.maketrans("çğıöşüÇĞİÖŞÜ", "cgiosucgiosu")).lower()
    out = ""
    for c in folded:
        if c.isascii() and (c.islower() or c.isdigit()):
            out += c
        elif not out.endswith("-"):
            out += "-"
    return "import-" + (out.strip("-") or "katman")


# The drawing: TUREF / TM36 (EPSG:5256); a layer named “Yol” (the road table goes onto it), a locked layer, a group.
SETUP = {
    "format": "kentos.document", "version": 1, "name": "GeoPackage alma",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis"},
    "origin": P(0, 0),
    "layers": [
        {"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
        {"id": "kilitli", "name": "Kilitli", "type": "layer", "visible": True, "locked": True, "expanded": True,
         "style": STYLE, "children": []},
        {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
    ],
    "activeLayer": "cizim",
    "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {}, "p": P(487500, 4420500)}],
    "styles": {"items": [], "categories": []},
}
SAME = {"ids": [1], "canUndo": False, "dirty": False, "revision": "same"}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def fx(name):
    return f"$fixture:geopackage/v1/{name}"


def imported(name, tables=None, routes=None, start=2, taken=()):
    """The command's answer and the objects it adds (their fields by id), for `tables` (all when none)."""
    r = read(name)
    chosen = [t for t in r["tables"] if tables is None or t["name"] in tables]
    chosen = [t for t in chosen if t["rows"]]
    out_tables, made, fields, ids, losses = [], [], {}, [], set()
    used = set(taken)
    slot = start
    group = slug(name.removesuffix(".gpkg"))
    for t in chosen:
        losses.update(t["losses"])
        layer = (routes or {}).get(t["name"])
        if layer is None:
            layer = slug(t["name"])
            k = 2
            while layer in used:
                layer = f"{slug(t['name'])}-{k}"
                k += 1
            used.add(layer)
            made.append({"id": layer, "name": t["name"]})
        for row in t["rows"]:
            fields[str(slot)] = {"kind": row["kind"], "layerId": layer, "attrs": row["attrs"], **row["geometry"]}
            ids.append(slot)
            slot += 1
        out_tables.append({"table": t["name"], "layer": layer, "read": len(t["rows"]), "objects": len(t["rows"]),
                           "records": sum(1 for x in t["rows"] if x["kind"] == "record"), "skipped": 0})
    srids = sorted({t["srid"] for t in chosen if t["srid"]})
    output = {"file": r["file"], "bytes": r["bytes"], "sha256": r["sha256"], "objects": len(ids),
              "tables": out_tables, "layers": made, "revision": "$current"}
    if made:
        output["group"] = group
    if srids:
        output["srid"] = srids[0]
    warnings = []
    if not srids:
        warnings.append({"code": "crs_undeclared",
                         "message": f"{r['file']} koordinat sistemi bildirmiyor; koordinatlar projenin sisteminde "
                                    "sayılıp olduğu gibi alındı.", "path": "path"})
    if losses:
        warnings.append({"code": "import_losses",
                         "message": f"{r['file']} dosyasının bazı değerleri ya da satırları olduğu gibi alınamadı: "
                                    + "; ".join(sorted(losses)) + ".", "path": "path"})
    return output, warnings, fields, ids, r


def step(inp, result, expect=None, op="execute", command=None):
    s = {"op": op, "input": inp, "result": result, "expect": expect or SAME}
    if command:
        s["command"] = command
    return s


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def take(name, inp_extra=None, tables=None, routes=None, op="execute", expect_layers=False):
    output, warnings, fields, ids, r = imported(name, tables, routes)
    inp = {"path": fx(name), **(inp_extra or {})}
    if op == "plan":
        output = {**output, "revision": "$current"}
        return step(inp, done(output, warnings), SAME, op="plan")
    expect = {"ids": [1, *ids], "fields": fields, "revision": "changed", "canUndo": True}
    return step(inp, done(output, warnings), expect)


def cases():
    q = "qgis.gpkg"
    parsel = read(q)["tables"][0]["rows"]
    return [
        case("QGIS'in yazdığı bütün tablolar: katmanlar alanlarıyla, kayıtlar, eğriler, Z ve M", take(q)),
        case("yalnız parseller: iki parçalı ve delikli alan, geometrisiz satır kayıt", take(q, {"tables": ["parsel"]},
                                                                                         tables=["parsel"])),
        case("yol tablosu var olan katmana yönlendirilir",
             take(q, {"tables": ["yol"], "layers": {"yol": "cizim"}}, tables=["yol"], routes={"yol": "cizim"})),
        case("eğri alan ve birleşik eğri: yaylar bulge olarak", take(q, {"tables": ["egri", "egri_yol"]},
                                                                      tables=["egri", "egri_yol"])),
        case("Z'li noktalar", take(q, {"tables": ["nokta"]}, tables=["nokta"])),
        case("geometrisiz öznitelik tablosu kayıt olur", take(q, {"tables": ["malik"]}, tables=["malik"])),
        case("plan hiçbir şey yazmaz", take(q, {"tables": ["parsel"]}, tables=["parsel"], op="plan")),
        case("validate yalnız denetler", step({"path": fx(q)}, done(None), op="validate")),
        case("aynı dosya ikinci kez alınmaz",
             take(q, {"tables": ["nokta"]}, tables=["nokta"]),
             step({"path": fx(q), "tables": ["nokta"]},
                  failed("entity_exists",
                         f"Dosyadaki bir kaydın kalıcı kimliği ({read(q)['tables'][2]['rows'][0]['uid']}) çizimde zaten "
                         f"bir nesnenin: {q} daha önce alınmış. Önceki alımı geri alın ya da başka bir çizimde açın.",
                         "path"),
                  {"ids": [1, 2], "revision": "same"})),
        case("satırın kalıcı kimliği tablo ve fid'den: ilişki komutu onu bulur",
             take(q, {"tables": ["parsel"]}, tables=["parsel"]),
             step({"a": [parsel[0]["uid"]], "b": [parsel[1]["uid"]], "predicate": "disjoint"},
                  done({"pairs": [{"a": parsel[0]["uid"], "b": parsel[1]["uid"], "matrix": "FF2FF1212"}],
                        "revision": "$current"}),
                  {"ids": [1, 2, 3, 4], "revision": "same"}, command="cad.entities.relate")),
        case("KentOS'un yazdığı dosyanın kentos_uids kimlikleri korunur",
             take("kentos.gpkg", {"tables": ["parsel"]}, tables=["parsel"]),
             step({"a": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7000"], "b": ["0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7001"],
                   "predicate": "disjoint"},
                  done({"pairs": [{"a": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7000",
                                   "b": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7001", "matrix": "FF2FF1212"}],
                        "revision": "$current"}),
                  {"ids": [1, 2, 3, 4], "revision": "same"}, command="cad.entities.relate")),
        case("sistem bildirmeyen dosya olduğu gibi alınır", take("sistemsiz.gpkg")),
        case("WGS 84'teki dosya çevirme istenmeden alınmaz",
             step({"path": fx("wgs84.gpkg")},
                  failed("crs_mismatch", "wgs84.gpkg EPSG:4326 sisteminde, proje EPSG:5256. Koordinatların projenin "
                                         "sistemine çevrilmesi için reproject verin ya da projeyi EPSG:4326'de açın.",
                         "reproject"))),
        case("WGS 84'teki dosya reproject ile projenin sistemine çevrilir",
             step({"path": fx("wgs84.gpkg"), "reproject": True},
                  done({"file": "wgs84.gpkg", "bytes": read("wgs84.gpkg")["bytes"],
                        "sha256": read("wgs84.gpkg")["sha256"], "srid": 4326, "moved": True, "objects": 1,
                        "tables": [{"table": "yer", "layer": "import-yer", "read": 1, "objects": 1, "records": 0,
                                    "skipped": 0}],
                        "layers": [{"id": "import-yer", "name": "yer"}], "group": "import-wgs84",
                        "revision": "$current"}),
                  {"ids": [1, 2], "revision": "changed", "canUndo": True,
                   "fields": {"2": {"kind": "point", "layerId": "import-yer", "attrs": {"ad": "Kızılay"}}}})),
        case("boş tablo: alınacak kayıt yok",
             step({"path": fx("bos.gpkg")},
                  failed("nothing_to_import", "bos.gpkg dosyasının seçilen tablolarında alınacak kayıt yok.", "path"))),
        case("dosyada olmayan tablo",
             step({"path": fx(q), "tables": ["parsel", "bina"]},
                  failed("table_not_found", "“bina” tablosu dosyada yok; dosyanın tablolarından birinin adını verin.",
                         "tables[1]"))),
        case("yönlendirilen katman yok",
             step({"path": fx(q), "tables": ["yol"], "layers": {"yol": "yok"}},
                  failed("layer_not_found", "“yok” kimlikli katman çizimde yok; “yol” tablosunun kayıtlarını var "
                                            "olan bir katmanın kimliğine yönlendirin.", "layers.yol"))),
        case("yönlendirilen düğüm grup",
             step({"path": fx(q), "tables": ["yol"], "layers": {"yol": "grup"}},
                  failed("not_a_layer", "“Grup” bir grup; kayıtlar yalnız katmana konur. Grubun altındaki bir katmanı "
                                        "verin.", "layers.yol"))),
        case("yönlendirilen katman kilitli",
             step({"path": fx(q), "tables": ["yol"], "layers": {"yol": "kilitli"}},
                  failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın ya da başka bir "
                                         "katman verin.", "layers.yol"))),
        case("göreli yol",
             step({"path": "veri/kadastro.gpkg"},
                  failed("invalid_file_path", "“veri/kadastro.gpkg” mutlak bir dosya yolu değil; okunacak dosyanın tam "
                                              "yolunu verin (/klasör/ad.gpkg).", "path"))),
        case("klasör",
             step({"path": "$fixture:geopackage/v1"},
                  failed("invalid_file_path", "$fixture:geopackage/v1 bir klasör; okunacak GeoPackage dosyasının adını "
                                              "da verin.", "path"))),
        case("olmayan dosya",
             step({"path": fx("yok.gpkg")},
                  failed("file_not_found", f"{fx('yok.gpkg')} yok; dosyanın yolunu denetleyin.", "path"))),
        case("GeoPackage olmayan dosya",
             step({"path": fx("bozuk.gpkg")},
                  failed("import_unreadable", f"{fx('bozuk.gpkg')} GeoPackage olarak okunamadı: “{fx('bozuk.gpkg')}” "
                                              "bir SQLite dosyası değil", "path"))),
    ]


def build():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "file.import.gpkg", "commandVersion": 1,
        "title": "GeoPackage içe aktar: tablolar katman, satırlar nesne ve kayıt",
        "note": "ADR 0502. Dosyalar GDAL'ın OGR'ıyla okunur (qgis.gpkg QGIS 4.2.3'ün yazdığı); nesneler ADR'nin kuralıyla "
                "burada kurulur: yay bulge'u üç noktasının çemberinden, halkanın kapanan köşesi düşer, NULL öznitelik "
                "yok, 2⁵³'ü aşan tam sayı metin, UTC zaman diliminsiz. Kalıcı kimlik kentos_uids'ten ya da "
                "UUIDv5(gpkg:tablo:fid). Bağımsız başvuru: scripts/fixtures/gpkg_import_cases.py.",
        "setup": SETUP, "cases": cases(),
    }


def main():
    if "--files" in sys.argv:
        write_files()
        return 0
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"GeoPackage içe aktarma durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
