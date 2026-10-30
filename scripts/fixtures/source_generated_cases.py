#!/usr/bin/env python3
"""The shared cases of a bound table's columns the table works out itself (docs/adr/0754): SQLite's generated columns,
`GENERATED ALWAYS AS (…) STORED | VIRTUAL`, as a bound layer's fields take them and as edits and writes treat them.

    python3 scripts/fixtures/source_generated_cases.py           # writes fixtures/commands/v1/cad.sources.generated.json
    python3 scripts/fixtures/source_generated_cases.py --check   # compares
    /usr/bin/python3 scripts/fixtures/source_generated_cases.py --files   # GDAL and sqlite3 write uretilen*.gpkg

The table's columns are read here with Python's sqlite3 (PRAGMA table_xinfo and the table's CREATE text in
sqlite_master), not with KentOS code; what a write leaves in the table is worked out by sqlite3 itself on a copy (the
same UPDATE or INSERT run there, the generated columns read back). ADR 0754's rules are written out again below:

- A generated column (table_xinfo's hidden 2: virtual, 3: stored) is a field like any other column, its value read from
  the table. Its `column.made` is its clause as the DDL says it, `GENERATED ALWAYS AS (<expression>) STORED|VIRTUAL`:
  the expression as the table's text writes it inside the parentheses after the definition's own AS (an AS inside a
  CHECK or a DEFAULT's parentheses is not it; the short `AS (…)` form says it in full), every run of white space or
  control characters one space (source_references_cases.py's made_of). It is never required and has no default, NOT
  NULL or not.
- A write never names it: an update leaves it out and reads it again, an insert leaves it out and reads it back, and the
  objects take what the table worked out in the write's step (a NULL as no attribute).
- A kept object's value of it is not changed, by cad.entities.set, cad.fields.write or cad.forms.submit:
  attribute_generated at the attribute's path (cad.fields.write says it among its values' problems); the same value is
  no change. A new object may hold any value; the table's replaces it at the write.
- Reading the table again writes the clause again as the table now says it, and the values.

The objects and the binding come from bound_sources_cases.py's link (GDAL's OGR reads the rows, generated ones too).
"""
import json
import shutil
import sqlite3
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import layer_field_cases as lf  # noqa: E402
import source_defaults_cases as sd  # noqa: E402
import source_references_cases as sr  # noqa: E402
import source_constraints_cases as sc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/cad.sources.generated.json"
U, U2 = "uretilen.gpkg", "uretilen-2.gpkg"
TABLE = "parsel"
LAYER = "import-parsel"
ANY = "$any"

# The parcels: code, block, parcel, made time, note, ring. The second file's first code is another (changed outside).
ROWS = [
    ("101.5", "101", "5", "2026-01-01T10:00:00", None,
     [(487000, 4420000), (487040, 4420000), (487040, 4420030), (487000, 4420030)]),
    ("101.6", "101", "6", "2026-02-01T10:00:00", "köşe",
     [(487050, 4420000), (487090, 4420000), (487090, 4420030), (487050, 4420030)]),
    ("102", "102", None, "2026-03-01T10:00:00", None,
     [(487100, 4420000), (487140, 4420000), (487140, 4420030), (487100, 4420030)]),
]
WRITTEN = ["kod", "ada", "parsel", "olusturma", "notu"]


def rows_of(variant):
    if variant == 2:
        return [("101.5.2", "101", "52", *ROWS[0][3:]), *ROWS[1:]]
    return ROWS


def columns(variant):
    """The parcel table's columns: three stored generated ones (one of the short form, one NOT NULL written over lines),
    two virtual ones, a default the database works out and plain ones. The second file joins the codes with a dash."""
    joined = "'-'" if variant == 2 else "''"
    return f"""
  "kod" TEXT NOT NULL CHECK (CAST(length(kod) AS INTEGER) > 0),
  "ada" TEXT,
  "parsel" TEXT,
  "ana" TEXT GENERATED ALWAYS AS (substr(kod, 1, instr(kod || '.', '.') - 1)) STORED,
  "birlesik" TEXT GENERATED ALWAYS AS (replace(kod, '.', {joined})) VIRTUAL,
  "etiket" TEXT AS (ada || '/' || parsel) STORED,
  "seviye" INTEGER NOT NULL GENERATED ALWAYS AS (CASE
      WHEN kod = '' THEN 0
      ELSE length(kod) - length(replace(kod, '.', '')) + 1
    END) STORED,
  "sabit" INTEGER AS (7),
  "olusturma" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "notu" TEXT
"""


def write_file(name, variant):
    """GDAL makes the GeoPackage and its parcels; sqlite3 makes the table again with its generated columns (GDAL writes
    none), the spatial index left out."""
    from osgeo import ogr, osr
    path = DIR / name
    if path.exists():
        path.unlink()
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
    lyr = ds.CreateLayer(TABLE, srs=srs, geom_type=ogr.wkbPolygon, options=["SPATIAL_INDEX=NO"])
    lyr.CreateField(ogr.FieldDefn("kod", ogr.OFTString))
    for r in rows_of(variant):
        f = ogr.Feature(lyr.GetLayerDefn())
        ring = r[-1]
        f.SetGeometry(ogr.CreateGeometryFromWkt(
            "POLYGON ((" + ", ".join(f"{x} {y}" for x, y in [*ring, ring[0]]) + "))"))
        f.SetField("kod", r[0])
        lyr.CreateFeature(f)
    ds = None
    c = sqlite3.connect(path)
    c.executescript(f'CREATE TABLE "{TABLE}_yeni" ("fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON, '
                    f"{columns(variant)});")
    names = ", ".join(f'"{n}"' for n in WRITTEN)
    for fid, r in enumerate(rows_of(variant), start=1):
        geom = c.execute(f'SELECT geom FROM "{TABLE}" WHERE fid = ?', (fid,)).fetchone()[0]
        c.execute(f'INSERT INTO "{TABLE}_yeni" ("fid", "geom", {names}) VALUES (?, ?, ?, ?, ?, ?, ?)',
                  (fid, geom, *r[:-1]))
    c.executescript(f'DROP TABLE "{TABLE}"; ALTER TABLE "{TABLE}_yeni" RENAME TO "{TABLE}";'
                    f"DELETE FROM gpkg_ogr_contents WHERE table_name = '{TABLE}';")
    c.commit()
    c.execute("VACUUM")
    c.close()
    print(f"yazıldı: {name}")


def write_files():
    for name, variant in ((U, 1), (U2, 2)):
        write_file(name, variant)


# ── ADR 0754's rules, written out again ──

GENERATED = ("ana", "birlesik", "etiket", "seviye", "sabit")


def tokens(sql):
    """SQL text's tokens as (kind, start, end): a string or a quoted name one token, blanks and comments left out."""
    out, i, n = [], 0, len(sql)
    while i < n:
        c = sql[i]
        if c.isspace():
            i += 1
        elif sql.startswith("--", i):
            j = sql.find("\n", i)
            i = n if j < 0 else j
        elif sql.startswith("/*", i):
            j = sql.find("*/", i + 2)
            i = n if j < 0 else j + 2
        elif c in "(),":
            out.append((c, i, i + 1))
            i += 1
        elif c in "'\"`[":
            end = "]" if c == "[" else c
            j = i + 1
            while j < n:
                if sql[j] == end:
                    if c != "[" and sql.startswith(end * 2, j):
                        j += 2
                        continue
                    j += 1
                    break
                j += 1
            out.append(("string" if c == "'" else "name", i, j))
            i = j
        elif c == "_" or c.isalnum():
            j = i
            while j < n and (sql[j] in "_$" or sql[j].isalnum()):
                j += 1
            out.append(("word", i, j))
            i = j
        else:
            out.append(("other", i, i + 1))
            i += 1
    return out


def name_of(sql, token):
    kind, a, b = token
    text = sql[a:b]
    if kind != "name" or len(text) < 2:
        return text
    inner = text[1:-1]
    return inner.replace(text[0] * 2, text[0]) if text[0] in "\"`" else inner


def generation_of(create, column):
    """The expression of `column` in the table's CREATE text: inside the parentheses after its definition's own AS."""
    t = tokens(create)
    start = next((k for k, x in enumerate(t) if x[0] == "("), None)
    if start is None:
        return None
    defs, depth, begin = [], 0, start + 1
    for k in range(start + 1, len(t)):
        kind = t[k][0]
        if kind == "(":
            depth += 1
        elif kind == ")" and depth == 0:
            defs.append(t[begin:k])
            break
        elif kind == ")":
            depth -= 1
        elif kind == "," and depth == 0:
            defs.append(t[begin:k])
            begin = k + 1
    d = next((d for d in defs if d and d[0][0] in ("word", "name")
              and name_of(create, d[0]).lower() == column.lower()), None)
    if d is None:
        return None
    depth = 0
    for k, x in enumerate(d):
        if x[0] == "(":
            depth += 1
        elif x[0] == ")":
            depth = max(depth - 1, 0)
        elif (x[0] == "word" and depth == 0 and create[x[1]:x[2]].upper() == "AS" and k + 1 < len(d)
              and d[k + 1][0] == "("):
            inner = 0
            for y in d[k + 2:]:
                if y[0] == "(":
                    inner += 1
                elif y[0] == ")" and inner == 0:
                    return create[d[k + 1][2]:y[1]].strip() or None
                elif y[0] == ")":
                    inner -= 1
            return None
    return None


def clause(expression, stored):
    if expression is None:
        return sr.GENERATED
    return f"{sr.GENERATED} AS ({expression}) {'STORED' if stored else 'VIRTUAL'}"


def fields(name):
    """The bound layer's fields from the table's own definition (table_xinfo: the generated columns too)."""
    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    create = c.execute("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?", (TABLE,)).fetchone()[0]
    out = []
    for _, cname, decl, notnull, default, pk, hidden in c.execute(f'PRAGMA table_xinfo("{TABLE}")'):
        if pk or cname == "geom" or hidden == 1:
            continue
        if hidden in (2, 3):
            default = clause(generation_of(create, cname), hidden == 3)
        kind = bs.kind_of(decl)
        f = {"name": cname, "kind": kind}
        if notnull and default is None:
            f["required"] = True
        if sr.making(default) == "plain":
            lit = sd.literal(default)
            value = sd.as_kind(kind, lit) if lit is not None else None
            if value is not None:
                f["default"] = value
        made = sr.made_of(default)
        if made is not None:
            f["column"] = {"made": made}
        assert lf.field_problem(f) is None, (f, lf.field_problem(f))
        out.append(f)
    c.close()
    return out


def made_text(field):
    """What a refusal says the table works the value out by: the expression in the field's clause."""
    m = field["column"]["made"]
    rest = m[len(sr.GENERATED):].strip()
    assert rest.startswith("AS (") and rest.endswith((" STORED", " VIRTUAL")), m
    return rest[len("AS ("):rest.rindex(" ")][:-1].strip()


def refusal(field):
    return (f"“{field['name']}” alanının değeri değiştirilemez: tablo onu hesaplıyor: {made_text(field)}. Değer, kayıt "
            "tabloya yazılınca tablodan gelir.")


def worked_out(sql, args):
    """The generated values sqlite3 works out when `sql` runs on a copy of the file: by fid, each column's value."""
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / U
        shutil.copy(DIR / U, path)
        c = sqlite3.connect(path)
        c.execute(sql, args)
        c.commit()
        names = ", ".join(f'"{n}"' for n in GENERATED)
        rows = {r[0]: dict(zip(GENERATED, r[1:])) for r in c.execute(f'SELECT fid, {names} FROM "{TABLE}"')}
        c.close()
    return rows


def with_generated(attrs, values):
    """An object's attributes with the table's generated values: each as its text, a NULL none."""
    out = {k: v for k, v in attrs.items() if k not in GENERATED}
    for k, v in values.items():
        if v is not None:
            out[k] = str(v)
    return {k: out[k] for k in sorted(out, key=ORDER.index)}


ORDER = ["kod", "ada", "parsel", *GENERATED, "olusturma", "notu"]


# ── The cases ──

def path_of(n):
    return f"$tmp:uretilen-{n}.gpkg"


def bound(n):
    """The file copied and its table bound onto a new layer: the steps, the objects, the layer's fields."""
    answer, warnings, objs, ids, b = bs.linked(TABLE, name=U)
    path = path_of(n)
    b = {**b, "path": path}
    # The table's CHECK and unique constraints come with it (docs/adr/0755), read by that item's reference.
    for key, found in (("checks", sc.checks_of(U)), ("uniques", sc.uniques_of(U))):
        if found:
            b[key] = found
    answer = {**answer, "link": b}
    layer_fields = fields(U)
    copy = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{U}", "to": path}
    link = bs.step({"edit": {"kind": "link", "path": path, "table": TABLE}}, bs.done(answer, warnings),
                   {"ids": [1, *ids], "fields": objs, "revision": "changed", "canUndo": True,
                    "settings": {"sources": [b]}, "nodes": {LAYER: {"fields": layer_fields}}},
                   command="cad.sources.edit")
    return [copy, link], objs, layer_fields


def by_name(layer_fields, name):
    return next(f for f in layer_fields if f["name"] == name)


def set_attrs(slot, attrs):
    return {"uids": [f"$uidOf:{slot}"], "attrs": attrs, "operation": "attributes"}


def written(n_added=0, n_modified=0):
    return bs.done({"added": n_added, "modified": n_modified, "removed": 0, "source": ANY, "revision": "$current"})


WRITE = {"layer": LAYER, "direction": "write"}
READ = {"layer": LAYER, "direction": "read"}
NOTHING_LEFT = bs.step(WRITE, written(), op="plan", command="cad.sources.sync")


def polygon(x, attrs):
    ring = [{"x": x, "y": 4420040.0}, {"x": x + 20.0, "y": 4420040.0}, {"x": x + 20.0, "y": 4420060.0},
            {"x": x, "y": 4420060.0}]
    return bs.other("cad.polygon.create", {"layerId": LAYER, "pts": ring, "attrs": attrs})


def cases():
    out = []
    n = 0

    def fresh():
        nonlocal n
        n += 1
        return bound(n)

    s, objs, fs = fresh()
    assert gi_slug() == LAYER
    ana, birlesik, etiket, seviye, sabit = (by_name(fs, k) for k in GENERATED)
    assert ana["column"]["made"].endswith(" STORED") and birlesik["column"]["made"].endswith(" VIRTUAL")
    assert etiket["column"]["made"] == "GENERATED ALWAYS AS (ada || '/' || parsel) STORED"
    assert "required" not in seviye and "\n" not in seviye["column"]["made"] and "  " not in seviye["column"]["made"]
    assert sabit["column"]["made"] == "GENERATED ALWAYS AS (7) VIRTUAL"
    assert by_name(fs, "olusturma")["column"]["made"] == "CURRENT_TIMESTAMP" and by_name(fs, "kod")["required"]
    # Binding: one case a rule (the whole field list and the objects said in each).
    out.append(bs.case("bağlanınca hesaplanan sütun alan olur: değeri tablodan, ifadesi column.made'de", *s))
    for name in ("VIRTUAL sütunun değeri de tablodan okunur, ifadesi VIRTUAL diye yazılır",
                 "kısa “AS (…)” biçimi tam yazılır; türü yazılmayan VIRTUAL'dır",
                 "NOT NULL hesaplanan sütun zorunlu alan olmaz; satırlara bölünmüş ifadesi tek satır yazılır",
                 "veritabanının hesapladığı varsayılanlı düz sütun düz kalır"):
        s, _, _ = fresh()
        out.append(bs.case(name, *s))
    s, _, _ = fresh()
    plan = dict(s[1])
    plan["op"] = "plan"
    plan["expect"] = {"ids": [1], "revision": "same", "canUndo": False, "dirty": False}
    out.append(bs.case("plan bağlamayı yazmaz", s[0], plan))
    # Reading.
    s, _, fs = fresh()
    out.append(bs.case("hemen okumak bir şey değiştirmez", *s,
                       bs.step(READ, bs.done(bs.synced(name=U)), bs.SAME, command="cad.sources.sync")))
    s, _, fs = fresh()
    after = json.loads(json.dumps(fs))
    by_name(after, "birlesik")["column"] = by_name(fields(U2), "birlesik")["column"]
    assert after == fields(U2) and after != fs
    objs2 = bs.gi.imported(U2, [TABLE], start=2)[2]
    changed = {k: v for k, v in objs2.items() if v != objs[k]}
    assert sorted(changed) == ["2", "3"], changed
    over = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{U2}", "to": path_of(n)}
    out.append(bs.case("dosyada kod değişince okumak hesaplanan değerleri getirir; ifadesi değişen sütunun "
                       "column.made'i yenilenir", *s, over,
                       bs.step(READ, bs.done(bs.synced(modified=2, name=U2)),
                               {"ids": [1, 2, 3, 4], "fields": changed, "revision": "changed", "canUndo": True,
                                "nodes": {LAYER: {"fields": after}}}, command="cad.sources.sync")))
    s, _, _ = fresh()
    over = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{U2}", "to": path_of(n)}
    out.append(bs.case("okumanın planı ne alanları ne nesneleri yazar", *s, over,
                       bs.step(READ, bs.done(bs.synced(modified=2, name=U2)), op="plan", command="cad.sources.sync")))
    # A kept object's generated values are not changed.
    for name, attrs, field in (
            ("kayıtlı nesnenin STORED hesaplanan değeri değiştirilemez", {"ana": "999"}, ana),
            ("VIRTUAL hesaplanan değer de değiştirilemez", {"birlesik": "x"}, birlesik),
            ("hesaplanan değeri kaldırmak da değiştirmektir", {"etiket": None}, etiket),
            ("ilk değişen hesaplanan alan söylenir, düz alanın değişikliği de yazılmaz",
             {"notu": "yeni", "seviye": "5", "ana": "1"}, ana)):
        s, _, _ = fresh()
        out.append(bs.case(name, *s, bs.step(set_attrs(2, attrs),
                                             bs.failed("attribute_generated", refusal(field), f"attrs.{field['name']}"),
                                             command="cad.entities.set")))
    s, objs, _ = fresh()
    out.append(bs.case("aynı değeri yazmak değişiklik değildir; düz alan yazılır", *s,
                       bs.other("cad.entities.set", set_attrs(2, {"ana": "101", "notu": "yeni not"}),
                                {"fields": {"2": {"attrs": {**objs["2"]["attrs"], "notu": "yeni not"}}},
                                 "revision": "changed"})))
    # cad.fields.write says them among its values' problems.
    calc = {"layer": LAYER, "write": {"kind": "calculate", "field": "ana", "expression": "kod"}}
    problems = [{"uid": f"$uidOf:{k}", "field": "ana", "code": "attribute_generated", "message": refusal(ana)}
                for k in (2, 3)]
    s, _, _ = fresh()
    out.append(bs.case("Alan hesapla hesaplanan alana yazamaz: değişen her kayıt sorunlu, hiçbiri yazılmaz", *s,
                       bs.step(calc, bs.failed("invalid_values",
                                               "2 nesnenin değeri alanların kurallarına uymuyor; hiçbir değer "
                                               f"yazılmadı. İlki: {refusal(ana)} (ana). Sorunların hepsi plan'da; "
                                               "uyanları yazmak için policy valid verin.", "write"),
                               command="cad.fields.write")))
    s, _, _ = fresh()
    out.append(bs.case("Alan hesapla'nın planı sorunları kayıt kayıt söyler; değişmeyen kayıt sorun değildir", *s,
                       bs.step(calc, bs.done({"changed": 0, "unchanged": 1, "kept": 0, "invalid": 2,
                                              "problems": problems, "revision": "$current"}),
                               op="plan", command="cad.fields.write")))
    s, _, _ = fresh()
    out.append(bs.case("Yapıştır'da hesaplanan alanın aynı değeri değişiklik değildir", *s,
                       bs.step({"layer": LAYER, "write": {"kind": "values",
                                                          "rows": [{"uid": "$uidOf:2", "values": {"ana": "101"}}]}},
                               bs.done({"changed": 0, "unchanged": 1, "kept": 0, "invalid": 0,
                                        "revision": "$current"}), command="cad.fields.write")))
    s, objs, _ = fresh()
    skipped = refusal(birlesik)
    out.append(bs.case("Yapıştır policy valid: düz alan yazılır, hesaplanan alan atlanır", *s,
                       bs.step({"layer": LAYER, "policy": "valid",
                                "write": {"kind": "values", "rows": [{"uid": "$uidOf:2", "values": {"notu": "yeni"}},
                                                                     {"uid": "$uidOf:3", "values": {"birlesik": "x"}}]}},
                               {"status": "completed",
                                "output": {"changed": 1, "unchanged": 0, "kept": 0, "invalid": 1,
                                           "changes": [{"uid": "$uidOf:2", "field": "notu", "after": "yeni"}],
                                           "problems": [{"uid": "$uidOf:3", "field": "birlesik",
                                                         "code": "attribute_generated", "message": skipped}],
                                           "revision": "$current"},
                                "warnings": [{"code": "values_skipped", "path": "write",
                                              "message": "1 nesnenin değeri kurallara uymadığı için yazılmadı. İlki: "
                                                         f"{skipped} (birlesik)."}]},
                               {"fields": {"2": {"attrs": {**objs["2"]["attrs"], "notu": "yeni"}}},
                                "revision": "changed"}, command="cad.fields.write")))
    # cad.forms.submit refuses it too (the draft of a form made from the layer's fields).
    form = bs.step({"edit": {"kind": "create", "name": "Parsel", "layer": LAYER}},
                   {"status": "completed", "output": ANY, "warnings": ANY}, {"ids": [1, 2, 3, 4]},
                   command="cad.forms.edit")
    s, _, _ = fresh()
    out.append(bs.case("Formla kaydet hesaplanan alanın değişikliğini reddeder", *s, form,
                       bs.step({"form": "parsel", "uid": "$uidOf:2", "values": {"ana": "999"}, "draft": True},
                               bs.failed("attribute_generated", refusal(ana), "values.ana"),
                               command="cad.forms.submit")))
    s, objs, _ = fresh()
    kept = {**objs["2"]["attrs"], "notu": "form notu"}
    out.append(bs.case("Formla kaydette hesaplanan alanın aynı değeri reddedilmez, düz alan yazılır", *s, form,
                       bs.step({"form": "parsel", "uid": "$uidOf:2", "values": {"ana": "101", "notu": "form notu"},
                                "draft": True},
                               bs.done({"uid": "$uidOf:2", "attrs": kept, "changed": True, "revision": "$current",
                                        "changes": [{"uid": "$uidOf:2", "layer": LAYER, "fields": ["notu"]}],
                                        "undo": "Form: Parsel", "snapshot": ANY}),
                               {"fields": {"2": {"attrs": kept}}, "revision": "changed"},
                               command="cad.forms.submit")))
    # Writing: the table works them out, the objects take them in the write's step.
    s, objs, _ = fresh()
    table = worked_out(f'UPDATE "{TABLE}" SET kod = ? WHERE fid = 1', ("103.7.1",))
    edited = {**objs["2"]["attrs"], "kod": "103.7.1"}
    took = with_generated(edited, table[1])
    assert took["ana"] == "103" and took["seviye"] == "3" and took["birlesik"] == "10371"
    write = bs.step(WRITE, written(n_modified=1), {"fields": {"2": {"attrs": took}}, "revision": "changed"},
                    command="cad.sources.sync")
    set_kod = bs.other("cad.entities.set", set_attrs(2, {"kod": "103.7.1"}))
    out.append(bs.case("kod değişince yazmak hesaplanan sütunları tablodan geri okur", *s, set_kod, write))
    s, _, _ = fresh()
    out.append(bs.case("yazdıktan sonra yazılacak bir şey kalmaz", *s, set_kod, write, NOTHING_LEFT))
    s, _, _ = fresh()
    out.append(bs.case("yazmayı geri almak tablodan gelen değerleri de geri alır", *s, set_kod, write,
                       {"op": "undo", "expect": {"fields": {"2": {"attrs": edited}}}}))
    s, objs, _ = fresh()
    out.append(bs.case("yazmanın planı tabloya yazmaz, hesaplananları getirmez", *s, set_kod,
                       bs.step(WRITE, written(n_modified=1), op="plan", command="cad.sources.sync")))
    s, objs, _ = fresh()
    table = worked_out(f'UPDATE "{TABLE}" SET parsel = NULL WHERE fid = 1', ())
    assert table[1]["etiket"] is None
    gone = with_generated({k: v for k, v in objs["2"]["attrs"].items() if k != "parsel"}, table[1])
    assert "etiket" not in gone
    out.append(bs.case("NULL'a dönen hesaplanan değer yazılınca nesneden kalkar", *s,
                       bs.other("cad.entities.set", set_attrs(2, {"parsel": None})),
                       bs.step(WRITE, written(n_modified=1), {"fields": {"2": {"attrs": gone}}, "revision": "changed"},
                               command="cad.sources.sync")))
    # A new object may hold one: the insert leaves it out (sqlite3 refuses a generated column's value), the table's
    # comes back.
    try:
        worked_out(f'INSERT INTO "{TABLE}" (kod, ana) VALUES (?, ?)', ("105.1", "999"))
        raise AssertionError("sqlite3 took a generated column's value")
    except sqlite3.OperationalError:
        pass
    s, _, _ = fresh()
    given = {"kod": "105.1", "ada": "105", "parsel": "1", "ana": "999"}
    table = worked_out(f'INSERT INTO "{TABLE}" (kod, ada, parsel) VALUES (?, ?, ?)', ("105.1", "105", "1"))
    new = with_generated({k: v for k, v in given.items() if k not in GENERATED}, table[4])
    assert new["ana"] == "105" and new["etiket"] == "105/1"
    out.append(bs.case("yeni nesneye verilen hesaplanan değeri tablo yazarken değiştirir", *s,
                       polygon(487200.0, given),
                       bs.step(WRITE, written(n_added=1), {"fields": {"5": {"attrs": {**new, "olusturma": ANY}}},
                                                           "revision": "changed"}, command="cad.sources.sync"),
                       NOTHING_LEFT))
    return out


def gi_slug():
    return bs.gi.slug(TABLE)


NOTE = ("ADR 0754. Tablonun sütunları sqlite3 ile PRAGMA table_xinfo'yla (hesaplananlar dahil) ve sqlite_master'daki "
        "CREATE metninden okunur, KentOS kodu olmadan; yazmadan sonra tablonun hesapladıkları sqlite3'ün kendisine "
        "dosyanın kopyasında aynı UPDATE ya da INSERT çalıştırılarak hesaplatılır. Satırlar ve bağ "
        "bound_sources_cases.py'nin bağlamasından (GDAL'ın OGR'ı hesaplanan sütunları da okur). Hesaplanan sütun alan "
        "olur, column.made'i tablonun yazdığı ifade (GENERATED ALWAYS AS (…) STORED | VIRTUAL), zorunlu değil, "
        "varsayılanı yok; yazmada adı geçmez, değeri tablodan geri okunur; çizimde var olan kaydın değeri "
        "değiştirilemez (attribute_generated).")


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.edit", "commandVersion": 1,
            "title": "Bağlı tablonun hesaplanan sütunları", "note": NOTE, "setup": bs.SETUP, "cases": cases()}


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
