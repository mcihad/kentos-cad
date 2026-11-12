"""Robust estimation of a GNSS network (`cad.survey.baselines` with `robust`, docs/adr/4960; plan/map-cad.md MAP-05),
held by the test's own iteratively reweighted least squares with numpy.

The network is the test's: an 8 × 8 grid of stations 1 km apart (64 stations, the first fixed), each joined to its
east, north and north-east neighbours by a baseline whose vector is the true difference plus a few millimetres and
whose covariance is correlated; one baseline carries a 9 cm blunder upwards. By the test's rule:

- each round the baselines' covariances divided by their factor, Aᵀ P A x = Aᵀ P l solved;
- a baseline's standardised residual the largest of its components' |v| / σ (σ from the given covariance);
- the next factor Huber c / |w| (c 1.5) or Danish exp(−((|w| − c) / c)²) (c 2.0) when |w| > c, else 1;
- the rounds stop when no factor moves by more than 1e-9; the answer is the round whose factors are reported.

The factors agree within 1e-9, the coordinates within a micrometre; no baseline is removed; the blunder's factor is
the smallest and Danish weighs it down further than Huber.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "python"))

from kentos import cad  # noqa: E402

W = 8
BASE = np.array([4_121_000.0, 2_652_000.0, 4_069_000.0])
EAST, NORTH, UP = np.array([-0.54, 0.84, 0.0]), np.array([-0.53, -0.34, 0.78]), np.array([0.65, 0.42, 0.63])
COV = np.array([[9e-6, 2.7e-6, 0.0], [2.7e-6, 9e-6, 0.0], [0.0, 0.0, 36e-6]])
BLUNDER = 40


def at(r: int, c: int) -> np.ndarray:
    return BASE + 1000.0 * (c * EAST + r * NORTH)


def network() -> tuple[list[dict], list[dict]]:
    stations = [{"name": "S0", "xyz": list(at(0, 0)), "fixed": True}] + [{"name": f"S{i}"} for i in range(1, W * W)]
    baselines = []
    for r in range(W):
        for c in range(W):
            for dr, dc in ((0, 1), (1, 0), (1, 1)):
                if r + dr < W and c + dc < W:
                    k = len(baselines)
                    noise = np.array([0.003 * math.sin(k * 0.7), 0.003 * math.cos(k * 1.3), 0.006 * math.sin(k * 0.9)])
                    if k == BLUNDER:
                        noise = noise + 0.09 * UP
                    d = at(r + dr, c + dc) - at(r, c) + noise
                    baselines.append({"from": f"S{r * W + c}", "to": f"S{(r + dr) * W + c + dc}", "d": list(d),
                                      "cov": [COV[0, 0], COV[0, 1], COV[0, 2], COV[1, 1], COV[1, 2], COV[2, 2]]})
    return stations, baselines


def irls(stations: list[dict], baselines: list[dict], method: str, c: float) -> tuple[list[float], dict]:
    """The test's own robust adjustment: the factors and the coordinates."""
    index = {s["name"]: i for i, s in enumerate(stations)}
    approx = {0: np.array(stations[0]["xyz"])}
    grew = True
    while grew:
        grew = False
        for b in baselines:
            f, t = index[b["from"]], index[b["to"]]
            if f in approx and t not in approx:
                approx[t] = approx[f] + np.array(b["d"]); grew = True
            elif t in approx and f not in approx:
                approx[f] = approx[t] - np.array(b["d"]); grew = True
    slot = {i: 3 * (i - 1) for i in range(1, len(stations))}
    u = 3 * (len(stations) - 1)
    rows, rhs = [], []
    for b in baselines:
        f, t = index[b["from"]], index[b["to"]]
        a = np.zeros((3, u))
        if t in slot:
            a[:, slot[t]:slot[t] + 3] = np.eye(3)
        if f in slot:
            a[:, slot[f]:slot[f] + 3] = -np.eye(3)
        rows.append(a)
        rhs.append(np.array(b["d"]) - (approx[t] - approx[f]))
    p0 = np.linalg.inv(COV)
    sigma = np.sqrt(np.diag(COV))

    def weight(w: float) -> float:
        if w <= c:
            return 1.0
        return c / w if method == "huber" else math.exp(-(((w - c) / c) ** 2))

    factors = [1.0] * len(baselines)
    for _ in range(100):
        n_mat = np.zeros((u, u))
        b_vec = np.zeros(u)
        for a, l, k in zip(rows, rhs, factors):
            n_mat += a.T @ (p0 * k) @ a
            b_vec += a.T @ (p0 * k) @ l
        x = np.linalg.solve(n_mat, b_vec)
        nxt = [weight(float(np.max(np.abs(a @ x - l) / sigma))) for a, l in zip(rows, rhs)]
        moved = max(abs(p - q) for p, q in zip(nxt, factors))
        if moved <= 1e-9:
            break
        factors = nxt
    coords = {s["name"]: approx[i] + (x[slot[i]:slot[i] + 3] if i in slot else 0.0) for i, s in enumerate(stations)}
    return factors, coords


class GnssRobust(unittest.TestCase):
    def run_robust(self, method: str) -> list[float]:
        stations, baselines = network()
        want_factors, want_xyz = irls(stations, baselines, method, 1.5 if method == "huber" else 2.0)
        doc = cad.Document.new(workspace="cad", coords="local")
        command = cad.command("cad.survey.baselines")
        data = {"stations": stations, "baselines": baselines, "robust": {"method": method}}
        answer = command.run(doc, command.input_type.from_json(data), op="plan")
        self.assertIsNone(answer.error, answer.error and answer.error.message)
        got = answer.output.to_json()
        self.assertTrue(got["robust"]["converged"])
        self.assertEqual(len(got["baselines"]), len(baselines), "nothing removed")
        factors = [b["factor"] for b in got["baselines"]]
        for k, (g, w) in enumerate(zip(factors, want_factors)):
            self.assertAlmostEqual(g, w, delta=1e-9, msg=(method, k))
        for s in got["stations"]:
            for c in range(3):
                self.assertAlmostEqual(s["xyz"][c], want_xyz[s["name"]][c], delta=1e-6, msg=(method, s["name"], c))
        self.assertEqual(min(range(len(factors)), key=factors.__getitem__), BLUNDER)
        self.assertEqual(got["robust"]["downweighted"], sum(1 for f in factors if f < 1.0))
        return factors

    def test_huber_and_danish_weigh_the_blunder_down(self) -> None:
        huber = self.run_robust("huber")
        danish = self.run_robust("danish")
        self.assertLess(danish[BLUNDER], huber[BLUNDER])


if __name__ == "__main__":
    unittest.main()
