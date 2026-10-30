#!/usr/bin/env python3
"""The shared cases of a bound table's foreign keys and of the defaults its database works out (docs/adr/0753): what
a bound layer's fields take from the table's columns when it is bound and at every reading of it.

    python3 scripts/fixtures/source_references_cases.py           # writes fixtures/commands/v1/cad.sources.references.json
    python3 scripts/fixtures/source_references_cases.py --check   # compares
    /usr/bin/python3 scripts/fixtures/source_references_cases.py --files   # GDAL and sqlite3 write baglar*.gpkg

The table's columns are read here with Python's sqlite3 (PRAGMA table_info and foreign_key_list, the referenced tables'
rows), not with KentOS code, and ADR 0753's rules are written out again below:

- A column's default the database works out (neither a literal, ADR 0750, nor NULL) is the field's `column.made`, its
  text as the table gives it, every run of white space or control characters one space (ADR 0754: a generated column's
  expression over several lines reads on one), trimmed; past 1000 characters cut to 999 and “…”. A clause of a column
  the table makes itself (`GENERATED …`, ADR 0754) is never a literal nor NULL.
- A single-column foreign key is the field's `column.refers`: the referenced table, its column (the primary key when the
  key names none) and the column its rows are named by: of its text columns (SQLite's text affinity: a declared type
  without INT that has CHAR, CLOB or TEXT), the referenced one left out, the first whose name, lowercased with ı ş ğ ü ö
  ç as i s g u o c, is ad, adi, isim, name, baslik, unvan, title, label or tanim, else the first; none without one. A key
  of several columns, or one whose table is not in the file, gives nothing.
- A field of a text or a number gets the referenced rows as its value list, at most 10 000 (more: no list, said): the
  rows (their key not NULL) in the natural order of their keys, then their labels; each key in the field's kind
  (layer_field_cases.py's check; one that does not fit left out and said, one met again once); its label trimmed, an
  empty or missing one the code; a label no other repeats (folded the Turkish way) stays, one that repeats says its code
  in parentheses, again while that is taken; the list in the natural order of its labels, then its codes
  (feature_table_cases.py's order).
- Reading the table again writes them again: the made default as the table says it; the list when the field holds the
  mirrored one or none of its own; a key gone takes its list with it; a list of the field's own stays.

The objects and the binding come from bound_sources_cases.py's link (GDAL's OGR reads the rows).
"""
import json
import sqlite3
import sys
import unicodedata
from collections import Counter
from functools import cmp_to_key
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import feature_table_cases as ft  # noqa: E402
import layer_field_cases as lf  # noqa: E402
import source_defaults_cases as sd  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/cad.sources.references.json"
B, B2, B3 = "baglar.gpkg", "baglar-2.gpkg", "baglar-3.gpkg"
TABLE = "ilce"
LAYER = "import-ilce"
ANY = "$any"
MOST = 10_000
MAX_MADE = 1000
NAMES = ("ad", "adi", "isim", "name", "baslik", "unvan", "title", "label", "tanim")

# The district rows: name, province, upper district, class, code, many, region pair, made time, label, kind, note.
ROWS = [
    ("Merkez", 58, None, "A", 1, 1, 1, 1, "2026-01-01T10:00:00", "A1", "İlçe", None,
     [(487000, 4420000), (487040, 4420000), (487040, 4420030), (487000, 4420030)]),
    ("Suşehri", 58, 1, "B", 2, 2, 1, 2, "2026-02-01T10:00:00", "B1", "İlçe", "n",
     [(487050, 4420000), (487090, 4420000), (487090, 4420030), (487050, 4420030)]),
    ("Merkez", 6, None, "C", None, None, None, None, "2026-03-01T10:00:00", "C1", "İlçe", None,
     [(487100, 4420000), (487140, 4420000), (487140, 4420030), (487100, 4420030)]),
]
NAMED = ["ad", "il_id", "ust_id", "sinif", "kod_no", "cok_id", "bolge", "alt", "olusturma", "etiket", "tur", "notu"]
PROVINCES = [(6, "06", "Ankara"), (34, "34", "İstanbul"), (58, "58", "Sivas"), (7, "07", "Antalya"), (99, "99", None)]


def columns(variant):
    """The district table's columns; the third file's province column has no foreign key."""
    il = "" if variant == 3 else " REFERENCES il(id)"
    return f"""
  "ad" TEXT NOT NULL,
  "il_id" INTEGER{il},
  "ust_id" INTEGER REFERENCES ilce(fid),
  "sinif" TEXT REFERENCES sinif(kod),
  "kod_no" INTEGER REFERENCES kodlar(kod),
  "cok_id" INTEGER REFERENCES cok,
  "bolge" INTEGER,
  "alt" INTEGER,
  "olusturma" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "etiket" TEXT DEFAULT (upper('ilce')),
  "tur" TEXT DEFAULT 'İlçe',
  "notu" TEXT DEFAULT NULL,
  FOREIGN KEY ("bolge", "alt") REFERENCES bolgeler (a, b)
"""


def write_file(name, variant):
    """GDAL makes the GeoPackage and its three districts; sqlite3 makes the table again with its keys and defaults
    (GDAL writes neither) and the tables they refer to, the spatial index left out. The second file has one province
    more, the third no key on the province column."""
    from osgeo import ogr, osr
    path = DIR / name
    if path.exists():
        path.unlink()
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
    lyr = ds.CreateLayer(TABLE, srs=srs, geom_type=ogr.wkbPolygon, options=["SPATIAL_INDEX=NO"])
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for r in ROWS:
        f = ogr.Feature(lyr.GetLayerDefn())
        ring = r[-1]
        f.SetGeometry(ogr.CreateGeometryFromWkt(
            "POLYGON ((" + ", ".join(f"{x} {y}" for x, y in [*ring, ring[0]]) + "))"))
        f.SetField("ad", r[0])
        lyr.CreateFeature(f)
    ds = None
    c = sqlite3.connect(path)
    c.executescript(
        'CREATE TABLE il (id INTEGER PRIMARY KEY, plaka TEXT, "Adı" TEXT, nufus INTEGER);'
        "CREATE TABLE sinif (kod TEXT PRIMARY KEY, aciklama TEXT, sira INTEGER);"
        "CREATE TABLE kodlar (kod TEXT UNIQUE, sira INTEGER);"
        "CREATE TABLE cok (id INTEGER PRIMARY KEY, ad TEXT);"
        "CREATE TABLE bolgeler (a INTEGER, b INTEGER, ad TEXT, UNIQUE (a, b));"
        f'CREATE TABLE "{TABLE}_yeni" ("fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON, '
        f"{columns(variant)});")
    provinces = PROVINCES + ([(1, "01", "Adana")] if variant == 2 else [])
    c.executemany("INSERT INTO il VALUES (?, ?, ?, ?)", [(i, p, a, 1000 * i) for i, p, a in provinces])
    c.executemany("INSERT INTO sinif VALUES (?, ?, ?)", [("A", "birinci", 1), ("B", "ikinci", 2), ("C", None, 3)])
    c.executemany("INSERT INTO kodlar VALUES (?, ?)", [("1", 1), ("2", 2), ("03", 3), ("3", 4), ("A", 5)])
    c.executemany("INSERT INTO cok (id) VALUES (?)", [(i,) for i in range(1, MOST + 2)])
    c.executemany("INSERT INTO bolgeler VALUES (?, ?, ?)", [(1, 1, "Kuzey"), (1, 2, "Güney")])
    for fid, r in enumerate(ROWS, start=1):
        geom = c.execute(f'SELECT geom FROM "{TABLE}" WHERE fid = ?', (fid,)).fetchone()[0]
        c.execute(f'INSERT INTO "{TABLE}_yeni" VALUES (?, ?, {", ".join("?" * len(NAMED))})', (fid, geom, *r[:-1]))
    c.executescript(f'DROP TABLE "{TABLE}"; ALTER TABLE "{TABLE}_yeni" RENAME TO "{TABLE}";'
                    "DELETE FROM gpkg_ogr_contents WHERE table_name = 'ilce';")
    c.commit()
    c.execute("VACUUM")
    c.close()
    print(f"yazıldı: {name}")


def write_files():
    for name, variant in ((B, 1), (B2, 2), (B3, 3)):
        write_file(name, variant)


# ── ADR 0753's rules, written out again ──

def text_affinity(decl):
    up = (decl or "").upper()
    return "INT" not in up and any(w in up for w in ("CHAR", "CLOB", "TEXT"))


def plain(name):
    return "".join({"ı": "i", "ş": "s", "ğ": "g", "ü": "u", "ö": "o", "ç": "c"}.get(c, c)
                   for c in name.lower() if c != "̇")


def label_column(texts):
    return next((t for t in texts if plain(t) in NAMES), texts[0] if texts else None)


def references(c, table):
    """The table's single-column foreign keys: column → (referenced table, its column, label column, rows or None)."""
    keys = c.execute(f'PRAGMA foreign_key_list("{table}")').fetchall()
    out = {}
    for kid, _, parent, frm, to, *_ in keys:
        if sum(1 for k in keys if k[0] == kid) != 1:
            continue
        info = c.execute(f'PRAGMA table_info("{parent}")').fetchall()
        if not info:
            continue
        if to is None:
            pk = [r[1] for r in info if r[5] > 0]
            if len(pk) != 1:
                continue
            to = pk[0]
        texts = [r[1] for r in info if r[1] != to and text_affinity(r[2])]
        label = label_column(texts)
        lab = f'CAST("{label}" AS TEXT)' if label else "NULL"
        rows = c.execute(f'SELECT CAST("{to}" AS TEXT), {lab} FROM "{parent}" WHERE "{to}" IS NOT NULL').fetchall()
        out[frm] = (parent, to, label, rows if len(rows) <= MOST else None)
    return out


def listed(field, rows):
    """The referenced rows as the field's value list, and how many were left out."""
    probe = {"name": field["name"], "kind": field["kind"]}
    rows = sorted(rows, key=cmp_to_key(lambda a, b: ft.natural_cmp(a[0], b[0]) or ft.natural_cmp(a[1] or "", b[1] or "")))
    left, codes, items = 0, set(), []
    for key, label in rows:
        r = lf.check(probe, key)
        code = r.get("value")
        if "error" in r or not code:
            left += 1
            continue
        if code in codes:
            continue
        codes.add(code)
        items.append((code, ft.trim(label or "") or code))
    count = Counter(ft.folded(l) for _, l in items)
    taken = {k for k, n in count.items() if n == 1}
    out = []
    for code, l in items:
        if count[ft.folded(l)] == 1:
            label = l
        else:
            label = f"{l} ({code})"
            while ft.folded(label) in taken:
                label = f"{label} ({code})"
            taken.add(ft.folded(label))
        out.append({"code": code, "label": label})
    out.sort(key=cmp_to_key(lambda a, b: ft.natural_cmp(a["label"], b["label"]) or ft.natural_cmp(a["code"], b["code"])))
    return out, left


# Unicode's White_Space property (PropList.txt), written out.
WHITE_SPACE = frozenset(map(chr, [*range(0x09, 0x0E), 0x20, 0x85, 0xA0, 0x1680, *range(0x2000, 0x200B), 0x2028, 0x2029,
                                  0x202F, 0x205F, 0x3000]))
IDENTITY, IDENTITY_ALWAYS, GENERATED = "GENERATED BY DEFAULT AS IDENTITY", "GENERATED ALWAYS AS IDENTITY", "GENERATED ALWAYS"


def making(default):
    """How the table makes a column's value, as its default's text says it (ADR 0754 §1)."""
    if default == IDENTITY:
        return "identity"
    if default == IDENTITY_ALWAYS:
        return "identityAlways"
    if default is not None and default.startswith(GENERATED):
        return "generated"
    return "plain"


def blank(ch):
    return ch in WHITE_SPACE or unicodedata.category(ch) == "Cc"


def made_of(default):
    if default is None:
        return None
    if making(default) == "plain":
        if sd.literal(default) is not None:
            return None
        t = default.strip()
        while len(t) >= 2 and t.startswith("(") and t.endswith(")"):
            t = t[1:-1].strip()
        if t.upper() == "NULL" or t.upper().startswith("NULL::"):
            return None
    out = []
    for ch in default:
        if not blank(ch):
            out.append(ch)
        elif not out or out[-1] != " ":
            out.append(" ")
    text = ft.trim("".join(out))
    if not text:
        return None
    return text if len(text) <= MAX_MADE else ft.trim(text[:MAX_MADE - 1]) + "…"


def warn(message, at):
    return {"code": "reference_values_missing", "message": message, "path": at}


def fields(name, at="edit.table"):
    """The bound layer's fields from the table's own definition, and the warnings binding (or reading) says at `at`."""
    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    refs = references(c, TABLE)
    out, warnings = [], []
    for _, cname, decl, notnull, default, pk in c.execute(f'PRAGMA table_info("{TABLE}")'):
        if pk or cname == "geom":
            continue
        kind = bs.kind_of(decl)
        f = {"name": cname, "kind": kind}
        if notnull and default is None:
            f["required"] = True
        lit = sd.literal(default)
        value = sd.as_kind(kind, lit) if lit is not None else None
        if value is not None:
            f["default"] = value
        column = {}
        made = made_of(default)
        if made is not None:
            column["made"] = made
        if cname in refs:
            parent, to, label, rows = refs[cname]
            column["refers"] = {"table": parent, "column": to, **({"label": label} if label else {})}
            if kind in ("text", "integer", "decimal", "bigint"):
                if rows is None:
                    warnings.append(warn(f"“{cname}” alanının değerleri “{parent}” tablosundan listelenmedi: tabloda "
                                         f"{MOST:,} satırdan çok var; alan serbest kalır.".replace(",", " "), at))
                else:
                    values, left = listed(f, rows)
                    if left:
                        warnings.append(warn(f"“{cname}” alanının değer listesine “{parent}” tablosunun {left} "
                                             "satırı girmedi: anahtarları alanın türüne uymuyor.", at))
                    if values:
                        f["values"] = values
        if column:
            f["column"] = column
        assert lf.field_problem(f) is None, (f, lf.field_problem(f))
        out.append(f)
    c.close()
    return out, warnings


def follow(layer_fields, name):
    """The layer's fields after a reading of the table `name`: each takes the table's made default; the list when it
    holds the mirrored one or none of its own; the key gone, the list goes."""
    table, warnings = fields(name, "layer")
    by = {t["name"]: t for t in table}
    out = []
    for f in layer_fields:
        f = json.loads(json.dumps(f))
        t = by.get(f["name"])
        if t is not None:
            mirrored = "refers" in f.get("column", {})
            own = not mirrored and ("values" in f or "domain" in f)
            made = t.get("column", {}).get("made")
            refers = f.get("column", {}).get("refers") if own else t.get("column", {}).get("refers")
            if not own and (mirrored or refers):
                f.pop("values", None)
                if refers and "values" in t:
                    f["values"] = t["values"]
            column = {**({"made": made} if made else {}), **({"refers": refers} if refers else {})}
            if column:
                f["column"] = column
            else:
                f.pop("column", None)
        out.append(f)
    return out, warnings


# ── The cases ──

def path_of(n):
    return f"$tmp:baglar-{n}.gpkg"


def bound(n, extent=None):
    """The file copied and its table bound onto a new layer (with `extent` its part, the rows whose box meets it): the
    steps, the layer's next object id, its fields."""
    answer, warnings, objs, ids, b = bs.linked(TABLE, name=B)
    if extent is not None:
        keep = {TABLE: bs.in_area(B, TABLE, extent)}
        _, warnings, objs, ids, _ = bs.gi.imported(B, [TABLE], keep=keep)
        b = {**b, "extent": extent}
        answer = {**answer, "objects": len(ids)}
    path = path_of(n)
    b = {**b, "path": path}
    answer = {**answer, "link": b}
    layer_fields, said = fields(B)
    copy = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{B}", "to": path}
    edit = {"kind": "link", "path": path, "table": TABLE}
    if extent is not None:
        edit["extent"] = extent
    link = bs.step({"edit": edit}, bs.done(answer, [*warnings, *said]),
                   {"ids": [1, *ids], "fields": objs, "revision": "changed", "canUndo": True,
                    "settings": {"sources": [b]}, "nodes": {LAYER: {"fields": layer_fields}}},
                   command="cad.sources.edit")
    return [copy, link], max(ids) + 1, layer_fields


def over(n, name):
    """The bound copy replaced by another file of the same rows (its table changed outside KentOS)."""
    return {"op": "copyFile", "from": f"$fixture:geopackage/v1/{name}", "to": path_of(n)}


def read(name, layer_fields, n_said=None, op="execute"):
    after, said = follow(layer_fields, name)
    expect = bs.SAME if op == "plan" or after == layer_fields else {"revision": "changed",
                                                                    "nodes": {LAYER: {"fields": after}}}
    return bs.step({"layer": LAYER, "direction": "read"}, bs.done(bs.synced(name=name), said), expect, op=op,
                   command="cad.sources.sync"), after


def by_name(layer_fields, name):
    return next(f for f in layer_fields if f["name"] == name)


def with_field(layer_fields, name, change):
    out = json.loads(json.dumps(layer_fields))
    f = by_name(out, name)
    change(f)
    return out


def layers_edit(layer_fields):
    return bs.step({"operation": "fields", "layer": LAYER, "fields": layer_fields},
                   {"status": "completed", "output": ANY, "warnings": ANY},
                   {"revision": "changed", "nodes": {LAYER: {"fields": layer_fields}}}, command="cad.layers.edit")


def polygon(x, attrs):
    ring = [{"x": x, "y": 4420040.0}, {"x": x + 20.0, "y": 4420040.0}, {"x": x + 20.0, "y": 4420060.0},
            {"x": x, "y": 4420060.0}]
    return bs.other("cad.polygon.create", {"layerId": LAYER, "pts": ring, "attrs": attrs})


def cases():
    out = []
    n = 0

    def fresh(name, *steps, extent=None):
        nonlocal n
        n += 1
        s, _, _ = bound(n, extent)
        out.append(bs.case(name, *s, *steps))

    _, nxt, base = bound(0)
    il = by_name(base, "il_id")
    assert [v["label"] for v in il["values"]][:2] == ["99", "Ankara"], il
    # Binding: one case a rule (the whole field list said in each).
    fresh("bağlanınca yabancı anahtarın tablosunun satırları alanın değer listesi olur")
    fresh("etiket sütunu adı “ad” olan metin sütunu: Türkçe harfli “Adı” ilk metin sütunu “plaka”nın önünde")
    fresh("etiketi boş satırın etiketi kodudur")
    fresh("kendine başvuran sütun: aynı adlı satırlar kodlarıyla ayrılır")
    fresh("adı tanınmayan metin sütunlarından ilki etikettir; metin anahtar metin alanının kodu")
    fresh("metin sütunu olmayan tablonun etiketleri kodlardır; aynı sayıyı veren iki anahtar bir kez")
    fresh("alanın türüne uymayan anahtar listeye girmez, söylenir")
    fresh("10 000'den çok satırlı tablo: başvuru yazılır, liste kurulmaz, söylenir")
    fresh("sütun adı vermeyen anahtar başvurulan tablonun birincil anahtarına başvurur")
    fresh("iki sütunlu yabancı anahtar alınmaz")
    fresh("veritabanının hesapladığı varsayılan alanda yazılı: zaman damgası ve ifade")
    fresh("sabit varsayılan alanın varsayılanı kalır, NULL varsayılan bir şey vermez")
    fresh("alan sınırıyla bağlanan parça da listeleri alır; kendine başvuran listede alanın dışındaki satır da var",
          extent=[486990.0, 4419990.0, 487095.0, 4420035.0])
    # The plan says it and writes nothing.
    n += 1
    s, _, _ = bound(n)
    plan = dict(s[1])
    plan["op"] = "plan"
    plan["expect"] = {"ids": [1], "revision": "same", "canUndo": False, "dirty": False}
    out.append(bs.case("plan bağlamayı yazmaz, uyarıları söyler", s[0], plan))
    # Reading.
    n += 1
    s, _, fs = bound(n)
    step, _ = read(B, fs)
    out.append(bs.case("hemen okumak bir şey değiştirmez, uyarılar yine söylenir", *s, step))
    n += 1
    s, _, fs = bound(n)
    step, after = read(B2, fs)
    assert after != fs and len(by_name(after, "il_id")["values"]) == 6
    out.append(bs.case("başvurulan tabloya satır eklenince okumak listeyi yeniler", *s, over(n, B2), step))
    n += 1
    s, _, fs = bound(n)
    step, after = read(B3, fs)
    assert "column" not in by_name(after, "il_id") and "values" not in by_name(after, "il_id")
    out.append(bs.case("yabancı anahtar kalkınca okumak başvuruyu ve listesini kaldırır", *s, over(n, B3), step))
    n += 1
    s, _, fs = bound(n)
    own = with_field(fs, "sinif", lambda f: (f.pop("column"), f.update(values=[{"code": "A", "label": "Birinci"},
                                                                               {"code": "B", "label": "İkinci"}])))
    step, after = read(B2, own)
    assert by_name(after, "sinif") == by_name(own, "sinif")
    out.append(bs.case("alanın kendi listesi okunurken kalır", *s, layers_edit(own), over(n, B2), step))
    n += 1
    s, _, fs = bound(n)
    edited = with_field(fs, "il_id", lambda f: f.update(values=[{"code": "6", "label": "Başkent"}]))
    step, after = read(B, edited)
    assert by_name(after, "il_id")["values"] == by_name(fs, "il_id")["values"]
    out.append(bs.case("elle değiştirilen bağlı liste okununca tablonunkine döner", *s, layers_edit(edited), step))
    n += 1
    s, _, fs = bound(n)
    step, after = read(B2, fs)
    undo = {"op": "undo", "returns": f"Kaynaktan oku: {TABLE}", "expect": {"nodes": {LAYER: {"fields": fs}}}}
    redo = {"op": "redo", "returns": f"Kaynaktan oku: {TABLE}", "expect": {"nodes": {LAYER: {"fields": after}}}}
    out.append(bs.case("okumayı geri almak listeyi geri getirir, yinelemek yeniden yazar", *s, over(n, B2), step, undo,
                       redo))
    n += 1
    s, _, fs = bound(n)
    step, _ = read(B2, fs, op="plan")
    out.append(bs.case("okumanın planı alanları yazmaz", *s, over(n, B2), step))
    # Values the list holds.
    n += 1
    s, nxt, fs = bound(n)
    wrong = lf.check(by_name(fs, "il_id"), "12")
    assert "error" in wrong
    out.append(bs.case("listede olmayan değer nesneye yazılmaz", *s,
                       bs.step({"uids": ["$uidOf:2"], "attrs": {"il_id": "12"}, "operation": "attributes"},
                               bs.failed("invalid_attribute", wrong["message"], "attrs.il_id"),
                               command="cad.entities.set")))
    n += 1
    s, nxt, fs = bound(n)
    out.append(bs.case("listeden seçilen değer yazılır ve tabloya gider", *s,
                       bs.other("cad.entities.set", {"uids": ["$uidOf:2"], "attrs": {"il_id": "34"},
                                                     "operation": "attributes"}),
                       bs.step({"layer": LAYER, "direction": "write"},
                               bs.done({"added": 0, "modified": 1, "removed": 0, "source": ANY, "revision": "$current"}),
                               {"revision": "changed"}, command="cad.sources.sync")))
    n += 1
    s, nxt, fs = bound(n)
    attrs = {"ad": "Yeni", "il_id": "6", "tur": "İlçe"}
    out.append(bs.case("yeni nesnenin veritabanının verdiği değerleri yazınca tablodan döner", *s,
                       polygon(487200.0, {"ad": "Yeni", "il_id": "6"}),
                       bs.step({"layer": LAYER, "direction": "write"},
                               bs.done({"added": 1, "modified": 0, "removed": 0, "source": ANY, "revision": "$current"}),
                               {"fields": {str(nxt): {"attrs": {**attrs, "etiket": "ILCE", "olusturma": ANY}}},
                                "revision": "changed"}, command="cad.sources.sync")))
    return out


NOTE = ("ADR 0753. Tablonun sütunları, yabancı anahtarları ve başvurulan tabloların satırları sqlite3 ile PRAGMA "
        "table_info, foreign_key_list ve SELECT'le okunur, KentOS kodu olmadan; satırlar ve bağ bound_sources_cases.py'nin "
        "bağlamasından (GDAL'ın OGR'ı). Veritabanının hesapladığı varsayılan alanın column.made'i; tek sütunlu yabancı "
        "anahtar column.refers ve başvurulan satırlar değer listesi (en çok 10 000, etiket adı tanınan ya da ilk metin "
        "sütunu, aynı etiket koduyla ayrılır, doğal sırada); her okuma yeniden yazar, alanın kendi listesi kalır.")


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.edit", "commandVersion": 1,
            "title": "Bağlı tablonun yabancı anahtarları ve veritabanının verdiği değerler", "note": NOTE,
            "setup": bs.SETUP, "cases": cases()}


def main():
    if "--files" in sys.argv:
        write_files()
        return
    data = build()
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT.name} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text)
    print(f"yazıldı: {OUT.name} ({len(data['cases'])} durum)")


if __name__ == "__main__":
    main()
