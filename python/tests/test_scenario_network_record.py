"""Senaryo ölçüleri: ağ üzerinden erişim, eğim haritası ve ölçülerin çizime yazılması (`cad.scenarios.impact`'in
`access.network` ve `slope.map`'i, `cad.scenarios.record`; docs/adr/5060, plan/map-cad.md MAP-10), held by the test's
own computation (no KentOS code in the expectations).

The drawings are the shared cases' (fixtures/commands/v1/cad.scenarios.impact.json):

- the access along the road network “Yollar”: the test lays the scenario's roads out itself (along y = −5 to x = 105,
  then north along x = 105) and walks them by hand; each area's way is its centroid's leg to the nearest road, the way
  along the roads and the target's leg; every route's points add up to its distance; a centroid farther from the roads
  than the leg allowed is off the network;
- the slope map: each part's size is its ring's shoelace, the parts of a class add up to the class's measure;
- the table written: its cells are the measures as the project writes them, the comparison reads it back as current,
  and stale once the base changes; undone, it is gone.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.scenarios.impact.json").read_text("utf-8"))
NET = next(c for c in CASES["cases"] if c["name"].startswith("ağ üzerinden erişim: konuttan"))
MAP = next(c for c in CASES["cases"] if c["name"].startswith("eğim haritası: alanların"))
X0, Y0 = 487000.0, 4420000.0


def centroid(e) -> tuple[float, float]:
    xs = [p["x"] - X0 for p in e["pts"]]
    ys = [p["y"] - Y0 for p in e["pts"]]
    return (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2


def by_hand(c: tuple[float, float], t: tuple[float, float]) -> float:
    """The way from centroid c to centroid t along the scenario's roads: y = −5 for x in [0, 105], x = 105 for y in
    [−5, 60]; each centroid joins its nearest road square to it."""
    def place(p):
        on_x = (min(max(p[0], 0.0), 105.0), -5.0)
        on_y = (105.0, min(max(p[1], -5.0), 60.0))
        a, b = math.dist(p, on_x), math.dist(p, on_y)
        return (on_x, a, "x") if a < b else (on_y, b, "y")
    (pa, la, ra), (pb, lb, rb) = place(c), place(t)
    if ra == rb:
        way = math.dist(pa, pb)
    else:
        corner = (105.0, -5.0)
        way = math.dist(pa, corner) + math.dist(corner, pb)
    return la + way + lb


class ScenarioNetworkRecord(unittest.TestCase):
    def test_the_access_along_the_roads(self) -> None:
        doc = cad.Document.from_bytes(kr.container(kr.document(NET["setup"])))
        access = {"distance": 100.0, "from": ["konut"], "to": ["park", "yesil"], "network": "yollar", "connect": 40.0}
        out = cad.scenarios.impact(doc, scenario="alt-a", access=cad.ImpactAccess.from_json(access)).to_json()
        uids = dict(zip(NET["setup"]["uids"], NET["setup"]["entities"]))
        routes = out["reach"]["alternativeRoutes"]
        self.assertEqual(len(routes), 3)
        for r in routes:
            c, t = centroid(uids[r["area"]]), centroid(uids[r["target"]])
            self.assertAlmostEqual(r["distance"], by_hand(c, t), places=6, msg=r["area"])
            walked = sum(math.dist(p, q) for p, q in zip(r["path"], r["path"][1:]))
            self.assertAlmostEqual(walked, r["distance"], places=6)
            # Straight is never longer than along the roads.
            self.assertLessEqual(math.dist(c, t), r["distance"] + 1e-9)
        reached = {r["area"] for r in routes if r["distance"] <= 100.0}
        self.assertEqual(set(out["reach"]["alternativeReached"]), reached)
        # 25 m allowed: the two western konut areas' centroids are 30 m from the road.
        access["connect"] = 25.0
        out = cad.scenarios.impact(doc, scenario="alt-a", access=cad.ImpactAccess.from_json(access)).to_json()
        self.assertEqual(len(out["reach"]["alternativeOff"]), 2)
        self.assertTrue(any("Ağa bağlanamayan" in g for g in out["gaps"]))

    def test_the_slope_map_parts(self) -> None:
        doc = cad.Document.from_bytes(kr.container(kr.document(MAP["setup"])))
        slope = {"surface": "t-1", "threshold": 10.0, "classes": [5.0, 15.0, 30.0], "aspect": True, "map": True}
        out = cad.scenarios.impact(doc, scenario="alt-a", slope=cad.ImpactSlope.from_json(slope)).to_json()
        measures = {m["id"]: m for m in out["measures"]}
        for side in ("base", "alternative"):
            by_class = [0.0] * 4
            for p in out["slopeMap"][side]:
                ring = p["ring"]
                shoelace = sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(ring, ring[1:] + ring[:1])) / 2
                self.assertGreater(shoelace, 0.0)  # counter-clockwise
                self.assertAlmostEqual(shoelace, p["size"], delta=1e-3)
                by_class[p["class"]] += p["size"]
            for k in range(4):
                self.assertAlmostEqual(by_class[k], measures[f"slopeClass{k + 1}"][side], delta=1e-6)

    def test_the_table_written_and_read_back(self) -> None:
        doc = cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))
        impact = cad.ScenariosImpact.from_json({"scenario": "alt-a"})
        written = cad.scenarios.record(doc, impact=impact, layer_id="0").to_json()
        out = cad.scenarios.impact(doc, scenario="alt-a").to_json()
        self.assertEqual(len(written["cells"]), len(out["measures"]) + 2)
        area = next(m for m in out["measures"] if m["id"] == "area")
        row = next(r for r in written["cells"] if r[0] == area["label"])
        self.assertEqual(row[2:4], [f"{area['base']:.2f}", f"{area['alternative']:.2f}"])
        self.assertEqual(written["cells"][-1][2:4], [out["stamp"], out["scenarioStamp"]])
        [record] = out["records"]
        self.assertEqual(record["table"], written["table"])
        self.assertFalse(record["baseChanged"] or record["scenarioChanged"])
        self.assertEqual(doc.undo(), "Ölçüleri çizime yaz")
        self.assertNotIn("records", cad.scenarios.impact(doc, scenario="alt-a").to_json())
        answer = cad.command("cad.scenarios.record").run(
            doc, cad.ScenariosRecord.from_json({"impact": {"scenario": "alt-a"}, "layerId": "karar"}))
        self.assertEqual((answer.error.code, answer.error.path), ("invalid_input", "layerId"))


if __name__ == "__main__":
    unittest.main()
