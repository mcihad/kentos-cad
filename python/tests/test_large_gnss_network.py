"""A GNSS network above the old dense limit (`cad.survey.baselines`, docs/adr/4560; plan/map-cad.md MAP-05), held by the
test's own solution: numpy's weighted least squares over the whole network.

The network is the test's: a 26 × 26 grid of stations 1 km apart near Ankara (676 stations; with the first fixed 2 025
unknowns, where the dense solve took at most 1 500), each joined to its east, north and north-east neighbours by a
baseline whose vector is the true difference plus a few millimetres and whose covariance is correlated; every
thirteenth station is weighted by 2 cm. By the test's rule (Gauss–Markov, the unknowns corrections to coordinates
carried along the baselines):

- the coordinates are the solution of Aᵀ P A x = Aᵀ P l (within a micrometre);
- the X, Y, Z standard deviations are m0·√Qᵢᵢ, m0 = √(vᵀPv / f) (within 10⁻⁹ m);
- n, u, f and vᵀPv are the test's.
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

W = 26
BASE = np.array([4_121_000.0, 2_652_000.0, 4_069_000.0])
EAST, NORTH = np.array([-0.54, 0.84, 0.0]), np.array([-0.53, -0.34, 0.78])
COV = np.array([[9e-6, 2.7e-6, 0.0], [2.7e-6, 9e-6, 0.0], [0.0, 0.0, 36e-6]])


def at(r: int, c: int) -> np.ndarray:
    return BASE + 1000.0 * (c * EAST + r * NORTH)


def network() -> tuple[list[dict], list[dict]]:
    stations = []
    for r in range(W):
        for c in range(W):
            i = r * W + c
            s = {"name": f"S{i}"}
            if i == 0:
                s.update(xyz=list(at(r, c)), fixed=True)
            elif i % 13 == 0:
                s.update(xyz=list(at(r, c)), sigma=[0.02, 0.02, 0.02])
            stations.append(s)
    baselines = []
    for r in range(W):
        for c in range(W):
            for dr, dc in ((0, 1), (1, 0), (1, 1)):
                if r + dr < W and c + dc < W:
                    k = len(baselines)
                    noise = np.array([0.003 * math.sin(k * 0.7), 0.003 * math.cos(k * 1.3), 0.006 * math.sin(k * 0.9)])
                    d = at(r + dr, c + dc) - at(r, c) + noise
                    baselines.append({"from": f"S{r * W + c}", "to": f"S{(r + dr) * W + c + dc}", "d": list(d),
                                      "cov": [COV[0, 0], COV[0, 1], COV[0, 2], COV[1, 1], COV[1, 2], COV[2, 2]]})
    return stations, baselines


def solved(stations: list[dict], baselines: list[dict]) -> dict:
    """The test's own adjustment: coordinates, standard deviations and counts."""
    names = [s["name"] for s in stations]
    index = {n: i for i, n in enumerate(names)}
    approx: dict[int, np.ndarray] = {i: np.array(s["xyz"]) for i, s in enumerate(stations) if "xyz" in s}
    grew = True
    while grew:
        grew = False
        for b in baselines:
            f, t = index[b["from"]], index[b["to"]]
            if f in approx and t not in approx:
                approx[t] = approx[f] + np.array(b["d"]); grew = True
            elif t in approx and f not in approx:
                approx[f] = approx[t] - np.array(b["d"]); grew = True
    slot, u = {}, 0
    for i, s in enumerate(stations):
        if not s.get("fixed"):
            slot[i] = u
            u += 3
    rows, rhs, weights = [], [], []
    for b in baselines:
        f, t = index[b["from"]], index[b["to"]]
        a = np.zeros((3, u))
        if t in slot:
            a[:, slot[t]:slot[t] + 3] = np.eye(3)
        if f in slot:
            a[:, slot[f]:slot[f] + 3] = -np.eye(3)
        rows.append(a)
        rhs.append(np.array(b["d"]) - (approx[t] - approx[f]))
        weights.append(np.linalg.inv(COV))
    for i, s in enumerate(stations):
        if "sigma" in s:
            a = np.zeros((3, u))
            a[:, slot[i]:slot[i] + 3] = np.eye(3)
            rows.append(a)
            rhs.append(np.array(s["xyz"]) - approx[i])
            weights.append(np.diag([1.0 / v ** 2 for v in s["sigma"]]))
    n_mat = np.zeros((u, u))
    b_vec = np.zeros(u)
    for a, l, p in zip(rows, rhs, weights):
        n_mat += a.T @ p @ a
        b_vec += a.T @ p @ l
    q = np.linalg.inv(n_mat)
    x = q @ b_vec
    omega = sum(float((a @ x - l) @ p @ (a @ x - l)) for a, l, p in zip(rows, rhs, weights))
    n_obs = 3 * len(rows)
    m0 = math.sqrt(omega / (n_obs - u))
    out = {}
    for i, name in enumerate(names):
        if i in slot:
            k = slot[i]
            out[name] = (approx[i] + x[k:k + 3], m0 * np.sqrt(np.diag(q)[k:k + 3]))
        else:
            out[name] = (approx[i], np.zeros(3))
    return {"stations": out, "n": n_obs, "u": u, "f": n_obs - u, "omega": omega}


class LargeGnssNetwork(unittest.TestCase):
    def test_a_network_above_the_old_limit_is_the_test_s_solution(self) -> None:
        stations, baselines = network()
        want = solved(stations, baselines)
        self.assertGreater(want["u"], 1500, "above the old dense limit")
        doc = cad.Document.new(workspace="cad", coords="local")
        command = cad.command("cad.survey.baselines")
        answer = command.run(doc, command.input_type.from_json({"stations": stations, "baselines": baselines}), op="plan")
        self.assertIsNone(answer.error, answer.error and answer.error.message)
        got = answer.output.to_json()
        self.assertEqual((got["n"], got["u"], got["f"]), (want["n"], want["u"], want["f"]))
        self.assertAlmostEqual(got["omega"], want["omega"], delta=1e-9 * want["omega"])
        for s in got["stations"]:
            xyz, sigma = want["stations"][s["name"]]
            for c in range(3):
                self.assertAlmostEqual(s["xyz"][c], xyz[c], delta=1e-6, msg=(s["name"], c))
                self.assertAlmostEqual(s["sxyz"][c], sigma[c], delta=1e-9, msg=(s["name"], "σ", c))


if __name__ == "__main__":
    unittest.main()
