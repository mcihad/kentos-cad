#!/usr/bin/env python3
"""The shared cases of `cad.fields.write` (docs/adr/0507, plan/gis.md CBS-20), written without KentOS code.

    python3 scripts/fixtures/fields_write_cases.py           # writes fixtures/commands/v1/cad.fields.write.json
    python3 scripts/fixtures/fields_write_cases.py --check   # compares

An expression's value per object comes from the language's independent reference (expression_language.py: its
parser and evaluator, the JavaScript number text); a value written to a field is checked by the fields' independent
reference (layer_field_cases.py's `check`: canonical text, kind, scale, length, required). Unique fields, labels that
follow, whole-or-nothing and the listing are the ADR's rules written here.
"""

import copy
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import expression_language as lang  # noqa: E402
import layer_field_cases as fields  # noqa: E402

ROOT = HERE.parents[1]
OUT = ROOT / "fixtures/commands/v1/cad.fields.write.json"


def P(x, y):
    return {"x": x, "y": y}


def square(x, y, side=20):
    return [P(x, y), P(x + side, y), P(x + side, y + side), P(x, y + side)]


FIELDS = [
    {"name": "Ada", "kind": "integer"},
    {"name": "Parsel", "kind": "integer", "required": True},
    {"name": "Alan", "kind": "decimal", "scale": 2},
    {"name": "Nitelik", "kind": "text", "length": 10},
    {"name": "Kod", "kind": "text", "unique": True},
]
BY_NAME = {f["name"]: f for f in FIELDS}


def layer(i, name, locked=False, fields_=None, children=None, kind="layer"):
    n = {"id": i, "name": name, "type": kind, "visible": True, "locked": locked, "expanded": True,
         "style": {"color": "#0090FF", "lineType": "continuous", "lineWeight": 0.35}, "children": children or []}
    if fields_:
        n["fields"] = fields_
    return n


ENTITIES = [
    {"kind": "polygon", "id": 1, "layerId": "parsel", "attrs": {"Ada": "101", "Parsel": "1", "Alan": "120.50", "Nitelik": "Arsa", "Kod": "A1"},
     "label": "1", "pts": square(500000, 4400000)},
    {"kind": "polygon", "id": 2, "layerId": "parsel", "attrs": {"Ada": "101", "Parsel": "2", "Alan": "80", "Nitelik": "Tarla", "Kod": "A2"},
     "pts": square(500030, 4400000)},
    {"kind": "polygon", "id": 3, "layerId": "parsel", "attrs": {"Ada": "102", "Parsel": "3", "Nitelik": "Arsa", "Kod": "A3"},
     "pts": square(500060, 4400000)},
    {"kind": "polygon", "id": 4, "layerId": "parsel", "attrs": {"Ada": "102", "Parsel": "4", "Alan": "15.25", "Kod": "A4"},
     "pts": square(500090, 4400000)},
    {"kind": "point", "id": 5, "layerId": "serbest", "attrs": {"Ad": "kuyu"}, "p": P(500000, 4400100)},
    {"kind": "line", "id": 6, "layerId": "serbest", "attrs": {}, "a": P(500010, 4400100), "b": P(500020, 4400100)},
    {"kind": "point", "id": 7, "layerId": "kilitli", "attrs": {"Ad": "eski"}, "p": P(500000, 4400200)},
]
BY_ID = {e["id"]: e for e in ENTITIES}
IDS = [e["id"] for e in ENTITIES]
NAMES = {"parsel": "Parsel", "serbest": "Serbest", "kilitli": "Kilitli", "grup": "Grup"}
PARCELS = [1, 2, 3, 4]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Toplu yazma",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 500000, "y": 4400000},
    "layers": [
        layer("parsel", "Parsel", fields_=FIELDS),
        layer("serbest", "Serbest"),
        layer("kilitli", "Kilitli", locked=True),
        layer("grup", "Grup", kind="group", children=[layer("ic", "İç")]),
    ],
    "activeLayer": "parsel",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

LISTED = 200


def uid(i):
    return f"$uidOf:{i}"


def evaluate(src, ids):
    """The expression's value as text per object (None: empty), as the language's reference gives it."""
    tree = lang.Parser(lang.tokenize(src)).parse()
    out = []
    for i in ids:
        v = lang.value(tree, dict(BY_ID[i]["attrs"]))
        out.append(None if lang.is_empty(v) else lang.text(v))
    return out


def run(layer_id, write, policy="all"):
    """The ADR's rules: (changed objects as written, counts, changes listed, problems listed)."""
    typed = layer_id == "parsel"
    if write["kind"] == "calculate":
        ids = [int(u.split(":")[1]) for u in write.get("uids", [])] or [e["id"] for e in ENTITIES if e["layerId"] == layer_id]
        values = evaluate(write["expression"], ids)
        wants, kept = [], 0
        for i, v in zip(ids, values):
            if v not in (None, ""):
                wants.append((i, {write["field"]: v}))
            elif write.get("empty", "keep") == "keep":
                kept += 1
            else:
                wants.append((i, {write["field"]: None}))
    else:
        kept = 0
        wants = [(int(r["uid"].split(":")[1]), r["values"]) for r in write["rows"]]
    problems, changed, unchanged, bad = [], [], 0, set()
    for i, values in wants:
        before = BY_ID[i]["attrs"]
        after = dict(before)
        failed = None
        for k in sorted(values):
            v = values[k]
            if typed and k in BY_NAME:
                got = fields.check(BY_NAME[k], v or "")
                if "error" in got:
                    failed = (k, v, "attribute_required" if got["error"] == "required" else "invalid_attribute", got["message"])
                    break
                canon = got["value"] if v is not None else None
            else:
                canon = v
            if canon in (None, ""):
                after.pop(k, None)
            else:
                after[k] = canon
        if failed:
            problems.append({"uid": uid(i), "field": failed[0], **({"value": failed[1]} if failed[1] is not None else {}),
                             "code": failed[2], "message": failed[3]})
            bad.add(i)
            continue
        if after == before:
            unchanged += 1
            continue
        changed.append((i, before, after))
    # Unique fields over the writes together, taken out until the rest fit.
    while typed and changed:
        clash = None
        written = {i for i, _, _ in changed}
        for f in (f for f in FIELDS if f.get("unique")):
            name = f["name"]
            counts = {}
            for e in ENTITIES:
                if e["layerId"] == layer_id and e["id"] not in written and name in e["attrs"]:
                    counts[e["attrs"][name]] = counts.get(e["attrs"][name], 0) + 1
            for _, _, after in changed:
                if after.get(name, "").strip():
                    counts[after[name]] = counts.get(after[name], 0) + 1
            for i, before, after in changed:
                v = after.get(name)
                if v and v.strip() and before.get(name) != v and counts.get(v, 0) > 1:
                    clash = (name, v)
                    break
            if clash:
                break
        if not clash:
            break
        name, v = clash
        msg = f"“{name}” alanı katmanda tekil; “{v}” başka bir kayıtta var. Başka bir değer verin."
        others = {e["attrs"].get(name) for e in ENTITIES if e["layerId"] == layer_id and e["id"] not in written}
        hit = [k for k, (i, _, after) in enumerate(changed)
               if after.get(name) is not None and (after[name] in others
                   or any(j != k and c[2].get(name) == after[name] for j, c in enumerate(changed)))]
        hit = hit or list(range(len(changed)))
        for k in reversed(hit):
            i, _, after = changed.pop(k)
            problems.append({"uid": uid(i), "field": name, **({"value": after[name]} if name in after else {}),
                             "code": "not_unique", "message": msg})
            bad.add(i)
    order = {i: n for n, (i, _) in enumerate(wants)}
    problems.sort(key=lambda p: order[int(p["uid"].split(":")[1])])
    changes = []
    for i, before, after in changed:
        for k in sorted(set(before) | set(after)):
            if before.get(k) != after.get(k) and len(changes) < LISTED:
                c = {"uid": uid(i), "field": k}
                if k in before:
                    c["before"] = before[k]
                if k in after:
                    c["after"] = after[k]
                changes.append(c)
    out = {"changed": len(changed), "unchanged": unchanged, "kept": kept, "invalid": len(bad)}
    if changes:
        out["changes"] = changes
    if problems:
        out["problems"] = problems[:LISTED]
    out["revision"] = "$current"
    return changed, out, problems


def written(changed):
    """The objects as written: attributes, and a label that showed a value written over follows it."""
    ents = {}
    for i, before, after in changed:
        e = copy.deepcopy(BY_ID[i])
        e["attrs"] = after
        label = e.get("label")
        if label is not None:
            for k in sorted(before):
                if before[k] == label and after.get(k) != before[k]:
                    if k in after:
                        e["label"] = after[k]
                    else:
                        e.pop("label")
                    break
        ents[str(i)] = e
    return ents


def step(op, inp, result, expect=None):
    s = {"op": op, "input": inp, "result": result}
    if expect is not None:
        s["expect"] = expect
    return s


def completed(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


UNCHANGED = {"ids": IDS, "revision": "same", "canUndo": False}


def write_case(name, layer_id, write, policy="all", undo=None):
    inp = {"layer": layer_id, "write": write}
    if policy != "all":
        inp["policy"] = policy
    changed, out, problems = run(layer_id, write, policy)
    if problems and policy == "all":
        first = problems[0]
        msg = (f"{out['invalid']} nesnenin değeri alanların kurallarına uymuyor; hiçbir değer yazılmadı. "
               f"İlki: {first['message']} ({first['field']}). Sorunların hepsi plan'da; uyanları yazmak için policy valid verin.")
        return {"name": name, "steps": [step("execute", inp, failed("invalid_values", msg, "write"), UNCHANGED)]}
    warnings = []
    if problems:
        first = problems[0]
        warnings.append({"code": "values_skipped", "path": "write",
                         "message": f"{out['invalid']} nesnenin değeri kurallara uymadığı için yazılmadı. İlki: {first['message']} ({first['field']})."})
    if changed:
        expect = {"ids": IDS, "entities": written(changed), "canUndo": True, "dirty": True, "revision": "changed"}
    else:
        expect = UNCHANGED
    steps = [step("execute", inp, completed(out, warnings), expect)]
    if undo:
        steps.append({"op": "undo", "returns": undo, "expect": {"entities": {str(i): BY_ID[i] for i, _, _ in changed}}})
    return {"name": name, "steps": steps}


def plan_case(name, layer_id, write, policy="all"):
    inp = {"layer": layer_id, "write": write}
    if policy != "all":
        inp["policy"] = policy
    _, out, _ = run(layer_id, write, policy)
    return {"name": name, "steps": [step("plan", inp, completed(out), UNCHANGED)]}


def refused(name, inp, code, message, path):
    return {"name": name, "steps": [step("execute", inp, failed(code, message, path), UNCHANGED)]}


def calc(field, expression, uids=None, empty=None):
    w = {"kind": "calculate", "field": field, "expression": expression}
    if uids:
        w["uids"] = [uid(i) for i in uids]
    if empty:
        w["empty"] = empty
    return w


def values(*rows):
    return {"kind": "values", "rows": [{"uid": uid(i), "values": v} for i, v in rows]}


def cases():
    out = [
        write_case("Alan hesapla: Ada || '/' || Parsel Nitelik'e; adımı “Alan hesapla: Nitelik”, geri alınır", "parsel",
                   calc("Nitelik", "Ada || '/' || Parsel"), undo="Alan hesapla: Nitelik"),
        write_case("Alan hesapla: Alan * 2 ondalık alana, kanonik metniyle; boş sonuçta değer kalır", "parsel",
                   calc("Alan", "Alan * 2")),
        write_case("Alan hesapla: boş sonuç clear ile değeri kaldırır (zaten yoksa değişmez)", "parsel",
                   calc("Alan", "Alan * 2", empty="clear")),
        write_case("Alan hesapla: yalnız verilen nesneler; değeri gösteren etiket yenisini gösterir", "parsel",
                   calc("Parsel", "Parsel * 10", uids=[1, 2])),
        write_case("Alan hesapla: ölçeğe sığmayan ondalıklar bütün yazmayı reddeder (policy all)", "parsel",
                   calc("Alan", "Alan / 3")),
        plan_case("Plan: sığmayanlar listelenir, reddedilmez", "parsel", calc("Alan", "Alan / 3")),
        write_case("policy valid: hiçbiri uymazsa yazılmaz, söylenir", "parsel", calc("Alan", "Alan / 3"), policy="valid"),
        write_case("Alan hesapla: zorunlu alana boş değer clear ile reddedilir", "parsel",
                   calc("Parsel", "''", empty="clear")),
        write_case("Alan hesapla: 10 karakteri aşan metin reddedilir", "parsel", calc("Nitelik", "Nitelik || ' ve bahçe'")),
        write_case("policy valid: uyanlar yazılır, uzun olanlar atlanır", "parsel",
                   calc("Nitelik", "Nitelik || ' ve bahçe'", uids=[1, 2]), policy="valid"),
        write_case("Alanı olmayan katmanda öznitelik adıyla yazılır", "serbest", calc("Ad", "Ad || '-x'")),
        write_case("Yapıştır: iki satır, adımı “Yapıştır”, geri alınır", "parsel",
                   values((1, {"Ada": "201"}), (2, {"Ada": "202", "Nitelik": "Bahçe"})), undo="Yapıştır"),
        write_case("Yapıştır: tam sayı alanına harfli değer bütün yazmayı reddeder", "parsel",
                   values((1, {"Ada": "201"}), (2, {"Ada": "12a"}))),
        write_case("Yapıştır policy valid: uyan satır yazılır, harfli değer atlanır", "parsel",
                   values((1, {"Ada": "201"}), (2, {"Ada": "12a"})), policy="valid"),
        write_case("Yapıştır: null değeri kaldırır", "parsel", values((1, {"Alan": None}))),
        write_case("Yapıştır: değer kanonik metnine çevrilir (+007 → 7, 15,5 → 15.5)", "parsel",
                   values((3, {"Ada": "+007", "Alan": "15,5"}))),
        write_case("Yapıştır: aynı değer değişiklik değildir", "parsel", values((1, {"Ada": "101"}))),
        write_case("Tekil alan: iki kaydın kodu yer değiştirebilir", "parsel", values((1, {"Kod": "A2"}), (2, {"Kod": "A1"}))),
        write_case("Tekil alan: başka kaydın değeri reddedilir", "parsel", values((2, {"Kod": "A1"}))),
        write_case("Tekil alan, policy valid: çakışan atlanır, öbürü yazılır", "parsel",
                   values((2, {"Kod": "A1"}), (3, {"Nitelik": "Bahçe"})), policy="valid"),
        plan_case("Plan: değişecekler eski ve yeni değerleriyle", "parsel", values((1, {"Ada": "201", "Alan": None}))),
        refused("olmayan katman", {"layer": "yok", "write": calc("Ad", "1")}, "layer_not_found", "“yok” kimlikli katman yok.", "layer"),
        refused("grup katman değil", {"layer": "grup", "write": calc("Ad", "1")}, "not_a_layer",
                "“Grup” bir grup; nesneleri katmanlardadır.", "layer"),
        refused("kilitli katman", {"layer": "kilitli", "write": calc("Ad", "1")}, "layer_locked",
                "“Kilitli” katmanı kilitli; önce kilidini açın.", "layer"),
        refused("satırsız yapıştırma", {"layer": "parsel", "write": {"kind": "values", "rows": []}}, "nothing_to_set",
                "Yazılacak satır yok.", "write.rows"),
        refused("değersiz satır", {"layer": "parsel", "write": values((1, {}))}, "nothing_to_set", "1. satırda değer yok.",
                "write.rows[0].values"),
        refused("adı boş alan", {"layer": "parsel", "write": calc(" ", "1")}, "invalid_attribute", "Alanın adı boş.", "write.field"),
        refused("bozuk kimlik", {"layer": "parsel", "write": {"kind": "calculate", "field": "Ada", "expression": "1", "uids": ["abc"]}},
                "invalid_uid", "“abc” bir nesne kimliği değil.", "write.uids[0]"),
        refused("çizimde olmayan nesne", {"layer": "parsel", "write": {"kind": "values", "rows": [
            {"uid": "0192f5a0-0000-7000-8000-000000000099", "values": {"Ada": "1"}}]}}, "entity_not_found",
                "“0192f5a0-0000-7000-8000-000000000099” kimlikli nesne “Parsel” katmanında yok.", "write.rows[0].uid"),
        refused("katmanın alanlarında olmayan alan (hesap)", {"layer": "parsel", "write": calc("Yok", "1")}, "field_not_found",
                "“Parsel” katmanında “Yok” alanı yok; Alanlar penceresinden ekleyin.", "write.field"),
        refused("katmanın alanlarında olmayan alan (yapıştırma)", {"layer": "parsel", "write": values((1, {"Yok": "1"}))},
                "field_not_found", "“Parsel” katmanında “Yok” alanı yok; Alanlar penceresinden ekleyin.", "write.rows[0].values.Yok"),
        refused("$ölçek alan hesabında kullanılamaz", {"layer": "parsel", "write": calc("Ada", "1 + $ölçek")}, "invalid_expression",
                "Alan hesabında $ölçek kullanılamaz: değer çizimin ölçeğine göre değişmemeli.", "write.expression"),
    ]
    return out


NOTE = ("ADR 0507. İfadenin değeri dilin bağımsız başvurusundan (expression_language.py), alana yazılan değerin denetimi ve "
        "kanonik metni alanların bağımsız başvurusundan (layer_field_cases.py'nin check'i); tekil alanlar, izleyen etiket, "
        "bütün ya da hiç ve listeleme ADR'nin kuralları. Bağımsız başvuru: scripts/fixtures/fields_write_cases.py.")


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.fields.write", "commandVersion": 1,
            "title": "Alanlara toplu yaz: Alan hesapla ve Yapıştır", "note": NOTE, "setup": SETUP, "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"Toplu yazma durumları tutarlı: {len(build()['cases'])} durum")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(build()['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
