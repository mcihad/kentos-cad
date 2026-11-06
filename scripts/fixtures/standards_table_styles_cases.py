#!/usr/bin/env python3
"""The shared cases of an office standard's table styles (docs/adr/1126 §6), written from the ADR without KentOS code:

- fixtures/commands/v1/cad.standards.install.tables.json (`cad.standards.install` v1: table style rows)
- fixtures/commands/v1/cad.standards.check.tables.json (`cad.standards.check` v1: table style findings)
- fixtures/commands/v1/cad.standards.conflicts.tables.json (`cad.standards.conflicts` v1: two offices' table styles)

    python3 scripts/fixtures/standards_table_styles_cases.py           # writes the files
    python3 scripts/fixtures/standards_table_styles_cases.py --check   # writes nothing; compares

The rules, as the ADR gives them. A package's `resources.tableStyleDefinitions` are table styles (an id, a name and
title, header and data cell styles). Of two bound standards a name both carry is the first's (name trimmed, case
aside). A table style is compared with the drawing's of its name by its appearance: everything but its id and name,
each cell style as a whole, numbers within 1e-9, a missing field apart from a present one. The install row is
`missing` (added by default; `add` or `keepLocal`), `same` (`keepLocal` only) or `differs` (kept by default;
`replace` keeps the drawing's name and id so its tables follow, `renameIncoming` adds the standard's under a new name
no table style of the drawing or of the standards has); an added style keeps its id, or takes `id-2`, `id-3` … when
the drawing has it; a row's `uses` counts the tables that follow the drawing's style of its name (ADR 1127 §1). Rows are the table styles in the standards' order. The check's findings follow the drawing
standard's own (standards_cases.py's `issues`, read as data here): a missing style (`missingTableStyleDefinition`)
and a differing one (`tableStyleDefinition`, its tables counted: those of the drawing in `objects`, those inside
block definitions in the message), both not fixable, `expected` and `found` the appearances as compact JSON with
sorted keys. A conflict (`tableStyleDefinition`) lists the differing cell styles and both values (null when absent).
"""

import copy
import importlib.util
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
STEP = "Standart kaynaklarını kur"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


# ── the resources ─────────────────────────────────────────────────────────────

LOCAL = [
    {"id": "ofis-cetvel", "name": "CETVEL", "data": {"align": "right", "height": 2.0}},
    {"id": "yerel-sade", "name": "SADE", "data": {"align": "left", "horizontalMargin": 1.5}},
    {"id": "ofis-baslik", "name": "YEREL", "title": {"fill": "#ffffff"}},
]
A_TABLES = [
    {"id": "ofis-cetvel-a", "name": "CETVEL", "header": {"align": "center", "fill": "#dddddd"},
     "data": {"align": "right", "height": 2.5}},
    {"id": "ofis-baslik", "name": "BASLIK", "title": {"textStyle": "A-TABLO", "height": 3.5, "align": "center",
                                                      "borderWeight": 0.5, "borderColor": "#000000"}},
    {"id": "ofis-sade", "name": "sade", "data": {"align": "left", "horizontalMargin": 1.5000000001}},
]
B_TABLES = [
    {"id": "b-cetvel", "name": "Cetvel", "data": {"align": "left"}},
    {"id": "b-liste", "name": "PLAN-LISTE", "data": {"verticalMargin": 1.0, "color": "#c0392b"}},
]
A = {"id": "ofis-a", "revision": "2026.1"}
B = {"id": "ofis-b", "revision": "2026.3"}


def package(id_, rev, tables):
    return {"format": "kentos.drawing-standard", "version": 2, "id": id_, "revision": rev,
            "standard": {"name": id_, "layers": []}, "resources": {"tableStyleDefinitions": tables}}


def table(id_, style, layer="0", x=0.0):
    t = {"kind": "table", "id": id_, "layerId": layer, "attrs": {}, "p": {"x": x, "y": 10.0}, "rotation": 0.0,
         "height": 0.25, "rows": [0.5, 0.5], "columns": [2.0, 2.0], "cells": [["Ada", "Alan"], ["101", "512"]],
         "header": True}
    if style is not None:
        t["tableStyle"] = style
    return t


def setup(tables=LOCAL, docs=(A_TABLES, B_TABLES)):
    texts = [{"id": "yazi-tablo", "name": "A-TABLO", "font": "arimo", "height": 2.5}]
    packs = [package(o["id"], o["revision"], t) for o, t in zip((A, B), docs)]
    return {"format": "kentos.document", "version": 1, "name": "Tablo stilleri",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 100, "workspace": "cad", "textStyles": texts, "tableStyles": copy.deepcopy(list(tables)),
                         "standardDocs": [{"text": json.dumps(p, ensure_ascii=False)} for p in packs]},
            "origin": {"x": 0, "y": 0},
            "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False, "expanded": True,
                        "style": STYLE, "children": []}],
            "activeLayer": "0",
            "entities": [table(1, "ofis-cetvel"), table(2, "yerel-sade", x=10.0), table(3, None, x=20.0)],
            "styles": {"items": [], "categories": []}}


# ── comparing ─────────────────────────────────────────────────────────────────

def appearance(t):
    return {k: v for k, v in t.items() if k not in ("id", "name")}


def equal(a, b):
    if isinstance(a, bool) or isinstance(b, bool):
        return a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return abs(a - b) <= 1e-9
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(equal(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(equal(x, y) for x, y in zip(a, b))
    return a == b


def differences(first, later):
    keys = sorted(set(first) | set(later))
    return [k for k in keys if not (k in first and k in later and equal(first[k], later[k]))]


def key(name):
    return name.strip().lower()


def resolved(docs):
    """The first table style of each name in the standards' order, with its source."""
    out, seen = [], set()
    for origin, tables in docs:
        for t in tables:
            if key(t["name"]) not in seen:
                seen.add(key(t["name"]))
                out.append((t, origin))
    return out


# ── cad.standards.install ─────────────────────────────────────────────────────

def following(style_id, entities, blocks=()):
    """The tables that follow table style `style_id` (docs/adr/1127 §1): the drawing's and its block definitions'."""
    inside = [e for b in blocks for e in b.get("entities", [])]
    return sum(1 for e in list(entities) + inside if e.get("kind") == "table" and e.get("tableStyle") == style_id)


def rows_of(local=LOCAL, docs=((A, A_TABLES), (B, B_TABLES)), entities=None):
    entities = setup()["entities"] if entities is None else entities
    out = []
    for t, origin in resolved(docs):
        mine = next((l for l in local if key(l["name"]) == key(t["name"])), None)
        if mine is None:
            status, fields = "missing", []
        else:
            fields = differences(appearance(mine), appearance(t))
            status = "differs" if fields else "same"
        r = {"kind": "tableStyle", "name": t["name"], "status": status,
             "action": "add" if status == "missing" else "keepLocal", "source": origin}
        if fields:
            r["fields"] = fields
        uses = following(mine["id"], entities) if mine is not None else 0
        if uses:
            r["uses"] = uses
        out.append(r)
    return out


def with_choices(rows, **over):
    rows = copy.deepcopy(rows)
    for r in rows:
        if r["name"] in over:
            a = over[r["name"]]
            if isinstance(a, tuple):
                r["action"], r["newName"] = a
            else:
                r["action"] = a
    # The field order of the contract: kind, name, status, action, newName, fields, source.
    ordered = []
    for r in rows:
        o = {k: r[k] for k in ("kind", "name", "status", "action") if k in r}
        if "newName" in r:
            o["newName"] = r["newName"]
        if "fields" in r:
            o["fields"] = r["fields"]
        if "uses" in r:
            o["uses"] = r["uses"]
        o["source"] = r["source"]
        ordered.append(o)
    return ordered


def installed(rows, local=LOCAL):
    incoming = [t for t, _ in resolved(((A, A_TABLES), (B, B_TABLES)))]
    out = copy.deepcopy(local)
    for r in rows:
        if r["status"] == "same" or r["action"] == "keepLocal":
            continue
        new = copy.deepcopy(next(t for t in incoming if key(t["name"]) == key(r["name"])))
        if r["action"] in ("add", "renameIncoming"):
            if r["action"] == "renameIncoming":
                new["name"] = r["newName"]
            ids = {t["id"] for t in out}
            base, k, fresh = new["id"], 2, new["id"]
            while fresh in ids:
                fresh, k = f"{base}-{k}", k + 1
            new["id"] = fresh
            out.append(new)
        else:
            i = next(i for i, t in enumerate(out) if key(t["name"]) == key(r["name"]))
            new["name"], new["id"] = out[i]["name"], out[i]["id"]
            out[i] = new
    return out


def answer(rows):
    n = lambda a: sum(1 for r in rows if r["action"] == a and r["status"] != "same")
    return {"rows": rows, "added": n("add"), "replaced": n("replace"), "renamed": n("renameIncoming"),
            "kept": sum(1 for r in rows if r["action"] == "keepLocal"), "unavailable": 0, "revision": "$current"}


def done(output):
    return {"status": "completed", "output": output, "warnings": []}


def run(choices, rows, op="execute", **kw):
    input_ = ({"choices": choices} if choices else {}) | kw
    if op == "validate":
        return {"op": op, "input": input_, "result": done(None), "expect": {"revision": "same", "canUndo": False}}
    step = {"op": op, "input": input_, "result": done(answer(rows))}
    writes = op == "execute" and any(r["status"] != "same" and r["action"] != "keepLocal" for r in rows)
    step["expect"] = ({"revision": "changed", "canUndo": True, "settings": {"tableStyles": installed(rows)}} if writes
                      else {"revision": "same", "canUndo": False, "settings": {"tableStyles": copy.deepcopy(LOCAL)}})
    return step


def refused(input_, code, message, path):
    return {"op": "execute", "input": input_,
            "result": {"status": "failed", "error": {"code": code, "message": message, "path": path}},
            "expect": {"revision": "same", "canUndo": False}}


def choice(name, action, new_name=None, kind="tableStyle"):
    c = {"kind": kind, "name": name, "action": action}
    if new_name is not None:
        c["newName"] = new_name
    return c


def install_cases():
    rows = rows_of()
    out = []

    def case(name, *steps, doc=None):
        c = {"name": name, "steps": list(steps)}
        if doc is not None:
            c["setup"] = doc
        out.append(c)

    case("Seçimsiz: eksik BASLIK ve PLAN-LISTE eklenir, farklı CETVEL korunur, aynı sade dokunulmaz",
         run([], with_choices(rows)))
    case("Plan yazmaz, aynı satırları verir", run([], with_choices(rows), op="plan"))
    case("Doğrulama yazmaz", run([], rows, op="validate"))
    case("İlk standardın CETVEL'i gelir; ikincininki satır değildir", run([], with_choices(rows)))
    case("Sayılar 1e-9 içinde eşit: sade aynı (kimlik ve ad dışında)", run([], with_choices(rows)))
    case("Çizimde aynı kimlik var: BASLIK ofis-baslik-2 olarak eklenir", run([], with_choices(rows)))
    case("Farklı CETVEL yerinde değiştirilir: yerel ad ve kimlik kalır, tablolar izler",
         run([choice("CETVEL", "replace")], with_choices(rows, CETVEL="replace")))
    case("Farklı CETVEL yeni adla eklenir, çizimdeki kalır",
         run([choice("cetvel", "renameIncoming", "CETVEL-OFIS")], with_choices(rows, CETVEL=("renameIncoming", "CETVEL-OFIS"))))
    case("Eksik olan atlanabilir",
         run([choice("BASLIK", "keepLocal")], with_choices(rows, BASLIK="keepLocal")))
    case("Hepsi atlanınca yazım yok",
         run([choice("BASLIK", "keepLocal"), choice("PLAN-LISTE", "keepLocal")],
             with_choices(rows, BASLIK="keepLocal", **{"PLAN-LISTE": "keepLocal"})))
    case("Eksik olan açıkça eklenir", run([choice("PLAN-LISTE", "add")], with_choices(rows)))
    case("Kurulumu geri al ve yinele",
         run([choice("CETVEL", "replace")], with_choices(rows, CETVEL="replace")),
         {"op": "undo", "returns": STEP, "expect": {"settings": {"tableStyles": copy.deepcopy(LOCAL)}}},
         {"op": "redo", "returns": STEP, "expect": {"settings": {"tableStyles": installed(with_choices(rows, CETVEL="replace"))}}})
    case("Çizimde tablo stili yok: hepsi eksik, eklenir",
         run([], with_choices(rows_of(local=[]))) | {"expect": {"revision": "changed", "canUndo": True,
                                                                "settings": {"tableStyles": installed(rows_of(local=[]), local=[])}}},
         doc=setup(tables=[]))
    case("Standartta olmayan ad: ret", refused({"choices": [choice("YOK", "add")]}, "standard_resource_not_found",
         "Bağlı standartlarda “YOK” adlı tablo stili yok. Kaynağı standartların listesinden seçin.", "choices[0].name"))
    case("Türü yanlış: ret", refused({"choices": [choice("CETVEL", "replace", kind="textStyle")]}, "standard_resource_not_found",
         "Bağlı standartlarda “CETVEL” adlı yazı stili yok. Kaynağı standartların listesinden seçin.", "choices[0].name"))
    case("Aynı stile iki seçim: ret", refused({"choices": [choice("CETVEL", "replace"), choice("cetvel", "keepLocal")]},
         "invalid_install_choice", "“cetvel” tablo stili için iki seçim verildi; her kaynağa bir seçim verin.", "choices[1]"))
    case("Eksik olanı değiştirmek: ret", refused({"choices": [choice("BASLIK", "replace")]}, "invalid_install_choice",
         "“BASLIK” tablo stili çizimde yok: eklenebilir ya da atlanabilir.", "choices[0].action"))
    case("Eksik olanı yeni adla eklemek: ret", refused({"choices": [choice("BASLIK", "renameIncoming", "B2")]},
         "invalid_install_choice", "“BASLIK” tablo stili çizimde yok: eklenebilir ya da atlanabilir.", "choices[0].action"))
    case("Aynı olanı eklemek: ret", refused({"choices": [choice("sade", "add")]}, "invalid_install_choice",
         "“sade” tablo stili çizimdekiyle aynı: değişecek bir şey yok.", "choices[0].action"))
    case("Farklı olanı eklemek: ret", refused({"choices": [choice("CETVEL", "add")]}, "invalid_install_choice",
         "“CETVEL” tablo stili çizimdekinden farklı: değiştirilebilir, yeni adla eklenebilir ya da çizimdeki korunabilir.",
         "choices[0].action"))
    case("Yeni adsız yeni adla ekleme: ret", refused({"choices": [choice("CETVEL", "renameIncoming")]}, "invalid_install_choice",
         "Yeni adla eklemek için kenarı boşluksuz, boş olmayan bir ad verin (newName).", "choices[0].newName"))
    case("Kenarı boşluklu yeni ad: ret", refused({"choices": [choice("CETVEL", "renameIncoming", "CETVEL2 ")]},
         "invalid_install_choice", "Yeni adla eklemek için kenarı boşluksuz, boş olmayan bir ad verin (newName).",
         "choices[0].newName"))
    case("Değiştirmede yeni ad: ret", refused({"choices": [choice("CETVEL", "replace", "X")]}, "invalid_install_choice",
         "Yeni ad yalnız yeni adla eklemede (renameIncoming) verilir.", "choices[0].newName"))
    case("Yeni ad çizimde var (büyük küçük harf ayırmadan): ret",
         refused({"choices": [choice("CETVEL", "renameIncoming", "yerel")]}, "duplicate_name",
                 "“yerel” adlı tablo stili çizimde ya da standartta var; başka bir ad verin.", "choices[0].newName"))
    case("Yeni ad standartta var: ret", refused({"choices": [choice("CETVEL", "renameIncoming", "PLAN-LISTE")]},
         "duplicate_name", "“PLAN-LISTE” adlı tablo stili çizimde ya da standartta var; başka bir ad verin.",
         "choices[0].newName"))
    case("Beklenen revizyon eski: çatışma", {"op": "execute", "input": {"expectedRevision": "999"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict",
            "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
            "path": "expectedRevision", "revision": "$current"}}, "expect": {"revision": "same"}})
    return out


# ── cad.standards.check ───────────────────────────────────────────────────────

def compact(v):
    return json.dumps(v, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def table_findings(resources, local, entities, blocks=()):
    uses = {}
    for e in entities:
        if e.get("kind") == "table" and "tableStyle" in e:
            uses.setdefault(e["tableStyle"], [0, 0])[0] += 1
    for b in blocks:
        for e in b.get("entities", []):
            if e.get("kind") == "table" and "tableStyle" in e:
                uses.setdefault(e["tableStyle"], [0, 0])[1] += 1
    out = []
    for t in resources:
        mine = next((l for l in local if key(l["name"]) == key(t["name"])), None)
        expected = appearance(t)
        if mine is None:
            out.append({"kind": "missingTableStyleDefinition", "subject": t["name"], "expected": compact(expected),
                        "fixable": False, "message": f"“{t['name']}” tablo stili çizimde yok; stili açıkça kurun."})
            continue
        if not differences(appearance(mine), expected):
            continue
        direct, parts = uses.get(mine["id"], [0, 0])
        f = {"kind": "tableStyleDefinition", "subject": t["name"], "expected": compact(expected),
             "found": compact(appearance(mine))}
        if direct:
            f["objects"] = direct
        f["fixable"] = False
        f["message"] = (f"“{t['name']}” tablo stilinin hücre stilleri ofis standardından farklı; değişimi açıkça "
                        f"inceleyin. Bu yerel stil {direct} doğrudan tabloda ve {parts} blok parçasında kullanılıyor.")
        out.append(f)
    return out


def check_cases():
    S = _load("standards_cases")
    st = S.STANDARD
    out = []

    def case(name, resources, local=LOCAL, extra=()):
        doc = S.setup()
        doc["settings"]["textStyles"] = doc["settings"].get("textStyles", [])
        doc["settings"]["tableStyles"] = copy.deepcopy(local)
        doc["entities"] = doc["entities"] + [table(100 + i, s, layer=doc["entities"][0]["layerId"], x=10.0 * i)
                                             for i, s in enumerate(extra)]
        base = S.report(st, doc["layers"], doc["entities"], doc["settings"])
        issues = base["issues"] + table_findings(resources, doc["settings"]["tableStyles"], doc["entities"])
        answer = {"standard": st["name"], "issues": issues, "fixable": sum(1 for i in issues if i["fixable"]),
                  "revision": "$current"}
        input_ = {"standard": st, "resources": {"tableStyleDefinitions": resources}}
        out.append({"name": name, "setup": doc, "steps": [
            {"op": "execute", "input": input_, "result": done(answer), "expect": {"revision": "same", "canUndo": False}},
            {"op": "plan", "input": input_, "result": done(answer), "expect": {"revision": "same"}}]})

    case("Eksik ve farklı tablo stili, aynısı bulgu değil; farklının tablosu sayılır", A_TABLES, extra=["ofis-cetvel"])
    case("Sayılar 1e-9 içinde eşit: bulgu yok", [A_TABLES[2]])
    case("Ad büyük küçük harf ayırmadan eşlenir", [{**A_TABLES[0], "name": "cetvel"}])
    case("Yerelde olmayan alan farktır (boş sayılmaz)", [{"id": "x", "name": "SADE", "data": {"align": "left"}}])
    case("İki tablo kullanır: nesne sayısı 2", [A_TABLES[0]], extra=["ofis-cetvel", "ofis-cetvel"])
    case("Kullanılmayan farklı stil: nesne sayısı yazılmaz", [A_TABLES[0]])
    case("Çizimde tablo stili yok: hepsi eksik", B_TABLES, local=[])
    case("Kaynak boş: tablo bulgusu yok", [])
    # Each cell field alone makes a difference: the standard's CETVEL's data cell with one more or another field.
    for field, value in (("textStyle", "A-TABLO"), ("height", 2.25), ("align", "center"), ("color", "#c0392b"),
                         ("fill", "#eeeeee"), ("horizontalMargin", 1.0), ("verticalMargin", 0.75),
                         ("borderWeight", 0.35), ("borderColor", "#000000")):
        cell = {**LOCAL[0]["data"], field: value}
        case(f"Veri hücresinin {field} alanı farkı bulgudur", [{"id": "s", "name": "CETVEL", "data": cell}],
             extra=["ofis-cetvel"])
    case("Renk yazısı harf büyüklüğüyle farklıdır", [{"id": "s", "name": "YEREL", "title": {"fill": "#FFFFFF"}}])
    case("Aynı alanlar başka hücrede: fark", [{"id": "s", "name": "SADE", "title": LOCAL[1]["data"]}])
    # A broken resource stops the check with its reason.
    doc = S.setup()

    def refusal(name, bad, message):
        out.append({"name": name, "setup": doc, "steps": [{
            "op": "execute", "input": {"standard": st, "resources": {"tableStyleDefinitions": bad}},
            "result": {"status": "failed", "error": {"code": "invalid_standard", "message": message,
                                                     "path": "resources.tableStyleDefinitions"}},
            "expect": {"revision": "same"}}]})

    refusal("Sıfır yazı yüksekliği: ret", [{"id": "x", "name": "BOZUK", "data": {"height": 0.0}}],
            "“BOZUK” tablo stilinin veri hücresi yazı yüksekliği 0; 0'dan büyük en çok 100 mm olmalı")
    refusal("Eksi kenar boşluğu: ret", [{"id": "x", "name": "BOZUK", "header": {"horizontalMargin": -1.5}}],
            "“BOZUK” tablo stilinin sütun başı hücresi yatay kenar boşluğu -1.5; 0 ile en çok 100 mm olmalı")
    refusal("Kısa renk yazısı: ret", [{"id": "x", "name": "BOZUK", "title": {"color": "#fff"}}],
            "“BOZUK” tablo stilinin başlık hücresi yazı rengi “#fff” #rrggbb olmalı")
    refusal("Kenarı boşluklu yazı stili adı: ret", [{"id": "x", "name": "BOZUK", "data": {"textStyle": " A"}}],
            "“BOZUK” tablo stilinin veri hücresi yazı stili adı “ A” geçersiz: boş olmayan, kenarı boşluksuz, en çok 255 "
            "bayt olmalı")
    refusal("Aynı ad iki kez: ret", [{"id": "x", "name": "AYNI"}, {"id": "y", "name": "ayni"}],
            "“ayni” adlı tablo stili iki kez var")
    refusal("Standart'ın adı: ret", [{"id": "x", "name": "Standart"}],
            "“Standart” adı Standart'ındır; tablo stili başka bir ad almalı")
    return out


# ── cad.standards.conflicts ───────────────────────────────────────────────────

def conflicts_cases():
    out = []

    def case(name, first, later):
        doc = setup(tables=[], docs=(first, later))
        found = []
        seen = {}
        for t in first:
            seen.setdefault(key(t["name"]), t)
        for t in later:
            f = seen.get(key(t["name"]))
            if f is None:
                continue
            fields = differences(appearance(f), appearance(t))
            if fields:
                found.append({"kind": "tableStyleDefinition", "name": t["name"], "firstId": "ofis-a", "laterId": "ofis-b",
                              "fields": fields, "firstValues": {k: f.get(k) for k in fields},
                              "laterValues": {k: t.get(k) for k in fields}})
        out.append({"name": name, "setup": doc, "steps": [{"op": "execute", "input": {}, "within": 1e-09,
            "result": done({"conflicts": found, "revision": "$current"}), "expect": {"revision": "same"}}]})

    case("Aynı adlı farklı tablo stili: farklı hücre stilleri ve iki değer", A_TABLES, B_TABLES)
    case("Aynı içerik, farklı kimlik ve ad harfi: fark değil", [A_TABLES[1]], [{**A_TABLES[1], "id": "b", "name": "baslik"}])
    case("Sayılar 1e-9 içinde eşit: fark değil", [A_TABLES[2]], [{**A_TABLES[2], "id": "b-sade"}])
    case("İkincide olmayan hücre stili null'dır", [A_TABLES[0]], [{"id": "b", "name": "CETVEL", "data": A_TABLES[0]["data"]}])
    # One field of one cell style apart: that cell style differs.
    base = {"id": "a-t", "name": "T", "title": {"align": "center"}, "header": {"fill": "#dddddd"},
            "data": {"textStyle": "A-TABLO", "height": 2.0, "align": "right", "color": "#111111", "fill": "#ffffff",
                     "horizontalMargin": 1.5, "verticalMargin": 1.0, "borderWeight": 0.25, "borderColor": "#000000"}}
    for field, value in (("textStyle", "B-TABLO"), ("height", 2.5), ("align", "left"), ("color", "#222222"),
                         ("fill", "#eeeeee"), ("horizontalMargin", 2.0), ("verticalMargin", 0.5),
                         ("borderWeight", 0.35), ("borderColor", "#333333")):
        later = copy.deepcopy(base)
        later["id"], later["data"][field] = "b-t", value
        case(f"Veri hücresinin {field} alanı farkı", [base], [later])
    case("Yalnız ikincide olan ad fark değil", [base], [{**base, "id": "b-t", "name": "BASKA", "data": {}}])
    case("Ad harf büyüklüğüyle farklı, içerik farklı: ikincinin adıyla", [base], [{**base, "id": "b", "name": "t",
                                                                                   "title": {"align": "left"}}])
    case("İkisi aynı, kimlikleri farklı: fark yok", [base], [{**base, "id": "b-t"}])
    case("Bir standartta hücre stili yok, öbüründe var", [{"id": "a", "name": "U"}], [{"id": "b", "name": "U",
                                                                                     "header": {"align": "center"}}])
    case("Renk yazısının harf büyüklüğü farktır", [base], [{**base, "id": "b-t", "header": {"fill": "#DDDDDD"}}])
    case("Üç hücre stili de farklı", [base], [{"id": "b-t", "name": "T", "title": {}, "header": {}, "data": {}}])
    case("Birinci standartta iki stil, ikincide biri farklı", [base, A_TABLES[1]],
         [{**A_TABLES[1], "id": "b-b", "title": {**A_TABLES[1]["title"], "height": 4.0}}, {**base, "id": "b-t"}])
    return out


def build(command, title, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "ADR 1126 §6. Bağımsız başvuru scripts/fixtures/standards_table_styles_cases.py: kurallar ADR'den elle "
                    "yazılır; KentOS kodu yok.",
            "setup": setup(), "cases": cases}


def files():
    return {
        OUT / "cad.standards.install.tables.json": build("cad.standards.install", "Standart kaynaklarını kur: tablo stilleri", install_cases()),
        OUT / "cad.standards.check.tables.json": build("cad.standards.check", "Standardı denetle: tablo stilleri", check_cases()),
        OUT / "cad.standards.conflicts.tables.json": build("cad.standards.conflicts", "Standart farkları: tablo stilleri", conflicts_cases()),
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
