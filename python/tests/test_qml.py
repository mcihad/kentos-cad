"""A layer's look in and out as a QGIS layer style file from Python (plan/gis.md CBS-17, docs/adr/0555).

QGIS 4.2's categorized QML (fixtures/qml/v1) made a layer's look, then written back out; read here with the standard
library only: the categories, their values and colours, and the NULL class QGIS keeps for all other values.
"""

from __future__ import annotations

import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
QML = ROOT / "fixtures/qml/v1/qgis-kategorili.qml"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"


class QmlTests(unittest.TestCase):
    def test_qgis_look_comes_in_and_goes_out(self) -> None:
        doc = cad.Document.new("Ada 101", srid=5256)
        took = cad.file.import_.gpkg(doc, path=str(GPKG))
        parsel = next(layer.id for layer in took.layers if layer.name == "parsel")
        got = cad.file.import_.qml(doc, layer=parsel, path=str(QML))
        self.assertEqual(got.kind, "categorized")
        self.assertEqual(got.renderer["expr"], "[ada]")
        self.assertEqual([c["value"] for c in got.renderer["categories"]], ["A", "B", "C", "D"])
        self.assertIn("other", got.renderer)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "parsel.qml"
            written = cad.file.export.qml(doc, layer=parsel, path=str(out))
            self.assertEqual(written.rules, 5)
            root = ET.parse(out).getroot()
            renderer = root.find("renderer-v2")
            self.assertEqual(renderer.get("type"), "categorizedSymbol")
            self.assertEqual(renderer.get("attr"), "ada")
            cats = renderer.find("categories").findall("category")
            self.assertEqual([c.get("value") for c in cats], ["A", "B", "C", "D", "NULL"])
            self.assertEqual(cats[-1].get("type"), "NULL")
            colours = [o.get("value") for o in renderer.iter("Option") if o.get("name") == "color"]
            self.assertEqual(colours[:2], ["228,26,28,255", "55,126,184,255"])
            with self.assertRaises(CommandFailed) as refused:
                cad.file.export.qml(doc, layer=parsel, path=str(out))
            self.assertEqual(refused.exception.code, "file_exists")
        with self.assertRaises(CommandFailed) as bad:
            cad.file.import_.qml(doc, layer=parsel, text="<x/>")
        self.assertEqual(bad.exception.code, "invalid_qml")


if __name__ == "__main__":
    unittest.main()
