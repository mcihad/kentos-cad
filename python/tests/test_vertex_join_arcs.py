"""Köşeyi sil joining two arcs of one circle (`cad.entities.vertex` remove with `join_arcs`, docs/adr/5000; plan/map-cad.md
MAP-07), held by the test's own computation (no KentOS code in the expectations).

The drawing is scripts/fixtures/vertex_command_cases.py's scene. The polyline 19 runs (27225, 0), (17977, 6936),
(13113, 2184) by two arcs of the circle about (20000, 0), bulges 1/2 and 1/4. Its middle vertex removed with a 1 mm
tolerance leaves one arc of both sweeps: tan((4·atan ½ + 4·atan ¼)/4), and the arc's centre, from its chord and sagitta,
is still (20000, 0). The neighbour 23 runs the same arcs the other way and takes the negative. Without the tolerance
the edge is straight; the polyline 20's second arc is of another circle: straight, with `arcs_not_joined`.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import vertex_command_cases as vc  # noqa: E402

UIDS = [f"0192f5c0-5000-7000-8000-{i:012x}" for i in range(1, len(vc.ENTITIES) + 1)]
WANT = math.tan((4 * math.atan(0.5) + 4 * math.atan(0.25)) / 4)


def drawing() -> cad.Document:
    setup = dict(vc.SETUP, version=2, uids=UIDS)
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def shape(doc: cad.Document, n: int) -> dict:
    return doc.entity(UIDS[n - 1]).entity.to_json()


def remove(doc, n, **kw):
    place = cad.SubPlace.from_json({"kind": "vertex", "part": 0, "ring": 0, "index": 1})
    return cad.entities.vertex(doc, uid=UIDS[n - 1], action="remove", place=place, **kw)


def centre(a, b, bulge):
    """The circle's centre of an arc edge: on the chord's perpendicular at R − sagitta from the chord's middle."""
    (ax, ay), (bx, by) = a, b
    half = math.dist(a, b) / 2
    s = half * bulge
    r = (half * half + s * s) / (2 * abs(s))
    ux, uy = (bx - ax) / (2 * half), (by - ay) / (2 * half)
    off = (r - abs(s)) * (1 if bulge > 0 else -1)
    return ((ax + bx) / 2 - uy * off, (ay + by) / 2 + ux * off)


class VertexJoinArcs(unittest.TestCase):
    def test_two_arcs_of_one_circle_become_one(self) -> None:
        doc = drawing()
        out = remove(doc, 19, join_arcs=0.001)
        self.assertEqual(list(out.ids), [19])
        e = shape(doc, 19)
        self.assertEqual(len(e["pts"]), 2)
        self.assertAlmostEqual(e["bulges"][0], WANT, places=12)
        a, b = [(p["x"] - vc.X, p["y"] - vc.Y) for p in e["pts"]]
        c = centre(a, b, e["bulges"][0])
        self.assertLess(math.dist(c, (20000, 0)), 1e-6, c)

    def test_the_neighbour_along_the_same_arcs_takes_it_too(self) -> None:
        doc = drawing()
        out = remove(doc, 19, join_arcs=0.001, topology=cad.StretchTopology())
        self.assertEqual(list(out.ids), [19, 23])
        self.assertAlmostEqual(shape(doc, 23)["bulges"][0], -WANT, places=12)

    def test_without_the_tolerance_the_edge_is_straight(self) -> None:
        doc = drawing()
        remove(doc, 19)
        self.assertFalse(any(shape(doc, 19).get("bulges", [])))

    def test_arcs_of_two_circles_stay_apart(self) -> None:
        doc = drawing()
        remove(doc, 20, join_arcs=0.001)
        self.assertFalse(any(shape(doc, 20).get("bulges", [])))


if __name__ == "__main__":
    unittest.main()
