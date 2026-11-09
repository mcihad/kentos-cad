"""Dönüşüm uygula by an axis and an angle (`pointcloud.transform`'s Eksen ve açı, docs/adr/4300; plan/map-cad.md MAP-13),
held by the test's own rotation: a unit quaternion (cos θ/2, sin θ/2 · n), a formula apart from the core's Rodrigues
rows, so the two agree only when the convention is the same (right-handed, degrees).

- Z ekseni çevresinde 0,5°, merkez boş: the cloud object's history keeps a matrix whose rotation is the quaternion's,
  whose centre (the middle of the cloud's box) goes to itself plus the move.
- A tilted axis (2, 2, 1; its length does not matter) at −0.25° about a given centre: the same.
- The core's own unit test turns (11, 0, 0) a quarter about Z about (10, 0, 0) to (10, 1, 0); here a right-handed
  quarter turn about X takes +Y to +Z.
"""

from __future__ import annotations

import json
import math
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

PROCESSING = json.loads((ROOT / "fixtures/processing/v1/pointcloud.json").read_text(encoding="utf-8"))


def quaternion_rotation(axis, degrees):
    """The rotation matrix of the unit quaternion about `axis` by `degrees`, right-handed."""
    n = math.sqrt(sum(a * a for a in axis))
    h = math.radians(degrees) / 2
    w, x, y, z = math.cos(h), *(math.sin(h) * a / n for a in axis)
    return [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]


def history_matrix(words: str) -> list[float]:
    return [float(v) for v in words.split()]


class CloudAxisAngle(unittest.TestCase):
    def ran(self, values: dict) -> tuple[list[float], list[float]]:
        """The run's history matrix and the cloud's box middle (the source's)."""
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            shutil.copy(ROOT / "fixtures/processing/v1/pointcloud.kcad", folder / "pointcloud.kcad")
            for name, rel in PROCESSING["rasters"].items():
                shutil.copy((ROOT / "fixtures/processing/v1" / rel).resolve(), folder / name)
            doc = cad.Document.open(folder / "pointcloud.kcad")
            source = next(r.entity.to_json() for r in doc.entities(layer="koy"))
            b = source["bounds"]
            middle = [(b[k] + b[k + 3]) / 2 for k in range(3)]
            run = cad.processing.run(doc, tool="pointcloud.transform",
                                     parameters={**values, "output": str(folder / "dondu.laz")})
            added = doc.entity(run.added[0]).entity.to_json()
            return history_matrix(added["attrs"]["Dönüşümler"]), middle

    def check(self, m: list[float], axis, degrees, centre, move) -> None:
        r = quaternion_rotation(axis, degrees)
        for i in range(3):
            for j in range(3):
                self.assertAlmostEqual(m[4 * i + j], r[i][j], delta=1e-12, msg=f"R[{i}][{j}]")
            image = m[4 * i] * centre[0] + m[4 * i + 1] * centre[1] + m[4 * i + 2] * centre[2] + m[4 * i + 3]
            self.assertAlmostEqual(image, centre[i] + move[i], delta=1e-6, msg=f"centre {i}")
        self.assertEqual(m[12:], [0.0, 0.0, 0.0, 1.0])

    def test_a_turn_about_z_about_the_cloud_s_middle(self) -> None:
        m, middle = self.ran({"input": {"scope": "layer", "layerId": "koy"}, "form": "axisAngle", "axis": "0 0 1",
                              "angle": 0.5, "translation": "1 -2 0.5"})
        self.check(m, (0, 0, 1), 0.5, middle, (1, -2, 0.5))

    def test_a_tilted_axis_about_a_given_centre(self) -> None:
        centre = (487030.0, 4420040.0, 850.0)
        m, _ = self.ran({"input": {"scope": "layer", "layerId": "koy"}, "form": "axisAngle", "axis": "2, 2, 1",
                         "angle": -0.25, "center": "487030 4420040 850"})
        self.check(m, (2, 2, 1), -0.25, centre, (0, 0, 0))

    def test_right_handed_a_quarter_about_x_takes_y_to_z(self) -> None:
        r = quaternion_rotation((1, 0, 0), 90)
        self.assertAlmostEqual(r[2][1], 1.0, delta=1e-15)  # +Y goes to +Z


if __name__ == "__main__":
    unittest.main()
