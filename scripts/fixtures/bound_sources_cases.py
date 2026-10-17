#!/usr/bin/env python3
"""The shared cases of the bound layers' commands cad.sources.describe, .edit and .sync (docs/adr/0503, plan/gis.md
CBS-01, CBS-02, CBS-11, CBS-36).

    /usr/bin/python3 scripts/fixtures/bound_sources_cases.py           # writes fixtures/commands/v1/cad.sources.*.json
    /usr/bin/python3 scripts/fixtures/bound_sources_cases.py --check   # compares

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
        case("WGS 84'teki tablo bağlanmaz: bağlı katman çevrilmez",
             step({"edit": {"kind": "link", "path": fx("wgs84.gpkg"), "table": "yer"}},
                  failed("crs_mismatch", "“yer” tablosu EPSG:4326 sisteminde, proje EPSG:5256. Bağlı katman dosyayla "
                         "aynı sistemde olmalı: projeyi EPSG:4326'de açın ya da tabloyu çevirip içe aktarın.",
                         "edit.path"))),
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

def synced(added=0, modified=0, removed=0, columns=(), name=Q):
    out = {"added": added, "modified": modified, "removed": removed, "source": sha(name), "revision": "$current"}
    if columns:
        out["columns"] = list(columns)
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
        case("validate yalnız denetler", s_link, step(w, done(None), op="validate")),
    ]


FILES = {
    "cad.sources.describe": ("Kaynakları tanı: dosyanın tabloları ve bağlı katmanların durumu", describe_cases),
    "cad.sources.edit": ("Kaynağa bağla: tablo katmana, bağı çöz, yeniden bağla", edit_cases),
    "cad.sources.sync": ("Kaynakla eşitle: katmanın değişiklikleri tabloya, tablonun satırları katmana", sync_cases),
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
    if "--check" in sys.argv and not bad:
        print("Bağlı katman durumları tutarlı: " + ", ".join(f"{c} {len(build(c)['cases'])}" for c in FILES))
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
