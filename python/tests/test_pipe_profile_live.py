"""A pipe run profile kept with its network (`cad.pipes.profile`, docs/adr/3560; plan/map-cad.md MAP-16) through the SDK,
on pipes_profile_cases.py's Harita project (the shared case's drawing, written as a file by the independent writer).

- Drawn, the profile is a drawing of the project's tree under the sewer network, up to date.
- P1's material written (an attribute the profile does not read) makes it stale, its network named.
- road.project.rebuild draws it again in place, as many objects; the tree is up to date.
- The project written as a .kcad and read back keeps the drawing with its network (schema 212).
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

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.pipes.profile.json").read_text())
P1 = "0192f640-1640-7000-8000-0000000000f1"
PROFILE = {"network": "atiksu", "start": P1, "inverts": {"from": "fields", "start": "Akar baş", "end": "Akar son"},
           "diameter": {"field": "Çap", "unit": "mm"}, "ground": {"from": "surface", "surface": "Zemin"},
           "layerId": "profil", "at": [487000.0, 4419700.0]}


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))


def rows(doc: cad.Document) -> list:
    return cad.road.project.query.execute_input(doc, cad.RoadProjectQuery.from_json({})).nodes


class PipeProfileLive(unittest.TestCase):
    def test_the_profile_follows_its_network(self) -> None:
        doc = drawing()
        drawn = cad.pipes.profile.execute_input(doc, cad.PipesProfile.from_json(PROFILE))
        self.assertEqual(len(drawn.uids), 112)
        tree = rows(doc)
        network = next(i for i, n in enumerate(tree) if n.kind == "network")
        profile = next(n for n in tree if n.kind == "drawing")
        self.assertEqual((tree[network].name, profile.name, profile.state, profile.parent),
                         ("Atıksu", "Hat profili · Profil", "current", network))
        cad.entities.set.execute_input(doc, cad.EntitiesSetProperties.from_json(
            {"uids": [P1], "attrs": {"Malzeme": "PVC"}, "operation": "attributes"}))
        profile = next(n for n in rows(doc) if n.kind == "drawing")
        self.assertEqual(profile.state, "stale")
        self.assertEqual(list(profile.why), ["“Atıksu” ağı çizildiğinden beri değişti."])
        again = cad.road.project.rebuild.execute_input(doc, cad.RoadProjectRebuild.from_json({}))
        self.assertEqual([(r.command, r.removed, len(r.uids)) for r in again.rebuilt], [("cad.pipes.profile", 112, 112)])
        self.assertTrue(all(n.state == "current" for n in rows(doc) if n.kind == "drawing"))
        # Written and read back: the drawing kept with its network, up to date.
        back = cad.Document.from_bytes(doc.to_bytes())
        profile = next(n for n in rows(back) if n.kind == "drawing")
        self.assertEqual((profile.name, profile.state), ("Hat profili · Profil", "current"))


if __name__ == "__main__":
    unittest.main()
