"""Scenario areas joined to their nearest infrastructure line (`cad.scenarios.impact`, docs/adr/3460; plan/map-cad.md
MAP-10) through the SDK, held by scripts/fixtures/scenarios_impact_cases.py's own reading (no KentOS code in the
expectations).

- The sewer line as the infrastructure: every measured area on each side joined to it, the reading's nearest place
  and distance (the centroid's perpendicular onto the line).
- The roads, the scenario with two: each area to the nearer road; the area as far from both goes to the first by the
  drawing's order.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import scenarios_impact_cases as sc  # noqa: E402


def links_of(setup, infrastructure):
    doc = cad.Document.from_bytes(kr.container(kr.document(setup)))
    out = cad.scenarios.impact(doc, scenario="alt-a", infrastructure=cad.ImpactInfrastructure.from_json(infrastructure))
    want = sc.report(setup, infrastructure=infrastructure)["near"]
    return out.near, want


class ImpactLinks(unittest.TestCase):
    def check(self, got, want) -> None:
        self.assertEqual(len(got), len(want))
        for g, w in zip(got, want):
            self.assertEqual((g.area, g.line), (w["area"], w["line"]))
            for a, b in ((g.from_, w["from"]), (g.to, w["to"])):
                self.assertAlmostEqual(a[0], b[0], places=6)
                self.assertAlmostEqual(a[1], b[1], places=6)
            self.assertAlmostEqual(g.distance, w["distance"], places=6)

    def test_the_sewer_line(self) -> None:
        near, want = links_of(sc.INFRA_SETUP, {"layer": "altyapi", "distance": 40.0})
        self.check(near.base_links, want["baseLinks"])
        self.check(near.alternative_links, want["alternativeLinks"])

    def test_the_roads_each_area_to_the_nearer(self) -> None:
        near, want = links_of(sc.BASE_SETUP, {"layer": "yol", "distance": 30.0})
        self.check(near.alternative_links, want["alternativeLinks"])
        self.assertEqual(len({l.line for l in near.alternative_links}), 2, "both roads are someone's nearest")


if __name__ == "__main__":
    unittest.main()
