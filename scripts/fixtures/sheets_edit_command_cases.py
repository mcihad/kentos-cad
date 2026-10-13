"""Writes fixtures/commands/v1/cad.sheets.edit.json: the shared cases of `cad.sheets.edit` v1 (docs/adr/0252 §2).

The expectations are built here from the contract's rules (crates/shared/contracts/src/cad_sheets.rs: the checks and
their order, the codes, paths and words), the sheet core's operations as its model writes them (kentos.sheet/1:
crates/shared/sheet/src/model.rs, ops.rs, docs/sheet/design.md) and docs/adr/0251 §3, without KentOS code:

- the book is the paper's: changing it makes the drawing dirty and is no undo step of it (docs/adr/0251 §3);
- the operations are applied in order as one change; the answer's label is the designer's name of the one operation
  (“Pafta ekle: <name>”, “Paftayı sil: <name>”) or “<n> işlem”; its inverse takes them all back, the last first;
- a book left empty is no book: the answer lists no sheet;
- a sheet is written here as the core keeps it, every field at its value (the model's defaults spelt out), so the
  inverse of removing it is adding it back as it was.

    python3 scripts/fixtures/sheets_edit_command_cases.py           # writes the file
    python3 scripts/fixtures/sheets_edit_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.sheets.edit.json"

INVALID_REVISION = "Beklenen sürüm “on iki” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."
NO_OPERATIONS = "Uygulanacak işlem yok: ops'a en az bir pafta işlemi verin."


def invalid_operation(n: int, why: str) -> str:
    return f"{n}. işlem pafta çekirdeğinin bir işlemi olarak okunamadı: {why}. İşlemin op adını ve alanlarını kataloğun örneğine göre denetleyin."


SETUP = {
    "format": "kentos.document", "version": 1, "name": "Paftalar",
    "settings": {"srid": 0, "lengthDecimals": 0, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg", "plotScale": 50,
                 "workspace": "cad", "drawingFont": "barlow", "drawingUnit": "mm"},
    "origin": {"x": 0, "y": 0},
    "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}],
    "activeLayer": "cizim",
    "entities": [{"kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": {"x": 0, "y": 0}, "b": {"x": 5, "y": 0}}],
    "styles": {"items": [], "categories": []},
}
IDS = [1]


def stroke(color: str, width: int) -> dict:
    return {"cap": "butt", "color": color, "dash": [], "join": "miter", "width": width}


def sheet(sid: str, name: str, scale: int) -> dict:
    """An A3 landscape sheet with a map frame at 1:scale, every field as the core keeps it: the item's shared
    properties at their defaults (constrained to the margins' top left, shown, printable, opaque), the map's (all
    layers, no grid, the overview frame's red, no label band), the sheet's (no guide, a 5 mm snap grid off, the
    export's 300 dpi SVG named after the sheet, no variable or variant)."""
    return {
        "export": {"dpi": 300, "fileName": "[% @pafta_adi %]", "format": "svg"},
        "guides": [],
        "id": sid,
        "items": [{
            "bindings": [],
            "constraints": {"h": "left", "relativeTo": "margins", "v": "top"},
            "frame": {"height": 230000, "left": 20000, "top": 10000, "width": 390000},
            "hidden": False,
            "id": f"{sid}-harita",
            "kind": {"clipToAtlasFeature": False, "grids": [], "labelBand": 0, "layers": {"type": "all"},
                     "overviewFrame": stroke("#d0021b", 500), "type": "map",
                     "view": {"center": {"x": 2.5, "y": 0.0}, "rotation": 0, "scale": scale, "type": "fixed"}},
            "locked": False, "name": "Harita", "opacity": 100, "padding": 0, "printable": True, "rotation": 0,
        }],
        "name": name,
        "page": {"margins": {"bottom": 10000, "left": 20000, "right": 10000, "top": 10000}, "orientation": "landscape",
                 "paper": "a3", "size": {"height": 297000, "width": 420000}},
        "snapGrid": {"enabled": False, "spacing": 5000, "visible": False},
        "variables": [],
        "variants": [],
    }


def listed(s: dict) -> dict:
    """A sheet as the answer lists it: its page in millimetres, its items' count, its maps with their scales."""
    maps = [{"item": i["id"], "name": i["name"], "scale": i["kind"]["view"]["scale"]}
            for i in s["items"] if i["kind"]["type"] == "map"]
    return {"height": s["page"]["size"]["height"] / 1000, "id": s["id"], "items": len(s["items"]), "maps": maps,
            "name": s["name"], "width": s["page"]["size"]["width"] / 1000}


PLAN = sheet("kat-plani", "Kat planı", 50)
SITE = sheet("vaziyet", "Vaziyet", 100)


def done(label: str, inverse: list, sheets: list) -> dict:
    return {"status": "completed", "output": {"inverse": inverse, "label": label, "revision": "$current", "sheets": sheets},
            "warnings": []}


def failed(code: str, message: str, path: str | None) -> dict:
    error = {"code": code, "message": message}
    if path is not None:
        error["path"] = path
    return {"status": "failed", "error": error}


UNCHANGED = {"ids": IDS, "canUndo": False, "dirty": False, "revision": "same", "sheets": []}


def cases() -> list:
    renamed = copy.deepcopy(PLAN)
    renamed["name"] = "Plan"
    return [
        {"name": "Kitapsız projeye pafta: kitap olur, çizim kirlenir, geri alma adımı değildir", "steps": [{
            "op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}]},
            "result": done("Pafta ekle: Kat planı", [{"id": "kat-plani", "op": "removeSheet"}], [listed(PLAN)]),
            "expect": {"ids": IDS, "canUndo": False, "dirty": True, "revision": "changed", "sheets": [listed(PLAN)]},
        }]},
        {"name": "İki pafta tek değişiklik; ters işlemler sondan başa", "steps": [{
            "op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}, {"op": "addSheet", "sheet": SITE}]},
            "result": done("2 işlem", [{"id": "vaziyet", "op": "removeSheet"}, {"id": "kat-plani", "op": "removeSheet"}],
                           [listed(PLAN), listed(SITE)]),
            "expect": {"sheets": [listed(PLAN), listed(SITE)], "canUndo": False},
        }]},
        {"name": "Ekle ve adlandır: tersi önce eski adı verir, sonra paftayı siler", "steps": [{
            "op": "execute",
            "input": {"ops": [{"op": "addSheet", "sheet": PLAN}, {"op": "renameSheet", "id": "kat-plani", "name": "Plan"}]},
            "result": done("2 işlem", [{"id": "kat-plani", "name": "Kat planı", "op": "renameSheet"},
                                       {"id": "kat-plani", "op": "removeSheet"}], [listed(renamed)]),
            "expect": {"sheets": [listed(renamed)]},
        }]},
        {"name": "Son pafta silinince kitap kalmaz; tersi paftayı olduğu gibi geri ekler", "steps": [
            {"op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}]},
             "result": done("Pafta ekle: Kat planı", [{"id": "kat-plani", "op": "removeSheet"}], [listed(PLAN)])},
            {"op": "execute", "input": {"ops": [{"op": "removeSheet", "id": "kat-plani"}]},
             "result": done("Paftayı sil: Kat planı", [{"index": 0, "op": "addSheet", "sheet": PLAN}], []),
             "expect": {"sheets": [], "dirty": True}},
            {"op": "execute", "input": {"ops": [{"index": 0, "op": "addSheet", "sheet": PLAN}]},
             "result": done("Pafta ekle: Kat planı", [{"id": "kat-plani", "op": "removeSheet"}], [listed(PLAN)]),
             "expect": {"sheets": [listed(PLAN)]}},
        ]},
        {"name": "Plan: kitabı yazmaz", "steps": [{
            "op": "plan", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}]},
            "result": done("Pafta ekle: Kat planı", [{"id": "kat-plani", "op": "removeSheet"}], [listed(PLAN)]),
            "expect": UNCHANGED,
        }]},
        {"name": "Doğrula: yazmaz", "steps": [{
            "op": "validate", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}]},
            "result": {"status": "completed", "output": None, "warnings": []},
            "expect": UNCHANGED,
        }]},
        {"name": "İşlem yok", "steps": [{
            "op": "execute", "input": {"ops": []},
            "result": failed("no_operations", NO_OPERATIONS, "ops"), "expect": UNCHANGED,
        }]},
        {"name": "Okunamayan işlem: alanı eksik", "steps": [{
            "op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}, {"op": "removeSheet"}]},
            "result": failed("invalid_operation", invalid_operation(2, "missing field `id`"), "ops[1]"),
            "expect": UNCHANGED,
        }]},
        {"name": "Çekirdeğin reddi: kitapta olmayan pafta, hiçbir şey yazılmaz", "steps": [{
            "op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}, {"op": "removeSheet", "id": "yok"}]},
            "result": failed("unknown_sheet", "“yok” paftası kitapta yok.", "ops[1]"),
            "expect": UNCHANGED,
        }]},
        {"name": "geçersiz beklenen sürüm", "steps": [{
            "op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}], "expectedRevision": "on iki"},
            "result": failed("invalid_revision", INVALID_REVISION, "expectedRevision"),
            "expect": UNCHANGED,
        }]},
        {"name": "sürüm çakışması: hiçbir şey yazılmaz", "steps": [
            {"op": "captureRevision", "as": "once"},
            {"op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": SITE}]},
             "result": done("Pafta ekle: Vaziyet", [{"id": "vaziyet", "op": "removeSheet"}], [listed(SITE)]),
             "expect": {"revision": "changed"}},
            {"op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}], "expectedRevision": "$once"},
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT,
                                                        "path": "expectedRevision", "revision": "$current"}},
             "expect": {"revision": "same", "sheets": [listed(SITE)]}},
        ]},
    ]


def item(sid: str, iid: str, name: str, scale: int) -> dict:
    """The sheet's map frame `iid` as the core keeps it (see :func:`sheet`)."""
    it = copy.deepcopy(sheet(sid, "", scale)["items"][0])
    it["id"], it["name"] = iid, name
    return it


def more_cases() -> list:
    """What the operations do to a book that has sheets: the inverse is the core's own undo of each."""
    add_plan = {"op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}]},
                "result": done("Pafta ekle: Kat planı", [{"id": "kat-plani", "op": "removeSheet"}], [listed(PLAN)])}
    add_both = {"op": "execute", "input": {"ops": [{"op": "addSheet", "sheet": PLAN}, {"op": "addSheet", "sheet": SITE}]},
                "result": done("2 işlem", [{"id": "vaziyet", "op": "removeSheet"}, {"id": "kat-plani", "op": "removeSheet"}],
                               [listed(PLAN), listed(SITE)])}
    renamed = copy.deepcopy(PLAN)
    renamed["name"] = "Zemin kat"
    second = item("kat-plani", "kat-plani-detay", "Detay", 20)
    with_detail = copy.deepcopy(PLAN)
    with_detail["items"].append(second)
    hidden = copy.deepcopy(PLAN)
    hidden["items"][0]["hidden"] = True
    return [
        {"name": "Paftaya ad ver: tersi eski adı verir", "steps": [add_plan, {
            "op": "execute", "input": {"ops": [{"op": "renameSheet", "id": "kat-plani", "name": "Zemin kat"}]},
            "result": done("Paftaya ad ver: Zemin kat", [{"id": "kat-plani", "name": "Kat planı", "op": "renameSheet"}],
                           [listed(renamed)]),
            "expect": {"sheets": [listed(renamed)], "canUndo": False},
        }]},
        {"name": "Paftayı taşı: kitabın sırası değişir, tersi eski yerine koyar", "steps": [add_both, {
            "op": "execute", "input": {"ops": [{"op": "moveSheet", "id": "vaziyet", "to": 0}]},
            "result": done("Paftayı taşı: Vaziyet", [{"id": "vaziyet", "op": "moveSheet", "to": 1}], [listed(SITE), listed(PLAN)]),
            "expect": {"sheets": [listed(SITE), listed(PLAN)]},
        }]},
        {"name": "Paftaya ikinci harita: sayılır, ölçeğiyle listelenir; tersi onu siler", "steps": [add_plan, {
            "op": "execute", "input": {"ops": [{"op": "addItems", "to": {"kind": "sheet", "id": "kat-plani"}, "items": [second]}]},
            "result": done("Ekle: Detay", [{"ids": ["kat-plani-detay"], "op": "removeItems"}], [listed(with_detail)]),
            "expect": {"sheets": [listed(with_detail)]},
        }]},
        {"name": "Haritayı gizle: tersi yeniden gösterir", "steps": [add_plan, {
            "op": "execute", "input": {"ops": [{"op": "hide", "ids": ["kat-plani-harita"], "value": True}]},
            "result": done("Gizle: Harita", [{"ids": ["kat-plani-harita"], "op": "hide", "value": False}], [listed(hidden)]),
            "expect": {"sheets": [listed(hidden)]},
        }]},
        {"name": "Kitaplı projede ikinci işlem reddedilince ilki de yazılmaz", "steps": [add_plan, {
            "op": "execute", "input": {"ops": [{"op": "renameSheet", "id": "kat-plani", "name": "Zemin kat"},
                                               {"op": "removeSheet", "id": "yok"}]},
            "result": failed("unknown_sheet", "“yok” paftası kitapta yok.", "ops[1]"),
            "expect": {"sheets": [listed(PLAN)], "revision": "same"},
        }]},
        {"name": "Kitaplı projede plan: kitabı yazmaz, sürüm aynı", "steps": [add_plan, {
            "op": "plan", "input": {"ops": [{"op": "removeSheet", "id": "kat-plani"}]},
            "result": done("Paftayı sil: Kat planı", [{"index": 0, "op": "addSheet", "sheet": PLAN}], []),
            "expect": {"sheets": [listed(PLAN)], "revision": "same"},
        }]},
        {"name": "Okunamayan işlem: alanın türü yanlış", "steps": [{
            "op": "execute", "input": {"ops": [{"op": "renameSheet", "id": "kat-plani", "name": 5}]},
            "result": failed("invalid_operation", invalid_operation(1, "invalid type: integer `5`, expected a string"), "ops[0]"),
            "expect": UNCHANGED,
        }]},
        {"name": "Okunamayan işlem: işlemin adı yok", "steps": [{
            "op": "execute", "input": {"ops": [{"id": "kat-plani"}]},
            "result": failed("invalid_operation", invalid_operation(1, "missing field `op`"), "ops[0]"),
            "expect": UNCHANGED,
        }]},
        {"name": "Gizlenen öğe kitapta yok: çekirdeğin reddi yoluyla", "steps": [add_plan, {
            "op": "execute", "input": {"ops": [{"op": "hide", "ids": ["yok"], "value": True}]},
            "result": failed("unknown_item", "“yok” öğesi kitapta yok.", "ops[0]"),
            "expect": {"sheets": [listed(PLAN)], "revision": "same"},
        }]},
    ]


def build() -> dict:
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.sheets.edit",
        "commandVersion": 1,
        "title": "Pafta kitabı",
        "note": "Written by scripts/fixtures/sheets_edit_command_cases.py from the contract's rules, the sheet core's "
                "model and docs/adr/0251 §3, 0252 §2; do not edit by hand.",
        "setup": SETUP,
        "cases": cases() + more_cases(),
    }


def main() -> int:
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            print(f"{OUT.relative_to(ROOT)} güncel değil: python3 {Path(__file__).relative_to(ROOT)}", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)}: {len(build()['cases'])} durum tutarlı.")
        return 0
    OUT.write_text(text, encoding="utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
