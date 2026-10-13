"""A NURBS curve from Python (docs/adr/0259 §3): ``kentos.cad.nurbs.create``
builds the curve through the points given, in the shared core, so it is the
curve the desktop's tool writes for the same points; its fit points are kept
and the plan shows exactly what is written.

The shared cases (fixtures/commands/v1/cad.nurbs.create.json, run by
test_command_cases.py) hold the numbers bit for bit; this checks the typed
layer: the union of ``by``, the stored entity read back, undo's step name.
"""

from __future__ import annotations

import unittest

import kentos.cad as cad


def layer_of(doc: cad.Document) -> str:
    def leaves(nodes):
        for n in nodes:
            if n.type == "layer":
                yield n.id
            yield from leaves(n.children)

    return next(leaves(doc.layers()))


class NurbsTests(unittest.TestCase):
    def test_a_curve_through_points_is_written_as_planned(self) -> None:
        doc = cad.Document.new("Eğri", workspace="cad")
        layer = layer_of(doc)
        pts = [cad.Vec2(0.0, 0.0), cad.Vec2(12.0, 7.5), cad.Vec2(25.0, 3.0), cad.Vec2(38.0, 11.0)]
        by = cad.FitNurbsBy(pts=pts)
        plan = cad.nurbs.create.plan(doc, layer_id=layer, by=by)
        made = cad.nurbs.create(doc, layer_id=layer, by=by, expected_revision=plan.revision)
        stored = next(r.entity for r in doc.entities(page_size=100) if r.uid == made.uid).to_json()
        self.assertEqual(stored["kind"], "nurbs")
        self.assertEqual(stored["degree"], 3)
        self.assertEqual(len(stored["ctrl"]), 4)
        self.assertEqual(stored["fit"], [p.to_json() for p in pts])
        self.assertEqual(stored["ctrl"], plan.entity.to_json()["ctrl"])
        # Clamped: the curve starts and ends at its end points.
        self.assertEqual(stored["ctrl"][0], {"x": 0.0, "y": 0.0})
        self.assertEqual(stored["ctrl"][-1], {"x": 38.0, "y": 11.0})
        self.assertEqual(doc.undo(), "Ekle")

    def test_a_closed_curve_by_control_points_wraps_them(self) -> None:
        doc = cad.Document.new("Eğri", workspace="cad")
        square = [cad.Vec2(0.0, 0.0), cad.Vec2(6.0, 0.0), cad.Vec2(6.0, 6.0), cad.Vec2(0.0, 6.0)]
        made = cad.nurbs.create(doc, layer_id=layer_of(doc), by=cad.ControlNurbsBy(degree=3, ctrl=square), closed=True)
        stored = next(r.entity for r in doc.entities(page_size=100) if r.uid == made.uid).to_json()
        self.assertEqual(len(stored["ctrl"]), 7)
        self.assertEqual(stored["knots"], [float(i) for i in range(11)])
        self.assertTrue(stored["closed"] and stored["periodic"])

    def test_points_in_a_row_at_one_place_are_refused_by_their_code(self) -> None:
        doc = cad.Document.new("Eğri", workspace="cad")
        p = cad.Vec2(1.0, 1.0)
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.nurbs.create(doc, layer_id=layer_of(doc), by=cad.FitNurbsBy(pts=[cad.Vec2(0.0, 0.0), p, p]))
        self.assertEqual(caught.exception.code, "coincident_points")
        self.assertEqual(caught.exception.path, "by.pts[2]")


if __name__ == "__main__":
    unittest.main()
