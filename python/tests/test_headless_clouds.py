"""Nokta bulutu ve çok boyutlu araçlar Python'dan (plan/gis.md CBS-41, docs/adr/0572).

The headless server's files read a point cloud's LAS, LAZ or COPC and a NetCDF file's slice as the desktop does (one
shared reader, `kentos_processing::cloud_files`). Here:

- the multidimensional tools' shared cases (fixtures/processing/v1/multidim.json, scripts/fixtures/
  multidim_processing_cases.py) run through `cad.processing.run`: each case's summary, table and refusal, and the
  files the Mesh hesaplayıcı writes byte for byte as the reference's;
- the village's cloud (fixtures/interaction/v1/pointclouds.kcad) thinned to LAZ and to COPC (the index built by the
  headless server) and its ground filtered, the counts those of the independent reference
  (fixtures/pointcloud/v1/ops.json), read back from the written files' headers with the standard library;
- Dönüşüm uygula's shared cases (fixtures/processing/v1/pointcloud.json, docs/adr/3100): each summary and refusal, the
  written file's count and box from its header, and a LAS's points' whole numbers as the reference's FNV-1a.

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
PROCESSING = json.loads((ROOT / "fixtures/processing/v1/pointcloud.json").read_text(encoding="utf-8"))


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


def las_box(path: Path) -> list[float]:
    """A LAS, LAZ or COPC file's box from its header: min X, Y, Z, max X, Y, Z."""
    mx, nx, my, ny, mz, nz = struct.unpack("<6d", path.read_bytes()[179:227])
    return [nx, ny, nz, mx, my, mz]


def las_points_fnv(path: Path) -> str:
    """An uncompressed LAS's points' X, Y, Z whole numbers in the file's order, as FNV-1a 64."""
    b = path.read_bytes()
    start = struct.unpack("<I", b[96:100])[0]
    size = struct.unpack("<H", b[105:107])[0]
    count, _ = las_count(path)
    h = 0xCBF29CE484222325
    for i in range(count):
        for byte in b[start + i * size:start + i * size + 12]:
            h ^= byte
            h = (h * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


class TransformTests(unittest.TestCase):
    def test_the_shared_cases_run_headless(self) -> None:
        cases = [c for c in PROCESSING["cases"] if c["run"]["tool"] == "pointcloud.transform"]
        self.assertGreaterEqual(len(cases), 10)
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            shutil.copy(ROOT / "fixtures/processing/v1/pointcloud.kcad", folder / "pointcloud.kcad")
            for name, rel in PROCESSING["rasters"].items():
                shutil.copy((ROOT / "fixtures/processing/v1" / rel).resolve(), folder / name)
            doc = cad.Document.open(folder / "pointcloud.kcad")
            for case in cases:
                with self.subTest(case["id"]):
                    expect: dict[str, Any] = case["expect"]
                    values = dict(case["values"])
                    if expect["status"] == "error":
                        with self.assertRaises(CommandFailed) as caught:
                            cad.processing.run(doc, tool="pointcloud.transform", parameters=values)
                        self.assertIn(expect["message"], str(caught.exception))
                        continue
                    (name, check), = expect["cloudOf"].items()
                    out = folder / name
                    values["output"] = str(out)
                    ran = cad.processing.run(doc, tool="pointcloud.transform", parameters=values)
                    self.assertEqual(ran.summary.replace(str(out), name), expect["summary"])
                    self.assertEqual(ran.outputs, expect["outputs"])
                    self.assertEqual(las_count(out)[0], check["count"])
                    for added in expect.get("added", []):
                        for got, want in zip(las_box(out), added["bounds"]):
                            self.assertAlmostEqual(got, want, places=9)
                    if name.endswith(".las"):
                        self.assertEqual(las_points_fnv(out), check["points"])


if __name__ == "__main__":
    unittest.main()
