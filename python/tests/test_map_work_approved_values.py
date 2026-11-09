"""A map work approval's values (`.kcad` schema 213, docs/adr/3680; plan/map-cad.md MAP-01), held by an independent
reading: the file is read by tools/kcad/kcad.py, the reader written apart from KentOS, and the changes are worked out
here from what it read.

The schema 211 sample (fixtures/kcad/v2/map-work-approved-parts.kcad: the approved design keeps its line's digest and
parts, no values) is opened and approved again. Saved:

- the file is schema 213 and the design's approval keeps one value per approved object, each the very text whose
  SHA-256 is the object's digest, and that text read as JSON is the object as the reader reads it, without its slot id;

then the line's height is set and `map.work.query` names the attribute with its value then (none) and now, worked out
here from the saved value; the line deleted, the query gives the line as approved: the saved value read as JSON.
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
from kentos.cad.types import MapWorkOperation  # noqa: E402

import kcad  # noqa: E402  (the independent reader)

SAMPLE = ROOT / "fixtures/kcad/v2/map-work-approved-parts.kcad"
LINE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e13"


def design(doc: cad.Document) -> dict:
    return cad.map.work.query(doc).to_json()["items"][2]


class ApprovedValues(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(SAMPLE.read_bytes())
        approve = MapWorkOperation.from_json({"op": "set", "item": "is-3", "status": "approved"})
        cad.map.work.edit(self.doc, operation=approve)
        self.bytes = self.doc.to_bytes()
        header, self.saved = kcad.read(self.bytes)
        self.item = self.saved["settings"]["mapWork"]["items"][2]

    def test_the_values_are_the_texts_the_digests_are_of(self) -> None:
        # The payload's schema: CBOR's "version" key, then 213 (0x18 0xd5).
        self.assertIn(b"\x67version\x18\xd5", self.bytes)
        values, pairs = self.item["approvedValues"], self.item["approvedObjects"]
        self.assertEqual(len(values), len(pairs))
        entities = dict(zip(self.saved["uids"], self.saved["entities"]))
        for text, (uid, digest) in zip(values, pairs):
            self.assertEqual(hashlib.sha256(text.encode("utf-8")).hexdigest(), digest)
            self.assertEqual(json.loads(text), {k: v for k, v in entities[uid].items() if k != "id"})

    def test_the_changed_attribute_and_the_deleted_line(self) -> None:
        then = json.loads(self.item["approvedValues"][0])
        cad.entities.set(self.doc, uids=[LINE], attrs={"Kot": "1060"}, operation="attributes")
        changed = design(self.doc)["changes"]["objects"][0]
        want = [{"name": k, **({"approved": then["attrs"][k]} if k in then["attrs"] else {}), "now": v}
                for k, v in sorted({"Kot": "1060"}.items()) if then["attrs"].get(k) != v]
        self.assertEqual(changed["attributes"], want)
        cad.entities.delete(self.doc, uids=[LINE])
        removed = design(self.doc)["changes"]["removedObjects"]
        self.assertEqual(removed, [{"uid": LINE, "approved": then}])


if __name__ == "__main__":
    unittest.main()
