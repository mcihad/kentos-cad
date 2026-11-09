"""A project opened from its delivery package finds the grids it names there (docs/adr/3580; plan/map-cad.md MAP-17),
through KentOS's Python, held against an independent reading (hashlib; no KentOS code in the expectations).

The scale factors' TM36 project (fixtures/commands/v1/cad.crs.factors.json) with its heights on a geoid model of its own
(TG-20 with its last node moved two millimetres: no other test holds it) is packed with the model in a scratch library
(`KENTOS_GRID_DIR`). With the library empty:

- the project file copied alone: `cad.crs.factors` is refused, `geoid_missing`;
- the package's own file: the model is found in `izgaralar/` (its name its SHA-256, hashlib) and the command answers;
- the empty library stays empty: opening does not take grids into it.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import struct
import tempfile
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


class PackageGridsFound(unittest.TestCase):
    def setUp(self) -> None:
        self.folder = Path(tempfile.mkdtemp())
        self.before = os.environ.get("KENTOS_GRID_DIR")

    def tearDown(self) -> None:
        if self.before is None:
            os.environ.pop("KENTOS_GRID_DIR", None)
        else:
            os.environ["KENTOS_GRID_DIR"] = self.before
        shutil.rmtree(self.folder, ignore_errors=True)

    def test_the_package_s_geoid_is_found_beside_its_file(self) -> None:
        data = bytearray((ROOT / "fixtures/geoid/v1/tg.gtx").read_bytes())
        (last,) = struct.unpack(">f", data[-4:])
        data[-4:] = struct.pack(">f", last + 0.002)
        sha = hashlib.sha256(data).hexdigest()
        library, empty = self.folder / "izgara", self.folder / "bos"
        library.mkdir()
        empty.mkdir()
        (library / f"{sha}.gtx").write_bytes(data)
        cases = json.loads((ROOT / "fixtures/commands/v1/cad.crs.factors.json").read_text("utf-8"))
        case = next(c for c in cases["cases"] if c["name"].startswith("Ortometrik kotlar ve TG-20 geoidi"))
        setup = case["setup"]
        setup["settings"]["heights"]["geoid"] = {"name": "TG-20 (paket)", "sha256": sha}
        doc = cad.Document.from_bytes(json.dumps(setup).encode())
        os.environ["KENTOS_GRID_DIR"] = str(library)
        package = cad.command("file.package")
        out = self.folder / "teslim"
        packed = package.run(doc, package.input_type.from_json({"path": str(out), "name": "harita"}), op="execute")
        self.assertIsNone(packed.error, packed.error and packed.error.message)
        kept = out / "izgaralar" / f"{sha}.gtx"
        self.assertEqual(hashlib.sha256(kept.read_bytes()).hexdigest(), sha)
        # Another machine: an empty library.
        os.environ["KENTOS_GRID_DIR"] = str(empty)
        factors = cad.command("cad.crs.factors")
        ask = factors.input_type.from_json(case["steps"][0]["input"])
        alone = self.folder / "tek"
        alone.mkdir()
        shutil.copy(out / "harita.kcad", alone / "harita.kcad")
        refused = factors.run(cad.Document.open(alone / "harita.kcad"), ask, op="execute")
        self.assertIsNotNone(refused.error)
        self.assertEqual(refused.error.code, "geoid_missing")
        answered = factors.run(cad.Document.open(out / "harita.kcad"), ask, op="execute")
        self.assertIsNone(answered.error, answered.error and answered.error.message)
        self.assertIsInstance(answered.output.points[0].undulation, float)
        self.assertEqual(list(empty.iterdir()), [], "opening takes no grid into the library")


if __name__ == "__main__":
    unittest.main()
