"""Bulutu konumla (`cad.pointclouds.place`, docs/adr/5100; plan/map-cad.md MAP-13), held by this test's own rotations.

Euler angles are checked against the product of three unit quaternions, q_z(yaw) · q_y(pitch) · q_x(roll) — the turns
about the turned axes z, y′, x″ of OGC GeoPose 1.0 Basic-YPR composed as quaternions, a formula apart from the core's
rows — so the two agree only when the order, the axes and the signs are the same.

- Euler angles (30°, −5°, 2°) about a centre with a move: each file's place has the quaternions' rotation, the centre
  goes to itself plus the move; the cloud's box is the file's box's corners moved.
- A right-handed quarter yaw takes +X to +Y about the centre.
- Nokta sorgula finds a point where the drawing now shows it: the file's own point taken through the place.
- Konumu kaldır brings the file back; a matrix that scales is refused at `place`.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import CloudPlaceBy, Vec2  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import pointcloud_place_cases as pc  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.pointclouds.place.json").read_text("utf-8"))
XYZ = ROOT / "fixtures/pointcloud/v1/files/bulut.xyz"
CENTRE = [500050.0, 4400050.0, 125.0]


def opened() -> cad.Document:
    setup = json.loads(json.dumps(CASES["setup"]).replace("$fixture:", str(ROOT / "fixtures") + "/"))
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def q_axis(axis, degrees):
    h = math.radians(degrees) / 2
    return [math.cos(h)] + [math.sin(h) * a for a in axis]


def q_mul(a, b):
    w1, x1, y1, z1 = a
    w2, x2, y2, z2 = b
    return [w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2, w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
            w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2, w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2]


def q_rotation(q):
    w, x, y, z = q
    return [[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
            [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
            [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]]


def move(m, p):
    return [sum(m[4 * i + k] * p[k] for k in range(3)) + m[4 * i + 3] for i in range(3)]


def euler(yaw, pitch, roll, center=None, translation=None):
    return CloudPlaceBy.from_json({"kind": "euler", "yaw": yaw, "pitch": pitch, "roll": roll,
                                   **({"center": center} if center else {}),
                                   **({"translation": translation} if translation else {})})


class CloudPlace(unittest.TestCase):
    def test_euler_angles_are_the_quaternions_turns_about_z_then_y_then_x(self) -> None:
        doc = opened()
        uid = pc.uid(1)
        out = cad.pointclouds.place(doc, uid=uid, place=euler(30.0, -5.0, 2.0, CENTRE, [1.0, 2.0, 3.0]))
        m = out.poses[0]
        q = q_mul(q_mul(q_axis([0, 0, 1], 30.0), q_axis([0, 1, 0], -5.0)), q_axis([1, 0, 0], 2.0))
        r = q_rotation(q)
        for i in range(3):
            for j in range(3):
                self.assertAlmostEqual(m[4 * i + j], r[i][j], places=12)
        c = move(m, CENTRE)
        for k, t in enumerate([1.0, 2.0, 3.0]):
            self.assertAlmostEqual(c[k], CENTRE[k] + t, places=6)
        # The cloud's box: the file's box's eight corners moved.
        b = [500000.0, 4400000.0, 100.0, 500100.0, 4400100.0, 150.0]
        corners = [move(m, [b[0 + 3 * (i & 1)], b[1 + 3 * ((i >> 1) & 1)], b[2 + 3 * ((i >> 2) & 1)]]) for i in range(8)]
        want = [min(p[k] for p in corners) for k in range(3)] + [max(p[k] for p in corners) for k in range(3)]
        for got, w in zip(out.bounds, want):
            self.assertAlmostEqual(got, w, places=6)

    def test_a_right_handed_quarter_yaw_takes_east_to_north(self) -> None:
        doc = opened()
        out = cad.pointclouds.place(doc, uid=pc.uid(1), place=euler(90.0, 0.0, 0.0, CENTRE))
        p = move(out.poses[0], [CENTRE[0] + 1.0, CENTRE[1], CENTRE[2]])
        self.assertAlmostEqual(p[0], CENTRE[0], places=9)
        self.assertAlmostEqual(p[1], CENTRE[1] + 1.0, places=9)

    def test_the_query_finds_the_point_where_the_drawing_shows_it(self) -> None:
        doc = opened()
        uid = pc.uid(1)
        out = cad.pointclouds.place(doc, uid=uid, place=euler(0.5, 0.25, -0.1, None, [3.0, -4.0, 0.5]))
        first = [float(v) for v in XYZ.read_text().splitlines()[1].split()[:3]]
        there = move(out.poses[0], first)
        found = cad.pointclouds.query(doc, at=Vec2(x=there[0], y=there[1]), radius=0.05, uids=[uid]).point
        self.assertIsNotNone(found)
        self.assertEqual(found.index, 0)
        for got, want in zip((found.x, found.y, found.z), there):
            self.assertAlmostEqual(got, want, places=6)

    def test_konumu_kaldir_and_a_scaled_matrix(self) -> None:
        doc = opened()
        placed = pc.uid(3)
        out = cad.pointclouds.place(doc, uid=placed, place=CloudPlaceBy.from_json({"kind": "remove"}))
        self.assertEqual(out.poses, [None])
        self.assertTrue(out.changed)
        scaled = [1.001, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]
        with self.assertRaises(CommandFailed) as caught:
            cad.pointclouds.place(doc, uid=pc.uid(1),
                                  place=CloudPlaceBy.from_json({"kind": "matrix", "matrix": scaled}))
        self.assertEqual(caught.exception.path, "place")


if __name__ == "__main__":
    unittest.main()
