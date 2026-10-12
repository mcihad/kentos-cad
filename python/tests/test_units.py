"""A new drawing's unit from Python (docs/adr/0245): the wizard's choices by
name, geometry in metres whatever the drawing unit, results that say their
units."""

from __future__ import annotations

import unittest

import kentos.cad as cad

ROOM = [(0, 0), (4, 0), (4, 3), (0, 3)]


class MillimetreProject(unittest.TestCase):
    def test_a_room_of_4000_by_3000_mm_measures_12_m2(self) -> None:
        doc = cad.Document.new("Oda", workspace="cad", drawing_unit="mm")
        info = doc.info()
        self.assertEqual(info.settings.srid, 0)
        self.assertEqual(info.settings.workspace, cad.Workspace.CAD)
        self.assertEqual(info.settings.drawing_unit, cad.DrawingUnit.MM)
        self.assertEqual(info.settings.plot_scale, 1.0)
        self.assertEqual(doc.units, cad.Units(length="m", area="m2", drawing=cad.DrawingUnit.MM))

        # 4000 × 3000 mm, written in metres as the wire takes them.
        width, depth = doc.units.to_metres(4000), doc.units.to_metres(3000)
        self.assertEqual((width, depth), (4.0, 3.0))
        made = cad.polygon.create(doc, layer_id=doc.active_layer, pts=ROOM)
        m = doc.measure(made.uid)
        self.assertEqual((m.area, m.length), (12.0, 14.0))
        self.assertEqual((m.units.area, m.units.length, m.units.drawing), ("m2", "m", cad.DrawingUnit.MM))
        self.assertEqual(m.units.from_metres(m.length or 0), 14000.0)

    def test_the_wizards_defaults_and_refusals(self) -> None:
        gis = cad.Document.new(srid=5256)
        self.assertEqual((gis.name, gis.settings.plot_scale, gis.settings.drawing_unit), ("Yeni proje", 1000.0, cad.UNSET))
        self.assertEqual(gis.units.drawing, cad.DrawingUnit.M)
        by_province = cad.Document.new(workspace="cad", coords="real", province=58)
        self.assertNotEqual(by_province.settings.srid, 0)
        with self.assertRaises(cad.InvalidInput) as unit:
            cad.Document.new(srid=5256, drawing_unit="mm")
        self.assertIn("drawingUnit", str(unit.exception))
        with self.assertRaises(cad.InvalidInput):
            cad.Document.new(workspace="gis", coords="local")
        with self.assertRaises(cad.InvalidInput):
            cad.Document.new("CRS'siz")
        with self.assertRaises(cad.InvalidInput) as unnamed:
            cad.Document.new(" ", srid=5256)
        self.assertIn("Proje adı boş olamaz.", str(unnamed.exception))
        with self.assertRaises(cad.UnknownSystem):
            cad.Document.new(srid=4)


if __name__ == "__main__":
    unittest.main()
