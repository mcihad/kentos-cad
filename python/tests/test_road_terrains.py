"""KentOS Yol's terrain surfaces from Python (plan/road.md Y2.3): the road project's sample drawing, a surface made
of its points and a breakline, its heights and section, out of date when its definition changes, accepted again."""

import dataclasses
import unittest
from pathlib import Path

from kentos import cad, road
from kentos.cad._runtime import UNSET

SAMPLE = Path(__file__).resolve().parents[2] / "fixtures/kcad/v2/road-work.kcad"
BREAKLINE = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e16"
Z3 = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3e14"


class RoadTerrains(unittest.TestCase):
    def test_a_surface_is_made_goes_out_of_date_and_is_accepted_again(self) -> None:
        doc = cad.Document.open(SAMPLE)
        self.assertEqual([s.state for s in road.surfaces(doc)], ["new"])
        made = road.create(doc, "Zemin 2", point_layers=["zemin"], breakline_uids=[BREAKLINE], max_edge=60.0)
        self.assertEqual((made.surface.id, made.surface.state), ("t-1", "current"))
        self.assertEqual(made.surface.figures.triangles, 6)

        z = road.heights(doc, "zemin 2", [(20.0, 15.0), (60.0, 0.0)])
        self.assertEqual(z[0].z, 101.75)
        self.assertEqual(z[1].place, "outside")
        line = road.section(doc, "t-1", [(0.0, 15.0), (40.0, 15.0)])
        self.assertEqual((line[0].station, line[-1].station), (0.0, 40.0))
        self.assertEqual(line[-1].z, 102.375)  # Z2 and Z3 halfway
        # Inside one triangle the plane's slope and the way it falls; on the breakline (an edge) none.
        inside, edge = road.heights(doc, "t-1", [(12.0, 9.0), (20.0, 15.0)])
        self.assertIsInstance(inside.slope, float)
        self.assertIs(edge.slope, UNSET)

        # A changed definition keeps the digest it was accepted from: out of date until accepted.
        defined = next(d for d in made.surfaces if d.id == "t-1")
        changed = cad.road.terrains.edit(doc, operation="set", surface=dataclasses.replace(defined, max_edge=35.0, built=UNSET))
        self.assertEqual(changed.surface.state, "stale")
        self.assertEqual(road.accept(doc, "t-1").surface.state, "current")
        lines = road.contours(doc, "t-1", 0.5, "zemin", major_every=2)
        self.assertTrue(lines.lines and all(len(l.points) >= 2 for l in lines.lines))
        self.assertEqual(len(lines.uids), len(lines.lines))
        with self.assertRaises(cad.CommandError) as refused:
            road.remove(doc, "yok")
        self.assertEqual(refused.exception.code, "unknown_terrain")


if __name__ == "__main__":
    unittest.main()
