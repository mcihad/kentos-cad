"""İzlemede dönemlerden doğru ve iki eşik (`cad.asbuilt.check`'s `trend` and `thresholds`; docs/adr/4840; plan/map-cad.md
MAP-19) through the SDK, held by this test's own arithmetic (numpy's polyfit, Student's t's closed form; no KentOS code
in the expectations).

On the shared cases' four-epoch drawing (scripts/fixtures/asbuilt_series_cases.py: four layers of marks, a day each):

- each mark measured in three epochs or more has a line: its slopes east, north and up are numpy.polyfit's of its
  movement from its first epoch against the years since that epoch, their σ s / √Sxx from the residuals; a component
  moves when |v| > t(n − 2; 0.975)·σ, Student's t in closed form (one degree of freedom tan(π(p − ½)), two
  (2p − 1) / √(2p(1 − p)));
- with thresholds each mark's state is the highest level its last movement or its fitted speed goes beyond.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
import warnings
from datetime import date
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.asbuilt.check.series.json").read_text("utf-8"))
CASE = next(c for c in CASES["cases"] if c["name"].startswith("doğru: dört dönem"))
SETUP, ASK = CASE["setup"], CASE["steps"][0]["input"]


def epochs() -> list[tuple[str, dict[str, tuple[float, float, float]]]]:
    """Each epoch's day and its marks by name (once), read from the drawing by this test, oldest first."""
    order = [e["layer"] for e in ASK["epochs"]] + [ASK["measured"]]
    days = [e["date"] for e in ASK["epochs"]] + [ASK["date"]]
    out = []
    for layer, day in zip(order, days):
        marks: dict[str, list] = {}
        for e in SETUP["entities"]:
            if e.get("layerId") == layer and e["kind"] == "point":
                marks.setdefault(e["attrs"]["Ad"], []).append((e["p"]["x"], e["p"]["y"], e["z"]))
        out.append((day, {k: v[0] for k, v in marks.items() if len(v) == 1}))
    return out


def line(t: list[float], d: list[float]) -> tuple[float, float]:
    slope, a = np.polyfit(t, d, 1)
    r = np.array(d) - (a + slope * np.array(t))
    sxx = float(np.sum((np.array(t) - np.mean(t)) ** 2))
    return float(slope), math.sqrt(float(np.sum(r * r)) / (len(t) - 2) / sxx)


class AsbuiltTrend(unittest.TestCase):
    def setUp(self) -> None:
        warnings.simplefilter("ignore", cad.CommandWarning)
        self.doc = cad.Document.from_bytes(kr.container(kr.document(SETUP)))

    def run_check(self, **more):
        ask = cad.AsbuiltCheck.from_json({**ASK, **more})
        out = cad.command("cad.asbuilt.check").run(self.doc, ask, op="plan")
        self.assertIsNone(out.error, out.error and out.error.message)
        return out.output

    def test_each_line_is_numpys(self) -> None:
        out = self.run_check()
        series = epochs()
        p = 0.975
        t_crit = {1: math.tan(math.pi * (p - 0.5)), 2: (2 * p - 1) / math.sqrt(2 * p * (1 - p))}
        checked = {3: 0, 4: 0}
        for track in out.tracks:
            seen = [(date.fromisoformat(day), marks[track.name]) for day, marks in series if track.name in marks]
            if len(seen) < 3:
                self.assertIs(track.fit, cad.UNSET, track.name)
                continue
            first_day, ref = seen[0]
            t = [(d - first_day).days / 365.25 for d, _ in seen]
            fit = track.fit
            self.assertEqual(fit.epochs, len(seen), track.name)
            for k, (key, sigma_key) in enumerate((("east", "sigma_east"), ("north", "sigma_north"), ("up", "sigma_up"))):
                v, s = line(t, [m[k] - ref[k] for _, m in seen])
                self.assertAlmostEqual(getattr(fit, key), v, delta=1e-9, msg=(track.name, key))
                self.assertAlmostEqual(getattr(fit, sigma_key), s, delta=1e-9, msg=(track.name, key))
                moving = list(fit.moving) if fit.moving is not cad.UNSET else []
                self.assertEqual(key in moving, abs(v) > t_crit[len(seen) - 2] * s, (track.name, key, v, s))
            self.assertAlmostEqual(fit.speed, math.hypot(fit.east, fit.north), places=12)
            checked[len(seen)] += 1
        self.assertGreaterEqual(checked[4], 5, checked)
        self.assertGreaterEqual(checked[3], 1, checked)

    def test_the_states_by_the_two_levels(self) -> None:
        warning = {"horizontal": 0.0075, "horizontalRate": 0.015}
        alarm = {"horizontal": 0.03, "horizontalRate": 0.06}
        out = self.run_check(thresholds={"warning": warning, "alarm": alarm})
        for track in out.tracks:
            last = track.steps[-1]
            speed = track.fit.speed if track.fit is not cad.UNSET else track.rate_horizontal
            values = {"horizontal": last.horizontal, "horizontalRate": speed}
            beyond = lambda level: [k for k in ("horizontal", "horizontalRate")  # noqa: E731
                                    if values[k] is not cad.UNSET and values[k] is not None and values[k] > level[k]]
            want = ("alarm", beyond(alarm)) if beyond(alarm) else ("warning", beyond(warning)) if beyond(warning) \
                else ("normal", [])
            exceeded = list(track.exceeded) if track.exceeded is not cad.UNSET else []
            self.assertEqual((track.state.value, exceeded), want, track.name)


if __name__ == "__main__":
    unittest.main()
