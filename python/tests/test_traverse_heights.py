"""A traverse's heights by trigonometric heighting (`cad.survey.points`' Poligon hesabı, docs/adr/4100; plan/map-cad.md
MAP-02), held by the test's own computation (no KentOS code in the expectations).

The drawing is the shared cases' (fixtures/commands/v1/cad.survey.points.json: the known points A, R, E, F). The test
lays its own traverses out and reads them as an instrument would: the angles from its own bearings, the lengths, and
zenith angles of its own choosing with instrument and target heights. Each leg's height difference is then, by the
rule written in the test, S·cot z + i − t + (1 − k)·S²/(2R), k the project's 0.13 and R 6 371 000 m:

- an open traverse's points get the start's height plus the differences, nothing adjusted;
- a closed traverse back on its start takes the misclosure off the legs in proportion to their lengths; the answer says
  the misclosure and the last point closes on the start;
- each point's `Z (m)` is its height to three decimals.
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
from kentos.cad.types import SurveyWork  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.survey.points.json").read_text("utf-8"))
X, Y = 487000, 4420000
K, R = 0.13, 6371000.0
A, BACK = (0.0, 0.0), (0.0, 100.0)


def bearing(a, b):
    return math.atan2(b[0] - a[0], b[1] - a[1]) % (2 * math.pi)


def grad(r):
    return r * 200.0 / math.pi


def readings(path, fore=None):
    """The angles (grad, clockwise from the line back to the line ahead) and the lengths along `path`."""
    angles, lengths, prev = [], [], BACK
    for i in range(len(path) - 1):
        angles.append(grad(bearing(path[i], path[i + 1]) - bearing(path[i], prev)) % 400.0)
        lengths.append(math.hypot(path[i + 1][0] - path[i][0], path[i + 1][1] - path[i][1]))
        prev = path[i]
    if fore is not None:
        angles.append(grad(bearing(path[-1], fore) - bearing(path[-1], path[-2])) % 400.0)
    return angles, lengths


def difference(s, z_grad, i, t):
    z = z_grad * math.pi / 200.0
    return s * math.cos(z) / math.sin(z) + i - t + (1 - K) * s * s / (2 * R)


def opened() -> cad.Document:
    # The cases' setup is a version 1 snapshot: as version 2 it takes persistent ids.
    setup = dict(CASES["setup"], version=2)
    setup["uids"] = [f"0192f5c0-4100-7000-8000-{i + 1:012x}" for i in range(len(setup["entities"]))]
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def at(p):
    return {"kind": "at", "at": {"x": X + p[0], "y": Y + p[1]}}


class TraverseHeights(unittest.TestCase):
    def heights_of(self, doc, out) -> list[float]:
        return [doc.entity(u).entity.to_json()["z"] for u in out.uids]

    def test_an_open_traverse_adds_its_differences(self) -> None:
        path = [A, (40.0, 130.0), (-35.0, 290.0)]
        angles, lengths = readings(path)
        zeniths, inst, tgt = [97.25, 101.75], [1.52, 1.47], [1.85, 1.2]
        doc = opened()
        work = SurveyWork.from_json({"kind": "traverse", "shape": "open", "start": at(A), "back": at(BACK),
                                     "angles": angles, "distances": lengths, "names": ["Y1", "Y2"],
                                     "heights": {"start": 812.5, "zeniths": zeniths, "instruments": inst,
                                                 "targets": tgt}})
        out = cad.survey.points(doc, layer_id="yapi", work=work, unit="grad")
        want, h = [], 812.5
        for s, z, i, t in zip(lengths, zeniths, inst, tgt):
            h += difference(s, z, i, t)
            want.append(h)
        for got, w in zip(self.heights_of(doc, out), want):
            self.assertAlmostEqual(got, w, delta=1e-9)
        self.assertIsNone(out.to_json().get("heightMisclosure"))
        attrs = doc.entity(out.uids[0]).entity.to_json()["attrs"]
        self.assertEqual(attrs["Z (m)"], f"{want[0]:.3f}")

    def test_a_closed_traverse_takes_its_misclosure_off_by_the_lengths(self) -> None:
        path = [A, (120.0, -40.0), (190.0, 85.0), (60.0, 140.0), A]
        angles, lengths = readings(path, fore=BACK)
        zeniths, inst, tgt = [99.4, 100.9, 101.3, 98.7], [1.5, 1.6, 1.55, 1.45], [1.5, 1.6, 1.55, 1.45]
        doc = opened()
        work = SurveyWork.from_json({"kind": "traverse", "shape": "closed", "start": at(A), "back": at(BACK),
                                     "angles": angles, "distances": lengths, "names": ["K1", "K2", "K3"],
                                     "heights": {"start": 40.0, "zeniths": zeniths, "instruments": inst,
                                                 "targets": tgt}})
        out = cad.survey.points(doc, layer_id="yapi", work=work, unit="grad")
        raw = [difference(s, z, i, t) for s, z, i, t in zip(lengths, zeniths, inst, tgt)]
        f = sum(raw)
        self.assertAlmostEqual(out.to_json()["heightMisclosure"], f, delta=1e-9)
        want, h = [], 40.0
        for d, s in zip(raw, lengths):
            h += d - f * s / sum(lengths)
            want.append(h)
        self.assertAlmostEqual(want[-1], 40.0, delta=1e-9, msg="back on the start")
        for got, w in zip(self.heights_of(doc, out), want[:-1]):
            self.assertAlmostEqual(got, w, delta=1e-9)


if __name__ == "__main__":
    unittest.main()
