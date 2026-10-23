"""A drawing's selection from Python (docs/adr/0622 §7): ``doc.select`` and
``doc.selection``. In the desktop's console they are the window's selection
(the console writes selections made by hand as ``doc.select([...])``); on a
drawing without a window, the drawing's own."""

from __future__ import annotations

import unittest

import kentos.cad as cad
from kentos.cad.errors import KentosError, UnknownObject


class Selection(unittest.TestCase):
    def test_a_drawing_without_a_window_keeps_its_own_selection(self) -> None:
        doc = cad.Document.new("Seçim", srid=5256)
        layer = doc.active_layer
        a = cad.point.create(doc, layer_id=layer, p=(500000, 4400000)).uid
        b = cad.line.create(doc, layer_id=layer, a=(500000, 4400000), b=(500010, 4400000)).uid
        self.assertEqual(doc.selection(), [])
        self.assertEqual(doc.select([a, b]), 2)
        self.assertEqual(doc.selection(), [a, b])
        self.assertEqual(doc.select([a, a, b]), 2)
        self.assertEqual(doc.selection(), [a, b])
        for bad in ([a, 17], [a, None]):
            with self.assertRaises(KentosError) as error:
                doc.select(bad)
            self.assertEqual(error.exception.code, "invalid_input")
            self.assertEqual(doc.selection(), [a, b])
        self.assertEqual(cad.Document.new("Başka", srid=5256).selection(), [])
        # The echo's code of a selection runs as written.
        code = f'doc.select(["{b}"])'
        exec(code, {"doc": doc})
        self.assertEqual(doc.selection(), [b])
        # An unknown id changes nothing.
        with self.assertRaises(UnknownObject):
            doc.select([a, "00000000-0000-7000-8000-000000000000"])
        self.assertEqual(doc.selection(), [b])
        # A deleted object leaves the selection.
        cad.entities.delete(doc, uids=[b])
        self.assertEqual(doc.selection(), [])
        self.assertEqual(doc.select([]), 0)


if __name__ == "__main__":
    unittest.main()
