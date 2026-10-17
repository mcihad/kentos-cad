#!/usr/bin/env python3
"""Shared cases of cad.processing.run and cad.processing.tools (docs/adr/0508), without KentOS code.

What a tool computes is not worked out here a second time: the expectations of `cad.processing.run` are İşlemler's
own shared cases (fixtures/processing/v1/cases.json and queries.json, each checked against its independent
reference), restated in the command's words — the selection as persistent ids, a features output's slots as
persistent ids, the run's messages as notes, the undo step as the step, the objects it changed and made by slot.
The cases the command adds are its own rules: refusals in order, the features input by ids, the input's `selection`
and `view`, a plan that writes nothing, undo.

The expectations of `cad.processing.tools` come from the web's frozen inventory (docs/inventory/web.json, written
from the web's TypeScript definitions of the same tools at 1aad00d0): each tool's id, name, aliases and parameters
(name, label, kind, whether optional), in order.

  python3 scripts/fixtures/processing_command_cases.py          # writes the two files
  python3 scripts/fixtures/processing_command_cases.py --check  # fails when they differ
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROCESSING = ROOT / "fixtures/processing/v1"
OUT = ROOT / "fixtures/commands/v1"
INVENTORY = ROOT / "docs/inventory/web.json"

INVENTORY_TOOLS = {t["id"]: t for t in json.loads(INVENTORY.read_text(encoding="utf-8"))["processing"]}


def document(name: str) -> dict:
    return json.loads((PROCESSING / name).read_text(encoding="utf-8"))


def uid(slot: int) -> str:
    return f"$uidOf:{slot}"


def features_kinds(tool: str) -> set[str]:
    """The names of a tool's features outputs, as the web's definitions give them."""
    t = INVENTORY_TOOLS.get(tool)
    if t is None:
        return set()
    return {o["name"] for o in t.get("outputs", []) if o.get("type") == "features"}


def restated_value(v):
    """A parameter's value in the command's words: a features value names its objects by persistent id; a text
    that begins with “$” is written “$$” (the cases' placeholders begin with one)."""
    if isinstance(v, str) and v.startswith("$"):
        return "$" + v
    if isinstance(v, dict) and v.get("scope") == "ids":
        out = {k: x for k, x in v.items() if k != "ids"}
        out["scope"] = "uids"
        out["uids"] = [uid(n) for n in v["ids"]]
        return out
    return v


def run_case(c: dict, fixture_doc: str) -> dict:
    """One of İşlemler's cases as a cad.processing.run case."""
    tool = c["run"]["tool"]
    doc = document(c["document"])
    inp: dict = {"tool": tool}
    if c.get("values"):
        inp["parameters"] = {k: restated_value(v) for k, v in c["values"].items()}
    if c.get("selection"):
        inp["selection"] = [uid(n) for n in c["selection"]]
    if c.get("view"):
        inp["view"] = c["view"]
    want = c["expect"]
    step: dict = {"op": "execute", "input": inp}
    expect: dict = {}
    if want["status"] == "ok":
        output: dict = {"tool": tool, "summary": want["summary"]}
        added = want.get("added") or want.get("addedShapes") or []
        if added:
            output["added"] = ["$any"] * len(added)
        updated = want.get("updated") or []
        if updated:
            output["changed"] = [uid(u["id"]) for u in updated]
        selection = want.get("selection") or []
        if selection:
            output["selected"] = [uid(n) for n in selection]
        layers = sorted([l["id"] for l in want.get("layers") or []] + [g["id"] for g in want.get("groups") or []])
        if layers:
            output["layers"] = layers
        outputs = want.get("outputs") or {}
        if outputs:
            kinds = features_kinds(tool)
            output["outputs"] = {
                k: ([uid(n) for n in v] if k in kinds and isinstance(v, list) else v) for k, v in outputs.items()
            }
        if want.get("log"):
            output["notes"] = want["log"]
        if want.get("undo"):
            output["step"] = want["undo"]
        output["revision"] = "$current"
        step["result"] = {"status": "completed", "output": output, "warnings": []}
        expect["canUndo"] = bool(want.get("undo"))
        expect["revision"] = "changed" if want.get("undo") else "same"
        fields: dict = {}
        for u in updated:
            f = {"attrs": u["attrs"]}
            if "label" in u:
                f["label"] = u["label"]
            fields[str(u["id"])] = f
        nxt = max(e["id"] for e in doc["entities"]) + 1
        for i, a in enumerate(want.get("added") or []):
            fields[str(nxt + i)] = {k: v for k, v in a.items() if k in ("kind", "layerId", "label", "attrs", "p")}
        if fields:
            expect["fields"] = fields
        nodes = {l["id"]: {"name": l["name"]} for l in want.get("layers") or []}
        if nodes:
            expect["nodes"] = nodes
    elif want["status"] == "invalid":
        issue = want["issues"][0]
        error = {"code": "invalid_parameters", "message": issue["message"]}
        if issue.get("param"):
            error["path"] = f"parameters.{issue['param']}"
        step["result"] = {"status": "failed", "error": error}
        expect = {"canUndo": False, "revision": "same"}
    else:
        step["result"] = {
            "status": "failed",
            "error": {"code": "tool_failed", "message": want["message"], "path": "tool"},
        }
        expect = {"canUndo": False, "revision": "same"}
    step["expect"] = expect
    case = {"name": f"İşlemler: {c['title']} ({c['id']})", "steps": [step]}
    if c["document"] != fixture_doc:
        case["setup"] = doc
    if want["status"] == "ok" and want.get("undo"):
        case["steps"].append(
            {"op": "undo", "returns": want["undo"], "expect": {"canUndo": False, "canRedo": True, "revision": "changed"}}
        )
    return case


def usable(c: dict) -> bool:
    """A case whose expectations say the whole result: a tool's run (not a model's) whose outputs, when the tool
    has any, are written out (a case may leave them to the tool's own tests)."""
    if "tool" not in c["run"]:
        return False
    want = c["expect"]
    if want["status"] != "ok":
        return True
    declared = INVENTORY_TOOLS.get(c["run"]["tool"], {}).get("outputs", [])
    return not declared or set(want.get("outputs") or {}) == {o["name"] for o in declared}


def failed(code: str, message: str, path: str) -> dict:
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


UNCHANGED = {"canUndo": False, "revision": "same"}

# The command's own rules.
OWN_RUN = [
    {
        "name": "Bilinmeyen araç: unknown_tool, aracın yolu",
        "steps": [
            {
                "op": "execute",
                "input": {"tool": "points.yok"},
                "result": failed(
                    "unknown_tool",
                    "“points.yok” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.",
                    "tool",
                ),
                "expect": UNCHANGED,
            }
        ],
    },
    {
        "name": "Seçimde UUID olmayan kimlik: invalid_uid, yeriyle",
        "steps": [
            {
                "op": "execute",
                "input": {"tool": "attributes.calculate", "selection": ["$uidOf:1", "parsel-2"]},
                "result": failed("invalid_uid", "“parsel-2” bir nesne kimliği değil (UUID olmalı).", "selection[1]"),
                "expect": UNCHANGED,
            }
        ],
    },
    {
        "name": "Seçimde çizimde olmayan nesne: entity_not_found",
        "steps": [
            {
                "op": "execute",
                "input": {"tool": "attributes.calculate", "selection": ["00000000-0000-4000-8000-000000000000"]},
                "result": failed(
                    "entity_not_found",
                    "00000000-0000-4000-8000-000000000000 kimlikli nesne çizimde yok.",
                    "selection[0]",
                ),
                "expect": UNCHANGED,
            }
        ],
    },
    {
        "name": "Aracın olmayan parametresi: invalid_parameters, parametreleri sayılarak",
        "steps": [
            {
                "op": "execute",
                "input": {"tool": "attributes.calculate", "parameters": {"alan": "Hesap"}, "selection": ["$uidOf:1"]},
                "result": failed(
                    "invalid_parameters",
                    "“Öznitelik hesapla” aracında “alan” parametresi yok; parametreleri: "
                    + ", ".join(p["name"] for p in INVENTORY_TOOLS["attributes.calculate"]["parameters"])
                    + ".",
                    "parameters.alan",
                ),
                "expect": UNCHANGED,
            }
        ],
    },
    {
        "name": "Görünen alandakiler, girdide view yokken: invalid_parameters",
        "steps": [
            {
                "op": "execute",
                "input": {"tool": "selection.byExpression", "parameters": {"input": {"scope": "visible"}}},
                "result": failed(
                    "invalid_parameters",
                    "“input”: görünen alandakiler için girdide view (kutu) verilmeli.",
                    "parameters.input",
                ),
                "expect": UNCHANGED,
            }
        ],
    },
    {
        "name": "Nesneler yuvalarıyla verilemez: kalıcı kimlik istenir",
        "steps": [
            {
                "op": "execute",
                "input": {"tool": "attributes.calculate", "parameters": {"input": {"scope": "ids", "ids": [1]}}},
                "result": failed(
                    "invalid_parameters",
                    "“input”: nesneler kalıcı kimlikleriyle verilir ({ \"scope\": \"uids\", \"uids\": [ … ] }).",
                    "parameters.input",
                ),
                "expect": UNCHANGED,
            }
        ],
    },
    {
        "name": "Nesne girdisinde çizimde olmayan kimlik: entity_not_found, parametrenin yeriyle",
        "steps": [
            {
                "op": "execute",
                "input": {
                    "tool": "attributes.calculate",
                    "parameters": {"input": {"scope": "uids", "uids": ["00000000-0000-4000-8000-000000000001"]}},
                },
                "result": failed(
                    "entity_not_found",
                    "00000000-0000-4000-8000-000000000001 kimlikli nesne çizimde yok.",
                    "parameters.input.uids[0]",
                ),
                "expect": UNCHANGED,
            }
        ],
    },
]


def own_run_from(cases: dict) -> list[dict]:
    """Cases the command adds on İşlemler's: the same runs with the objects named another way, and a plan."""
    out = []
    area = cases["calculate-area"]["expect"]
    one = next(u for u in area["updated"] if u["id"] == 1)
    out.append(
        {
            "name": "Öznitelik hesapla, nesneler kalıcı kimlikleriyle (scope uids): yalnız adı verilen yazılır",
            "steps": [
                {
                    "op": "execute",
                    "input": {
                        "tool": "attributes.calculate",
                        "parameters": {"input": {"scope": "uids", "uids": ["$uidOf:1"]}},
                    },
                    "result": {
                        "status": "completed",
                        "output": {
                            "tool": "attributes.calculate",
                            "summary": area["summary"].replace("3 nesnede", "1 nesnede"),
                            "changed": ["$uidOf:1"],
                            "outputs": {"count": 1, "changed": ["$uidOf:1"]},
                            "step": area["undo"],
                            "revision": "$current",
                        },
                        "warnings": [],
                    },
                    "expect": {
                        "canUndo": True,
                        "revision": "changed",
                        "fields": {"1": {"attrs": one["attrs"], "label": one["label"]}},
                    },
                }
            ],
        }
    )
    nv = cases["number-vertices"]
    run = run_case(nv, "parcels.kcad")
    planned = json.loads(json.dumps(run["steps"][0]))
    planned["op"] = "plan"
    planned["expect"] = {"canUndo": False, "dirty": False, "revision": "same", "ids": list(range(1, 11))}
    out.append(
        {
            "name": "Köşe noktalarını numarala, plan: aynı sonuç bir kopyada, çizim değişmez",
            "steps": [planned],
        }
    )
    validated = {
        "op": "validate",
        "input": run["steps"][0]["input"],
        "result": {"status": "completed", "output": None, "warnings": []},
        "expect": UNCHANGED,
    }
    out.append({"name": "Köşe noktalarını numarala, validate: değerler uyuyor, yazılmaz", "steps": [validated]})
    return out


# The tools a listing case asks for, by id or alias, and how.
LISTED = [
    ("points.numberVertices", "id"),
    ("attributes.calculate", "id"),
    ("selection.byExpression", "id"),
    ("annotation.edgeLengths", "id"),
    ("annotation.edgeLengths", "alias"),
    ("points.numberVertices", "alias"),
    ("attributes.calculate", "alias"),
    ("selection.byExpression", "alias"),
    ("attributes.joinByField", "id"),
    ("attributes.fromInside", "id"),
    ("proximity.nearest", "id"),
    ("sample.grid", "id"),
    ("geometry.clip", "id"),
    ("geometry.dissolve", "id"),
    ("geometry.validity", "id"),
    ("geometry.simplify", "id"),
    ("layers.splitByAttribute", "id"),
    ("layers.deleteDuplicates", "id"),
    ("convert.toPoints", "id"),
    ("generate.voronoi", "id"),
]

KINDS = {"enum": "choice", "string": "text"}

# Parameters the desktop added after the web paused (docs/adr/0228; each in TODOS.md §0.2 for the web).
ADDED_SINCE = {
    # Yinelenen anahtar (docs/adr/0439 §4).
    "attributes.joinByField": [{"name": "duplicates", "type": "enum", "label": "Yinelenen anahtar"}],
}


def listed_tool(t: dict) -> dict:
    return {
        "id": t["id"],
        "label": t["label"],
        "category": "$any",
        "description": "$any",
        "aliases": t.get("aliases", []),
        "parameters": [
            {
                "name": p["name"],
                "label": p["label"],
                "kind": KINDS.get(p["type"], p["type"]),
                "description": "$any",
                "optional": bool(p.get("optional")),
                "advanced": "$any",
                "default": "$any",
                "options": "$any",
            }
            for p in t["parameters"] + ADDED_SINCE.get(t["id"], [])
        ],
    }


def tools_cases() -> list[dict]:
    out = []
    for tool, how in LISTED:
        t = INVENTORY_TOOLS.get(tool)
        if t is None:
            raise SystemExit(f"envanterde yok: {tool}")
        if how == "alias":
            if not t.get("aliases"):
                raise SystemExit(f"takma adı yok: {tool}")
            query = t["aliases"][0].lower()
            name = f"{t['label']}, takma adıyla ({query})"
        else:
            query = tool
            name = f"{t['label']}, kimliğiyle"
        out.append(
            {
                "name": name,
                "steps": [
                    {
                        "op": "execute",
                        "input": {"query": query},
                        "result": {"status": "completed", "output": {"tools": [listed_tool(t)]}, "warnings": []},
                        "expect": UNCHANGED,
                    }
                ],
            }
        )
    out.append(
        {
            "name": "Hiçbir aracın tutmadığı sözcük: boş liste",
            "steps": [
                {
                    "op": "execute",
                    "input": {"query": "qqqzzz"},
                    "result": {"status": "completed", "output": {"tools": []}, "warnings": []},
                    "expect": UNCHANGED,
                }
            ],
        }
    )
    out.append(
        {
            "name": "Bilinmeyen kategori: invalid_input",
            "steps": [
                {
                    "op": "execute",
                    "input": {"category": "yok"},
                    "result": failed("invalid_input", "“yok” bir işlem kategorisi değil.", "category"),
                    "expect": UNCHANGED,
                }
            ],
        }
    )
    t = INVENTORY_TOOLS["points.numberVertices"]
    out.append(
        {
            "name": "plan da listeler, çizimi değiştirmez",
            "steps": [
                {
                    "op": "plan",
                    "input": {"query": "points.numberVertices"},
                    "result": {"status": "completed", "output": {"tools": [listed_tool(t)]}, "warnings": []},
                    "expect": UNCHANGED,
                }
            ],
        }
    )
    return out


def write(name: str, data: dict, check: bool) -> bool:
    path = OUT / name
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} güncel değil: --check'siz çalıştırın")
            return False
        return True
    path.write_text(text, encoding="utf-8")
    return True


def main() -> int:
    check = "--check" in sys.argv
    cases = {c["id"]: c for c in json.loads((PROCESSING / "cases.json").read_text(encoding="utf-8"))["cases"]}
    queries = json.loads((PROCESSING / "queries.json").read_text(encoding="utf-8"))["cases"]
    run_cases = [run_case(c, "parcels.kcad") for c in cases.values() if usable(c)]
    run_cases += [run_case(c, "parcels.kcad") for c in queries if usable(c)][:10]
    # Tools that run and stop (tool_failed): their own refusals, nothing written.
    for file, ids in (
        ("queries.json", ("refuse-field-rule", "join-duplicates-stop")),
        ("generate.json", ("voronoi-one", "delaunay-line")),
        ("geometry.json", ("refuse-reproject-same",)),
    ):
        listed = {c["id"]: c for c in json.loads((PROCESSING / file).read_text(encoding="utf-8"))["cases"]}
        run_cases += [run_case(listed[i], "parcels.kcad") for i in ids]
    run_cases += own_run_from(cases)
    run_cases += OWN_RUN
    run = {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.processing.run",
        "commandVersion": 1,
        "title": "İşlem aracını çalıştır",
        "note": "Üretici: scripts/fixtures/processing_command_cases.py (docs/adr/0508). Araçların sonuçları İşlemler'in kendi ortak durumlarından (fixtures/processing/v1/cases.json, queries.json), komutun sözleriyle.",
        "setup": document("parcels.kcad"),
        "cases": run_cases,
    }
    tools = {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.processing.tools",
        "commandVersion": 1,
        "title": "İşlem araçları",
        "note": "Üretici: scripts/fixtures/processing_command_cases.py (docs/adr/0508). Araçların kimlikleri, adları, takma adları ve parametreleri web'in donmuş envanterinden (docs/inventory/web.json).",
        "setup": document("parcels.kcad"),
        "cases": tools_cases(),
    }
    ok = write("cad.processing.run.json", run, check) & write("cad.processing.tools.json", tools, check)
    if ok:
        print(f"İşlem komutlarının durumları tutarlı: {len(run_cases)} + {len(tools['cases'])} durum")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
