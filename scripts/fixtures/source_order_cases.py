#!/usr/bin/env python3
"""The shared cases of cad.sources.query's order (docs/adr/0591, plan/gis.md CBS-20).

    python3 scripts/fixtures/source_order_cases.py --files   # GDAL writes fixtures/sources/v1/sira.gpkg
    python3 scripts/fixtures/source_order_cases.py           # writes fixtures/commands/v1/cad.sources.query.order.json
    python3 scripts/fixtures/source_order_cases.py --check   # compares

The file is written here with GDAL's OGR: a point table `olay` of 2400 rows (more than two of the parts a source hands
over) with a text, a real, an integer, a date and a true/false column, some of them NULL and some empty. Its rows are
read back with Python's sqlite3, not KentOS code. The expected orders are written here from ADR 0591's rules, not by
asking a database:

- a number, date or true/false column is ordered by the source as SQLite orders it: numbers by value, dates by their
  text (YYYY-MM-DD), false before true; NULL after every value either way; equal values by key, rising either way;
- a text column, and a layer of the drawing, are ordered here by the attribute table's rule (docs/adr/0199 §4; the
  natural order and the kinds' keys of feature_table_cases.py), a record without a value (or an empty one) last,
  equal ones by their place (key, lasting id);
- a page's `next` is `o:` and the compact JSON of [the last row's key or lasting id, its value]: the source's raw
  value (a number, a date's text, 0 or 1) or the attribute's text here; a page goes on after that place in the order;
- the source hands over rows in parts of 1000 in the order, after the cursor; what is read, counted and summed is
  what comes from the cursor on, as without an order.

Which rows pass a filter and what is sent is ADR 0581 §3's rule, as source_query_cases.py writes it.
"""
import functools
import json
import sqlite3
import sys
import uuid
from datetime import date, timedelta
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import feature_table_cases as ft  # noqa: E402
import source_query_cases as sqc  # noqa: E402
import spatial_query_cases as sq  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures/sources/v1/sira.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.sources.query.order.json"
FX = "$fixture:sources/v1/sira.gpkg"
ANY = "$any"
CHUNK = 1000
X0, Y0 = 487000.0, 4420000.0
ROWS = 2400
NAMES = ["Çınar", "çam", "Ağaç", "ılgın", "İncir", "Zeytin", "armut", "Ödül", "şimşir", "Ulu", "kavak", "ceviz"]
COLUMNS = ["ad", "puan", "sayi", "tarih", "acik"]
KINDS = {"ad": "text", "puan": "real", "sayi": "integer", "tarih": "date", "acik": "boolean"}


def olay_rows():
    out = []
    for fid in range(1, ROWS + 1):
        i = fid - 1
        if fid % 29 == 0:
            ad = None
        elif fid % 31 == 0:
            ad = ""
        else:
            ad = f"{NAMES[(i * 7) % 12]} {(i * 5) % 37}"
        puan = None if fid % 13 == 0 else ((fid * 7919) % 1000) / 8
        sayi = (fid * 37) % 50 - 25
        tarih = None if fid % 11 == 0 else (date(2020, 1, 1) + timedelta(days=(fid * 13) % 900)).isoformat()
        acik = None if fid % 9 == 0 else fid % 2 == 0
        out.append({"fid": fid, "x": X0 + (i % 60) * 10, "y": Y0 + (i // 60) * 10, "ad": ad, "puan": puan,
                    "sayi": sayi, "tarih": tarih, "acik": acik})
    return out


def write_files():
    from osgeo import ogr, osr
    osr.UseExceptions()
    ogr.UseExceptions()
    if FILE.exists():
        FILE.unlink()
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(FILE))
    tm = osr.SpatialReference()
    tm.ImportFromEPSG(5256)
    tm.SetAxisMappingStrategy(osr.OAMS_TRADITIONAL_GIS_ORDER)
    lyr = ds.CreateLayer("olay", tm, ogr.wkbPoint, ["FID=fid"])
    lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
    lyr.CreateField(ogr.FieldDefn("puan", ogr.OFTReal))
    lyr.CreateField(ogr.FieldDefn("sayi", ogr.OFTInteger))
    lyr.CreateField(ogr.FieldDefn("tarih", ogr.OFTDate))
    b = ogr.FieldDefn("acik", ogr.OFTInteger)
    b.SetSubType(ogr.OFSTBoolean)
    lyr.CreateField(b)
    lyr.StartTransaction()
    for r in olay_rows():
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(r["fid"])
        if r["ad"] is not None:
            f.SetField("ad", r["ad"])
        if r["puan"] is not None:
            f.SetField("puan", r["puan"])
        f.SetField("sayi", r["sayi"])
        if r["tarih"] is not None:
            y, m, d = (int(p) for p in r["tarih"].split("-"))
            f.SetField("tarih", y, m, d, 0, 0, 0, 0)
        if r["acik"] is not None:
            f.SetField("acik", 1 if r["acik"] else 0)
        g = ogr.Geometry(ogr.wkbPoint)
        g.AddPoint_2D(r["x"], r["y"])
        f.SetGeometry(g)
        lyr.CreateFeature(f)
    lyr.CommitTransaction()
    ds = None


# ── Reading the file back ────────────────────────────────────────────────────

def attr_text(column, v):
    """A cell as the drawing's attribute (ADR 0502): a true/false column's 0 and 1 are `false` and `true`."""
    if v is None:
        return None
    if KINDS[column] == "boolean":
        return "true" if v else "false"
    if isinstance(v, float):
        return sqc.sq_num(v)
    return str(v)


@functools.cache
def rows():
    c = sqlite3.connect(f"file:{FILE}?mode=ro", uri=True)
    out = []
    cols = ", ".join(f'"{k}"' for k in COLUMNS)
    for r in c.execute(f'SELECT fid, {cols} FROM "olay" ORDER BY fid'):
        raw = dict(zip(COLUMNS, r[1:]))
        attrs = {k: attr_text(k, v) for k, v in raw.items() if v is not None}
        out.append({"fid": r[0], "uid": str(uuid.uuid5(sqc.NAMESPACE, f"gpkg:olay:{r[0]}")), "raw": raw,
                    "attrs": attrs})
    c.close()
    assert len(out) == ROWS
    return out


# ── The orders ───────────────────────────────────────────────────────────────

def source_cmp(column, descending):
    """SQLite's order of the column (ORDER BY c ASC|DESC NULLS LAST, fid): integers and reals by value, texts by
    their bytes; a value always before NULL; equal ones by key."""
    def cmp(a, b):
        x, y = a["raw"][column], b["raw"][column]
        if x is None or y is None:
            o = (x is None) - (y is None)
        else:
            o = (x > y) - (x < y)
            if descending:
                o = -o
        return o or (a["fid"] > b["fid"]) - (a["fid"] < b["fid"])
    return functools.cmp_to_key(cmp)


def here_value(r, field):
    v = r["attrs"].get(field)
    return v if v else None


def here_cmp(field, kind, descending, place):
    """The attribute table's order (feature_table_cases.py's key_cmp), a record without a value last, equal ones by
    their place."""
    def cmp(a, b):
        x, y = here_value(a, field), here_value(b, field)
        if x is None or y is None:
            o = (x is None) - (y is None)
        else:
            o = ft.key_cmp(kind, x, y)
            if descending:
                o = -o
        return o or (place(a) > place(b)) - (place(a) < place(b))
    return functools.cmp_to_key(cmp)


def cursor(place, value):
    return "o:" + json.dumps([place, value], ensure_ascii=False, separators=(",", ":"))


def answer(table_rows, order_kind, field, descending, *, keep=lambda r: True, pushed_keep=lambda r: True,
           fields=None, limit=5, start=None, count=False, statistics=(), drawing=False, layer=None, kind="text",
           pushed=None, exact=True, local=(), read_columns=(), plan_box=None, raw_cursor=True):
    """The answer of an ordered question. `order_kind`: `column` (the source orders) or `here`."""
    place = (lambda r: r["uid"]) if drawing else (lambda r: r["fid"])
    if order_kind == "column":
        key = source_cmp(field, descending)
    else:
        key = here_cmp(field, kind, descending, place)
    handed_all = [r for r in table_rows if pushed_keep(r)]
    ordered = sorted(handed_all, key=key)
    if start is not None:
        at_place, at_value = start
        probe = {"fid": at_place if not drawing else None, "uid": at_place if drawing else None}
        if order_kind == "column":
            probe["raw"] = {field: at_value}
        else:
            probe["attrs"] = {field: at_value} if at_value is not None else {}
        k = key(probe)
        ordered = [r for r in ordered if key(r) > k]
    matched = [r for r in ordered if keep(r)]
    page = matched[:limit]
    more = len(matched) > limit and limit > 0
    out_rows = []
    for r in page:
        attrs = {k: v for k, v in r["attrs"].items() if fields is None or k in fields}
        row = {"uid": r["uid"], "attrs": attrs}
        if not drawing:
            row["key"] = r["fid"]
        out_rows.append(row)
    # What is read: the source's order hands over parts after the cursor until the page and its next are known
    # (everything when counting); an order here reads every record the source hands over, the drawing every object.
    if drawing:
        read = len(table_rows)
    elif order_kind == "here":
        read = len(handed_all)
    elif count or statistics or not more:
        read = len(ordered)
    else:
        read = sqc._chunked(ordered, keep, limit)
    out = {}
    if drawing:
        out["source"] = {"kind": "drawing", "layer": layer, "srid": 5256}
    else:
        out["source"] = {"kind": "geopackage", "path": FX, "table": "olay", "srid": 5256}
        if layer:
            out["source"]["layer"] = layer
    out["rows"] = out_rows
    plan = {"columns": [] if drawing else [c for c in COLUMNS if c in read_columns], "geometry": False,
            "exact": exact, "local": list(local), "order": order_kind}
    if plan_box:
        plan["box"] = plan_box
        plan["geometry"] = False
    if pushed:
        plan["pushed"] = pushed
    out["plan"] = plan
    out["partial"] = False
    out["read"] = read
    if not drawing:
        out["stamp"] = ANY
    out["revision"] = "$current"
    if more:
        last = page[-1]
        if drawing or order_kind == "here":
            v = here_value(last, field)
        else:
            v = last["raw"][field]
            if KINDS[field] == "boolean" and v is not None:
                v = int(v)
        out["next"] = cursor(last["uid"] if drawing else last["fid"], v)
    if count or statistics:
        out["total"] = len(matched)
    if statistics:
        out["statistics"] = []
        for f in statistics:
            fig = sq.figures([r["attrs"].get(f) for r in matched if f in r["attrs"]])
            st = {"field": f, "read": fig["read"], "skipped": fig["skipped"]}
            for k in ("sum", "mean", "min", "max", "std"):
                if fig[k] is not None:
                    st[k] = fig[k]
            out["statistics"].append(st)
    return out


# ── The drawing's layer ──────────────────────────────────────────────────────

STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
DRAWN = [
    # (no, ad, kot): `no` an integer field, `ad` a text one, `kot` a decimal one.
    ("12", "Çeşme 10", "105.5"),
    ("3", "çeşme 9", "99.25"),
    ("-4", "Bahçe", None),
    ("27", "ılık", "100"),
    ("8", "İnci", "100.0001"),
    ("100", "avlu", "-2"),
    (None, "Ödemiş", "7"),
    ("5", "", "50"),
    ("41", "Zeren 2", "12"),
    ("0", "Bahçe 1", "12"),
]
DRAWN_UIDS = [f"0192f5a5-5555-7000-8000-{k + 1:012d}" for k in range(len(DRAWN))]


def drawn_rows():
    out = []
    for k, (no, ad, kot) in enumerate(DRAWN):
        attrs = {}
        if no is not None:
            attrs["no"] = no
        if ad is not None:
            attrs["ad"] = ad
        if kot is not None:
            attrs["kot"] = kot
        out.append({"fid": None, "uid": DRAWN_UIDS[k], "attrs": attrs, "raw": {}})
    return out


def setup():
    entities = [{"kind": "point", "id": k + 1, "layerId": "kayitlar", "attrs": r["attrs"],
                 "p": {"x": X0 + 10 * k, "y": Y0 - 50.0}} for k, r in enumerate(drawn_rows())]
    return {"format": "kentos.document", "version": 2, "name": "Sıralı sorgu",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0},
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [{"id": "kayitlar", "name": "Kayıtlar", "type": "layer", "visible": True, "locked": False,
                        "expanded": True, "style": STYLE, "children": [],
                        "fields": [{"name": "no", "kind": "integer"}, {"name": "ad", "kind": "text"},
                                   {"name": "kot", "kind": "decimal", "scale": 4}]}],
            "activeLayer": "kayitlar", "entities": entities, "uids": DRAWN_UIDS,
            "styles": {"items": [], "categories": []}}


# ── The cases ────────────────────────────────────────────────────────────────

def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def step(inp, result):
    return {"op": "execute", "input": inp, "result": result, "expect": {"revision": "same"}}


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def q(**kw):
    out = {"path": FX, "table": "olay"}
    out.update(kw)
    return out


def num_push(column, op, value):
    return f"(\"{column}\" {op} {sqc.sq_num(value)} OR typeof(\"{column}\") = 'text')"


def cases():
    rs = rows()
    a = lambda r, k: r["attrs"].get(k)  # noqa: E731
    out = []
    ALL = COLUMNS

    def ordered_case(name, field, descending=False, *, start=None, **kw):
        inp = q(order={"field": field, **({"descending": True} if descending else {})}, **kw.pop("inp", {}))
        if start is not None:
            inp["cursor"] = cursor(*start)
        reads = kw.pop("reads", ALL)
        kind = "here" if KINDS[field] == "text" else "column"
        res = answer(rs, kind, field, descending, start=start, read_columns=reads, **kw)
        out.append(case(name, step(inp, done(res))))
        return res

    def follow(res):
        """The place and value the answer's next names."""
        place, value = json.loads(res["next"][2:])
        return place, value

    # 1–4. A real column ordered by the source, both ways, page after page.
    r1 = ordered_case("ondalık sütun kaynakta artan sıralanır; NULL'lar sonda, eşitler anahtarla", "puan",
                      inp={"limit": 5}, limit=5)
    r2 = ordered_case("imleçten sonraki sayfa: eşit değerler arasında anahtarla sürer", "puan",
                      start=follow(r1), inp={"limit": 5}, limit=5)
    ordered_case("üçüncü sayfa", "puan", start=follow(r2), inp={"limit": 5}, limit=5)
    r4 = ordered_case("ondalık sütun azalan: büyükten küçüğe, NULL'lar yine sonda", "puan", True,
                      inp={"limit": 7}, limit=7)
    ordered_case("azalan sıranın sonraki sayfası", "puan", True, start=follow(r4), inp={"limit": 7}, limit=7)
    # 6. The NULL tail: a cursor whose value is null goes on among the NULLs by key.
    nulls = [r for r in rs if r["raw"]["puan"] is None]
    ordered_case("NULL bölümünde imleç: anahtarla sürer", "puan", start=(nulls[3]["fid"], None),
                 inp={"limit": 4}, limit=4)
    # 7. The last page of an order has no next.
    ordered_case("sıranın son sayfası: sonraki imleç yok", "puan", start=(nulls[-3]["fid"], None),
                 inp={"limit": 5}, limit=5)
    # 8. An integer column, negative values.
    ordered_case("tam sayı sütunu: eksiler önce, yalnız istenen sütunlar", "sayi",
                 inp={"limit": 6, "fields": ["ad", "sayi"]}, limit=6, fields=["ad", "sayi"], reads=["ad", "sayi"])
    # 9–10. A date column, both ways.
    r9 = ordered_case("tarih sütunu kaynakta takvim sırasıyla", "tarih", inp={"limit": 5, "fields": ["tarih"]},
                      limit=5, fields=["tarih"], reads=["tarih"])
    ordered_case("tarih sütunu azalan, imleçten", "tarih", True, start=(follow(r9)[0], "2021-06-01"),
                 inp={"limit": 5, "fields": ["tarih"]}, limit=5, fields=["tarih"], reads=["tarih"])
    # 11. A true/false column: false, true, then NULL.
    ordered_case("evet/hayır sütunu: önce hayır, sonra evet, boşlar sonda", "acik",
                 start=(rs[1500]["fid"], 0), inp={"limit": 4, "fields": ["acik"]}, limit=4, fields=["acik"],
                 reads=["acik"])
    # 12. A filter the source cannot check, with the order: parts of 1000 in the order until the page is full.
    keep7 = lambda r: (a(r, "ad") or "").endswith(" 7")  # noqa: E731
    ordered_case("burada denetlenen süzgeç ve kaynağın sırası: 1000'lik parçalar sırayla okunur", "puan",
                 inp={"limit": 40, "filter": "ad gibi '% 7'", "fields": ["ad", "puan"]}, limit=40, keep=keep7,
                 fields=["ad", "puan"], reads=["ad", "puan"], exact=False, local=["filter"])
    # 13. A filter sent with the order.
    gt = lambda r: sqc.gt(a(r, "puan"), 100)  # noqa: E731
    ordered_case("kaynağa giden süzgeç ve sıra, sayıyla", "puan", True,
                 inp={"limit": 3, "filter": "puan > 100", "count": True, "fields": ["ad"]}, limit=3, keep=gt,
                 pushed_keep=gt, count=True, fields=["ad"], reads=["ad", "puan"],
                 pushed=num_push("puan", ">", 100), exact=False, local=["filter"])
    # 14. Statistics count what comes from the cursor on.
    ordered_case("imleçten sonraki kayıtların sayısı ve istatistiği", "sayi", start=(rs[200]["fid"], 10),
                 inp={"limit": 2, "statistics": ["puan"], "fields": ["sayi"]}, limit=2, statistics=["puan"],
                 fields=["sayi"], reads=["puan", "sayi"], local=["statistics"])
    # 15–18. A text column is ordered here, in the natural Turkish order.
    r15 = ordered_case("metin sütunu burada doğal Türkçe sırayla; bütün tablo okunur", "ad",
                       inp={"limit": 6, "fields": ["ad"]}, limit=6, fields=["ad"], reads=["ad"])
    ordered_case("burada sıranın sonraki sayfası", "ad", start=follow(r15), inp={"limit": 6, "fields": ["ad"]},
                 limit=6, fields=["ad"], reads=["ad"])
    ordered_case("metin sütunu azalan; boş ve NULL sonda", "ad", True, inp={"limit": 6, "fields": ["ad"]}, limit=6,
                 fields=["ad"], reads=["ad"])
    ordered_case("burada sıra, boş değerlerin arasında imleç", "ad",
                 start=([r for r in rs if not a(r, "ad")][5]["fid"], None), inp={"limit": 4, "fields": ["ad"]},
                 limit=4, fields=["ad"], reads=["ad"])
    # 19. A filter sent, the order here.
    neg = lambda r: sqc.lt(a(r, "sayi"), 0)  # noqa: E731
    ordered_case("kaynağa giden süzgeç, burada sıra", "ad",
                 inp={"limit": 5, "filter": "sayi < 0", "count": True, "fields": ["ad", "sayi"]}, limit=5,
                 keep=neg, pushed_keep=neg, count=True, fields=["ad", "sayi"], reads=["ad", "sayi"],
                 pushed=num_push("sayi", "<", 0), exact=False, local=["filter"])
    # 20. A cursor whose record is gone: the page goes on after where it stood.
    ordered_case("imlecin kaydı yoksa sıradaki yerinden sürer", "ad", start=(99999, "Ulu 12"),
                 inp={"limit": 3, "fields": ["ad"]}, limit=3, fields=["ad"], reads=["ad"])

    # 21–25. The drawing's layer: ordered here by the fields' kinds.
    dr = drawn_rows()

    def drawn_case(name, field, kind, descending=False, start=None, limit=3):
        inp = {"layer": "kayitlar", "order": {"field": field, **({"descending": True} if descending else {})},
               "limit": limit}
        if start is not None:
            inp["cursor"] = cursor(*start)
        res = answer(dr, "here", field, descending, start=start, drawing=True, layer="kayitlar", kind=kind,
                     limit=limit)
        out.append(case(name, step(inp, done(res))))
        return res

    d1 = drawn_case("çizimin katmanı tam sayı alanıyla: sayı sırası, değersiz sonda", "no", "number")
    drawn_case("çizimin sonraki sayfası kalıcı kimlikle", "no", "number", start=follow(d1))
    drawn_case("ondalık alan azalan: tam değerleriyle", "kot", "number", True, limit=4)
    drawn_case("metin alanı doğal Türkçe sırayla, boş sonda", "ad", "text", limit=10)
    drawn_case("eşit değerler kalıcı kimlikle sıralanır", "kot", "number", start=(DRAWN_UIDS[0], "12"), limit=2)

    # Refusals.
    bad = [
        ("sırada olmayan sütun", q(order={"field": "yok"}), "field_not_found", "“yok” sütunu “olay” tablosunda yok.",
         "order.field"),
        ("çizimde olmayan alan", {"layer": "kayitlar", "order": {"field": "yok"}}, "field_not_found",
         "“yok” alanı “Kayıtlar” katmanında yok.", "order.field"),
        ("sırasız soruya sıranın imleci", q(cursor=cursor(5, 1.5)), "invalid_cursor",
         "Bu imleç bir sıralamanın; sırayı (order) ilk sayfadaki gibi verin.", "cursor"),
        ("sıralı soruya anahtar imleci", q(order={"field": "puan"}, cursor="k:5"), "invalid_cursor",
         "Bu imleç sırasız bir sorunun; sıralı sorunun sayfaları bir önceki sıralı cevabın next'iyle sürer.",
         "cursor"),
        ("bozuk sıra imleci", q(order={"field": "puan"}, cursor="o:[5]"), "invalid_cursor",
         "“o:[5]” bu sorunun sayfa imleci değil; bir önceki cevabın next'ini verin.", "cursor"),
        ("boşluklu sıra imleci", q(order={"field": "puan"}, cursor="o:[5, 1.5]"), "invalid_cursor",
         "“o:[5, 1.5]” bu sorunun sayfa imleci değil; bir önceki cevabın next'ini verin.", "cursor"),
        ("kaynağa çizimin sıra imleci", q(order={"field": "puan"}, cursor=cursor(DRAWN_UIDS[0], 1.5)),
         "invalid_cursor", "Bu imleç çizimin; bir kaynağın kayıtları anahtarlarıyla sayfalanır.", "cursor"),
        ("çizime kaynağın sıra imleci", {"layer": "kayitlar", "order": {"field": "no"}, "cursor": cursor(5, "3")},
         "invalid_cursor", "Bu imleç bir kaynağın; çizimin nesneleri kalıcı kimlikleriyle sayfalanır.", "cursor"),
    ]
    for name, inp, code, msg, path in bad:
        out.append(case(f"ret: {name}", step(inp, failed(code, msg, path))))
    out.append(case("ret: dosya ilk sayfadan beri değişti",
                    step(q(order={"field": "puan"}, expectedStamp="0:0"),
                         failed("source_changed", ANY, "expectedStamp", status="conflict"))))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.sources.query", "commandVersion": 1,
            "title": "Kaynağı sorgula: sıralama kaynakta ya da burada, sıralı sayfalar",
            "note": "ADR 0591. Dosyayı GDAL yazar (--files); satırlar sqlite3 ile dosyanın kendisinden okunur. Sayı, "
                    "tarih ve evet/hayır sütununun sırası SQLite'ın değer sırasıyla (NULL sonda, eşitler anahtarla), "
                    "metin sütunu ve çizimin katmanı öznitelik tablosunun kuralıyla (feature_table_cases.py) burada; "
                    "imleç son kaydın yeri ve değeri. Süzgeçler ve gönderilen SQL source_query_cases.py'nin "
                    "kurallarıyla. Bağımsız başvuru: scripts/fixtures/source_order_cases.py.",
            "setup": setup(), "cases": cases()}


def main():
    if "--files" in sys.argv:
        write_files()
        return
    data = build()
    text_out = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text() != text_out:
            print(f"{OUT} güncel değil", file=sys.stderr)
            sys.exit(1)
        print(f"{OUT.name}: {len(data['cases'])} durum")
        return
    OUT.write_text(text_out)
    print(f"{OUT.name}: {len(data['cases'])} durum yazıldı")


if __name__ == "__main__":
    main()
