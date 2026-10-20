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

Models run through the command too (docs/adr/0529): the shared model case (the three-step parcel sheet) is restated
as `cad.processing.run`'s case, and a model whose middle step stops (a layer's integer field refuses an edge length)
takes the first step back. Every run's answer carries its record (kentos.run-record 1, docs/adr/0529 §2); its
volatile parts (times, revision, content digests) are `$any`, its counts and steps are written out for the cases
where the shared cases say what flowed.

  python3 scripts/fixtures/processing_command_cases.py          # writes the two files
  python3 scripts/fixtures/processing_command_cases.py --check  # fails when they differ
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROCESSING = ROOT / "fixtures/processing/v1"
OUT = ROOT / "fixtures/commands/v1"
INVENTORY = ROOT / "docs/inventory/web.json"

WEB_TOOLS = ROOT / "apps/web/src/processing/builtin"
INVENTORY_TOOLS = {t["id"]: t for t in json.loads(INVENTORY.read_text(encoding="utf-8"))["processing"]}


def document(name: str) -> dict:
    return json.loads((PROCESSING / name).read_text(encoding="utf-8"))


def uid(slot: int) -> str:
    return f"$uidOf:{slot}"


MODEL_FEATURE_OUTPUTS = {"model:builtin.parcelSheet": {"points"}}
MODEL_LAYERS = {"model:builtin.parcelSheet": "parsel"}


def features_kinds(tool: str) -> set[str]:
    """The names of a tool's features outputs, as the web's definitions give them."""
    if tool.startswith("model:"):
        return MODEL_FEATURE_OUTPUTS.get(tool, set())
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


def default_scope(tool: str, param: str) -> str:
    """A features parameter's default scope, from the web's definition of the tool (its `default: { scope: … }`);
    none written is the selection."""
    import re

    for f in sorted(WEB_TOOLS.rglob("*.ts")):
        text = f.read_text(encoding="utf-8")
        if f"id: '{tool}'" not in text:
            continue
        for line in text.splitlines():
            if f"name: '{param}'" in line and "type: 'features'" in line:
                m = re.search(r"default: \{ scope: '(\w+)'", line)
                return m.group(1) if m else "selection"
    return "selection"


def tool_of(c: dict) -> str:
    """A case's tool id: a model's is “model:” and its own id (docs/adr/0529)."""
    run = c["run"]
    return run["tool"] if "tool" in run else "model:" + run["model"]


def inputs_of(c: dict, doc: dict) -> list[dict]:
    """The layers a run read, as the command lists them: a features input's layer, or the layers of the objects named
    or chosen, in the inputs' order (ADR 0508 §3). A tool's features parameters come from the web's definitions; one left
    at its default reads the selection."""
    tool = tool_of(c)
    if tool.startswith("model:"):
        return [{"layer": MODEL_LAYERS[tool], "digest": "$any"}]
    t = INVENTORY_TOOLS.get(c["run"]["tool"], {})
    layer_of = {e["id"]: e["layerId"] for e in doc["entities"]}
    sources = {s["layer"]: s for s in (doc.get("settings", {}).get("sources") or [])}
    out: list[str] = []
    for p in t.get("parameters", []):
        if p["type"] != "features":
            continue
        v = (c.get("values") or {}).get(p["name"], {"scope": default_scope(c["run"]["tool"], p["name"])})
        scope = v.get("scope")
        if scope == "layer":
            found = [v["layerId"]]
        elif scope == "ids":
            found = [layer_of[n] for n in v["ids"] if n in layer_of]
        elif scope == "selection":
            found = [layer_of[n] for n in c.get("selection") or [] if n in layer_of]
        else:
            found = []
        for l in found:
            if l not in out:
                out.append(l)
    # A layer's digest is the SHA-256 of a canonical JSON KentOS writes (sources::layer_digest); its value is not
    # written here.
    return [
        {"layer": l, "digest": "$any",
         **({"source": sources[l]["id"], "revision": sources[l]["revision"]} if l in sources else {})}
        for l in out
    ]


def run_case(c: dict, fixture_doc: str) -> dict:
    """One of İşlemler's cases as a cad.processing.run case."""
    tool = tool_of(c)
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
        inputs = inputs_of(c, doc)
        if inputs:
            output["inputs"] = inputs
        if want.get("undo"):
            output["step"] = want["undo"]
        output["record"] = "$any"
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
        return False  # a model's case is restated by `model_cases`
    want = c["expect"]
    if want["status"] != "ok":
        return True
    declared = INVENTORY_TOOLS.get(c["run"]["tool"], {}).get("outputs", [])
    return not declared or set(want.get("outputs") or {}) == {o["name"] for o in declared}


LEFT_KINDS = {"text": "yazı", "dimension": "ölçü", "insert": "blok", "hatch": "tarama", "leader": "kılavuz", "table": "tablo"}


BATCH_ROWS = 5000


def columns_of(made: list[dict]) -> list[dict]:
    """The columns a table of these objects has (docs/adr/0549 §4): a new layer has no fields, so one text column per
    attribute the written objects carry, in the attributes' byte order (a name is its own column here)."""
    keys: set[str] = set()
    for a in made:
        if a.get("kind", "") not in LEFT_KINDS:
            keys |= set(a.get("attrs", {}))
    return [{"attribute": k, "column": k} for k in sorted(keys, key=lambda k: k.encode("utf-8"))]


def sink_base(c: dict, inputs: list[str]) -> dict:
    """The pieces every sink case of İşlemler's case `c` shares: its setup, the run's step with the answer minus the
    sink, the table it makes (the new layer's name, its rows after what GeoPackage cannot hold) and the loss."""
    run = run_case(c, "parcels.kcad")
    step = run["steps"][0]
    out = step["result"]["output"]
    want = c["expect"]
    layer = want["layers"][0]
    made = want.get("addedShapes") or want.get("added") or []
    # GeoPackage holds no texts, dimensions, blocks or hatches: they are counted by kind (ADR 0502 §3).
    left: dict[str, int] = {}
    for a in made:
        word = LEFT_KINDS.get(a.get("kind", ""))
        if word:
            left[word] = left.get(word, 0) + 1
    rows = len(made) - sum(left.values())
    out.pop("step", None)
    out["record"] = "$any"
    out["inputs"] = [{"layer": l, "digest": "$any"} for l in inputs]
    out.pop("layers", None)
    out["layers"] = [layer["id"]]
    return {
        "case": c, "setup": document(c["document"]), "step": step, "layer": layer, "left": left, "rows": rows,
        "columns": columns_of(made),
    }


def sink_answer(base: dict, path: str, mode: str, replaced: bool = False, untouched: list[str] | None = None) -> dict:
    """The answer of a run that wrote `base`'s table to the file at `path` (docs/adr/0549 §4)."""
    layer = base["layer"]
    answer = json.loads(json.dumps(base["step"]["result"]["output"]))
    table = {
        "table": layer["name"], "layer": layer["id"], "rows": base["rows"], "geometry": "$any", "z": "$any",
        "m": "$any", **({"replaced": True} if replaced else {}), "columns": base["columns"],
    }
    answer["sink"] = {
        "path": path, "bytes": "$any", "sha256": "$any",
        "tables": [{"table": layer["name"], "layer": layer["id"], "rows": base["rows"], "geometry": "$any"}],
        **({"left": base["left"]} if base["left"] else {}),
        "approximated": 0,
    }
    answer["sinkReport"] = {
        "mode": mode, "tables": [table], **({"untouched": untouched} if untouched else {}),
        "batches": 1, "batchRows": BATCH_ROWS, "ids": "kentos_uids",
    }
    return answer


def sink_warnings(base: dict) -> list[dict]:
    if not base["left"]:
        return []
    parts = ", ".join(f"{n} {k}" for k, n in sorted(base["left"].items()))
    return [{
        "code": "export_losses",
        "message": f"GeoPackage'da karşılığı olmayan nesneler yazılmadı: {parts}.",
        "path": "sink",
    }]


UNDISTURBED = {"canUndo": False, "dirty": False, "revision": "same"}


def sink_step(base: dict, op: str, sink: dict, answer: dict) -> dict:
    step = json.loads(json.dumps(base["step"]))
    step["op"] = op
    step["input"]["sink"] = sink
    result: dict = json.loads(json.dumps(base["step"]["result"]))
    result["output"] = answer
    if base["left"]:
        result["warnings"] = sink_warnings(base)
    step["result"] = result
    step["expect"] = dict(UNDISTURBED)
    return step


def sink_case(c: dict, inputs: list[str]) -> dict:
    """One of İşlemler's cases planned with a sink (docs/adr/0508 §4): the same run on a copy, its new objects as one
    table named by the new layer's name, the drawing unchanged. A plan writes no file; the file's own bytes are the
    sink test's (processing_sink_cases.py)."""
    base = sink_base(c, inputs)
    path = "$fixture:processing/v1/sonuc-plan.gpkg"
    step = sink_step(base, "plan", {"path": path}, sink_answer(base, path, "new"))
    return {"name": f"Dosyaya (GeoPackage), plan: {c['title']} ({c['id']})", "setup": base["setup"], "steps": [step]}


# The file a result is added to: QGIS 4.2.3's own (its tables are checked against OGR by geopackage_cases.py), so an
# added table beside them is told from them by the tables they are described as.
QGIS_FILE = "geopackage/v1/qgis.gpkg"


def qgis_tables() -> list[dict]:
    """qgis.gpkg's tables as `cad.sources.describe` lists them: the shared case of that command, written from OGR."""
    d = json.loads((OUT / "cad.sources.describe.json").read_text(encoding="utf-8"))
    return d["cases"][0]["steps"][0]["result"]["output"]["tables"]


def listed(base: dict, replace: bool = False) -> dict:
    """The table a sink made, as `cad.sources.describe` lists it in the file."""
    return {
        "name": base["layer"]["name"], "data": "features", "rows": base["rows"], "geometry": "$any", "srid": 5254,
        "columns": "$any", "capabilities": "$any",
    }


def described(path: str, tables: list[dict]) -> dict:
    return {
        "op": "execute", "command": "cad.sources.describe", "input": {"path": path},
        "result": {"status": "completed", "output": {"tables": tables, "revision": "$any"}, "warnings": []},
        "expect": dict(UNDISTURBED),
    }


def copy_qgis(to: str) -> dict:
    return {"op": "copyFile", "from": f"$fixture:{QGIS_FILE}", "to": to}


def failed_step(base: dict, sink: dict, code: str, message: str, path: str = "sink.path") -> dict:
    step = json.loads(json.dumps(base["step"]))
    step["op"] = "execute"
    step["input"]["sink"] = sink
    step["result"] = failed(code, message, path)
    step["expect"] = {"canUndo": False, "revision": "same"}
    return step


SINK_TOOLS = [
    ("geometry", "intersection", ["parsel", "imar"], "ekle-kesisim"),
    ("geometry", "buffer-points", ["agac"], "ekle-tampon"),
    ("geometry", "dissolve-ada", ["parsel"], "ekle-birlestirme"),
]


def into_cases(geometry: dict) -> list[dict]:
    """Results written into a GeoPackage that is there (docs/adr/0549): tables added beside the file's own, which no
    write touches; a table of the same name refused unless replacing is asked; a plan that leaves the file alone."""
    old = qgis_tables()
    old_names = [t["name"] for t in old]
    out = []
    for _, cid, inputs, slug in SINK_TOOLS:
        base = sink_base(geometry[cid], inputs)
        title = geometry[cid]["title"]
        file = f"$tmp:{slug}.gpkg"
        add = {"path": file, "mode": "addTables"}
        replace = {"path": file, "mode": "replaceTables"}
        mine = listed(base)
        setup = base["setup"]
        out.append({
            "name": f"Var olan GeoPackage'a tablo ekleme: {title}; dosyanın tabloları olduğu gibi",
            "setup": setup,
            "steps": [
                copy_qgis(file),
                sink_step(base, "execute", add, sink_answer(base, file, "addTables", untouched=old_names)),
                described(file, old + [mine]),
            ],
        })
        out.append({
            "name": f"Var olan GeoPackage'a ekleme, plan: {title}; dosya yazılmaz",
            "setup": setup,
            "steps": [
                copy_qgis(file),
                sink_step(base, "plan", add, sink_answer(base, file, "addTables", untouched=old_names)),
                described(file, old),
            ],
        })
        out.append({
            "name": f"Aynı adlı tablo: addTables reddeder (table_exists), dosya değişmez: {title}",
            "setup": setup,
            "steps": [
                copy_qgis(file),
                sink_step(base, "execute", add, sink_answer(base, file, "addTables", untouched=old_names)),
                failed_step(base, add, "table_exists",
                            f"“{base['layer']['name']}” tablosu {file} dosyasında var (tablonun adı katmanın ya da aracın tablo "
                            "çıktısının adıdır); üzerine yazmak için sink.mode: replaceTables verin ya da başka bir "
                            "dosyaya yazın. Dosyaya dokunulmadı."),
                described(file, old + [mine]),
            ],
        })
        out.append({
            "name": f"replaceTables aynı adlı tabloyu yenisiyle değiştirir, dosyanın öbür tablolarına dokunmaz: {title}",
            "setup": setup,
            "steps": [
                copy_qgis(file),
                sink_step(base, "execute", add, sink_answer(base, file, "addTables", untouched=old_names)),
                sink_step(base, "execute", replace,
                          sink_answer(base, file, "replaceTables", replaced=True, untouched=old_names)),
                described(file, old + [mine]),
            ],
        })
        out.append({
            "name": f"replaceTables aynı adlı tablo yoksa yalnız ekler (replaced yok): {title}",
            "setup": setup,
            "steps": [
                copy_qgis(file),
                sink_step(base, "execute", replace, sink_answer(base, file, "replaceTables", untouched=old_names)),
                described(file, old + [mine]),
            ],
        })
    # Two results into one file: both tables are there, the file's own six beside them.
    first = sink_base(geometry["intersection"], ["parsel", "imar"])
    second = sink_base(geometry["dissolve-ada"], ["parsel"])
    file = "$tmp:iki-sonuc.gpkg"
    out.append({
        "name": "Aynı dosyaya iki sonuç: ikisi de eklenir, dosyanın tabloları olduğu gibi",
        "setup": first["setup"],
        "steps": [
            copy_qgis(file),
            sink_step(first, "execute", {"path": file, "mode": "addTables"},
                      sink_answer(first, file, "addTables", untouched=old_names)),
            sink_step(second, "execute", {"path": file, "mode": "addTables"},
                      sink_answer(second, file, "addTables", untouched=old_names + [first["layer"]["name"]])),
            described(file, old + [listed(first), listed(second)]),
        ],
    })
    # A new file, then another result added to it.
    file = "$tmp:yeni-sonra-ekle.gpkg"
    out.append({
        "name": "Yeni dosya, sonra ona tablo ekleme: iki tablo",
        "setup": first["setup"],
        "steps": [
            sink_step(first, "execute", {"path": file}, sink_answer(first, file, "new")),
            sink_step(second, "execute", {"path": file, "mode": "addTables"},
                      sink_answer(second, file, "addTables", untouched=[first["layer"]["name"]])),
            described(file, [listed(first), listed(second)]),
        ],
    })
    # The refusals before anything is written.
    base = sink_base(geometry["buffer-points"], ["agac"])
    missing = "$tmp:yok-dosya.gpkg"
    out.append({
        "name": "addTables: dosya yok (file_not_found), hiçbir şey yazılmaz",
        "setup": base["setup"],
        "steps": [failed_step(base, {"path": missing, "mode": "addTables"}, "file_not_found",
                              f"{missing} yok; tablo eklenecek GeoPackage'ın yolunu verin ya da yeni bir dosya için "
                              "sink.mode vermeyin.")],
    })
    out.append({
        "name": "replaceTables: dosya yok (file_not_found)",
        "setup": base["setup"],
        "steps": [failed_step(base, {"path": missing, "mode": "replaceTables"}, "file_not_found",
                              f"{missing} yok; tablo eklenecek GeoPackage'ın yolunu verin ya da yeni bir dosya için "
                              "sink.mode vermeyin.")],
    })
    out.append({
        "name": "addTables ile overwrite birlikte verilmez (invalid_input)",
        "setup": base["setup"],
        "steps": [failed_step(base, {"path": "$tmp:x.gpkg", "mode": "addTables", "overwrite": True}, "invalid_input",
                              "overwrite yeni dosyanın üzerine yazmak içindir; addTables ve replaceTables var olan "
                              "dosyaya dokunmadan tablo ekler ya da yenisiyle değiştirir.", "sink.overwrite")],
    })
    out.append({
        "name": "replaceTables ile overwrite birlikte verilmez (invalid_input)",
        "setup": base["setup"],
        "steps": [failed_step(base, {"path": "$tmp:x.gpkg", "mode": "replaceTables", "overwrite": True},
                              "invalid_input",
                              "overwrite yeni dosyanın üzerine yazmak içindir; addTables ve replaceTables var olan "
                              "dosyaya dokunmadan tablo ekler ya da yenisiyle değiştirir.", "sink.overwrite")],
    })
    text = "$fixture:processing/v1/metin.gpkg"
    out.append({
        "name": "addTables: GeoPackage olmayan dosya (import_unreadable), dosyaya dokunulmaz",
        "setup": base["setup"],
        "steps": [failed_step(base, {"path": text, "mode": "addTables"}, "import_unreadable",
                              f"{text} GeoPackage olarak açılamadı: “{text}” bir SQLite dosyası değil. "
                              "Dosyaya dokunulmadı.")],
    })
    out.append({
        "name": "addTables: .gpkg olmayan ad (invalid_file_path)",
        "setup": base["setup"],
        "steps": [failed_step(base, {"path": "$fixture:processing/v1/sonuc.shp", "mode": "addTables"},
                              "invalid_file_path",
                              "$fixture:processing/v1/sonuc.shp bir GeoPackage adı değil; sonucun dosyası .gpkg ile "
                              "biter.")],
    })
    return out


# ── A tool's table output as a table of the file (docs/adr/0551 §3) ───────────────────────────────────────────────

def web_label(tool: str, output: str) -> str:
    """An output's label in the web's frozen definition of the tool (the table is named by it)."""
    for f in sorted(WEB_TOOLS.rglob("*.ts")):
        text = f.read_text(encoding="utf-8")
        at = text.find(f"id: '{tool}'")
        if at < 0:
            continue
        m = re.compile(r"name: '%s',\s*label: '([^']*)'" % re.escape(output)).search(text, at)
        if m:
            return m.group(1)
    raise KeyError(f"{tool}.{output}")


def unique_columns(headers: list[str]) -> list[str]:
    """The columns of a text table: a header as it is, an empty one `sutun_N`, a repeated one (or `fid`) `_2`, `_3` …
    (SQLite's column names are one whatever their case)."""
    taken = {"fid"}
    out = []
    for i, h in enumerate(headers):
        base = h.strip() or f"sutun_{i + 1}"
        col, n = base, 2
        while col.lower() in taken:
            col = f"{base}_{n}"
            n += 1
        taken.add(col.lower())
        out.append(col)
    return out


def table_outputs(c: dict) -> list[dict]:
    """The table outputs of a case in the order the command writes them (by the output's name), each with the table's
    name, its headers and rows."""
    tool = c["run"]["tool"]
    found = []
    for name, v in sorted((c["expect"].get("outputs") or {}).items()):
        if isinstance(v, dict) and "columns" in v and "rows" in v:
            found.append({"output": name, "label": web_label(tool, name), "columns": v["columns"], "rows": v["rows"]})
    return found


def named(tables: list[dict], taken: list[str]) -> list[dict]:
    """The tables' names: the label, made one the write has not given yet (`_2`, `_3` …)."""
    names = {n.lower() for n in taken} | {"kentos_uids"}
    out = []
    for t in tables:
        base = t["label"].strip()
        name, n = base, 2
        while name.lower() in names:
            name = f"{base}_{n}"
            n += 1
        names.add(name.lower())
        out.append({**t, "table": name})
    return out


def text_entry(t: dict, replaced: bool = False) -> dict:
    return {
        "table": t["table"], "output": t["output"], "rows": len(t["rows"]), "z": "none", "m": "none",
        **({"replaced": True} if replaced else {}),
        "columns": [{"attribute": a, "column": c} for a, c in zip(t["columns"], unique_columns(t["columns"]))],
    }


def text_answer(out: dict, path: str, mode: str, tables: list[dict], *, untouched=(), replaced=False,
                before=None) -> dict:
    """The answer of a run that wrote `tables` (and, when `before` is the answer of a sink with objects, its layers'
    tables) to the file at `path`."""
    answer = json.loads(json.dumps(out if before is None else before))
    answer.pop("step", None)
    sink = answer.setdefault("sink", {"path": path, "bytes": "$any", "sha256": "$any", "tables": [], "approximated": 0})
    report = answer.setdefault("sinkReport", {"mode": mode, "tables": [], "batches": 0, "batchRows": BATCH_ROWS,
                                              "ids": "kentos_uids"})
    for t in tables:
        sink["tables"].append({"table": t["table"], "layer": t["output"], "rows": len(t["rows"])})
        report["tables"].append(text_entry(t, replaced))
        report["batches"] += -(-len(t["rows"]) // BATCH_ROWS)
    if untouched:
        report["untouched"] = list(untouched)
    return answer


def text_step(base_step: dict, op: str, sink: dict, answer: dict, warnings: list | None = None) -> dict:
    step = json.loads(json.dumps(base_step))
    step["op"] = op
    step["input"]["sink"] = sink
    step["result"] = {"status": "completed", "output": answer, "warnings": warnings or []}
    step["expect"] = dict(UNDISTURBED)
    return step


def listed_text(t: dict) -> dict:
    """A text table as `cad.sources.describe` lists it: an attribute table, its columns text."""
    return {"name": t["table"], "data": "attributes", "rows": len(t["rows"]), "columns": "$any", "capabilities": "$any"}


def tables_into_cases(queries: dict, others: dict) -> list[dict]:
    """A tool's table outputs written with `sink.tables` (docs/adr/0551 §3): a new file, a file that is there (added,
    refused for a clash, replaced), a plan, a table beside the layers of a tool that makes objects, and the refusals."""
    old = qgis_tables()
    old_names = [t["name"] for t in old]
    out: list[dict] = []

    def base_of(c: dict) -> tuple[dict, dict, list[dict]]:
        run = run_case(c, "parcels.kcad")
        step = run["steps"][0]
        answer = step["result"]["output"]
        answer.pop("step", None)
        return step, answer, named(table_outputs(c), [])

    # 1. The summary by Ada: one attribute table, its cells the text the tool shows.
    c = queries["summary-grouped"]
    step, answer, tables = base_of(c)
    setup = document(c["document"])
    file = "$tmp:ozet.gpkg"
    new = {"path": file, "tables": True}
    out.append({
        "name": f"Tablo çıktısı dosyaya (yeni dosya): {c['title']}; öznitelik tablosu, metin sütunları",
        "setup": setup,
        "steps": [text_step(step, "execute", new, text_answer(answer, file, "new", tables)),
                  described(file, [listed_text(t) for t in tables])],
    })
    out.append({
        "name": f"Tablo çıktısı, plan: {c['title']}; dosya yazılmaz",
        "setup": setup,
        "steps": [text_step(step, "plan", {"path": "$tmp:ozet-plan.gpkg", "tables": True},
                            text_answer(answer, "$tmp:ozet-plan.gpkg", "new", tables))],
    })
    # 2. Into QGIS's file: added beside its tables, which no write touches; a clash refused; replaced by name.
    add = {"path": file, "mode": "addTables", "tables": True}
    replace = {"path": file, "mode": "replaceTables", "tables": True}
    name = tables[0]["table"]
    out.append({
        "name": f"Tablo çıktısı var olan GeoPackage'a eklenir; dosyanın tabloları olduğu gibi: {c['title']}",
        "setup": setup,
        "steps": [copy_qgis(file),
                  text_step(step, "execute", add, text_answer(answer, file, "addTables", tables, untouched=old_names)),
                  described(file, old + [listed_text(t) for t in tables])],
    })
    out.append({
        "name": f"Tablo çıktısının adı dosyada var: addTables reddeder (table_exists), dosya değişmez: {c['title']}",
        "setup": setup,
        "steps": [copy_qgis(file),
                  text_step(step, "execute", add, text_answer(answer, file, "addTables", tables, untouched=old_names)),
                  failed_step({"step": step}, add, "table_exists",
                              f"“{name}” tablosu {file} dosyasında var (tablonun adı katmanın ya da aracın tablo "
                              "çıktısının adıdır); üzerine yazmak için sink.mode: replaceTables verin ya da başka bir "
                              "dosyaya yazın. Dosyaya dokunulmadı."),
                  described(file, old + [listed_text(t) for t in tables])],
    })
    out.append({
        "name": f"replaceTables tablo çıktısının eski tablosunu yenisiyle değiştirir, dosyanın öbür tablolarına dokunmaz: {c['title']}",
        "setup": setup,
        "steps": [copy_qgis(file),
                  text_step(step, "execute", add, text_answer(answer, file, "addTables", tables, untouched=old_names)),
                  text_step(step, "execute", replace,
                            text_answer(answer, file, "replaceTables", tables, untouched=old_names, replaced=True)),
                  described(file, old + [listed_text(t) for t in tables])],
    })
    # 3. Other tools' tables: ungrouped summary, a distance list and its summary, parcel neighbours.
    for case_id, source, slug in (("summary-plain", queries, "ozet-duz"),
                                  ("matrix-list", others["proximity.json"], "uzaklik-listesi"),
                                  ("matrix-summary", others["proximity.json"], "uzaklik-ozeti"),
                                  ("neighbours-parcels", others["proximity.json"], "komsular")):
        c2 = source[case_id]
        step2, answer2, tables2 = base_of(c2)
        f2 = f"$tmp:{slug}.gpkg"
        out.append({
            "name": f"Tablo çıktısı dosyaya: {c2['title']} ({case_id}); tablo sayısı, adı ve sütunları",
            "setup": document(c2["document"]),
            "steps": [text_step(step2, "execute", {"path": f2, "tables": True},
                                text_answer(answer2, f2, "new", tables2)),
                      described(f2, [listed_text(t) for t in tables2])],
        })
    # 4. A tool that makes objects and a table: the layers' tables first, the output's beside them.
    c3 = others["geometry.json"]["simplify"]
    base = sink_base(c3, ["parcel"] if False else [i["layer"] for i in run_case(c3, "parcels.kcad")["steps"][0]["result"]["output"].get("inputs", [])])
    tables3 = named(table_outputs(c3), [base["layer"]["name"]])
    f3 = "$tmp:sadelestir.gpkg"
    layered = sink_answer(base, f3, "new")
    answer3 = text_answer(None, f3, "new", tables3, before=layered)
    out.append({
        "name": f"Nesne ve tablo çıktısı birlikte: katmanın tablosu önce, çıktının tablosu yanında: {c3['title']}",
        "setup": base["setup"],
        "steps": [text_step(base["step"], "execute", {"path": f3, "tables": True}, answer3,
                            sink_warnings(base)),
                  described(f3, [listed(base)] + [listed_text(t) for t in tables3])],
    })
    # 5. Without `tables` a tool whose result is only a table has nothing to write; neither has one without a table.
    hint = ("Araç yeni nesne yapmadı; dosyaya yazılacak sonuç yok (değiştiren ya da seçen araçların sonucu çizimdedir; "
            "aracın tablo çıktısını yazmak için sink.tables verin).")
    out.append({
        "name": "Tablo çıktısı sink.tables olmadan yazılmaz: nothing_to_export, nasıl isteneceğini söyler",
        "setup": setup,
        "steps": [failed_step({"step": step}, {"path": "$tmp:yok.gpkg"}, "nothing_to_export", hint)],
    })
    out.append({
        "name": "sink.tables açıkken tablo çıktısı olmayan araç: nothing_to_export, dosya yazılmaz",
        "steps": [{
            "op": "execute",
            "input": {"tool": "selection.byExpression", "sink": {"path": "$tmp:secen.gpkg", "tables": True}},
            "result": failed("nothing_to_export", hint, "sink.path"),
            "expect": UNCHANGED,
        }],
    })
    return out


SINK_REFUSALS = [
    ("göreli yol", "sonuc.gpkg", "invalid_file_path",
     "“sonuc.gpkg” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/sonuç.gpkg)."),
    ("GeoPackage olmayan ad", "$fixture:processing/v1/sonuc.shp", "invalid_file_path",
     "$fixture:processing/v1/sonuc.shp bir GeoPackage adı değil; sonucun dosyası .gpkg ile biter."),
    ("olmayan klasör", "$fixture:processing/v1/yok/sonuc.gpkg", "invalid_file_path",
     "$fixture:processing/v1/yok/sonuc.gpkg dosyasının klasörü yok; önce klasörü açın ya da başka bir yol verin."),
    ("var olan dosya, overwrite yok", "$fixture:geopackage/v1/qgis.gpkg", "file_exists",
     "$fixture:geopackage/v1/qgis.gpkg var; üzerine yazmak için overwrite: true verin."),
]


def sink_refusals() -> list[dict]:
    out = []
    for name, path, code, message in SINK_REFUSALS:
        out.append({
            "name": f"Dosyaya yazma reddi: {name} ({code})",
            "steps": [{
                "op": "execute",
                "input": {"tool": "attributes.calculate", "selection": ["$uidOf:1"], "sink": {"path": path}},
                "result": failed(code, message, "sink.path"),
                "expect": UNCHANGED,
            }],
        })
    out.append({
        "name": "Dosyaya yazma: seçen araç yeni nesne yapmaz (nothing_to_export), çizim değişmez",
        "steps": [{
            "op": "execute",
            "input": {"tool": "selection.byExpression", "sink": {"path": "$fixture:processing/v1/sonuc.gpkg"}},
            "result": failed(
                "nothing_to_export",
                "Araç yeni nesne yapmadı; dosyaya yazılacak sonuç yok (değiştiren ya da seçen araçların sonucu çizimdedir; "
                "aracın tablo çıktısını yazmak için sink.tables verin).",
                "sink.path",
            ),
            "expect": UNCHANGED,
        }],
    })
    return out


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


def record(kind: str, tool: str, label: str, *, steps: list[dict], total: tuple[int, int], selection=(),
           inputs=(), srid: int | None = 5256, definition: bool = False, parameters="$any", view=None,
           failed=()) -> dict:
    """A run's record as the command gives it (docs/adr/0529 §2), its volatile parts `$any`: the revision, the engine,
    the times, a model's definition digest. The counts are written out by the caller from the shared cases."""
    r: dict = {"format": "kentos.run-record", "version": 1, "kind": kind, "tool": tool, "label": label, "toolVersion": 1}
    if definition:
        r["definition"] = "$any"
    r["parameters"] = parameters
    if selection:
        r["selection"] = list(selection)
    if view:
        r["view"] = view
    if inputs:
        r["inputs"] = list(inputs)
    r.update({"documentRevision": "$any", "engine": "$any", "host": "headless", "startedAt": "$any", "ms": "$any"})
    if srid:
        r["srid"] = srid
    r["steps"] = steps
    r["total"] = {"recordsIn": total[0], "recordsOut": total[1]}
    if failed:
        r["failed"] = list(failed)
    return r


def step_of(id: str, tool: str, label: str, n_in: int, n_out: int, summary: str) -> dict:
    return {"id": id, "tool": tool, "label": label, "recordsIn": n_in, "recordsOut": n_out, "ms": "$any", "summary": summary}


PARCEL_MODEL = "model:builtin.parcelSheet"
# The model's steps: their ids, tools and the tools' names (the record says a step by its tool's name; a refusal says
# “2. adım (Kenar uzunluklarını yaz)”).
PARCEL_STEPS = [
    ("corners", "points.numberVertices", "Köşe noktalarını numarala"),
    ("edges", "annotation.edgeLengths", "Kenar uzunluklarını yaz"),
    ("area", "attributes.calculate", "Öznitelik hesapla"),
]


def model_record(cases: dict) -> dict:
    """The parcel sheet's record from İşlemler's own shared case: each step read the two chosen parcels
    (`recordsIn` 2); what the steps made or changed gives `recordsOut` (the points, the texts, the parcels written),
    and the whole made or changed the distinct objects of all three."""
    want = cases["model-parcel-sheet"]["expect"]
    points = [a for a in want["added"] if a["kind"] == "point"]
    texts = [a for a in want["added"] if a["kind"] == "text"]
    written = want["updated"]
    chosen = cases["model-parcel-sheet"]["selection"]
    outs = [len(points), len(texts), len(written)]
    # What each step said is the model's summary's sentences, in order.
    sentences = want["summary"].split(". ", 1)[1]
    parts = [
        "2 nesnede 6 köşe numaralandı: P00001 – P00006; 2 ortak köşe komşularıyla tek numara aldı.",
        "2 nesneye 7 kenar uzunluğu yazıldı; 1 ortak kenar bir kez yazıldı.",
        "2 nesnede “Hesap alanı” yazıldı.",
    ]
    assert " ".join(parts) == sentences, sentences
    steps = [step_of(i, t, name, len(chosen), n, text) for (i, t, name), n, text in zip(PARCEL_STEPS, outs, parts)]
    return record(
        "model", PARCEL_MODEL, "Parsel ölçü yazıları",
        steps=steps, total=(len(chosen) * 3, len(points) + len(texts) + len(written)),
        selection=[uid(n) for n in chosen], inputs=[{"layer": "parsel", "digest": "$any"}],
        definition=True,
    )


def model_cases(cases: dict) -> list[dict]:
    """The model through the command (docs/adr/0529 §3): the shared parcel-sheet case restated, its record, and a model
    whose middle step stops."""
    c = cases["model-parcel-sheet"]
    ran = run_case(c, "parcels.kcad")
    ran["name"] = "Model: " + ran["name"]
    ran["steps"][0]["result"]["output"]["record"] = model_record(cases)
    out = [ran]
    # A model that is not there, and a plan that writes nothing.
    out.append({
        "name": "Bilinmeyen model: unknown_tool, aracın yolu",
        "steps": [{
            "op": "execute",
            "input": {"tool": "model:builtin.yok"},
            "result": failed(
                "unknown_tool",
                "“model:builtin.yok” bir işlem aracı değil. Araçların listesi cad.processing.tools'ta.",
                "tool",
            ),
            "expect": UNCHANGED,
        }],
    })
    planned = json.loads(json.dumps(ran["steps"][0]))
    planned["op"] = "plan"
    # The objects the plan makes are the copy's: their ids are not the drawing's to name.
    outputs = planned["result"]["output"]["outputs"]
    outputs["points"] = ["$any"] * len(outputs["points"])
    planned["expect"] = {"canUndo": False, "dirty": False, "revision": "same", "ids": list(range(1, 11))}
    out.append({"name": "Model, plan: aynı sonuç bir kopyada, çizim değişmez", "steps": [planned]})
    # The middle step stops: the first step's points and layer are taken back, nothing is written (docs/adr/0529 §3).
    doc = document("parcels.kcad")
    doc["layers"].append({
        "id": "islem-kenar-olculeri", "name": "Kenar ölçüleri", "type": "layer", "visible": True, "locked": False,
        "expanded": True, "style": {"color": "fg-dim", "lineType": "continuous", "lineWeight": 0.18}, "children": [],
        "fields": [{"name": "Uzunluk (m)", "kind": "integer"}],
    })
    out.append({
        "name": "Model: ortadaki adım çalışmaz (katmanın tam sayı alanı kenar uzunluğunu reddeder), ilk adım geri alınır",
        "setup": doc,
        "steps": [{
            "op": "execute",
            "input": {"tool": PARCEL_MODEL, "selection": [uid(1), uid(2)]},
            "result": failed(
                "tool_failed",
                "2. adım (Kenar uzunluklarını yaz) çalışmadı: Öznitelik yazılamadı (yeni nesne): “Uzunluk (m)” alanı "
                "tam sayı ister; “20.000” verildi. Rakamlarla, ondalıksız yazın. Önceki adımların sonuçları geri alındı.",
                "tool",
            ),
            "expect": {"canUndo": False, "dirty": False, "revision": "same", "ids": [e["id"] for e in doc["entities"]]},
        }],
    })
    return out


def tool_record(cases: dict) -> dict:
    """Öznitelik hesapla's record for the shared area case: it read the three chosen parcels and wrote all three."""
    c = cases["calculate-area"]
    n = len(c["selection"])
    return record(
        "tool", "attributes.calculate", "Öznitelik hesapla",
        steps=[step_of("attributes.calculate", "attributes.calculate", "Öznitelik hesapla", n, len(c["expect"]["updated"]),
                       c["expect"]["summary"])],
        total=(n, len(c["expect"]["updated"])),
        selection=[uid(i) for i in c["selection"]], inputs=[{"layer": "parsel", "digest": "$any"}],
    )


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
                            "inputs": [{"layer": "parsel", "digest": "$any"}],
                            "step": area["undo"],
                            "record": "$any",
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
    area_case = run_case(cases["calculate-area"], "parcels.kcad")
    area_case["name"] = "Kayıt: " + area_case["name"]
    area_case["steps"][0]["result"]["output"]["record"] = tool_record(cases)
    out.append(area_case)
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
    # The models, when asked (docs/adr/0529): a model's id is “model:” and its own, its inputs its parameters (the web's
    # built-in model, apps/web/src/processing/builtin/models.ts).
    model_input = lambda name, label, kind: {  # noqa: E731
        "name": name, "label": label, "kind": kind, "description": "$any", "optional": False,
        "advanced": "$any", "default": "$any", "options": "$any",
    }
    model = {
        "id": "model:builtin.parcelSheet",
        "label": "Parsel ölçü yazıları",
        "category": "$any",
        "description": "Parsellerin köşelerini numaralar, kenar uzunluklarını yazar ve hesaplanan alanı özniteliğe yazar; "
                       "hepsi tek adımda geri alınır.",
        "aliases": [],
        "parameters": [model_input("parcels", "Parseller", "features"), model_input("prefix", "Nokta öneki", "text")],
    }
    out.append({
        "name": "Modeller: models ile model da listelenir (girdileri parametreleri)",
        "steps": [{
            "op": "execute",
            "input": {"query": "model:builtin.parcelSheet", "models": True},
            "result": {"status": "completed", "output": {"tools": [model]}, "warnings": []},
            "expect": UNCHANGED,
        }],
    })
    out.append({
        "name": "Modeller: models olmadan listede model yok",
        "steps": [{
            "op": "execute",
            "input": {"query": "model:builtin.parcelSheet"},
            "result": {"status": "completed", "output": {"tools": []}, "warnings": []},
            "expect": UNCHANGED,
        }],
    })
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


NOT_A_GEOPACKAGE = "Bu bir GeoPackage değil: düz metin, SQLite başlığı yok (docs/adr/0549).\n"


def write_text(name: str, text: str, check: bool) -> bool:
    """A file a case reads that is no GeoPackage (a sink that is asked to add tables to it says so and leaves it)."""
    path = PROCESSING / name
    if check:
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            print(f"{path.relative_to(ROOT)} eski: üreticiyi çalıştırın")
            return False
        return True
    path.write_text(text, encoding="utf-8")
    return True


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
    # The result to a GeoPackage (docs/adr/0508 §4): G1's parcels × zoning, a buffer, a dissolve by Ada.
    geometry = {c["id"]: c for c in json.loads((PROCESSING / "geometry.json").read_text(encoding="utf-8"))["cases"]}
    run_cases += [
        sink_case(geometry["intersection"], ["parsel", "imar"]),
        sink_case(geometry["buffer-points"], ["agac"]),
        sink_case(geometry["dissolve-ada"], ["parsel"]),
        sink_case(cases["number-vertices-both"], ["parsel"]),
    ]
    run_cases += sink_refusals()
    run_cases += into_cases(geometry)
    others = {
        f: {c["id"]: c for c in json.loads((PROCESSING / f).read_text(encoding="utf-8"))["cases"]}
        for f in ("proximity.json", "geometry.json")
    }
    run_cases += tables_into_cases({c["id"]: c for c in queries}, others)
    run_cases += own_run_from(cases)
    run_cases += model_cases(cases)
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
    ok &= write_text("metin.gpkg", NOT_A_GEOPACKAGE, check)
    if ok:
        print(f"İşlem komutlarının durumları tutarlı: {len(run_cases)} + {len(tools['cases'])} durum")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
