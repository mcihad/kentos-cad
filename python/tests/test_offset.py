"""Ötele by its meaning from Python (docs/adr/0246): ``kentos.cad.entities.offset``
on the interaction traces' drawing (fixtures/interaction/v1/edits.kcad) gives
the copy the desktop's Ötele tool writes for the same request, bit for bit.

The desktop's side of the equivalence is in crates/native/interaction
(tests/all/edits.rs, ``the_offset_tool_writes_what_the_semantic_command_writes``),
which asserts the tool's copy of the line 2 moved 0.2 m west is the line
x = 486979.8 from y = 4420006 to 4420018; MCP's is in crates/native/mcp
(tests/mcp.rs). The three ask the same thing and must get these numbers.
"""

from __future__ import annotations

import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
EDITS = ROOT / "fixtures/interaction/v1/edits.kcad"


def drawing() -> cad.Document:
    return cad.Document.from_bytes(EDITS.read_bytes())


def uid_of(doc: cad.Document, slot: int) -> str:
    return next(r.uid for r in doc.entities(page_size=1000) if r.entity.id == slot)


def by_uid(doc: cad.Document, uid: str) -> cad.Entity:
    return next(r.entity for r in doc.entities(page_size=1000) if r.uid == uid)


class OffsetTests(unittest.TestCase):
    def test_the_line_moved_0_2_m_west_is_the_tools_copy(self) -> None:
        doc = drawing()
        line = uid_of(doc, 2)
        by = cad.DistanceOffsetBy(distance=0.2, side=cad.Vec2(486977.0, 4420010.0))
        plan = cad.entities.offset.plan(doc, uids=[line], by=by)
        made = cad.entities.offset(doc, uids=[line], by=by, expected_revision=plan.revision)
        self.assertEqual(made.distances, [0.2])
        self.assertEqual(len(made.created), 1)
        copy = by_uid(doc, made.created[0]).to_json()
        self.assertEqual(copy["kind"], "line")
        self.assertEqual((copy["a"]["x"], copy["a"]["y"]), (486979.8, 4420006.0))
        self.assertEqual((copy["b"]["x"], copy["b"]["y"]), (486979.8, 4420018.0))
        # The plan showed exactly what was written.
        planned = plan.entities[0].to_json()
        self.assertEqual((planned["a"], planned["b"]), (copy["a"], copy["b"]))
        self.assertEqual(doc.undo(), "Ötele")

    def test_both_sides_and_delete_source_through_a_point(self) -> None:
        doc = drawing()
        line = uid_of(doc, 2)
        made = cad.entities.offset(
            doc,
            uids=[line],
            by=cad.ThroughOffsetBy(point=cad.Vec2(486980.5, 4420010.0)),
            both_sides=True,
            delete_source=True,
        )
        self.assertEqual(made.distances, [0.5])
        self.assertEqual(made.removed, [line])
        xs = sorted(by_uid(doc, u).to_json()["a"]["x"] for u in made.created)
        self.assertEqual(xs, [486979.5, 486980.5])

    def test_an_unknown_id_is_refused_by_its_code(self) -> None:
        doc = drawing()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.entities.offset(doc, uids=["01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"],
                                by=cad.DistanceOffsetBy(distance=0.2, side=cad.Vec2(0.0, 0.0)))
        self.assertEqual(caught.exception.code, "entity_not_found")


if __name__ == "__main__":
    unittest.main()
