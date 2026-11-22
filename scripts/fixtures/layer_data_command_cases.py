"""Writes fixtures/commands/v1/cad.layers.data.json: the shared cases of `cad.layers.data` v1 (docs/adr/9101 §4), their
expectations built here from the contract's rules (crates/shared/commands/src/gis/cad_layers_data.rs and
crates/shared/contracts/src/gis/data_layer.rs: the checks and their order, the codes, the paths and the words; where a
node goes; what a data layer draws; the step's name), without KentOS code. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python run them.

    python3 scripts/fixtures/layer_data_command_cases.py           # writes the file
    python3 scripts/fixtures/layer_data_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.layers.data.json"

STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
# A new layer's look: the layer tree's defaults (desktop `default_style`).
NEW_STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.18}

PARCELS = {"driver": "postgis", "address": "postgresql://kbs@db.ornek.bel.tr:5432/kent", "table": "kadastro.parseller",
           "key": "fid", "geometry": {"column": "geom", "typeName": "MULTIPOLYGON", "srid": 5256}}
POLES = {"driver": "gpkg", "address": "/veri/altyapi.gpkg", "relative": "veri/altyapi.gpkg", "table": "direkler",
         "key": "fid", "geometry": {"column": "geom", "typeName": "POINT", "srid": 5256, "z": True}, "readOnly": True}
ROADS = {"driver": "postgis", "address": "postgresql://kbs@db.ornek.bel.tr:5432/kent", "table": "ulasim.yollar",
         "key": "gid", "geometry": {"column": "geom", "typeName": "MULTILINESTRING", "srid": 5256}}
OWNERS = {"driver": "gpkg", "address": "/veri/altyapi.gpkg", "table": "malikler", "key": "fid"}
ANY = {"driver": "gpkg", "address": "/veri/altyapi.gpkg", "table": "karisik", "key": "fid",
       "geometry": {"column": "geom", "typeName": "GEOMETRY"}}
# A query read as a table (docs/adr/9113 §1): its text is the table, its first line an SQL comment saying the key and
# the geometry; read-only. The command reads no source: the text is kept as given.
QUERY = {"driver": "postgis", "address": "postgresql://kbs@db.ornek.bel.tr:5432/kent",
         "table": "-- kentos: key=fid geometry=geom\nSELECT fid, ada, geom FROM kadastro.parseller WHERE ada = 105",
         "key": "fid", "geometry": {"column": "geom", "typeName": "MULTIPOLYGON", "srid": 5256}, "readOnly": True}
# The longest text a data layer keeps in a field (contracts' MAX_DATA_TEXT), in bytes.
MAX_TEXT = 2048
PARCEL_FIELDS = [{"name": "ada", "kind": "integer"}, {"name": "parsel", "kind": "integer"}, {"name": "mahalle", "kind": "text"}]
ROAD_FIELDS = [{"name": "ad", "kind": "text"}, {"name": "tur", "kind": "text"}]

# What a table's geometry draws (docs/adr/9101 §2): points, lines, areas; a generic column or a collection draws any.
FAMILY = {"POINT": "point", "MULTIPOINT": "point", "LINESTRING": "line", "MULTILINESTRING": "line",
          "CIRCULARSTRING": "line", "COMPOUNDCURVE": "line", "MULTICURVE": "line", "POLYGON": "area",
          "MULTIPOLYGON": "area", "CURVEPOLYGON": "area", "MULTISURFACE": "area"}


def family(data):
    g = data.get("geometry")
    return FAMILY.get(g["typeName"]) if g else None


def node(id_, name, type_="layer", children=None, style=STYLE, **extra):
    return {"id": id_, "name": name, "type": type_, "visible": True, "locked": False, "expanded": True,
            "style": dict(style), "children": children or [], **extra}


def data_node(id_, name, data, fields=None, style=STYLE):
    extra = {"data": copy.deepcopy(data)}
    if fields:
        extra["fields"] = copy.deepcopy(fields)
    if family(data):
        extra["geometry"] = family(data)
    return node(id_, name, style=style, **extra)


def new_layer(name, data, fields=None):
    """The node `add` makes: the tree's defaults, typed by its table, its id the one the command gives (`$layer`)."""
    return data_node("$layer", name, data, fields, style=NEW_STYLE)


SETUP_LAYERS = [
    data_node("parseller", "Parseller", PARCELS, PARCEL_FIELDS),
    node("veri", "Veri", "group", [data_node("direkler", "Direkler", POLES)]),
    node("0", "0"),
]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Veri katmanları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 500000, "y": 4400000},
    "layers": SETUP_LAYERS,
    "activeLayer": "0",
    "entities": [{"kind": "point", "id": 1, "layerId": "0", "attrs": {}, "p": {"x": 500010, "y": 4400030}}],
    "styles": {"items": [], "categories": []},
}

LABEL = {"add": "Veri katmanı ekle", "update": "Veri katmanını değiştir", "remove": "Veri katmanını kaldır"}


def tree():
    return copy.deepcopy(SETUP_LAYERS)


def find(nodes, id_):
    for i, n in enumerate(nodes):
        if n["id"] == id_:
            return nodes, i
        hit = find(n["children"], id_)
        if hit:
            return hit
    return None


def parent_of(nodes, id_, parent=None):
    for n in nodes:
        if n["id"] == id_:
            return parent
        found = parent_of(n["children"], id_, n)
        if found is not False:
            return found
    return False


def inserted(layers, new, parent=None, index=None):
    """Where a new node goes: into `parent` when it is a group, into the group of `parent` when it is a layer, else the
    top; at `index` among that group's nodes (first on top), last when none or past the end."""
    out = copy.deepcopy(layers)
    home = out
    if parent is not None and find(out, parent):
        siblings, i = find(out, parent)
        target = siblings[i]
        if target["type"] == "group":
            home = target["children"]
            target["expanded"] = True
        else:
            group = parent_of(out, parent)
            home = group["children"] if group else out
    at = len(home) if index is None else min(index, len(home))
    home.insert(at, new)
    return out


def changed(layers, id_, name=None, data=None, fields=None):
    """A data layer after `update`: its name, its table and what the table draws, its fields."""
    out = copy.deepcopy(layers)
    siblings, i = find(out, id_)
    n = siblings[i]
    if name is not None:
        n["name"] = name.strip()
    if data is not None:
        n["data"] = copy.deepcopy(data)
        n.pop("geometry", None)
        if family(data):
            n["geometry"] = family(data)
    if fields is not None:
        if fields:
            n["fields"] = copy.deepcopy(fields)
        else:
            n.pop("fields", None)
    return out


def removed(layers, id_):
    out = copy.deepcopy(layers)
    siblings, i = find(out, id_)
    del siblings[i]
    return out


def node_of(layers, id_):
    siblings, i = find(layers, id_)
    return siblings[i]


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(op, input_=None, result=None, expect=None, **extra):
    s = {"op": op}
    if input_ is not None:
        s["input"] = input_
    if result is not None:
        s["result"] = result
    s.update(extra)
    if expect is not None:
        s["expect"] = expect
    return s


WRITTEN = completed({"layer": "$layer", "revision": "$current"})
UNCHANGED = {"ids": [1], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def done(layer):
    return completed({"layer": layer, "revision": "$current"})


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [step("execute", input_, failed(code, message, path), {**UNCHANGED, "layers": tree()})]}


PASSWORD = "adreste parola var; parola projeye yazılmaz: PGPASSWORD ortam değişkenine ya da ~/.pgpass dosyasına koyun"


def cases():
    out = []
    roads = new_layer("Yollar", ROADS, ROAD_FIELDS)
    with_roads = inserted(tree(), roads)
    out.append({
        "name": "Ekle: PostGIS tablosu ağacın sonuna; çizgi katmanı olur, satırları çizime girmez; adımı “Veri katmanı ekle”, geri alınır ve yinelenir",
        "steps": [
            step("execute", {"operation": "add", "name": "Yollar", "data": ROADS, "fields": ROAD_FIELDS}, WRITTEN,
                 {"ids": [1], "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed", "layers": with_roads}),
            step("undo", returns=LABEL["add"], expect={"layers": tree(), "canUndo": False, "canRedo": True}),
            step("redo", returns=LABEL["add"], expect={"layers": with_roads, "canUndo": True}),
        ],
    })
    out.append({
        "name": "Ekle: grubun başına (parent bir grup, index 0); adın baştaki ve sondaki boşluğu atılır",
        "steps": [step("execute", {"operation": "add", "name": "  Yollar  ", "parent": "veri", "index": 0, "data": ROADS, "fields": ROAD_FIELDS},
                       WRITTEN, {"layers": inserted(tree(), copy.deepcopy(roads), "veri", 0)})],
    })
    out.append({
        "name": "Ekle: bir katmanın yanına (parent bir katman) onun grubunun sonuna",
        "steps": [step("execute", {"operation": "add", "name": "Yollar", "parent": "direkler", "data": ROADS, "fields": ROAD_FIELDS},
                       WRITTEN, {"layers": inserted(tree(), copy.deepcopy(roads), "direkler")})],
    })
    out.append({
        "name": "Ekle: SQL sorgusu tablonun metni olarak (ADR 9113); salt okunur alan katmanı, metni olduğu gibi saklanır",
        "steps": [step("execute", {"operation": "add", "name": "Sorgu: parseller", "data": QUERY,
                                   "fields": [{"name": "ada", "kind": "integer"}]},
                       WRITTEN, {"layers": inserted(tree(), new_layer("Sorgu: parseller", QUERY, [{"name": "ada", "kind": "integer"}]))})],
    })
    out.append({
        "name": "Ekle: geometrisiz tablo; türsüz katman, alansız",
        "steps": [step("execute", {"operation": "add", "name": "Malikler", "data": OWNERS}, WRITTEN,
                       {"layers": inserted(tree(), new_layer("Malikler", OWNERS))})],
    })
    out.append({
        "name": "Ekle: her türden geometri tutan sütun; katman türsüz kalır",
        "steps": [step("execute", {"operation": "add", "name": "Karışık", "data": ANY}, WRITTEN,
                       {"layers": inserted(tree(), new_layer("Karışık", ANY))})],
    })
    out.append({
        "name": "Ekle: index sondan büyükse sona; bilinmeyen parent ağacın en üstü",
        "steps": [step("execute", {"operation": "add", "name": "Yollar", "parent": "yok", "index": 99, "data": ROADS, "fields": ROAD_FIELDS},
                       WRITTEN, {"layers": inserted(tree(), copy.deepcopy(roads))})],
    })
    moved = {**POLES, "address": "/yeni/altyapi.gpkg", "relative": "yeni/altyapi.gpkg"}
    out.append({
        "name": "Değiştir: adı ve taşınan dosyanın yeni yeri tek adımda; adımı “Veri katmanını değiştir”, geri alınır",
        "steps": [
            step("execute", {"operation": "update", "layer": "direkler", "name": " Elektrik direkleri ", "data": moved}, done("direkler"),
                 {"layers": changed(tree(), "direkler", name="Elektrik direkleri", data=moved), "canUndo": True, "revision": "changed"}),
            step("undo", returns=LABEL["update"], expect={"layers": tree()}),
            step("redo", returns=LABEL["update"], expect={"layers": changed(tree(), "direkler", name="Elektrik direkleri", data=moved)}),
        ],
    })
    more = PARCEL_FIELDS + [{"name": "alan", "kind": "decimal"}]
    out.append({
        "name": "Değiştir: yalnız alanlar (tablo yeniden okundu)",
        "steps": [step("execute", {"operation": "update", "layer": "parseller", "fields": more}, done("parseller"),
                       {"layers": changed(tree(), "parseller", fields=more), "canUndo": True})],
    })
    points = {**PARCELS, "geometry": {"column": "geom", "typeName": "MULTIPOINT", "srid": 5256}}
    out.append({
        "name": "Değiştir: tablonun geometrisi değişti; katmanın türü onu izler",
        "steps": [step("execute", {"operation": "update", "layer": "parseller", "data": points}, done("parseller"),
                       {"layers": changed(tree(), "parseller", data=points)})],
    })
    out.append({
        "name": "Değiştir: aynı değerler bir şey yazmaz",
        "steps": [step("execute", {"operation": "update", "layer": "parseller", "name": "Parseller", "data": PARCELS}, done("parseller"), UNCHANGED)],
    })
    out.append({
        "name": "Kaldır: veri katmanı gider, tablosu kaynağında kalır; adımı “Veri katmanını kaldır”, geri alınır, yerine döner",
        "steps": [
            step("execute", {"operation": "remove", "layer": "direkler"}, done("direkler"),
                 {"layers": removed(tree(), "direkler"), "canUndo": True, "revision": "changed"}),
            step("undo", returns=LABEL["remove"], expect={"layers": tree()}),
        ],
    })
    active = copy.deepcopy(SETUP)
    active["activeLayer"] = "parseller"
    out.append({
        "name": "Kaldır: etkin katman kaldırılmaz; belgenin sözüyle reddedilir",
        "setup": active,
        "steps": [step("execute", {"operation": "remove", "layer": "parseller"},
                       failed("layer_refused", "“Parseller” etkin katman; silinemez. Önce başka bir katmanı etkinleştirin.", "layer"),
                       {**UNCHANGED, "layers": tree()})],
    })
    out.append({
        "name": "Doğrula: yazmaz",
        "steps": [step("validate", {"operation": "add", "name": "Yollar", "data": ROADS}, {"status": "completed", "output": None, "warnings": []}, UNCHANGED)],
    })
    out.append({
        "name": "Plan: eklenecek katman; yazmaz",
        "steps": [step("plan", {"operation": "add", "name": "Yollar", "data": ROADS, "fields": ROAD_FIELDS},
                       completed({"node": copy.deepcopy(roads), "revision": "$current"}), {**UNCHANGED, "layers": tree()})],
    })
    planned = changed(tree(), "direkler", name="Elektrik direkleri", data=moved)
    out.append({
        "name": "Plan: değişecek katman; yazmaz",
        "steps": [step("plan", {"operation": "update", "layer": "direkler", "name": "Elektrik direkleri", "data": moved},
                       completed({"node": node_of(planned, "direkler"), "revision": "$current"}), UNCHANGED)],
    })
    out.append({
        "name": "Plan: kaldırmada katman yok",
        "steps": [step("plan", {"operation": "remove", "layer": "parseller"}, completed({"revision": "$current"}), UNCHANGED)],
    })
    # The refusals, in the contract's order.
    out.append(refused("katmanı olmayan değiştirme", {"operation": "update", "name": "Yollar"}, "no_layer",
                       "Değiştirilecek ya da kaldırılacak katmanın kimliğini (layer) verin.", "layer"))
    out.append(refused("katmanı olmayan kaldırma", {"operation": "remove"}, "no_layer",
                       "Değiştirilecek ya da kaldırılacak katmanın kimliğini (layer) verin.", "layer"))
    out.append(refused("boş ad (yalnız boşluk)", {"operation": "add", "name": " \t", "data": ROADS}, "empty_name",
                       "Katmanın adı boş olamaz; bir ad verin.", "name"))
    out.append(refused("değiştirmede boş ad", {"operation": "update", "layer": "parseller", "name": ""}, "empty_name",
                       "Katmanın adı boş olamaz; bir ad verin.", "name"))
    out.append(refused("tablosuz ekleme", {"operation": "add", "name": "Boş"}, "no_data",
                       "Eklenecek veri tablosu (data) verilmedi: sürücüyü, kaynağın adresini, tabloyu ve anahtarı verin.", "data"))
    out.append(refused("tanınmayan sürücü", {"operation": "add", "name": "Oracle", "data": {**ROADS, "driver": "oracle"}}, "invalid_data",
                       "veri katmanının sürücüsü “oracle” olamaz: gpkg ya da postgis", "data"))
    out.append(refused("adreste parola", {"operation": "add", "name": "Yollar", "data": {**ROADS, "address": "postgresql://kbs:gizli@db.ornek.bel.tr:5432/kent"}},
                       "invalid_data", PASSWORD, "data"))
    out.append(refused("küçük harfli geometri türü", {"operation": "add", "name": "Yollar", "data": {**ROADS, "geometry": {"column": "geom", "typeName": "MultiLineString", "srid": 5256}}},
                       "invalid_data", "geometri türü “MultiLineString” tanınmıyor (büyük harfle OGC adı: POINT, MULTIPOLYGON …)", "data"))
    out.append(refused("boş tablo adı", {"operation": "add", "name": "Yollar", "data": {**ROADS, "table": "  "}}, "invalid_data",
                       f"veri katmanının tablosu boş ya da {MAX_TEXT} karakterden uzun", "data"))
    # A query's text counts its bytes as the contract does (UTF-8): “ı” and “ş” are two each.
    long_query = "-- kentos: key=fid\nSELECT fid FROM kadastro.parseller WHERE " + " OR ".join(
        f"mahalle = 'Işıklar {i}'" for i in range(90))
    assert len(long_query.encode()) > MAX_TEXT
    out.append(refused("2048 bayttan uzun sorgu", {"operation": "add", "name": "Sorgu", "data": {**QUERY, "table": long_query}},
                       "invalid_data", f"veri katmanının tablosu boş ya da {MAX_TEXT} karakterden uzun", "data"))
    out.append(refused("veritabanının göreli yolu", {"operation": "add", "name": "Yollar", "data": {**ROADS, "relative": "a/b"}}, "invalid_data",
                       "veritabanının göreli yolu olmaz", "data"))
    out.append(refused("aynı adlı iki alan", {"operation": "add", "name": "Yollar", "data": ROADS, "fields": [{"name": "ad", "kind": "text"}, {"name": "AD", "kind": "text"}]},
                       "invalid_fields", "“AD” adlı iki alan var; alan adları bir kez kullanılır.", "fields"))
    out.append(refused("geçersiz beklenen sürüm", {"operation": "add", "name": "Yollar", "data": ROADS, "expectedRevision": "on iki"}, "invalid_revision",
                       "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    out.append({
        "name": "sürüm çakışması: plan edilen sürümden sonra çizim değişmişse hiçbir şey yazılmaz",
        "steps": [
            step("captureRevision", **{"as": "once"}),
            step("execute", {"operation": "update", "layer": "parseller", "name": "Kadastro"}, done("parseller"), {"revision": "changed"}),
            step("execute", {"operation": "remove", "layer": "direkler", "expectedRevision": "$once"},
                 {"status": "conflict", "error": {"code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.", "path": "expectedRevision", "revision": "$current"}},
                 {"revision": "same", "layers": changed(tree(), "parseller", name="Kadastro")}),
        ],
    })
    out.append(refused("çizimde olmayan katman", {"operation": "remove", "layer": "yok"}, "layer_not_found",
                       "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layer"))
    out.append(refused("grup", {"operation": "update", "layer": "veri", "name": "Veriler"}, "not_a_layer",
                       "“Veri” bir katman grubu; veri tablosu yalnız katmanın olur.", "layer"))
    out.append(refused("veri katmanı olmayanı değiştirme", {"operation": "update", "layer": "0", "data": ROADS}, "not_a_data_layer",
                       "“0” bir veri katmanı değil; bu komut yalnız veri katmanını değiştirir ya da kaldırır.", "layer"))
    out.append(refused("veri katmanı olmayanı kaldırma", {"operation": "remove", "layer": "0"}, "not_a_data_layer",
                       "“0” bir veri katmanı değil; bu komut yalnız veri katmanını değiştirir ya da kaldırır.", "layer"))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.data",
        "commandVersion": 1,
        "title": "Veri katmanı: GeoPackage ya da PostGIS tablosu ekle, değiştir, kaldır (ADR 9101 §4)",
        "note": "scripts/fixtures/layer_data_command_cases.py yazar; beklentiler sözleşmenin kurallarından (crates/shared/commands/src/gis/cad_layers_data.rs, crates/shared/contracts/src/gis/data_layer.rs) KentOS kodu olmadan kurulur. `$layer` komutun eklediği (plan: ekleyeceği) katmanın kimliğidir: ağaçta olmayan bir `layer-N`; `expect.layers` katman ağacının tamamıdır.",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırıp farkı okuyun.")
            return 1
        print(f"cad.layers.data durumları güncel: {len(build()['cases'])} durum.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
