"""Çalışma alanı ve DBSCAN parametre taraması from Python (plan/gis.md CBS-25, docs/adr/0574).

The shared cases of En yakın komşu over a study area and DBSCAN's sweep (fixtures/processing/v1/spatial-inference.json,
written by scripts/fixtures/spatial_study_cases.py without KentOS code), run through ``cad.processing`` on the shared
drawing: the same runner the desktop's İşlemler window runs, the same summaries, tables and numbers.
"""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

import kentos.cad as cad
from kentos.cad.errors import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
FILE = json.loads((ROOT / "fixtures/processing/v1/spatial-inference.json").read_text(encoding="utf-8"))
DRAWING = ROOT / "fixtures/processing/v1/spatial-stats.kcad"
CASES = [c for c in FILE["cases"] if c["id"].startswith(("nn-study", "sweep-"))]


class SpatialStudyTests(unittest.TestCase):
    def test_there_are_cases(self) -> None:
        self.assertGreaterEqual(len(CASES), 14)

    def test_the_shared_cases_run_headless(self) -> None:
        for case in CASES:
            if "selection" in case:
                continue  # a selection is slots in the shared cases; the command's equivalent is in the Rust equivalence test
            with self.subTest(case["id"]):
                doc = cad.Document.open(DRAWING)
                expect: dict[str, Any] = case["expect"]
                params = {"tool": case["run"]["tool"], "parameters": case["values"]}
                if expect["status"] in ("error", "invalid"):
                    message = expect["message"] if expect["status"] == "error" else expect["issues"][0]["message"]
                    with self.assertRaises(CommandFailed) as caught:
                        cad.processing.run(doc, **params)
                    self.assertIn(message, str(caught.exception))
                    continue
                ran = cad.processing.run(doc, **params)
                self.assertEqual(ran.summary, expect["summary"])
                outputs = ran.outputs or {}
                for name, want in expect["outputs"].items():
                    if name == "experiment":
                        self.assertEqual(json.loads(outputs[name]), json.loads(want), name)
                    elif isinstance(want, dict):
                        self.assertEqual(outputs[name]["rows"], want["rows"], name)
                    else:
                        self.assertAlmostEqual(outputs[name], want, places=9, msg=name)

    def test_a_sweep_suggests_a_pair_in_its_grid(self) -> None:
        doc = cad.Document.open(DRAWING)
        ran = cad.processing.run(
            doc, tool="stats.dbscanSweep", parameters={"input": {"scope": "layer", "layerId": "kaza"}}
        )
        out = ran.outputs or {}
        radii = {row[1] for row in out["table"]["rows"]}
        self.assertEqual(out["cells"], 42)
        self.assertEqual(out["suggestedMinPoints"], 4)
        self.assertIn(f"{out['suggestedRadius']:.3f}", radii)


if __name__ == "__main__":
    unittest.main()
