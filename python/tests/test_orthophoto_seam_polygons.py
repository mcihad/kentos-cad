"""The seam measured along the two images' seamline polygons' shared boundary (`cad.orthophoto.compare`'s
`seam.polygons`, docs/adr/4460; plan/map-cad.md MAP-12), held by this test's own readings.

The drawing is scripts/fixtures/orthophoto_compare_cases.py's: two synthetic GeoTIFFs overlapping 10 m, and the first
image's seamline polygon west of a bent seam line with the second's east of it (a mosaic's: one polygon an image, the
part of it used, no overlap).

- The shared boundary is the edges the two rings have in common (their vertices are the same points here), read as
  such by this test: the bent line, in the first polygon's ring order; asked the other way round, the second's.
- Measured along the polygons, every place is the place measured along a polyline drawn on that boundary (the test
  adds it to the drawing): the same points, shifts and correlations, each with its part.
- The second polygon pulled back east between points of the line's edges: the shared runs read by sampling the first's
  boundary every centimetre and keeping the samples within 1 mm of the second's (KentOS's on-a-boundary tolerance);
  two runs, their ends and lengths within a centimetre.
- 2 mm apart, they share nothing: refused at `seam.polygons`.
"""

from __future__ import annotations

import copy
import json
import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402
from kentos.cad.types import OrthoSeamAsk  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.orthophoto.compare.json").read_text("utf-8"))


def case(start: str) -> dict:
    return next(c for c in CASES["cases"] if c["name"].startswith(start))


def setup_of(start: str) -> dict:
    return json.loads(json.dumps(case(start)["setup"]).replace("$fixture:", str(ROOT / "fixtures") + "/"))


def opened(setup: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def uid(n: int) -> str:
    return f"0192f5c0-1780-7000-8000-{n:012x}"


def ring(setup: dict, n: int) -> list[tuple[float, float]]:
    e = setup["entities"][n - 1]
    return [(p["x"], p["y"]) for p in e["pts"]]


def ask(**over) -> OrthoSeamAsk:
    return OrthoSeamAsk(step=1.0, window=21, search=5, correlation=0.8, **over)


def compare(doc: cad.Document, seam: OrthoSeamAsk):
    return cad.orthophoto.compare.plan(doc, first=uid(1), second=uid(2), first_points="okuma-2024",
                                       second_points="okuma-2026", seam=seam).seam


def common_edges(a: list, b: list) -> list:
    """The edges of ring `a` that ring `b` has too (either way round), chained in `a`'s order."""
    theirs = {frozenset((b[k], b[(k + 1) % len(b)])) for k in range(len(b))}
    runs: list = []
    for k in range(len(a)):
        p, q = a[k], a[(k + 1) % len(a)]
        if frozenset((p, q)) not in theirs:
            continue
        if runs and runs[-1][-1] == p:
            runs[-1].append(q)
        else:
            runs.append([p, q])
    return runs


def near_boundary(v, ring_: list, tol: float) -> bool:
    for k in range(len(ring_)):
        (ax, ay), (bx, by) = ring_[k], ring_[(k + 1) % len(ring_)]
        dx, dy = bx - ax, by - ay
        t = max(0.0, min(1.0, ((v[0] - ax) * dx + (v[1] - ay) * dy) / (dx * dx + dy * dy)))
        if math.hypot(v[0] - (ax + t * dx), v[1] - (ay + t * dy)) <= tol:
            return True
    return False


def sampled_runs(a: list, b: list, step: float = 0.01, tol: float = 1e-3) -> list:
    """The runs of `a`'s boundary within `tol` of `b`'s, read by samples every `step` along `a`."""
    runs: list = []
    inside = False
    for k in range(len(a)):
        p, q = a[k], a[(k + 1) % len(a)]
        length = math.hypot(q[0] - p[0], q[1] - p[1])
        for i in range(int(length / step) + 1):
            f = min(1.0, i * step / length)
            v = (p[0] + f * (q[0] - p[0]), p[1] + f * (q[1] - p[1]))
            if near_boundary(v, b, tol):
                if not inside:
                    runs.append([v, v, 0.0])
                else:
                    runs[-1][2] += math.hypot(v[0] - runs[-1][1][0], v[1] - runs[-1][1][1])
                    runs[-1][1] = v
                inside = True
            else:
                inside = False
    return [(r[0], r[1], r[2]) for r in runs if r[2] > tol]


class SeamPolygons(unittest.TestCase):
    def test_the_shared_boundary_is_the_common_edges_and_its_places_a_drawn_line_s(self) -> None:
        setup = setup_of("dikiş çokgenlerinden")
        doc = opened(setup)
        got = compare(doc, ask(polygons=[uid(3), uid(4)]))
        want = common_edges(ring(setup, 3), ring(setup, 4))
        self.assertEqual([[tuple(v) for v in part] for part in got.shared.parts], want)
        # The same boundary drawn as a polyline: the same places, each with its part.
        drawn = copy.deepcopy(setup)
        drawn["entities"].append({"kind": "polyline", "layerId": "0", "attrs": {}, "id": 5,
                                  "pts": [{"x": x, "y": y} for x, y in want[0]]})
        drawn["uids"].append(uid(5))
        along = compare(opened(drawn), ask(line=uid(5)))
        self.assertEqual(len(got.patches), len(along.patches))
        for p, q in zip(got.patches, along.patches):
            self.assertEqual(p.part, 0)
            self.assertEqual((p.at, p.station, p.state), (q.at, q.station, q.state))
            self.assertEqual((p.dx, p.dy, p.correlation), (q.dx, q.dy, q.correlation))
        self.assertEqual((got.shared.places, got.shared.outside), (along.line.places, along.line.outside))
        # The other way round: the second's ring order.
        back = compare(doc, ask(polygons=[uid(4), uid(3)]))
        self.assertEqual([[tuple(v) for v in part] for part in back.shared.parts],
                         common_edges(ring(setup, 4), ring(setup, 3)))
        self.assertAlmostEqual(back.shared.length, got.shared.length, delta=1e-9)

    def test_two_runs_read_by_sampling(self) -> None:
        setup = setup_of("dikiş çokgenlerinin iki ortak parçası")
        got = compare(opened(setup), ask(polygons=[uid(3), uid(4)]))
        runs = sampled_runs(ring(setup, 3), ring(setup, 4))
        self.assertEqual(len(got.shared.parts), len(runs), runs)
        for part, (start, end, length) in zip(got.shared.parts, runs):
            self.assertLess(math.dist(part[0], start), 0.01)
            self.assertLess(math.dist(part[-1], end), 0.01)
            mine = sum(math.dist(a, b) for a, b in zip(part, part[1:]))
            self.assertAlmostEqual(mine, length, delta=0.01)
        self.assertEqual({p.part for p in got.patches}, {0, 1})

    def test_two_millimetres_apart_share_nothing(self) -> None:
        setup = setup_of("dikiş çokgenlerinden")
        moved = copy.deepcopy(setup)
        for v in moved["entities"][3]["pts"]:
            v["x"] += 0.002
        with self.assertRaises(CommandFailed) as refused:
            compare(opened(moved), ask(polygons=[uid(3), uid(4)]))
        self.assertEqual((refused.exception.code, refused.exception.path), ("invalid_input", "seam.polygons"))


if __name__ == "__main__":
    unittest.main()
