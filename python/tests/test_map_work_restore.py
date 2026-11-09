"""An approved item's objects put back as approved (`map.work.restore`, docs/adr/4080; plan/map-cad.md MAP-01), held by
an independent reading: the file is read by tools/kcad/kcad.py, the reader written apart from KentOS, and the objects
are judged here against the approval's own digests (SHA-256 of the object without its slot id, keys sorted, compact
JSON: `kentos.object-digest/1`).

The schema 211 sample (fixtures/kcad/v2/map-work-approved-parts.kcad: the approved design's line) is approved again so
its approval keeps its values. Then the line's height is set, and it is restored; then deleted, a line is added to the
design's layer, and restored:

- the changed line comes back to its approved value: its digest is the approval's again, its slot the same;
- the deleted line comes back with its persistent id; saved and read by the independent reader, it is the approved
  value itself (without its slot id) and its digest the approval's; the line added after stays and is counted;
- one undo takes the restore back; the query then names the line as removed again;
- a second restore finds nothing to put back.
"""

from __future__ import annotations

import hashlib
import json
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


def digest(e: dict) -> str:
    """The object digest, worked out here: the object without its slot id, keys sorted, compact JSON, SHA-256."""
    text = json.dumps({k: v for k, v in e.items() if k != "id"}, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


class Restore(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        approve = MapWorkOperation.from_json({"op": "set", "item": "is-3", "status": "approved"})
        cad.map.work.edit(self.doc, operation=approve)
        _, saved = kcad.read(self.doc.to_bytes())
        self.item = saved["settings"]["mapWork"]["items"][2]
        self.approved = dict(map(tuple, self.item["approvedObjects"]))
        self.layer = saved["entities"][saved["uids"].index(LINE)]["layerId"]

    def read(self) -> dict:
        _, saved = kcad.read(self.doc.to_bytes())
        return dict(zip(saved["uids"], saved["entities"]))

    def test_a_changed_line_comes_back_to_its_value(self) -> None:
        slot = self.read()[LINE]["id"]
        cad.entities.set(self.doc, uids=[LINE], attrs={"Kot": "1060"}, operation="attributes")
        self.assertNotEqual(digest(self.read()[LINE]), self.approved[LINE])
        out = cad.map.work.restore(self.doc, item="is-3")
        self.assertEqual((list(out.restored), list(out.reverted), out.added, out.as_approved), ([], [LINE], 0, True))
        now = self.read()[LINE]
        self.assertEqual(digest(now), self.approved[LINE])
        self.assertEqual(now["id"], slot)

    def test_a_deleted_line_comes_back_with_its_id_and_the_added_one_stays(self) -> None:
        then = json.loads(self.item["approvedValues"][[u for u, _ in self.item["approvedObjects"]].index(LINE)])
        cad.entities.delete(self.doc, uids=[LINE])
        made = cad.line.create(self.doc, layer_id=self.layer, a=(then["a"]["x"], then["a"]["y"] + 5.0),
                               b=(then["b"]["x"], then["b"]["y"] + 5.0))
        out = cad.map.work.restore(self.doc, item="is-3")
        self.assertEqual((list(out.restored), list(out.reverted), out.added, out.as_approved), ([LINE], [], 1, False))
        objects = self.read()
        self.assertEqual({k: v for k, v in objects[LINE].items() if k != "id"}, then)
        self.assertEqual(digest(objects[LINE]), self.approved[LINE])
        self.assertIn(made.uid, objects, "the line added after the approval stays")
        # One undo takes it back: the query names the line as removed again.
        self.doc.undo()
        removed = [o["uid"] for o in cad.map.work.query(self.doc).to_json()["items"][2]["changes"]["removedObjects"]]
        self.assertEqual(removed, [LINE])
        self.doc.redo()
        with self.assertRaises(CommandFailed) as nothing:
            cad.map.work.restore(self.doc, item="is-3")
        self.assertEqual((nothing.exception.code, nothing.exception.path), ("nothing_to_restore", "item"))


if __name__ == "__main__":
    unittest.main()
