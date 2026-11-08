"""`road.project.rebuild` as a job from Python (plan/map-cad.md MAP-20, docs/adr/2440), on the shared cases' road
project with its corridor and guardrail drawn and the template they read renamed: two drawings stale (the case “İki
eskiyen çizim birlikte, tek adımda”).

- A job stopped as it starts is cancelled and writes nothing: the drawing's bytes are the same.
- A job let run is done with its progress whole; it rebuilds what a plain call rebuilds on a copy (the drawings and
  the objects each took away), so the GUI's job, Python's job and a plain call give one result.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/road.project.rebuild.json").read_text("utf-8"))
STEPS = [
    ("road.corridors.draw", {"corridor": "c-1", "layerId": "0"}),
    ("road.guardrails.draw", {"guardrail": "o-1", "layerId": "korkuluk"}),
    ("road.templates.edit", {"template": "tk-2", "operation": "set", "name": "Tip 2b"}),
]


def stale() -> cad.Document:
    doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))
    for command, payload in STEPS:
        out = doc.call(command, payload)
        assert out.ok, (command, out)
    return doc


def drawn(output) -> list[tuple[str, int]]:
    data = output.to_json() if hasattr(output, "to_json") else output
    return [(r["drawing"], r["removed"]) for r in data["rebuilt"]]


class RebuildAsAJob(unittest.TestCase):
    def test_a_stopped_job_writes_nothing(self) -> None:
        doc = stale()
        before = doc.to_bytes()
        job = doc.start("road.project.rebuild", {})
        job.cancel()
        status = job.wait()
        self.assertEqual(status.phase, "cancelled")
        self.assertEqual(doc.to_bytes(), before)

    def test_a_job_let_run_rebuilds_what_a_call_does(self) -> None:
        doc = stale()
        status = doc.start("road.project.rebuild", {}).wait()
        self.assertEqual(status.phase, "done")
        self.assertEqual(status.progress, 1.0)
        w = cad.command("road.project.rebuild")
        plain = w.run(stale(), w.input_type.from_json({}), op="execute")
        self.assertIsNone(plain.error, plain.error and plain.error.message)
        self.assertEqual(drawn(status.answer.output), drawn(plain.output))
        self.assertEqual([d for d, _ in drawn(plain.output)], ["cz-1", "cz-2"])


if __name__ == "__main__":
    unittest.main()
