"""The project's named views from Python (docs/adr/0285): ``kentos.cad.views.edit`` keeps, renames and removes them as
the desktop's Görünümler window does; ``Document.named_views`` reads them; they are a setting, no undo step, and they
travel in the project file."""

from __future__ import annotations

import math
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad


class ViewsTests(unittest.TestCase):
    def test_views_are_kept_renamed_removed_and_saved_in_the_file(self) -> None:
        doc = cad.Document.new("Ada", workspace="cad", drawing_unit="m")
        self.assertEqual(doc.named_views, [])
        kept = cad.views.edit(
            doc,
            edit=cad.SaveViewsOp(name="  Ada 112 ", center=cad.Vec2(120.0, 80.0), width=60.0, height=40.0, twist=-math.pi / 2),
        )
        self.assertTrue(kept.changed)
        self.assertEqual(kept.view.name if kept.view else None, "Ada 112")
        self.assertEqual(kept.view.twist if kept.view else None, 1.5 * math.pi)
        # A settings change: no undo step.
        self.assertFalse(doc.info().can_undo)
        cad.views.edit(doc, edit=cad.SaveViewsOp(name="Bütün", center=cad.Vec2(0.0, 0.0), width=500.0, height=300.0))
        cad.views.edit(doc, edit=cad.RenameViewsOp(name="Ada 112", to="Ada 112 kuzey"))
        self.assertEqual([v.name for v in doc.named_views], ["Ada 112 kuzey", "Bütün"])
        self.assertEqual(doc.named_views[1].twist, 0.0)

        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / "ada.kcad"
            doc.save(path)
            again = cad.Document.open(path)
            self.assertEqual(again.named_views, doc.named_views)

        cad.views.edit(doc, edit=cad.DeleteViewsOp(name="Bütün"))
        self.assertEqual([v.name for v in doc.named_views], ["Ada 112 kuzey"])

    def test_refusals_say_their_codes(self) -> None:
        doc = cad.Document.new("Ada", workspace="cad", drawing_unit="m")
        with self.assertRaises(cad.CommandFailed) as unknown:
            cad.views.edit(doc, edit=cad.DeleteViewsOp(name="Yok"))
        self.assertEqual(unknown.exception.code, "unknown_view")
        with self.assertRaises(cad.CommandFailed) as size:
            cad.views.edit(doc, edit=cad.SaveViewsOp(name="Düz", center=cad.Vec2(0.0, 0.0), width=0.0, height=10.0))
        self.assertEqual(size.exception.code, "invalid_view_size")


if __name__ == "__main__":
    unittest.main()
