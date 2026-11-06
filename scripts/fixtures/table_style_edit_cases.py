#!/usr/bin/env python3
"""The shared cases of editing table styles and a table's style (docs/adr/1127 §2–§3), written from the ADR and the
contracts' rules without KentOS code:

- fixtures/commands/v1/cad.styles.edit.tables.json (`cad.styles.edit` v1, kind `table`)
- fixtures/commands/v1/cad.entities.edit.tablestyle.json (`cad.entities.edit` v1: a table's `tableStyle`)

    python3 scripts/fixtures/table_style_edit_cases.py           # writes the files
    python3 scripts/fixtures/table_style_edit_cases.py --check   # writes nothing; compares

The rules, as the ADR gives them. A table style is the project's setting (`tableStyles`); `cad.styles.edit` creates,
updates, renames, removes, exports and imports one as it does a text style, its change one undo step “Tablo stili”. A
table draws its style where it sets nothing itself, so an update or a rename writes no table (`followed` 0); a removed
style's tables lose the link (`tableStyle` taken away), those on a locked layer are left and counted in `locked`. A text
style renamed renames itself in the table styles' cell styles that name it (its letters' case aside). The table styles'
rules (`table_style_defs_problem`): ids and names as every style table's (an empty id, an id twice, an empty name, a
name with white space at its ends, the name Standart, a name twice whatever its case), then each style's cell styles in
the order title, header, data, each its text style name, then its height (over 0), its horizontal and vertical margins
and its borders' weight (0 or more), each at most 100 mm, then its colours (`#rrggbb`).

`cad.entities.edit`'s table geometry carries `tableStyle`: absent, the table's kept; null, taken away; an id, the
project's table style it follows (`unknown_style` at `changes[i].geometry.tableStyle` for an id the project has not).
A locked layer refuses the whole edit; every change is one undo step named after the operation, and one that leaves
every object as it was writes nothing (no step). A plan answers the objects as they would be.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
LABEL = "Tablo stili"
FORMAT = "kentos.annotation-styles"
CONFLICT = ("Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
            "yeniden hazırlayın.")
NOTE = ("ADR 1127 §2–§3. Bağımsız başvuru scripts/fixtures/table_style_edit_cases.py: kurallar ADR'den ve sözleşmeden "
        "elle yazılır; KentOS kodu yok.")

TEXT = {"id": "yazi-tablo", "name": "A-TABLO", "font": "arimo"}
CETVEL = {"id": "ofis-cetvel", "name": "CETVEL", "title": {"align": "center", "height": 3.5},
          "header": {"fill": "#dddddd", "borderWeight": 0.5},
          "data": {"textStyle": "A-TABLO", "align": "right", "height": 2.5, "horizontalMargin": 1.5}}
SADE = {"id": "yerel-sade", "name": "SADE", "data": {"align": "left"}}
TABLES = [CETVEL, SADE]


def node(id_, name, locked=False):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": locked, "expanded": True,
            "style": dict(STYLE), "children": []}


def table(id_, style, layer="0", x=0.0):
    t = {"kind": "table", "id": id_, "layerId": layer, "attrs": {}, "p": {"x": x, "y": 10.0}, "rotation": 0.0,
         "height": 0.25, "rows": [0.6, 0.5, 0.5], "columns": [2.0, 2.0],
         "cells": [["Liste", ""], ["Ada", "Alan"], ["101", "512"]],
         "merges": [{"row": 0, "col": 0, "rows": 1, "cols": 2}], "header": True}
    if style is not None:
        t["tableStyle"] = style
    return t


ENTITIES = [table(1, "ofis-cetvel"), table(2, "yerel-sade", x=10.0), table(3, "ofis-cetvel", layer="kilitli", x=20.0),
            table(4, None, x=30.0),
            {"kind": "text", "id": 5, "layerId": "0", "attrs": {}, "p": {"x": 0.0, "y": 0.0}, "text": "Not",
             "height": 0.25, "rotation": 0.0}]


def setup():
    return {"format": "kentos.document", "version": 1, "name": "Tablo stilleri",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 100, "workspace": "cad", "textStyles": [dict(TEXT)],
                         "tableStyles": copy.deepcopy(TABLES)},
            "origin": {"x": 0, "y": 0},
            "layers": [node("0", "0"), node("kilitli", "Kilitli katman", locked=True)],
            "activeLayer": "0", "entities": copy.deepcopy(ENTITIES), "styles": {"items": [], "categories": []}}


def num(v):
    """A number as Rust's `Display` writes an f64: a whole one without its point."""
    v = float(v)
    return str(int(v)) if v.is_integer() else repr(v)


# ── the table styles' rules ───────────────────────────────────────────────────

STANDARD = "Standart"


def names_problem(styles):
    seen_ids, seen_names = [], []
    for s in styles:
        id_, name = s["id"], s["name"]
        if not id_:
            return "tablo stili kimliği boş"
        if id_ in seen_ids:
            return f"“{id_}” kimlikli tablo stili iki kez var"
        trimmed = name.strip()
        if not trimmed:
            return "tablo stili adı boş"
        if trimmed != name:
            return f"“{name}” tablo stili adının başında ya da sonunda boşluk var"
        folded = trimmed.lower()
        if folded == STANDARD.lower():
            return f"“{trimmed}” adı Standart'ındır; tablo stili başka bir ad almalı"
        if folded in seen_names:
            return f"“{trimmed}” adlı tablo stili iki kez var"
        seen_ids.append(id_)
        seen_names.append(folded)
    return None


def hex_colour(c):
    return len(c) == 7 and c[0] == "#" and all(ch in "0123456789abcdefABCDEF" for ch in c[1:])


def cell_problem(cell, what):
    name = cell.get("textStyle")
    if name is not None and (not name.strip() or name.strip() != name or len(name.encode()) > 255):
        return (f"{what} yazı stili adı “{name}” geçersiz: boş olmayan, kenarı boşluksuz, en çok 255 bayt olmalı")
    for field, words, positive in (("height", "yazı yüksekliği", True), ("horizontalMargin", "yatay kenar boşluğu", False),
                                   ("verticalMargin", "dikey kenar boşluğu", False),
                                   ("borderWeight", "kenar çizgisi kalınlığı", False)):
        v = cell.get(field)
        if v is None:
            continue
        low = v > 0 if positive else v >= 0
        if not (low and v <= 100):
            start = "0'dan büyük" if positive else "0 ile"
            return f"{what} {words} {num(v)}; {start} en çok 100 mm olmalı"
    for field, words in (("color", "yazı rengi"), ("fill", "dolgu rengi"), ("borderColor", "kenar çizgisi rengi")):
        c = cell.get(field)
        if c is not None and not hex_colour(c):
            return f"{what} {words} “{c}” #rrggbb olmalı"
    return None


def problem(styles):
    p = names_problem(styles)
    if p:
        return p
    for s in styles:
        for key, what in (("title", "başlık hücresi"), ("header", "sütun başı hücresi"), ("data", "veri hücresi")):
            if key in s:
                p = cell_problem(s[key], f"“{s['name']}” tablo stilinin {what}")
                if p:
                    return p
    return None


# ── cad.styles.edit, kind table ───────────────────────────────────────────────

def edited(style, followed=0, locked=0, changed=True, **lists):
    out = {"style": style, "changed": changed, "followed": followed, "locked": locked}
    for k in ("added", "replaced", "skipped", "library"):
        if lists.get(k):
            out[k] = lists[k]
    out["revision"] = "$current"
    return {"status": "completed", "output": out, "warnings": []}


def failed(code, message, path, conflict=False):
    e = {"code": code, "message": message, "path": path}
    if conflict:
        e["revision"] = "$current"
    return {"status": "conflict" if conflict else "failed", "error": e}


def library(styles, kind="table"):
    return json.dumps({"format": FORMAT, "version": 1, "kind": kind, "styles": styles}, ensure_ascii=False,
                      sort_keys=True, separators=(",", ":"))


def without_style(e):
    e = copy.deepcopy(e)
    e.pop("tableStyle", None)
    return e


def styles_cases():
    out = []

    def case(name, *steps):
        out.append({"name": name, "steps": list(steps)})

    def execute(input_, result, expect):
        return {"op": "execute", "input": input_, "result": result, "expect": expect}

    def written(tables, entities=None):
        e = {"revision": "changed", "canUndo": True, "dirty": True, "settings": {"tableStyles": tables}}
        if entities:
            e["entities"] = entities
        return e

    def undone(entities=None):
        e = {"settings": {"tableStyles": copy.deepcopy(TABLES)}, "canUndo": False}
        if entities:
            e["entities"] = entities
        return {"op": "undo", "returns": LABEL, "expect": e}

    def refused(name, input_, code, message, path):
        case(name, execute(input_, failed(code, message, path),
                           {"revision": "same", "canUndo": False, "settings": {"tableStyles": copy.deepcopy(TABLES)}}))

    def invalid(name, input_, tables, path):
        refused(name, input_, "invalid_style", f"Tablo stilleri: {problem(tables)}.", path)

    liste = {"id": "ofis-liste", "name": "LISTE", "header": {"align": "center", "borderColor": "#c0392b"},
             "data": {"verticalMargin": 1.0, "borderWeight": 0.25, "color": "#2244dd"}}
    case("Oluştur: tablonun sonuna; tablo yazılmaz; tek adım “Tablo stili”, geri al stili kaldırır",
         execute({"kind": "table", "operation": "create", "table": liste}, edited("ofis-liste"),
                 written(TABLES + [liste])),
         undone())
    fresh = {**liste, "id": ""}
    case("Oluştur: boş kimliğe yeni bir kimlik verilir",
         execute({"kind": "table", "operation": "create", "table": fresh}, edited("$any"),
                 written(TABLES + [{**liste, "id": "$any"}])))
    refused("Oluştur: tanımsız ret", {"kind": "table", "operation": "create"}, "no_definition",
            "Stilin tanımını table ile verin.", "table")
    refused("Oluştur: var olan kimlik ret", {"kind": "table", "operation": "create", "table": {**liste, "id": "yerel-sade"}},
            "duplicate_style",
            "“yerel-sade” kimlikli tablo stili zaten var; yeni stile başka bir kimlik verin ya da kimliği boş bırakın.",
            "table.id")
    for name, bad in (("Oluştur: ad büyük küçük harf ayırmadan var, ret", {**liste, "name": "cetvel"}),
                      ("Oluştur: Standart adı ret", {**liste, "name": "standart"}),
                      ("Oluştur: kenarı boşluklu ad ret", {**liste, "name": "LISTE "}),
                      ("Oluştur: boş ad ret", {**liste, "name": "  "}),
                      ("Oluştur: yazı yüksekliği 0 ret", {**liste, "data": {"height": 0.0}}),
                      ("Oluştur: yazı yüksekliği 100 mm'den büyük ret", {**liste, "title": {"height": 100.5}}),
                      ("Oluştur: eksi yatay kenar boşluğu ret", {**liste, "header": {"horizontalMargin": -1.0}}),
                      ("Oluştur: kenar çizgisi kalınlığı 100 mm'den büyük ret", {**liste, "data": {"borderWeight": 120.0}}),
                      ("Oluştur: #rrggbb olmayan dolgu rengi ret", {**liste, "header": {"fill": "gri"}}),
                      ("Oluştur: kısa kenar çizgisi rengi ret", {**liste, "data": {"borderColor": "#12345"}}),
                      ("Oluştur: kenarı boşluklu yazı stili adı ret", {**liste, "title": {"textStyle": " A-TABLO"}}),
                      ("Oluştur: başlık sütun başından önce denetlenir",
                       {**liste, "title": {"color": "kırmızı"}, "header": {"height": 0.0}})):
        invalid(name, {"kind": "table", "operation": "create", "table": bad}, TABLES + [bad], "table")
    cetvel2 = {**CETVEL, "data": {**CETVEL["data"], "height": 3.0, "borderWeight": 0.35, "borderColor": "#000000"}}
    case("Güncelle: yerinde; tablolar onu çizdiği için yazılmaz; geri al eskisini getirir",
         execute({"kind": "table", "operation": "update", "table": cetvel2}, edited("ofis-cetvel"),
                 written([cetvel2, SADE], {"1": ENTITIES[0], "3": ENTITIES[2]})),
         undone())
    refused("Güncelle: projede olmayan kimlik ret", {"kind": "table", "operation": "update", "table": {**SADE, "id": "yok"}},
            "style_not_found", "“yok” kimlikli tablo stili projede yok.", "table.id")
    refused("Güncelle: tanımsız ret", {"kind": "table", "operation": "update"}, "no_definition",
            "Stilin tanımını table ile verin.", "table")
    case("Yeniden adlandır: tablolar kimlikle bağlı kalır",
         execute({"kind": "table", "operation": "rename", "style": "yerel-sade", "name": " LISTE "}, edited("yerel-sade"),
                 written([CETVEL, {**SADE, "name": "LISTE"}], {"2": ENTITIES[1]})),
         undone())
    invalid("Yeniden adlandır: başkasının adı ret", {"kind": "table", "operation": "rename", "style": "yerel-sade",
                                                     "name": "Cetvel"}, [CETVEL, {**SADE, "name": "Cetvel"}], "name")
    refused("Yeniden adlandır: boş ad ret", {"kind": "table", "operation": "rename", "style": "yerel-sade", "name": " "},
            "empty_name", "Stilin adı boş olamaz; bir ad yazın.", "name")
    refused("Yeniden adlandır: stilsiz ret", {"kind": "table", "operation": "rename", "name": "X"}, "no_style",
            "Stili kimliğiyle verin (style).", "style")
    case("Sil: tabloları bağı yitirir, kilitli katmandaki kalır ve sayılır; geri al stili ve bağı getirir",
         execute({"kind": "table", "operation": "remove", "style": "ofis-cetvel"}, edited("ofis-cetvel", 1, 1),
                 written([SADE], {"1": without_style(ENTITIES[0]), "3": ENTITIES[2]})),
         undone({"1": ENTITIES[0], "3": ENTITIES[2]}))
    case("Sil: stilsiz tablo ve yazı dokunulmaz",
         execute({"kind": "table", "operation": "remove", "style": "yerel-sade"}, edited("yerel-sade", 1),
                 written([CETVEL], {"2": without_style(ENTITIES[1]), "4": ENTITIES[3], "5": ENTITIES[4]})))
    refused("Sil: projede olmayan stil ret", {"kind": "table", "operation": "remove", "style": "yok"}, "style_not_found",
            "“yok” kimlikli tablo stili projede yok.", "style")
    case("Planla: silmenin ne yapacağını söyler, yazmaz",
         {"op": "plan", "input": {"kind": "table", "operation": "remove", "style": "ofis-cetvel"},
          "result": edited("ofis-cetvel", 1, 1),
          "expect": {"revision": "same", "canUndo": False, "settings": {"tableStyles": copy.deepcopy(TABLES)},
                     "entities": {"1": ENTITIES[0]}}})
    case("Dışa aktar: hepsi tablodaki sırayla",
         execute({"kind": "table", "operation": "export"}, edited("ofis-cetvel", changed=False, library=library(TABLES)),
                 {"revision": "same", "canUndo": False}))
    case("Dışa aktar: biri",
         execute({"kind": "table", "operation": "export", "styles": ["yerel-sade"]},
                 edited("yerel-sade", changed=False, library=library([SADE])), {"revision": "same", "canUndo": False}))
    refused("Dışa aktar: projede olmayan stil ret", {"kind": "table", "operation": "export", "styles": ["yok"]},
            "style_not_found", "“yok” kimlikli tablo stili projede yok.", "styles[0]")
    incoming = [{**CETVEL, "id": "baska-cetvel", "data": {"align": "center"}}, liste]
    case("Al: yeni ad eklenir, var olan ad korunur",
         execute({"kind": "table", "operation": "import", "library": library(incoming)},
                 edited("ofis-liste", added=["ofis-liste"], skipped=["CETVEL"]), written(TABLES + [liste])))
    case("Al, değiştirerek: var olan ad yerinde güncellenir, kimliği ve adı kalır",
         execute({"kind": "table", "operation": "import", "library": library(incoming), "replace": True},
                 edited("ofis-liste", added=["ofis-liste"], replaced=["ofis-cetvel"]),
                 written([{**incoming[0], "id": "ofis-cetvel"}, SADE, liste])))
    refused("Al: başka türün dosyası ret", {"kind": "table", "operation": "import", "library": library([TEXT], "text")},
            "invalid_library",
            f"Stil dosyası okunamadı: dosyanın stilleri “text”; istenen “table”. KentOS'un stilleri dışa aktarmasının "
            f"yazdığı bir {FORMAT} dosyası verin.", "library")
    bad = [{**liste, "data": {"height": 0.0}}]
    invalid("Al: dosyanın stili kurala uymuyor, ret", {"kind": "table", "operation": "import", "library": library(bad)},
            TABLES + bad, "library")
    refused("Al: dosyasız ret", {"kind": "table", "operation": "import"}, "no_library",
            "Alınacak stil dosyasının metnini (library) verin.", "library")
    renamed = copy.deepcopy(CETVEL)
    renamed["data"]["textStyle"] = "B-TABLO"
    case("Yazı stili yeniden adlandırılınca tablo stilinin hücre stili onu yeni adıyla anar",
         execute({"kind": "text", "operation": "rename", "style": "yazi-tablo", "name": "B-TABLO"}, edited("yazi-tablo"),
                 {"revision": "changed", "canUndo": True,
                  "settings": {"textStyles": [{**TEXT, "name": "B-TABLO"}], "tableStyles": [renamed, SADE]}}),
         {"op": "undo", "returns": "Yazı stili",
          "expect": {"settings": {"textStyles": [TEXT], "tableStyles": copy.deepcopy(TABLES)}}})
    case("Beklenen revizyon eski: çatışma",
         execute({"kind": "table", "operation": "remove", "style": "yerel-sade", "expectedRevision": "999"},
                 failed("revision_conflict", CONFLICT, "expectedRevision", conflict=True), {"revision": "same"}))
    return out


# ── cad.entities.edit: a table's style ────────────────────────────────────────

def geometry(e, style="keep"):
    """The table's geometry as `cad.entities.edit` takes it: its kind and the fields that place and shape it."""
    g = {k: v for k, v in e.items() if k not in ("id", "layerId", "attrs", "tableStyle")}
    if style != "keep":
        g["tableStyle"] = style
    return g


def entities_cases():
    out = []

    def case(name, *steps):
        out.append({"name": name, "steps": list(steps)})

    def change(id_, g):
        return {"kind": "update", "uid": f"$uidOf:{id_}", "geometry": g}

    def done(ids):
        return {"status": "completed", "output": {"changed": ids, "created": [], "removed": [], "revision": "$current"},
                "warnings": []}

    def execute(changes, ids, entities, operation="properties", writes=True):
        expect = {"revision": "changed", "canUndo": True} if writes else {"revision": "same", "canUndo": False}
        return {"op": "execute", "input": {"operation": operation, "changes": changes}, "result": done(ids),
                "expect": expect | {"entities": entities}}

    def undo(label, entities):
        return {"op": "undo", "returns": label, "expect": {"entities": entities, "canUndo": False}}

    def refused(name, changes, code, message, path, operation="properties"):
        case(name, {"op": "execute", "input": {"operation": operation, "changes": changes},
                    "result": failed(code, message, path),
                    "expect": {"revision": "same", "canUndo": False,
                               "entities": {str(e["id"]): e for e in ENTITIES}}})

    t1, t2, t3, t4 = ENTITIES[:4]
    uid = lambda i: f"$uidOf:{i}"
    case("Stil ver: stilsiz tabloya CETVEL; tek adım “Değiştir”, geri al stili kaldırır",
         execute([change(4, geometry(t4, "ofis-cetvel"))], [uid(4)], {"4": {**t4, "tableStyle": "ofis-cetvel"}}),
         undo("Değiştir", {"4": t4}))
    case("Stili değiştir: CETVEL'den SADE'ye",
         execute([change(1, geometry(t1, "yerel-sade"))], [uid(1)], {"1": {**t1, "tableStyle": "yerel-sade"}}),
         undo("Değiştir", {"1": t1}))
    case("Stili kaldır: null bağı kaldırır, tablonun kendi değerleri kalır",
         execute([change(1, geometry(t1, None))], [uid(1)], {"1": without_style(t1)}),
         undo("Değiştir", {"1": t1}))
    case("Stilsiz geometri: tablonun stili kalır (ADR 1127 §3)",
         execute([change(1, geometry(t1))], [uid(1)], {"1": t1}, writes=False))
    moved = {**t1, "p": {"x": 5.0, "y": 12.0}}
    case("Taşınan tablo stilini korur",
         execute([change(1, geometry(moved))], [uid(1)], {"1": moved}))
    wider = {**t2, "columns": [3.0, 2.5]}
    case("Tabloyu düzenle: sütunlar değişir, stil kalır; adım “Tablo”",
         execute([change(2, geometry(wider))], [uid(2)], {"2": wider}, operation="table"),
         undo("Tablo", {"2": t2}))
    case("Tabloyu düzenle: stil ve hücreler birlikte",
         execute([change(2, geometry({**wider, "cells": [["Liste", ""], ["Ada", "Alan"], ["102", "640"]]}, "ofis-cetvel"))],
                 [uid(2)], {"2": {**wider, "cells": [["Liste", ""], ["Ada", "Alan"], ["102", "640"]],
                                  "tableStyle": "ofis-cetvel"}}, operation="table"))
    case("İki tabloya bir adımda: biri verilir, biri kaldırılır",
         execute([change(2, geometry(t2, "ofis-cetvel")), change(4, geometry(t4, "yerel-sade"))], [uid(2), uid(4)],
                 {"2": {**t2, "tableStyle": "ofis-cetvel"}, "4": {**t4, "tableStyle": "yerel-sade"}}),
         undo("Değiştir", {"2": t2, "4": t4}))
    case("Aynı stili yeniden vermek tabloyu değiştirmez",
         execute([change(1, geometry(t1, "ofis-cetvel"))], [uid(1)], {"1": t1}, writes=False))
    case("Stilsiz tabloya null: değişmez",
         execute([change(4, geometry(t4, None))], [uid(4)], {"4": t4}, writes=False))
    case("Geri al ve yinele",
         execute([change(4, geometry(t4, "yerel-sade"))], [uid(4)], {"4": {**t4, "tableStyle": "yerel-sade"}}),
         undo("Değiştir", {"4": t4}),
         {"op": "redo", "returns": "Değiştir", "expect": {"entities": {"4": {**t4, "tableStyle": "yerel-sade"}}}})
    unknown = ("“yok” kimlikli tablo stili projede yok: silinmiş ya da başka bir projenin olabilir. Projenin bir tablo "
               "stilinin kimliğini verin ya da null ile stili kaldırın (Standart).")
    refused("Projede olmayan stil: ret", [change(4, geometry(t4, "yok"))], "unknown_style", unknown,
            "changes[0].geometry.tableStyle")
    refused("İkinci değişikliğin stili yok: hiçbiri yazılmaz",
            [change(2, geometry(t2, "ofis-cetvel")), change(4, geometry(t4, "yok"))], "unknown_style", unknown,
            "changes[1].geometry.tableStyle")
    refused("Kimliğin harf büyüklüğü farkı: ret (kimlik tam eşleşir)", [change(4, geometry(t4, "OFIS-CETVEL"))],
            "unknown_style", unknown.replace("“yok”", "“OFIS-CETVEL”"), "changes[0].geometry.tableStyle")
    locked = "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
    refused("Kilitli katmandaki tablo: ret", [change(3, geometry(t3, None))], "layer_locked", locked, "changes[0].uid")
    refused("Kilitli katmandaki tablo başka bir değişiklikle: hiçbiri yazılmaz",
            [change(4, geometry(t4, "ofis-cetvel")), change(3, geometry(t3, "yerel-sade"))], "layer_locked", locked,
            "changes[1].uid")
    refused("Tabloyu düzenle yalnız tabloları değiştirir",
            [change(5, {"kind": "text", "p": {"x": 0.0, "y": 0.0}, "text": "Not", "height": 0.25, "rotation": 0.0})],
            "not_a_table",
            "Tablonun düzenlemesi yalnız tabloları değiştirir: her değişiklik bir tablonun yeni hâli olmalı (update, tablo "
            "geometrisi).", "changes[0]", operation="table")
    case("Planla: stil verilir gibi söyler, yazmaz",
         {"op": "plan", "input": {"operation": "properties", "changes": [change(4, geometry(t4, "ofis-cetvel"))]},
          "result": done([{**t4, "tableStyle": "ofis-cetvel"}]),
          "expect": {"revision": "same", "canUndo": False, "entities": {"4": t4}}})
    case("Doğrula: yazmaz",
         {"op": "validate", "input": {"operation": "properties", "changes": [change(4, geometry(t4, "ofis-cetvel"))]},
          "result": {"status": "completed", "output": None, "warnings": []},
          "expect": {"revision": "same", "canUndo": False}})
    case("Doğrula: projede olmayan stil ret",
         {"op": "validate", "input": {"operation": "properties", "changes": [change(4, geometry(t4, "yok"))]},
          "result": failed("unknown_style", unknown, "changes[0].geometry.tableStyle"),
          "expect": {"revision": "same", "canUndo": False}})
    return out


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": NOTE, "setup": setup(), "cases": cases}


def files():
    return {
        OUT / "cad.styles.edit.tables.json": build("cad.styles.edit", "Stilleri düzenle: tablo stilleri", styles_cases()),
        OUT / "cad.entities.edit.tablestyle.json": build("cad.entities.edit", "Nesneleri düzenle: tablonun stili",
                                                         entities_cases()),
    }


def main():
    bad = 0
    for path, doc in files().items():
        text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv[1:]:
            if not path.exists() or path.read_text("utf-8") != text:
                print(f"{path.relative_to(ROOT)} güncel değil")
                bad += 1
            continue
        path.write_text(text, "utf-8")
        print(f"yazıldı: {path.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
