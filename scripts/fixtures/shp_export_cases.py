#!/usr/bin/env python3
"""The Shapefiles `file.export.shp` writes (docs/adr/0514), read without KentOS code.

    KENTOS_WRITE_SHP=1 cargo test -p kentos-headless --test headless shapefiles_go_out   # writes fixtures/shapefile/v1/out
    python3 scripts/fixtures/shp_export_cases.py           # writes fixtures/shapefile/v1/expected.json
    python3 scripts/fixtures/shp_export_cases.py --check   # checks the files and that expected.json is current

The drawing is QGIS 4.2.3's GeoPackage (fixtures/geopackage/v1/qgis.gpkg) taken in, a triangle on the point layer, a
circle of 25 m on the road layer and a text on the parcel layer (crates/native/headless/tests/headless.rs,
`shapefile_drawing`). The source is read here with GDAL's OGR; the Shapefiles with OGR and, when QGIS 4.2 is
installed, QGIS's own Python. What is checked, by the ADR's rules written here again:

- the sets: one a layer, `_nokta` / `_cizgi` / `_alan` for a layer of several families, a table alone a .dbf;
- the .dbf: dBASE III, dated 1 January 1980, the .cpg's code page and its language driver byte;
- column names: ten bytes in the code page cut at a character's end, without a space at the end, unique whatever
  their case (`_1`, `_2` …); types: text C, integer N without fraction digits, decimal N with at least one, date D,
  yes or no L, date-time C as `YYYY-MM-DDTHH:MM:SS`, an integer past 2⁵³ (a text field once taken in) C;
- values: the source's, empty text read back as none;
- geometry: straight rings and paths vertex for vertex up to the start and the direction, the outline clockwise and
  holes counter-clockwise, Z and M as the source's; curves as chords whose every point and every chord's middle lies
  within 0.1 mm of the curve, and the curve within 0.1 mm of the chords; the circle's vertices on it;
- the .prj names EPSG:5256;
- the Windows-1254 archive reads to the same Turkish values.
"""

import json
import os
import struct
import subprocess
import sys
import unicodedata
from pathlib import Path

from osgeo import ogr, osr

ogr.UseExceptions()
osr.UseExceptions()

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / "fixtures/geopackage/v1/qgis.gpkg"
OUT = ROOT / "fixtures/shapefile/v1/out"
EXPECTED = ROOT / "fixtures/shapefile/v1/expected.json"
QGIS = Path("/Applications/QGIS-final-4_2_2.app/Contents")
TOL = 1e-4

failures = []


def check(ok, what):
    if not ok:
        failures.append(what)


# ── The ADR's rules ──────────────────────────────────────────────────────────────────────────────────────────────


def column_names(attrs, enc="utf-8"):
    """Each attribute's column name: ten bytes cut at a character's end, no space at the end, unique by case."""
    taken, out = set(), []

    def cut(name, room):
        b = b""
        for ch in name:
            try:
                c = ch.encode(enc)
            except UnicodeEncodeError:
                c = b"_"
            if len(b) + len(c) > room:
                break
            b += c
        b = b.rstrip(b" ")
        return b or b"ALAN"

    for a in attrs:
        name = cut(a, 10)
        n = 1
        while name.decode(enc).lower() in taken:
            tail = f"_{n}".encode()
            name = cut(a, 10 - len(tail)) + tail
            n += 1
        taken.add(name.decode(enc).lower())
        out.append(name.decode(enc))
    return out


def ring_key(pts):
    """A closed ring without its closing point, from its least point, both ways round."""
    pts = pts[:-1] if pts and pts[0] == pts[-1] else pts
    i = min(range(len(pts)), key=lambda k: pts[k])
    a = pts[i:] + pts[:i]
    r = list(reversed(pts))
    j = min(range(len(r)), key=lambda k: r[k])
    b = r[j:] + r[:j]
    return min(a, b)


def signed_area(pts):
    return sum(pts[i][0] * pts[i + 1][1] - pts[i + 1][0] * pts[i][1] for i in range(len(pts) - 1)) / 2


def rings_of(g):
    """A (multi)polygon's rings, part by part, each its points (x, y)."""
    parts = []
    t = ogr.GT_Flatten(g.GetGeometryType())
    polys = [g.GetGeometryRef(i) for i in range(g.GetGeometryCount())] if t == ogr.wkbMultiPolygon else [g]
    for p in polys:
        parts.append([[p.GetGeometryRef(k).GetPoint_2D(i) for i in range(p.GetGeometryRef(k).GetPointCount())]
                      for k in range(p.GetGeometryCount())])
    return parts


def within(out_g, src_g, what):
    """Every vertex and chord middle of `out_g` within TOL of the curve `src_g`, and the curve within TOL of it."""
    dense = src_g.GetLinearGeometry(0.01)
    bounds = dense.GetBoundary() if ogr.GT_Flatten(dense.GetGeometryType()) in (ogr.wkbPolygon, ogr.wkbMultiPolygon) else dense
    lines = out_g.GetBoundary() if ogr.GT_Flatten(out_g.GetGeometryType()) in (ogr.wkbPolygon, ogr.wkbMultiPolygon) else out_g
    far = 0.0
    for line in ([lines] if lines.GetGeometryCount() == 0 else [lines.GetGeometryRef(i) for i in range(lines.GetGeometryCount())]):
        pts = [line.GetPoint_2D(i) for i in range(line.GetPointCount())]
        for a, b in zip(pts, pts[1:]):
            for q in (a, ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)):
                p = ogr.Geometry(ogr.wkbPoint)
                p.AddPoint_2D(*q)
                far = max(far, p.Distance(bounds))
    for line in ([bounds] if bounds.GetGeometryCount() == 0 else [bounds.GetGeometryRef(i) for i in range(bounds.GetGeometryCount())]):
        for i in range(line.GetPointCount()):
            p = ogr.Geometry(ogr.wkbPoint)
            p.AddPoint_2D(*line.GetPoint_2D(i))
            far = max(far, p.Distance(lines))
    check(far <= TOL + 1e-7, f"{what}: kirişler eğriden {far:.6g} m uzak (en çok {TOL})")
    return far


# ── Reading ──────────────────────────────────────────────────────────────────────────────────────────────────────


def dbf_header(path):
    b = path.read_bytes()
    return {"version": b[0], "date": list(b[1:4]), "driver": b[29]}


def read_set(path):
    ds = ogr.Open(str(path))
    layer = ds.GetLayer(0)
    d = layer.GetLayerDefn()
    fields = [{"name": d.GetFieldDefn(i).GetName(), "type": d.GetFieldDefn(i).GetTypeName(),
               "subtype": ogr.GetFieldSubTypeName(d.GetFieldDefn(i).GetSubType()),
               "width": d.GetFieldDefn(i).GetWidth(), "precision": d.GetFieldDefn(i).GetPrecision()}
              for i in range(d.GetFieldCount())]
    rows = []
    for f in layer:
        g = f.GetGeometryRef()
        rows.append({"values": [f.GetField(i) for i in range(d.GetFieldCount())],
                     "geometry": g.Clone() if g is not None else None})
    return {"type": ogr.GeometryTypeToName(layer.GetGeomType()), "fields": fields, "rows": rows,
            "srs": layer.GetSpatialRef()}


def source():
    ds = ogr.Open(str(SRC))
    out = {}
    for i in range(ds.GetLayerCount()):
        layer = ds.GetLayer(i)
        d = layer.GetLayerDefn()
        names = [d.GetFieldDefn(j).GetName() for j in range(d.GetFieldCount())]
        rows = []
        for f in layer:
            g = f.GetGeometryRef()
            rows.append({"values": dict(zip(names, [f.GetField(j) for j in range(len(names))])),
                         "geometry": g.Clone() if g is not None and not g.IsEmpty() else None})
        out[layer.GetName()] = {"fields": names,
                                "types": [d.GetFieldDefn(j).GetTypeName() for j in range(d.GetFieldCount())],
                                "rows": rows}
    return out


def expected_value(kind, value, name, src_types):
    """What OGR reads back of a source value in its column."""
    if value is None or value == "":
        return None
    if kind == "Integer64" and abs(value) > 2 ** 53 - 1:
        return str(value)
    if kind == "Integer64" and name == "kimlik":
        # The column holds 9007199254740993: the whole column was taken in as text (docs/adr/0502).
        return str(value)
    if kind == "DateTime":
        # 2024/03/01 10:20:30+00 → the drawing's local date-time without its zone, as text.
        date, time = value.split(" ")
        return f"{date.replace('/', '-')}T{time[:8]}"
    return value


# ── The checks ───────────────────────────────────────────────────────────────────────────────────────────────────


def main():
    src = source()
    files = sorted(p.name for p in OUT.iterdir())
    sets = sorted({p.split(".")[0] for p in files if not p.endswith(".zip")})
    want_sets = ["egri", "egri_yol", "malik", "nokta_alan", "nokta_nokta", "parsel", "yol"]
    check(sets == want_sets, f"Shapefile'lar {sets}, beklenen {want_sets}")
    check("malik.shp" not in files and "malik.dbf" in files, "geometrisiz katman yalnız .dbf olmalı")
    summary = {"format": "kentos.shp-export-cases/1", "note": "ADR 0514. OGR'ın okuduğu; betik KentOS kodu kullanmaz.",
               "sets": {}}
    for name in sets:
        path = OUT / (f"{name}.shp" if (OUT / f"{name}.shp").exists() else f"{name}.dbf")
        got = read_set(path)
        h = dbf_header(OUT / f"{name}.dbf")
        check(h == {"version": 3, "date": [80, 1, 1], "driver": 0}, f"{name}.dbf başlığı {h}")
        check((OUT / f"{name}.cpg").read_text() == "UTF-8", f"{name}.cpg UTF-8 değil")
        summary["sets"][name] = {"type": got["type"], "rows": len(got["rows"]), "fields": got["fields"]}
        if (OUT / f"{name}.prj").exists():
            srs = osr.SpatialReference()
            srs.ImportFromWkt((OUT / f"{name}.prj").read_text())
            matches = [m.GetAuthorityCode(None) for m, conf in srs.FindMatches() if conf >= 70]
            check("5256" in matches, f"{name}.prj EPSG:5256 olarak tanınmadı: {matches}")
        layer = name.split("_nokta")[0].split("_alan")[0]
        if layer in src:
            s = src[layer]
            want_names = column_names(s["fields"] + (["Mülkiyet Durumu", "mülkiyet durumu tarihi"] if layer == "nokta" and name == "nokta_alan" else []))
            got_names = [f["name"] for f in got["fields"]]
            check(got_names == want_names, f"{name}: sütunlar {got_names}, beklenen {want_names}")
            kinds = dict(zip(s["fields"], s["types"]))
            for f, k in zip(got["fields"], s["fields"]):
                t = kinds[k]
                want = {"String": "String", "Integer": "Integer", "Real": "Real", "Date": "Date",
                        "DateTime": "String", "Integer64": "String"}[t]
                if t == "Integer" and k == "tapulu":
                    want = "Integer"  # L: OGR reads yes or no as an Integer of the Boolean subtype
                    check(f["subtype"] == "Boolean", f"{name}.{k}: evet/hayır L değil ({f})")
                check(f["type"] == want, f"{name}.{k}: tür {f['type']}, beklenen {want}")
                if t == "Real":
                    check(f["precision"] >= 1, f"{name}.{k}: ondalık sütunun kesir basamağı yok")
            if name in ("nokta_nokta", "parsel", "yol", "egri", "egri_yol", "malik"):
                geometric = [r for r in s["rows"] if r["geometry"] is not None]
                bare = [r for r in s["rows"] if r["geometry"] is None]
                ordered = geometric + bare if name != "malik" else s["rows"]
                for i, r in enumerate(ordered):
                    if i >= len(got["rows"]):
                        check(False, f"{name}: satır {i} yok")
                        continue
                    o = got["rows"][i]
                    for j, k in enumerate(s["fields"]):
                        want = expected_value(kinds[k], r["values"][k], k, kinds)
                        have = o["values"][j]
                        if kinds[k] == "Date" and have is not None:
                            have = have.replace("-", "/")
                        check(have == want, f"{name}[{i}].{k}: {have!r}, beklenen {want!r}")
                    sg, og = r["geometry"], o["geometry"]
                    if sg is None:
                        check(og is None, f"{name}[{i}]: geometrisiz satır boş şekil olmalı")
                        continue
                    if og is None:
                        check(False, f"{name}[{i}]: geometri yok")
                        continue
                    if sg.HasCurveGeometry():
                        within(og, sg, f"{name}[{i}]")
                        continue
                    t = ogr.GT_Flatten(sg.GetGeometryType())
                    if t in (ogr.wkbPolygon, ogr.wkbMultiPolygon):
                        sr, orr = rings_of(sg), rings_of(og)
                        check(len(sr) == len(orr), f"{name}[{i}]: parça sayısı {len(orr)}, beklenen {len(sr)}")
                        for a, b in zip(sr, orr):
                            check([ring_key(x) for x in a] == [ring_key(y) for y in b], f"{name}[{i}]: halkalar farklı")
                            check(signed_area(b[0]) < 0, f"{name}[{i}]: dış sınır saat yönünde değil")
                            check(all(signed_area(h) > 0 for h in b[1:]), f"{name}[{i}]: delik saat yönünün tersine değil")
                        check(og.IsValid(), f"{name}[{i}]: geçersiz alan")
                    else:
                        a, b = sg.ExportToIsoWkt(), og.ExportToIsoWkt()
                        check(a == b, f"{name}[{i}]: {b}, beklenen {a}")
    # The triangle on the point layer, clockwise; the circle on the road layer, its vertices on it.
    tri = read_set(OUT / "nokta_alan.shp")["rows"][0]
    pts = [tri["geometry"].GetGeometryRef(0).GetPoint_2D(i) for i in range(4)]
    check(ring_key(pts) == ring_key([(487400, 4420000), (487410, 4420000), (487410, 4420010), (487400, 4420000)]),
          f"üçgen {pts}")
    check(signed_area(pts) < 0, "üçgen saat yönünde değil")
    check(tri["values"][2:] == ["hisseli", "2026-10-19"], f"üçgenin uzun adlı öznitelikleri {tri['values']}")
    circle = read_set(OUT / "yol.shp")["rows"][2]["geometry"]
    cps = [circle.GetPoint(i) for i in range(circle.GetPointCount())]
    import math
    radii = [math.hypot(x - 487500, y - 4420000) for x, y, *_ in cps]
    check(all(abs(r - 25) < 1e-9 for r in radii), "dairenin köşeleri daire üstünde değil")
    steps = [abs(math.atan2(b[1] - 4420000, b[0] - 487500) - math.atan2(a[1] - 4420000, a[0] - 487500)) % (2 * math.pi)
             for a, b in zip(cps, cps[1:])]
    sag = max(25 * (1 - math.cos(min(s, 2 * math.pi - s) / 2)) for s in steps)
    check(sag <= TOL + 1e-12, f"dairenin kirişleri {sag} m sapıyor")
    check(cps[0][:2] == cps[-1][:2], "daire kapanmıyor")
    # The Windows-1254 archive.
    z = f"/vsizip/{OUT / 'teslim-1254.zip'}"
    parcels = read_set(f"{z}/parsel.shp")
    check(parcels["rows"][1]["values"][0] == "Işık-Ğüşöç", f"1254 arşivinde ad {parcels['rows'][1]['values'][0]!r}")
    ds = ogr.Open(f"{z}/parsel.dbf")
    raw = ogr.Open(z)
    del ds, raw
    zf = __import__("zipfile").ZipFile(OUT / "teslim-1254.zip")
    head = zf.read("parsel.dbf")
    check(head[29] == 0xCA and zf.read("parsel.cpg") == b"1254", "1254 arşivinin sürücü baytı ya da .cpg'si yanlış")
    summary["qgis"] = qgis_counts(sets)
    return summary


def qgis_counts(sets):
    """What QGIS 4.2 reads: features, the system and the column names; none when QGIS is not installed."""
    py = QGIS / "MacOS/python3.12"
    if not py.exists():
        print("QGIS 4.2 yok; QGIS denetimi atlandı")
        return None
    code = f"""
import json
from qgis.core import QgsApplication, QgsVectorLayer
QgsApplication.setPrefixPath('{QGIS}/MacOS', True)
app = QgsApplication([], False); app.initQgis()
out = {{}}
for n in {sets!r}:
    p = '{OUT}/' + n + ('.shp' if n != 'malik' else '.dbf')
    l = QgsVectorLayer(p, n, 'ogr')
    out[n] = [l.isValid(), l.featureCount(), l.crs().authid(), [f.name() for f in l.fields()]]
print(json.dumps(out, ensure_ascii=False))
"""
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen", PYTHONHOME=str(QGIS / "Frameworks"),
               PYTHONPATH=f"{QGIS}/Resources/qgis/python:{QGIS}/Frameworks/lib/python3.12/site-packages",
               PROJ_DATA=str(QGIS / "Resources/qgis/proj"))
    r = subprocess.run([str(py), "-c", code], env=env, capture_output=True, text=True, timeout=120)
    line = [x for x in r.stdout.splitlines() if x.startswith("{")]
    if not line:
        check(False, f"QGIS okuyamadı: {r.stderr[-400:]}")
        return None
    got = json.loads(line[-1])
    for n, (valid, count, crs, names) in got.items():
        check(valid, f"QGIS {n}'yi açmadı")
        if n != "malik":
            check(crs == "EPSG:5256", f"QGIS {n}'nin sistemini {crs} okudu")
    return got


if __name__ == "__main__":
    summary = main()
    text = json.dumps(summary, ensure_ascii=False, indent=1, sort_keys=True) + "\n"
    if "--check" in sys.argv:
        if not EXPECTED.exists() or EXPECTED.read_text() != text:
            failures.append(f"{EXPECTED.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun")
    else:
        EXPECTED.write_text(text)
    for f in failures:
        print("HATA:", f)
    if failures:
        sys.exit(1)
    print(f"{len(summary['sets'])} Shapefile OGR'la denetlendi" + (", QGIS 4.2'yle de" if summary.get("qgis") else ""))
