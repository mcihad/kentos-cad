"""What KentOS measured in a drawing for an audit rule (``facts.project``, ``kentos.auditFacts`` 1; docs/adr/8057 §36):
the facts KentOS's Rust worked out on fixtures/audit-facts/v1/denetim.kcad (``kentos_processing::audit_facts``, whose
test keeps fixtures/audit-facts/v1/denetim.json byte for byte) read through the SDK's typed helpers, as a rule reads
them on the desktop and in a cloud job alike."""

from __future__ import annotations

import json
import os
import unittest

from kentos import ext
from kentos.ext_host import findings_of

FACTS = os.path.join(os.path.dirname(__file__), "..", "..", "fixtures", "audit-facts", "v1", "denetim.json")


def uid(n: int) -> str:
    return f"0192f5a1-2590-7000-8000-{n:012d}"


def measured() -> dict:
    with open(FACTS, encoding="utf-8") as f:
        return json.load(f)


class ProjectAuditFactsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.raw = measured()
        self.facts = ext.GisAuditFacts({"layers": [], "cut": False, "project": self.raw}).project

    def test_the_layers_and_their_counts_are_the_drawing_s(self) -> None:
        self.assertEqual(self.raw["format"], "kentos.auditFacts")
        self.assertEqual(self.facts.tolerance, 0.001)
        self.assertFalse(self.facts.cut)
        self.assertEqual([layer.id for layer in self.facts.layers()], ["parsel", "yapi", "nokta", "yol"])
        parsel = self.facts.layer("Parsel")
        self.assertIs(parsel, self.facts.layer("parsel"), "by its name or its id")
        self.assertEqual((parsel.points, parsel.lines, parsel.areas, parsel.other), (0, 0, 8, 1))
        self.assertEqual(parsel.area, 1140.0)
        self.assertEqual(parsel.totals, {"invalid": 0, "overlaps": 1, "gaps": 1})
        self.assertEqual(self.facts.layer("nokta").points, 2)
        self.assertEqual(self.facts.layer("Yol").length, 45.708, "a half circle's arc exact")

    def test_overlaps_gaps_and_invalid_shapes_are_typed(self) -> None:
        parsel = self.facts.layer("parsel")
        self.assertEqual(parsel.overlaps, [ext.Overlap(uid(2), uid(3), 50.0, (17.5, 5.0))])
        gap = parsel.gaps[0]
        self.assertIsInstance(gap, ext.Gap)
        self.assertEqual((gap.area, gap.at), (60.0, (15.0, 33.0)))
        self.assertEqual(gap.around[0], uid(6), "the neighbour sharing the longest boundary first")
        self.assertEqual(sorted(gap.around), [uid(n) for n in (4, 5, 6, 7, 8)])
        invalid = self.facts.layer("Yapı").invalid
        self.assertEqual([(x.uid, x.problem) for x in invalid],
                         [(uid(10), "ringCrossing"), (uid(11), "repeated"), (uid(12), "zeroArea")])
        self.assertEqual(invalid[0].at, (105.0, 5.0))
        with self.assertRaises(AttributeError):
            invalid[0].problem = "repeated"  # type: ignore[misc]  # facts are read, not changed

    def test_a_layer_without_objects_has_empty_facts(self) -> None:
        empty = self.facts.layer("bos")
        self.assertEqual((empty.areas, empty.area, empty.overlaps, empty.gaps, empty.invalid), (0, 0.0, [], [], []))
        self.assertEqual(empty.totals, {"invalid": 0, "overlaps": 0, "gaps": 0})

    def test_a_rule_answers_findings_from_the_facts_alone(self) -> None:
        def no_overlaps(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
            parcels = facts.project.layer("parsel")
            return [ext.AuditFinding(f"Parsel {o.area:g} m² çakışıyor", "error", o.a) for o in parcels.overlaps] + [
                ext.AuditFinding(f"{g.area:g} m² boşluk", "warning", g.around[0]) for g in parcels.gaps]

        answer = findings_of(no_overlaps(ext.GisAuditFacts({"project": self.raw})))
        self.assertEqual(answer["findings"], [
            {"severity": "error", "message": "Parsel 50 m² çakışıyor", "featureId": uid(2)},
            {"severity": "warning", "message": "60 m² boşluk", "featureId": uid(6)},
        ])

    def test_a_kentos_that_does_not_measure_says_the_ability_is_missing(self) -> None:
        facts = ext.GisAuditFacts({"layers": [], "cut": False})
        self.assertEqual(facts.layers(), [], "the layers are read as before")
        with self.assertRaises(ext.NotAvailable):
            facts.project


if __name__ == "__main__":
    unittest.main()
