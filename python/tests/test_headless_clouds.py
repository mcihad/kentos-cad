"""Nokta bulutu ve çok boyutlu araçlar Python'dan (plan/gis.md CBS-41, docs/adr/0572).

The headless server's files read a point cloud's LAS, LAZ or COPC and a NetCDF file's slice as the desktop does (one
shared reader, `kentos_processing::cloud_files`). Here:

- the multidimensional tools' shared cases (fixtures/processing/v1/multidim.json, scripts/fixtures/
  multidim_processing_cases.py) run through `cad.processing.run`: each case's summary, table and refusal, and the
  files the Mesh hesaplayıcı writes byte for byte as the reference's;
- the village's cloud (fixtures/interaction/v1/pointclouds.kcad) thinned to LAZ and to COPC (the index built by the
  headless server) and its ground filtered, the counts those of the independent reference
  (fixtures/pointcloud/v1/ops.json), read back from the written files' headers with the standard library.

Both drawings are copied into a scratch folder first: the tools write beside their sources.
"""

from __future__ import annotations

import json
import shutil
import struct
import tempfile
import unittest
from pathlib import Path
from typing import Any

import kentos.cad as cad
from kentos.cad.errors import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
MULTIDIM = json.loads((ROOT / "fixtures/processing/v1/multidim.json").read_text(encoding="utf-8"))
OPS = json.loads((ROOT / "fixtures/pointcloud/v1/ops.json").read_text(encoding="utf-8"))["cases"]


def las_count(path: Path) -> tuple[int, int]:
    """A LAS, LAZ or COPC file's point count from its header and its version's minor number."""
    b = path.read_bytes()[:375]
    assert b[:4] == b"LASF", path
    minor = b[25]
    count = struct.unpack("<Q", b[247:255])[0] if minor >= 4 else struct.unpack("<I", b[107:111])[0]
    return count, minor


class MultidimTests(unittest.TestCase):
    def test_the_shared_cases_run_headless(self) -> None:
        for case in MULTIDIM["cases"]:
            with self.subTest(case["id"]), tempfile.TemporaryDirectory() as tmp:
                folder = Path(tmp)
                shutil.copy(ROOT / "fixtures/processing/v1/multidim.kcad", folder / "multidim.kcad")
                shutil.copytree(ROOT / "fixtures/processing/v1/multidim", folder / "multidim")
                doc = cad.Document.open(folder / "multidim.kcad")
                # The NetCDF files are in the drawing's `multidim` folder: a search path.
                doc.call("cad.xrefs.edit", {"operation": {"kind": "paths", "paths": ["multidim"]}})
                expect: dict[str, Any] = case["expect"]
                params = {"tool": case["run"]["tool"], "parameters": case["values"]}
                if expect["status"] == "error":
                    with self.assertRaises(CommandFailed) as caught:
                        cad.processing.run(doc, **params)
                    self.assertIn(expect["message"], str(caught.exception))
                    continue
                ran = cad.processing.run(doc, **params)
                written_at = (ran.outputs or {}).get("file")
                summary = ran.summary
                if written_at:
                    # The shared cases name the written file; here it is written where it is asked or beside its source.
                    summary = summary.replace(written_at, Path(written_at).name)
                self.assertEqual(summary, expect["summary"])
                want_table = (expect.get("outputs") or {}).get("table")
                if want_table:
                    self.assertEqual((ran.outputs or {})["table"]["rows"], want_table["rows"])
                for written, reference in (expect.get("multidimOf") or {}).items():
                    self.assertEqual(Path(written_at).name, written)
                    got = Path(written_at).read_bytes()
                    self.assertEqual(got, (ROOT / "fixtures/multidim/v1/files" / reference).read_bytes(), written)


class CloudTests(unittest.TestCase):
    def test_the_village_is_thinned_indexed_and_filtered_headless(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            shutil.copy(ROOT / "fixtures/interaction/v1/pointclouds.kcad", folder / "pointclouds.kcad")
            shutil.copytree(ROOT / "fixtures/interaction/v1/pointclouds", folder / "pointclouds")
            doc = cad.Document.open(folder / "pointclouds.kcad")
            koy = {"scope": "layer", "layerId": "koy"}

            out = folder / "seyrek.laz"
            ran = cad.processing.run(doc, tool="pointcloud.thin",
                                     parameters={"input": koy, "size": 1.0, "output": str(out)})
            self.assertIn("13.967 nokta kaldı", ran.summary)
            self.assertEqual(las_count(out)[0], OPS["thinCell"]["count"])

            out = folder / "seyrek-dizinli.copc.laz"
            ran = cad.processing.run(doc, tool="pointcloud.thin",
                                     parameters={"input": koy, "size": 1.0, "format": "copc", "output": str(out)})
            count, minor = las_count(out)
            self.assertEqual((count, minor), (OPS["thinCell"]["count"], 4), "a COPC is LAS 1.4")
            self.assertIn(b"copc", out.read_bytes()[:2048], "the COPC info VLR")
            self.assertFalse(any(p.name.endswith((".yaziliyor", ".kutular")) for p in folder.iterdir()))

            out = folder / "zemin.laz"
            ran = cad.processing.run(doc, tool="pointcloud.ground", parameters={"input": koy, "output": str(out)})
            self.assertIn("57.461 nokta zemin", ran.summary)
            self.assertEqual(las_count(out)[0], 71_212)


if __name__ == "__main__":
    unittest.main()
