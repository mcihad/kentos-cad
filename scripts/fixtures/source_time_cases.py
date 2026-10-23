#!/usr/bin/env python3
"""The shared cases of a source question's time (docs/adr/0595, plan/gis.md CBS-33): `cad.sources.query`'s `time`.

    python3 scripts/fixtures/source_time_cases.py --files   # GDAL writes fixtures/sources/v1/zaman.gpkg
    python3 scripts/fixtures/source_time_cases.py           # writes fixtures/commands/v1/cad.sources.query.time.json
    python3 scripts/fixtures/source_time_cases.py --check   # compares

The file is written with GDAL's OGR: `surum`, a parcel's versions, with a valid time (`bas`, `bit`, DATE columns) and a
system time (`kayit`, `silindi`, DATETIME columns), and `olay`, events at moments in a TEXT column written as people
write them (UTC, a zone, a date, `01.06.2024`, nothing, a word). Its rows are read back with Python's sqlite3, not
KentOS code; a DATETIME cell is its first 19 characters (ADR 0502: the drawing's time field is local, its zone left).
What a question keeps is ADR 0210 §4's and ADR 0537's rules (temporal_cases.py's and time_query_cases.py's readings of
them); what the source hands over is ADR 0595 §2's condition, worked out here on the stored texts: a column's value
kept when empty, when it does not begin with an ISO date, or when its first ten characters are within the moments
widened a day on each side.
"""
import hashlib
import json
import sqlite3
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import temporal_cases as tc  # noqa: E402
from source_query_cases import NAMESPACE  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures/sources/v1/zaman.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.sources.query.time.json"
FX = "$fixture:sources/v1/zaman.gpkg"
ANY = "$any"
SAME = {"revision": "same"}
X0, Y0 = 487000.0, 4420000.0
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
DAY = 86_400_000

# A parcel's versions: name, valid start and end (DATE), recorded and superseded (DATETIME). A text the DATE column
# should not hold goes in by sqlite afterwards.
SURUM = [
    ("P1a", "2020-01-01", "2022-06-01", "2020-01-05T09:00:00Z", "2022-06-03T10:00:00Z"),
    ("P1b", "2022-06-01", None, "2022-06-03T10:00:00Z", None),
    ("P2", "2021-03-01", "2024-01-01", "2025-02-01T08:30:00Z", None),
    ("P3", None, "2023-01-01", "2019-05-05T00:00:00Z", None),
    ("P4", "2023-05-05", None, "2023-05-06T12:00:00Z", None),
    ("P5", None, None, "2018-01-01T00:00:00Z", None),
    ("P6", "2020-01-01", None, "2020-01-01T00:00:00Z", None),
    ("P7", "2024-02-29", "2024-03-01", "2024-02-29T00:00:00Z", None),
    ("P8", "2019-01-01", "2020-01-01", "2019-01-02T00:00:00Z", "2021-01-01T00:00:00Z"),
]
LATER = {"P4": ("bas", "05.05.2023"), "P6": ("bas", "bilinmiyor")}
OLAY = [
    ("O1", "2024-06-01T10:00:00Z"),
    ("O2", "2024-06-01T13:00:00+03:00"),
    ("O3", "2024-06-01"),
    ("O4", "01.06.2024"),
    ("O5", None),
    ("O6", "2024-06-01T09:59:59Z"),
    ("O7", "2024-05-31T23:30:00-05:00"),
    ("O8", "yarın"),
    ("O9", "2024-06-02T00:00:00Z"),
]
COLUMNS = {"surum": ["ad", "bas", "bit", "kayit", "silindi"], "olay": ["ad", "zaman"]}
DATETIME = {"surum": {"kayit", "silindi"}, "olay": set()}
VERSIONS = {"start": "bas", "end": "bit", "recordedStart": "kayit", "recordedEnd": "silindi"}
EVENTS = {"start": "zaman"}


def write_files():
    from osgeo import ogr, osr
    ogr.UseExceptions()
    FILE.unlink(missing_ok=True)
    tm = osr.SpatialReference()
    tm.ImportFromEPSG(5256)
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(FILE))
    lyr = ds.CreateLayer("surum", tm, ogr.wkbPoint, ["FID=fid"])
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    for name, kind in (("bas", ogr.OFTDate), ("bit", ogr.OFTDate), ("kayit", ogr.OFTDateTime),
                       ("silindi", ogr.OFTDateTime)):
        lyr.CreateField(ogr.FieldDefn(name, kind))
    for k, (name, bas, bit, kayit, silindi) in enumerate(SURUM):
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(k + 1)
        f.SetField("ad", name)
        for col, v in (("bas", bas), ("bit", bit), ("kayit", kayit), ("silindi", silindi)):
            if v is not None:
                f.SetField(col, v)
        g = ogr.Geometry(ogr.wkbPoint)
        g.AddPoint_2D(X0 + k * 20, Y0)
        f.SetGeometry(g)
        lyr.CreateFeature(f)
    lyr = ds.CreateLayer("olay", tm, ogr.wkbPoint, ["FID=fid"])
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    lyr.CreateField(ogr.FieldDefn("zaman", ogr.OFTString))
    for k, (name, zaman) in enumerate(OLAY):
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(k + 1)
        f.SetField("ad", name)
        if zaman is not None:
            f.SetField("zaman", zaman)
        g = ogr.Geometry(ogr.wkbPoint)
        g.AddPoint_2D(X0 + k * 20, Y0 + 100)
        f.SetGeometry(g)
        lyr.CreateFeature(f)
    for name, (col, value) in LATER.items():
        ds.ExecuteSQL(f"UPDATE surum SET {col} = '{value}' WHERE ad = '{name}'")
    ds = None


# ── The file, read here ──────────────────────────────────────────────────────

def stored(table):
    """Each row's fid, uid, stored texts and attributes (a DATETIME cell its first 19 characters, T for a space)."""
    c = sqlite3.connect(f"file:{FILE}?mode=ro", uri=True)
    cols = COLUMNS[table]
    out = []
    for r in c.execute(f'SELECT fid, {", ".join(chr(34) + k + chr(34) for k in cols)} FROM "{table}" ORDER BY fid'):
        raw = dict(zip(cols, r[1:]))
        attrs = {}
        for k, v in raw.items():
            if v is None:
                continue
            attrs[k] = v[:19].replace(" ", "T") if k in DATETIME[table] else v
        out.append({"fid": r[0], "uid": str(uuid.uuid5(NAMESPACE, f"gpkg:{table}:{r[0]}")), "raw": raw,
                    "attrs": attrs})
    c.close()
    return out


def sha256():
    return hashlib.sha256(FILE.read_bytes()).hexdigest()


# ── The rules ────────────────────────────────────────────────────────────────

def recorded_interval(start, end):
    s = tc.read(start) if start is not None else "empty"
    e = tc.read(end) if end is not None else "empty"
    if not isinstance(s, int) or e == "unreadable":
        return None
    return (s, tc.INF if e == "empty" else e)


def window_of(t):
    if "at" in t:
        return {"instant": tc.read(t["at"])}
    if "from" in t:
        return {"range": (tc.read(t["from"]), tc.read(t["to"]))}
    return None


def keeps(row, t, fields):
    """ADR 0210 §4 and ADR 0537 on the record's attributes."""
    a = row["attrs"]
    rule = {"ranged": "end" in fields, "cumulative": fields.get("cumulative", False)}
    w = window_of(t)
    valid = tc.object_time(rule, a.get(fields["start"]), a.get(fields.get("end")) if rule["ranged"] else None)
    if valid is not None and w is not None and not tc.shows(valid, w):
        return False
    if "knownAt" in t:
        iv = recorded_interval(a.get(fields.get("recordedStart")), a.get(fields.get("recordedEnd")))
        k = tc.read(t["knownAt"])
        return iv is not None and iv[0] <= k < iv[1]
    return True


def day(ms):
    return tc.write(ms, True)


def conditions(t, fields):
    """ADR 0595 §2: (column, '<=' or '>=', day) in the order the source is sent them."""
    out = []
    w = window_of(t)
    rule = {"ranged": "end" in fields, "cumulative": fields.get("cumulative", False)}
    if w is not None:
        lo, hi = (w["instant"], w["instant"]) if "instant" in w else w["range"]
        out.append((fields["start"], "<=", day(hi + DAY)))
        if not rule["cumulative"]:
            if rule["ranged"]:
                out.append((fields["end"], ">=", day(lo - DAY)))
            else:
                out.append((fields["start"], ">=", day(lo - DAY)))
    if "knownAt" in t:
        k = tc.read(t["knownAt"])
        if "recordedStart" in fields:
            out.append((fields["recordedStart"], "<=", day(k + DAY)))
        if "recordedEnd" in fields:
            out.append((fields["recordedEnd"], ">=", day(k - DAY)))
    return out


def iso_start(v):
    return len(v) >= 10 and all(v[i].isdigit() for i in (0, 1, 2, 3, 5, 6, 8, 9)) and v[4] == "-" and v[7] == "-"


def handed(row, conds):
    for col, op, d in conds:
        v = row["raw"].get(col)
        if v is None or not iso_start(str(v)):
            continue
        p = str(v)[:10]
        if not (p <= d if op == "<=" else p >= d):
            return False
    return True


def shown(conds):
    return " AND ".join(
        f"(\"{c}\" IS NULL OR \"{c}\" NOT GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]*' OR "
        f"substr(\"{c}\", 1, 10) {op} '{d}')" for c, op, d in conds)


# ── The answers ──────────────────────────────────────────────────────────────

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


def answer(table, t, fields, *, keep_also=lambda r: True, count=False, layer=None):
    rows = stored(table)
    conds = conditions(t, fields)
    given = [r for r in rows if handed(r, conds)]
    kept = [r for r in given if keep_also(r) and keeps(r, t, fields)]
    source = {"kind": "geopackage", "path": FX, "table": table, "srid": 5256}
    if layer:
        source = {"kind": "geopackage", "layer": layer, "path": FX, "table": table, "srid": 5256}
    plan = {"columns": COLUMNS[table], "geometry": False, "exact": False, "local": ["time"], "order": "key"}
    if conds:
        plan["pushed"] = shown(conds)
    out = {"source": source, "rows": [{"uid": r["uid"], "key": r["fid"], "attrs": r["attrs"]} for r in kept],
           "plan": plan, "partial": False, "read": len(given), "stamp": ANY, "revision": "$current"}
    if count:
        out["total"] = len(kept)
    return out


def q(table, t, **kw):
    out = {"path": FX, "table": table, "time": t}
    out.update(kw)
    return out


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Kaynakta zaman",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False,
                        "expanded": True, "style": STYLE, "children": []}],
            "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []}}


def names(table, t, fields):
    return ", ".join(r["attrs"]["ad"] for r in stored(table) if keeps(r, t, fields)) or "hiçbiri"


def cases():
    out = []

    def asked(title, table, t, fields, **kw):
        tt = {**t, "fields": fields}
        out.append(case(f"{title}: {names(table, t, fields)}",
                        step(q(table, tt, **{k: v for k, v in kw.items() if k != "keep_also"}),
                             done(answer(table, t, fields, keep_also=kw.get("keep_also", lambda r: True),
                                         count=kw.get("count", False))))))

    # The valid time: an instant, a period, the ends of the half-open intervals.
    asked("an 2021-06-01", "surum", {"at": "2021-06-01"}, VERSIONS)
    asked("an 2022-06-01: biten sürümün bitişi dahil değil, yenisinin başlangıcı dahil", "surum", {"at": "2022-06-01"},
          VERSIONS)
    asked("aralık [2022-01-01, 2023-01-01)", "surum", {"from": "2022-01-01", "to": "2023-01-01"}, VERSIONS)
    asked("an 2024-02-29: artık günün tek günlük sürümü", "surum", {"at": "2024-02-29"}, VERSIONS)
    asked("an 2024-03-01: bitiş anı dahil değil", "surum", {"at": "2024-03-01"}, VERSIONS)
    asked("an 2019-06-01: başlangıcı boş sürüm başından beri geçerli", "surum", {"at": "2019-06-01"}, VERSIONS)
    asked("01.06.2023'te: ISO olmayan başlangıç kaynakta tutulur, burada okunur", "surum", {"at": "2023-06-01"},
          VERSIONS)
    asked("birikimli: başlangıcından beri, bitişi sayılmaz", "surum", {"at": "2025-01-01"},
          {**VERSIONS, "cumulative": True})
    # The day known.
    asked("bilinen gün 2022-06-02, an 2022-06-01: yeni sürüm henüz bilinmiyor", "surum",
          {"at": "2022-06-01", "knownAt": "2022-06-02"}, VERSIONS)
    asked("bilinen gün 2024-12-31, an 2022-01-01: geç kaydedilen sürüm bilinmiyor", "surum",
          {"at": "2022-01-01", "knownAt": "2024-12-31"}, VERSIONS)
    asked("bilinen gün 2025-03-01, an 2022-01-01: geç kayıt artık biliniyor", "surum",
          {"at": "2022-01-01", "knownAt": "2025-03-01"}, VERSIONS)
    asked("yalnız bilinen gün 2020-06-01", "surum", {"knownAt": "2020-06-01"}, VERSIONS)
    asked("bilinen gün, kayıt anının kendisi dahil", "surum", {"knownAt": "2022-06-03T10:00:00Z"}, VERSIONS)
    # Moments written as people write them: an instant layer.
    asked("an 2024-06-01T10:00Z: bölgesiyle aynı an da", "olay", {"at": "2024-06-01T10:00:00Z"}, EVENTS)
    asked("an 2024-06-01: gece yarısı, Türkçe tarih de", "olay", {"at": "2024-06-01"}, EVENTS)
    asked("aralık [2024-06-01, 2024-06-02): bölgeli, okunmayan ve boş", "olay",
          {"from": "2024-06-01", "to": "2024-06-02"}, EVENTS)
    asked("aralık bölgeli bir anda başlar", "olay", {"from": "2024-06-01T04:30:00Z", "to": "2024-06-01T10:00:00Z"},
          EVENTS)
    # With the other parts of a question.
    asked("sayım", "surum", {"at": "2021-06-01"}, VERSIONS, count=True, limit=0)
    out[-1]["steps"][0]["result"]["output"]["rows"] = []
    asked("süzgeçle", "surum", {"at": "2021-06-01"}, VERSIONS, filter="ad = 'P2'",
          keep_also=lambda r: r["attrs"].get("ad") == "P2")
    fix = out[-1]["steps"][0]["result"]["output"]
    fix["plan"]["pushed"] = "(\"ad\" = 'P2') AND " + fix["plan"]["pushed"]
    fix["plan"]["local"] = ["filter", "time"]
    fix["read"] = len([r for r in stored("surum") if r["attrs"].get("ad") == "P2"
                       and handed(r, conditions({"at": "2021-06-01"}, VERSIONS))])
    # A layer's own time settings.
    link = step({"edit": {"kind": "link", "path": FX, "table": "surum"}},
                {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"}, "cad.sources.edit")
    settime = step({"layer": "import-surum", "time": dict(VERSIONS)},
                   {"status": "completed", "output": ANY, "warnings": ANY}, {"revision": "changed"}, "cad.layers.time")
    t = {"at": "2022-01-01", "knownAt": "2025-03-01"}
    layered = answer("surum", t, VERSIONS, layer="import-surum")
    out.append(case(f"bağlı katmanın kendi zaman ayarı, kaynağın tümü: {names('surum', t, VERSIONS)}", link, settime,
                    step({"layer": "import-surum", "scope": "source", "time": t}, done(layered))))
    drawn = {"source": {"kind": "drawing", "layer": "import-surum", "srid": 5256},
             "rows": [{"uid": r["uid"], "attrs": r["attrs"]} for r in sorted(stored("surum"), key=lambda r: r["uid"])
                      if keeps(r, t, VERSIONS)],
             "plan": {"columns": [], "geometry": False, "exact": False, "local": ["time"], "order": "uid"},
             "partial": False, "read": len(SURUM), "revision": "$current"}
    out.append(case("bağlı katmanın çalışma kopyası aynı kuralla", link, settime,
                    step({"layer": "import-surum", "time": t}, done(drawn))))
    # İşlemler's input read from its source at a time (docs/adr/0595 §3): the records the question keeps, as points.
    t = {"at": "2021-06-01", "fields": VERSIONS}
    n = sum(1 for r in stored("surum") if keeps(r, t, VERSIONS))
    run = {"tool": "convert.toPoints", "parameters": {"input": {"scope": "source", "path": FX, "table": "surum",
                                                                "time": t}},
           "sink": {"path": "$fixture:processing/v1/sonuc-plan.gpkg"}}
    out.append(case(
        f"İşlemler'in girdisi kaynaktan zamanla okunur: {n} sürüm, {n} nokta",
        step(run, done({"tool": "convert.toPoints", "summary": f"{n} nesneden {n} nokta yazıldı.", "added": [ANY] * n,
                        "outputs": {"count": n},
                        "inputs": [{"layer": "kaynak-input", "digest": ANY, "revision": sha256(),
                                    "table": "zaman.gpkg › surum", "rows": n, "path": FX}],
                        "record": ANY, "revision": "$current", "layers": ["islem-merkezler"], "sink": ANY,
                        "sinkReport": ANY}), op="plan", command="cad.processing.run")))
    bad_run = {**run, "parameters": {"input": {"scope": "source", "path": FX, "table": "surum",
                                               "time": {"at": "dün", "fields": VERSIONS}}}}
    out.append(case(
        "ret: İşlemler'in girdisinin anı okunmuyor",
        step(bad_run, failed("invalid_moment", "“dün” bir an değil: bir tarih (2024-06-01 ya da 01.06.2024) ya da tarih "
                             "ve saat (2024-06-01T14:30, bölgesiyle 2024-06-01T14:30+03:00) yazın.",
                             "parameters.input.time.at"), op="plan", command="cad.processing.run")))
    # Refusals.
    bad = [
        ("an ve aralık birden", {"at": "2021-01-01", "from": "2021-01-01", "to": "2022-01-01", "fields": VERSIONS},
         "invalid_period", "Geçerlilik zamanı ya bir an (at) ya bir aralıktır (from ve to); ikisi birden verilmez.",
         "time.at"),
        ("bitişsiz aralık", {"from": "2021-01-01", "fields": VERSIONS}, "invalid_period",
         "Aralığın başlangıcı (from) ve bitişi (to) birlikte verilir.", "time.to"),
        ("ters aralık", {"from": "2022-01-01", "to": "2021-01-01", "fields": VERSIONS}, "invalid_period",
         "Aralığın bitişi başlangıcından sonra olmalı; [from, to) bitişi içermez.", "time.to"),
        ("an değil", {"at": "dün", "fields": VERSIONS}, "invalid_moment",
         "“dün” bir an değil: bir tarih (2024-06-01 ya da 01.06.2024) ya da tarih ve saat (2024-06-01T14:30, bölgesiyle "
         "2024-06-01T14:30+03:00) yazın.", "time.at"),
        ("boş zaman sorusu", {"fields": VERSIONS}, "invalid_period",
         "Zaman sorusunda bir an (at), bir aralık (from, to) ya da bilinen gün (knownAt) verin.", "time"),
        ("kayıt zamanı olmadan bilinen gün", {"knownAt": "2022-01-01", "fields": {"start": "bas", "end": "bit"}},
         "not_recorded", "Zaman alanları sürümlerin kayıt zamanını tutmuyor; bilinen günle sormak için kayıt başlangıcı "
         "alanını (recordedStart) verin.", "time.knownAt"),
        ("tablonun zaman alanları yok", {"at": "2022-01-01"}, "not_temporal",
         "Tablonun zaman ayarı yok; zaman alanlarını (time.fields: start, isteğe bağlı end, recordedStart) verin.",
         "time.fields"),
        ("başlangıç ve bitiş aynı alan", {"at": "2022-01-01", "fields": {"start": "bas", "end": "bas"}}, "invalid_input",
         "Zaman alanları: başlangıç ve bitiş aynı alan olamaz.", "time.fields"),
        ("bitiş alanı tabloda yok", {"at": "2022-01-01", "fields": {"start": "bas", "end": "son"}}, "field_not_found",
         "“son” sütunu “surum” tablosunda yok.", "time.fields.end"),
    ]
    for name, t, code, msg, path in bad:
        out.append(case(f"ret: {name}", step({"path": FX, "table": "surum", "time": t}, failed(code, msg, path))))
    out.append(case("ret: zaman ayarı olmayan katman", link,
                    step({"layer": "import-surum", "scope": "source", "time": {"at": "2022-01-01"}},
                         failed("not_temporal", "Katmanın zaman ayarı yok; zaman alanlarını (time.fields: start, "
                                "isteğe bağlı end, recordedStart) verin.", "time.fields"))))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.query", "commandVersion": 1,
            "title": "Kaynağı sorgula: zaman (geçerlilik ve bilinen gün) kaynağa da gider",
            "note": "Üretici: scripts/fixtures/source_time_cases.py (docs/adr/0595). Kayıtlar zaman.gpkg'den sqlite3'le; "
                    "kurallar ADR 0210 §4, ADR 0537 ve ADR 0595 §2'den.",
            "setup": setup(), "cases": cases()}


def main():
    if "--files" in sys.argv:
        write_files()
        print(f"{FILE.relative_to(ROOT)} yazıldı")
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
