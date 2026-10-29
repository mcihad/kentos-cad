#!/usr/bin/env python3
"""The shared cases of a source that could not be read (docs/adr/0740, plan/gis.md CBS-40).

    python3 scripts/fixtures/source_trouble_cases.py           # writes fixtures/commands/v1/cad.sources.query.trouble.json
    python3 scripts/fixtures/source_trouble_cases.py --check   # compares

A page asked of a GeoPackage that is not where the question says is refused and nothing is written: a file moved
away or deleted (`file_not_found`), a file that is not a GeoPackage in its place (`import_unreadable`), a GeoPackage
without the table (`table_not_found`). The same question asked again once the file is back goes on from its cursor:
its rows are the ones an uninterrupted reading gives, no row twice, none skipped. The file asked in its new place
with the old cursor goes on there too (a cursor is the table's, not the path's), and a bound layer whose file moved
goes on once it is relinked to the new place (`cad.sources.edit`'s `relink`). A missing file is said before a stale
stamp (the file is looked for first).

The pages are written here from ADR 0581's and 0591's rules over the file read with Python's sqlite3, as
source_order_cases.py and source_query_cases.py write them, not by KentOS code. Each case works on its own copy of
`fixtures/sources/v1/sira.gpkg` in the runners' `$tmp:` folder; the runners' `moveFile` step renames it (its bytes
and time kept, as a move on one disk does).
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import source_order_cases as so  # noqa: E402
import source_query_cases as sqc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.sources.query.trouble.json"
FX = so.FX
ANY = "$any"
COLUMNS = so.COLUMNS
NOT_GPKG = "$fixture:csv/v1/nokta.csv"
OTHER_GPKG = "$fixture:sources/v1/sorgu.gpkg"
LAYER = "import-olay"


def tmp(n, name="sira.gpkg"):
    return f"$tmp:sorun-{n}/{name}"


def copy(to, source=FX):
    return {"op": "copyFile", "from": source, "to": to}


def move(a, b):
    return {"op": "moveFile", "from": a, "to": b}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, path):
    return {"status": "failed", "error": {"code": code, "message": ANY, "path": path}}


def conflict(code, path):
    return {"status": "conflict", "error": {"code": code, "message": ANY, "path": path}}


def step(inp, result, command=None):
    out = {"op": "execute", "input": inp, "result": result, "expect": {"revision": "same"}}
    if command:
        out["command"] = command
    return out


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def link(path):
    """The table bound as a layer (its rows the drawing's objects; the answer is the binding's own cases')."""
    return {"op": "execute", "command": "cad.sources.edit", "input": {"edit": {"kind": "link", "path": path,
                                                                                "table": "olay"}},
            "result": {"status": "completed", "output": ANY, "warnings": ANY}, "expect": {"revision": "changed"}}


def relink(path):
    return {"op": "execute", "command": "cad.sources.edit", "input": {"edit": {"kind": "relink", "id": "kaynak-1",
                                                                                "path": path}},
            "result": {"status": "completed", "output": ANY, "warnings": ANY}, "expect": {"revision": "changed"}}


def key_answer(path, *, after=None, limit=100, fields=None, count=False, statistics=(), keep=lambda r: True,
               pushed_keep=lambda r: True, pushed=None, exact=True, local=(), reads=None, layer=None):
    """A question in the table's key order (ADR 0581): rows after the cursor's key, the source handing over the ones
    its condition keeps in parts of 1000, every one of them checked here."""
    rs = so.rows()
    start = [r for r in rs if after is None or r["fid"] > after]
    handed = [r for r in start if pushed_keep(r)]
    matched = [r for r in handed if keep(r)]
    page = matched[:limit]
    more = len(matched) > limit and limit > 0
    read = len(handed) if (count or statistics or not more) else sqc._chunked(handed, keep, limit)
    rows = []
    for r in page:
        rows.append({"uid": r["uid"], "attrs": {k: v for k, v in r["attrs"].items() if fields is None or k in fields},
                     "key": r["fid"]})
    wanted = set(reads or ()) | set(statistics)
    columns = [c for c in COLUMNS if fields is None or c in fields or c in wanted]
    plan = {"columns": columns, "geometry": False, "exact": exact, "local": list(local), "order": "key"}
    if pushed:
        plan["pushed"] = pushed
    source = {"kind": "geopackage", "path": path, "table": "olay", "srid": 5256}
    if layer:
        source["layer"] = layer
    out = {"source": source, "rows": rows, "plan": plan, "partial": False, "read": read, "stamp": ANY,
           "revision": "$current"}
    if more:
        out["next"] = f"k:{page[-1]['fid']}"
    if count or statistics:
        out["total"] = len(matched)
    if statistics:
        out["statistics"] = []
        for f in statistics:
            fig = so.sq.figures([r["attrs"].get(f) for r in matched if f in r["attrs"]])
            st = {"field": f, "read": fig["read"], "skipped": fig["skipped"]}
            for k in ("sum", "mean", "min", "max", "std"):
                if fig[k] is not None:
                    st[k] = fig[k]
            out["statistics"].append(st)
    return out


def ordered_answer(path, field, descending=False, *, start=None, limit=50, fields=None, reads=None, **kw):
    """A question ordered by `field` (ADR 0591): by the source for a number, date or true/false column, here for a
    text one; the order cases' rule, over this case's copy of the file."""
    kind = "here" if so.KINDS[field] == "text" else "column"
    out = so.answer(so.rows(), kind, field, descending, start=start, limit=limit, fields=fields,
                    read_columns=reads or COLUMNS, **kw)
    out["source"]["path"] = path
    return out


def follow(answer):
    """The place and value an ordered answer's next names."""
    place, value = json.loads(answer["next"][2:])
    return place, value


def cursor_of(answer):
    return answer["next"]


def cases():
    out = []
    a = lambda r, k: r["attrs"].get(k)  # noqa: E731

    # 1. The first page of a file that is not there: refused, written nothing; once it is there, the first page.
    p = tmp(1)
    out.append(case(
        "dosya yerinde değilken ilk sayfa reddedilir (file_not_found); dosya gelince ilk sayfa okunur",
        copy(tmp(1, "uzakta.gpkg")),
        step({"path": p, "table": "olay", "limit": 300, "count": True}, failed("file_not_found", "path")),
        move(tmp(1, "uzakta.gpkg"), p),
        step({"path": p, "table": "olay", "limit": 300, "count": True},
             done(key_answer(p, limit=300, count=True)))))

    # 2. The second page while the file is away: refused; the same question once it is back goes on from the cursor.
    p = tmp(2)
    first = key_answer(p, limit=300)
    second = key_answer(p, after=300, limit=300)
    out.append(case(
        "ikinci sayfa okunurken dosya taşındı: reddedilir; geri gelince aynı imleçle kaldığı yerden sürer",
        copy(p),
        step({"path": p, "table": "olay", "limit": 300}, done(first)),
        move(p, tmp(2, "baska/sira.gpkg")),
        step({"path": p, "table": "olay", "limit": 300, "cursor": cursor_of(first)}, failed("file_not_found", "path")),
        move(tmp(2, "baska/sira.gpkg"), p),
        step({"path": p, "table": "olay", "limit": 300, "cursor": cursor_of(first)}, done(second))))

    # 3. Pages after a lost one, to the table's end: their rows the whole table once.
    p = tmp(3)
    p1 = key_answer(p, limit=800)
    p2 = key_answer(p, after=800, limit=800)
    p3 = key_answer(p, after=1600, limit=800)
    assert [r["key"] for r in p1["rows"] + p2["rows"] + p3["rows"]] == list(range(1, so.ROWS + 1))
    assert "next" not in p3
    out.append(case(
        "kaybolan sayfadan sonra tablonun sonuna: sayfaların satırları bütün tablo, her biri bir kez",
        copy(p),
        step({"path": p, "table": "olay", "limit": 800}, done(p1)),
        move(p, tmp(3, "silindi.gpkg")),
        step({"path": p, "table": "olay", "limit": 800, "cursor": cursor_of(p1)}, failed("file_not_found", "path")),
        move(tmp(3, "silindi.gpkg"), p),
        step({"path": p, "table": "olay", "limit": 800, "cursor": cursor_of(p1)}, done(p2)),
        step({"path": p, "table": "olay", "limit": 800, "cursor": cursor_of(p2)}, done(p3))))

    # 4. The file in its new place with the old cursor: a cursor is the table's, not the path's.
    p, q = tmp(4), tmp(4, "yeni-yer/sira.gpkg")
    first = key_answer(p, limit=250)
    out.append(case(
        "dosyanın yeni yerinde eski imleçle: imleç tablonundur, yolun değil; sayfa yeni yerde sürer",
        copy(p),
        step({"path": p, "table": "olay", "limit": 250}, done(first)),
        move(p, q),
        step({"path": q, "table": "olay", "limit": 250, "cursor": cursor_of(first)},
             done(key_answer(q, after=250, limit=250)))))

    # 5. A file that is not a GeoPackage in its place: unreadable; the GeoPackage back, the page goes on.
    p = tmp(5)
    first = key_answer(p, limit=400)
    out.append(case(
        "dosyanın yerinde GeoPackage olmayan bir dosya: okunamaz (import_unreadable); asıl dosya dönünce sürer",
        copy(p),
        step({"path": p, "table": "olay", "limit": 400}, done(first)),
        copy(p, NOT_GPKG),
        step({"path": p, "table": "olay", "limit": 400, "cursor": cursor_of(first)},
             failed("import_unreadable", "path")),
        copy(p),
        step({"path": p, "table": "olay", "limit": 400, "cursor": cursor_of(first)},
             done(key_answer(p, after=400, limit=400)))))

    # 6. Another GeoPackage without the table: the table is not there; the file back, the page goes on.
    p = tmp(6)
    first = key_answer(p, limit=400)
    out.append(case(
        "yerinde tabloyu taşımayan başka bir GeoPackage: tablo yok (table_not_found); asıl dosya dönünce sürer",
        copy(p),
        step({"path": p, "table": "olay", "limit": 400}, done(first)),
        copy(p, OTHER_GPKG),
        step({"path": p, "table": "olay", "limit": 400, "cursor": cursor_of(first)},
             failed("table_not_found", "table")),
        copy(p),
        step({"path": p, "table": "olay", "limit": 400, "cursor": cursor_of(first)},
             done(key_answer(p, after=400, limit=400)))))

    # 7–10. Ordered questions: the source's order and the order here go on from their cursors.
    for n, field, descending, name in (
            (7, "puan", False, "kaynakta artan sıra (ondalık sütun)"),
            (8, "tarih", True, "kaynakta azalan sıra (tarih sütunu)"),
            (9, "acik", False, "kaynakta evet/hayır sırası"),
            (10, "ad", False, "burada doğal Türkçe sıra (metin sütunu)")):
        p = tmp(n)
        order = {"field": field, **({"descending": True} if descending else {})}
        fields = None if field == "puan" else ["ad", field] if field != "ad" else ["ad"]
        reads = COLUMNS if fields is None else fields
        first = ordered_answer(p, field, descending, limit=60, fields=fields, reads=reads)
        second = ordered_answer(p, field, descending, start=follow(first), limit=60, fields=fields, reads=reads)
        inp = {"path": p, "table": "olay", "limit": 60, "order": order}
        if fields is not None:
            inp["fields"] = fields
        out.append(case(
            f"{name}: dosya taşınınca sayfa reddedilir, geri gelince aynı imleçle sürer",
            copy(p),
            step(inp, done(first)),
            move(p, tmp(n, "uzakta.gpkg")),
            step({**inp, "cursor": first["next"]}, failed("file_not_found", "path")),
            move(tmp(n, "uzakta.gpkg"), p),
            step({**inp, "cursor": first["next"]}, done(second))))

    # 11. A filter sent to the source, counted on the first page; the second page after a lost one, not counted.
    p = tmp(11)
    neg = lambda r: sqc.lt(a(r, "sayi"), 0)  # noqa: E731
    pushed = so.num_push("sayi", "<", 0)
    first = key_answer(p, limit=200, count=True, keep=neg, pushed_keep=neg, pushed=pushed, exact=False,
                       local=["filter"], fields=["ad", "sayi"], reads=["ad", "sayi"])
    second = key_answer(p, after=first["rows"][-1]["key"], limit=200, keep=neg, pushed_keep=neg, pushed=pushed,
                        exact=False, local=["filter"], fields=["ad", "sayi"], reads=["ad", "sayi"])
    inp = {"path": p, "table": "olay", "limit": 200, "filter": "sayi < 0", "fields": ["ad", "sayi"]}
    out.append(case(
        "kaynağa giden süzgeçle: ilk sayfa sayılır; kaybolan ikinci sayfa geri gelince süzgeçle sürer",
        copy(p),
        step({**inp, "count": True}, done(first)),
        move(p, tmp(11, "uzakta.gpkg")),
        step({**inp, "cursor": first["next"]}, failed("file_not_found", "path")),
        move(tmp(11, "uzakta.gpkg"), p),
        step({**inp, "cursor": first["next"]}, done(second))))

    # 12. A filter checked here: parts of 1000 in key order until the page is full, again after a lost page.
    p = tmp(12)
    seven = lambda r: (a(r, "ad") or "").endswith(" 7")  # noqa: E731
    first = key_answer(p, limit=30, keep=seven, exact=False, local=["filter"], fields=["ad"], reads=["ad"])
    second = key_answer(p, after=first["rows"][-1]["key"], limit=30, keep=seven, exact=False, local=["filter"],
                        fields=["ad"], reads=["ad"])
    inp = {"path": p, "table": "olay", "limit": 30, "filter": "ad gibi '% 7'", "fields": ["ad"]}
    out.append(case(
        "burada denetlenen süzgeçle: 1000'lik parçalar; kaybolan sayfa geri gelince kaldığı yerden",
        copy(p),
        step(inp, done(first)),
        move(p, tmp(12, "uzakta.gpkg")),
        step({**inp, "cursor": first["next"]}, failed("file_not_found", "path")),
        move(tmp(12, "uzakta.gpkg"), p),
        step({**inp, "cursor": first["next"]}, done(second))))

    # 13. Statistics of the whole source: refused while the file is away, the same figures once it is back.
    p = tmp(13)
    inp = {"path": p, "table": "olay", "limit": 0, "count": True, "statistics": ["puan"], "fields": ["puan"]}
    out.append(case(
        "kaynağın istatistiği: dosya yokken reddedilir; dosya gelince aynı sayılar",
        copy(tmp(13, "uzakta.gpkg")),
        step(inp, failed("file_not_found", "path")),
        move(tmp(13, "uzakta.gpkg"), p),
        step(inp, done(key_answer(p, limit=0, count=True, statistics=["puan"], fields=["puan"],
                                  local=["statistics"])))))

    # 14. The cursor at the table's end: refused while the file is away; then an empty page without a next.
    p = tmp(14)
    inp = {"path": p, "table": "olay", "limit": 100, "cursor": f"k:{so.ROWS}"}
    end = key_answer(p, after=so.ROWS, limit=100)
    assert end["rows"] == [] and "next" not in end
    out.append(case(
        "tablonun sonundaki imleç: dosya yokken reddedilir; gelince boş sayfa, sonraki imleç yok",
        copy(tmp(14, "uzakta.gpkg")),
        step(inp, failed("file_not_found", "path")),
        move(tmp(14, "uzakta.gpkg"), p),
        step(inp, done(end))))

    # 15. A copy of the file elsewhere (the same bytes, a new time): the old cursor goes on there.
    p, q = tmp(15), tmp(15, "kopya/sira.gpkg")
    first = key_answer(p, limit=500)
    out.append(case(
        "dosyanın başka yerdeki kopyası (aynı bayt, yeni zaman): eski imleç kopyada sürer",
        copy(p),
        step({"path": p, "table": "olay", "limit": 500}, done(first)),
        copy(q),
        step({"path": q, "table": "olay", "limit": 500, "cursor": cursor_of(first)},
             done(key_answer(q, after=500, limit=500)))))

    # 16. A stale stamp and a missing file: the file is looked for first.
    p = tmp(16)
    inp = {"path": p, "table": "olay", "limit": 100, "cursor": "k:100", "expectedStamp": "0:0"}
    out.append(case(
        "damgası tutmayan soru dosya yokken: önce dosya yok denir; dosya gelince damga tutmaz (source_changed)",
        copy(tmp(16, "uzakta.gpkg")),
        step(inp, failed("file_not_found", "path")),
        move(tmp(16, "uzakta.gpkg"), p),
        step(inp, conflict("source_changed", "expectedStamp"))))

    # 17. A folder where the file was: not a file.
    p = tmp(17)
    out.append(case(
        "dosyanın yerinde bir klasör: dosya yolu değil (invalid_file_path); dosya gelince sürer",
        copy(tmp(17, "sira.gpkg/icinde.gpkg")),
        step({"path": p, "table": "olay", "limit": 50, "cursor": "k:50"}, failed("invalid_file_path", "path")),
        move(tmp(17, "sira.gpkg/icinde.gpkg"), tmp(17, "yan.gpkg")),
        move(tmp(17, "sira.gpkg"), tmp(17, "klasor")),
        move(tmp(17, "yan.gpkg"), p),
        step({"path": p, "table": "olay", "limit": 50, "cursor": "k:50"},
             done(key_answer(p, after=50, limit=50)))))

    # 18–20. A bound layer whose file moved: its question is refused at the layer; relinked to the new place, the
    # same question goes on from its cursor.
    p, q = tmp(18), tmp(18, "tasindi/sira.gpkg")
    first = key_answer(p, limit=300, layer=LAYER)
    out.append(case(
        "bağlı katmanın dosyası taşındı: soru katmanda reddedilir (file_not_found, layer); yeniden bağlanınca aynı "
        "imleçle sürer",
        copy(p), link(p),
        step({"layer": LAYER, "scope": "source", "limit": 300}, done(first)),
        move(p, q),
        step({"layer": LAYER, "scope": "source", "limit": 300, "cursor": cursor_of(first)},
             failed("file_not_found", "layer")),
        relink(q),
        step({"layer": LAYER, "scope": "source", "limit": 300, "cursor": cursor_of(first)},
             done(key_answer(q, after=300, limit=300, layer=LAYER)))))

    p = tmp(19)
    first = key_answer(p, limit=300, layer=LAYER)
    out.append(case(
        "bağlı katmanın dosyası geri geldi: yeniden bağlamadan aynı imleçle sürer",
        copy(p), link(p),
        step({"layer": LAYER, "scope": "source", "limit": 300}, done(first)),
        move(p, tmp(19, "uzakta.gpkg")),
        step({"layer": LAYER, "scope": "source", "limit": 300, "cursor": cursor_of(first)},
             failed("file_not_found", "layer")),
        move(tmp(19, "uzakta.gpkg"), p),
        step({"layer": LAYER, "scope": "source", "limit": 300, "cursor": cursor_of(first)},
             done(key_answer(p, after=300, limit=300, layer=LAYER)))))

    p = tmp(20)
    first = ordered_answer(p, "puan", True, limit=80, layer=LAYER)
    inp = {"layer": LAYER, "scope": "source", "limit": 80, "order": {"field": "puan", "descending": True}}
    second = ordered_answer(p, "puan", True, start=follow(first), limit=80, layer=LAYER)
    out.append(case(
        "bağlı katmanın kaynakta sıralı sorusu: dosya yerinde değilken reddedilir, gelince sıra sürer",
        copy(p), link(p),
        step(inp, done(first)),
        copy(tmp(20, "bozuk-yer.gpkg"), NOT_GPKG),
        move(p, tmp(20, "uzakta.gpkg")),
        move(tmp(20, "bozuk-yer.gpkg"), p),
        step({**inp, "cursor": first["next"]}, failed("import_unreadable", "layer")),
        move(tmp(20, "uzakta.gpkg"), p),
        step({**inp, "cursor": first["next"]}, done(second))))

    # 21. A bound layer's table renamed in its file: the table is not there.
    p = tmp(21)
    first = key_answer(p, limit=300, layer=LAYER)
    out.append(case(
        "bağlı katmanın dosyasında tablo yok: tablo yok (table_not_found); asıl dosya dönünce sürer",
        copy(p), link(p),
        step({"layer": LAYER, "scope": "source", "limit": 300}, done(first)),
        copy(p, OTHER_GPKG),
        step({"layer": LAYER, "scope": "source", "limit": 300, "cursor": cursor_of(first)},
             failed("table_not_found", "table")),
        copy(p),
        step({"layer": LAYER, "scope": "source", "limit": 300, "cursor": cursor_of(first)},
             done(key_answer(p, after=300, limit=300, layer=LAYER)))))

    # 22. Two lost pages one after the other, then the rest: the rows of the pages are the table's in key order.
    p = tmp(22)
    pages = [key_answer(p, after=k * 600 if k else None, limit=600) for k in range(4)]
    assert [r["key"] for pg in pages for r in pg["rows"]] == list(range(1, so.ROWS + 1))
    out.append(case(
        "üst üste iki kaybolan sayfa: her biri geri gelince sürer, sayfalar bütün tablo",
        copy(p),
        step({"path": p, "table": "olay", "limit": 600}, done(pages[0])),
        move(p, tmp(22, "a.gpkg")),
        step({"path": p, "table": "olay", "limit": 600, "cursor": pages[0]["next"]}, failed("file_not_found", "path")),
        move(tmp(22, "a.gpkg"), p),
        step({"path": p, "table": "olay", "limit": 600, "cursor": pages[0]["next"]}, done(pages[1])),
        copy(p, NOT_GPKG),
        step({"path": p, "table": "olay", "limit": 600, "cursor": pages[1]["next"]},
             failed("import_unreadable", "path")),
        copy(p),
        step({"path": p, "table": "olay", "limit": 600, "cursor": pages[1]["next"]}, done(pages[2])),
        step({"path": p, "table": "olay", "limit": 600, "cursor": pages[2]["next"]}, done(pages[3]))))
    return out


def build():
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.sources.query",
        "commandVersion": 1,
        "title": "Kaynağı sorgula: okunamayan kaynak ve kaldığı yerden süren sayfa",
        "note": "docs/adr/0740. Her durum fixtures/sources/v1/sira.gpkg'nin $tmp: kopyasında; moveFile dosyayı "
                "taşır (baytı ve zamanı kalır). Beklenenler scripts/fixtures/source_trouble_cases.py'den, ADR 0581 "
                "ve 0591'in kurallarıyla sqlite3'ün okuduğu tablodan.",
        "setup": so.setup(),
        "cases": cases(),
    }


def main():
    want = build()
    text = json.dumps(want, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        have = OUT.read_text() if OUT.exists() else ""
        if have != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{len(want['cases'])} durum güncel")
        return 0
    OUT.write_text(text)
    print(f"{len(want['cases'])} durum yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
