"""A changed geometry's object as approved (`map.work.query`'s `changes.objects[].approved`, docs/adr/4480;
plan/map-cad.md MAP-01), held by an independent reading: the file is read by tools/kcad/kcad.py, the reader written
apart from KentOS, and what the query should say is worked out here from what it read.

The schema 211 sample (fixtures/kcad/v2/map-work-approved-parts.kcad: an approved design and its line) is approved
again, so its approval keeps its objects' values. Then:

- the line is moved 0.25 m east and 0.1 m south: its geometry changed, and the query gives it as approved — the
  approval's saved value, read as JSON, its ends where the reader read them before the move;
- its height set instead: only its attributes changed, the query gives no shape;
- moved and its height set: both, the attribute named and the shape given.
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))

from kentos import cad  # noqa: E402
from kentos.cad.types import MapWorkOperation  # noqa: E402

import kcad  # noqa: E402  (the independent reader)

SAMPLE = ROOT / "fixtures/kcad/v2/map-work-approved-parts.kcad"
LINE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13"


def design(doc: cad.Document) -> dict:
    return cad.map.work.query(doc).to_json()["items"][2]


def line_of(changes: dict) -> dict:
    return next(o for o in changes["objects"] if o["uid"] == LINE)


class ApprovedShape(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        approve = MapWorkOperation.from_json({"op": "set", "item": "is-3", "status": "approved"})
        cad.map.work.edit(self.doc, operation=approve)
        _, saved = kcad.read(self.doc.to_bytes())
        item = saved["settings"]["mapWork"]["items"][2]
        values = dict(zip((u for u, _ in item["approvedObjects"]), item["approvedValues"]))
        self.then = json.loads(values[LINE])
        self.read = dict(zip(saved["uids"], saved["entities"]))[LINE]

    def move(self) -> None:
        cad.entities.transform(self.doc, uids=[LINE], transform=cad.MoveTransform(dx=0.25, dy=-0.1))

    def test_a_moved_line_comes_as_approved(self) -> None:
        self.move()
        changed = line_of(design(self.doc)["changes"])
        self.assertEqual(changed["parts"], ["geometry"])
        self.assertEqual(changed["approved"], self.then)
        self.assertEqual((changed["approved"]["a"], changed["approved"]["b"]), (self.read["a"], self.read["b"]))

    def test_a_height_alone_gives_no_shape(self) -> None:
        cad.entities.set(self.doc, uids=[LINE], attrs={"Kot": "1060"}, operation="attributes")
        changed = line_of(design(self.doc)["changes"])
        self.assertEqual(changed["parts"], ["attributes"])
        self.assertNotIn("approved", changed)

    def test_moved_and_its_height_set_both_are_said(self) -> None:
        self.move()
        cad.entities.set(self.doc, uids=[LINE], attrs={"Kot": "1060"}, operation="attributes")
        changed = line_of(design(self.doc)["changes"])
        self.assertEqual(changed["parts"], ["geometry", "attributes"])
        self.assertEqual(changed["approved"], self.then)
        self.assertEqual([a["name"] for a in changed["attributes"]], ["Kot"])


if __name__ == "__main__":
    unittest.main()
