#!/usr/bin/env python3
"""The shared cases of a batch whose results go to a file or a database, its rows reading their inputs from their
source (docs/adr/0593, plan/gis.md CBS-24).

    python3 scripts/fixtures/processing_batch_sink_cases.py           # writes fixtures/commands/v1/cad.processing.batch.sink.json
    python3 scripts/fixtures/processing_batch_sink_cases.py --check   # compares

The source is source_query_cases.py's file (fixtures/sources/v1/sorgu.gpkg, written by GDAL), read here with Python's
sqlite3 as processing_source_cases.py reads it (its helpers are taken from there). What a batch answers is written
from ADR 0593's rules:

- with a sink the rows run one after another on a copy of the drawing; the drawing never changes, the answer has no
  undo step;
- the batch's own input read from its source (`parameters`) is read once into a layer `kaynak-<parameter>`; a row's
  own into `kaynak-<parameter>-<row number>`;
- a layer the rows made is one table named as the layer (rows giving the same output layer fill one table); the
  record says which row made a table when one row did;
- a row that stops takes the batch back (`batch_row_failed`), or with `onError: continue` only itself; a refusal of
  the sink or of the batch's own source input refuses the batch with its own code;
- the record keeps the source inputs as given, each read input's file, table, records and SHA-256, and where the
  result went.

The tool is Nesneden nokta (`convert.toPoints`): one point a record, its summary `n nesneden n nokta yazıldı.`.
"""
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from processing_source_cases import (  # noqa: E402
    ANY, COLUMNS, FX, X0, Y0, case, done, failed, link, meets, rows, setup, sha256, step,
)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.processing.batch.sink.json"
PLAN = "$fixture:processing/v1/sonuc-plan.gpkg"
TOOL = "convert.toPoints"
FOLD = str.maketrans("çğıİöşüÇĞÖŞÜ", "cgiiosuCGOSU")


def layer_id(name):
    """ADR 0217's rule for a new layer's id: the folded name's letters and digits, “-” for every other run."""
    slug, gap = "", False
    for c in name.translate(FOLD).lower():
        if c.isascii() and (c.isalpha() or c.isdigit()):
            slug += c
            gap = False
        elif not gap:
            slug += "-"
            gap = True
    return f"islem-{slug}"


def source(table="bina", **kw):
    out = {"scope": "source", "path": FX, "table": table}
    out.update(kw)
    return out


def staged(table, n, layer):
    return {"layer": layer, "digest": ANY, "revision": sha256(), "table": f"sorgu.gpkg › {table}", "rows": n,
            "path": FX}


def ran(n, the_input, layers):
    """A row's answer on the copy: n points, no undo step."""
    out = {"tool": TOOL, "summary": f"{n} nesneden {n} nokta yazıldı.", "added": [ANY] * n,
           "outputs": {"count": n}, "inputs": [the_input], "record": ANY, "revision": "$current"}
    if layers:
        out["layers"] = layers
    return out


def batched(rows_ran, tables, path, *, failed_rows=(), record=ANY, mode="new", untouched=(), summary=None):
    """The batch's answer: `rows_ran` [(index, label, n, input, made layer or None)], `tables` [(name, layer id, n,
    columns)] in the order they are written."""
    objects = sum(r[2] for r in rows_ran)
    if summary is None:
        place = "sonuc-plan.gpkg" if path == PLAN else path.split(":", 1)[1]
        summary = f"{len(rows_ran)} satır çalıştı: {objects} nesne {place} dosyasına {len(tables)} tabloya yazıldı."
        if failed_rows:
            names = ", ".join(f["label"] for f in failed_rows)
            summary += f" {len(failed_rows)} satır çalışmadı ve geri alındı ({names})."
    out = {
        "tool": TOOL,
        "summary": summary,
        "rows": [{"index": i, "label": label, "ran": ran(n, inp, [lay] if lay else [])}
                 for i, label, n, inp, lay in rows_ran],
        "sink": {"path": path, "bytes": ANY, "sha256": ANY,
                 "tables": [{"table": t, "layer": lay, "rows": n, "geometry": ANY} for t, lay, n, _ in tables],
                 "approximated": 0},
        "sinkReport": {"mode": mode,
                       "tables": [{"table": t, "layer": lay, "rows": n, "geometry": ANY, "z": ANY, "m": ANY,
                                   "columns": [{"attribute": c, "column": c} for c in sorted(cols)]}
                                  for t, lay, n, cols in tables],
                       "batches": sum(math.ceil(n / 5000) for _, _, n, _ in tables), "batchRows": 5000,
                       "ids": "kentos_uids"},
        "record": record,
        "revision": "$current",
    }
    if untouched:
        out["sinkReport"]["untouched"] = list(untouched)
    if failed_rows:
        out["failed"] = list(failed_rows)
    return out


def batch(rows_, sink, parameters=None, on_error=None):
    out = {"tool": TOOL, "rows": rows_, "sink": sink}
    if parameters:
        out["parameters"] = parameters
    if on_error:
        out["onError"] = on_error
    if sink is None:
        del out["sink"]
    return out


def row(inp=None, layer=None, label=None):
    p = {}
    if inp is not None:
        p["input"] = inp
    if layer is not None:
        p["layer"] = {"newName": layer}
    out = {"parameters": p} if p else {}
    if label:
        out["label"] = label
    return out


UNTOUCHED = ["nokta", "bina", "wgs", "kayit", "bozuk", "nz"]
SAME = {"canUndo": False, "dirty": False, "revision": "same"}


def cases():
    bina, nokta = rows("bina"), rows("nokta")
    cols = COLUMNS["bina"]
    out = []
    low = [r for r in bina if r["kat"] <= 2]
    high = [r for r in bina if r["kat"] >= 3]

    def split_rows(sink_path, op="plan", record=ANY):
        b = batch([row(source(filter="kat <= 2"), "Alçak"), row(source(filter="kat >= 3"), "Yüksek")],
                  {"path": sink_path})
        a = batched([(0, "Satır 1", len(low), staged("bina", len(low), "kaynak-input-1"), layer_id("Alçak")),
                     (1, "Satır 2", len(high), staged("bina", len(high), "kaynak-input-2"), layer_id("Yüksek"))],
                    [("Alçak", layer_id("Alçak"), len(low), cols), ("Yüksek", layer_id("Yüksek"), len(high), cols)],
                    sink_path, record=record)
        return b, a

    # 1. Two rows, each its own source and layer, planned.
    b, a = split_rows(PLAN)
    out.append(case(f"iki satır, her biri kendi süzgeciyle kaynaktan: {len(low)} ve {len(high)} nokta, iki tablo",
                    step(b, done(a), op="plan")))
    # 2. Written: the file has the two tables, the drawing does not change; the record says where.
    record = {
        "format": "kentos.run-record", "version": 1, "kind": "batch", "tool": TOOL, "label": "Nesneden nokta",
        "toolVersion": 1, "parameters": {},
        "inputs": [staged("bina", len(low), "kaynak-input-1"), staged("bina", len(high), "kaynak-input-2")],
        "documentRevision": ANY, "srid": 5256, "engine": ANY, "host": ANY, "startedAt": ANY, "ms": ANY,
        "steps": [{"id": f"rows[{i}]", "tool": TOOL, "label": f"Satır {i + 1}", "recordsIn": n, "recordsOut": n,
                   "ms": ANY, "summary": f"{n} nesneden {n} nokta yazıldı."}
                  for i, n in enumerate([len(low), len(high)])],
        "total": {"recordsIn": len(bina), "recordsOut": len(bina)},
        "sink": {"path": "$tmp:toplu-yazilan.gpkg", "mode": "new",
                 "tables": [{"table": "Alçak", "of": layer_id("Alçak"), "row": "rows[0]", "rows": len(low)},
                            {"table": "Yüksek", "of": layer_id("Yüksek"), "row": "rows[1]", "rows": len(high)}],
                 "bytes": ANY, "sha256": ANY},
    }
    b, a = split_rows("$tmp:toplu-yazilan.gpkg", record=record)
    out.append(case(
        "yazılan dosya iki tabloyla, kayıtta yeri ve satırları; çizim değişmez",
        step(b, done(a), SAME),
        step({"path": "$tmp:toplu-yazilan.gpkg"},
             done({"revision": ANY, "tables": [
                 {"name": t, "data": "features", "rows": n, "geometry": "POINT", "srid": 5256, "extent": ANY, "columns": ANY,
                  "capabilities": ANY} for t, n in [("Alçak", len(low)), ("Yüksek", len(high))]]}),
             command="cad.sources.describe")))
    # 3. The batch's own source input, read once; the rows differ in where the point goes and in their layer.
    n = len(bina)
    common = staged("bina", n, "kaynak-input")
    out.append(case(
        "ortak kaynak bir kez okunur; satırlar konumla ve katmanla ayrılır",
        step(batch([row(layer="Merkez") | {"parameters": {"layer": {"newName": "Merkez"}, "location": "centroid"}},
                    {"parameters": {"layer": {"newName": "İç nokta"}, "location": "inside"}}],
                   {"path": PLAN}, {"input": source()}),
             done(batched([(0, "Satır 1", n, common, layer_id("Merkez")),
                           (1, "Satır 2", n, common, layer_id("İç nokta"))],
                          [("Merkez", layer_id("Merkez"), n, cols), ("İç nokta", layer_id("İç nokta"), n, cols)],
                          PLAN)), op="plan")))
    # 4. Rows writing to one layer: one table holds them all (the record names no row for it).
    out.append(case(
        "aynı çıktı katmanına yazan satırlar tek tabloyu doldurur",
        step(batch([row(source(filter="kat <= 2")), row(source(filter="kat >= 3"))], {"path": PLAN}),
             done(batched([(0, "Satır 1", len(low), staged("bina", len(low), "kaynak-input-1"),
                            layer_id("Merkezler")),
                           (1, "Satır 2", len(high), staged("bina", len(high), "kaynak-input-2"), None)],
                          [("Merkezler", layer_id("Merkezler"), len(bina), cols)], PLAN)), op="plan")))
    # 5. Named rows, three point kinds.
    kinds = ["rögar", "ağaç", "direk"]
    counts = [sum(1 for r in nokta if r["tur"] == t) for t in kinds]
    labels = ["Rögarlar", "Ağaçlar", "Direkler"]
    rs = [row(source("nokta", filter=f"tur = '{t}'"), lab, lab) for t, lab in zip(kinds, labels)]
    out.append(case(
        "üç adlı satır, nokta türleri: " + ", ".join(f"{t} {c}" for t, c in zip(kinds, counts)),
        step(batch(rs, {"path": PLAN}),
             done(batched([(i, lab, c, staged("nokta", c, f"kaynak-input-{i + 1}"), layer_id(lab))
                           for i, (lab, c) in enumerate(zip(labels, counts))],
                          [(lab, layer_id(lab), c, COLUMNS["nokta"]) for lab, c in zip(labels, counts)], PLAN)),
             op="plan")))
    # 6. A box on one row, a filter on another.
    bbox = [X0 - 1, Y0 + 590, X0 + 95, Y0 + 700]
    nb = sum(1 for r in bina if meets(r, bbox))
    n2 = sum(1 for r in bina if r["kat"] == 2)
    out.append(case(
        f"bir satır kutuyla ({nb}), öbürü süzgeçle ({n2})",
        step(batch([row(source(bbox=bbox), "Kutu"), row(source(filter="kat = 2"), "İki kat")], {"path": PLAN}),
             done(batched([(0, "Satır 1", nb, staged("bina", nb, "kaynak-input-1"), layer_id("Kutu")),
                           (1, "Satır 2", n2, staged("bina", n2, "kaynak-input-2"), layer_id("İki kat"))],
                          [("Kutu", layer_id("Kutu"), nb, cols), ("İki kat", layer_id("İki kat"), n2, cols)],
                          PLAN)), op="plan")))
    # 7. A bound layer read whole as the batch's input.
    ext = [X0 - 5, Y0 + 595, X0 + 105, Y0 + 625]
    part = sum(1 for r in bina if meets(r, ext))
    whole = {"layer": "import-bina", "digest": ANY, "source": ANY, "revision": sha256(),
             "table": "sorgu.gpkg › bina", "rows": n, "path": FX}
    out.append(case(
        f"parçalı bağlı katman bütün tablosuyla okunur: {part} değil {n} kayıt",
        link("bina", ext),
        step(batch([row(layer="Bir"), row(layer="İki")], {"path": PLAN},
                   {"input": {"scope": "source", "layer": "import-bina"}}),
             done(batched([(0, "Satır 1", n, whole, layer_id("Bir")), (1, "Satır 2", n, whole, layer_id("İki"))],
                          [("Bir", layer_id("Bir"), n, cols), ("İki", layer_id("İki"), n, cols)], PLAN)),
             op="plan")))
    # 8. The same layer by scope layer: the part, said once.
    bound = {"layer": "import-bina", "digest": ANY, "source": ANY, "revision": ANY}
    msg = ("“bina” katmanı bina tablosunun yalnız bir parçası; araç o parçayla çalıştı. Bütün tablo için girdiyi "
           "{ \"scope\": \"source\", \"layer\": \"import-bina\" } ile verin.")
    out.append(case(
        f"parçalı bağlı katman scope: layer ile: her satır parçanın {part} kaydıyla, partial_scope bir kez",
        link("bina", ext),
        step(batch([row(layer="Bir"), row(layer="İki")], {"path": PLAN},
                   {"input": {"scope": "layer", "layerId": "import-bina"}}),
             done(batched([(0, "Satır 1", part, bound, layer_id("Bir")),
                           (1, "Satır 2", part, bound, layer_id("İki"))],
                          [("Bir", layer_id("Bir"), part, cols), ("İki", layer_id("İki"), part, cols)], PLAN),
                  [{"code": "partial_scope", "message": msg, "path": "parameters.input"}]), op="plan")))
    # 9. Tables added to a file that is there: its tables untouched.
    out.append(case(
        "var olan dosyaya tablo eklenir, dosyanın tabloları dokunulmaz",
        {"op": "copyFile", "from": FX, "to": "$tmp:toplu-ekle.gpkg"},
        step(batch([row(source(filter="kat <= 2"), "Alçak"), row(source(filter="kat >= 3"), "Yüksek")],
                   {"path": "$tmp:toplu-ekle.gpkg", "mode": "addTables"}),
             done(batched([(0, "Satır 1", len(low), staged("bina", len(low), "kaynak-input-1"), layer_id("Alçak")),
                           (1, "Satır 2", len(high), staged("bina", len(high), "kaynak-input-2"),
                            layer_id("Yüksek"))],
                          [("Alçak", layer_id("Alçak"), len(low), cols),
                           ("Yüksek", layer_id("Yüksek"), len(high), cols)],
                          "$tmp:toplu-ekle.gpkg", mode="addTables", untouched=UNTOUCHED)), SAME)))
    # 10. A table of the file's name: addTables refuses, the file as it was; replaceTables replaces.
    out.append(case(
        "aynı adlı tablo: addTables reddeder (table_exists)",
        {"op": "copyFile", "from": FX, "to": "$tmp:toplu-ayni.gpkg"},
        step(batch([row(source(filter="kat <= 2"), "bina")], {"path": "$tmp:toplu-ayni.gpkg", "mode": "addTables"}),
             failed("table_exists",
                    "“bina” tablosu $tmp:toplu-ayni.gpkg dosyasında var (tablonun adı katmanın ya da aracın tablo "
                    "çıktısının adıdır); üzerine yazmak için sink.mode: replaceTables verin ya da başka bir dosyaya "
                    "yazın. Dosyaya dokunulmadı.", "sink.path"), SAME)))
    rest = [t for t in UNTOUCHED if t != "bina"]
    replaced = batched([(0, "Satır 1", len(low), staged("bina", len(low), "kaynak-input-1"), layer_id("bina"))],
                       [("bina", layer_id("bina"), len(low), cols)], "$tmp:toplu-degistir.gpkg", mode="replaceTables",
                       untouched=rest)
    replaced["sinkReport"]["tables"][0]["replaced"] = True
    out.append(case(
        "aynı adlı tablo: replaceTables yerine yazar, öteki tablolar kalır",
        {"op": "copyFile", "from": FX, "to": "$tmp:toplu-degistir.gpkg"},
        step(batch([row(source(filter="kat <= 2"), "bina")],
                   {"path": "$tmp:toplu-degistir.gpkg", "mode": "replaceTables"}), done(replaced), SAME)))
    # 11–13. A row that stops: the batch back, or only the row.
    missing = "“yok” tablosu dosyada yok; dosyanın tablolarından birinin adını verin (cad.sources.describe listeler)."
    out.append(case(
        "olmayan tabloyu okuyan satır, Dur: hiçbir şey yazılmaz",
        step(batch([row(source(filter="kat <= 2"), "Alçak"), row(source("yok"), "Yok")], {"path": PLAN}),
             failed("batch_row_failed", f"Satır 2 çalışmadı: {missing} Toplu işin hiçbir sonucu yazılmadı.",
                    "rows[1]"), op="plan")))
    stopped = [{"index": 1, "label": "Satır 2", "code": "table_not_found", "message": missing}]
    out.append(case(
        "olmayan tabloyu okuyan satır, Devam et: yalnız o satır geri alınır",
        step(batch([row(source(filter="kat <= 2"), "Alçak"), row(source("yok"), "Yok"),
                    row(source(filter="kat >= 3"), "Yüksek")], {"path": PLAN}, on_error="continue"),
             done(batched([(0, "Satır 1", len(low), staged("bina", len(low), "kaynak-input-1"), layer_id("Alçak")),
                           (2, "Satır 3", len(high), staged("bina", len(high), "kaynak-input-3"),
                            layer_id("Yüksek"))],
                          [("Alçak", layer_id("Alçak"), len(low), cols),
                           ("Yüksek", layer_id("Yüksek"), len(high), cols)], PLAN, failed_rows=stopped),
                  [{"code": "batch_rows_failed",
                    "message": "1 satır çalışıp durdu ve tek tek geri alındı (Satır 2); öteki satırlar yazıldı.",
                    "path": "rows"}]), op="plan")))
    empty = "“Nesneler”: bu katmanda uygun nesne yok. Başka bir katman seçin."
    out.append(case(
        "her satır durursa, Devam et: hiçbir şey yazılmaz, dosya yok",
        step(batch([row(source("yok"), "Yok"), row(source(filter="kat > 9"), "Boş")], {"path": "$tmp:toplu-hic.gpkg"},
                   on_error="continue"),
             done({"tool": TOOL, "summary": "Hiçbir satır çalışmadı; 2 satır geri alındı, sonuç yazılmadı.",
                   "rows": [], "failed": [
                       {"index": 0, "label": "Satır 1", "code": "table_not_found", "message": missing},
                       {"index": 1, "label": "Satır 2", "code": "invalid_parameters", "message": empty}],
                   "record": ANY, "revision": "$current"},
                  [{"code": "batch_rows_failed",
                    "message": "2 satır çalışıp durdu ve tek tek geri alındı (Satır 1, Satır 2); hiçbir satır "
                               "yazılmadı.", "path": "rows"}]), SAME),
        step({"path": "$tmp:toplu-hic.gpkg"}, {"status": "failed", "error": ANY}, command="cad.sources.describe")))
    # 14. A selecting tool makes nothing: nothing to write.
    out.append(case(
        "seçen araç yeni nesne yapmaz: nothing_to_export",
        step({"tool": "selection.byExpression", "rows": [{}, {}], "sink": {"path": PLAN}},
             failed("nothing_to_export",
                    "Araç yeni nesne yapmadı; dosyaya yazılacak sonuç yok (değiştiren ya da seçen araçların sonucu "
                    "çizimdedir; aracın tablo çıktısını yazmak için sink.tables verin).", "sink.path"), op="plan")))
    # 15. Table outputs: each row's own table, two of one name told apart by the rows' labels.
    srec = {
        "format": "kentos.run-record", "version": 1, "kind": "batch", "tool": "statistics.summary",
        "label": "Özet istatistik", "toolVersion": 1, "parameters": {"input": source()},
        "inputs": [staged("bina", len(bina), "kaynak-input")], "documentRevision": ANY, "srid": 5256,
        "engine": ANY, "host": ANY, "startedAt": ANY, "ms": ANY, "steps": ANY, "total": ANY,
        "sink": {"path": "$tmp:toplu-ozet.gpkg", "mode": "new", "bytes": ANY, "sha256": ANY, "tables": [
            # Ungrouped: one row; grouped by ad: a row for each of the 12 names and the total.
            {"table": "Özet tablosu (Satır 1)", "of": "rows[0].table", "row": "rows[0]", "rows": 1},
            {"table": "Özet tablosu (Satır 2)", "of": "rows[1].table", "row": "rows[1]",
             "rows": len({r["ad"] for r in bina}) + 1}]},
    }
    out.append(case(
        "tablo çıktıları: her satırın kendi tablosu, aynı ad satırın adıyla ayrılır",
        step({"tool": "statistics.summary", "parameters": {"input": source()},
              "rows": [{"parameters": {"field": "kat"}}, {"parameters": {"field": "kat", "group": "ad"}}],
              "sink": {"path": "$tmp:toplu-ozet.gpkg", "tables": True}},
             done({"tool": "statistics.summary",
                   "summary": "2 satır çalıştı: 0 nesne toplu-ozet.gpkg dosyasına 2 tabloya yazıldı.",
                   "rows": ANY, "sink": ANY, "sinkReport": ANY, "record": srec, "revision": "$current"}), SAME)))
    # Refusals before any row runs.
    sink_words = ("“input” kaynağından okunuyor: çalıştırma bir kopyada yapılır ve sonucu bir GeoPackage'a ya da "
                  "PostGIS'e yazılır; sink verin.")
    bad = [
        ("ortak kaynak sink ister", batch([{}], None, {"input": source()}), "invalid_parameters", sink_words,
         "parameters.input"),
        ("satırın kaynağı sink ister", batch([{}, row(source())], None), "invalid_parameters", sink_words,
         "rows[1].parameters.input"),
        ("parolalı veritabanı adresi", batch([row(source())], {"path": "postgresql://kentos:gizli@localhost/vt"}),
         "invalid_address",
         "postgresql://kentos:gizli@localhost/vt: adreste parola var; parola projeye yazılmaz: PGPASSWORD ortam "
         "değişkenine ya da ~/.pgpass dosyasına koyun.", "sink.path"),
        ("GeoPackage'a şema", batch([row(source())], {"path": PLAN, "schema": "sonuc"}), "invalid_input",
         "Şema yalnız bir veritabanına yazılırken verilir; GeoPackage'ın şeması yok.", "sink.schema"),
        ("GeoPackage olmayan sonuç adı", batch([row(source())], {"path": "$fixture:processing/v1/sonuc.shp"}),
         "invalid_file_path", "$fixture:processing/v1/sonuc.shp bir GeoPackage adı değil; sonucun dosyası .gpkg ile "
         "biter.", "sink.path"),
        ("var olan dosya", batch([row(source())], {"path": FX}), "file_exists",
         f"{FX} var; üzerine yazmak için overwrite: true verin.", "sink.path"),
        ("tablo eklenecek dosya yok", batch([row(source())], {"path": "$tmp:toplu-yok.gpkg", "mode": "addTables"}),
         "file_not_found", "$tmp:toplu-yok.gpkg yok; tablo eklenecek GeoPackage'ın yolunu verin ya da yeni bir dosya için "
         "sink.mode vermeyin.", "sink.path"),
        ("ortak kaynağın katmanı yok", batch([{}], {"path": PLAN}, {"input": {"scope": "source", "layer": "yok"}}),
         "layer_not_found", "“yok” kimlikli katman çizimde yok.", "parameters.input.layer"),
        ("ortak kaynağın tablosu yok", batch([{}], {"path": PLAN}, {"input": source("yok")}), "table_not_found",
         missing, "parameters.input.table"),
        ("ortak kaynağın ters kutusu", batch([{}], {"path": PLAN}, {"input": source(bbox=[10, 0, 0, 10])}),
         "invalid_extent",
         "Kutu [batı, güney, doğu, kuzey] olmalı: sonlu sayılar, batı doğudan, güney kuzeyden büyük değil.",
         "parameters.input.bbox"),
        ("ortak kaynağın derlenmeyen süzgeci", batch([{}], {"path": PLAN}, {"input": source(filter="kat >")}),
         "invalid_expression", ANY, "parameters.input.filter"),
        ("ortak kaynak GeoPackage değil", batch([{}], {"path": PLAN}, {"input": {
            "scope": "source", "path": "$fixture:sources/v1/cases.json", "table": "bina"}}), "import_unreadable", ANY,
         "parameters.input.path"),
        ("ulaşılamayan ya da sağlayıcısı olmayan veritabanına yazma",
         batch([row(source())], {"path": "postgresql://kentos@127.0.0.1:1/vt"}), "source_unreachable", ANY,
         "sink.path"),
    ]
    for name, inp, code, msg, path in bad:
        out.append(case(f"ret: {name}", step(inp, failed(code, msg, path), op="plan")))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.processing.batch", "commandVersion": 1,
            "title": "Toplu çalıştırma: sonucu dosyaya ya da veritabanına yazmak, girdiyi kaynağından okumak",
            "note": "Üretici: scripts/fixtures/processing_batch_sink_cases.py (docs/adr/0593). Kaynağın kayıtları "
                    "sorgu.gpkg'den sqlite3'le; komutun sözleri ADR'lerden.",
            "setup": setup(), "cases": cases()}


def main():
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
