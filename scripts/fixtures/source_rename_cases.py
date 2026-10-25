#!/usr/bin/env python3
"""The shared cases of a column renamed in a bound table's source (docs/adr/0635 §1, plan/gis.md CBS-17):

    /usr/bin/python3 scripts/fixtures/source_rename_cases.py --files   # GDAL writes fixtures/geopackage/v1/ad-*.gpkg
    python3 scripts/fixtures/source_rename_cases.py                    # writes fixtures/commands/v1/cad.sources.sync.renamed.json
    python3 scripts/fixtures/source_rename_cases.py --check            # compares

The table `parsel` (ada text, parsel_no integer, alan real; three rows) as it was bound, and as a GIS left it: a
column renamed, two renamed, one renamed with a row changed, one dropped, one added, one renamed with its type changed,
two swapped, the key column renamed. Each file is written by GDAL's OGR with the same rows and fids, as SQLite's
`RENAME COLUMN` leaves them. What a case expects is the ADR's rule, applied here to the columns as listed:

- a layer field the table lacks and a table column the layer lacks, at the same place and of the same kind, is a
  rename: the layer's field takes the new name, the objects' attributes move to it, the layer's filter that names it
  follows, the link's key too; `source_columns_renamed` says them;
- anything else (a dropped or added column moves the places, a type changed, names that are on both sides) is no
  rename: the field stays as it was and the objects take the table's attributes, as before the ADR.

A row's lasting id is the importer's (uuid5 of the table and the key, gpkg_import_cases.py).
"""
import json
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import gpkg_import_cases as gi  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/cad.sources.sync.renamed.json"
ANY = "$any"
TABLE = "parsel"
LAYER = f"import-{TABLE}"

ROWS = [
    {"ada": "101", "parsel_no": 7, "alan": 1250.5},
    {"ada": "101", "parsel_no": 8, "alan": 980.25},
    {"ada": "102", "parsel_no": 1, "alan": 2100.0},
]
BASE = [("ada", "text"), ("parsel_no", "integer"), ("alan", "real")]

# The files: name → (columns as (name, kind), how a row's values are read from the base row by column, changes).
VARIANTS = {
    "ad-tek": [("ada_no", "text", "ada"), ("parsel_no", "integer", "parsel_no"), ("alan", "real", "alan")],
    "ad-iki": [("ada_no", "text", "ada"), ("no", "integer", "parsel_no"), ("alan", "real", "alan")],
    "ad-satir": [("ada_no", "text", "ada"), ("parsel_no", "integer", "parsel_no"), ("alan", "real", "alan")],
    "ad-dusen": [("ada", "text", "ada"), ("alan", "real", "alan")],
    "ad-eklenen": [("ada", "text", "ada"), ("parsel_no", "integer", "parsel_no"), ("alan", "real", "alan"),
                   ("mahalle", "text", None)],
    "ad-tur": [("ada", "text", "ada"), ("parsel", "text", "parsel_no"), ("alan", "real", "alan")],
    "ad-takas": [("parsel_no", "integer", "parsel_no"), ("ada", "text", "ada"), ("alan", "real", "alan")],
    "ad-hepsi": [("ada_no", "text", "ada"), ("no", "integer", "parsel_no"), ("yuzolcum", "real", "alan")],
}
# A row changed in the GIS beside the rename (ad-satir): the second parcel's area.
CHANGED = {"ad-satir": (1, "alan", 990.0)}


def ogr_type(kind):
    from osgeo import ogr
    return {"text": ogr.OFTString, "integer": ogr.OFTInteger64, "real": ogr.OFTReal}[kind]


def square(i):
    x, y = 487000.0 + i * 50, 4420000.0
    return [(x, y), (x + 40, y), (x + 40, y + 30), (x, y + 30)]


def write_file(name, columns, rows):
    from osgeo import ogr, osr
    path = DIR / f"{name}.gpkg"
    if path.exists():
        path.unlink()
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    lyr = ds.CreateLayer(TABLE, srs, ogr.wkbPolygon, ["FID=fid", "SPATIAL_INDEX=NO"])
    for col, kind, _ in columns:
        lyr.CreateField(ogr.FieldDefn(col, ogr_type(kind)))
    for i, r in enumerate(rows):
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(i + 1)
        for col, kind, src in columns:
            if src is None:
                continue
            v = r[src]
            f.SetField(col, str(v) if kind == "text" else v)
        ring = ogr.Geometry(ogr.wkbLinearRing)
        for x, y in square(i) + square(i)[:1]:
            ring.AddPoint_2D(x, y)
        g = ogr.Geometry(ogr.wkbPolygon)
        g.AddGeometry(ring)
        f.SetGeometry(g)
        lyr.CreateFeature(f)
    ds = None


def write_files():
    write_file("ad-temel", [(c, k, c) for c, k in BASE], ROWS)
    for name, cols in VARIANTS.items():
        rows = [dict(r) for r in ROWS]
        if name in CHANGED:
            i, col, v = CHANGED[name]
            rows[i][col] = v
        write_file(name, cols, rows)
    print("wrote", ", ".join(["ad-temel", *VARIANTS]))


# ── What the cases expect ───────────────────────────────────────────────────

def uid_of(fid):
    return str(uuid.uuid5(gi.NAMESPACE, f"gpkg:{TABLE}:{fid}"))


def text(kind, v):
    """An attribute as the importer writes it: a real without a needless fraction, an integer as digits."""
    if v is None:
        return None
    if kind == "real":
        return gi.real_text(v)
    return str(v)


def field(name, kind):
    return {"name": name, "kind": {"text": "text", "integer": "integer", "real": "decimal"}[kind]}


def renames(old, new):
    """The ADR's rule: pairs at one place, a name only on its side, the kind the same; nothing when the counts differ."""
    if len(old) != len(new):
        return []
    in_new = {n for n, _ in new}
    in_old = {n for n, _ in old}
    return [(o, n) for (o, ok), (n, nk) in zip(old, new) if o not in in_new and n not in in_old and ok == nk]


def attrs_after(name, row):
    cols = VARIANTS[name]
    rows = [dict(r) for r in ROWS]
    if name in CHANGED:
        i, col, v = CHANGED[name]
        rows[i][col] = v
    r = rows[row]
    out = {}
    for col, kind, src in cols:
        if src is None:
            continue
        out[col] = text(kind, r[src])
    return out


def fields_after(name):
    old = BASE
    new = [(c, k) for c, k, _ in VARIANTS[name]]
    pairs = dict(renames(old, new))
    return [field(pairs.get(c, c), k) for c, k in old]


def linked_steps(name, n):
    tmp = f"$tmp:ad-{n}.gpkg"
    return tmp, [
        {"op": "copyFile", "from": "$fixture:geopackage/v1/ad-temel.gpkg", "to": tmp},
        bs.other("cad.sources.edit", {"edit": {"kind": "link", "path": tmp, "table": TABLE}}),
    ]


def read(tmp, name, op="execute", discard=False, filtered=None):
    old = BASE
    new = [(c, k) for c, k, _ in VARIANTS[name]]
    pairs = renames(old, new)
    warnings = []
    if pairs:
        warnings.append({"code": "source_columns_renamed", "message": (
            "Kaynakta yeniden adlandırılan sütunları katmanın alanları izledi: "
            + ", ".join(f"{a} → {b}" for a, b in pairs)
            + ". Görünüş, etiketler, formlar ve ilişkiler yeni adı aldı."), "path": "layer"})
    out = {"added": 0, "modified": ANY, "removed": 0, "source": ANY, "revision": "$current"}
    if pairs:
        out["renamed"] = [{"from": a, "to": b} for a, b in pairs]
    inp = {"layer": LAYER, "direction": "read"}
    if discard:
        inp["discard"] = True
    expect = {"revision": "same"} if op == "plan" else {"revision": "changed"}
    if op != "plan":
        expect["fields"] = {str(2 + i): {"attrs": attrs_after(name, i)} for i in range(3)}
        node = {"fields": fields_after(name)}
        if filtered is not None:
            node["filter"] = {"expression": filtered}
        expect["nodes"] = {LAYER: node}
    return {"op": op, "command": "cad.sources.sync", "input": inp,
            "result": {"status": "completed", "output": out, "warnings": warnings}, "expect": expect}


def swap(tmp, name):
    return {"op": "copyFile", "from": f"$fixture:geopackage/v1/{name}.gpkg", "to": tmp}


def cases():
    out = []
    n = [0]

    def c(title, name, *extra, op="execute", before=(), filtered=None):
        n[0] += 1
        tmp, steps = linked_steps(name, n[0])
        out.append(bs.case(title, *steps, *before, swap(tmp, name), *extra,
                           read(tmp, name, op=op, filtered=filtered)))

    c("bir sütun kaynakta yeniden adlandırılınca alan ve öznitelikler izler", "ad-tek")
    c("plan yazmadan söyler", "ad-tek", op="plan")
    c("iki sütun birlikte", "ad-iki")
    c("iki sütunun planı yazmadan ikisini söyler", "ad-iki", op="plan")
    c("bütün sütunlar yeniden adlandırılmış", "ad-hepsi")
    c("yeniden adlandırma ve değişen satır birlikte", "ad-satir")
    c("düşen sütun yerleri kaydırır: yeniden adlandırma sayılmaz", "ad-dusen")
    c("eklenen sütun: yeniden adlandırma yok", "ad-eklenen")
    c("türü değişen sütun yeniden adlandırma sayılmaz", "ad-tur")
    c("yerleri takas edilen sütunlar: iki yanda da adlar var", "ad-takas")
    c("katmanın süzgeci yeni adı alır", "ad-tek",
      before=(bs.other("cad.layers.filter", {"layer": LAYER, "filter": {"expression": "ada = '101'"}}),),
      filtered="ada_no = '101'")
    c("iki alanı anan süzgeç ikisini de izler", "ad-iki",
      before=(bs.other("cad.layers.filter", {"layer": LAYER,
                                              "filter": {"expression": "ada = '101' and parsel_no > 7"}}),),
      filtered="ada_no = '101' and no > 7")
    c("adı değişmeyen alanı anan süzgeç değişmez", "ad-tek",
      before=(bs.other("cad.layers.filter", {"layer": LAYER, "filter": {"expression": "alan > 1000"}}),),
      filtered="alan > 1000")
    # Undo puts names and values back.
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    out.append(bs.case("geri alma adları ve değerleri geri getirir", *steps, swap(tmp, "ad-tek"),
                       read(tmp, "ad-tek"),
                       {"op": "undo", "returns": f"Kaynaktan oku: {TABLE}", "expect": {
                           "fields": {str(2 + i): {"attrs": {c: text(k, ROWS[i][c]) for c, k in BASE}}
                                      for i in range(3)},
                           "nodes": {LAYER: {"fields": [field(c, k) for c, k in BASE]}}}}))
    # Refusals: the layer changed since it last matched, a locked layer.
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    out.append(bs.case("katman değiştiyse okuma reddedilir, adlar değişmez", *steps,
                       bs.other("cad.entities.set", {"uids": [uid_of(1)], "attrs": {"ada": "999"},
                                                     "operation": "attributes"}),
                       swap(tmp, "ad-tek"),
                       {"op": "execute", "command": "cad.sources.sync",
                        "input": {"layer": LAYER, "direction": "read"},
                        "result": {"status": "conflict", "error": {"code": "pending_changes", "message": ANY,
                                                                    "path": "layer"}},
                        "expect": {"revision": "same", "nodes": {LAYER: {"fields": [field(c, k) for c, k in BASE]}}}}))
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    discard = read(tmp, "ad-tek", discard=True)
    out.append(bs.case("katmanın değişikliği bırakılarak okunursa adlar izler", *steps,
                       bs.other("cad.entities.set", {"uids": [uid_of(1)], "attrs": {"ada": "999"},
                                                     "operation": "attributes"}),
                       swap(tmp, "ad-tek"), discard))
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    out.append(bs.case("kilitli katman okunmaz", *steps, swap(tmp, "ad-tek"),
                       bs.other("cad.layers.set", {"operation": "locked", "layer": LAYER, "value": True}),
                       {"op": "execute", "command": "cad.sources.sync",
                        "input": {"layer": LAYER, "direction": "read"},
                        "result": {"status": "failed", "error": {"code": "layer_locked", "message": ANY,
                                                                  "path": "layer"}},
                        "expect": {"revision": "same"}}))
    # A second read finds nothing more to rename.
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    again = read(tmp, "ad-tek")
    again2 = json.loads(json.dumps(again))
    again2["result"]["warnings"] = []
    again2["result"]["output"].pop("renamed", None)
    again2["expect"] = {"revision": "same"}
    out.append(bs.case("ikinci okuma yeniden adlandırmaz", *steps, swap(tmp, "ad-tek"), again, again2))
    # The renamed table written back: the layer's attributes are the new columns'.
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    out.append(bs.case("izlenen adlarla yazma tabloya aynı sütunları yazar", *steps, swap(tmp, "ad-tek"),
                       read(tmp, "ad-tek"),
                       bs.other("cad.entities.set", {"uids": [uid_of(2)], "attrs": {"ada_no": "103"},
                                                     "operation": "attributes"}),
                       {"op": "execute", "command": "cad.sources.sync",
                        "input": {"layer": LAYER, "direction": "write"},
                        "result": {"status": "completed", "output": {"added": 0, "modified": 1, "removed": 0,
                                                                     "source": ANY, "revision": "$current"},
                                   "warnings": []},
                        "expect": {"revision": "changed"}}))
    # docs/adr/0635 §2: the layer's look takes a library symbol that names the column; the rename follows the layer,
    # the shared symbol keeps the old name and is said.
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    lib = read(tmp, "ad-tek")
    lib["result"]["warnings"].append({"code": "library_symbol_field", "message": (
        "Kitaplık › “Ada rengi” simgesi › Veriye bağlı değer “ada” alanını anıyor (yeni adı “ada_no”). "
        "Kitaplığın simgesi katmanlar arasında paylaşılır, yeniden adlandırma onu değiştirmez: simgeyi Stil yöneticisinde "
        "düzenleyin."), "path": "layer"})
    out.append({**bs.case("görünüşün kitaplık simgesi eski adı anar: değişmez ve söylenir", *steps,
                          bs.other("cad.layers.renderer", {"layer": LAYER, "renderer": {
                              "type": "single", "symbols": {"fill": {"ref": "kitaplik-ada"}}}}),
                          swap(tmp, "ad-tek"), lib), "setup": lib_setup()})
    # Fields the layer adds: the counts differ, no rename.
    n[0] += 1
    tmp, steps = linked_steps("ad-tek", n[0])
    fields = [field(c, k) for c, k in BASE] + [{"name": "durum", "kind": "text"}]
    out.append(bs.case("katmanın kendi eklediği alan varken yeniden adlandırma sayılmaz", *steps,
                       bs.other("cad.layers.edit", {"operation": "fields", "layer": LAYER, "fields": fields}),
                       swap(tmp, "ad-tek"),
                       {"op": "execute", "command": "cad.sources.sync",
                        "input": {"layer": LAYER, "direction": "read", "discard": True},
                        "result": {"status": "completed", "output": {"added": 0, "modified": ANY, "removed": 0,
                                                                     "source": ANY, "revision": "$current"},
                                   "warnings": []},
                        "expect": {"revision": "changed", "nodes": {LAYER: {"fields": fields}}}}))
    return out


# A library symbol coloured by the parcel's block (docs/adr/0635 §2).
LIBRARY = [{"id": "kitaplik-ada", "name": "Ada rengi", "kind": "symbol",
            "symbol": {"type": "fill", "layers": [{"id": "f", "type": "simpleFill",
                                                   "color": {"expr": "[ada]", "fallback": "#CCCCCC"}}]}}]


def lib_setup():
    doc = bs.setup()
    doc["styles"]["items"] = json.loads(json.dumps(LIBRARY))
    return doc


def cases_file():
    setup = bs.setup()
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.sync", "commandVersion": 1,
            "title": "Kaynakta yeniden adlandırılan sütun: bağlı katmanın alanı ve onu ananlar izler",
            "note": "ADR 0635 §1. GeoPackage'ları GDAL'ın OGR'ı yazar (scripts/fixtures/source_rename_cases.py --files); "
                    "beklenen ADR'nin kuralı: aynı yerde, yalnız bir yanda olan, türü aynı ad çifti yeniden adlandırmadır.",
            "setup": setup, "cases": cases()}


def main():
    if "--files" in sys.argv:
        write_files()
        return
    text = json.dumps(cases_file(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text() != text:
            sys.exit(f"{OUT} is not what the script writes; run it without --check and read the difference")
        print(f"{len(cases())} cases ok")
        return
    OUT.write_text(text)
    print(f"wrote {OUT} ({len(cases())} cases)")


if __name__ == "__main__":
    main()
