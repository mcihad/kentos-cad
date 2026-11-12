"""Tarama ağı dengelemesi (`cad.pointclouds.network`, docs/adr/4720; plan/map-cad.md MAP-13), held by this test's own
readings.

The drawing is scripts/fixtures/pointcloud_network_cases.py's: pieces of a slope with a mound, a ridge and a roof,
each scan moved by its own known rotation and translation (the truth).

- With Huber weights each moved scan's adjusted matrix takes its moved ground back within a centimetre (the truth read
  here, through the scan's own misalignment and back).
- Round the loop the residuals cancel its misclosure: r_ab + r_bc − r_ac = −(m_ab + m_bc − m_ac), so their translation's
  length is the loop's.
- Along a chain without loops the corrections add: x_c = m_ab + m_bc, and every residual is zero.
- Two scans are refused at `uids`.
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
from kentos.cad.types import RegisterRobust  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import pointcloud_network_cases as nc  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.pointclouds.network.json").read_text("utf-8"))


def opened() -> cad.Document:
    setup = json.loads(json.dumps(CASES["setup"]).replace("$fixture:", str(ROOT / "fixtures") + "/"))
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def uid(name: str) -> str:
    return nc.uid(nc.IDS[name])


A, B, C = "ag-a.xyz", "ag-b.xyz", "ag-c.xyz"
HUBER = RegisterRobust(kind="huber")


class Network(unittest.TestCase):
    def test_huber_takes_each_moved_scan_back_within_a_centimetre(self) -> None:
        got = cad.pointclouds.network.plan(opened(), uids=[uid(A), uid(B), uid(C)], fixed=[uid(A)], robust=HUBER)
        by = {s.uid: s for s in got.scans}
        for name in (B, C):
            _, area, (w, t) = nc.SCANS[name]
            x, y = nc.X0 + (area[0] + area[1]) / 2.0, nc.Y0 + (area[2] + area[3]) / 2.0
            p = [x, y, nc.rc.ground(x - nc.X0, y - nc.Y0)]
            r = nc.rc.rot(w)
            d = [p[i] - nc.rc.MID[i] for i in range(3)]
            q = [sum(r[i][j] * d[j] for j in range(3)) + nc.rc.MID[i] + t[i] for i in range(3)]
            m = by[uid(name)].matrix
            back = [m[4 * i] * q[0] + m[4 * i + 1] * q[1] + m[4 * i + 2] * q[2] + m[4 * i + 3] for i in range(3)]
            self.assertLess(math.dist(back, p), 0.01, name)

    def test_the_loops_residuals_cancel_its_misclosure(self) -> None:
        got = cad.pointclouds.network.plan(opened(), uids=[uid(A), uid(B), uid(C)], fixed=[uid(A)], robust=HUBER)
        links = {tuple(l.scans): l for l in got.links}
        ab, ac, bc = links[(uid(A), uid(B))], links[(uid(A), uid(C))], links[(uid(B), uid(C))]
        r = [ab.residual[i] + bc.residual[i] - ac.residual[i] for i in range(6)]
        w = [ab.observed[i] + bc.observed[i] - ac.observed[i] for i in range(6)]
        for i in range(6):
            self.assertAlmostEqual(r[i], -w[i], delta=1e-9)
        self.assertEqual(len(got.loops), 1)
        self.assertAlmostEqual(math.hypot(*r[3:]), got.loops[0].translation, delta=1e-9)
        self.assertAlmostEqual(math.hypot(*r[:3]), got.loops[0].rotation, delta=1e-12)

    def test_along_a_chain_the_corrections_add(self) -> None:
        got = cad.pointclouds.network.plan(opened(), uids=[uid(A), uid(B), uid(C)], fixed=[uid(A)], robust=HUBER,
                                           links=[[uid(A), uid(B)], [uid(B), uid(C)]])
        self.assertEqual(got.loops, [])
        ab, bc = got.links
        c = next(s for s in got.scans if s.uid == uid(C))
        mine = [ab.observed[i] + bc.observed[i] for i in range(6)]
        theirs = list(c.rotation) + list(c.translation)
        for i in range(6):
            self.assertAlmostEqual(theirs[i], mine[i], delta=1e-9)
        self.assertTrue(all(abs(v) < 1e-9 for l in got.links for v in l.residual))

    def test_the_outer_loop_is_the_rounds_maps_together(self) -> None:
        """docs/adr/5100: five rounds; the first round's matrices are one round's answer, the rounds' moves fall, and
        each scan's matrix takes the centre as far as its translation says (this test's own arithmetic)."""
        one = cad.pointclouds.network.plan(opened(), uids=[uid(A), uid(B), uid(C)], fixed=[uid(A)], robust=HUBER)
        got = cad.pointclouds.network.plan(opened(), uids=[uid(A), uid(B), uid(C)], fixed=[uid(A)], robust=HUBER,
                                           rounds=5)
        moves = list(got.rounds)
        self.assertEqual(len(moves), 5)
        self.assertGreater(moves[0], 0.3)
        self.assertLess(moves[1], moves[0] / 50)
        self.assertFalse(one.rounds, "one round: no list")
        c = got.centre
        for s in got.scans:
            m = s.matrix
            moved = [m[4 * i] * c[0] + m[4 * i + 1] * c[1] + m[4 * i + 2] * c[2] + m[4 * i + 3] for i in range(3)]
            for i in range(3):
                self.assertAlmostEqual(moved[i] - c[i], s.translation[i], delta=1e-6)
        # The rounds after the first move a scan a few millimetres at most.
        before = {s.uid: s for s in one.scans}
        for s in got.scans:
            self.assertLess(math.dist(s.translation, before[s.uid].translation), 0.02, s.uid)

    def test_two_scans_are_refused(self) -> None:
        with self.assertRaises(CommandFailed) as refused:
            cad.pointclouds.network.plan(opened(), uids=[uid(A), uid(B)], fixed=[uid(A)])
        self.assertEqual((refused.exception.code, refused.exception.path), ("invalid_input", "uids"))


if __name__ == "__main__":
    unittest.main()
