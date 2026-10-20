"""A layer's look in and out as a MapLibre GL style from Python (plan/gis.md CBS-17, docs/adr/0560).

A third party's categorized style (fixtures/maplibre/v1) made a layer's look, then written back out with its GeoJSON
beside it; read here with the standard library only: the style's `match`, its source, the GeoJSON's coordinates in
WGS 84 and the layers' order when the whole map goes out.
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
STYLE = ROOT / "fixtures/maplibre/v1/kategorili.json"
GPKG = ROOT / "fixtures/geopackage/v1/qgis.gpkg"


class MaplibreTests(unittest.TestCase):
    def test_a_style_comes_in_and_goes_out(self) -> None:
        doc = cad.Document.new("Ada 101", srid=5256)
        took = cad.file.import_.gpkg(doc, path=str(GPKG))
        ids = {layer.name: layer.id for layer in took.layers}
        parsel = ids["parsel"]
        got = cad.file.import_.maplibre(doc, layer=parsel, path=str(STYLE))
        self.assertEqual(got.changed, True)
        taken = got.layers[0]
        self.assertEqual(taken.kind, "categorized")
        self.assertEqual(taken.renderer["expr"], "[ada]")
        self.assertEqual([c["value"] for c in taken.renderer["categories"]], ["A", "B", "C"])
        self.assertEqual(taken.from_, ["parsel-dolgu", "parsel-kenar"])
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "parsel.json"
            planned = cad.file.export.maplibre.plan(doc, layer=parsel, path=str(out))
            self.assertFalse(out.exists())
            written = cad.file.export.maplibre(doc, layer=parsel, path=str(out))
            self.assertEqual(written.bytes, planned.bytes)
            style = json.loads(out.read_text(encoding="utf-8"))
            self.assertEqual(style["version"], 8)
            fill = next(layer for layer in style["layers"] if layer["type"] == "fill")
            self.assertEqual(fill["paint"]["fill-color"][0], "match")
            self.assertEqual(fill["paint"]["fill-color"][1], ["to-string", ["get", "ada"]])
            data = Path(tmp) / style["sources"][parsel]["data"]
            geo = json.loads(data.read_text(encoding="utf-8"))
            self.assertEqual(len(geo["features"]), written.data[0].features)
            position = geo["features"][0]["geometry"]["coordinates"]
            while isinstance(position[0], list):
                position = position[0]
            lon, lat = position[:2]
            self.assertTrue(20 < lon < 46 and 34 < lat < 43, (lon, lat))
            with self.assertRaises(CommandFailed) as refused:
                cad.file.export.maplibre(doc, layer=parsel, path=str(out))
            self.assertEqual(refused.exception.code, "file_exists")
            # The whole map: the layer tree lists the top layer first, a style's first layer is the bottom.
            whole = Path(tmp) / "harita.json"
            everything = cad.file.export.maplibre(doc, path=str(whole))
            self.assertLess(everything.layers.index(ids["nokta"]), everything.layers.index(parsel))
            self.assertEqual(len(everything.data), len(everything.layers))
        with self.assertRaises(CommandFailed) as bad:
            cad.file.import_.maplibre(doc, layer=parsel, text="[]")
        self.assertEqual(bad.exception.code, "invalid_maplibre")


if __name__ == "__main__":
    unittest.main()
