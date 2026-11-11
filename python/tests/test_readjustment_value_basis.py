"""Değer esaslı dağıtım (`cad.readjustment.distribute`'s `basis: value`, docs/adr/5080; plan/map-cad.md MAP-11), held
by the test's own arithmetic in fractions (not the reference's, not KentOS): the distribution mass (registered − the
deduction rounded once) goes to the parcels in proportion to their values before the readjustment (Baugesetzbuch
§ 56(1), § 57), each share rounded to the places with the remainder to the one furthest short; every parcel's
deduction is its area less its share; the totals are kept exactly; every answer says it is no rule of the regulation;
without the values' source nothing is computed.
"""

from __future__ import annotations

import sys
import unittest
import warnings
from decimal import Decimal
from fractions import Fraction
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import readjustment_scope_cases as rs  # noqa: E402

RULES = {"id": "deneme", "release": 1, "jurisdiction": "Sentetik", "source": "Sentetik deneme", "effective": "2026-01-01",
         "approved": False, "deduction": "0.35", "scale": 2, "rounding": "half_even", "remainder": "largest_remainder",
         "basis": "value", "valueSource": "Sentetik deneme değerleri"}
# Three parcels of synthetic areas and values (TL); one holder each.
PARCELS = [("A", "812.40", "1250000"), ("B", "1200", "975000.50"), ("C", "433.33", "2100000")]


def entries():
    return [{"parcel": p, "holder": f"H{k}", "area": a, "share": "1/1", "value": v} for k, (p, a, v) in enumerate(PARCELS)]


def cents(f: Fraction) -> Fraction:
    """`f` rounded to two places, half to even, exactly (integer division, no float)."""
    q, r = divmod(f.numerator * 100, f.denominator)
    if 2 * r > f.denominator or (2 * r == f.denominator and q % 2):
        q += 1
    return Fraction(q, 100)


class ValueBasis(unittest.TestCase):
    def run_(self, rules):
        doc = cad.Document.from_bytes(kr.container(kr.document(rs.setup(rs.MAIN))))
        command = cad.command("cad.readjustment.distribute")
        return command.run(doc, command.input_type.from_json({"rules": rules, "entries": entries()}), op="plan")

    def test_the_mass_goes_by_the_values(self) -> None:
        with warnings.catch_warnings(record=True) as said:
            warnings.simplefilter("always")
            doc = cad.Document.from_bytes(kr.container(kr.document(rs.setup(rs.MAIN))))
            out = cad.readjustment.distribute(doc, rules=cad.ReadjustmentRules.from_json(RULES),
                                              entries=[cad.ReadjustmentEntry.from_json(e) for e in entries()])
        self.assertTrue([w for w in said if "kural değildir" in str(w.message)], [str(w.message) for w in said])
        areas = [Fraction(Decimal(a)) for _, a, _ in PARCELS]
        values = [Fraction(Decimal(v)) for _, _, v in PARCELS]
        deduction = cents(sum(areas) * Fraction(35, 100))
        mass = sum(areas) - deduction
        exact = [mass * v / sum(values) for v in values]
        shares = [cents(e) for e in exact]
        left = mass - sum(shares)
        # The remainder, a cent at a time, to the shares furthest short (first among equals).
        for k in sorted(range(3), key=lambda i: (-(exact[i] - shares[i]) * (1 if left > 0 else -1), i))[:abs(int(left * 100))]:
            shares[k] += Fraction(1, 100) * (1 if left > 0 else -1)
        self.assertEqual(out.basis, "value")
        self.assertEqual(Fraction(Decimal(out.deduction)), deduction)
        for p, area, share, v in zip(out.parcels, areas, shares, values):
            self.assertEqual(Fraction(Decimal(p.net)), share, p.parcel)
            self.assertEqual(Fraction(Decimal(p.deduction)), area - share, p.parcel)
            n, d = p.value_share.split("/")
            self.assertEqual(Fraction(int(n), int(d)), v / sum(values))
        self.assertTrue(out.conserved)
        self.assertEqual(sum(Fraction(Decimal(p.net)) for p in out.parcels), mass)

    def test_without_the_values_source_nothing_is_computed(self) -> None:
        answer = self.run_(dict(RULES, valueSource=""))
        self.assertEqual((answer.error.code, answer.error.path), ("invalid_input", "rules.valueSource"))

    def test_values_by_area_are_refused(self) -> None:
        rules = {k: v for k, v in RULES.items() if k not in ("basis", "valueSource")}
        answer = self.run_(rules)
        self.assertEqual((answer.error.code, answer.error.path), ("invalid_input", "entries[0].value"))


if __name__ == "__main__":
    unittest.main()
