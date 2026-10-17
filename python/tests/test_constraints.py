"""Constraints and parameters from Python (docs/adr/0334–0336): ``kentos.cad.constraints.edit`` adds them by the parts
they name and solves the drawing, ``.query`` lists them with the diagnosis, ``kentos.cad.parameters.edit`` changes a
value and the drawing follows; ``Document.constraints()`` reads the table, a file keeps it (schema 73) and one undo
takes a step back.

The shared cases (fixtures/commands/v1/cad.constraints.*.json, cad.parameters.edit.json) run here too through
test_command_cases.py; this test is the reading side and the flow a script follows.
"""

from __future__ import annotations

import unittest

import kentos.cad as cad


def room() -> tuple[cad.Document, list[str]]:
    """A new drawing with a room of four lines, not quite square."""
    doc = cad.Document.new(name="Oda", workspace="cad")
    layer = doc.active_layer
    corners = [(0.0, 0.0), (6.0, 0.3), (6.2, 4.0), (0.0, 4.1)]
    uids = []
    for k in range(4):
        made = cad.line.create(
            doc,
            layer_id=layer,
            a=cad.Vec2(x=corners[k][0], y=corners[k][1]),
            b=cad.Vec2(x=corners[(k + 1) % 4][0], y=corners[(k + 1) % 4][1]),
        )
        uids.append(made.uid)
    return doc, uids


def part(uid: str, element: dict) -> dict:
    return {"uid": uid, "element": element}


def point(uid: str, place: str) -> dict:
    return part(uid, {"kind": "point", "place": {"kind": place}})


def segment(uid: str) -> dict:
    return part(uid, {"kind": "segment", "part": 0, "ring": 0, "index": 0})


def rectangle(uids: list[str]) -> list[dict]:
    rules: list[dict] = [{"rule": {"kind": "coincident", "a": point(uids[k], "end"), "b": point(uids[(k + 1) % 4], "start")}}
                         for k in range(4)]
    rules += [
        {"rule": {"kind": "horizontal", "a": segment(uids[0])}},
        {"rule": {"kind": "parallel", "a": segment(uids[0]), "b": segment(uids[2])}},
        {"rule": {"kind": "parallel", "a": segment(uids[1]), "b": segment(uids[3])}},
        {"rule": {"kind": "perpendicular", "a": segment(uids[0]), "b": segment(uids[1])}},
        {"rule": {"kind": "fix", "a": point(uids[0], "start")}},
        {"rule": {"kind": "distance", "a": point(uids[0], "start"), "b": point(uids[0], "end"), "parameter": "genislik"}, "value": "6"},
        {"rule": {"kind": "distance", "a": point(uids[1], "start"), "b": point(uids[1], "end"), "parameter": "yukseklik"}, "value": "4"},
    ]
    return rules


def ends(doc: cad.Document, uid: str) -> tuple[tuple[float, float], tuple[float, float]]:
    e = doc.entity(uid).entity.to_json()
    return (e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])


class ConstraintTests(unittest.TestCase):
    def test_a_room_made_a_rectangle_widens_by_its_parameter(self) -> None:
        doc, uids = room()
        made = cad.constraints.edit(doc, edit=cad.ConstraintEdit.from_json({"kind": "add", "constraints": rectangle(uids)}))
        self.assertEqual(len(made.added), 11)
        self.assertEqual(made.diagnosis.status, "fully")
        table = doc.constraints()
        self.assertEqual([p["name"] for p in table["parameters"]], ["genislik", "yukseklik"])
        listed = cad.constraints.query(doc)
        self.assertTrue(all(c.holds for c in listed.constraints))
        self.assertEqual([c.label for c in listed.constraints][:5], ["Çakışık"] * 4 + ["Yatay"])

        # The width's parameter set to 9: the rectangle widens and stays one.
        out = cad.parameters.edit(doc, edits=[cad.ParameterEdit.from_json({"kind": "set", "name": "genislik", "expression": "9"})])
        self.assertEqual(out.diagnosis.status, "fully")
        values = {v.name: v.value for v in out.values}
        self.assertAlmostEqual(values["genislik"], 9.0, places=9)
        (a, b), (c, d) = ends(doc, uids[0]), ends(doc, uids[1])
        self.assertAlmostEqual(b[0] - a[0], 9.0, places=9)
        self.assertAlmostEqual(b[1] - a[1], 0.0, places=9)
        self.assertAlmostEqual(c[0] - d[0], 0.0, places=9)
        self.assertAlmostEqual(d[1] - c[1], 4.0, places=9)

        # A file keeps them; one undo takes the width back.
        again = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(again.constraints(), doc.constraints())
        self.assertEqual(doc.undo(), "Parametreler")
        a, b = ends(doc, uids[0])
        self.assertAlmostEqual(b[0] - a[0], 6.0, places=9)

    def test_a_conflict_is_refused_and_writes_nothing(self) -> None:
        doc, uids = room()
        cad.constraints.edit(doc, edit=cad.ConstraintEdit.from_json({"kind": "add", "constraints": rectangle(uids)}))
        before = ends(doc, uids[2])
        extra = {"rule": {"kind": "distance", "a": point(uids[2], "start"), "b": point(uids[2], "end"), "parameter": ""}, "value": "7"}
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.constraints.edit(doc, edit=cad.ConstraintEdit.from_json({"kind": "add", "constraints": [extra]}))
        self.assertEqual(refused.exception.code, "constraint_conflict")
        self.assertEqual(ends(doc, uids[2]), before)

    def test_an_edit_keeps_the_constraints_or_relaxes_them(self) -> None:
        """docs/adr/0337 §1: the right edge moved, the rectangle follows; the top edge moved against the held corner
        and the height, nothing written; relaxed, written, the corners it breaks gone."""
        doc, uids = room()
        cad.constraints.edit(doc, edit=cad.ConstraintEdit.from_json({"kind": "add", "constraints": rectangle(uids)}))

        def move(dx: float, dy: float) -> cad.Transform:
            return cad.Transform.from_json({"kind": "move", "dx": dx, "dy": dy})


        # The right edge moved against the width 6: refused.
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.entities.transform(doc, uids=[uids[1]], transform=move(2, 0))
        self.assertEqual(refused.exception.code, "constraint_conflict")

        # The top edge moved up: the held corner and the height refuse it; relaxed, two corners go.
        top = ends(doc, uids[2])
        with self.assertRaises(cad.CommandFailed) as refused:
            cad.entities.transform(doc, uids=[uids[2]], transform=move(0, 1))
        self.assertEqual(refused.exception.code, "constraint_conflict")
        self.assertEqual(ends(doc, uids[2]), top)
        count = len(doc.constraints()["constraints"])
        with doc.relaxed(), self.assertWarns(cad.CommandWarning):
            cad.entities.transform(doc, uids=[uids[2]], transform=move(0, 1))
        self.assertEqual(len(doc.constraints()["constraints"]), count - 2)
        self.assertAlmostEqual(ends(doc, uids[2])[0][1] - top[0][1], 1.0, places=9)

        # Without the fix and the width, the right edge moved: the top and the bottom follow, the height stays 4.
        doc, uids = room()
        rules = [r for r in rectangle(uids) if r["rule"]["kind"] != "fix" and r["rule"].get("parameter") != "genislik"]
        cad.constraints.edit(doc, edit=cad.ConstraintEdit.from_json({"kind": "add", "constraints": rules}))
        right = ends(doc, uids[1])
        cad.entities.transform(doc, uids=[uids[1]], transform=move(2, 0))
        (a, b), (c, d) = ends(doc, uids[0]), ends(doc, uids[1])
        self.assertAlmostEqual(b[0], right[0][0] + 2, places=9)
        self.assertAlmostEqual(b[1] - a[1], 0.0, places=9)
        self.assertAlmostEqual(d[1] - c[1], 4.0, places=9)
        self.assertEqual(doc.undo(), "Taşı")


if __name__ == "__main__":
    unittest.main()
