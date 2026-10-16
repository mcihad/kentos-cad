#!/usr/bin/env python3
"""`cad.records.create` (docs/adr/0410 §3): records without a geometry, a table's rows, on a named layer, as one undo
step (“Kayıt ekle”). Each row's attributes by the layer's fields (ADR 0199 §2, the drawing commands' rule of
primitive_fields_cases.py); a blank label is not written. The refusals in their order: `invalid_count` (1 to 10 000
rows), the expected revision, the layer, each row's attributes in the rows' order.

Written from the ADR without KentOS code; the values' rules are scripts/fixtures/layer_field_cases.py's. Writes
fixtures/commands/v1/cad.records.create.json; `--check` compares instead.
"""

import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("primitive_fields_cases", ROOT / "scripts/fixtures/primitive_fields_cases.py")
pf = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pf)

OUT = ROOT / "fixtures/commands/v1/cad.records.create.json"
STEP = "Kayıt ekle"
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def layer(i, name, fields=None, locked=False, visible=True):
    out = pf.layer(i, name, fields, locked)
    out["visible"] = visible
    return out


def group(i, name, children, locked=False):
    return {"id": i, "name": name, "type": "group", "visible": True, "locked": locked, "expanded": True,
            "style": pf.STYLE, "children": children}


SETUP = copy.deepcopy(pf.SETUP)
SETUP["name"] = "Kayıtlar"
SETUP["layers"] = [
    layer("malik", "Malik", pf.FIELDS),
    layer("yapi", "Yapı"),
    layer("kilitli", "Kilitli katman", locked=True),
    layer("gizli", "Gizli katman", visible=False),
    group("plan", "Plan", [layer("ada", "Ada")]),
    group("arsiv", "Arşiv", [layer("eski", "Eski")], locked=True),
]
SETUP["activeLayer"] = "malik"
NOTHING = {"ids": [], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
FIELDS = {"malik": pf.FIELDS}
NAMES = {"malik": "Malik", "yapi": "Yapı", "kilitli": "Kilitli katman", "gizli": "Gizli katman", "eski": "Eski"}


def failed(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def hidden(name):
    return {"code": "layer_hidden", "message": f"“{name}” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}


def locked(name):
    return failed("layer_locked", f"“{name}” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId")


def record(layer_id, row, id_):
    """The record a row becomes, or the refusal: (entity, None) or (None, (code, message, path suffix))."""
    attrs, refused = pf.written(FIELDS.get(layer_id, []), row.get("attrs") or {})
    if refused:
        return None, refused
    out = {"kind": "record", "id": id_, "layerId": layer_id, "attrs": attrs}
    label = row.get("label")
    if label is not None and label.strip():
        out["label"] = label
    return out, None


def run(layer_id, rows, first=1, warnings=()):
    """The step writing `rows` on `layer_id` when the drawing has `first - 1` objects: its result and expectation."""
    made = []
    for i, row in enumerate(rows):
        e, refused = record(layer_id, row, first + i)
        if refused:
            code, message, path = refused
            return failed(code, message, f"rows.{i}.{path}"), None
        made.append(e)
    ids = [e["id"] for e in made]
    result = {"status": "completed", "output": {"uids": ["$uid"] * len(ids), "ids": ids, "revision": "$current"},
              "warnings": list(warnings)}
    return result, {"entities": {str(e["id"]): e for e in made}}


def execute(layer_id, rows, before=(), warnings=(), **extra):
    result, made = run(layer_id, rows, len(before) + 1, warnings)
    inp = {"layerId": layer_id, "rows": rows, **extra}
    if made is None:
        return {"op": "execute", "input": inp, "result": result,
                "expect": {**NOTHING, "ids": list(before), "canUndo": bool(before)}}
    ids = list(before) + [int(k) for k in made["entities"]]
    return {"op": "execute", "input": inp, "result": result,
            "expect": {"ids": ids, **made, "canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}}


def cases():
    out = []
    add = lambda name, steps, note=None: out.append({"name": name, **({"note": note} if note else {}), "steps": steps})  # noqa: E731

    add("tek kayıt alansız katmana olduğu gibi yazılır; tek adımda geri alınır, yinelenir", [
        execute("yapi", [{"attrs": {"Not": " serbest ", "x": "1"}}]),
        {"op": "undo", "returns": STEP, "expect": {"ids": [], "canUndo": False, "canRedo": True, "revision": "changed"}},
        {"op": "redo", "returns": STEP, "expect": {"ids": [1], "canUndo": True, "canRedo": False, "revision": "changed"}},
    ])
    add("birden çok kayıt sırayla ve tek adımda; geri alma hepsini kaldırır", [
        execute("yapi", [{"attrs": {"Ad": "A"}}, {"attrs": {"Ad": "B"}}, {"attrs": {"Ad": "C"}}]),
        {"op": "undo", "returns": STEP, "expect": {"ids": [], "canUndo": False, "canRedo": True}},
    ])
    add("ikinci çağrı ayrı adımdır", [
        execute("yapi", [{"attrs": {"Ad": "A"}}]),
        execute("yapi", [{"attrs": {"Ad": "B"}}], before=[1]),
        {"op": "undo", "returns": STEP, "expect": {"ids": [1], "canUndo": True, "canRedo": True}},
    ])
    add("alanlı katmanda listenin etiketi koda çevrilir, verilmeyen alana varsayılanı yazılır", [
        execute("malik", [{"attrs": {"Ad": "Çınar", "Tür": "Çalı"}}]),
    ])
    add("özniteliksiz satır alanların varsayılanlarını alır", [execute("malik", [{}])])
    add("etiket yazılır, boş etiket yazılmaz", [
        execute("yapi", [{"attrs": {"Ad": "Ayşe"}, "label": "Ayşe Demir"}, {"attrs": {"Ad": "Ali"}, "label": "  "}]),
    ])
    add("sayı, tarih ve evet/hayır tek biçimde", [
        execute("malik", [{"attrs": {"Ad": "Çınar", "Boy": "12,50", "Dikim": "5.11.2026", "Korunan": "Evet"}}]),
    ])
    add("çok değerli alan listenin sırasıyla", [
        execute("malik", [{"attrs": {"Ad": "Çınar", "Bakım": "Sulama; budama; sulama"}}]),
    ])
    add("alanı olmayan anahtar olduğu gibi kalır", [execute("malik", [{"attrs": {"Ad": "Çınar", "Not": " serbest "}}])])
    add("ikinci satırın kuralı bozan değeri bütün çağrıyı reddeder, yeri satırıyla", [
        execute("malik", [{"attrs": {"Ad": "Çınar"}}, {"attrs": {"Ad": "Meşe", "Tür": "Ağaççık"}}]),
    ])
    add("zorunlu alanın boş değeri reddedilir", [execute("malik", [{"attrs": {"Ad": " "}}])])
    add("aralık dışı sayı reddedilir", [execute("malik", [{"attrs": {"Ad": "Çınar", "Boy": "41"}}])])
    add("satırın retleri adların sırasıyla", [execute("malik", [{"attrs": {"Ad": "Çınar", "Dikim": "x", "Boy": "x"}}])])
    add("satırsız çağrı reddedilir: invalid_count", [
        {"op": "execute", "input": {"layerId": "yapi", "rows": []},
         "result": failed("invalid_count", "Kayıt sayısı 1 ile 10000 arasında olmalı.", "rows"), "expect": NOTHING},
    ])
    add("10 000'den çok satır reddedilir", [
        {"op": "execute", "input": {"layerId": "yapi", "rows": [{}] * 10_001},
         "result": failed("invalid_count", "Kayıt sayısı 1 ile 10000 arasında olmalı.", "rows"), "expect": NOTHING},
    ])
    add("sayı katmandan ve sürümden önce denetlenir", [
        {"op": "execute", "input": {"layerId": "yok", "rows": [], "expectedRevision": "12.0"},
         "result": failed("invalid_count", "Kayıt sayısı 1 ile 10000 arasında olmalı.", "rows"), "expect": NOTHING},
    ])
    add("geçersiz sürüm yazısı: invalid_revision", [
        {"op": "execute", "input": {"layerId": "yapi", "rows": [{}], "expectedRevision": "12.0"},
         "result": failed("invalid_revision", "Beklenen sürüm “12.0” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"),
         "expect": NOTHING},
    ])
    add("çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict; çakışma katmandan önce", [
        {"op": "captureRevision", "as": "r0"},
        execute("yapi", [{"attrs": {"Ad": "A"}}]),
        {"op": "execute", "input": {"layerId": "kilitli", "rows": [{}], "expectedRevision": "$r0"},
         "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT, "path": "expectedRevision", "revision": "$current"}},
         "expect": {"ids": [1], "revision": "same"}},
    ])
    add("bilinmeyen katman: layer_not_found", [
        {"op": "execute", "input": {"layerId": "Malik", "rows": [{}]},
         "result": failed("layer_not_found", "“Malik” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), "expect": NOTHING},
    ])
    add("grup katman değildir: not_a_layer", [
        {"op": "execute", "input": {"layerId": "plan", "rows": [{}]},
         "result": failed("not_a_layer", "“Plan” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"), "expect": NOTHING},
    ])
    add("kilitli katmana ve kilitli grubun katmanına yazılmaz: layer_locked", [
        {"op": "execute", "input": {"layerId": "kilitli", "rows": [{}]}, "result": locked("Kilitli katman"), "expect": NOTHING},
        {"op": "execute", "input": {"layerId": "eski", "rows": [{}]}, "result": locked("Eski"), "expect": NOTHING},
    ])
    add("katman satırların değerlerinden önce denetlenir", [
        {"op": "execute", "input": {"layerId": "kilitli", "rows": [{"attrs": {"Ad": " "}}]}, "result": locked("Kilitli katman"), "expect": NOTHING},
    ])
    add("gizli katmana uyarıyla yazılır", [execute("gizli", [{"attrs": {"Ad": "A"}}], warnings=[hidden("Gizli katman")])])
    plan_rows = [{"attrs": {"Ad": "Çınar", "Tür": "Çalı"}, "label": "Ç-1"}]
    planned, _ = run("malik", plan_rows, 0)
    planned_entity, _ = record("malik", plan_rows[0], 0)
    add("doğrulama hiçbir şey yazmaz; plan yazılacak kayıtları gösterir (yuvaları 0); planın sürümüyle yazılır", [
        {"op": "validate", "input": {"layerId": "malik", "rows": plan_rows}, "result": {"status": "completed", "output": None, "warnings": []}, "expect": NOTHING},
        {"op": "plan", "input": {"layerId": "malik", "rows": plan_rows},
         "result": {"status": "completed", "output": {"entities": [planned_entity], "revision": "$current"}, "warnings": []}, "expect": NOTHING},
        {"op": "captureRevision", "as": "plan"},
        execute("malik", plan_rows, expectedRevision="$plan"),
    ])
    add("plan da reddeder, yeri satırıyla", [
        {"op": "plan", "input": {"layerId": "malik", "rows": [{}, {"attrs": {"Korunan": "belki"}}]},
         "result": run("malik", [{}, {"attrs": {"Korunan": "belki"}}])[0], "expect": NOTHING},
    ])
    assert planned["status"] == "completed"
    return out


def main():
    data = {
        "format": "kentos.command-cases", "version": 1, "command": "cad.records.create", "commandVersion": 1,
        "title": "Geometrisiz kayıtlar",
        "note": "ADR 0410 §3: kayıtlar katmana sırayla ve tek adımda (“Kayıt ekle”) yazılır; her satırın öznitelikleri "
                "katmanın alanlarıyla. Kural ve beklenenler scripts/fixtures/records_command_cases.py'de, değerlerin kuralı "
                "layer_field_cases.py'de KentOS kodu olmadan yazılmıştır.",
        "setup": SETUP, "cases": cases(),
    }
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT.relative_to(ROOT)}: güncel değil", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum, güncel.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(data['cases'])} durum, yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
