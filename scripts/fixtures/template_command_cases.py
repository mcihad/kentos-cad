#!/usr/bin/env python3
"""`cad.templates.apply`'s shared cases (docs/adr/0346 §1), from the ADR and the template's data, without KentOS code.

Writes fixtures/commands/v1/cad.templates.apply.json; `--check` compares and writes nothing. The rules:

- the name trimmed and lowercased is a KentOS template (`mimari`), else `unknown_template`; a unit given is mm or cm,
  else `invalid_template_unit`; then the expected revision;
- the standard's layers and styles as `cad.standards.apply` adds them with `restyle` and `rename` off (the standard
  apply's own reference, `standards_cases.py`); the library's blocks whose names the drawing has not, in the library's
  order; one undo step “Mimari şablonu”;
- the unit (the call's, else the template's) written to a drawing without objects; a drawing with objects keeps its
  own, said by `template_unit_kept` when they differ; the plot scale the template's; its levels the drawing has not (by
  id or name), in order; its space method when the drawing sets none; KentOS's wall styles the project has not;
- its sheets the book has not, by id, in order.
"""

import argparse
import copy
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.templates.apply.json"
COMMAND = "cad.templates.apply"


def _load(name):
    spec = importlib.util.spec_from_file_location(name, Path(__file__).with_name(f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


S = _load("standards_cases")
TEMPLATE = json.loads((ROOT / "catalog/v1/templates/mimari.json").read_text(encoding="utf-8"))
STANDARD = S.BUILT_IN[TEMPLATE["standard"]]
LIBRARY = [b["name"] for b in json.loads((ROOT / "catalog/v1/blocks/kentos-temel.kblok").read_text(encoding="utf-8"))["blocks"]]
# KentOS's wall styles (docs/adr/0339 §3), by name, in their order.
WALL_STYLES = ["Dış duvar 25", "İç duvar 15", "Gazbeton 20", "Betonarme perde 25", "Bölme duvar 10"]
INVALID_REVISION = ("Beklenen sürüm “07” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da "
                    "komutun planından alın.")
CONFLICT = ("Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden "
            "hazırlayın.")


def layer(id, name, color="fg", line_type="continuous", weight=0.25):
    return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": line_type, "lineWeight": weight}, "children": []}


BLOCK = {"id": "0192f5b0-0000-7000-8000-0000000000aa", "name": "Masa", "base": {"x": 0.0, "y": 0.0},
         "entities": [{"kind": "line", "id": 1, "layerId": "", "attrs": {}, "a": {"x": 0.0, "y": 0.0}, "b": {"x": 1.2, "y": 0.0}}]}


def setup(objects=False, unit=None, levels=None, method=None, text_styles=None, blocks=False, layers=None):
    settings = {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 100,
                "workspace": "cad"}
    if unit:
        settings["drawingUnit"] = unit
    arch = {}
    if levels:
        arch["levels"] = levels
    if method:
        arch["spaceMethod"] = method
    if arch:
        settings["arch"] = arch
    if text_styles:
        settings["textStyles"] = text_styles
    doc = {"format": "kentos.document", "version": 1, "name": "Şablon", "settings": settings, "origin": {"x": 0, "y": 0},
           "layers": layers or [layer("cizim", "Çizim")], "activeLayer": (layers or [layer("cizim", "Çizim")])[0]["id"],
           "styles": {"items": [], "categories": []},
           "entities": [{"kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 5, "y": 0}}] if objects else []}
    if blocks:
        doc["blocks"] = [BLOCK]
    return doc


def applied(doc, inp, sheets_have=()):
    """The answer and warnings: what the template adds to `doc`."""
    s = doc["settings"]
    tree, _, _, sets, answer = S.apply(STANDARD, copy.deepcopy(doc["layers"]), [], doc["activeLayer"], copy.deepcopy(s),
                                       layers_=True, restyle=False, rename=False, styles=True)
    names = {n["id"]: n["name"] for n in S.leaves(tree)}
    layers = [names[i] for i in answer["created"]]
    styles = answer["textStyles"] + answer["dimensionStyles"]
    have = {b["name"] for b in doc.get("blocks", [])}
    blocks = [n for n in LIBRARY if n not in have]
    warnings = []
    unit_now = s.get("drawingUnit", "m")
    unit = inp.get("unit", TEMPLATE["unit"])
    if unit_now != unit:
        if doc["entities"]:
            warnings.append({"code": "template_unit_kept",
                             "message": f"Çizimde nesne var; birimi ({unit_now}) değişmedi. Birimi Proje ayarları ya da Birimi değiştir ile değiştirin.",
                             "path": "unit"})
        else:
            unit_now = unit
    levels_have = s.get("arch", {}).get("levels", [])
    levels = [l["name"] for l in TEMPLATE["levels"] if not any(x["id"] == l["id"] or x["name"] == l["name"] for x in levels_have)]
    sheets = [sh["name"] for sh in TEMPLATE["sheets"] if sh["id"] not in sheets_have]
    out = {"unit": unit_now, "layers": layers, "styles": styles, "blocks": blocks, "wallStyles": list(WALL_STYLES),
           "levels": levels, "sheets": sheets, "revision": "$current"}
    return out, warnings


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def step(op, inp, doc, sheets_have=(), result=None, expect=None):
    s = {"op": op, "command": COMMAND, "input": inp}
    if result is not None:
        s["result"] = result
        s["expect"] = expect or {"revision": "same"}
        return s
    out, warnings = applied(doc, inp, sheets_have)
    if op == "validate":
        s["result"] = {"status": "completed", "output": None, "warnings": []}
        s["expect"] = {"revision": "same"}
    elif op == "plan":
        s["result"] = {"status": "completed", "output": out, "warnings": warnings}
        s["expect"] = {"revision": "same"}
    else:
        s["result"] = {"status": "completed", "output": out, "warnings": warnings}
        s["expect"] = expect or {"revision": "changed", "dirty": True}
    return s


def sheet_prelude(ids):
    """The book with the template's own sheets `ids` already in it, through `cad.sheets.edit`."""
    import viewmark_command_cases as vm  # noqa: E402  (a sheet as the sheet core keeps it)
    return {"op": "execute", "command": "cad.sheets.edit", "note": "kitapta var olan pafta",
            "input": {"ops": [{"op": "addSheet", "sheet": vm.sheet(i, f"Var olan {i}", [(f"{i}-h", "Harita")])} for i in ids]},
            "result": {"status": "completed", "output": "$any", "warnings": "$any"}, "expect": {"revision": "changed"}}


def cases():
    out = []

    def add(name, steps, doc):
        assert name not in [c["name"] for c in out], name
        out.append({"name": name, "setup": doc, "steps": steps})

    e = setup()
    add("Boş çizime: santimetre, katmanlar, stiller, bloklar, duvar stilleri, katlar, paftalar", [step("execute", {"name": "mimari"}, e)], e)
    add("Boş çizime milimetreyle", [step("execute", {"name": "mimari", "unit": "mm"}, e)], e)
    add("Ad kırpılır ve büyük harfi sayılmaz", [step("execute", {"name": "  Mimari "}, e)], e)
    add("Plan yazmadan söyler", [step("plan", {"name": "mimari"}, e)], e)
    add("Doğrula yazmaz", [step("validate", {"name": "mimari"}, e)], e)
    o = setup(objects=True)
    add("Nesneli çizimin birimi korunur ve söylenir", [step("execute", {"name": "mimari"}, o)], o)
    add("Nesneli çizimin planı da birimi korur", [step("plan", {"name": "mimari", "unit": "mm"}, o)], o)
    c = setup(objects=True, unit="cm")
    add("Birimi zaten santimetre olan nesneli çizimde uyarı yok", [step("execute", {"name": "mimari"}, c)], c)
    k = setup(layers=[layer("duvar", "A-DUVAR", "#E5484D", "continuous", 0.5), layer("cizim", "Çizim")])
    add("Var olan standart katman korunur, görünüşü değişmez", [step("execute", {"name": "mimari"}, k)], k)
    b = setup(blocks=True)
    add("Var olan blok korunur, kitaplığın öbürleri eklenir", [step("execute", {"name": "mimari"}, b)], b)
    lv = setup(levels=[{"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.2}])
    add("Var olan kat korunur, eksik kat eklenir", [step("execute", {"name": "mimari"}, lv)], lv)
    ln = setup(levels=[{"id": "k1", "name": "1. kat", "elevation": 3.2}])
    add("Aynı adlı kat kimliği başka olsa da var sayılır", [step("execute", {"name": "mimari"}, ln)], ln)
    m = setup(levels=[{"id": "zemin", "name": "Zemin kat", "elevation": 0.0}], method="gross")
    add("Projenin mahal yöntemi korunur", [step("execute", {"name": "mimari"}, m)], m)
    ts = setup(text_styles=[{"id": "0192f5a0-0000-7000-8000-000000000001", "name": "A-Yazı", "font": "barlow", "height": 2.5}])
    add("Var olan yazı stili korunur", [step("execute", {"name": "mimari"}, ts)], ts)
    first = step("execute", {"name": "mimari"}, e)
    twice = {"op": "execute", "command": COMMAND, "input": {"name": "mimari"},
             "result": {"status": "completed", "output": {"unit": "cm", "layers": [], "styles": [], "blocks": [], "wallStyles": [],
                                                          "levels": [], "sheets": [], "revision": "$current"}, "warnings": []},
             "expect": {"revision": "same"}}
    add("İkinci kez uygulamak bir şey eklemez", [first, twice], e)
    add("Kitapta olan pafta korunur, öbürleri eklenir",
        [sheet_prelude(["a-101"]), step("execute", {"name": "mimari"}, e, sheets_have=("a-101",))], e)
    add("Kitapta hepsi varsa pafta eklenmez",
        [sheet_prelude(["a-101", "a-301", "a-501"]), step("execute", {"name": "mimari"}, e, sheets_have=("a-101", "a-301", "a-501"))], e)
    add("Geri almada katmanlar ve bloklar gider, ayarlar ve paftalar kalır",
        [step("execute", {"name": "mimari"}, e), {"op": "undo", "returns": "Mimari şablonu", "expect": {"canRedo": True, "revision": "changed"}}], e)
    add("Bilinmeyen şablon reddedilir", [step("execute", {"name": "peyzaj"}, e, result=refusal(
        "unknown_template", "“peyzaj” adlı çalışma şablonu yok; KentOS'un şablonu: mimari.", "name"))], e)
    add("Metre şablonun birimi olamaz", [step("execute", {"name": "mimari", "unit": "m"}, e, result=refusal(
        "invalid_template_unit", "Şablonun birimi mm ya da cm olur.", "unit"))], e)
    add("Ayak şablonun birimi olamaz", [step("execute", {"name": "mimari", "unit": "ft"}, e, result=refusal(
        "invalid_template_unit", "Şablonun birimi mm ya da cm olur.", "unit"))], e)
    add("Geçersiz beklenen sürüm reddedilir", [step("execute", {"name": "mimari", "expectedRevision": "07"}, e, result=refusal(
        "invalid_revision", INVALID_REVISION, "expectedRevision"))], e)
    stale = refusal("revision_conflict", CONFLICT, "expectedRevision", "conflict")
    stale["error"]["revision"] = "$current"
    add("Sürüm değişince yazılmaz (çakışma)",
        [{"op": "captureRevision", "as": "prepared"}, step("execute", {"name": "mimari"}, e),
         step("execute", {"name": "mimari", "expectedRevision": "$prepared"}, e, result=stale)], e)
    return out


def doc():
    return {"format": "kentos.command-cases", "version": 1, "command": COMMAND, "commandVersion": 1, "title": "Mimari şablonu",
            "note": "scripts/fixtures/template_command_cases.py yazar; ADR 0346'nın kurallarından ve şablonun verisinden, KentOS kodu olmadan.",
            "setup": setup(), "cases": cases()}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    d = doc()
    assert len(d["cases"]) >= 20, len(d["cases"])
    text = json.dumps(d, ensure_ascii=False, indent=2) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"{COMMAND}: {len(d['cases'])} durum başvuruyla aynı.")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(d['cases'])} durum yazıldı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
