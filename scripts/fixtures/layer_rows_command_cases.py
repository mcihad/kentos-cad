"""Writes fixtures/data/v1/satirlar.gpkg and fixtures/commands/v1/cad.layers.rows.json: the shared cases of
`cad.layers.rows` v1 (docs/adr/9102 §4). The GeoPackage is written here with sqlite3 (OGC 12-128r19: its tables, the
`GP` blobs with little-endian WKB, the R*Tree index), and every expectation is worked out here from the contract's
rules (crates/shared/commands/src/gis/cad_layers_rows.rs): the Turkish alphabet's order (docs/adr/9102 §3), the
search folded the Turkish way, the filters' meaning, the page, the count, the refusals and their order. No KentOS code
is used. The desktop (crates/native/application/tests/all/fixtures) runs them.

    python3 scripts/fixtures/layer_rows_command_cases.py           # writes both files
    python3 scripts/fixtures/layer_rows_command_cases.py --check   # compares the cases with the ones on disk
"""
import json
import sqlite3
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GPKG = ROOT / "fixtures/data/v1/satirlar.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.layers.rows.json"
ADDRESS = "$fixture:data/v1/satirlar.gpkg"

MAHALLE = ["Çınar", "Cumhuriyet", "İnönü", "Işıklar", "Esentepe", "Şehitler", "Selçuklu", "Ümraniye", "Ulaş", "Ağaçlı"]
KULLANIM = ["Konut", "Park", "Ticaret", "Eğitim"]
X0, Y0, CELL = 600000.0, 4410000.0, 50.0


def rows():
    """The table's 30 rows: key, ada, parsel, mahalle, alan (decimal), kullanim, the square's corner and side."""
    out = []
    for i in range(30):
        side = 20.0 + (i * 7) % 15
        out.append({
            "fid": i + 1,
            "ada": 100 + i // 10,
            "parsel": i % 10 + 1,
            "mahalle": MAHALLE[(i * 3) % len(MAHALLE)],
            "alan": round(side * side + (i % 4) * 0.25, 2),
            "kullanim": KULLANIM[(i * 5 // 3) % 4] if i % 9 != 8 else None,
            "x": X0 + (i % 6) * CELL,
            "y": Y0 + (i // 6) * CELL,
            "side": side,
        })
    return out


ROWS = rows()


def gp_polygon(x, y, s):
    """A square as a GeoPackage blob: `GP`, version 0, flags (little-endian, envelope [minx, maxx, miny, maxy]), SRS
    5256, then the little-endian WKB polygon."""
    ring = [(x, y), (x + s, y), (x + s, y + s), (x, y + s), (x, y)]
    wkb = struct.pack("<BII", 1, 3, 1) + struct.pack("<I", len(ring)) + b"".join(struct.pack("<dd", *p) for p in ring)
    header = b"GP" + bytes([0, 0b00000011]) + struct.pack("<i", 5256) + struct.pack("<dddd", x, x + s, y, y + s)
    return header + wkb


def write_gpkg():
    GPKG.parent.mkdir(parents=True, exist_ok=True)
    if GPKG.exists():
        GPKG.unlink()
    db = sqlite3.connect(GPKG)
    db.executescript(
        """
        PRAGMA application_id = 1196444487;
        PRAGMA user_version = 10400;
        CREATE TABLE gpkg_spatial_ref_sys (srs_name TEXT NOT NULL, srs_id INTEGER PRIMARY KEY, organization TEXT NOT NULL,
            organization_coordsys_id INTEGER NOT NULL, definition TEXT NOT NULL, description TEXT);
        INSERT INTO gpkg_spatial_ref_sys VALUES ('Undefined cartesian SRS', -1, 'NONE', -1, 'undefined', NULL),
            ('Undefined geographic SRS', 0, 'NONE', 0, 'undefined', NULL),
            ('WGS 84 geodetic', 4326, 'EPSG', 4326, 'GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]]', NULL),
            ('TUREF / TM36', 5256, 'EPSG', 5256, 'PROJCS["TUREF / TM36",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,298.257222101]],PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]],PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],PARAMETER["central_meridian",36],PARAMETER["scale_factor",1],PARAMETER["false_easting",500000],PARAMETER["false_northing",0],UNIT["metre",1]]', NULL);
        CREATE TABLE gpkg_contents (table_name TEXT NOT NULL PRIMARY KEY, data_type TEXT NOT NULL, identifier TEXT UNIQUE,
            description TEXT DEFAULT '', last_change DATETIME NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
            min_x DOUBLE, min_y DOUBLE, max_x DOUBLE, max_y DOUBLE, srs_id INTEGER);
        CREATE TABLE gpkg_geometry_columns (table_name TEXT NOT NULL, column_name TEXT NOT NULL,
            geometry_type_name TEXT NOT NULL, srs_id INTEGER NOT NULL, z TINYINT NOT NULL, m TINYINT NOT NULL,
            CONSTRAINT pk_geom_cols PRIMARY KEY (table_name, column_name));
        CREATE TABLE gpkg_extensions (table_name TEXT, column_name TEXT, extension_name TEXT NOT NULL,
            definition TEXT NOT NULL, scope TEXT NOT NULL);
        CREATE TABLE parseller (fid INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, geom POLYGON, ada INTEGER,
            parsel INTEGER, mahalle TEXT, alan DOUBLE, kullanim TEXT);
        CREATE VIRTUAL TABLE rtree_parseller_geom USING rtree(id, minx, maxx, miny, maxy);
        """
    )
    for r in ROWS:
        s = r["side"]
        db.execute(
            "INSERT INTO parseller (fid, geom, ada, parsel, mahalle, alan, kullanim) VALUES (?, ?, ?, ?, ?, ?, ?)",
            (r["fid"], gp_polygon(r["x"], r["y"], s), r["ada"], r["parsel"], r["mahalle"], r["alan"], r["kullanim"]),
        )
        db.execute("INSERT INTO rtree_parseller_geom VALUES (?, ?, ?, ?, ?)",
                   (r["fid"], r["x"], r["x"] + s, r["y"], r["y"] + s))
    xs = [r["x"] for r in ROWS] + [r["x"] + r["side"] for r in ROWS]
    ys = [r["y"] for r in ROWS] + [r["y"] + r["side"] for r in ROWS]
    db.execute("INSERT INTO gpkg_contents (table_name, data_type, identifier, last_change, min_x, min_y, max_x, max_y, srs_id) "
               "VALUES ('parseller', 'features', 'parseller', '2026-11-21T00:00:00.000Z', ?, ?, ?, ?, 5256)",
               (min(xs), min(ys), max(xs), max(ys)))
    db.execute("INSERT INTO gpkg_geometry_columns VALUES ('parseller', 'geom', 'POLYGON', 5256, 0, 0)")
    db.execute("INSERT INTO gpkg_extensions VALUES ('parseller', 'geom', 'gpkg_rtree_index', "
               "'http://www.geopackage.org/spec120/#extension_rtree', 'write-only')")
    db.commit()
    db.close()


# The Turkish alphabet's order (docs/adr/9102 §3), written again here.
ALPHABET = "abcçdefgğhıijklmnoöpqrsştuüvwxyz"
CIRCUMFLEX = {"â": "a", "Â": "a", "î": "i", "Î": "i", "û": "u", "Û": "u"}


def plain(c):
    if c in CIRCUMFLEX:
        return CIRCUMFLEX[c]
    return {"İ": "i", "I": "ı"}.get(c, c.lower())


def tr_key(text):
    first = []
    for c in text:
        if c.isdigit() and c.isascii():
            first.append((0, ord(c)))
        elif plain(c) in ALPHABET:
            first.append((1, ALPHABET.index(plain(c))))
        else:
            first.append((2, ord(plain(c))))
    return (first, [c in CIRCUMFLEX for c in text], [c.isupper() for c in text], text.encode())


def fold(text):
    return "".join({"İ": "i", "I": "ı"}.get(c, c.lower()) for c in text)


def number_text(v):
    """A number as the source's value reads: an integer as is, a double in its shortest form without `.0`."""
    if isinstance(v, int):
        return str(v)
    return str(int(v)) if float(v).is_integer() else repr(float(v))


FIELDS = ["ada", "parsel", "mahalle", "alan", "kullanim"]


def text_of(r, f):
    v = r[f]
    if v is None:
        return None
    return v if isinstance(v, str) else number_text(v)


def ordered(rs, order):
    """`order` then the key; an empty value after every one, either way."""
    def cmp_key(r):
        parts = []
        for field, desc in order:
            v = r[field]
            if v is None:
                parts.append((1, 0))
                continue
            k = tr_key(v) if isinstance(v, str) else v
            parts.append((0, Desc(k) if desc else k))
        parts.append(r["fid"])
        return parts
    return sorted(rs, key=cmp_key)


class Desc:
    def __init__(self, k):
        self.k = k

    def __lt__(self, o):
        return o.k < self.k

    def __eq__(self, o):
        return self.k == o.k


def contains(r, words):
    w = fold(words)
    return any(t is not None and w in fold(t) for t in (text_of(r, f) for f in FIELDS))


def meets(r, b):
    return not (r["x"] + r["side"] < b[0] or r["x"] > b[2] or r["y"] + r["side"] < b[1] or r["y"] > b[3])


def answer(inp, keep=lambda r: True, checked="source"):
    """The answer the contract gives for `inp`, the filter's meaning being `keep`."""
    rs = [r for r in ROWS if keep(r)]
    if "bbox" in inp:
        rs = [r for r in rs if meets(r, inp["bbox"])]
    if "keys" in inp:
        rs = [r for r in rs if r["fid"] in inp["keys"]]
    if inp.get("search", "").strip():
        rs = [r for r in rs if contains(r, inp["search"].strip())]
    rs = ordered(rs, [(o["field"], o.get("descending", False)) for o in inp.get("order", [])])
    offset, limit = inp.get("offset", 0), inp.get("limit", 100)
    fields = inp.get("fields", FIELDS)
    page = []
    for r in rs[offset:offset + limit]:
        row = {"key": r["fid"], "attrs": {f: text_of(r, f) for f in fields if text_of(r, f) is not None}}
        if inp.get("geometry"):
            s = r["side"]
            pts = [(r["x"], r["y"]), (r["x"] + s, r["y"]), (r["x"] + s, r["y"] + s), (r["x"], r["y"] + s), (r["x"], r["y"])]
            row["wkt"] = "POLYGON ((" + ", ".join(f"{number_text(x)} {number_text(y)}" for x, y in pts) + "))"
        page.append(row)
    out = {"fields": fields, "rows": page, "offset": offset}
    if inp.get("count"):
        out["total"] = len(rs)
    if inp.get("allKeys"):
        out["keys"] = [r["fid"] for r in rs]
    out["checked"] = checked
    return {"status": "completed", "output": out, "warnings": []}


def refused(code, message, path):
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Veri katmanının satırları",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 600000, "y": 4410000},
    "layers": [
        {"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": []},
        {"id": "parseller", "name": "Parseller", "type": "layer", "visible": True, "locked": False, "expanded": True,
         "style": STYLE, "children": [], "geometry": "area",
         "data": {"driver": "gpkg", "address": ADDRESS, "table": "parseller", "key": "fid",
                  "geometry": {"column": "geom", "typeName": "POLYGON", "srid": 5256}},
         "fields": [{"name": "ada", "kind": "integer"}, {"name": "parsel", "kind": "integer"},
                    {"name": "mahalle", "kind": "text"}, {"name": "alan", "kind": "decimal"},
                    {"name": "kullanim", "kind": "text"}]},
    ],
    "activeLayer": "cizim",
    "entities": [],
    "styles": {"items": [], "categories": []},
}


def case(name, inp, result):
    return {"name": name, "steps": [{"op": "execute", "input": inp, "result": result}]}


def area(r):
    return r["side"] * r["side"]


def cases():
    p = {"layer": "parseller"}
    out = [
        case("İlk sayfa: bütün satırlar anahtar sırasıyla, sayısıyla", {**p, "count": True}, answer({**p, "count": True})),
        case("Mahalleye göre Türk alfabesiyle: Ağaçlı, Cumhuriyet, Çınar, Esentepe, Işıklar, İnönü …",
             {**p, "order": [{"field": "mahalle"}], "limit": 12, "fields": ["mahalle", "ada"]},
             answer({**p, "order": [{"field": "mahalle"}], "limit": 12, "fields": ["mahalle", "ada"]})),
        case("Mahalle azalan, sonra ada: eşitler anahtarla",
             {**p, "order": [{"field": "mahalle", "descending": True}, {"field": "ada"}], "limit": 10},
             answer({**p, "order": [{"field": "mahalle", "descending": True}, {"field": "ada"}], "limit": 10})),
        case("Kullanım: boş değerler iki yönde de sonda",
             {**p, "order": [{"field": "kullanim", "descending": True}], "offset": 22, "limit": 8, "fields": ["kullanim"]},
             answer({**p, "order": [{"field": "kullanim", "descending": True}], "offset": 22, "limit": 8,
                     "fields": ["kullanim"]})),
        case("Alana göre azalan, ikinci sayfa", {**p, "order": [{"field": "alan", "descending": True}], "offset": 5,
                                                 "limit": 5},
             answer({**p, "order": [{"field": "alan", "descending": True}], "offset": 5, "limit": 5})),
        case("Arama Türkçe katlanır: “çınar” Çınar'ı bulur", {**p, "search": "çınar", "count": True},
             answer({**p, "search": "çınar", "count": True})),
        case("Arama büyük harfle: “İNÖNÜ” İnönü'yü, “IŞIK” Işıklar'ı bulur, “isik” bulmaz",
             {**p, "search": "İNÖNÜ", "count": True, "limit": 3}, answer({**p, "search": "İNÖNÜ", "count": True, "limit": 3})),
        case("Arama: “IŞIK”", {**p, "search": "IŞIK", "count": True, "limit": 0},
             answer({**p, "search": "IŞIK", "count": True, "limit": 0})),
        case("Arama: “isik” Işıklar'ı bulmaz (ı ile i ayrı harf)", {**p, "search": "isik", "count": True, "limit": 0},
             answer({**p, "search": "isik", "count": True, "limit": 0})),
        case("Arama sayılarda da: “10” adası ve parseli", {**p, "search": "10", "count": True, "fields": ["ada", "parsel"]},
             answer({**p, "search": "10", "count": True, "fields": ["ada", "parsel"]})),
        case("Seçililer: anahtarlar", {**p, "keys": [3, 7, 11, 40], "count": True},
             answer({**p, "keys": [3, 7, 11, 40], "count": True})),
        case("Görünümdekiler: kutuya değenler", {**p, "bbox": [600040.0, 4410040.0, 600160.0, 4410110.0], "count": True,
                                                 "fields": ["ada", "parsel"]},
             answer({**p, "bbox": [600040.0, 4410040.0, 600160.0, 4410110.0], "count": True, "fields": ["ada", "parsel"]})),
        case("Geometri projenin sisteminde WKT", {**p, "keys": [1, 8], "geometry": True, "fields": ["parsel"]},
             answer({**p, "keys": [1, 8], "geometry": True, "fields": ["parsel"]})),
        case("Yalnız sayı: limit 0", {**p, "limit": 0, "count": True}, answer({**p, "limit": 0, "count": True})),
        case("Bütün anahtarlar sırasıyla", {**p, "order": [{"field": "mahalle"}], "limit": 0, "allKeys": True},
             answer({**p, "order": [{"field": "mahalle"}], "limit": 0, "allKeys": True})),
        case("Süzgeç: kullanim = 'Konut' (burada sorulur)", {**p, "filter": "kullanim = 'Konut'", "count": True},
             answer({**p, "filter": "kullanim = 'Konut'", "count": True}, lambda r: r["kullanim"] == "Konut", "here")),
        case("Süzgeç ve sıra: alan > 500 ve kullanim = 'Park', alana göre azalan",
             {**p, "filter": "alan > 500 ve kullanim = 'Park'", "order": [{"field": "alan", "descending": True}],
              "count": True, "allKeys": True},
             answer({**p, "filter": "alan > 500 ve kullanim = 'Park'", "order": [{"field": "alan", "descending": True}],
                     "count": True, "allKeys": True},
                    lambda r: r["alan"] > 500 and r["kullanim"] == "Park", "here")),
        # The case runner reads a text starting with `$` as its own reference: the measure is not the filter's first word.
        case("Süzgeç geometriyi ölçer: ada > 0 ve $alan > 900, ikinci sayfa",
             {**p, "filter": "ada > 0 ve $alan > 900", "offset": 3, "limit": 3, "count": True},
             answer({**p, "filter": "ada > 0 ve $alan > 900", "offset": 3, "limit": 3, "count": True},
                    lambda r: r["ada"] > 0 and area(r) > 900, "here")),
        case("Süzgeç, arama ve kutu birlikte",
             {**p, "filter": "ada >= 101", "search": "e", "bbox": [600000.0, 4410000.0, 600300.0, 4410150.0], "count": True,
              "fields": ["ada", "mahalle"]},
             answer({**p, "filter": "ada >= 101", "search": "e", "bbox": [600000.0, 4410000.0, 600300.0, 4410150.0],
                     "count": True, "fields": ["ada", "mahalle"]}, lambda r: r["ada"] >= 101, "here")),
        case("Ret: katman verilmedi", {"layer": " "},
             refused("no_layer", "Satırları okunacak veri katmanını (layer) verin.", "layer")),
        case("Ret: sayfa 1 000'i geçer", {**p, "limit": 1001},
             refused("invalid_count", "Bir sayfa en çok 1000 satır tutar; 1001 istendi.", "limit")),
        case("Ret: ters kutu", {**p, "bbox": [600100.0, 4410000.0, 600000.0, 4410100.0]},
             refused("invalid_extent", "Kutu batı, güney, doğu, kuzey sırasıyla sonlu dört sayı olmalı; batı doğuyu, "
                     "güney kuzeyi geçemez.", "bbox")),
        case("Ret: katman yok", {"layer": "yok"}, refused("layer_not_found", "“yok” kimlikli katman çizimde yok.", "layer")),
        case("Ret: veri katmanı değil", {"layer": "cizim"},
             refused("not_a_data_layer", "“Çizim” bir veri katmanı değil; satırları kaynaktan okunmaz.", "layer")),
        case("Ret: sıranın alanı yok", {**p, "order": [{"field": "ada"}, {"field": "kat"}]},
             refused("field_not_found", "“kat” alanı “Parseller” katmanında yok.", "order[1].field")),
        case("Ret: dönen alan yok", {**p, "fields": ["ada", "malik"]},
             refused("field_not_found", "“malik” alanı “Parseller” katmanında yok.", "fields[1]")),
        # The compiler's own words are its own: any message.
        case("Ret: süzgeç derlenmiyor", {**p, "filter": "kullanim ="},
             refused("invalid_expression", "$any", "filter")),
    ]
    # A table whose file is not there: the source is not reached.
    gone = json.loads(json.dumps(SETUP))
    gone["layers"][1]["data"]["address"] = "$fixture:data/v1/yok.gpkg"
    out.append({"name": "Ret: kaynak okunamıyor (dosya yok)", "setup": gone,
                "steps": [{"op": "execute", "input": {**p}, "result": refused("source_unreachable", "$any", None)}]})
    return out


def document():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.layers.rows",
        "commandVersion": 1,
        "title": "Veri katmanının satırları: sıra, sayfa, arama, süzgeç, kutu, seçili anahtarlar (ADR 9102 §4)",
        "note": "ADR 9102. GeoPackage'ı bu betik sqlite3 ile yazar (fixtures/data/v1/satirlar.gpkg); beklentiler "
                "sözleşmenin kurallarıyla burada hesaplanır: Türk alfabesinin sırası, Türkçe katlanan arama, süzgecin "
                "anlamı, sayfa, sayı, retler.",
        "setup": SETUP,
        "cases": cases(),
    }


def main():
    text = json.dumps(document(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        same = OUT.read_text() == text
        print("aynı" if same else f"{OUT} farklı")
        sys.exit(0 if same else 1)
    write_gpkg()
    OUT.write_text(text)
    print(f"yazıldı: {GPKG.relative_to(ROOT)}, {OUT.relative_to(ROOT)} ({len(cases())} durum)")


if __name__ == "__main__":
    main()
