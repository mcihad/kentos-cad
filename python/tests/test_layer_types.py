"""A layer's type and Otomatik seçim from Python (docs/adr/0622 §5).

A typed layer holds one kind of object; a write of another kind is refused
with ``layer_geometry_mismatch`` and nothing is written, as in the desktop.
Otomatik seçim is the project's setting, written with ``cad.settings.set``."""

from __future__ import annotations

import unittest

import kentos.cad as cad
from kentos.cad.errors import CommandFailed


def square(x: float, y: float) -> list[tuple[float, float]]:
    return [(x, y), (x + 10, y), (x + 10, y + 10), (x, y + 10)]


class LayerTypes(unittest.TestCase):
    def test_a_typed_layer_takes_only_its_kind(self) -> None:
        doc = cad.Document.new("Türler", srid=5256)
        added = cad.layers.edit(doc, operation="add", name="Parsel", geometry="area")
        parcels = added.layer
        node = next(n for n in doc.layers() if n.id == parcels)
        self.assertEqual(node.geometry, "area")
        cad.polygon.create(doc, layer_id=parcels, pts=square(500000, 4400000))
        revision = doc.revision
        with self.assertRaises(CommandFailed) as refused:
            cad.line.create(doc, layer_id=parcels, a=(500000, 4400000), b=(500010, 4400000))
        self.assertEqual(refused.exception.code, "layer_geometry_mismatch")
        self.assertIn("“Parsel” bir alan katmanı", refused.exception.message)
        self.assertEqual(doc.revision, revision, "nothing written")
        # A line layer cannot be made of a layer holding an area; mixed takes the type away.
        with self.assertRaises(CommandFailed) as line_layer:
            cad.layers.edit(doc, operation="geometry", layer=parcels, geometry="line")
        self.assertEqual(line_layer.exception.code, "layer_geometry_mismatch")
        cad.layers.edit(doc, operation="geometry", layer=parcels, geometry="mixed")
        node = next(n for n in doc.layers() if n.id == parcels)
        self.assertIs(node.geometry, cad.UNSET)
        cad.line.create(doc, layer_id=parcels, a=(500000, 4400000), b=(500010, 4400000))
        # The type is one undo step.
        self.assertIsNotNone(doc.undo(), "the line")
        self.assertEqual(doc.undo(), "Katman türü")
        node = next(n for n in doc.layers() if n.id == parcels)
        self.assertEqual(node.geometry, "area")

    def test_auto_select_is_the_projects_setting(self) -> None:
        doc = cad.Document.new("Seçim", srid=5256)
        # A project not asked its type selects as it did: any layer's objects.
        self.assertIs(doc.settings.auto_select, cad.UNSET)
        changed = cad.settings.set(doc, auto_select=False)
        self.assertEqual(changed.changed, ["autoSelect"])
        self.assertIs(doc.settings.auto_select, False)
        # Not an undo step.
        self.assertFalse(doc.info().can_undo)


if __name__ == "__main__":
    unittest.main()
