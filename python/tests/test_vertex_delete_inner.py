"""Köşe sil with Topolojik düzenleme (`cad.entities.vertex` remove, docs/adr/4200; plan/map-cad.md MAP-07): what lay
inside the two edges about the lost corner goes to the same share of the edge joining them, held by the test's own
computation (no KentOS code in the expectations).

The drawing is scripts/fixtures/topology_inner_cases.py's scene. The road R runs (0, −40), (40, −40), (40, −60); the
branch T2 ends on its first edge at (20, −40). Removing R's middle vertex joins (0, −40) and (40, −60); the old edges
are 40 m and 20 m long, so T2's end, 20 m along them, goes to 20/60 of the joined edge: (40/3, −40 − 20/3). A's
south-east corner removed joins (0, 0) and (40, 30) (40 m and 30 m): the road's end at (10, 0) goes to 10/70 of it. An
open path's first vertex removed joins nothing: T2 stays. Without Topolojik düzenleme nothing follows.
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
import topology_inner_cases as tc  # noqa: E402

UIDS = [f"0192f5c0-4200-7000-8000-{i:012x}" for i in range(1, len(tc.ENTITIES) + 1)]


def drawing() -> cad.Document:
    setup = dict(tc.SETUP, version=2, uids=UIDS)
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def places(doc: cad.Document, n: int) -> list[tuple[float, float]]:
    e = doc.entity(UIDS[n - 1]).entity.to_json()
    pts = e.get("pts") or [e["a"], e["b"]]
    return [(p["x"] - tc.X, p["y"] - tc.Y) for p in pts]


def has(pts, p) -> bool:
    return any(math.dist(q, p) < 1e-6 for q in pts)


def remove(doc, n, index, topology=True):
    place = cad.SubPlace.from_json({"kind": "vertex", "part": 0, "ring": 0, "index": index})
    kw = {"topology": cad.StretchTopology()} if topology else {}
    return cad.entities.vertex(doc, uid=UIDS[n - 1], action="remove", place=place, **kw)


def share(p, q, l1, l2, along):
    """The place `along` metres along two edges of l1 and l2 metres, on the edge p–q joining their ends."""
    f = along / (l1 + l2)
    return (p[0] + (q[0] - p[0]) * f, p[1] + (q[1] - p[1]) * f)


class VertexDeleteInner(unittest.TestCase):
    def test_the_branch_s_end_goes_to_its_share_of_the_joined_edge(self) -> None:
        doc = drawing()
        out = remove(doc, 15, 1)
        self.assertEqual(list(out.ids), [15, 16])
        self.assertEqual(len(places(doc, 15)), 2)
        want = share((0, -40), (40, -60), 40, 20, 20)
        self.assertTrue(has(places(doc, 16), want), (places(doc, 16), want))

    def test_a_corner_of_an_area_too(self) -> None:
        doc = drawing()
        remove(doc, 1, 1)
        want = share((0, 0), (40, 30), 40, 30, 10)
        self.assertTrue(has(places(doc, 4), want), (places(doc, 4), want))

    def test_an_open_path_s_end_joins_nothing(self) -> None:
        doc = drawing()
        out = remove(doc, 15, 0)
        self.assertEqual(list(out.ids), [15])
        self.assertTrue(has(places(doc, 16), (20, -40)))

    def test_without_topology_nothing_follows(self) -> None:
        doc = drawing()
        out = remove(doc, 15, 1, topology=False)
        self.assertEqual(list(out.ids), [15])
        self.assertTrue(has(places(doc, 16), (20, -40)))


if __name__ == "__main__":
    unittest.main()
