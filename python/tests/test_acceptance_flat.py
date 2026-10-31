"""The 2+1 apartment's acceptance flow from Python (plan/architecture-advanced.md ADV-07, docs/adr/0776): the shared
scenario (fixtures/acceptance/v1/flat.json, worked out by scripts/fixtures/acceptance_flat.py without KentOS code)
replayed through kentos.cad. Every write step is undone and redone (one step, its name, the drawing back); every
refusal's code is the scenario's; after the reopening every bound object is the same by the same id. With
``KENTOS_ACCEPTANCE_OUT`` the drawing is saved there as ``python.kcad`` for the hosts' comparison
(``acceptance_flat.py --compare``). The MCP server replays the same file."""

from __future__ import annotations

import json
import os
import unittest
from collections import Counter
from pathlib import Path
from typing import Any

import kentos.cad as cad

from test_acceptance import bind, matches

ROOT = Path(__file__).resolve().parents[2]
SCENARIO = json.loads((ROOT / "fixtures/acceptance/v1/flat.json").read_text(encoding="utf-8"))


def bound(output: dict[str, Any], key: Any, names: dict[str, str]) -> None:
    """The step's ``as``: a name the output's ``uid``, ``set`` or first ``uids``; a list its ``uids`` or ``created``."""
    if isinstance(key, str):
        names[key] = output.get("uid") or output.get("set") or output["uids"][0]
    elif isinstance(key, list):
        names.update(zip(key, output.get("created") or output["uids"]))


def replay(test: unittest.TestCase, scenario: dict[str, Any], out_name: str) -> None:
    """Replays ``scenario`` (a ``kentos.acceptance`` file) through kentos.cad and holds every step to it; with
    ``KENTOS_ACCEPTANCE_OUT`` the drawing is saved there as ``out_name``."""
    p = scenario["project"]
    doc = cad.Document.new(p["name"], workspace=p["workspace"], coords=p["coords"],
                           drawing_unit=p["drawingUnit"], plot_scale=p["plotScale"])
    names: dict[str, str] = {"layer": doc.active_layer}
    # The scenario's files by name, absolute (a command reads an absolute path); not objects.
    files = {k: str(ROOT / v) for k, v in scenario.get("files", {}).items()}
    names.update(files)
    tolerance = scenario["tolerance"]
    undone = 0
    for step in scenario["steps"]:
        name = step["name"]
        output: dict[str, Any] = {}
        if "command" in step:
            before = len(doc)
            answer = doc.run(step["command"], bind(step["input"], names))
            if "refuse" in step:
                test.assertEqual(answer.get("status"), "failed", f"{name}: {answer}")
                test.assertEqual(answer["error"]["code"], step["refuse"], name)
                test.assertEqual(len(doc), before, f"{name}: a refusal writes nothing")
            else:
                test.assertEqual(answer.get("status"), "completed", f"{name}: {answer}")
                output = answer["output"]
                bound(output, step.get("as"), names)
                label = step.get("label")
                if label is not None:
                    # One step: undone it is the drawing before, redone the drawing after.
                    after = len(doc)
                    shown = {k: doc.entity(u).entity.to_json() for k, u in names.items() if k != "layer" and k not in files}
                    test.assertEqual(doc.undo(), label, name)
                    test.assertEqual(len(doc), before, f"{name}: undone")
                    test.assertEqual(doc.redo(), label, name)
                    test.assertEqual(len(doc), after, f"{name}: redone")
                    again = {k: doc.entity(u).entity.to_json() for k, u in names.items() if k != "layer" and k not in files}
                    test.assertEqual(again, shown, f"{name}: redone the same objects")
                    undone += 1
        elif "query" in step:
            answer = doc.run(step["query"], bind(step["input"], names))
            test.assertEqual(answer.get("status"), "completed", f"{name}: {answer}")
            output = answer["output"]
            bound(output, step.get("as"), names)
        elif step.get("reopen"):
            shown = {k: doc.entity(u).entity.to_json() for k, u in names.items() if k != "layer" and k not in files}
            doc = cad.Document.from_bytes(doc.to_bytes())
            for k, u in names.items():
                if k != "layer" and k not in files:
                    test.assertEqual(doc.entity(u).entity.to_json(), shown[k], f"{name}: {k} by its id")
        problems: list[str] = []
        expect = step.get("expect", {})
        for key, want in expect.get("objects", {}).items():
            problems += matches(doc.entity(names[key]).entity.to_json(), bind(want, names), tolerance, key)
        for key, want in expect.get("measures", {}).items():
            m = doc.measure(names[key])
            problems += matches({"area": m.area, "length": m.length}, want, tolerance, f"{key} ölçü")
        if "count" in expect:
            problems += matches(len(doc), expect["count"], 0, "nesne sayısı")
        if "kinds" in expect:
            kinds = Counter(r.entity.to_json()["kind"] for r in doc.entities())
            problems += matches({k: kinds.get(k, 0) for k in expect["kinds"]}, expect["kinds"], 0, "türler")
        if "output" in expect:
            problems += matches(output, bind(expect["output"], names), 1e-9, "cevap")
        test.assertEqual(problems, [], name)
    test.assertGreater(undone, scenario.get("undone", 10))
    out = os.environ.get("KENTOS_ACCEPTANCE_OUT")
    if out:
        (Path(out) / out_name).parent.mkdir(parents=True, exist_ok=True)
        doc.save(Path(out) / out_name)


class AcceptanceFlat(unittest.TestCase):
    def test_the_apartment_is_drawn_changed_scheduled_and_reopened_as_the_scenario_says(self) -> None:
        replay(self, SCENARIO, "python.kcad")

if __name__ == "__main__":
    unittest.main()
