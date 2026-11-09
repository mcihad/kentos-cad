"""Altyapı bağlarını çiz from a script (`cad.processing.run` with `scenarios.links`, docs/adr/3860; plan/map-cad.md
MAP-10), held by `cad.scenarios.impact`'s shared case (its independent reference's links), no KentOS code in the
expectations.

On the scenario case's drawing (fixtures/processing/v1/scenario-links.kcad), the sewer line 40 m: each of the case's
links on both sides is a line from the area's centroid to the line's nearest place, its length the link's distance by
this test's own arithmetic; only the scenario's side gives only its links; one undo step takes them away.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

import kentos.cad as cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

FOLDER = ROOT / "fixtures/processing/v1"
CASE = "altyapıya yakınlık: kanalizasyon hattına 40 m"


def drawing() -> cad.Document:
    setup = json.loads((FOLDER / "scenario-links.kcad").read_text("utf-8"))
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def near() -> dict:
    cases = json.loads((ROOT / "fixtures/commands/v1/cad.scenarios.impact.json").read_text("utf-8"))["cases"]
    return next(c for c in cases if c["name"].startswith(CASE))["steps"][0]["result"]["output"]["near"]


PARAMS = {"scenario": "Alternatif A", "infrastructure": {"scope": "layer", "layerId": "altyapi"}, "distance": 40}


class ScenarioLinks(unittest.TestCase):
    def test_each_link_is_a_line_from_the_centroid_to_the_line(self) -> None:
        doc = drawing()
        before = len(list(doc.entities()))
        ran = cad.processing.run(doc, tool="scenarios.links", parameters=PARAMS)
        links = near()["baseLinks"] + near()["alternativeLinks"]
        self.assertEqual(len(ran.added), len(links))
        self.assertEqual(ran.step, "Altyapı bağlarını çiz")
        for uid, link in zip(ran.added, links):
            e = doc.entity(uid).entity.to_json()
            self.assertEqual((e["attrs"]["Alan"], e["attrs"]["Hat"]), (link["area"], link["line"]))
            self.assertAlmostEqual(math.dist((e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])), link["distance"], places=9)
        doc.undo()
        self.assertEqual(len(list(doc.entities())), before, "one step")

    def test_the_scenario_s_side_alone(self) -> None:
        doc = drawing()
        ran = cad.processing.run(doc, tool="scenarios.links", parameters={**PARAMS, "side": "alternative"})
        self.assertEqual(len(ran.added), len(near()["alternativeLinks"]))
        self.assertTrue(all(doc.entity(u).entity.to_json()["attrs"]["Taraf"] == "Senaryo" for u in ran.added))


if __name__ == "__main__":
    unittest.main()
