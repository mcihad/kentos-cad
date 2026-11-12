"""A kept adjustment (`cad.survey.adjust` with `keep`, docs/adr/5200; plan/map-cad.md MAP-18): the record the project
keeps is the answer itself (each new point's place and σ, f, m0, the test), read back by the independent `.kcad` reader;
a second write under the same name replaces it.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402

import kcad  # noqa: E402  (the independent reader)

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.survey.adjust.json").read_text("utf-8"))


class KeptAdjustment(unittest.TestCase):
    def test_the_record_is_the_answer(self) -> None:
        case = next(c for c in CASES["cases"] if c["name"].startswith("Yatay ağ sonucu projede saklanır"))
        setup = case.get("setup", CASES["setup"])
        doc = cad.Document.from_bytes(json.dumps(setup).encode())
        step = next(s for s in case["steps"] if s["op"] == "execute")
        out = cad.survey.adjust.execute_input(doc, cad.SurveyAdjust.from_json(step["input"])).to_json()
        self.assertEqual(out["kept"], "Poligon 2026/1")
        _, saved = kcad.read(doc.to_bytes())
        kept = saved["settings"]["surveyAdjustments"]
        self.assertEqual(len(kept), 1)
        self.assertEqual([(p["name"], p["at"], p["sigma"]) for p in kept[0]["points"]],
                         [(p["name"], p["at"], p["sigma"]) for p in out["points"]])
        self.assertEqual((kept[0]["f"], kept[0]["m0"], kept[0]["passed"]), (out["f"], out["m0"], out["passed"]))
        # The same name again replaces it.
        cad.survey.adjust.execute_input(doc, cad.SurveyAdjust.from_json(step["input"]))
        _, saved = kcad.read(doc.to_bytes())
        self.assertEqual(len(saved["settings"]["surveyAdjustments"]), 1)


if __name__ == "__main__":
    unittest.main()
