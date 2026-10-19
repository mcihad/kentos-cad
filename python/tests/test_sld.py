"""A layer's look in and out as SLD from Python (plan/gis.md CBS-17, docs/adr/0516).

QGIS 4.2's categorized SLD (fixtures/sld/v1) made a layer's look, then written back out; read here with the standard
library only: the rules, their filters and colours.
"""

from __future__ import annotations

import tempfile
import unittest
import xml.etree.ElementTree as ET
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
SLD = ROOT / "fixtures/sld/v1/qgis-kategorili.sld"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"
SE = "{http://www.opengis.net/se}"
OGC = "{http://www.opengis.net/ogc}"


class SldTests(unittest.TestCase):
    def test_qgis_look_comes_in_and_goes_out(self) -> None:
        doc = cad.Document.new("Ada 101", srid=5256)
        took = cad.file.import_.gpkg(doc, path=str(GPKG))
        parsel = next(layer.id for layer in took.layers if layer.name == "parsel")
        got = cad.file.import_.sld(doc, layer=parsel, path=str(SLD))
        self.assertEqual(got.kind, "categorized")
        self.assertEqual(got.rules, 2)
        self.assertEqual(got.renderer["expr"], "[ada]")
        self.assertEqual([c["value"] for c in got.renderer["categories"]], ["A", "B"])
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "parsel.sld"
            written = cad.file.export.sld(doc, layer=parsel, path=str(out))
            self.assertEqual(written.rules, 2)
            root = ET.parse(out).getroot()
            rules = list(root.iter(f"{SE}Rule"))
            self.assertEqual([r.find(f"{SE}Name").text for r in rules], ["Ada A", "Ada B"])
            literals = [r.find(f".//{OGC}Literal").text for r in rules]
            self.assertEqual(literals, ["A", "B"])
            fills = [p.text for p in root.iter(f"{SE}SvgParameter") if p.get("name") == "fill"]
            self.assertEqual(fills, ["#e41a1c", "#377eb8"])
            with self.assertRaises(CommandFailed) as refused:
                cad.file.export.sld(doc, layer=parsel, path=str(out))
            self.assertEqual(refused.exception.code, "file_exists")
        with self.assertRaises(CommandFailed) as bad:
            cad.file.import_.sld(doc, layer=parsel, text="<x/>")
        self.assertEqual(bad.exception.code, "invalid_sld")


if __name__ == "__main__":
    unittest.main()
