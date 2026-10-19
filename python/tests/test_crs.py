"""Points between coordinate systems with their heights from Python (docs/adr/0519): the project's heights and
epoch kept with the drawing, the operation's record, the refusals that stand where KentOS has no engine."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

# Bursa in ED50 / TM30 (PROJ, scripts/fixtures/crs_vertical_cases.py).
ED50 = cad.CrsPoint(x=420531.2140016982, y=4450407.827412584, z=812.4)


class Transform(unittest.TestCase):
    def test_an_ellipsoidal_height_rides_the_helmert_and_the_record_says_how(self) -> None:
        doc = cad.Document.new(srid=5254)
        ellipsoidal = cad.HeightReference(kind="ellipsoidal")
        out = cad.crs.transform(
            doc,
            points=[ED50],
            from_=cad.CrsSide(srid=2320, heights=ellipsoidal),
            to=cad.CrsSide(srid=5254, heights=ellipsoidal),
            uncertainty=cad.SourceUncertainty(horizontal=0.05),
        )
        op = out.operation
        self.assertEqual(op.id, "EPSG:1783 + EPSG:5260")
        self.assertEqual(op.accuracy, 2.1)
        self.assertEqual(op.heights, cad.HeightMethod.ELLIPSOIDAL)
        self.assertTrue(op.pipeline.startswith("+proj=pipeline +step +inv +proj=tmerc"))
        self.assertIn("+proj=helmert +x=-84.1", op.pipeline)
        self.assertEqual(op.area.name, "Türkiye (Turkey) - onshore and offshore.")
        point = out.points[0]
        assert point is not None
        self.assertAlmostEqual(point.z, 854.4819061812013, places=6)
        self.assertEqual(out.report.combined, 2.101)
        self.assertEqual(out.report.topology_tolerance, 0.001)

    def test_heights_between_ellipsoid_and_geoid_are_refused(self) -> None:
        doc = cad.Document.new(srid=5254)
        cad.settings.set(doc, heights=cad.HeightReference(kind="orthometric", vertical_srid=5775))
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.crs.transform(
                doc,
                points=[cad.CrsPoint(x=ED50.x, y=ED50.y, z=10.0)],
                from_=cad.CrsSide(srid=2320, heights=cad.HeightReference(kind="ellipsoidal")),
            )
        self.assertEqual(caught.exception.code, "geoid_required")

    def test_a_local_project_has_no_side_of_its_own(self) -> None:
        doc = cad.Document.new("Oda", workspace="cad", drawing_unit="mm")
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.crs.transform(doc, points=[cad.CrsPoint(x=0.0, y=0.0)])
        self.assertEqual(caught.exception.code, "no_system")


class Settings(unittest.TestCase):
    def test_the_heights_and_the_epoch_are_kept_in_the_file(self) -> None:
        doc = cad.Document.new(srid=5254)
        heights = cad.HeightReference(
            kind="orthometric", vertical_srid=5775, geoid=cad.GeoidModel(name="TG-20")
        )
        changed = cad.settings.set(doc, heights=heights, coordinate_epoch=2023.5)
        self.assertEqual(changed.changed, ["heights", "coordinateEpoch"])
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "kotlar.kcad"
            doc.save(path)
            back = cad.Document.open(path)
        self.assertEqual(back.settings.heights, heights)
        self.assertEqual(back.settings.coordinate_epoch, 2023.5)


if __name__ == "__main__":
    unittest.main()
