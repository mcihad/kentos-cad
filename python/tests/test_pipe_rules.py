"""Dış çap and the rule package (`cad.pipes.check`'s `diameter.outer` and `rules`, `cad.pipes.profile`'s own walls;
docs/adr/4780; plan/map-cad.md MAP-16) through the SDK, held by this test's own arithmetic (no KentOS code in the
expectations).

On the shared cases' outers drawing (scripts/fixtures/pipes_check_cases.py: the sewer's pipes carry `Et` and
`Dış çap`, in millimetres):

- a pipe whose outer diameter is its inner plus two walls answers its wall from `Dış çap` alone exactly as from `Et`;
  P4's outer is 10 mm more, so its wall from the outer alone is half the difference, and with both asked it is said
  (``wallMismatch``) and has none;
- with KentOS's sewer system (the 2017 regulation's EK-1, read here from catalog/v1/rules/kanal-csb-2017.json as data),
  each pipe's class is the first whose φ range holds its diameter in millimetres, and each slope above 1/greatest is
  ``steepSlope``, each one under 1/least ``lowSlope``; every cover under 2.70 m is ``shallow``;
- the run's own outer walls in the profile: two more thin red lines, the invert's polyline lowered by the wall and the
  crown's raised by it.
"""

from __future__ import annotations

import json
import sys
import unittest
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.pipes.check.json").read_text("utf-8"))
SETUP = next(c for c in CASES["cases"] if c["name"].startswith("dış çap: et dış çaptan"))["setup"]
PROFILES = json.loads((ROOT / "fixtures/commands/v1/cad.pipes.profile.json").read_text("utf-8"))
OWN = next(c for c in PROFILES["cases"] if c["name"].startswith("Hattın kendi borularının dış duvarı"))
PACKAGE = json.loads((ROOT / "catalog/v1/rules/kanal-csb-2017.json").read_text("utf-8"))
SEWER_IN = cad.PipeInverts.from_json({"from": "fields", "start": "Akar baş", "end": "Akar son"})
ZEMIN = cad.PipeGround.from_json({"from": "surface", "surface": "Zemin"})


def attrs() -> dict[str, dict]:
    """Each sewer pipe's attributes by its uid, read from the drawing by this test."""
    return {u: e["attrs"] for u, e in zip(SETUP["uids"], SETUP["entities"]) if e.get("layerId") == "atiksu"}


def diameter(**read: str) -> cad.PipeDiameter:
    return cad.PipeDiameter(field="Çap", unit=cad.PipeUnit.MM, **read)


class PipeRules(unittest.TestCase):
    def setUp(self) -> None:
        warnings.simplefilter("ignore", cad.CommandWarning)
        self.doc = cad.Document.from_bytes(kr.container(kr.document(SETUP)))

    def check(self, **ask) -> cad.PipesChecked:
        return cad.pipes.check(self.doc, network="atiksu", inverts=SEWER_IN, ground=ZEMIN, **ask)

    def test_the_wall_from_the_outer_diameter(self) -> None:
        by_et = {p.uid: p for p in self.check(diameter=diameter(wall="Et")).pipes}
        alone = {p.uid: p for p in self.check(diameter=diameter(outer="Dış çap")).pipes}
        both = {p.uid: p for p in self.check(diameter=diameter(wall="Et", outer="Dış çap")).pipes}
        agreeing = 0
        for uid, a in attrs().items():
            if "Dış çap" not in a or "Et" not in a:
                continue
            outer, inner, wall = float(a["Dış çap"]), float(a["Çap"]), float(a["Et"])
            if outer == inner + 2 * wall:
                agreeing += 1
                self.assertEqual(alone[uid], by_et[uid])
                self.assertEqual(both[uid], by_et[uid])
            elif outer > inner:
                self.assertAlmostEqual(alone[uid].wall, (outer - inner) / 2 / 1000.0, places=12)
                self.assertIn("wallMismatch", both[uid].findings)
                self.assertIs(both[uid].wall, cad.UNSET)
        self.assertGreaterEqual(agreeing, 4)

    def test_the_sewer_systems_limits(self) -> None:
        system = next(s for s in PACKAGE["systems"] if s["id"] == "atiksu")
        out = self.check(diameter=diameter(), rules=cad.PipeRulesAsk(system="atiksu"))
        mine = attrs()
        checked = 0
        for p in out.pipes:
            mm = float(mine[p.uid]["Çap"])
            holds = [c for c in system["slopes"]
                     if mm >= c.get("from", -1.0) and mm > c.get("over", -1.0) and mm <= c.get("to", float("inf"))]
            if not holds:
                self.assertIn("noRule", p.findings)
                continue
            c = holds[0]
            self.assertEqual(p.class_, c["name"])
            if p.slope is cad.UNSET or p.slope <= 0:
                continue
            checked += 1
            self.assertEqual("steepSlope" in p.findings, p.slope > 1.0 / c["greatest"])
            self.assertEqual("lowSlope" in p.findings, p.slope < 1.0 / c["least"])
            if p.min_cover is not cad.UNSET and p.min_cover >= 0:
                self.assertEqual("shallow" in p.findings, p.min_cover < system["minCover"])
        self.assertGreaterEqual(checked, 4)

    def test_the_runs_own_outer_walls(self) -> None:
        doc = cad.Document.from_bytes(kr.container(kr.document(OWN["setup"])))
        ask = OWN["steps"][0]["input"]
        plain = doc.run("cad.pipes.profile", {**ask, "diameter": {"field": "Çap", "unit": "mm"}})
        doc = cad.Document.from_bytes(kr.container(kr.document(OWN["setup"])))
        walled = doc.run("cad.pipes.profile", ask)
        self.assertEqual((plain["status"], walled["status"]), ("completed", "completed"))
        made = [doc.entity(u).entity.to_json() for u in walled["output"]["uids"]]
        red = [e for e in made if e["kind"] == "polyline" and e.get("color") == "#d62828"]
        thin = [e for e in red if e.get("lineWeight") == 0.13]
        self.assertEqual(len(thin), 2)
        invert = next(e for e in red if e.get("lineWeight") == 0.5)
        crown = next(e for e in red if e.get("lineWeight") == 0.25)
        bottom, top = sorted(thin, key=lambda e: e["pts"][0]["y"])
        k = walled["output"]["exaggeration"]
        walls = [p["wall"] for p in walled["output"]["pipes"]]
        # Each pipe's two ends along the run: the outer bottom is the invert lowered by its wall times the
        # exaggeration, the outer top the crown raised by it.
        for i, w in enumerate(w for w in walls for _ in (0, 1)):
            self.assertAlmostEqual(invert["pts"][i]["y"] - bottom["pts"][i]["y"], w * k, places=9)
            self.assertAlmostEqual(top["pts"][i]["y"] - crown["pts"][i]["y"], w * k, places=9)
            self.assertAlmostEqual(bottom["pts"][i]["x"], invert["pts"][i]["x"], places=9)


if __name__ == "__main__":
    unittest.main()
