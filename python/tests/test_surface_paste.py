"""Yüzey yapıştır (docs/adr/3920; plan/map-cad.md MAP-14) through the SDK, held by this test's own reading of the rule.

On the independent writer's paste drawing (fixtures/kcad/v2/surface-pastes.json: Mevcut zemin, its points, breakline,
boundary and four edits; Son zemin pasting it):

- Son zemin with the paste alone is Mevcut zemin's triangles: the same heights inside them, the same area, and one
  triangle more where Mevcut zemin's taken-out triangle was (its edges came in; nothing of Son zemin is under it, so
  the paste's corners make it);
- Mevcut zemin pasting Son zemin back (``road.paste``) is said (``cycle``) and pastes nothing;
- a surface pasting itself is refused (``invalid_terrain`` at ``surface.edits[0]``).
"""

from __future__ import annotations

import dataclasses
import unittest
import warnings
from pathlib import Path

from kentos import cad, road

DRAWING = Path(__file__).resolve().parents[2] / "fixtures/kcad/v2/surface-pastes.kcad"


class SurfacePaste(unittest.TestCase):
    def setUp(self) -> None:
        # What a surface could not use comes as the command's warnings; this test reads them from the report.
        warnings.simplefilter("ignore", cad.CommandWarning)

    def test_the_pasted_surface_is_the_ground_where_it_lies(self) -> None:
        doc = cad.Document.open(DRAWING)
        son = next(d for d in doc.settings.road.surfaces if d.id == "t-son")
        only = cad.road.terrains.edit(
            doc, operation="set", surface=dataclasses.replace(son, edits=[cad.PasteSurfaceEdit(terrain="t-zemin")])
        )
        self.assertEqual(list(only.surface.left or []), [])
        reports = {r.id: r for r in road.surfaces(doc)}
        zemin, pasted = reports["t-zemin"].figures, reports["t-son"].figures
        self.assertEqual(pasted.triangles, zemin.triangles + 1)
        # The boundary's 40 m × 30 m; Mevcut zemin lacks the triangle taken out (15 m × 30 m halved).
        self.assertEqual((zemin.area, pasted.area), (975.0, 1200.0))
        inside = [(10.0, 10.0), (5.0, 25.0), (30.0, 5.0), (12.0, 20.0)]
        for p, x, y in zip(inside, road.heights(doc, "t-zemin", inside), road.heights(doc, "t-son", inside)):
            self.assertEqual((x.place, y.place), ("inside", "inside"), p)
            self.assertAlmostEqual(x.z, y.z, places=9, msg=p)
        # Where Mevcut zemin's triangle was taken out it has no height; Son zemin has, from the pasted corners.
        out = [(20.0, 20.0), (35.0, 27.0)]
        self.assertEqual([h.place for h in road.heights(doc, "t-zemin", out)], ["outside", "outside"])
        self.assertEqual([h.place for h in road.heights(doc, "t-son", out)], ["inside", "inside"])

    def test_a_paste_back_is_said_and_one_of_itself_refused(self) -> None:
        doc = cad.Document.open(DRAWING)
        back = road.paste(doc, "t-zemin", "t-son")
        # Its own edge across the breakline was said before; the paste back is said now.
        said = [(l.id, l.why) for l in back.surface.left or [] if l.role == "edits"]
        self.assertEqual(sorted(said), [("edits[2]", "crosses"), ("edits[4]", "cycle")])
        self.assertEqual(back.surface.figures.triangles, 7, "pastes nothing")
        with self.assertRaises(cad.CommandError) as refused:
            road.paste(doc, "t-son", "t-son")
        self.assertEqual(refused.exception.code, "invalid_terrain")
        self.assertEqual(refused.exception.path, "surface.edits[2]")


if __name__ == "__main__":
    unittest.main()
