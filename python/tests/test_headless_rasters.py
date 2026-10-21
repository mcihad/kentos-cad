"""A JPEG picture, a JPEG-compressed TIFF and a virtual mosaic read by the headless server (plan/gis.md CBS-41,
docs/adr/0573).

The headless server's files read them as the desktop does (one shared reader, `kentos_processing::raster_files`).
Bölgesel istatistik runs over each through ``cad.processing.run`` with one zone covering the whole raster, and its count,
mean, least and most are GDAL's (fixtures/raster/v1/headless.json, scripts/fixtures/headless_raster_cases.py). A JPEG's
pixels are its decoder's: the JPEGs' means are held to half a unit, their counts and the mosaic's numbers exactly (to the
three decimals the table writes). An embedded mosaic is refused, saying why.
"""

from __future__ import annotations

import base64
import json
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad.errors import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
SUMMARY = json.loads((ROOT / "fixtures/raster/v1/headless.json").read_text(encoding="utf-8"))["files"]
LAYER = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}


def drawing(raster: dict, bounds: tuple[float, float, float, float], styles: dict | None = None) -> cad.Document:
    """A drawing with the raster on `raster` and one zone on `bolge` a little larger than the raster's bounds."""
    x0, y0, x1, y1 = bounds
    pad = 10.0
    zone = [{"x": x0 - pad, "y": y0 - pad}, {"x": x1 + pad, "y": y0 - pad},
            {"x": x1 + pad, "y": y1 + pad}, {"x": x0 - pad, "y": y1 + pad}]
    setup = {
        "format": "kentos.document", "version": 1, "name": "Raster",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 1000, "workspace": "gis"},
        "origin": {"x": x0, "y": y0},
        "layers": [
            {"id": "raster", "name": "Raster", "type": "layer", "visible": True, "locked": False, "expanded": True,
             "style": LAYER, "children": []},
            {"id": "bolge", "name": "Bölge", "type": "layer", "visible": True, "locked": False, "expanded": True,
             "style": LAYER, "children": []},
        ],
        "activeLayer": "raster",
        "entities": [
            {"kind": "raster", "layerId": "raster", "attrs": {}, "id": 1, "srid": 5256, **raster},
            {"kind": "polygon", "layerId": "bolge", "attrs": {"Ad": "Hepsi"}, "id": 2, "pts": zone},
        ],
        "styles": styles or {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(setup).encode())


def bounds_of(affine: list[float], width: int, height: int) -> tuple[float, float, float, float]:
    x0, px, _, y_top, _, py = affine
    x1, y_bottom = x0 + px * width, y_top + py * height
    return (min(x0, x1), min(y_top, y_bottom), max(x0, x1), max(y_top, y_bottom))


def zonal(doc: cad.Document) -> list[str]:
    ran = cad.processing.run(doc, tool="raster.zonalStatistics", parameters={
        "input": {"scope": "layer", "layerId": "raster"}, "zones": {"scope": "layer", "layerId": "bolge"},
        "stat": "mean", "output": "Ortalama",
    })
    rows = (ran.outputs or {})["table"]["rows"]
    assert len(rows) == 1, rows
    return rows[0]


class HeadlessRasterTests(unittest.TestCase):
    def check(self, name: str, raster: dict, mean_within: float) -> None:
        want = SUMMARY[name]
        affine = raster["affine"]
        doc = drawing(raster, bounds_of(affine, raster["width"], raster["height"]))
        _, count, _total, mean, least, most, _sd = zonal(doc)
        self.assertEqual(int(count), want["count"], f"{name}: cells")
        if mean_within == 0:
            # The table's three decimals: GDAL's numbers written the same way.
            self.assertEqual((mean, least, most), (f"{want['mean']:.3f}", f"{want['min']:.3f}", f"{want['max']:.3f}"))
        else:
            self.assertLessEqual(abs(float(mean) - want["mean"]), mean_within, f"{name}: mean {mean} for {want['mean']}")

    def test_a_jpeg_picture(self) -> None:
        info = SUMMARY["rgb.jpg"]
        self.check("rgb.jpg", {
            "affine": [487000.0, 1.0, 0.0, 4420300.0, 0.0, -1.0], "width": info["width"], "height": info["height"],
            "bands": 3, "sample": "u8", "file": str(ROOT / "fixtures/raster/v1/files/rgb.jpg"),
            "style": {"render": "rgb", "bands": [1, 2, 3]},
        }, 0.5)

    def test_a_jpeg_compressed_tiff(self) -> None:
        info = SUMMARY["rgb-jpeg-ycbcr.tif"]
        self.check("rgb-jpeg-ycbcr.tif", {
            "affine": info["affine"], "width": info["width"], "height": info["height"], "bands": 3, "sample": "u8",
            "file": str(ROOT / "fixtures/raster/v1/files/rgb-jpeg-ycbcr.tif"),
            "style": {"render": "rgb", "bands": [1, 2, 3]},
        }, 0.5)

    def test_a_virtual_mosaic(self) -> None:
        info = SUMMARY["kentos-dem-verilen.vrt"]
        self.check("kentos-dem-verilen.vrt", {
            "affine": info["affine"], "width": info["width"], "height": info["height"], "bands": 1, "sample": "f32",
            "file": str(ROOT / "fixtures/raster/v1/mosaic/kentos-dem-verilen.vrt"),
            "style": {"render": "gray", "bands": [1]},
        }, 0.0)

    def test_an_embedded_mosaic_is_refused(self) -> None:
        info = SUMMARY["kentos-dem-verilen.vrt"]
        text = (ROOT / "fixtures/raster/v1/mosaic/kentos-dem-verilen.vrt").read_bytes()
        lib = {"items": [{"kind": "asset", "id": "raster-mozaik", "name": "mozaik.vrt",
                          "data": "data:application/xml;base64," + base64.b64encode(text).decode()}],
               "categories": []}
        doc = drawing({
            "affine": info["affine"], "width": info["width"], "height": info["height"], "bands": 1, "sample": "f32",
            "asset": "raster-mozaik", "style": {"render": "gray", "bands": [1]},
        }, bounds_of(info["affine"], info["width"], info["height"]), lib)
        with self.assertRaises(CommandFailed) as caught:
            zonal(doc)
        self.assertIn("Gömülü mozaik (VRT) açılmaz", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
