"""The atlas as one PDF from Python (plan/gis.md CBS-21, docs/adr/0534).

The shared drawing (fixtures/atlas/v1/cases.json, written without KentOS code): 105 parcels, five of them outside the
layer's filter, an owner each that a form marks `redact`. The sheet goes in through `cad.sheets.edit`, the atlas is
planned and written through the typed layer, its answer read back: a hundred pages, the pin, the bindings, and a pin
that is refused once the drawing has moved on.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/atlas/v1/cases.json").read_text(encoding="utf-8"))


def drawing() -> cad.Document:
    setup = CASES["setups"]["main"]
    doc = cad.Document.from_bytes(json.dumps(setup["drawing"]).encode())
    for sheet in setup["book"]["sheets"]:
        cad.sheets.edit(doc, ops=[{"op": "addSheet", "sheet": sheet}])
    return doc


class AtlasTests(unittest.TestCase):
    def test_a_hundred_parcels_make_a_hundred_pages_from_a_pinned_state(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = drawing()
            out = Path(tmp) / "atlas.pdf"
            planned = cad.file.export.atlas.plan(doc, path=str(out), sheet="atlas-paftasi")
            self.assertFalse(out.exists(), "plan writes nothing")
            self.assertEqual(len(planned.pages), 100)
            self.assertEqual(planned.pages[0].name, "A1/1")
            self.assertEqual(planned.filtered_out, 5)
            self.assertEqual(planned.fields, ["Ada", "Parsel", "Durum"])
            self.assertIn("Malik", planned.withheld)
            written = cad.file.export.atlas(doc, path=str(out), sheet="atlas-paftasi", pin=planned.pin)
            self.assertEqual(len(written.pages), 100)
            self.assertEqual(written.pin.digest, planned.pin.digest)
            data = out.read_bytes()
            self.assertTrue(data.startswith(b"%PDF-"))
            self.assertIn(planned.pin.digest.encode(), data, "the keywords say what it was made from")
            self.assertEqual(written.bindings.srid, 5253)

    def test_a_pin_the_drawing_has_moved_on_from_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            doc = drawing()
            out = Path(tmp) / "atlas.pdf"
            pin = cad.file.export.atlas.plan(doc, path=str(out), sheet="atlas-paftasi").pin
            cad.polygon.create(
                doc, layer_id="parsel", pts=[(1.0, 1.0), (5.0, 1.0), (5.0, 4.0)]
            )
            with self.assertRaises(cad.RevisionConflict) as refused:
                cad.file.export.atlas(doc, path=str(out), sheet="atlas-paftasi", pin=pin)
            self.assertEqual(refused.exception.code, "pin_mismatch")
            self.assertFalse(out.exists())

    def test_pages_drawn_in_another_system_say_both(self) -> None:
        doc = drawing()
        planned = cad.file.export.atlas.plan(doc, path="/tmp/atlas.pdf", sheet="atlas-paftasi", crs=5254)
        self.assertEqual(planned.bindings.srid, 5254)
        self.assertEqual(planned.bindings.project_srid, 5253)
        self.assertEqual(planned.bindings.crs_name, "TUREF / TM30")
        self.assertEqual(len(planned.pages), 100)

    def test_a_coverage_not_bound_is_not_read_from_a_source(self) -> None:
        doc = drawing()
        with self.assertRaises(CommandFailed) as refused:
            cad.file.export.atlas.plan(doc, path="/tmp/atlas.pdf", sheet="atlas-paftasi", coverage="source")
        self.assertEqual(refused.exception.code, "coverage_not_bound")

    def test_the_bindings_are_explicit(self) -> None:
        doc = drawing()
        with self.assertRaises(CommandFailed) as refused:
            cad.file.export.atlas.plan(doc, path="/tmp/atlas.pdf", sheet="atlas-paftasi", crs=4326)
        self.assertEqual(refused.exception.code, "crs_unsupported")


if __name__ == "__main__":
    unittest.main()
