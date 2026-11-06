"""CAD-ADV-20 acceptance (docs/adr/1004): the parametric plate of fixtures/acceptance/v1/detail.json (written by
scripts/fixtures/acceptance_detail.py without KentOS code) replayed through the Python library, step by step, every
expected point, length and area held to it, with the half-way script, the cancelled job and the detail sheet of
docs/adr/1011. The MCP server replays the same file (crates/native/mcp/tests/acceptance_detail.rs):
both hosts answer the same values, refusals and undo from the one command catalog."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

import kentos.cad as cad
from kentos.cad.protocol import Expected

ROOT = Path(__file__).resolve().parents[2]
SCENARIO = json.loads((ROOT / "fixtures/acceptance/v1/detail.json").read_text(encoding="utf-8"))


def bind(value: Any, names: dict[str, str]) -> Any:
    if isinstance(value, str) and value.startswith("$") and value[1:] in names:
        return names[value[1:]]
    if isinstance(value, list):
        return [bind(v, names) for v in value]
    if isinstance(value, dict):
        return {k: bind(v, names) for k, v in value.items()}
    return value


def at_path(value: Any, path: str) -> Any:
    for key in path.split("."):
        value = value[int(key)] if key.isdigit() else value[key]
    return value


class ScriptCrash(RuntimeError):
    """The script's own failure half way (docs/adr/1011)."""


def status_of(answer: dict[str, Any]) -> tuple[str, Any]:
    result = answer.get("result", answer)
    return result.get("status"), result


class AcceptanceDetail(unittest.TestCase):
    def test_the_plate_follows_its_parameters_and_refusals_write_nothing(self) -> None:
        doc = cad.Document.from_bytes(json.dumps(SCENARIO["setup"]).encode())
        names = dict(zip(SCENARIO["names"], [r.uid for r in doc.entities()]))
        tol = SCENARIO["tolerance"]
        for step in SCENARIO["steps"]:
            name = step["name"]
            before = doc.info()
            if "command" in step:
                expected = Expected(revision=step["expected"]["revision"]) if "expected" in step else None
                answer = doc.run(step["command"], bind(step["input"], names), request_id=step.get("requestId"),
                                 expected=expected)
                status, result = status_of(answer)
                if "refuse" in step:
                    self.assertIn(status, ("failed", "conflict"), f"{name}: {answer}")
                    self.assertEqual(result["error"]["code"], step["refuse"], name)
                    self.assertEqual(doc.info(), before, f"{name}: a refusal writes nothing")
                else:
                    self.assertEqual(status, "completed", f"{name}: {answer}")
                    if isinstance(step.get("as"), list):
                        names.update(zip(step["as"], result["output"]["created"]))
                    for path, want in step.get("output", {}).items():
                        self.assertEqual(at_path(result["output"], path), want, f"{name}: output.{path}")
                if step.get("twice"):
                    after = doc.info()
                    again = doc.run(step["command"], bind(step["input"], names), request_id=step["requestId"])
                    self.assertTrue(again.get("replayed"), f"{name}: answered from before")
                    self.assertEqual(doc.info(), after, f"{name}: applied once")
            elif "crash" in step:
                # The script runs the steps in a group and fails after the last; the group takes all of it back.
                crash = step["crash"]
                with self.assertRaises(ScriptCrash, msg=name):
                    with doc.group(crash["label"]):
                        for inner in crash["steps"]:
                            doc.run(inner["command"], bind(inner["input"], names))
                        raise ScriptCrash("betik yarıda çöktü")
                self.assertEqual(doc.info(), before, f"{name}: nothing of it is written")
            elif "job" in step:
                job = doc.start(step["job"]["command"], bind(step["job"]["input"], names))
                if step.get("cancel"):
                    job.cancel()
                self.assertEqual(job.wait().phase, "cancelled", name)
                self.assertEqual(doc.info(), before, f"{name}: nothing is written")
            elif "undo" in step:
                self.assertEqual(doc.undo(), step["undo"], name)
            elif step.get("reopen"):
                doc = cad.Document.from_bytes(doc.to_bytes())
            for key, want in step["expect"].get("points", {}).items():
                e = doc.entity(names[key]).entity.to_json()
                for end in ("a", "b"):
                    got = (e[end]["x"], e[end]["y"])
                    for g, w in zip(got, want[end]):
                        self.assertAlmostEqual(g, w, delta=tol, msg=f"{name}: {key}.{end}")
            for key, want in step["expect"].get("areas", {}).items():
                self.assertAlmostEqual(doc.measure(names[key]).area, want, delta=1e-6, msg=f"{name}: {key} area")
            for key, want in step["expect"].get("lengths", {}).items():
                self.assertAlmostEqual(doc.measure(names[key]).length, want, delta=1e-6, msg=f"{name}: {key}")


if __name__ == "__main__":
    unittest.main()
