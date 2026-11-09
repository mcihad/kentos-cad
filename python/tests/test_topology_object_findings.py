"""One object's topology findings (`cad.topology.check` with `uids`, docs/adr/3800; plan/map-cad.md MAP-07) through the
SDK, held by this test's own reading of the whole check (no KentOS code in the expectations).

On the shared cases' drawing (five parcels; A1–A2 and A3–A4 overlap, A5 has a 0.5 m edge; rules r1 and r2):

- for each parcel, the findings asked by its id are the whole check's findings naming it, in the same order, and the
  counts are theirs, the rules and the objects the whole check's;
- two parcels together give the union, each finding once;
- an empty list is refused with `invalid_input`, an id not in the drawing with `entity_not_found`; nothing changes.
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

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.topology.check.json").read_text("utf-8"))

UIDS = CASES["setup"]["uids"]


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(CASES["setup"])))


class ObjectFindings(unittest.TestCase):
    def test_each_parcel_s_findings_are_the_whole_check_s_naming_it(self) -> None:
        doc = drawing()
        whole = cad.topology.check(doc)
        self.assertEqual(whole.total, 3)
        for uid in UIDS:
            mine = cad.topology.check(doc, uids=[uid])
            want = [f.key for f in whole.findings if uid in f.uids]
            self.assertEqual([f.key for f in mine.findings], want, uid)
            self.assertEqual((mine.total, mine.open, mine.exceptions), (len(want), len(want), 0))
            self.assertEqual((mine.rules, mine.objects, mine.complete), (whole.rules, whole.objects, whole.complete))
            self.assertTrue(all(uid in f.uids for f in mine.findings))

    def test_two_parcels_give_the_union_once(self) -> None:
        doc = drawing()
        whole = cad.topology.check(doc)
        asked = [UIDS[0], UIDS[1], UIDS[4]]
        both = cad.topology.check(doc, uids=asked)
        want = [f.key for f in whole.findings if any(u in asked for u in f.uids)]
        self.assertEqual([f.key for f in both.findings], want)
        self.assertEqual(len(set(want)), len(want), "each finding once")
        self.assertEqual(both.total, 2, "A1–A2's overlap and A5's short edge")

    def test_refusals_change_nothing(self) -> None:
        doc = drawing()
        before = doc.to_bytes()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.topology.check(doc, uids=[])
        self.assertEqual((caught.exception.code, caught.exception.path), ("invalid_input", "uids"))
        missing = "0192f5a9-1414-7000-8000-000000000009"
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.topology.check(doc, uids=[UIDS[0], missing])
        self.assertEqual((caught.exception.code, caught.exception.path), ("entity_not_found", "uids[1]"))
        self.assertEqual(doc.to_bytes(), before)


if __name__ == "__main__":
    unittest.main()
