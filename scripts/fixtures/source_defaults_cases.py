#!/usr/bin/env python3
"""The shared cases of a bound table's NOT NULL columns and defaults (docs/adr/0750): what a bound layer's fields take
from the table, what a write leaves to the table and gets back, what it refuses.

    python3 scripts/fixtures/source_defaults_cases.py           # writes fixtures/commands/v1/cad.sources.defaults.json
    python3 scripts/fixtures/source_defaults_cases.py --check   # compares
    /usr/bin/python3 scripts/fixtures/source_defaults_cases.py --files   # GDAL and sqlite3 write kurallar.gpkg

The table's rules are read here with Python's sqlite3 from `PRAGMA table_info`, not with KentOS code; ADR 0750's
rules are written out again below: NOT NULL without a default is a required field; a literal default (a quoted text,
a number, true or false; SQLite's 0 and 1 for a boolean) is the field's default in its kind's text; NULL and an
expression the database works out (`upper('ilce')`, CURRENT_TIMESTAMP) give the field nothing. A new object takes the
field defaults when made (ADR 0199 §2); its empty value of a column with a default is left to the table, whose value
comes back to the object in the write's step; a column the table keeps NOT NULL left empty is refused, nothing written.
The objects and the binding come from bound_sources_cases.py's link (GDAL's OGR reads the rows).
"""
import json
import sqlite3
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import bound_sources_cases as bs  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
OUT = ROOT / "fixtures/commands/v1/cad.sources.defaults.json"
K = "kurallar.gpkg"
TABLE = "ilce"
LAYER = "import-ilce"
ANY = "$any"
# The two rows the file starts with.
ROWS = [
    ("Merkez", "İlçe", 1, 1, 2.5, "A", "2026-01-01T10:00:00", 4, None, "x",
     [(487000, 4420000), (487040, 4420000), (487040, 4420030), (487000, 4420030)]),
    ("Suşehri", "İlçe", 2, 0, 0.5, "B", "2026-02-01T10:00:00", 2, "n", None,
     [(487050, 4420000), (487090, 4420000), (487090, 4420030), (487050, 4420030)]),
]
COLUMNS = """
  "ad" TEXT NOT NULL,
  "tur" TEXT NOT NULL DEFAULT 'İlçe',
  "kod" INTEGER DEFAULT 7,
  "merkez" BOOLEAN NOT NULL DEFAULT 0,
  "oran" REAL DEFAULT (-1.5),
  "etiket" TEXT DEFAULT (upper('ilce')),
  "zaman" DATETIME DEFAULT CURRENT_TIMESTAMP,
  "kat" INTEGER DEFAULT '3',
  "notu" TEXT DEFAULT NULL,
  "soz" TEXT DEFAULT 'Ali''nin'
"""
NAMES = ["ad", "tur", "kod", "merkez", "oran", "etiket", "zaman", "kat", "notu", "soz"]


def write_files():
    """kurallar.gpkg: GDAL makes the GeoPackage and its two features, sqlite3 makes the table again with its rules (GDAL
    writes neither expression defaults nor these NOT NULLs), the spatial index left out."""
    from osgeo import ogr, osr
    path = DIR / K
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
    c.executescript(f'CREATE TABLE "{TABLE}_yeni" ("fid" INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, '
                    f'"geom" POLYGON, {COLUMNS});')
    for fid, r in enumerate(ROWS, start=1):
        geom = c.execute(f'SELECT geom FROM "{TABLE}" WHERE fid = ?', (fid,)).fetchone()[0]
        c.execute(f'INSERT INTO "{TABLE}_yeni" VALUES (?, ?, {", ".join("?" * len(NAMES))})',
                  (fid, geom, *r[:-1]))
    c.executescript(f'DROP TABLE "{TABLE}"; ALTER TABLE "{TABLE}_yeni" RENAME TO "{TABLE}";'
                    "DELETE FROM gpkg_ogr_contents WHERE table_name = 'ilce';")
    c.commit()
    c.execute("VACUUM")
    c.close()
    print(f"yazıldı: {K}")


# ── ADR 0750's rules, written out again ──

KIND = {"TEXT": "text", "INTEGER": "integer", "BOOLEAN": "boolean", "REAL": "decimal", "DATETIME": "datetime"}


def literal(default):
    """A default as a value when it is a literal, else None (ADR 0750 §1)."""
    if default is None:
        return None
    t = default.strip()
    while t.startswith("(") and t.endswith(")"):
        t = t[1:-1].strip()
    if t.startswith("'") and t.endswith("'") and len(t) >= 2:
        inner = t[1:-1]
        if "'" in inner.replace("''", ""):
            return None
        return inner.replace("''", "'")
    if t.lower() in ("true", "false"):
        return t.lower()
    try:
        float(t)
        return t
    except ValueError:
        return None


def as_kind(kind, text):
    """A literal in its kind's canonical text, or None when it does not read as one."""
    if kind == "boolean":
        return {"0": "false", "1": "true", "true": "true", "false": "false"}.get(text)
    if kind == "integer":
        try:
            return str(int(text))
        except ValueError:
            return None
    if kind == "decimal":
        try:
            v = float(text)
        except ValueError:
            return None
        return repr(v) if v != int(v) else str(int(v))
    return text or None


def fields():
    """The bound layer's fields from the table's own definition (sqlite3, PRAGMA table_info)."""
    c = sqlite3.connect(f"file:{DIR / K}?mode=ro", uri=True)
    out = []
    for _, name, decl, notnull, default, pk in c.execute(f'PRAGMA table_info("{TABLE}")'):
        if pk or name == "geom":
            continue
        kind = KIND[decl.upper()]
        f = {"name": name, "kind": kind}
        if notnull and default is None:
            f["required"] = True
        lit = literal(default)
        value = as_kind(kind, lit) if lit is not None else None
        if value is not None:
            f["default"] = value
        # docs/adr/0753 §1: a default the table works out is the field's `column.made`, as the table gives it.
        if default is not None and lit is None and default.strip().upper() != "NULL":
            f["column"] = {"made": default.strip()}
        out.append(f)
    return out


FIELDS = None


def field_defaults():
    return {f["name"]: f["default"] for f in FIELDS if "default" in f}


def computed():
    """The columns whose default the table works out: an empty value of theirs comes back from the table."""
    c = sqlite3.connect(f"file:{DIR / K}?mode=ro", uri=True)
    out = {}
    for _, name, decl, _, default, pk in c.execute(f'PRAGMA table_info("{TABLE}")'):
        if pk or default is None or literal(default) is not None or default.upper() == "NULL":
            continue
        out[name] = default
    return out


def evaluated(name):
    """What SQLite gives a computed default (a time stamp is any value)."""
    default = computed()[name]
    if default.upper().startswith("CURRENT_"):
        return ANY
    c = sqlite3.connect(":memory:")
    return c.execute(f"SELECT {default}").fetchone()[0]


# ── The cases ──

def path_of(n):
    return f"$tmp:kurallar-{n}.gpkg"


def bound(n):
    """The file copied and its table bound onto a new layer: the steps, the layer's next object id."""
    answer, warnings, objs, ids, b = bs.linked(TABLE, name=K)
    path = path_of(n)
    b = {**b, "path": path}
    answer = {**answer, "link": b}
    copy = {"op": "copyFile", "from": f"$fixture:geopackage/v1/{K}", "to": path}
    link = bs.step({"edit": {"kind": "link", "path": path, "table": TABLE}}, bs.done(answer, warnings),
                   {"ids": [1, *ids], "fields": objs, "revision": "changed", "canUndo": True,
                    "settings": {"sources": [b]}, "nodes": {LAYER: {"fields": FIELDS}}},
                   command="cad.sources.edit")
    return [copy, link], max(ids) + 1


def polygon(x, attrs=None):
    ring = [{"x": x, "y": 4420040.0}, {"x": x + 20.0, "y": 4420040.0}, {"x": x + 20.0, "y": 4420060.0},
            {"x": x, "y": 4420060.0}]
    inp = {"layerId": LAYER, "pts": ring}
    if attrs is not None:
        inp["attrs"] = attrs
    return bs.other("cad.polygon.create", inp)


def written(added=0, modified=0):
    return {"added": added, "modified": modified, "removed": 0, "source": ANY, "revision": "$current"}


WRITE = {"layer": LAYER, "direction": "write"}
READ = {"layer": LAYER, "direction": "read"}


def made(attrs):
    """A new object's attributes: those given, the fields' defaults where none (ADR 0199 §2)."""
    out = dict(field_defaults())
    out.update(attrs)
    return out


def back(attrs):
    """The attributes after the write: the table's values of the computed columns left empty."""
    out = dict(attrs)
    for name in computed():
        if name not in out:
            out[name] = evaluated(name)
    return out


def not_null(names, n, uid):
    words = ", ".join(f"“{x}”" for x in names)
    return {"status": "failed", "error": {
        "code": "source_not_null",
        "message": f"“{TABLE}” tablosu {words} sütununun boş kalmasına izin vermiyor (NOT NULL); {n} nesnede boş. "
                   "O nesnelere değer verin (formda zorunlu); hiçbir şey yazılmadı.",
        "path": "layer", "uids": uid}}


def cases():
    global FIELDS
    FIELDS = fields()
    out = []
    by = {f["name"]: f for f in FIELDS}

    def fresh(name, *steps, n):
        s, _ = bound(n)
        out.append(bs.case(name, *s, *steps))

    # The fields a binding takes from the table (one case a rule).
    fresh("bağlanınca NOT NULL ve varsayılansız sütunun alanı zorunlu", n=1)
    for n, (name, words) in enumerate((
            ("tur", "tırnaklı metin varsayılanı alanın varsayılanı"),
            ("kod", "sayı varsayılanı alanın varsayılanı"),
            ("merkez", "SQLite'ın 0'ı mantıksal alanın false varsayılanı, NOT NULL olsa da zorunlu değil"),
            ("oran", "ayraçlı eksi sayı varsayılanı alanın varsayılanı"),
            ("kat", "tırnaklı sayı tam sayı alanının varsayılanı"),
            ("soz", "çift tırnak tek tırnağa döner"),
            ("etiket", "hesaplanan varsayılan alana konmaz"),
            ("zaman", "zaman damgası varsayılanı alana konmaz"),
            ("notu", "NULL varsayılanı varsayılan değil")), start=2):
        f = by[name]
        assert ("default" in f) == (name not in ("etiket", "zaman", "notu")), (name, f)
        fresh(f"bağlanınca {words}", n=n)
    # A new object: its defaults written, the computed ones left to the table and back.
    n = 20
    s, nxt = bound(n)
    attrs = made({"ad": "Yeni"})
    out.append(bs.case("yeni nesne alanların varsayılanlarını alır", *s, polygon(487100.0, {"ad": "Yeni"}),
                       bs.step(WRITE, bs.done(written(added=1)), {"fields": {str(nxt): {"attrs": back(attrs)}}})))
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("hesaplanan sütunun değerini tablo verir, nesneye aynı adımda döner", *s,
                       polygon(487100.0, {"ad": "Yeni"}),
                       bs.step(WRITE, bs.done(written(added=1)),
                               {"fields": {str(nxt): {"attrs": {**made({"ad": "Yeni"}),
                                                                "etiket": evaluated("etiket"), "zaman": ANY}}},
                                "revision": "changed", "canUndo": True})))
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("yazıldıktan sonra okumak bir şey değiştirmez", *s, polygon(487100.0, {"ad": "Yeni"}),
                       bs.step(WRITE, bs.done(written(added=1)), {"revision": "changed"}),
                       bs.step(READ, bs.done(written()), bs.SAME)))
    n += 1
    s, nxt = bound(n)
    given = {"ad": "Yeni", "etiket": "Benim"}
    out.append(bs.case("değer verilen sütun varsayılanı almaz", *s, polygon(487100.0, given),
                       bs.step(WRITE, bs.done(written(added=1)),
                               {"fields": {str(nxt): {"attrs": back(made(given))}}})))
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("iki yeni nesne birlikte: ikisi de tablonun değerini alır", *s,
                       polygon(487100.0, {"ad": "Bir"}), polygon(487130.0, {"ad": "İki"}),
                       bs.step(WRITE, bs.done(written(added=2)),
                               {"fields": {str(nxt): {"attrs": back(made({"ad": "Bir"}))},
                                           str(nxt + 1): {"attrs": back(made({"ad": "İki"}))}}})))
    n += 1
    s, nxt = bound(n)
    cleared = made({"ad": "Yeni"})
    cleared.pop("merkez")
    out.append(bs.case("boşaltılan varsayılanlı NOT NULL sütunu tablo doldurur", *s,
                       polygon(487100.0, {"ad": "Yeni"}),
                       bs.other("cad.entities.set", {"uids": [f"$uidOf:{nxt}"], "attrs": {"merkez": None},
                                                     "operation": "attributes"}),
                       bs.step(WRITE, bs.done(written(added=1)),
                               {"fields": {str(nxt): {"attrs": {**back(cleared), "merkez": "false"}}}})))
    # Refused: a NOT NULL column without a default left empty.
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("zorunlu sütunu boş yeni nesne yazılmaz, nesnesi söylenir", *s, polygon(487100.0),
                       bs.step(WRITE, not_null(["ad"], 1, [f"$uidOf:{nxt}"]))))
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("plan da reddeder", *s, polygon(487100.0),
                       bs.step(WRITE, not_null(["ad"], 1, [f"$uidOf:{nxt}"]), op="plan")))
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("iki boş nesne: ikisi de söylenir", *s, polygon(487100.0), polygon(487130.0),
                       bs.step(WRITE, not_null(["ad"], 2, [f"$uidOf:{nxt}", f"$uidOf:{nxt + 1}"]))))
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("biri boş biri dolu: hiçbiri yazılmaz", *s, polygon(487100.0, {"ad": "Dolu"}),
                       polygon(487130.0),
                       bs.step(WRITE, not_null(["ad"], 1, [f"$uidOf:{nxt + 1}"])),
                       bs.step(WRITE, not_null(["ad"], 1, [f"$uidOf:{nxt + 1}"]), op="plan")))
    n += 1
    s, nxt = bound(n)
    out.append(bs.case("zorunlu alan nesnede boşaltılamaz", *s,
                       bs.step({"uids": ["$uidOf:2"], "attrs": {"ad": None}, "operation": "attributes"},
                               {"status": "failed", "error": {"code": "attribute_required", "message": ANY,
                                                              "path": "attrs.ad"}},
                               command="cad.entities.set")))
    return out


NOTE = ("ADR 0750. Tablonun kuralları sqlite3 ile PRAGMA table_info'dan okunur, KentOS kodu olmadan; satırlar ve bağ "
        "bound_sources_cases.py'nin bağlamasından (GDAL'ın OGR'ı). NOT NULL ve varsayılansız sütun zorunlu alan, sabit "
        "varsayılan alanın varsayılanı, hesaplanan varsayılan alanın column.made'i (ADR 0753); yeni nesnenin boş değeri "
        "varsayılanlı sütunda tablonundur ve aynı adımda nesneye döner; NOT NULL sütunu boş bırakan yazma reddedilir, "
        "hiçbir şey yazılmaz.")


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.sync", "commandVersion": 1,
            "title": "Bağlı tablonun zorunlu ve varsayılanlı sütunları", "note": NOTE, "setup": bs.SETUP,
            "cases": cases()}


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
