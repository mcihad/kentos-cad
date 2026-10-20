"""COG olarak kaydet from Python (plan/gis.md CBS-28, docs/adr/0515).

The drawing of fixtures/cog/v1 (its rasters' files named from its `src` folder, found there through the project's
search path, docs/adr/0548 §3) run through `cad.processing.run`: the headless server reads the linked GeoTIFF and
writes a cloud-optimized one. Checked here with the standard library
only: the header, GDAL's structural metadata naming the directories before the data, and every directory before every
tile.
"""

from __future__ import annotations

import struct
import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
FOLDER = ROOT / "fixtures/cog/v1"


def layout(b: bytes) -> tuple[list[int], list[int]]:
    """A classic little-endian TIFF's directories' places and its tiles' offsets."""
    assert b[:4] == b"II*\0"
    dirs, tiles = [], []
    at = struct.unpack("<I", b[4:8])[0]
    while at:
        dirs.append(at)
        n = struct.unpack("<H", b[at:at + 2])[0]
        for k in range(n):
            e = at + 2 + 12 * k
            tag, kind, count, value = struct.unpack("<HHII", b[e:e + 12])
            if tag == 324:
                if count == 1:
                    tiles.append(value)
                else:
                    size = 2 if kind == 3 else 4
                    fmt = "<" + ("H" if size == 2 else "I") * count
                    tiles.extend(struct.unpack(fmt, b[value:value + size * count]))
        at = struct.unpack("<I", b[at + 2 + 12 * n:at + 6 + 12 * n])[0]
    return dirs, tiles


class CogTests(unittest.TestCase):
    def test_a_linked_geotiff_goes_out_as_a_cog(self) -> None:
        doc = cad.Document.open(FOLDER / "cog.kcad")
        # The rasters are in the drawing's `src` folder: a search path, from whatever folder Python runs in.
        doc.call("cad.xrefs.edit", {"operation": {"kind": "paths", "paths": ["src"]}})
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "kot-cog.tif"
            ran = cad.processing.run(doc, tool="raster.cog", parameters={
                "input": {"scope": "layer", "layerId": "kot"},
                "output": str(out),
                "add": True,
            })
            self.assertEqual(len(ran.added), 1, "the COG's raster in the drawing")
            b = out.read_bytes()
        self.assertTrue(b[8:].startswith(b"GDAL_STRUCTURAL_METADATA_SIZE="))
        self.assertIn(b"LAYOUT=IFDS_BEFORE_DATA", b[:200])
        dirs, tiles = layout(b)
        self.assertEqual(len(dirs), 3, "the image and two overviews")
        self.assertTrue(max(dirs) < min(t for t in tiles if t), "every directory before every tile")


if __name__ == "__main__":
    unittest.main()
