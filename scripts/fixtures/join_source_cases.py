#!/usr/bin/env python3
"""The shared cases of a join view's source table and summing columns (docs/adr/0632, plan/gis.md CBS-09).

    /usr/bin/python3 scripts/fixtures/join_source_cases.py --files   # GDAL writes fixtures/joins/v1/tapu.gpkg
    python3 scripts/fixtures/join_source_cases.py                    # writes fixtures/commands/v1/cad.joins.query.sources.json
    python3 scripts/fixtures/join_source_cases.py --check            # compares

No KentOS code. The file is written with GDAL's OGR: an attribute table `tapu` (the land registry's owners: parcel no,
owner, share) and a point table `kadastro`; one share column then holds a text (SQLite keeps it), as a GeoPackage may.
Its rows are read back here with Python's sqlite3 (source_query_cases.py's value rule: an integer its digits, a real
the shortest text, NULL no attribute) and a row's lasting id is UUIDv5 of `gpkg:<table>:<fid>` (ADR 0502).

The rules, from the ADR:

- A view's joined records are a layer's (`with`) or a source's table's (`source: {path, table}`), never both; a source
  view's `with` is empty. The table is read whole whenever the view is; its columns are the table's (only `fields`
  when given), its key a column or `$uid`; keys meet by ADR 0527's rule (join_view_cases.py). The source's cells are
  never written (the reason says so); the report gives the source's version (`sourceStamp`). A table or file that
  cannot be read, or a key column the table has not, breaks the view: said, no rows, no copy.
- Summing columns follow the joined fields, read only: for each record of the main layer the count of its joined
  records (`count`; with a field the count of its non-empty values), or the exact sum, least, greatest and mean
  (kentos.statistics/1, spatial_query_cases.py's `figures`) of the field's values; a record without a match counts
  0 and sums nothing. Every row of a record shows the same values; the totals count each record once.
- The shape: at most 20 columns, names holding and unique, a field for all but a count; a source's path and table
  written, no password in an address. `cad.joins.edit` checks a summing column's field on a joined layer; a source's
  columns are not read by the edit.
"""
import copy
import json
import sqlite3
import sys
import uuid
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import join_view_cases as jv  # noqa: E402
import source_query_cases as sqc  # noqa: E402
import spatial_query_cases as sq  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
FILE = ROOT / "fixtures/joins/v1/tapu.gpkg"
OUT = ROOT / "fixtures/commands/v1/cad.joins.query.sources.json"
FX = "$fixture:joins/v1/tapu.gpkg"
FIXED = "Kaynağın kaydı (“{}”): görünümden yazılmaz; tabloyu katman olarak bağlayıp orada düzenleyin."
SUMMED = "Toplama sütunu: birleşen kayıtlardan her okuyuşta hesaplanır."

TAPU = [
    (1, "101/5", "Ayşe", 0.5),
    (2, "101/5", "Ali", 0.25),
    (3, "101/5", "Can", "yok"),
    (4, "101/6", "Veli", 1.0),
    (5, "7", "Eda", 1.0),
    (6, "999/1", "Zeki", 0.5),
    (7, None, "Ferhat", 1.0),
    (8, "102/1", "Gül", None),
]


def write_files():
    from osgeo import ogr, osr
    ogr.UseExceptions()
    osr.UseExceptions()
    FILE.parent.mkdir(parents=True, exist_ok=True)
    if FILE.exists():
        FILE.unlink()
    ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(FILE))
    lyr = ds.CreateLayer("tapu", None, ogr.wkbNone, ["FID=fid"])
    for name, kind in (("parsel_no", ogr.OFTString), ("malik", ogr.OFTString), ("pay", ogr.OFTReal)):
        lyr.CreateField(ogr.FieldDefn(name, kind))
    for fid, no, owner, share in TAPU:
        f = ogr.Feature(lyr.GetLayerDefn())
        f.SetFID(fid)
        if no is not None:
            f.SetField("parsel_no", no)
        f.SetField("malik", owner)
        if isinstance(share, float):
            f.SetField("pay", share)
        lyr.CreateFeature(f)
    tm = osr.SpatialReference()
    tm.ImportFromEPSG(5256)
    pts = ds.CreateLayer("kadastro", tm, ogr.wkbPoint, ["FID=fid"])
    pts.CreateField(ogr.FieldDefn("no", ogr.OFTString))
    for fid, no in ((1, "101/5"), (2, "101/6")):
        f = ogr.Feature(pts.GetLayerDefn())
        f.SetFID(fid)
        f.SetField("no", no)
        f.SetGeometry(ogr.CreateGeometryFromWkt(f"POINT ({487000 + fid} 4420000)"))
        pts.CreateFeature(f)
    # A text in the REAL column (SQLite's affinity keeps it), through GDAL's own connection.
    ds.ExecuteSQL("UPDATE tapu SET pay = 'yok' WHERE fid = 3")
    ds = None
    print(f"{FILE.relative_to(ROOT)} yazıldı")


def source_rows(table, columns):
    c = sqlite3.connect(f"file:{FILE}?mode=ro", uri=True)
    out = []
    for r in c.execute(f'SELECT fid, {", ".join(chr(34) + k + chr(34) for k in columns)} FROM "{table}" ORDER BY fid'):
        attrs = {k: sqc.text(v) for k, v in zip(columns, r[1:]) if v is not None}
        out.append({"id": r[0], "uid": str(uuid.uuid5(sqc.NAMESPACE, f"gpkg:{table}:{r[0]}")), "attrs": attrs})
    c.close()
    return out


TAPU_COLUMNS = ["parsel_no", "malik", "pay"]


# ── The reading ─────────────────────────────────────────────────────────


def summed(aggregates, found, field_names):
    out = []
    for a in aggregates:
        field = field_names.get(a.get("field")) if "field" in a else None
        values = [r["attrs"].get(field) for r in found] if field else []
        if a["how"] == "count":
            out.append(str(len(found)) if "field" not in a
                       else str(sum(1 for v in values if v is not None and v.strip(jv.SPACES))))
        else:
            out.append(sq.figures(values)[a["how"]])
    return out


def read(v, uids, source=True, table="tapu", columns=None, entities=None):
    """The whole reading of a source view (or, `source` false, a layer view with summing columns)."""
    empty = {"records": 0, "matched": 0, "unmatched": 0, "nullKeys": 0, "withRecords": 0, "withUsed": 0, "rows": 0,
             "cardinality": ""}
    entities = entities or jv.ENTITIES
    lk = jv.key_on(v["layer"], v["key"])
    left_cols = jv.columns_of(v["layer"])
    aggregates = v.get("aggregates", [])
    if source:
        cols = columns if columns is not None else TAPU_COLUMNS
        rights = source_rows(table, cols)
        if v["withKey"] != "$uid" and v["withKey"] not in cols:
            return {"broken": "$any", "columns": [], "rows": [], "report": empty, "totals": []}
        rk = ("uid",) if v["withKey"] == "$uid" else ("attr", v["withKey"], False)
        shown = [c for c in cols if not v.get("fields") or c in v["fields"]]
        if v.get("fields"):
            shown = [c for c in v["fields"] if c in cols]
        right_cols = [(c, c) for c in shown]
        right_name = v["source"]["table"]
        fixed = FIXED.format(right_name)
        locked_right = False
        field_names = {c: c for c in cols}
        right_uid = {r["id"]: r["uid"] for r in rights}
    else:
        rights = [e for e in entities if e["layerId"] == v["with"]]
        rk = jv.key_on(v["with"], v["withKey"])
        right_cols = jv.columns_of(v["with"], v.get("fields", ()))
        right_name = jv.NAMES[v["with"]]
        fixed = None
        locked_right = False
        field_names = {f.get("id", f["name"]): f["name"] for f in (jv.LAYERS[v["with"]] or [])}
        right_uid = {r["id"]: uids[r["id"]] for r in rights}

    def column(name, side, attr, k, reason_fixed=None):
        if reason_fixed:
            reason = reason_fixed
        elif k[0] == "attr" and k[1] == attr:
            reason = jv.KEY_SAYS
        elif side == "right" and locked_right:
            reason = f"“{right_name}” katmanı kilitli."
        else:
            reason = None
        c = {"name": name, "side": side, "field": attr, "editable": reason is None}
        if reason:
            c["reason"] = reason
        return c

    columns_out = [column(n, "left", a, lk) for n, a in left_cols]
    left_names = {n for n, _ in left_cols}
    for n, a in right_cols:
        columns_out.append(column(f"{n} ({right_name})" if n in left_names else n, "right", a, rk, fixed))
    for a in aggregates:
        columns_out.append({"name": a["name"], "side": "aggregate", "field": a["name"], "editable": False,
                            "reason": SUMMED})

    def keys_right(r):
        if rk[0] == "uid":
            return [jv.join_key(right_uid[r["id"]])]
        val = r["attrs"].get(rk[1])
        j = jv.join_key(val) if val is not None else None
        return [j] if j else []

    by_key = {}
    for i, r in enumerate(rights):
        for k in keys_right(r):
            by_key.setdefault(k, []).append(i)
    rep = dict(empty, withRecords=len(rights))
    rows, used, fan, lefts_in, rights_in = [], {}, False, [], []

    def cells(e, cols):
        return [None if e is None else e["attrs"].get(a) for _, a in cols]

    for e in (x for x in entities if x["layerId"] == v["layer"]):
        rep["records"] += 1
        keys = jv.keys_of(lk, e, uids)
        if not keys:
            rep["nullKeys"] += 1
        found = sorted({i for k in keys for i in by_key.get(k, [])})
        u = uids[e["id"]]
        sums = summed(aggregates, [rights[i] for i in found], field_names)
        if not found:
            rep["unmatched"] += 1
            if v.get("keep") == "matched":
                continue
            rows.append({"id": f"{u}/", "left": u, "cells": cells(e, left_cols) + cells(None, right_cols) + sums})
            lefts_in.append(e)
            continue
        rep["matched"] += 1
        fan = fan or len(found) > 1
        lefts_in.append(e)
        for i in found:
            used[i] = used.get(i, 0) + 1
            if i not in rights_in:
                rights_in.append(i)
            r = rights[i]
            rid = right_uid[r["id"]]
            rows.append({"id": f"{u}/{rid}", "left": u, "right": rid,
                         "cells": cells(e, left_cols) + cells(r, right_cols) + sums})
    rep["withUsed"] = len(used)
    rep["rows"] = len(rows)
    shared = any(n > 1 for n in used.values())
    rep["cardinality"] = "" if not used else {(False, False): "1:1", (True, False): "1:N", (False, True): "N:1",
                                              (True, True): "N:N"}[(fan, shared)]
    if source:
        rep["sourceStamp"] = "$any"
    totals = []
    for k, c in enumerate(columns_out):
        if c["side"] == "aggregate":
            seen, vals = set(), []
            for row in rows:
                if row["left"] not in seen:
                    seen.add(row["left"])
                    vals.append(row["cells"][k])
        else:
            recs = lefts_in if c["side"] == "left" else [rights[i] for i in sorted(rights_in)]
            vals = [r["attrs"].get(c["field"]) for r in recs]
        nums, skipped = [], 0
        for val in vals:
            if val is None or not val.strip(jv.SPACES):
                continue
            n = jv.number(val)
            if n is None:
                skipped += 1
            else:
                nums.append(n)
        if nums:
            top = max(s for _, s in nums)
            total = sum(m * 10 ** (top - s) for m, s in nums)
            t = {"column": k, "sum": jv.dec_text(total, top), "count": len(nums)}
            if skipped:
                t["skipped"] = skipped
            totals.append(t)
    return {"broken": None, "columns": columns_out, "rows": rows, "report": rep, "totals": totals}


# ── The cases ────────────────────────────────────────────────────────────


def answer(v, limit=100, cursor=0, **kw):
    r = read(v, jv.uids_named(), **kw)
    rows = r["rows"][cursor:cursor + limit]
    out = {"name": v["name"]}
    if r["broken"]:
        out["broken"] = "$any"
    out.update({"columns": r["columns"], "rows": rows})
    end = min(cursor + limit, len(r["rows"]))
    if end < len(r["rows"]):
        out["next"] = str(end)
    out["report"] = r["report"]
    if r["totals"]:
        out["totals"] = r["totals"]
    out["revision"] = "$current"
    return out


def broken_answer(name):
    return {"name": name, "broken": "$any", "columns": [], "rows": [],
            "report": {"records": 0, "matched": 0, "unmatched": 0, "nullKeys": 0, "withRecords": 0, "withUsed": 0,
                       "rows": 0, "cardinality": ""}, "revision": "$current"}


def src_view(id_, name, key="no", with_key="parsel_no", table="tapu", path=FX, keep=None, fields=None, aggregates=None):
    v = {"id": id_, "name": name, "layer": "parsel", "key": key, "with": "", "withKey": with_key,
         "source": {"path": path, "table": table}}
    if keep:
        v["keep"] = keep
    if fields:
        v["fields"] = fields
    if aggregates:
        v["aggregates"] = aggregates
    return v


def agg(name, how, field=None):
    a = {"name": name, "how": how}
    if field is not None:
        a["field"] = field
    return a


ALL_SUMS = [agg("Kayıt", "count"), agg("Paylı kayıt", "count", "pay"), agg("Pay toplamı", "sum", "pay"),
            agg("En küçük pay", "min", "pay"), agg("En büyük pay", "max", "pay"), agg("Ortalama pay", "mean", "pay")]
READ = jv.READ
WROTE = {"ids": jv.IDS, "canUndo": False, "canRedo": False, "revision": "changed", "dirty": True}
# A reading after a written view: the revision the writing left, the drawing not saved.
AFTER = {"ids": jv.IDS, "canUndo": False, "canRedo": False, "revision": "same", "dirty": True}
SETUP = jv.document([jv.PARSEL_MALIK])


def q(name, input_, result, setup=None, before=(), expect=None):
    return {"name": name, "setup": setup or SETUP,
            "steps": jv.CAPTURES + list(before) + [{"op": "execute", "input": jv.escaped(input_), "result": result,
                                                    "expect": expect or READ}]}


def ok(output):
    return jv.completed(output)


def refused(code, path):
    return jv.failed(code, path)


def edit_step(edit, result):
    return {"op": "execute", "command": "cad.joins.edit", "input": jv.escaped({"edit": edit}), "result": result,
            "expect": WROTE if result["status"] == "completed" else jv.NOTHING}


def edited():
    return ok({"views": "$any", "changed": True, "revision": "$current"})


def cases():
    src = src_view("parsel-tapu", "Parseller ve tapu kayıtları")
    matched = src_view("tapulu", "Tapulu parseller", keep="matched")
    owners = src_view("malikler", "Tapu malikleri", fields=["malik"])
    sums = src_view("tapu-ozet", "Tapu özeti", aggregates=ALL_SUMS)
    sums_only = src_view("tapu-sayi", "Tapu sayısı", keep="matched", fields=["malik"],
                         aggregates=[agg("Kayıt", "count"), agg("Pay", "sum", "pay")])
    layer_sums = dict(jv.PARSEL_MALIK, id="malik-ozet", name="Malik özeti",
                      aggregates=[agg("Malik sayısı", "count"), agg("Pay toplamı", "sum", "pay"),
                                  agg("Ortalama pay", "mean", "pay")])
    no_field = dict(jv.PARSEL_MALIK, id="alansiz", name="Alansız",
                    aggregates=[agg("Kat", "sum", "kat"), agg("Kat sayısı", "count", "kat")])
    out = [
        q("kaynak tablosu: üç tapu kaydı olan parsel üç satır, 007 ile 7 eşleşir, anahtarsız parsel boş hücreli; kaynağın hücreleri yazılmaz",
          {"definition": src}, ok(answer(src))),
        q("kaynak tablosu, yalnız eşleşenler", {"definition": matched}, ok(answer(matched))),
        q("kaynak tablosunun yalnız seçilen sütunu", {"definition": owners}, ok(answer(owners))),
        q("toplama sütunları: sayı, değerli kayıt, kesin toplam, en küçük, en büyük ve ortalama; sayı olmayan pay atlanır, eşleşmeyen 0 sayar",
          {"definition": sums}, ok(answer(sums))),
        q("toplama sütunları yalnız eşleşenlerde ve seçilen sütunla", {"definition": sums_only}, ok(answer(sums_only))),
        q("katmanla birleşen görünümde toplama sütunları: malik sayısı, pay toplamı, ortalama",
          {"definition": layer_sums}, ok(answer(layer_sums, source=False))),
        q("birleşen katmanda olmayan alanın toplamı boş, sayısı 0",
          {"definition": no_field}, ok(answer(no_field, source=False))),
        q("kaynak tablosu sayfa sayfa: ilk iki satır ve imleç",
          {"definition": src, "limit": 2}, ok(answer(src, limit=2))),
        q("kaynak tablosu: imleçten sonraki satırlar", {"definition": src, "limit": 3, "cursor": "2"},
          ok(answer(src, limit=3, cursor=2))),
        q("kaynağın anahtar sütunu yok: kopuk, satır yok",
          {"definition": src_view("yanlis", "Yanlış anahtar", with_key="ada")},
          ok(answer(src_view("yanlis", "Yanlış anahtar", with_key="ada")))),
        q("kaynağın tablosu yok: kopuk, eski okuma gösterilmez",
          {"definition": src_view("tablosuz", "Tablosuz", table="yok")},
          ok(broken_answer("Tablosuz"))),
        q("kaynak dosyası yok: kopuk",
          {"definition": src_view("dosyasiz", "Dosyasız", path="$fixture:joins/v1/olmayan.gpkg")},
          ok(broken_answer("Dosyasız"))),
        q("hem katman hem kaynak: ret", {"definition": dict(src, **{"with": "malik"})},
          refused("invalid_join_view", "definition")),
        q("kaynağın tablosu yazılmamış: ret", {"definition": src_view("x", "X", table=" ")},
          refused("invalid_join_view", "definition")),
        q("adreste parola: ret",
          {"definition": src_view("x", "X", path="postgresql://kentos:gizli@localhost/kent")},
          refused("invalid_join_view", "definition")),
        q("alansız toplam: ret", {"definition": src_view("x", "X", aggregates=[agg("T", "sum")])},
          refused("invalid_join_view", "definition")),
        q("aynı adlı iki toplama sütunu: ret",
          {"definition": src_view("x", "X", aggregates=[agg("T", "count"), agg("T", "sum", "pay")])},
          refused("invalid_join_view", "definition")),
        q("21 toplama sütunu: ret",
          {"definition": src_view("x", "X", aggregates=[agg(f"S{i}", "count") for i in range(21)])},
          refused("invalid_join_view", "definition")),
        q("kayıtlı kaynaklı görünüm: düzenlemeyle kurulur, adıyla okunur",
          {"view": "tapu-ozet"}, ok(answer(sums)), before=[edit_step({"kind": "create", "view": sums}, edited())],
          expect=AFTER),
        q("katman görünümüne toplama sütunu eklenir ve okunur",
          {"view": "parsel-malik"}, ok(answer(dict(layer_sums, id="parsel-malik"), source=False)),
          before=[edit_step({"kind": "update", "view": dict(layer_sums, id="parsel-malik")}, edited())],
          expect=AFTER),
        q("düzenleme: katmanda olmayan alanın toplama sütunu reddedilir",
          {"view": "parsel-malik"}, ok(answer(jv.PARSEL_MALIK, source=False)),
          before=[edit_step({"kind": "create", "view": dict(no_field, id="yeni")},
                            refused("field_not_found", "edit.view.aggregates.0.field"))]),
        q("düzenleme: kaynaklı görünümün sütunları okunmaz; görünüm yazılır, okununca kopuk",
          {"view": "tablosuz"},
          ok(broken_answer("Tablosuz")),
          before=[edit_step({"kind": "create", "view": src_view("tablosuz", "Tablosuz", table="yok")}, edited())],
          expect=AFTER),
        q("kaynaklı görünüm silinir", {"view": "parsel-malik"}, ok(answer(jv.PARSEL_MALIK, source=False)),
          before=[edit_step({"kind": "create", "view": src}, edited()),
                  edit_step({"kind": "remove", "id": "parsel-tapu"}, edited())], expect=AFTER),
    ]
    return out


def fix(cases_):
    return cases_


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.joins.query", "commandVersion": 1,
            "title": "Birleşik görünümde kaynak tablosu ve toplama sütunları",
            "note": "ADR 0632 §1–§2: birleşen kayıtlar bir GeoPackage tablosundan (fixtures/joins/v1/tapu.gpkg, GDAL'ın; "
                    "satırlar sqlite3'le okunur, kalıcı kimlik gpkg:<tablo>:<fid>'in UUIDv5'i) ve kayıt başına toplama "
                    "sütunları (kentos.statistics/1). Kural ve beklenenler scripts/fixtures/join_source_cases.py'de KentOS "
                    "kodu olmadan yazılmıştır; kopukluğun ve retlerin cümleleri ile kaynağın sürümü çekirdeğindir ($any).",
            "setup": SETUP, "cases": fix(cases())}


def main():
    if "--files" in sys.argv[1:]:
        write_files()
        return 0
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"kaynaklı birleşik görünüm durumları tutarlı: {len(json.loads(text)['cases'])} durum")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(json.loads(text)['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
