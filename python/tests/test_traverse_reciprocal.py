"""A traverse's reciprocal zenith angles (`cad.survey.points`' Poligon hesabı `heights.back`, docs/adr/4500;
plan/map-cad.md MAP-02), held by the test's own computation (no KentOS code in the expectations).

The drawing is the shared cases' (fixtures/commands/v1/cad.survey.points.json). The test lays out its own open
traverse, chooses its own heights and makes the zenith angles both ways that an air of k = 0.2 would give over them
(one way: S·cot z + i − t + (1 − k)·S²/(2R), R 6 371 000 m), each back sight between the same two points as its forward
one (the far end's instrument at the forward target's height, the station's target at the forward instrument's):

- every leg sighted back, the points get the test's heights, whatever k the project has (0.13 or 0.25): the mean of the
  two one-way differences leaves k out;
- each pair's refraction coefficient is Baselga, García-Asenjo and Garrigues' (J. Surv. Eng. 2014, eq. 11),
  ½·sin z + ½·sin z′ + (2R/s)·cos((z + z′)/2) with s = S / sin z, worked out here, and close to the air's 0.2;
- a leg left one way takes the project's k (its point misses the height by the difference of the two k's terms) and has
  no pair's k.
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
from kentos.cad.types import SurveyWork  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.survey.points.json").read_text("utf-8"))
X, Y = 487000, 4420000
R = 6371000.0
AIR = 0.2
A, BACK = (0.0, 0.0), (0.0, 100.0)
PATH = [A, (60.0, 140.0), (180.0, 220.0), (330.0, 250.0)]
HEIGHTS = [500.0, 503.25, 501.8, 506.4]
INST, TGT = [1.55, 1.48, 1.62], [1.7, 1.35, 1.58]


def bearing(a, b):
    return math.atan2(b[0] - a[0], b[1] - a[1]) % (2 * math.pi)


def grad(r):
    return r * 200.0 / math.pi


def rad(g):
    return g * math.pi / 200.0


def readings():
    angles, lengths, prev = [], [], BACK
    for i in range(len(PATH) - 1):
        angles.append(grad(bearing(PATH[i], PATH[i + 1]) - bearing(PATH[i], prev)) % 400.0)
        lengths.append(math.hypot(PATH[i + 1][0] - PATH[i][0], PATH[i + 1][1] - PATH[i][1]))
        prev = PATH[i]
    return angles, lengths


def sights(lengths):
    """The zenith angles (grad) both ways an air of k AIR gives, each back sight on the same two points."""
    fore, back = [], []
    for i, s in enumerate(lengths):
        c = (1 - AIR) * s * s / (2 * R)
        rise = HEIGHTS[i + 1] - HEIGHTS[i]
        fore.append(grad(math.atan2(1.0, (rise - INST[i] + TGT[i] - c) / s)))
        back.append(grad(math.atan2(1.0, (-rise - TGT[i] + INST[i] - c) / s)))
    return fore, back


def opened(k: float) -> cad.Document:
    setup = copy.deepcopy(dict(CASES["setup"], version=2))
    setup["uids"] = [f"0192f5c0-4500-7000-8000-{i + 1:012x}" for i in range(len(setup["entities"]))]
    setup["settings"]["survey"] = dict(setup["settings"].get("survey") or {}, refraction=k)
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def at(p):
    return {"kind": "at", "at": {"x": X + p[0], "y": Y + p[1]}}


class TraverseReciprocal(unittest.TestCase):
    def run_traverse(self, k: float, back: list) -> tuple[list[float], dict]:
        angles, lengths = readings()
        fore, _ = sights(lengths)
        doc = opened(k)
        work = SurveyWork.from_json({"kind": "traverse", "shape": "open", "start": at(A), "back": at(BACK),
                                     "angles": angles, "distances": lengths, "names": ["Y1", "Y2", "Y3"],
                                     "heights": {"start": HEIGHTS[0], "zeniths": fore, "instruments": INST,
                                                 "targets": TGT, "back": back}})
        out = cad.survey.points(doc, layer_id="yapi", work=work, unit="grad")
        return [doc.entity(u).entity.to_json()["z"] for u in out.uids], out.to_json()

    def test_sighted_back_the_heights_are_the_paths_whatever_k(self) -> None:
        _, lengths = readings()
        fore, back = sights(lengths)
        every = [{"zenith": z, "instrument": t, "target": i} for z, i, t in zip(back, INST, TGT)]
        for k in (0.13, 0.25):
            heights, out = self.run_traverse(k, every)
            for got, want in zip(heights, HEIGHTS[1:]):
                self.assertAlmostEqual(got, want, delta=1e-9)
            for got, z, zb, s in zip(out["refractions"], fore, back, lengths):
                want = (math.sin(rad(z)) / 2 + math.sin(rad(zb)) / 2
                        + 2 * R / (s / math.sin(rad(z))) * math.cos((rad(z) + rad(zb)) / 2))
                self.assertAlmostEqual(got, want, delta=1e-9)
                self.assertAlmostEqual(got, AIR, delta=2e-3)

    def test_a_leg_left_one_way_takes_the_project_s_k(self) -> None:
        _, lengths = readings()
        _, back = sights(lengths)
        some = [{"zenith": back[0], "instrument": TGT[0], "target": INST[0]}, None,
                {"zenith": back[2], "instrument": TGT[2], "target": INST[2]}]
        heights, out = self.run_traverse(0.13, some)
        s = lengths[1]
        miss = (AIR - 0.13) * s * s / (2 * R)  # (1 − 0.13)·S²/2R − (1 − AIR)·S²/2R
        self.assertAlmostEqual(heights[0], HEIGHTS[1], delta=1e-9)
        self.assertAlmostEqual(heights[1] - heights[0], HEIGHTS[2] - HEIGHTS[1] + miss, delta=1e-9)
        self.assertIsNone(out["refractions"][1])
        self.assertIsNotNone(out["refractions"][0])


if __name__ == "__main__":
    unittest.main()
