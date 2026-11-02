"""A road project from Python (plan/road.md Y1.3): KentOS Yol's work, on the province's system, its heights
TUDKA-99's orthometric ones with the TG-20 geoid; a road without a system is refused in the wizard's words."""

import tempfile
import unittest
from pathlib import Path

from kentos import cad


class RoadProject(unittest.TestCase):
    def test_a_road_starts_on_turkiyes_heights_and_keeps_them_saved(self) -> None:
        doc = cad.Document.new("D-200", workspace="cad", cad_work="road", province=58)
        s = doc.settings
        self.assertEqual((s.cad_work, s.srid, s.plot_scale), ("road", 5256, 1000.0))
        self.assertEqual(s.heights.kind, "orthometric")
        self.assertEqual(s.heights.vertical_srid, 5775)
        self.assertEqual(s.heights.geoid.name, "TG-20")
        with tempfile.TemporaryDirectory() as dir:
            path = Path(dir) / "yol.kcad"
            doc.save(path)
            again = cad.Document.open(path)
            self.assertEqual(again.settings.cad_work, "road")
            self.assertEqual(again.settings.heights.vertical_srid, 5775)

    def test_a_road_needs_a_system(self) -> None:
        with self.assertRaises(cad.InvalidInput) as refused:
            cad.Document.new("Yol", workspace="cad", cad_work="road", coords="local")
        self.assertIn("koordinat sistemlidir", str(refused.exception))


if __name__ == "__main__":
    unittest.main()
