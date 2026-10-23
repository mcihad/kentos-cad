#!/usr/bin/env python3
"""The shared cases of İşlemler reading its inputs from their source and writing to a database (docs/adr/0592,
plan/gis.md CBS-23).

    python3 scripts/fixtures/processing_source_cases.py           # writes fixtures/commands/v1/cad.processing.run.source.json
    python3 scripts/fixtures/processing_source_cases.py --check   # compares

The source is source_query_cases.py's file (fixtures/sources/v1/sorgu.gpkg, written by GDAL): a point table `nokta`
of 250 rows and a square table `bina` of 12. Its rows are read back here with Python's sqlite3 and the file's SHA-256
with hashlib, not KentOS code. What a run answers is written from ADR 0592's rules:

- an input `{ "scope": "source", "path", "table" }` reads the whole table (the rows a filter and a box leave, by
  ADR 0581 §3's rule) into a layer `kaynak-<parameter>` named as the table; the answer's input says the table as
  `file › table`, the records read and the file's SHA-256;
- `{ "scope": "source", "layer" }` reads a bound layer's whole table in place of the part the drawing holds; that
  layer given by `scope: layer` when it holds a part is said (`partial_scope`);
- a source input needs a sink (the run is made on a copy); the drawing never changes;
- a sink whose path is a database address writes there (a password in it is refused; a schema is for a database
  only, without a dot or a quote); where no PostGIS provider is, the run is refused (`source_unreachable`).

The tool is Nesneden nokta (`convert.toPoints`): one point a record, its summary `n nesneden n nokta yazıldı.`; the
points' places are the squares' centroids in the drawing's system (checked in the file processing_sink.rs writes by
this script's `--written`, with PROJ through GDAL's osr: run it with a Python that has GDAL).
"""
import hashlib
import json
import sqlite3
import struct
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures/sources/v1/sorgu.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.processing.run.source.json"
WRITTEN = ROOT / "fixtures/geopackage/v1/kentos-run-kaynak.gpkg"
FX = "$fixture:sources/v1/sorgu.gpkg"
PLAN = "$fixture:processing/v1/sonuc-plan.gpkg"
ANY = "$any"
SAME = {"revision": "same"}
X0, Y0 = 487000.0, 4420000.0
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


# ── The file, read here ──────────────────────────────────────────────────────

def sha256():
    return hashlib.sha256(FILE.read_bytes()).hexdigest()


def point_of(blob):
    """A GeoPackage point blob's x and y."""
    flags = blob[3]
    env = {0: 0, 1: 32, 2: 48, 3: 48, 4: 64}[(flags >> 1) & 7]
    wkb = blob[8 + env:]
    return struct.unpack("<dd" if wkb[0] == 1 else ">dd", wkb[5:21])


def ring_of(blob):
    """A GeoPackage polygon blob's outer ring, its closing point dropped."""
    flags = blob[3]
    env = {0: 0, 1: 32, 2: 48, 3: 48, 4: 64}[(flags >> 1) & 7]
    wkb = blob[8 + env:]
    e = "<" if wkb[0] == 1 else ">"
    kind = struct.unpack(e + "I", wkb[1:5])[0]
    at = 5
    if kind == 6:  # a multi polygon of one: its polygon
        at += 4 + 1 + 4
    rings = struct.unpack(e + "I", wkb[at:at + 4])[0]
    assert rings >= 1
    n = struct.unpack(e + "I", wkb[at + 4:at + 8])[0]
    pts = [struct.unpack(e + "dd", wkb[at + 8 + 16 * i:at + 24 + 16 * i]) for i in range(n)]
    return pts[:-1] if pts[0] == pts[-1] else pts


def rows(table):
    c = sqlite3.connect(f"file:{FILE}?mode=ro", uri=True)
    if table == "bina":
        out = [{"fid": f, "kat": k, "ad": a, "ring": ring_of(g)}
               for f, k, a, g in c.execute('SELECT fid, kat, ad, geom FROM bina ORDER BY fid')]
    else:
        out = [{"fid": f, "tur": t, "ad": a, "xy": point_of(g)}
               for f, t, a, g in c.execute('SELECT fid, tur, ad, geom FROM nokta ORDER BY fid')]
    c.close()
    return out


def box_of(r):
    pts = r["ring"] if "ring" in r else [r["xy"]]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    return min(xs), min(ys), max(xs), max(ys)


def meets(r, bbox):
    """ADR 0581 §3: a record is in a box when its geometry's box meets it (edges count)."""
    x0, y0, x1, y1 = box_of(r)
    return x0 <= bbox[2] and bbox[0] <= x1 and y0 <= bbox[3] and bbox[1] <= y1


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


def source(table="bina", **kw):
    out = {"scope": "source", "path": FX, "table": table}
    out.update(kw)
    return out


def run_input(inp, sink):
    return {"tool": "convert.toPoints", "parameters": {"input": inp}, "sink": sink}


COLUMNS = {"bina": ["kat", "ad"], "nokta": ["ad", "tur", "kot", "no"]}


def answer(n, the_input, sink_path, *, mode="new", table="Merkezler", columns=None, untouched=()):
    """convert.toPoints's answer with a sink: n points to a new table of the file."""
    # The writer's columns: the result layer's fields (none here), then the attributes the objects carry in their
    # names' order (docs/adr/0508 §4).
    cols = [{"attribute": c, "column": c} for c in sorted(columns or [])]
    out = {
        "tool": "convert.toPoints",
        "summary": f"{n} nesneden {n} nokta yazıldı.",
        "added": [ANY] * n,
        "outputs": {"count": n},
        "inputs": [the_input],
        "record": ANY,
        "revision": "$current",
        "layers": ["islem-merkezler"],
        "sink": {"path": sink_path, "bytes": ANY, "sha256": ANY,
                 "tables": [{"table": table, "layer": "islem-merkezler", "rows": n, "geometry": ANY}],
                 "approximated": 0},
        "sinkReport": {"mode": mode,
                       "tables": [{"table": table, "layer": "islem-merkezler", "rows": n, "geometry": ANY,
                                   "z": ANY, "m": ANY, "columns": cols}],
                       "batches": 1 if n else 0, "batchRows": 5000, "ids": "kentos_uids"},
    }
    if untouched:
        out["sinkReport"]["untouched"] = list(untouched)
    return out


def staged(table, n, layer=None, src=False):
    out = {"layer": layer or "kaynak-input", "digest": ANY, "revision": sha256(),
           "table": f"sorgu.gpkg › {table}", "rows": n, "path": FX}
    if src:
        out["source"] = ANY
    return out


def setup():
    return {"format": "kentos.document", "version": 1, "name": "İşlemler kaynaktan",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False,
                        "expanded": True, "style": STYLE, "children": []}],
            "activeLayer": "cizim",
            "entities": [{"kind": "point", "id": 1, "layerId": "cizim", "attrs": {"tur": "direk"},
                          "p": {"x": X0 + 500, "y": Y0 + 500}}],
            "styles": {"items": [], "categories": []}}


# ── The cases ────────────────────────────────────────────────────────────────

def cases():
    bina, nokta = rows("bina"), rows("nokta")
    out = []
    plan = {"path": PLAN}

    def planned(name, inp, n, table="bina", **kw):
        out.append(case(name, step(run_input(inp, plan),
                                   done(answer(n, staged(table, n), PLAN, columns=COLUMNS[table], **kw)),
                                   op="plan")))

    # 1–2. The whole table, planned and written.
    planned("dosyanın tablosu bütünüyle okunur, sonuç dosyaya: 12 bina, 12 nokta", source(), len(bina))
    out.append(case(
        "yazılan dosya: 12 nokta, çizim değişmez",
        step(run_input(source(), {"path": "$tmp:kaynak.gpkg"}),
             done(answer(len(bina), staged("bina", len(bina)), "$tmp:kaynak.gpkg", columns=COLUMNS["bina"])),
             {"canUndo": False, "dirty": False, "revision": "same"}),
        step({"path": "$tmp:kaynak.gpkg"},
             done({"revision": ANY, "tables": [{"name": "Merkezler", "data": "features", "rows": len(bina), "geometry": "POINT",
                               "srid": 5256, "extent": ANY, "columns": ANY, "capabilities": ANY}]}),
             command="cad.sources.describe")))
    # 3–6. A filter: what passes is what goes.
    for f, keep in [("kat >= 3", lambda r: r["kat"] >= 3),
                    ("kat = 1", lambda r: r["kat"] == 1),
                    ("ad = 'B7'", lambda r: r["ad"] == "B7"),
                    ("kat > 9", lambda r: False)]:
        n = sum(1 for r in bina if keep(r))
        if n:
            planned(f"süzgeçle: {f}, {n} kayıt okunur", source(filter=f), n)
        else:
            # An empty read is an empty layer: the tool's own refusal.
            out.append(case(f"süzgeçle hiçbir kayıt kalmaz ({f}): araç boş katmanı reddeder",
                            step(run_input(source(filter=f), plan),
                                 failed("invalid_parameters",
                                        "“Nesneler”: bu katmanda uygun nesne yok. Başka bir katman seçin.",
                                        "parameters.input"), op="plan")))
    # 7–8. A box, and a box with a filter.
    bbox = [X0 - 1, Y0 + 590, X0 + 95, Y0 + 700]
    n = sum(1 for r in bina if meets(r, bbox))
    planned(f"kutuyla: kutusu kutuya değen {n} bina", source(bbox=bbox), n)
    n = sum(1 for r in bina if meets(r, bbox) and r["kat"] % 2 == 0)
    planned(f"kutu ve süzgeç birlikte: {n} bina", source(bbox=bbox, filter="kat % 2 = 0"), n)
    # 9–10. The point table, with a text filter.
    for t in ["rögar", "ağaç"]:
        n = sum(1 for r in nokta if r["tur"] == t)
        planned(f"nokta tablosu, tur = '{t}': {n} nokta", source("nokta", filter=f"tur = '{t}'"), n, table="nokta")
    # 11. A bound layer read whole in place of its part.
    ext = [X0 - 5, Y0 + 595, X0 + 105, Y0 + 625]
    part = sum(1 for r in bina if meets(r, ext))
    out.append(case(
        f"parçalı bağlı katman kaynağından: {part} değil {len(bina)} kayıt",
        link("bina", ext),
        step(run_input({"scope": "source", "layer": "import-bina"}, plan),
             done(answer(len(bina), staged("bina", len(bina), "import-bina", src=True), PLAN,
                         columns=COLUMNS["bina"])), op="plan")))
    # 12. The same layer by scope layer: the part, said.
    out.append(case(
        f"parçalı bağlı katman scope: layer ile: parçanın {part} kaydı ve partial_scope",
        link("bina", ext),
        step(run_input({"scope": "layer", "layerId": "import-bina"}, plan),
             done(answer(part, {"layer": "import-bina", "digest": ANY, "source": ANY, "revision": ANY}, PLAN,
                         columns=COLUMNS["bina"]),
                  [{"code": "partial_scope",
                    "message": "“bina” katmanı bina tablosunun yalnız bir parçası; araç o parçayla çalıştı. Bütün "
                               "tablo için girdiyi { \"scope\": \"source\", \"layer\": \"import-bina\" } ile verin.",
                    "path": "parameters.input"}]), op="plan")))
    # 13. A bound layer whole, filtered.
    n = sum(1 for r in bina if r["kat"] == 2)
    out.append(case(
        f"bağlı katman kaynağından süzgeçle: kat = 2, {n} kayıt",
        link("bina", ext),
        step(run_input({"scope": "source", "layer": "import-bina", "filter": "kat = 2"}, plan),
             done(answer(n, staged("bina", n, "import-bina", src=True), PLAN, columns=COLUMNS["bina"])),
             op="plan")))
    # Refusals.
    bad = [
        ("kaynaktan okuma sink ister", run_input(source(), None), "invalid_parameters",
         "“input” kaynağından okunuyor: çalıştırma bir kopyada yapılır ve sonucu bir GeoPackage'a ya da PostGIS'e "
         "yazılır; sink verin.", "parameters.input"),
        ("olmayan katman", run_input({"scope": "source", "layer": "yok"}, plan), "layer_not_found",
         "“yok” kimlikli katman çizimde yok.", "parameters.input.layer"),
        ("bağsız katman", run_input({"scope": "source", "layer": "cizim"}, plan), "invalid_input",
         "“Çizim” katmanı bir kaynağa bağlı değil; kaynağından okunamaz (katmanı scope: layer ile verin).",
         "parameters.input.layer"),
        ("katman ve tablo birden", run_input({"scope": "source", "layer": "cizim", "path": FX, "table": "bina"}, plan),
         "invalid_parameters",
         "“input”: kaynak bir bağlı katmanla (layer) ya da bir dosya veya adres ve tabloyla (path, table) verilir, "
         "ikisi birden değil.", "parameters.input"),
        ("tablosuz yol", run_input({"scope": "source", "path": FX}, plan), "invalid_parameters",
         "“input”: kaynak bir bağlı katmanla (layer) ya da bir dosya veya adres ve tabloyla (path, table) verilir, "
         "ikisi birden değil.", "parameters.input"),
        ("üç sayılı kutu", run_input(source(bbox=[0, 0, 1]), plan), "invalid_parameters",
         "“input”: kutu (bbox) dört sonlu sayıdır: batı, güney, doğu, kuzey.", "parameters.input.bbox"),
        ("ters kutu", run_input(source(bbox=[10, 0, 0, 10]), plan), "invalid_extent",
         "Kutu [batı, güney, doğu, kuzey] olmalı: sonlu sayılar, batı doğudan, güney kuzeyden büyük değil.",
         "parameters.input.bbox"),
        ("olmayan tablo", run_input(source("yok"), plan), "table_not_found",
         "“yok” tablosu dosyada yok; dosyanın tablolarından birinin adını verin (cad.sources.describe listeler).",
         "parameters.input.table"),
        ("süzgeçte $sıra", run_input(source(filter="($sıra > 3)"), plan), "invalid_expression",
         "Süzgeç: Süzgeçte $sıra kullanılamaz: süzgeç çalıştırmaya göre değişmemeli.", "parameters.input.filter"),
        ("derlenmeyen süzgeç", run_input(source(filter="kat >"), plan), "invalid_expression", ANY,
         "parameters.input.filter"),
        ("GeoPackage olmayan dosya", run_input(source(), plan) | {"parameters": {"input": {
            "scope": "source", "path": "$fixture:sources/v1/cases.json", "table": "bina"}}}, "import_unreadable", ANY,
         "parameters.input.path"),
        ("parolalı veritabanı adresine yazma", run_input(source(), {"path": "postgresql://kentos:gizli@localhost/vt"}),
         "invalid_address",
         "postgresql://kentos:gizli@localhost/vt: adreste parola var; parola projeye yazılmaz: PGPASSWORD ortam "
         "değişkenine ya da ~/.pgpass dosyasına koyun.", "sink.path"),
        ("GeoPackage'a şema", run_input(source(), {"path": PLAN, "schema": "sonuc"}), "invalid_input",
         "Şema yalnız bir veritabanına yazılırken verilir; GeoPackage'ın şeması yok.", "sink.schema"),
        ("noktalı şema", run_input(source(), {"path": "postgresql://kentos@localhost/vt", "schema": "a.b"}),
         "invalid_input", "“a.b” bir şema adı değil; nokta ve tırnak taşımaz.", "sink.schema"),
        ("veritabanında replaceTables ile overwrite",
         run_input(source(), {"path": "postgresql://kentos@localhost/vt", "mode": "replaceTables",
                              "overwrite": True}),
         "invalid_input",
         "overwrite yeni tabloların aynı adlıların yerine geçmesi içindir; replaceTables bunu zaten yapar.",
         "sink.overwrite"),
        ("ulaşılamayan ya da sağlayıcısı olmayan veritabanına yazma",
         run_input(source(), {"path": "postgresql://kentos@127.0.0.1:1/vt"}), "source_unreachable",
         # Without a provider (the desktop's cases) it says so; with one (Python's) the port refuses the connection.
         ANY, "sink.path"),
    ]
    for name, inp, code, msg, path in bad:
        if inp["sink"] is None:
            del inp["sink"]
        out.append(case(f"ret: {name}", step(inp, failed(code, msg, path), op="plan")))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.processing.run", "commandVersion": 1,
            "title": "İşlemler: girdiyi kaynağından okumak ve sonucu veritabanına yazmak",
            "note": "Üretici: scripts/fixtures/processing_source_cases.py (docs/adr/0592). Kaynağın kayıtları "
                    "sorgu.gpkg'den sqlite3'le; komutun sözleri ADR'den.",
            "setup": setup(), "cases": cases()}


def centroid(ring):
    """A ring's centroid by the shoelace formula, about its first corner (far from the origin the products would
    lose the square's few metres)."""
    ox, oy = ring[0]
    ring = [(x - ox, y - oy) for x, y in ring]
    a = cx = cy = 0.0
    for (x0, y0), (x1, y1) in zip(ring, ring[1:] + ring[:1]):
        w = x0 * y1 - x1 * y0
        a += w
        cx += (x0 + x1) * w
        cy += (y0 + y1) * w
    return ox + cx / (3 * a), oy + cy / (3 * a)


def check_written():
    """The file processing_sink.rs writes (`KENTOS_WRITE_GPKG=1`): its drawing's system is EPSG:5254, the source's
    5256; each square's corners are moved by PROJ (GDAL's osr) and the point is the moved square's centroid, with the
    square's attributes."""
    from osgeo import osr
    osr.UseExceptions()
    src, dst = osr.SpatialReference(), osr.SpatialReference()
    src.ImportFromEPSG(5256)
    dst.ImportFromEPSG(5254)
    for s_ in (src, dst):
        s_.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    move = osr.CoordinateTransformation(src, dst)
    bina = rows("bina")
    c = sqlite3.connect(f"file:{WRITTEN}?mode=ro", uri=True)
    got = list(c.execute('SELECT kat, ad, geom FROM "Merkezler" ORDER BY ad'))
    srs = c.execute("SELECT organization_coordsys_id FROM gpkg_spatial_ref_sys s JOIN gpkg_geometry_columns g "
                    "ON g.srs_id = s.srs_id WHERE g.table_name = 'Merkezler'").fetchone()[0]
    c.close()
    assert srs == 5254, srs
    want = sorted(bina, key=lambda r: r["ad"])
    assert len(got) == len(want), (len(got), len(want))
    worst = 0.0
    for (kat, ad, g), r in zip(got, want):
        cx, cy = centroid([move.TransformPoint(x, y)[:2] for x, y in r["ring"]])
        x, y = point_of(g)
        assert (int(kat), ad) == (r["kat"], r["ad"]), (kat, ad, r)
        worst = max(worst, abs(x - cx), abs(y - cy))
    assert worst < 1e-6, worst
    print(f"{WRITTEN.name}: {len(got)} nokta karelerin EPSG:5254'teki merkezlerinde (en büyük fark {worst:.1e} m), "
          "öznitelikleriyle")


def main():
    if "--written" in sys.argv:
        check_written()
        return
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın.")
        print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
