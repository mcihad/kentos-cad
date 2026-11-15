"""What KentOS measured in a drawing's architecture for an audit rule (``facts.project.architecture``; docs/adr/8057
§39): the facts KentOS's Rust worked out on fixtures/audit-facts/v1/mimari.kcad (a two-storey building drawn with
KentOS's commands by scripts/fixtures/audit_facts_mimari.py; ``kentos_processing::audit_facts`` keeps mimari.json byte
for byte) read through the SDK's typed helpers."""

from __future__ import annotations

import json
import os
import unittest

from kentos import ext
from kentos.ext_host import findings_of

HERE = os.path.join(os.path.dirname(__file__), "..", "..", "fixtures", "audit-facts", "v1")


def measured(name: str) -> dict:
    with open(os.path.join(HERE, name), encoding="utf-8") as f:
        return json.load(f)


class ArchitectureFactsTest(unittest.TestCase):
    def setUp(self) -> None:
        facts = ext.GisAuditFacts({"layers": [], "cut": False, "project": measured("mimari.json")})
        self.a = facts.project.architecture

    def named(self, level: str, name: str) -> ext.RoomFacts:
        return next(r for r in self.a.rooms if r.level == level and r.name == name)

    def test_the_storeys_and_their_exits(self) -> None:
        levels = [(s.id, s.elevation, s.height) for s in self.a.levels]
        self.assertEqual(levels, [("zemin", 0.0, 3.0), ("k1", 3.0, 3.0)])
        self.assertEqual((self.a.exits("zemin"), self.a.exits("k1"), self.a.exits("çatı")), (2, 1, 0))

    def test_rooms_carry_their_areas_and_walks(self) -> None:
        self.assertEqual(len(self.a.rooms), 15)
        archive = self.named("k1", "Arşiv")
        self.assertIs(self.a.room(archive.uid), archive)
        self.assertEqual((archive.number, archive.unit, archive.net), ("K04", "of", 79.73))
        walk = archive.escape.exit
        self.assertIsInstance(walk, ext.EscapeWalk)
        self.assertEqual((walk.length, walk.one_way, walk.exits), (22.52, 22.52, 1))
        self.assertEqual(walk.path[0], (0.5, 8.5), "its farthest point, 40 cm in front of its walls")
        living = self.named("zemin", "Salon").escape
        self.assertTrue(living.has_entrance)
        self.assertEqual((living.entrance.length, living.entrance.one_way), (4.21, None), "to the flat's door")
        self.assertTrue(self.named("k1", "Merdiven evi").stair)
        for level, name in (("zemin", "Merdiven evi"), ("zemin", "Asansör boşluğu"), ("k1", "Balkon")):
            self.assertIsNone(self.named(level, name).escape, name)
        self.assertTrue(self.named("zemin", "Asansör boşluğu").void and self.named("k1", "Balkon").outdoor)
        with self.assertRaises(AttributeError):
            archive.net = 1.0  # type: ignore[misc]  # facts are read, not changed

    def test_groups_carry_article_4_s_areas(self) -> None:
        self.assertEqual([u.id for u in self.a.units], ["d1", "d2", "dk", "of", "dp", "ortak"])
        flat, storage = self.a.unit("d1"), self.a.unit("1-E")
        self.assertEqual((storage.kind, storage.annex_of), ("annex", "d1"))
        self.assertEqual(flat.annex_gross, storage.gross)
        self.assertAlmostEqual(flat.total, flat.gross + flat.annex_gross, places=3)
        self.assertAlmostEqual(flat.general, flat.total + flat.share, places=3)
        self.assertTrue(flat.dwelling and not self.a.unit("of").dwelling)
        self.assertIsNone(self.a.unit("ortak").share)
        self.assertEqual([(lv.level, lv.net) for lv in self.a.unit("of").levels], [("zemin", 53.0975), ("k1", 79.73)])
        self.assertIsNone(self.a.unit("yok"))

    def test_a_dwelling_unit_s_door_walks_to_the_nearest_exit(self) -> None:
        doors = {d.unit: d for d in self.a.doors}
        self.assertEqual(sorted(doors), ["d1", "d2"])
        self.assertEqual((doors["d2"].at, doors["d2"].exit.one_way), ((8.0, 4.0), 6.3))
        self.assertFalse(doors["d2"].into_stair, "it opens into the corridor, not the stair's room")

    def test_rooms_carry_their_kind_and_narrow_side(self) -> None:
        """docs/adr/8057 §53: the kind Mimari denetim reads from the name or use, the net width (dar kenar) as it
        measures it, and the project's sprinkler setting (none on this drawing)."""
        living, corridor, archive = self.named("zemin", "Salon"), self.named("k1", "Koridor"), self.named("k1", "Arşiv")
        self.assertEqual((living.kind, living.narrow), ("living", 3.85))
        self.assertEqual((corridor.kind, corridor.narrow), ("corridor", 1.4))
        self.assertEqual((self.named("k1", "Yatak odası 2").kind, archive.kind), ("bedroom", None))
        self.assertEqual(archive.narrow, 3.35)
        self.assertFalse(self.a.sprinklers)
        older = ext.ArchitectureFacts({"rooms": [{"uid": "x", "net": 9.0}], "doors": [{"uid": "d", "unit": "u"}]})
        self.assertEqual((older.rooms[0].kind, older.rooms[0].narrow, older.doors[0].into_stair), (None, None, False))
        self.assertFalse(older.sprinklers, "facts of an older KentOS read with the fields' defaults")

    def test_a_rule_finds_what_the_fire_audit_finds(self) -> None:
        """A BYKHY 32/4 and 4/i rule written on the facts alone: an office's or a corridor's one-way walk over 15 m.
        Mimari denetim's own fire rules say the same two on this drawing (Arşiv 22.52 m, the first floor's corridor
        21.82 m)."""
        def one_way(facts: ext.GisAuditFacts) -> list[ext.AuditFinding]:
            a = facts.project.architecture
            out = []
            for r in a.rooms:
                unit = a.unit(r.unit) if r.unit else None
                if r.escape and r.escape.exit and not (unit and unit.dwelling) and r.escape.exit.one_way > 15:
                    out.append(ext.AuditFinding(f"{r.name}: tek yön {r.escape.exit.one_way:.2f} m > 15.00 m", "error",
                                                r.uid))
            return out

        answer = findings_of(one_way(ext.GisAuditFacts({"project": measured("mimari.json")})))
        self.assertEqual([f["message"] for f in answer["findings"]],
                         ["Koridor: tek yön 21.82 m > 15.00 m", "Arşiv: tek yön 22.52 m > 15.00 m"])

    def test_a_drawing_without_rooms_has_empty_architecture(self) -> None:
        a = ext.ProjectAuditFacts(measured("denetim.json")).architecture
        self.assertEqual((a.levels, a.rooms, a.units, a.doors, a.exits(None)), ([], [], [], [], 0))


if __name__ == "__main__":
    unittest.main()
