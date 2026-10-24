"""GeoPackage tile pyramids as rasters from Python (docs/adr/0556, plan/gis.md CBS-03's raster profile):
``kentos.cad.rasters.gpkgwrite`` writes a GDAL-made GeoTIFF of the drawing as a tile table and
``kentos.cad.rasters.gpkgread`` reads GDAL's own GeoPackage back as a raster layer, through the same commands the
desktop's window runs. Checked here with the standard library only (sqlite3 and the files' headers); the pixels are held
to GDAL's own by scripts/fixtures/gpkg_tiles_cases.py.
"""

from __future__ import annotations

import json
import sqlite3
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / "fixtures/gpkg-tiles/v1"


def drawing(file: str, width: int, height: int, bands: int, pixel: float) -> cad.Document:
    setup = {
        "format": "kentos.document", "version": 1, "name": "Karolar",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                     "plotScale": 1000, "workspace": "gis"},
        "origin": {"x": 487200.0, "y": 4420600.0},
        "layers": [{"id": "altlik", "name": "Altlık", "type": "layer", "visible": True, "locked": False,
                    "expanded": True, "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25},
                    "children": []}],
        "activeLayer": "altlik",
        "entities": [{
            "kind": "raster", "layerId": "altlik", "attrs": {}, "id": 1,
            "affine": [487200.0, pixel, 0.0, 4420600.0, 0.0, -pixel], "width": width, "height": height,
            "bands": bands, "sample": "u8", "srid": 5256, "file": str(FOLDER / "kaynak" / file),
            "style": {"render": "rgb", "bands": [1, 2, 3]},
        }],
        "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(setup).encode())


class GpkgTilesTests(unittest.TestCase):
    def test_a_raster_goes_out_as_a_pyramid_of_png_tiles(self) -> None:
        doc = drawing("rgb.tif", 700, 300, 3, 1.5)
        uid = next(iter(doc.entities(page_size=10))).uid
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "orto.gpkg"
            plan = cad.rasters.gpkgwrite.plan(doc, uid=uid, path=str(path), table="orto")
            self.assertFalse(path.exists(), "a plan writes nothing")
            made = cad.rasters.gpkgwrite(doc, uid=uid, path=str(path), table="orto")
            self.assertEqual(made.tiles, plan.tiles)
            self.assertEqual([z.zoom for z in made.zooms], [0, 1, 2])
            # 700 × 300 halves to 350 × 150 and 175 × 75: 3 × 2, 2 × 1 and one tile of a 4 × 4, 2 × 2, 1 × 1 matrix.
            self.assertEqual([z.tiles for z in made.zooms], [1, 2, 6])
            self.assertEqual([z.matrix_width for z in made.zooms], [1, 2, 4])
            self.assertEqual(list(made.zooms[2].pixel), [1.5, 1.5])
            self.assertEqual(made.srid, 5256)
            self.assertEqual(made.bands, 4, "an RGB raster's tiles carry alpha")
            self.assertFalse(doc.info().can_undo, "writing tiles changes no drawing")
            db = sqlite3.connect(path)
            self.assertEqual(db.execute("select count(*) from orto").fetchone()[0], 9)
            kind = db.execute("select data_type from gpkg_contents where table_name='orto'").fetchone()[0]
            self.assertEqual(kind, "tiles")
            first = db.execute("select tile_data from orto limit 1").fetchone()[0]
            self.assertTrue(first.startswith(b"\x89PNG"))
            db.close()
            # A table is added to the file that is there; the same name is refused.
            more = cad.rasters.gpkgwrite(doc, uid=uid, path=str(path), table="ikinci", mode="add")
            self.assertEqual(more.table, "ikinci")
            with self.assertRaises(cad.CommandFailed) as refused:
                cad.rasters.gpkgwrite(doc, uid=uid, path=str(path), table="orto", mode="add")
            self.assertEqual(refused.exception.code, "table_exists")

    def test_gdals_geopackage_is_read_as_a_raster_layer_and_undone(self) -> None:
        doc = drawing("rgb.tif", 700, 300, 3, 1.5)
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "orto.tif"
            described = cad.rasters.gpkgread(doc, path=str(FOLDER / "orto.gpkg"))
            self.assertEqual([t.table for t in described.tables], ["orto"])
            self.assertEqual((described.width, described.height), (600, 420))
            self.assertIs(described.path, cad.UNSET)
            done = cad.rasters.gpkgread(
                doc, path=str(FOLDER / "orto.gpkg"), zoom=1, output=str(out), layer="altlik",
            )
            self.assertEqual((done.width, done.height, done.bands), (300, 210, 4))
            self.assertEqual(done.affine[1], 4.0)
            self.assertEqual(done.srid, 5256)
            self.assertEqual(done.tiles_missing, 0)
            self.assertTrue(out.read_bytes().startswith(b"II"), "a GeoTIFF")
            self.assertTrue(done.uid)
            self.assertEqual(doc.undo(), "GeoPackage karoları")
            # JPEG tiles are read since docs/adr/0627 (zune-jpeg): the zoom with tiles, its window as a GeoTIFF.
            jpeg = cad.rasters.gpkgread(doc, path=str(FOLDER / "jpeg.gpkg"), output=str(Path(tmp) / "jpeg.tif"))
            self.assertEqual((jpeg.table, jpeg.zoom), ("foto", 1))
            self.assertEqual((jpeg.width, jpeg.height, jpeg.bands), (300, 200, 4))
            self.assertTrue((Path(tmp) / "jpeg.tif").read_bytes().startswith(b"II"), "a GeoTIFF")


if __name__ == "__main__":
    unittest.main()
