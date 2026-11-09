"""The record checked with its field package's own surface (docs/adr/3220; plan/map-cad.md MAP-19), shown without
KentOS code that the check reads the package's `yuzey.xml` and not the drawing's surface.

KentOS writes the package of scripts/fixtures/asbuilt_cases.py's drawing (`cad.field.package`, the design surface
“Tasarım”, the plane G). Python's own `zipfile`, `xml.etree` and `hashlib` then lower every height of its `yuzey.xml`
by 5 cm and write the file's new SHA-256 into the manifest, as a later design revision would. The check with the
package's design gives every measured point's surface difference 5 cm more than with the untouched package, which
equals the drawing's (measured height − G, the generator's rule); with the SHA-256 left as it was the check refuses
the package.
"""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402

import asbuilt_cases as asb  # noqa: E402
import kcad_v2_reference as kr  # noqa: E402

LX = "http://www.landxml.org/schema/LandXML-1.2"


def lowered(data: bytes, by: float) -> bytes:
    """`yuzey.xml` with every point's height lowered by `by`, read and written by xml.etree."""
    ET.register_namespace("", LX)
    root = ET.fromstring(data)
    for p in root.iter(f"{{{LX}}}P"):
        north, east, z = p.text.split()
        p.text = f"{north} {east} {float(z) - by!r}"
    return ET.tostring(root, encoding="utf-8", xml_declaration=True)


def rewritten(src: Path, dst: Path, surface: bytes, sha256: str) -> None:
    """The package at `src` with `yuzey.xml` and the manifest's surface SHA-256 replaced, the other files as they were."""
    with zipfile.ZipFile(src) as z:
        files = {n: z.read(n) for n in z.namelist()}
    manifest = json.loads(files["saha-paketi.json"])
    manifest["surface"]["sha256"] = sha256
    files["saha-paketi.json"] = json.dumps(manifest, ensure_ascii=False, indent=2).encode("utf-8")
    files["yuzey.xml"] = surface
    with zipfile.ZipFile(dst, "w") as z:
        for n, data in files.items():
            z.writestr(n, data)


class PackageSurface(unittest.TestCase):
    def setUp(self) -> None:
        self.dir = tempfile.TemporaryDirectory()
        self.folder = Path(self.dir.name)
        self.doc = cad.Document.from_bytes(kr.container(kr.document(asb.setup())))
        self.package = self.folder / "saha.zip"
        cad.field.package(self.doc, design="tasarim", lines="bordur", surface="Tasarım", path=str(self.package),
                          stamp="2026-11-09T08:00:00")

    def tearDown(self) -> None:
        self.dir.cleanup()

    def check(self, package: Path):
        return cad.asbuilt.check(self.doc, measured="olcum", surface="Tasarım", search=0.5, package=str(package),
                                 package_design=True)

    def test_the_check_reads_the_package_s_surface(self) -> None:
        same = self.check(self.package)
        self.assertTrue(same.package.surface)
        self.assertIs(same.surface_changed, cad.UNSET)
        # The untouched package: measured − G, the drawing's.
        for p, (name, x, y, z, _) in zip(same.points, asb.MEASURED):
            if z is None:
                continue
            self.assertAlmostEqual(p.surface, z - asb.G(x, y), places=9, msg=name)
        with zipfile.ZipFile(self.package) as z:
            surface = lowered(z.read("yuzey.xml"), 0.05)
        later = self.folder / "sonra.zip"
        rewritten(self.package, later, surface, hashlib.sha256(surface).hexdigest())
        low = self.check(later)
        for a, b in zip(same.points, low.points):
            if a.surface is cad.UNSET:
                continue
            self.assertAlmostEqual(b.surface - a.surface, 0.05, places=9, msg=a.name)
        # The drawing's surface is still the one the package names: nothing said changed.
        self.assertIs(low.surface_changed, cad.UNSET)

    def test_a_surface_not_its_sha256_s_is_refused(self) -> None:
        with zipfile.ZipFile(self.package) as z:
            surface = lowered(z.read("yuzey.xml"), 0.05)
            manifest = json.loads(z.read("saha-paketi.json"))
        broken = self.folder / "bozuk.zip"
        rewritten(self.package, broken, surface, manifest["surface"]["sha256"])
        with self.assertRaises(CommandFailed) as refused:
            self.check(broken)
        self.assertEqual((refused.exception.code, refused.exception.path), ("invalid_asbuilt_check", "package"))
        self.assertIn("yuzey.xml", str(refused.exception))


if __name__ == "__main__":
    unittest.main()
