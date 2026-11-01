"""The user coordinate system from Python (docs/adr/0261): ``kentos.cad.ucs.set`` sets it as the desktop's UCS tools
do, the summary's conventions say it, and points are turned between it and the world explicitly; the wire stays the
world."""

from __future__ import annotations

import math
import unittest

import kentos.cad as cad


def near(a: tuple[float, float], b: tuple[float, float]) -> bool:
    return abs(a[0] - b[0]) < 1e-9 and abs(a[1] - b[1]) < 1e-9


class UcsTests(unittest.TestCase):
    def test_unit_correction_scales_frames_and_survives_save_undo_redo(self) -> None:
        doc = cad.Document.new("Birim ve UCS", workspace="cad", drawing_unit="mm")
        frame = cad.FrameUcsTo(origin=cad.Vec2(10.0, 20.0), angle=math.pi / 2)
        cad.ucs.set(doc, to=frame)
        cad.ucs.set(doc, to=cad.SaveUcsTo(name="Oda aksı"))
        a, b = doc.to_world([(0.0, 0.0), (4.0, 0.0)])
        line = cad.line.create(doc, layer_id=doc.active_layer, a=a, b=b)
        plan = cad.units.change.plan(doc, from_="mm", to="m", geometry="scale")
        self.assertEqual(plan.factor, 1000.0)
        self.assertEqual(doc.ucs.origin, (10.0, 20.0), "plan writes nothing")
        cad.units.change(doc, from_="mm", to="m", geometry="scale")
        self.assertEqual(doc.ucs.origin, (10000.0, 20000.0))
        self.assertEqual(doc.named_ucs[0].origin, (10000.0, 20000.0))
        self.assertEqual(doc.ucs.angle, math.pi / 2)
        self.assertAlmostEqual(doc.measure(line.uid).length or 0, 4000.0)
        restored = cad.Document.from_bytes(doc.to_bytes())
        self.assertEqual(restored.ucs, doc.ucs)
        self.assertEqual(restored.named_ucs, doc.named_ucs)
        self.assertAlmostEqual(restored.measure(line.uid).length or 0, 4000.0)
        self.assertEqual(doc.undo(), "Birimi değiştir")
        self.assertEqual(doc.ucs.origin, (10.0, 20.0))
        self.assertEqual(doc.named_ucs[0].origin, (10.0, 20.0))
        self.assertAlmostEqual(doc.measure(line.uid).length or 0, 4.0)
        self.assertEqual(doc.redo(), "Birimi değiştir")
        self.assertEqual(doc.ucs.origin, (10000.0, 20000.0))

    def test_a_ucs_is_set_named_and_its_points_turned(self) -> None:
        doc = cad.Document.new("Bina", workspace="cad", drawing_unit="m")
        self.assertTrue(doc.ucs.is_world)
        self.assertIsNone(doc.conventions.ucs)
        # The wall from (100, 200) going north is the UCS's x axis.
        set_ = cad.ucs.set(doc, to=cad.TwoPointsUcsTo(origin=cad.Vec2(100.0, 200.0), x_point=cad.Vec2(100.0, 210.0)))
        self.assertTrue(set_.changed)
        self.assertEqual(set_.ucs.angle, math.pi / 2)
        self.assertEqual(doc.ucs.origin, (100.0, 200.0))
        # A settings change: no undo step.
        self.assertFalse(doc.info().can_undo)

        saved = cad.ucs.set(doc, to=cad.SaveUcsTo(name="Duvar"))
        self.assertEqual(saved.named, ["Duvar"])
        self.assertEqual(doc.ucs.name, "Duvar")
        self.assertEqual([n.name for n in doc.named_ucs], ["Duvar"])

        # 10 along the UCS's x is 10 north in the world; and back.
        (w,) = doc.to_world([(10.0, 0.0)])
        self.assertTrue(near(w, (100.0, 210.0)), w)
        (u,) = doc.to_ucs([w])
        self.assertTrue(near(u, (10.0, 0.0)), u)

        # The world again; the named one stays and still turns points.
        cad.ucs.set(doc, to=cad.WorldUcsTo())
        self.assertTrue(doc.ucs.is_world)
        (w2,) = doc.to_world([(10.0, 0.0)], ucs="Duvar")
        self.assertTrue(near(w2, (100.0, 210.0)), w2)

        # Commands take world points whatever the UCS: the line lies where its points say.
        cad.ucs.set(doc, to=cad.NamedUcsTo(name="Duvar"))
        a, b = doc.to_world([(0.0, 0.0), (5.0, 0.0)])
        made = cad.line.create(doc, layer_id=doc.active_layer, a=cad.Vec2(*a), b=cad.Vec2(*b))
        m = doc.measure(made.uid)
        self.assertAlmostEqual(m.length or 0.0, 5.0, places=9)

    def test_refusals_say_their_codes(self) -> None:
        doc = cad.Document.new("Bina", workspace="cad", drawing_unit="m")
        with self.assertRaises(cad.CommandFailed) as unknown:
            cad.ucs.set(doc, to=cad.NamedUcsTo(name="Yok"))
        self.assertEqual(unknown.exception.code, "unknown_ucs")
        with self.assertRaises(cad.CommandFailed) as same:
            cad.ucs.set(doc, to=cad.TwoPointsUcsTo(origin=cad.Vec2(1.0, 1.0), x_point=cad.Vec2(1.0, 1.0)))
        self.assertEqual(same.exception.code, "same_points")
        with self.assertRaises(cad.UnknownObject):
            doc.to_world([(0.0, 0.0)], ucs="Yok")


if __name__ == "__main__":
    unittest.main()
