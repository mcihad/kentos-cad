#!/usr/bin/env python3
"""The shared cases of `cad.joins.edit` and `cad.joins.query` (docs/adr/0527), written without KentOS code.

    python3 scripts/fixtures/join_view_cases.py           # writes fixtures/commands/v1/cad.joins.{edit,query}.json
    python3 scripts/fixtures/join_view_cases.py --check   # writes nothing; compares

The rules, from the ADR:

- A view joins the main layer's records (`layer`, `key`) with the joined layer's (`with`, `withKey`). A key is a typed
  field's key, `$uid` (the record's persistent id) or, on a layer without typed fields, an attribute's name. Keys meet
  by Anahtarla birleştir's rule: the spaces round them dropped; a number (sign, digits, at most one `.` or `,`, at
  most 30 digits) by its canonical text, so `007` meets `7`; an empty value is no key. A multiple text field's codes
  (split at `;`) are each a key.
- Rows: the main layer's records in document order; each with its matches in document order, a row each, its id
  `left/right` of the persistent ids; a record without a match one row `left/` (none when `keep` is `matched`).
- Columns: the main layer's typed fields in order (heading the alias, else the name), then the joined layer's (only
  `fields` when given); a layer without typed fields its records' attribute names sorted. A joined heading the main
  layer has too gets “ (layer name)”. A key column cannot be written (“Birleştirmenin anahtarı; değişince satır başka
  kayda bağlanır.”); a locked layer's columns neither (““Ad” katmanı kilitli.”).
- Report: records, matched, unmatched, records without a key, the joined layer's records and those met, rows, and the
  cardinality: 1:1, 1:N (a record met several), N:1 (a joined record met by several), N:N, empty without a match.
- Totals: a column with a number among its records' values (each record of the rows once): the exact sum at the
  values' largest scale, how many numbers, how many values that are not.
- A view whose layer is gone, a group, or whose key a typed layer has not is broken: said, no rows.
- Pages: 1–1000 rows (100 when not said); the cursor the decimal offset a former answer gave.
- The edit's checks, in order: the view's shape; the revision; the id (unknown, taken) and the count (50); the
  relation of `fromRelation`; the layers (found, a layer), the keys and the fields (`$uid` always; a layer without
  typed fields takes any name).
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
SPACES = " \t\n\r\x0b\x0c                 　﻿"
KEY_SAYS = "Birleştirmenin anahtarı; değişince satır başka kayda bağlanır."


# ── The number and key rules ────────────────────────────────────────────


def number(text):
    """(mantissa, scale) of a decimal number, or None."""
    t = text.strip(SPACES)
    neg = t.startswith("-")
    body = t[1:] if neg else (t[1:] if t.startswith("+") else t)
    seps = [i for i, c in enumerate(body) if c in ".,"]
    if len(seps) > 1:
        return None
    if seps:
        whole, frac = body[: seps[0]], body[seps[0] + 1:]
    else:
        whole, frac = body, ""
    if not whole and not frac:
        return None
    if not all(c in "0123456789" for c in whole + frac):
        return None
    whole = whole.lstrip("0")
    if len(whole) + len(frac) > 30:
        return None
    m = int(whole + frac or "0")
    return (-m if neg else m, len(frac))


def dec_text(m, scale):
    neg = m < 0
    digits = str(abs(m))
    if scale == 0:
        body = digits
    elif len(digits) > scale:
        body = digits[:-scale] + "." + digits[-scale:]
    else:
        body = "0." + "0" * (scale - len(digits)) + digits
    return "-" + body if neg else body


def join_key(text):
    t = text.strip(SPACES)
    if not t:
        return None
    n = number(t)
    if n is None:
        return "$" + t
    s = dec_text(*n)
    if "." in s:
        s = s.rstrip("0").rstrip(".")
        if s == "-0":
            s = "0"
    return "#" + s


# ── The drawing ─────────────────────────────────────────────────────────


def layer(id_, name, fields=None, locked=False, kind="layer", children=None):
    node = {"id": id_, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": children or []}
    if fields:
        node["fields"] = fields
    return node


def field(name, kind="text", alias=None, multiple=False):
    f = {"name": name, "kind": kind}
    if alias:
        f["alias"] = alias
    if multiple:
        f["multiple"] = True
    return f


def point(id_, layer_id, attrs, x):
    return {"kind": "point", "id": id_, "layerId": layer_id, "attrs": attrs, "p": {"x": 487000 + x, "y": 4420000}}


PARSEL_FIELDS = [field("no", alias="Parsel no"), field("ada"), field("ad"), field("alan", "decimal")]
MALIK_FIELDS = [field("ad"), field("parsel"), field("pay", "decimal"), field("parselUid")]
HISSE_FIELDS = [field("parseller", multiple=True), field("oran", "decimal")]

ENTITIES = [
    point(1, "parsel", {"no": "101/5", "ada": "101", "ad": "Çayır", "alan": "500"}, 1),
    point(2, "parsel", {"no": "101/6", "ada": "101", "ad": "Bağ", "alan": "300.5"}, 2),
    point(3, "parsel", {"no": "102/1", "ada": "102", "alan": "1200"}, 3),
    point(4, "parsel", {"no": " 007", "alan": "80"}, 4),
    point(5, "parsel", {"ada": "103"}, 5),
    point(6, "malik", {"ad": "Ayşe", "parsel": "101/5", "pay": "0.5"}, 6),
    point(7, "malik", {"ad": "Ali", "parsel": "101/5", "pay": "0.5"}, 7),
    point(8, "malik", {"ad": "Veli", "parsel": "101/6", "pay": "1"}, 8),
    point(9, "malik", {"ad": "Can", "parsel": "7", "pay": "1"}, 9),
    point(10, "malik", {"ad": "Eda", "parsel": "999/1", "pay": "x"}, 10),
    point(11, "hisse", {"parseller": "101/5; 101/6", "oran": "0.25"}, 11),
    point(12, "hisse", {"parseller": "101/5", "oran": "0.75"}, 12),
    point(13, "kayit", {"Parsel": "101/5", "Not": "tapu"}, 13),
    point(14, "kayit", {"Parsel": "102/1"}, 14),
]


def document(views=(), locked_malik=False, relations=()):
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"}
    if views:
        settings["joinViews"] = list(views)
    if relations:
        settings["relations"] = list(relations)
    return {
        "format": "kentos.document", "version": 1, "name": "Birleşik görünümler", "settings": settings,
        "origin": {"x": 487000, "y": 4420000},
        "layers": [
            layer("parsel", "Parsel", PARSEL_FIELDS),
            layer("malik", "Malik", MALIK_FIELDS, locked=locked_malik),
            layer("hisse", "Hisse", HISSE_FIELDS),
            layer("kayit", "Kayıt"),
            layer("bos", "Boş", [field("no")]),
            layer("grup", "Kadastro", kind="group", children=[layer("ic", "İç", [field("no")])]),
        ],
        "activeLayer": "parsel", "entities": copy.deepcopy(ENTITIES), "styles": {"items": [], "categories": []},
    }


IDS = [e["id"] for e in ENTITIES]
LAYERS = {"parsel": PARSEL_FIELDS, "malik": MALIK_FIELDS, "hisse": HISSE_FIELDS, "kayit": None, "bos": [field("no")],
          "ic": [field("no")]}
NAMES = {"parsel": "Parsel", "malik": "Malik", "hisse": "Hisse", "kayit": "Kayıt", "bos": "Boş", "ic": "İç",
         "grup": "Kadastro"}


def view(id_, name, layer_id, key, with_id, with_key, keep=None, fields=None):
    v = {"id": id_, "name": name, "layer": layer_id, "key": key, "with": with_id, "withKey": with_key}
    if keep:
        v["keep"] = keep
    if fields:
        v["fields"] = fields
    return v


PARSEL_MALIK = view("parsel-malik", "Parseller ve malikleri", "parsel", "no", "malik", "parsel")


# ── The reading ─────────────────────────────────────────────────────────


def key_on(layer_id, key):
    fields = LAYERS.get(layer_id, "missing")
    if fields == "missing":
        return "missing"
    if key == "$uid":
        return ("uid",)
    if not fields:
        return ("attr", key, False)
    for f in fields:
        if f.get("id", f["name"]) == key:
            return ("attr", f["name"], bool(f.get("multiple")) and f["kind"] == "text")
    return None


def keys_of(k, e, uids):
    if k[0] == "uid":
        return [join_key(uids[e["id"]])]
    v = e["attrs"].get(k[1])
    if v is None:
        return []
    if k[2]:
        out = []
        for code in v.split(";"):
            j = join_key(code)
            if j and j not in out:
                out.append(j)
        return out
    j = join_key(v)
    return [j] if j else []


def columns_of(layer_id, only=()):
    fields = LAYERS[layer_id]
    if fields:
        pick = [f for f in fields] if not only else [f for k in only for f in fields if f.get("id", f["name"]) == k]
        return [(f.get("alias") or f["name"], f["name"]) for f in pick]
    names = sorted({k for e in ENTITIES if e["layerId"] == layer_id for k in e["attrs"]})
    if only:
        names = [n for n in names if n in only]
    return [(n, n) for n in names]


def broken_reason(v):
    for lid, key, what in ((v["layer"], v["key"], "katmanı"), (v["with"], v["withKey"], "birleşen katmanı")):
        if lid not in LAYERS:
            return f"Görünümün {what} “{lid}” çizimde yok ya da bir grup; görünüm okunamıyor."
        if key_on(lid, key) is None:
            return f"“{NAMES[lid]}” katmanında görünümün anahtarı “{key}” alanı yok; görünüm okunamıyor."
    return None


def read(v, entities, uids, locked=()):
    """The whole reading: columns, rows, report, totals, broken."""
    why = broken_reason(v)
    empty = {"records": 0, "matched": 0, "unmatched": 0, "nullKeys": 0, "withRecords": 0, "withUsed": 0, "rows": 0,
             "cardinality": ""}
    if why:
        return {"broken": why, "columns": [], "rows": [], "report": empty, "totals": []}
    lk, rk = key_on(v["layer"], v["key"]), key_on(v["with"], v["withKey"])
    left_cols, right_cols = columns_of(v["layer"]), columns_of(v["with"], v.get("fields", ()))

    def column(name, side, attr, k, lid):
        if k[0] == "attr" and k[1] == attr:
            reason = KEY_SAYS
        elif lid in locked:
            reason = f"“{NAMES[lid]}” katmanı kilitli."
        else:
            reason = None
        c = {"name": name, "side": side, "field": attr, "editable": reason is None}
        if reason:
            c["reason"] = reason
        return c

    columns = [column(n, "left", a, lk, v["layer"]) for n, a in left_cols]
    left_names = {n for n, _ in left_cols}
    for n, a in right_cols:
        columns.append(column(f"{n} ({NAMES[v['with']]})" if n in left_names else n, "right", a, rk, v["with"]))
    rights = [e for e in entities if e["layerId"] == v["with"]]
    by_key = {}
    for i, e in enumerate(rights):
        for k in keys_of(rk, e, uids):
            by_key.setdefault(k, []).append(i)
    rep = dict(empty, withRecords=len(rights))
    rows, used, fan, lefts_in, rights_in = [], {}, False, [], []

    def cells(e, cols):
        return [None if e is None else e["attrs"].get(a) for _, a in cols]

    for e in (x for x in entities if x["layerId"] == v["layer"]):
        rep["records"] += 1
        keys = keys_of(lk, e, uids)
        if not keys:
            rep["nullKeys"] += 1
        found = sorted({i for k in keys for i in by_key.get(k, [])})
        u = uids[e["id"]]
        if not found:
            rep["unmatched"] += 1
            if v.get("keep") == "matched":
                continue
            rows.append({"id": f"{u}/", "left": u, "cells": cells(e, left_cols) + cells(None, right_cols)})
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
            rows.append({"id": f"{u}/{uids[r['id']]}", "left": u, "right": uids[r["id"]],
                         "cells": cells(e, left_cols) + cells(r, right_cols)})
    rep["withUsed"] = len(used)
    rep["rows"] = len(rows)
    shared = any(n > 1 for n in used.values())
    rep["cardinality"] = "" if not used else {(False, False): "1:1", (True, False): "1:N", (False, True): "N:1",
                                              (True, True): "N:N"}[(fan, shared)]
    totals = []
    for k, c in enumerate(columns):
        recs = lefts_in if c["side"] == "left" else [rights[i] for i in sorted(rights_in)]
        nums, skipped = [], 0
        for r in recs:
            val = r["attrs"].get(c["field"])
            if val is None or not val.strip(SPACES):
                continue
            n = number(val)
            if n is None:
                skipped += 1
            else:
                nums.append(n)
        if nums:
            top = max(s for _, s in nums)
            total = sum(m * 10 ** (top - s) for m, s in nums)
            t = {"column": k, "sum": dec_text(total, top), "count": len(nums)}
            if skipped:
                t["skipped"] = skipped
            totals.append(t)
    return {"broken": None, "columns": columns, "rows": rows, "report": rep, "totals": totals}


# ── The cases ────────────────────────────────────────────────────────────


def uids_named():
    return {i: f"$uid:o{i}" for i in IDS}


CAPTURES = [{"op": "captureUid", "id": i, "as": f"o{i}"} for i in IDS]


def query_answer(v, name, entities=None, locked=(), limit=100, cursor=0):
    r = read(v, entities or ENTITIES, uids_named(), locked)
    rows = r["rows"][cursor:cursor + limit]
    out = {"name": name}
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


def completed(output):
    return {"status": "completed", "output": output, "warnings": []}


def failed(code, path, status="failed"):
    err = {"code": code, "message": "$any", "path": path}
    if code == "revision_conflict":
        err["revision"] = "$current"
    return {"status": status, "error": err}


READ = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same", "dirty": False}


def q_case(name, input_, result, setup=None, before=()):
    return {"name": name, "setup": setup or document([PARSEL_MALIK]),
            "steps": CAPTURES + list(before) + [{"op": "execute", "input": input_, "result": result, "expect": READ}]}


def query_cases():
    pm = PARSEL_MALIK
    matched = view("eslesen", "Malikli parseller", "parsel", "no", "malik", "parsel", keep="matched")
    some = view("adlar", "Malik adları", "parsel", "no", "malik", "parsel", fields=["ad", "pay"])
    owners = view("malik-parsel", "Maliklerin parselleri", "malik", "parsel", "parsel", "no")
    shares = view("hisseler", "Parsel hisseleri", "parsel", "no", "hisse", "parseller")
    records = view("kayitlar", "Parsel kayıtları", "parsel", "no", "kayit", "Parsel")
    tree = view("ada", "Aynı adadakiler", "parsel", "ada", "parsel", "ada")
    empty = view("bos", "Boş katmanla", "parsel", "no", "bos", "no")
    gone = view("eski", "Eski katmanla", "parsel", "no", "eski", "no")
    nokey = view("anahtarsiz", "Anahtarsız", "parsel", "numara", "malik", "parsel")
    group = view("grupla", "Grupla", "parsel", "no", "grup", "no")
    by_uid = view("uid", "Kimlikle", "parsel", "$uid", "malik", "parselUid")
    out = [
        q_case("kayıtlı görünüm: iki maliki olan parsel iki satır, eşleşmeyen ve anahtarsız parsel boş hücreli; 007 ile 7 eşleşir, kardinalite 1:N",
               {"view": "parsel-malik"}, completed(query_answer(pm, pm["name"]))),
        q_case("yalnız eşleşenler: eşleşmeyen ve anahtarsız parsel satır vermez, rapor yine sayar",
               {"view": "eslesen"}, completed(query_answer(matched, matched["name"])), setup=document([matched])),
        q_case("birleşen katmanın yalnız seçilen alanları, ana katmanda da olan başlık katmanın adıyla",
               {"view": "adlar"}, completed(query_answer(some, some["name"])), setup=document([some])),
        q_case("ters yön: her malik bir parselle, iki malik aynı parselle; kardinalite N:1",
               {"definition": owners}, completed(query_answer(owners, owners["name"]))),
        q_case("çok değerli alanın her kodu bir anahtar: hisse iki parselde; kardinalite N:N",
               {"definition": shares}, completed(query_answer(shares, shares["name"]))),
        q_case("alanı olmayan katman: sütunlar kayıtların öznitelik adları, sıralı",
               {"definition": records}, completed(query_answer(records, records["name"]))),
        q_case("katman kendisiyle: aynı adadaki parseller",
               {"definition": tree}, completed(query_answer(tree, tree["name"]))),
        q_case("kaydı olmayan katmanla: bütün parseller eşleşmeyen, kardinalite boş",
               {"definition": empty}, completed(query_answer(empty, empty["name"]))),
        q_case("tanım önizlemesi kayıtlıyla aynı satırları verir",
               {"definition": pm}, completed(query_answer(pm, pm["name"]))),
        q_case("sayfa: ilk iki satır ve sonraki sayfanın imleci",
               {"view": "parsel-malik", "limit": 2}, completed(query_answer(pm, pm["name"], limit=2))),
        q_case("sayfa: imleçten iki satır",
               {"view": "parsel-malik", "limit": 2, "cursor": "2"}, completed(query_answer(pm, pm["name"], limit=2, cursor=2))),
        q_case("son sayfa: sonraki imleç yok",
               {"view": "parsel-malik", "limit": 4, "cursor": "4"}, completed(query_answer(pm, pm["name"], limit=4, cursor=4))),
        q_case("kilitli birleşen katmanın sütunları yazılamaz, nedeniyle",
               {"view": "parsel-malik"}, completed(query_answer(pm, pm["name"], locked=("malik",))),
               setup=document([pm], locked_malik=True)),
        q_case("kayıtlı görünümün katmanı silinmiş: kopuk, satır yok",
               {"view": "eski"}, completed(query_answer(gone, gone["name"])), setup=document([gone])),
        q_case("anahtar alanı ana katmanda yok: kopuk",
               {"definition": nokey}, completed(query_answer(nokey, nokey["name"]))),
        q_case("birleşen düğüm bir grup: kopuk",
               {"definition": group}, completed(query_answer(group, group["name"]))),
        q_case("görünüm de tanım da verilmemiş: ret", {}, failed("invalid_join_view", "view")),
        q_case("görünüm ve tanım birlikte: ret", {"view": "parsel-malik", "definition": pm}, failed("invalid_join_view", "view")),
        q_case("olmayan görünüm: ret", {"view": "yok"}, failed("unknown_join_view", "view")),
        q_case("kurala uymayan tanım: ret", {"definition": dict(pm, id="bir görünüm")}, failed("invalid_join_view", "definition")),
        q_case("sayfa 0 satır: ret", {"view": "parsel-malik", "limit": 0}, failed("invalid_count", "limit")),
        q_case("sayfa 1001 satır: ret", {"view": "parsel-malik", "limit": 1001}, failed("invalid_count", "limit")),
        q_case("imleç sayı değil: ret", {"view": "parsel-malik", "cursor": "x"}, failed("invalid_cursor", "cursor")),
        q_case("imleç başında sıfırlı: ret", {"view": "parsel-malik", "cursor": "02"}, failed("invalid_cursor", "cursor")),
        q_case("imleç satırların ötesinde: ret", {"view": "parsel-malik", "cursor": "99"}, failed("invalid_cursor", "cursor")),
    ]
    # Through the persistent ids (a relation's link): the owners' field holds the parcels' ids.
    linked = copy.deepcopy(ENTITIES)
    sets = []
    for owner, parcel in ((6, 1), (7, 1), (8, 2)):
        next(e for e in linked if e["id"] == owner)["attrs"]["parselUid"] = f"$uid:o{parcel}"
        sets.append({"op": "execute", "command": "cad.entities.set",
                     "input": {"uids": [f"$uidOf:{owner}"], "operation": "attributes",
                               "attrs": {**next(e for e in ENTITIES if e["id"] == owner)["attrs"], "parselUid": f"$uidOf:{parcel}"}},
                     "result": completed({"changed": [f"$uidOf:{owner}"], "ids": [owner], "revision": "$current"})})
    after = {"ids": IDS, "canUndo": True, "canRedo": False, "revision": "same", "dirty": True}
    out.append({"name": "kalıcı kimlikle: maliklerin alanı parselin kimliğini tutar",
                "setup": document([by_uid]),
                "steps": CAPTURES + sets + [{"op": "execute", "input": {"view": "uid"},
                                             "result": completed(query_answer(by_uid, by_uid["name"], entities=linked)),
                                             "expect": after}]})
    return out


def views_after(views):
    return list(views)


def escaped(v):
    """A step's `$uid` key as the text itself (`$$…`: the runner reads `$name` as a placeholder)."""
    if isinstance(v, str):
        return "$$uid" if v == "$uid" else v
    if isinstance(v, list):
        return [escaped(x) for x in v]
    if isinstance(v, dict):
        return {k: escaped(x) for k, x in v.items()}
    return v


def e_case(name, steps, setup=None):
    return {"name": name, "setup": setup or document([PARSEL_MALIK], relations=[RELATION]), "steps": escaped(steps)}


RELATION = {"id": "parsel-malik-iliski", "name": "Parselin malikleri", "parent": "parsel", "child": "malik", "field": "parselUid"}
WROTE = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "changed", "dirty": True}
NOTHING = {"ids": IDS, "canUndo": False, "canRedo": False, "revision": "same", "dirty": False}


def edit_ok(name, edit, views, op="execute"):
    out = {"views": views, "changed": True, "revision": "$current"}
    return e_case(name, [{"op": op, "input": {"edit": edit}, "result": completed(out),
                          "expect": WROTE if op == "execute" else NOTHING}])


def edit_refused(name, input_, code, path, setup=None, status="failed"):
    return e_case(name, [{"op": "execute", "input": input_, "result": failed(code, path, status), "expect": NOTHING}], setup)


def edit_cases():
    pm = PARSEL_MALIK
    owners = view("malik-parsel", "Maliklerin parselleri", "malik", "parsel", "parsel", "no", keep="matched",
                  fields=["no", "alan"])
    uid = view("uid", "Kimlikle", "parsel", "$uid", "malik", "parselUid")
    loose = view("kayitlar", "Parsel kayıtları", "parsel", "no", "kayit", "Belge", fields=["Herhangi"])
    rel = {"id": "iliskiden", "name": "Parselin malikleri", "layer": "parsel", "key": "$uid", "with": "malik",
           "withKey": "parselUid"}
    fifty = [view(f"v{i}", f"Görünüm {i}", "parsel", "no", "malik", "parsel") for i in range(50)]
    cases = [
        edit_ok("görünüm oluşturulur; varsayılan keep yazılmaz", {"kind": "create", "view": owners}, [pm, owners]),
        edit_ok("kalıcı kimlik anahtarıyla görünüm", {"kind": "create", "view": uid}, [pm, uid]),
        edit_ok("alanı olmayan katmanda her öznitelik adı anahtar ve alan olur", {"kind": "create", "view": loose}, [pm, loose]),
        edit_ok("ilişkiden: üst katman $uid, alt katman ilişkinin alanıyla",
                {"kind": "fromRelation", "relation": RELATION["id"], "id": "iliskiden", "name": "Parselin malikleri"}, [pm, rel]),
        edit_ok("ilişkiden, yalnız eşleşenler",
                {"kind": "fromRelation", "relation": RELATION["id"], "id": "iliskiden", "name": "Parselin malikleri", "keep": "matched"},
                [pm, dict(rel, keep="matched")]),
        edit_ok("görünüm değiştirilir", {"kind": "update", "view": dict(pm, name="Malikler", keep="matched")},
                [dict(pm, name="Malikler", keep="matched")]),
        edit_ok("görünüm silinir; kayıtlar değişmez", {"kind": "remove", "id": "parsel-malik"}, []),
        edit_ok("plan yazmaz", {"kind": "create", "view": owners}, [pm, owners], op="plan"),
        edit_refused("kimlik kurala uymuyor", {"edit": {"kind": "create", "view": dict(owners, id="bir görünüm")}},
                     "invalid_join_view", "edit.view"),
        edit_refused("ad boş", {"edit": {"kind": "create", "view": dict(owners, name="")}}, "invalid_join_view", "edit.view"),
        edit_refused("anahtar yazılmamış", {"edit": {"kind": "create", "view": dict(owners, key="")}}, "invalid_join_view", "edit.view"),
        edit_refused("alan iki kez", {"edit": {"kind": "create", "view": dict(owners, fields=["no", "no"])}}, "invalid_join_view", "edit.view"),
        edit_refused("silinecek kimlik kurala uymuyor", {"edit": {"kind": "remove", "id": "bir görünüm"}}, "invalid_join_view", "edit.id"),
        edit_refused("ilişkiden, ad boş", {"edit": {"kind": "fromRelation", "relation": RELATION["id"], "id": "x", "name": " "}},
                     "invalid_join_view", "edit"),
        edit_refused("beklenen sürüm geçersiz", {"edit": {"kind": "create", "view": owners}, "expectedRevision": "07"},
                     "invalid_revision", "expectedRevision"),
        edit_refused("beklenen sürüm çakışan", {"edit": {"kind": "create", "view": owners}, "expectedRevision": "999"},
                     "revision_conflict", "expectedRevision", status="conflict"),
        edit_refused("aynı kimlikli görünüm var", {"edit": {"kind": "create", "view": pm}}, "join_view_exists", "edit.view.id"),
        edit_refused("ilişkiden, aynı kimlikli görünüm var",
                     {"edit": {"kind": "fromRelation", "relation": RELATION["id"], "id": "parsel-malik", "name": "Yine"}},
                     "join_view_exists", "edit.id"),
        edit_refused("51. görünüm", {"edit": {"kind": "create", "view": owners}}, "too_many_join_views", "edit",
                     setup=document(fifty, relations=[RELATION])),
        edit_refused("değiştirilecek görünüm yok", {"edit": {"kind": "update", "view": owners}}, "unknown_join_view", "edit.view.id"),
        edit_refused("silinecek görünüm yok", {"edit": {"kind": "remove", "id": "yok"}}, "unknown_join_view", "edit.id"),
        edit_refused("ilişki yok", {"edit": {"kind": "fromRelation", "relation": "yok", "id": "x", "name": "X"}},
                     "unknown_relation", "edit.relation"),
        edit_refused("katman yok", {"edit": {"kind": "create", "view": dict(owners, layer="yok")}}, "layer_not_found", "edit.view.layer"),
        edit_refused("birleşen düğüm bir grup", {"edit": {"kind": "create", "view": dict(owners, **{"with": "grup"})}},
                     "not_a_layer", "edit.view.with"),
        edit_refused("ana katmanda anahtar alanı yok", {"edit": {"kind": "create", "view": dict(owners, key="numara")}},
                     "field_not_found", "edit.view.key"),
        edit_refused("birleşen katmanda anahtar alanı yok", {"edit": {"kind": "create", "view": dict(owners, withKey="numara")}},
                     "field_not_found", "edit.view.withKey"),
        edit_refused("gösterilecek alan birleşen katmanda yok", {"edit": {"kind": "create", "view": dict(owners, fields=["no", "kat"])}},
                     "field_not_found", "edit.view.fields.1"),
    ]
    return cases


def files():
    edit = {"format": "kentos.command-cases", "version": 1, "command": "cad.joins.edit", "commandVersion": 1,
            "title": "Birleşik görünümler",
            "note": "ADR 0527: projenin birleşik görünümleri; oluşturma, ilişkiden kurma, değiştirme, silme. Proje ayarıdır, "
                    "geri alma adımı değildir. Kural ve beklenenler scripts/fixtures/join_view_cases.py'de KentOS kodu olmadan "
                    "yazılmıştır; retlerin cümleleri çekirdeğindir ($any).",
            "setup": document([PARSEL_MALIK], relations=[RELATION]), "cases": edit_cases()}
    query = {"format": "kentos.command-cases", "version": 1, "command": "cad.joins.query", "commandVersion": 1,
             "title": "Birleşik görünümü oku",
             "note": "ADR 0527: görünümün satırları çizimin şimdiki hâlinden; çoğalan satırlar iki kaydın kalıcı kimliğiyle, "
                     "eşleşme raporu ve her kaydı bir kez sayan toplamlar; kopuk görünüm satır vermez. Kural ve beklenenler "
                     "scripts/fixtures/join_view_cases.py'de KentOS kodu olmadan yazılmıştır; kopukluğun ve retlerin cümleleri "
                     "çekirdeğindir ($any).",
             "setup": document([PARSEL_MALIK]), "cases": query_cases()}
    return [(OUT / "cad.joins.edit.json", edit), (OUT / "cad.joins.query.json", query)]


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def main():
    outs = [(p, text_of(d).encode("utf-8")) for p, d in files()]
    if "--check" in sys.argv[1:]:
        bad = [p for p, data in outs if not p.exists() or p.read_bytes() != data]
        for p in bad:
            print(f"{p.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        n = sum(len(json.loads(data)["cases"]) for _, data in outs)
        print(f"Birleşik görünümlerin durumları tutarlı: {n} durum")
        return 0
    for p, data in outs:
        p.write_bytes(data)
    print(f"written: {', '.join(str(p.relative_to(ROOT)) for p, _ in outs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
