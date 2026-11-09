"""Tariften üret (`road.project.produce`, docs/adr/3240; plan/map-cad.md MAP-20): the same small production job in two
projects, held by an independent reading (no KentOS code in the expectations).

Project A is scripts/fixtures/road_terrain_zeroline_cases.py's Yol project with its surfaces under other ids (a-5 Düz,
a-6 Teras, a-9 Set, a-10 Tümsek); its zero lines are drawn there and İş grafiği's graph is saved as JSON with Python's
own `json`, as Tarifi kaydet… would. Project B is the same project with its own ids (t-5, t-6, t-9, t-10). Produced in
B from A's recipe:

- each drawing's main source is B's surface of the same name (the recipe's id there given by its name);
- the zero lines are the reference's: one closed line under Tümsek, two open lines under Set, each line's plan length
  the reference's (road_terrain_zeroline_cases.py);
- B's objects are A's, coordinate for coordinate (the surfaces are the same lattices);
- B's graph holds the drawings as its own, their inputs by B's ids, current.
"""

from __future__ import annotations

import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import road_terrain_zeroline_cases as zc  # noqa: E402

OTHER = {"t-5": "a-5", "t-6": "a-6", "t-9": "a-9", "t-10": "a-10"}
PAIRS = [("Düz", "Tümsek"), ("Teras", "Set")]


def project(ids: dict[str, str] | None = None) -> cad.Document:
    setup = zc.setup(work="road")
    for s in setup["settings"]["road"]["surfaces"]:
        s["id"] = (ids or {}).get(s["id"], s["id"])
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def lines_of(doc: cad.Document, uids: list[str]) -> list[list[tuple[float, float]]]:
    """The objects' places, in order."""
    out = []
    for uid in uids:
        e = doc.entity(uid).entity.to_json()
        out.append([(p["x"], p["y"]) for p in e["pts"]])
    return out


def plan_length(pts: list[tuple[float, float]], closed: bool) -> float:
    run = sum(math.dist(pts[k], pts[k + 1]) for k in range(len(pts) - 1))
    return run + (math.dist(pts[-1], pts[0]) if closed and len(pts) > 2 else 0.0)


class RecipeProduce(unittest.TestCase):
    def test_the_recipe_of_one_project_draws_the_same_lines_in_another(self) -> None:
        a = project(OTHER)
        drawn = [cad.road.terrains.zeroline(a, base=b, comparison=c, layer_id="sifir") for b, c in PAIRS]
        graph = cad.road.project.graph(a)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "tarif.json"
            path.write_text(json.dumps(graph.to_json(), ensure_ascii=False), "utf-8")
            recipe = json.loads(path.read_text("utf-8"))
            ids = [n["recipe"]["input"]["base"] for n in recipe["nodes"] if n["kind"] == "drawing"]
            self.assertEqual(ids, ["a-5", "a-6"], "A keeps its own ids")
            b = project()
            out = cad.road.project.produce(b, path=str(path))
        self.assertEqual([(p.source.id, p.source.name) for p in out.produced], [("t-5", "Düz"), ("t-6", "Teras")])
        for (base, comparison), p, mine in zip(PAIRS, out.produced, drawn):
            want = zc.answer({"base": base, "comparison": comparison})["lines"]
            got = lines_of(b, list(p.uids))
            self.assertEqual(len(got), len(want), base)
            for pts, w in zip(got, want):
                self.assertAlmostEqual(plan_length(pts, w["closed"]), w["length"], places=6)
            self.assertEqual(got, lines_of(a, list(mine.uids)), "B's lines are A's")
        tree = cad.road.project.graph(b)
        inputs = [n.recipe.input for n in tree.nodes if n.kind == "drawing"]
        self.assertEqual([i["base"] for i in inputs], ["t-5", "t-6"], "kept as B's")
        self.assertEqual((tree.drawings, tree.outdated), (2, 0))


if __name__ == "__main__":
    unittest.main()
