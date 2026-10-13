"""A new drawing's unit from Python (docs/adr/0245): the wizard's choices by
name, geometry in metres whatever the drawing unit, results that say their
units."""

from __future__ import annotations

import unittest
from pathlib import Path

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



class UnitChange(unittest.TestCase):
    """docs/adr/0256 §1: the sizes or the numbers kept, in one undo step."""

    def test_keeping_the_numbers_scales_the_drawing_and_undo_gives_both_back(self) -> None:
        doc = cad.Document.new("Oda", workspace="cad", drawing_unit="m")
        # A drawing in millimetres taken for metres: a 4000 × 3000 “m” room.
        made = cad.polygon.create(doc, layer_id=doc.active_layer, pts=[(0, 0), (4000, 0), (4000, 3000), (0, 3000)])
        plan = cad.units.change.plan(doc, from_="m", to="mm", geometry="scale")
        self.assertEqual((plan.factor, plan.objects), (0.001, 1))
        self.assertEqual(doc.measure(made.uid).area, 12_000_000.0, "a plan writes nothing")
        out = cad.units.change(doc, from_="m", to="mm", geometry="scale")
        self.assertEqual(out.to, cad.DrawingUnit.MM)
        m = doc.measure(made.uid)
        self.assertEqual((m.area, m.units.drawing), (12.0, cad.DrawingUnit.MM))
        self.assertEqual(doc.undo(), "Birimi değiştir")
        self.assertEqual(doc.units.drawing, cad.DrawingUnit.M)
        self.assertEqual(doc.measure(made.uid).area, 12_000_000.0)

    def test_keeping_the_sizes_changes_only_the_unit(self) -> None:
        doc = cad.Document.new("Oda", workspace="cad", drawing_unit="mm")
        made = cad.polygon.create(doc, layer_id=doc.active_layer, pts=ROOM)
        cad.units.change(doc, from_="mm", to="cm", geometry="keep")
        self.assertEqual((doc.units.drawing, doc.measure(made.uid).area), (cad.DrawingUnit.CM, 12.0))
        with self.assertRaises(cad.CommandFailed) as wrong:
            cad.units.change(doc, from_="mm", to="m", geometry="keep")
        self.assertEqual(wrong.exception.code, "unit_mismatch")


class MeasureBasis(unittest.TestCase):
    """docs/adr/0256 §4, §5: what a measure was taken in and how; a batch; the conventions."""

    def test_a_batch_sums_and_says_its_basis(self) -> None:
        doc = cad.Document.new("Oda", workspace="cad", drawing_unit="mm")
        room = cad.polygon.create(doc, layer_id=doc.active_layer, pts=ROOM)
        wall = cad.line.create(doc, layer_id=doc.active_layer, a=(0, 0), b=(0, 2.5))
        many = doc.measure_many([room.uid, wall.uid])
        self.assertEqual((many.count, many.area, many.length), (2, 12.0, 16.5))
        self.assertEqual((many.basis.kind, many.basis.frame, many.basis.method), ("local", "world", "plane"))
        self.assertIsNone(many.items[1].area)
        with self.assertRaises(cad.HostError) as no_system:
            doc.measure(room.uid, method="ellipsoid")
        self.assertEqual(no_system.exception.code, "method_unavailable")

    def test_the_conventions_by_the_project_type(self) -> None:
        c = cad.Document.new("Oda", workspace="cad", drawing_unit="mm").conventions
        self.assertEqual((c.axes, c.labels, c.polar_zero, c.polar_turn, c.angle_unit), (("east", "north"), ("X", "Y"), "east", "ccw", "deg"))
        g = cad.Document.new(srid=5256).conventions
        self.assertEqual((g.workspace, g.labels, g.polar_zero, g.polar_turn), ("gis", ("Y", "X"), "north", "cw"))

    def test_on_the_ellipsoid_in_a_projected_system(self) -> None:
        doc = cad.Document.new(srid=5256)
        x, y = 500_000.0, 4_400_000.0
        sq = cad.polygon.create(doc, layer_id=doc.active_layer, pts=[(x, y), (x + 100, y), (x + 100, y + 100), (x, y + 100)])
        m = doc.measure(sq.uid, method="ellipsoid")
        self.assertEqual((m.basis.kind, m.basis.method, m.units.area), ("projected", "ellipsoid", "m2"))
        self.assertAlmostEqual(m.area or 0, 10_000.0, delta=1.0)


if __name__ == "__main__":
    unittest.main()


class DxfUnit(unittest.TestCase):
    """A DXF's unit decided by one rule (docs/adr/0257 §1): what the file declares in a local project, metres with a
    coordinate system, the project's unit for a file that declares none; or the one chosen."""

    FILES = Path(__file__).resolve().parents[2] / "fixtures/formats/v1/dxf-units"

    def test_a_millimetre_room_into_a_metre_project_and_undo(self) -> None:
        doc = cad.Document.new("Oda", workspace="cad", drawing_unit="m")
        done = doc.import_dxf(self.FILES / "mm.dxf")
        self.assertEqual((done.unit.declared, done.unit.used, done.unit.source), (4, 4, cad.UnitSource.DECLARED))
        self.assertEqual((done.unit.factor, done.unit.assumed, done.objects, done.blocks), (0.001, False, 5, 1))
        self.assertEqual([layer.name for layer in done.layers], ["DUVAR"])
        wall = next(r.entity for r in doc.entities() if r.entity.kind == "line")
        self.assertEqual((wall.a.x, wall.b.x), (0.0, 4.0))
        self.assertEqual(doc.undo(), "DXF: mm.dxf")

    def test_a_file_declaring_no_unit_is_said_and_a_unit_can_be_chosen(self) -> None:
        doc = cad.Document.new("Oda", workspace="cad", drawing_unit="mm")
        plan = cad.file.import_.dxf.run(doc, cad.ImportDxf(path=str(self.FILES / "unitless.dxf")), op="plan")
        assert plan.output is not None
        self.assertEqual((plan.output.unit.source, plan.output.unit.assumed), (cad.UnitSource.PROJECT, True))
        self.assertEqual([w.code for w in plan.warnings], ["unit_assumed"])
        self.assertEqual(doc.info().can_undo, False)
        chosen = doc.import_dxf(self.FILES / "unitless.dxf", file_unit="cm", group="")
        self.assertEqual((chosen.unit.used, chosen.unit.factor, chosen.group), (5, 0.01, cad.UNSET))
