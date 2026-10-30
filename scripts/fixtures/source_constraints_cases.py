#!/usr/bin/env python3
"""The shared cases of a bound table's CHECK and unique constraints (docs/adr/0755): what binding and reading a
GeoPackage table bring into its link and its fields, what KentOS refuses before the table, and what only the table
refuses, said by its constraint.

    python3 scripts/fixtures/source_constraints_cases.py           # writes fixtures/commands/v1/cad.sources.constraints.json
    python3 scripts/fixtures/source_constraints_cases.py --check   # compares
    /usr/bin/python3 scripts/fixtures/source_constraints_cases.py --files   # GDAL and sqlite3 write kisitli*.gpkg

The table's constraints are read here with Python's sqlite3, not with KentOS code: the CHECKs from the table's CREATE
text (a column's own and the table's, with their CONSTRAINT names), the unique ones from PRAGMA index_list and
index_info (a constraint's and a plain unique index's, a partial one or one over an expression left out, in the order
they were made). What a write does is SQLite's own answer: the same values written into a copy of the file. ADR 0755's
rules are written out again below:

- The link keeps each CHECK's name (none without one), its text as written and the table's columns it reads (in the
  table's order); a CHECK reading a column with a collation other than BINARY is the table's only (`tableOnly`).
- A one-column CHECK tying its column to a list (`x IN (…)`) gives the field that list, codes and labels the values in
  their natural order, and the field's column names the constraint (its name, else its text).
- KentOS refuses a write that breaks a CHECK it judges (`rule_broken`, the constraint's sentence) and one whose values
  of a unique constraint another record holds (`not_unique`); a CHECK with a function it has not and the table's own
  ones are asked by the table when the layer is written: `source_check`, `source_unique`, and nothing is written.
- Reading the table again writes the link's constraints again: one gone from the table goes from the link, with the
  list it gave.
"""
import re
import shutil
import sqlite3
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402
import layer_field_cases as lf  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/cad.sources.constraints.json"
K, K2 = "kisitli.gpkg", "kisitli-2.gpkg"
TABLE = "parsel"
LAYER = "import-parsel"
ANY = "$any"

# The parcels: block, parcel, code, use, area, floors, number, note, name, ring.
ROWS = [
    ("101", "5", "K1", "konut", 120.5, 2, None, None, "a",
     [(487000, 4420000), (487040, 4420000), (487040, 4420030), (487000, 4420030)]),
    ("101", "6", "K2", "ticaret", 80.0, 4, 7, "köşe", "b",
     [(487050, 4420000), (487090, 4420000), (487090, 4420030), (487050, 4420030)]),
    ("102", "1", None, None, None, None, None, None, None,
     [(487100, 4420000), (487140, 4420000), (487140, 4420030), (487100, 4420030)]),
]
WRITTEN = ["ada", "parsel", "kod", "tur", "alan", "kat", "no", "notu", "ad"]


def columns(variant):
    """The parcel table's columns and constraints: a column's list, a named range, a function KentOS has not, a collated
    column's list, a table CHECK and a unique pair; the second file drops the use's list and widens the range."""
    use = "" if variant == 2 else " CHECK (tur IN ('konut', 'ticaret', 'tarla'))"
    top = 60 if variant == 2 else 40
    return f"""
  "ada" TEXT,
  "parsel" TEXT,
  "kod" TEXT,
  "tur" TEXT{use},
  "alan" DOUBLE,
  "kat" INTEGER CONSTRAINT kat_aralik CHECK (kat BETWEEN 0 AND {top}),
  "no" INTEGER,
  "notu" TEXT CHECK (notu IS NULL OR hex(notu) <> '41'),
  "ad" TEXT COLLATE NOCASE CHECK (ad IN ('a', 'b')),
  CONSTRAINT alan_pozitif CHECK (alan > 0),
  CONSTRAINT ada_parsel UNIQUE (ada, parsel)
"""


def write_file(name, variant):
    """GDAL makes the GeoPackage and its parcels; sqlite3 makes the table again with its constraints (GDAL writes none)
    and its unique indexes, the spatial index left out."""
    from osgeo import ogr, osr
    path = DIR / name
    if path.exists():
        path.unlink()
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(5256)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
    lyr = ds.CreateLayer(TABLE, srs=srs, geom_type=ogr.wkbPolygon, options=["SPATIAL_INDEX=NO"])
    lyr.CreateField(ogr.FieldDefn("ada", ogr.OFTString))
    for r in ROWS:
        f = ogr.Feature(lyr.GetLayerDefn())
        ring = r[-1]
        f.SetGeometry(ogr.CreateGeometryFromWkt(
            "POLYGON ((" + ", ".join(f"{x} {y}" for x, y in [*ring, ring[0]]) + "))"))
        f.SetField("ada", r[0])
        lyr.CreateFeature(f)
    ds = None
    c = sqlite3.connect(path)
    c.executescript(f'CREATE TABLE "{TABLE}_yeni" ("fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, "geom" POLYGON, '
                    f"{columns(variant)});")
    names = ", ".join(f'"{n}"' for n in WRITTEN)
    for fid, r in enumerate(ROWS, start=1):
        geom = c.execute(f'SELECT geom FROM "{TABLE}" WHERE fid = ?', (fid,)).fetchone()[0]
        c.execute(f'INSERT INTO "{TABLE}_yeni" ("fid", "geom", {names}) VALUES (?, ?, {", ".join("?" * len(WRITTEN))})',
                  (fid, geom, *r[:-1]))
    c.executescript(f'DROP TABLE "{TABLE}"; ALTER TABLE "{TABLE}_yeni" RENAME TO "{TABLE}";'
                    f"CREATE UNIQUE INDEX parsel_kod ON {TABLE} (kod);"
                    f"CREATE UNIQUE INDEX parsel_no ON {TABLE} (no) WHERE no > 0;"
                    f"CREATE UNIQUE INDEX parsel_ad_buyuk ON {TABLE} (upper(ad));"
                    f"DELETE FROM gpkg_ogr_contents WHERE table_name = '{TABLE}';")
    c.commit()
    c.execute("VACUUM")
    c.close()
    print(f"yazıldı: {name}")


def write_files():
    for name, variant in ((K, 1), (K2, 2)):
        write_file(name, variant)


# ── ADR 0755's rules, written out again ──

def definitions(create):
    """The CREATE text's definitions: the texts between the commas of its outer parentheses (strings kept whole)."""
    body = create[create.index("(") + 1:]
    out, depth, cur, i = [], 0, "", 0
    while i < len(body):
        ch = body[i]
        if ch == "'":
            j = i + 1
            while j < len(body) and not (body[j] == "'" and (j + 1 >= len(body) or body[j + 1] != "'")):
                j += 2 if body[j] == "'" else 1
            cur += body[i:j + 1]
            i = j + 1
            continue
        if ch == "(":
            depth += 1
        elif ch == ")":
            if depth == 0:
                out.append(cur.strip())
                return out
            depth -= 1
        elif ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
            i += 1
            continue
        cur += ch
        i += 1
    return out


def inside(text, at):
    """The text inside the parentheses opening at `at`, and the place after them."""
    depth, i = 0, at
    while i < len(text):
        if text[i] == "'":
            i = text.index("'", i + 1) + 1
            continue
        if text[i] == "(":
            depth += 1
        elif text[i] == ")":
            depth -= 1
            if depth == 0:
                return text[at + 1:i].strip(), i + 1
        i += 1
    raise ValueError(text)


def names_in(sql):
    """The words of a constraint's text outside its strings, lowercased."""
    plain = re.sub(r"'(?:[^']|'')*'", "''", sql)
    return {w.lower() for w in re.findall(r"[A-Za-z_][A-Za-z0-9_$]*", plain)}


def table_columns(c):
    return [r[1] for r in c.execute(f'PRAGMA table_info("{TABLE}")')]


def affinity(decl):
    """SQLite's affinity of a declared type, by the rules of “Datatypes In SQLite” §3.1, in their order."""
    up = decl.upper()
    if "INT" in up:
        return "INTEGER"
    if "CHAR" in up or "CLOB" in up or "TEXT" in up:
        return "TEXT"
    if "BLOB" in up or not up.strip():
        return "BLOB"
    if "REAL" in up or "FLOA" in up or "DOUB" in up:
        return "REAL"
    return "NUMERIC"


# The affinity KentOS judges a field of each kind with (ADR 0755 §1): a column declared otherwise holds what KentOS
# writes as another storage class, so a constraint reading it is the table's only.
NATURAL = {"text": "TEXT", "uuid": "TEXT", "json": "TEXT", "integer": "INTEGER", "bigint": "INTEGER",
           "decimal": "REAL", "binary": "BLOB", "boolean": "NUMERIC", "date": "NUMERIC", "datetime": "NUMERIC",
           "time": "NUMERIC", "instant": "NUMERIC"}


def checks_of(name):
    """The table's CHECKs in the order its text writes them: name, text, the columns read (table order), table only."""
    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    create = c.execute("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?", (TABLE,)).fetchone()[0]
    cols = table_columns(c)
    geometry = {r[0] for r in c.execute("SELECT column_name FROM gpkg_geometry_columns WHERE table_name = ?", (TABLE,))}
    # A field's column: neither the key nor the geometry; its kind as the layer's field has it.
    unfit = {cname for _, cname, decl, _, _, pk, _ in c.execute(f'PRAGMA table_xinfo("{TABLE}")')
             if not pk and cname not in geometry and bs.kind_of(decl)
             and affinity(decl) != NATURAL[bs.kind_of(decl)]}
    c.close()
    collated = set()
    out = []
    for d in definitions(create):
        words = d.split()
        first = words[0].strip('"').lower()
        column = None if first in ("constraint", "primary", "unique", "check", "foreign") else words[0].strip('"')
        if column and re.search(r"\bCOLLATE\s+(?!BINARY\b)\w+", d, re.I):
            collated.add(column)
        for m in re.finditer(r"(?:CONSTRAINT\s+(\w+)\s+)?CHECK\s*\(", d, re.I):
            sql, _ = inside(d, m.end() - 1)
            read = names_in(sql)
            used = [n for n in cols if n.lower() in read]
            check = {"name": m.group(1)} if m.group(1) else {}
            check.update({"sql": sql, "columns": used})
            if any(n in collated or n in unfit for n in used):
                check["tableOnly"] = True
            out.append(check)
    return out


def uniques_of(name):
    """The unique constraints and plain unique indexes in the order they were made (index_list lists the newest first);
    a partial one and one over an expression left out; an index's name, none for a constraint's."""
    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    out = []
    for _, iname, unique, origin, partial in reversed(c.execute(f'PRAGMA index_list("{TABLE}")').fetchall()):
        if not unique or partial or origin not in ("u", "c"):
            continue
        info = c.execute(f'PRAGMA index_info("{iname}")').fetchall()
        if not info or any(cid < 0 or n is None for _, cid, n in info):
            continue
        u = {"name": iname} if origin == "c" else {}
        u["columns"] = [n for _, _, n in info]
        out.append(u)
    c.close()
    return out


def natural_key(s):
    return [(0, int(p)) if p.isdigit() else (1, p) for p in re.findall(r"\d+|\D+", s)]


def fields(name):
    """The bound layer's fields: the table's columns, a one-column list CHECK's values as a text field's list."""
    c = sqlite3.connect(f"file:{DIR / name}?mode=ro", uri=True)
    info = c.execute(f'PRAGMA table_info("{TABLE}")').fetchall()
    c.close()
    checks = checks_of(name)
    out = []
    for _, cname, decl, notnull, default, pk in info:
        if pk or cname == "geom":
            continue
        f = {"name": cname, "kind": bs.kind_of(decl)}
        if notnull:
            f["required"] = True
        for ch in checks:
            m = re.fullmatch(rf"{re.escape(cname)}\s+IN\s*\((.*)\)", ch["sql"], re.I)
            if ch.get("tableOnly") or ch["columns"] != [cname] or not m:
                continue
            values = re.findall(r"'((?:[^']|'')*)'", m.group(1))
            codes = sorted(dict.fromkeys(v.replace("''", "'") for v in values), key=natural_key)
            f["values"] = [{"code": v, "label": v} for v in codes]
            f["column"] = {"check": ch.get("name", ch["sql"])}
        assert lf.field_problem(f) is None, (f, lf.field_problem(f))
        out.append(f)
    return out


def goes_in(row_fid, values):
    """Whether SQLite takes `values` written over parcel `row_fid` of a copy of the file, and its refusal's words."""
    with tempfile.TemporaryDirectory() as d:
        path = Path(d) / K
        shutil.copy(DIR / K, path)
        c = sqlite3.connect(path)
        sets = ", ".join(f'"{k}" = ?' for k in values)
        try:
            c.execute(f'UPDATE "{TABLE}" SET {sets} WHERE fid = ?', (*values.values(), row_fid))
            c.commit()
            return True, None
        except sqlite3.IntegrityError as e:
            return False, str(e)
        finally:
            c.close()


# ── The cases ──

def path_of(n):
    return f"$tmp:kisitli-{n}.gpkg"


def binding(name, path):
    b = {**bs.link(TABLE, LAYER, name=name), "path": path, "checks": checks_of(name), "uniques": uniques_of(name)}
    return b


def bound(n):
    """The file copied and its table bound onto a new layer: the steps, the objects, the layer's fields, the link."""
    answer, warnings, objs, ids, _ = bs.linked(TABLE, name=K)
    path = path_of(n)
    b = binding(K, path)
    answer = {**answer, "link": b}
    layer_fields = fields(K)
    copy = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{K}", "to": path}
    link = bs.step({"edit": {"kind": "link", "path": path, "table": TABLE}}, bs.done(answer, warnings),
                   {"ids": [1, *ids], "fields": objs, "revision": "changed", "canUndo": True,
                    "settings": {"sources": [b]}, "nodes": {LAYER: {"fields": layer_fields}}},
                   command="cad.sources.edit")
    return [copy, link], objs, layer_fields, b


def set_attrs(slot, attrs):
    return {"uids": [f"$uidOf:{slot}"], "attrs": attrs, "operation": "attributes"}


def set_step(slot, attrs, result=None):
    if result is None:
        return bs.other("cad.entities.set", set_attrs(slot, attrs))
    return bs.step(set_attrs(slot, attrs), result, command="cad.entities.set")


def broken(field, message):
    return bs.failed("rule_broken", message, f"attrs.{field}")


def written(n_modified=1):
    return bs.done({"added": 0, "modified": n_modified, "removed": 0, "source": ANY, "revision": "$current"})


WRITE = {"layer": LAYER, "direction": "write"}
READ = {"layer": LAYER, "direction": "read"}


def polygon(x, attrs, result=None):
    ring = [{"x": x, "y": 4420040.0}, {"x": x + 20.0, "y": 4420040.0}, {"x": x + 20.0, "y": 4420060.0},
            {"x": x, "y": 4420060.0}]
    inp = {"layerId": LAYER, "pts": ring, "attrs": attrs}
    if result is None:
        return bs.other("cad.polygon.create", inp)
    return bs.step(inp, result, command="cad.polygon.create")


def cases():
    out = []
    n = 0

    def fresh():
        nonlocal n
        n += 1
        return bound(n)

    s, objs, fs, b = fresh()
    slot = {objs[k]["attrs"].get("parsel") + "/" + objs[k]["attrs"]["ada"]: int(k) for k in objs}
    p5, p6, p1 = slot["5/101"], slot["6/101"], slot["1/102"]
    checks = b["checks"]
    assert [c.get("name") for c in checks] == [None, "kat_aralik", None, None, "alan_pozitif"], checks
    assert checks[3].get("tableOnly") and b["uniques"] == [{"columns": ["ada", "parsel"]},
                                                          {"name": "parsel_kod", "columns": ["kod"]}]
    tur = next(f for f in fs if f["name"] == "tur")
    assert [v["code"] for v in tur["values"]] == ["konut", "tarla", "ticaret"]
    # Binding.
    out.append(bs.case("bağlanınca tablonun CHECK ve tekil kısıtları bağa gelir; listeli CHECK alanın listesi olur", *s))
    s, _, _, _ = fresh()
    plan = dict(s[1])
    plan["op"] = "plan"
    plan["expect"] = {"ids": [1], "revision": "same", "canUndo": False, "dirty": False}
    out.append(bs.case("plan bağlamayı yazmaz", s[0], plan))
    s, _, _, _ = fresh()
    out.append(bs.case("hemen okumak bir şey değiştirmez", *s,
                       bs.step(READ, bs.done(bs.synced(name=K)), bs.SAME, command="cad.sources.sync")))
    # KentOS asks the CHECKs it judges, and the unique constraints, before the table.
    for name, attrs, field, message in (
            ("listede olmayan tür alanın listesince reddedilir", {"tur": "fabrika"}, "tur", None),
            ("sıfır alan alan_pozitif kısıtını bozar", {"alan": "0"}, "alan",
             "Tablonun “alan_pozitif” kısıtı: alan 0 değerinden büyük olmalı."),
            ("eksi alan da bozar", {"alan": "-5"}, "alan",
             "Tablonun “alan_pozitif” kısıtı: alan 0 değerinden büyük olmalı."),
            ("aralığın dışındaki kat kat_aralik kısıtını bozar", {"kat": "41"}, "kat",
             "Tablonun “kat_aralik” kısıtı: kat 0 ile 40 arasında olmalı.")):
        ok, why = goes_in(1, {k: v for k, v in attrs.items()})
        assert not ok and "CHECK constraint failed" in why, (name, why)
        s, _, _, _ = fresh()
        result = (bs.failed("invalid_attribute", f"“{field}” alanı listedeki değerlerden birini ister; "
                                                 f"“{attrs[field]}” listede yok.", f"attrs.{field}")
                  if message is None else broken(field, message))
        out.append(bs.case(name, *s, set_step(p5, attrs, result)))
    for name, attrs in (("pozitif alan geçer", {"alan": "12.5"}),
                        ("alanı boşaltmak geçer: boş değer kısıtı tutar", {"alan": None}),
                        ("aralığın ucundaki kat geçer", {"kat": "40"}),
                        ("listedeki tür geçer", {"tur": "tarla"})):
        ok, why = goes_in(1, attrs)
        assert ok, (name, why)
        s, _, _, _ = fresh()
        out.append(bs.case(name, *s, set_step(p5, attrs), bs.step(WRITE, written(), op="plan",
                                                                  command="cad.sources.sync")))
    ok, why = goes_in(2, {"parsel": "5"})
    assert not ok and "UNIQUE constraint failed: parsel.ada, parsel.parsel" in why, why
    s, _, _, _ = fresh()
    out.append(bs.case("ada ve parsel birlikte tekil: aynı çift başka kayıtta", *s, set_step(p6, {"parsel": "5"}, bs.failed(
        "not_unique", "“ada”, “parsel” alanları tabloda birlikte tekil; (“101”, “5”) başka bir kayıtta var. Başka "
                      "değerler verin.", "attrs.ada"))))
    ok, _ = goes_in(2, {"parsel": "9"})
    assert ok
    s, _, _, _ = fresh()
    out.append(bs.case("aynı ada başka parsel geçer", *s, set_step(p6, {"parsel": "9"})))
    ok, why = goes_in(2, {"kod": "K1"})
    assert not ok and "UNIQUE constraint failed: parsel.kod" in why, why
    s, _, _, _ = fresh()
    out.append(bs.case("kod tekil dizinle tekil", *s, set_step(p6, {"kod": "K1"}, bs.failed(
        "not_unique", "“kod” alanı tabloda tekil; “K1” başka bir kayıtta var. Başka bir değer verin.", "attrs.kod"))))
    ok, _ = goes_in(1, {"kod": None})
    assert ok
    s, _, _, _ = fresh()
    out.append(bs.case("boş kod tekilliğe girmez: boş değerler birbirinden farklı", *s, set_step(p5, {"kod": None})))
    s, _, _, _ = fresh()
    out.append(bs.case("yeni kayıt her kısıtı sorar", *s, polygon(487200.0, {"alan": "-1"}, broken(
        "alan", "Tablonun “alan_pozitif” kısıtı: alan 0 değerinden büyük olmalı."))))
    s, _, _, _ = fresh()
    out.append(bs.case("yeni kaydın türü de listeden olmalı", *s, polygon(487200.0, {"tur": "fabrika"}, bs.failed(
        "invalid_attribute", "“tur” alanı listedeki değerlerden birini ister; “fabrika” listede yok.", "attrs.tur"))))
    s, _, _, _ = fresh()
    out.append(bs.case("kısıtlara uyan yeni kayıt geçer", *s, polygon(487200.0, {"alan": "50", "tur": "konut",
                                                                                  "kat": "1"})))
    # The table asks what KentOS does not judge: nothing is written, the refusal is said by the constraint.
    ok, why = goes_in(1, {"notu": "A"})
    assert not ok and why == "CHECK constraint failed: notu IS NULL OR hex(notu) <> '41'", why
    s, _, _, _ = fresh()
    out.append(bs.case("KentOS'un bilmediği işlevli kısıtı tablo sorar: source_check", *s, set_step(p5, {"notu": "A"}),
                       bs.step(WRITE, bs.failed(
                           "source_check", "“parsel” tablosunun kısıtı yazmayı reddetti: notu IS NULL OR hex(notu) <> "
                                           "'41'. Hiçbir şey yazılmadı; değeri kısıta göre düzeltip yeniden yazın.",
                           "layer"), command="cad.sources.sync")))
    ok, why = goes_in(1, {"ad": "c"})
    assert not ok and why == "CHECK constraint failed: ad IN ('a', 'b')", why
    s, _, _, _ = fresh()
    out.append(bs.case("harmanlamalı sütunun kısıtını yalnız tablo sorar", *s, set_step(p5, {"ad": "c"}),
                       bs.step(WRITE, bs.failed(
                           "source_check", "“parsel” tablosunun kısıtı yazmayı reddetti: ad IN ('a', 'b'). Hiçbir şey "
                                           "yazılmadı; değeri kısıta göre düzeltip yeniden yazın.", "layer"),
                               command="cad.sources.sync")))
    ok, why = goes_in(1, {"no": 7})
    assert not ok and why == "UNIQUE constraint failed: parsel.no", why
    s, _, _, _ = fresh()
    out.append(bs.case("kısmi tekil dizini tablo sorar: source_unique", *s, set_step(p5, {"no": "7"}),
                       bs.step(WRITE, bs.failed(
                           "source_unique", "“parsel” tablosunun tekil kısıtı (no) yazmayı reddetti: aynı değerler "
                                            "başka bir satırda var; hiçbir şey yazılmadı.", "layer"),
                               command="cad.sources.sync")))
    ok, _ = goes_in(3, {"no": 3})
    assert ok
    s, _, _, _ = fresh()
    out.append(bs.case("kısmi tekil dizine uyan değer yazılacak", *s, set_step(p1, {"no": "3"}),
                       bs.step(WRITE, written(), op="plan", command="cad.sources.sync")))
    # Reading writes the link's constraints again.
    s, _, fs, _ = fresh()
    after = fields(K2)
    assert [f for f in after if f["name"] == "tur"] == [{"name": "tur", "kind": "text"}]
    b2 = {**binding(K2, path_of(n))}
    assert len(b2["checks"]) == 4 and b2["checks"][0]["sql"] == "kat BETWEEN 0 AND 60"
    over = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{K2}", "to": path_of(n)}
    out.append(bs.case("dosyada kısıt kalkınca bağdan da kalkar, verdiği liste de; değişen kısıt yenilenir", *s, over,
                       bs.step(READ, bs.done(bs.synced(name=K2)),
                               {"revision": "changed", "canUndo": True, "settings": {"sources": [b2]},
                                "nodes": {LAYER: {"fields": after}}}, command="cad.sources.sync")))
    s, _, _, _ = fresh()
    over = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{K2}", "to": path_of(n)}
    out.append(bs.case("okumanın planı ne bağı ne alanları yazar", *s, over,
                       bs.step(READ, bs.done(bs.synced(name=K2)), op="plan", command="cad.sources.sync")))
    s, _, _, _ = fresh()
    out.append(bs.case("okunan yeni aralık kat'ı yeniden yargılar", *s, over_with(n),
                       bs.step(READ, bs.done(bs.synced(name=K2)), {"revision": "changed", "canUndo": True},
                               command="cad.sources.sync"),
                       set_step(p5, {"kat": "50"}),
                       set_step(p5, {"kat": "61"}, broken("kat", "Tablonun “kat_aralik” kısıtı: kat 0 ile 60 arasında "
                                                                 "olmalı."))))
    return out


def over_with(n):
    return {"op": "copyFile", "from": f"$fixture:geopackage/v1/{K2}", "to": path_of(n)}


NOTE = ("ADR 0755. Tablonun kısıtları sqlite3 ile okunur, KentOS kodu olmadan: CHECK'ler CREATE metninden (sütunun ve "
        "tablonun, CONSTRAINT adlarıyla), tekil kısıtlar PRAGMA index_list ve index_info'dan (kısmi ve ifadeli dizin "
        "dışarıda, yapıldıkları sırayla); yazmanın kabul edilip edilmediği SQLite'ın kendi cevabı (aynı değerler "
        "dosyanın kopyasına yazılır). Bağ CHECK'leri adıyla, metniyle ve okuduğu sütunlarla tutar, harmanlamalı sütunu "
        "okuyan yalnız tablonundur; tek sütunlu IN listesi alanın listesi olur. KentOS yargıladığı CHECK'i (rule_broken) "
        "ve tekil kısıtı (not_unique) tablodan önce sorar; bilmediği işlevli ve yalnız tablonun kısıtını, kısmi tekil "
        "dizini tablo sorar (source_check, source_unique), hiçbir şey yazılmaz.")


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.edit", "commandVersion": 1,
            "title": "Bağlı tablonun CHECK ve tekil kısıtları", "note": NOTE, "setup": bs.SETUP, "cases": cases()}


def main():
    if "--files" in sys.argv:
        write_files()
        return
    import json
    doc = build()
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            raise SystemExit(f"{OUT.relative_to(ROOT)} eski; betiği --check olmadan çalıştırın")
        print(f"{len(doc['cases'])} durum tutuyor")
        return
    OUT.write_text(text)
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")


if __name__ == "__main__":
    main()
