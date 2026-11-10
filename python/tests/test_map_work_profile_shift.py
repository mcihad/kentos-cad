"""A map work item's typed layers' profile and how far a changed object moved (`map.work.edit`'s `profile`,
`map.work.query`'s `profile` and `changes.objects[].shift`, docs/adr/4880; plan/map-cad.md MAP-01), held by an
independent reading: the file is read by tools/kcad/kcad.py, the reader written apart from KentOS, and what the answers
should say is worked out here from what it read and from the profile's own file.

- The schema 211 sample's approved design (fixtures/kcad/v2/map-work-approved-parts.kcad) is approved again, so its
  approval keeps its objects' values, and its line moved 0.3 m east and 0.4 m north: its two ends pair, each moved
  0.5 m (the reader's ends before and after); the Hausdorff distance of two straight segments is reached at an end of
  one of them, worked out here from the four ends.
- The design takes the map assets profile's control points: the saved file names the profile, its layer is the one at
  the class's path in the reader's tree, holding the class's fields (the profile file's names); a point there without
  its name is a finding, and approving the design is refused.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import MapWorkOperation  # noqa: E402

import kcad  # noqa: E402  (the independent reader)

SAMPLE = ROOT / "fixtures/kcad/v2/map-work-approved-parts.kcad"
LINE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13"
PROFILE = json.loads((ROOT / "catalog/v1/profiles/harita-varliklari.json").read_text("utf-8"))
CLASS = next(c for c in PROFILE["classes"] if c["id"] == "kontrol-noktasi")


def edit(doc: cad.Document, **operation) -> None:
    cad.map.work.edit(doc, operation=MapWorkOperation.from_json(operation))


def design(doc: cad.Document) -> dict:
    return cad.map.work.query(doc).to_json()["items"][2]


def to_segment(p, a, b) -> float:
    """The distance from p to the segment a–b."""
    vx, vy = b[0] - a[0], b[1] - a[1]
    t = max(0.0, min(1.0, ((p[0] - a[0]) * vx + (p[1] - a[1]) * vy) / (vx * vx + vy * vy)))
    return math.hypot(p[0] - a[0] - vx * t, p[1] - a[1] - vy * t)


def ends(e: dict):
    return (e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])


def find(nodes: list, path: list):
    """The layer at `path` (names) in the reader's tree."""
    here, node = nodes, None
    for name in path:
        node = next(n for n in here if n["name"] == name)
        here = node.get("children", [])
    return node


class Shift(unittest.TestCase):
    def test_a_moved_line_s_ends_pair_and_its_outlines_are_the_move_apart(self) -> None:
        doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        edit(doc, op="set", item="is-3", status="approved")
        _, before = kcad.read(doc.to_bytes())
        cad.entities.transform(doc, uids=[LINE], transform=cad.MoveTransform(dx=0.3, dy=0.4))
        _, after = kcad.read(doc.to_bytes())
        a0, b0 = ends(dict(zip(before["uids"], before["entities"]))[LINE])
        a1, b1 = ends(dict(zip(after["uids"], after["entities"]))[LINE])
        changed = next(o for o in design(doc)["changes"]["objects"] if o["uid"] == LINE)
        shift = changed["shift"]
        self.assertTrue(shift["paired"])
        self.assertEqual(shift["vertices"], [2, 2])
        moves = [math.dist(a0, a1), math.dist(b0, b1)]
        self.assertAlmostEqual(shift["largest"]["d"], max(moves), delta=1e-9)
        self.assertAlmostEqual(shift["rms"], math.sqrt((moves[0] ** 2 + moves[1] ** 2) / 2), delta=1e-9)
        mine = max(to_segment(a0, a1, b1), to_segment(b0, a1, b1), to_segment(a1, a0, b0), to_segment(b1, a0, b0))
        self.assertAlmostEqual(shift["hausdorff"]["d"], mine, delta=1e-8)


class Profile(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        edit(self.doc, op="profile", item="is-3", name="harita-varliklari", classes=["kontrol-noktasi"])
        _, self.saved = kcad.read(self.doc.to_bytes())
        self.layer = find(self.saved["layers"], CLASS["layer"])

    def test_the_class_s_layer_joins_the_item_with_its_fields(self) -> None:
        item = self.saved["settings"]["mapWork"]["items"][2]
        self.assertEqual(item["profile"], {"name": "harita-varliklari", "classes": ["kontrol-noktasi"]})
        self.assertIn(self.layer["id"], item["layers"])
        self.assertEqual([f["name"] for f in self.layer["fields"]], [f["name"] for f in CLASS["fields"]])

    def test_a_point_without_its_name_is_a_finding_and_stops_the_approval(self) -> None:
        point = cad.NewObject.from_json({"geometry": {"kind": "point", "p": {"x": 500003.0, "y": 4400003.0}}})
        cad.entities.create(self.doc, layer_id=self.layer["id"], objects=[point])
        row = design(self.doc)["profile"]
        self.assertTrue(row["found"])
        self.assertIn("requiredEmpty", [f["kind"] for f in row["findings"]])
        with self.assertRaises(CommandFailed) as refused:
            edit(self.doc, op="set", item="is-3", status="approved")
        self.assertEqual((refused.exception.code, refused.exception.path), ("approval_profile", "operation.status"))


if __name__ == "__main__":
    unittest.main()
