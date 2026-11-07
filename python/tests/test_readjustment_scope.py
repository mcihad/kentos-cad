"""`cad.readjustment.scope` (Düzenleme kapsamı, docs/adr/2060; plan/map-cad.md MAP-11) held to what must be true of any
answer, with Python's own `decimal`, no KentOS code. The drawing is scripts/fixtures/readjustment_scope_cases.py's:

- a partial parcel with a basis keeps registered = basis + what the part outside keeps, exactly (Article 31(2): the
  area error stays outside); a whole one's basis is its record as written, the comma a point;
- every parcel's parts inside and outside add up to its computed area (the geometric area is not lost or made);
- the bases' sum is the sum of the bases, exactly, and the counts are the parcels';
- no parcel needing review has a basis; a computed basis has exactly the policy's places;
- the same drawing with the rounding changed changes only the computed bases.
"""

from __future__ import annotations

import sys
import unittest
from decimal import Decimal
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import readjustment_scope_cases as rs  # noqa: E402

BOUNDARY = rs.uid(1)
REVIEW = {"needsCorrection", "noRecord", "invalidRecord", "finerThanPolicy", "outsideTooSmall", "sliver"}


def given(v):
    """A field the answer left out reads as cad.UNSET."""
    return None if v is cad.UNSET else v


def opened() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(rs.setup(rs.MAIN))))


class ScopeHolds(unittest.TestCase):
    def test_the_areas_are_kept(self) -> None:
        r = cad.readjustment.scope(opened(), boundary=BOUNDARY, scale=2, rounding="half_even")
        self.assertTrue(r.parcels)
        for p in r.parcels:
            self.assertAlmostEqual(p.inside + p.outside, p.calculated, delta=1e-6, msg=p.parcel)
            if p.status in REVIEW or p.status == "outside":
                self.assertIsNone(given(p.basis), p.parcel)
            if p.status == "registered":
                self.assertEqual(p.basis, p.recorded_text.replace(",", "."), p.parcel)
            if p.status == "computed":
                self.assertEqual(len(p.basis.split(".")[1]), 2, p.parcel)
                registered = Decimal(p.recorded_text.replace(",", "."))
                self.assertEqual(Decimal(p.basis) + Decimal(p.outside_recorded), registered, p.parcel)
        bases = [Decimal(p.basis) for p in r.parcels if given(p.basis) is not None]
        self.assertEqual(Decimal(r.basis), sum(bases, Decimal(0)))
        self.assertEqual(r.ready, len(bases))
        self.assertEqual(r.whole + r.partial + sum(p.kind == "outside" for p in r.parcels), len(r.parcels))
        self.assertAlmostEqual(r.uncovered, r.boundary_area - sum(p.inside for p in r.parcels), delta=1e-6)

    def test_the_rounding_changes_only_the_computed_bases(self) -> None:
        doc = opened()
        even = cad.readjustment.scope(doc, boundary=BOUNDARY, scale=2, rounding="half_even")
        away = cad.readjustment.scope(doc, boundary=BOUNDARY, scale=2, rounding="away_from_zero")
        for a, b in zip(even.parcels, away.parcels):
            self.assertEqual(a.status, b.status, a.parcel)
            if a.status == "registered":
                self.assertEqual(a.basis, b.basis, a.parcel)
        p3 = {p.parcel: p for p in away.parcels}["101/3"]
        self.assertEqual((p3.basis, p3.outside_recorded), ("333.34", "265.06"))


if __name__ == "__main__":
    unittest.main()
