"""Hat profilinde kesişen borunun dış duvarı (`cad.pipes.profile`'s `crossing.diameter.wall`; docs/adr/4360;
plan/map-cad.md MAP-16) through the SDK, held by this test's own geometry.

On the shared cases' walls drawing (the water network's pipes carry `Et`, the wall in millimetres), the sewer's run from
P1 with the water network's pipes crossing it. The profile's frame has its lower left at `at`, a metre along the run is
a unit across and a metre of height `exaggeration` units up from the datum. For each crossing pipe with an invert, a
diameter d and a wall w read from its own `Et`:

- the inner wall is an ellipse about (x, y) = (at.x + station, at.y + (invert − datum)·e + d/2·e), d/2 across and
  d/2·e high; the outer one about the same centre, d/2 + w across and (d/2 + w)·e high;
- a filled hatch bounds the ring: its boundary two loops, each one whole ellipse, the outer then the inner;
- the words sit over the outer wall's top.
"""

from __future__ import annotations

import json
import math
import sys
import unittest
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/cad.pipes.profile.json").read_text("utf-8"))
CASE = next(c for c in CASES["cases"] if c["name"].startswith("Kesişen borunun dış duvarı"))
SETUP, INPUT = CASE["setup"], CASE["steps"][0]["input"]


def own_walls() -> dict[str, float]:
    """Each water pipe's wall in metres, read from the drawing by this test."""
    return {u: float(e["attrs"]["Et"]) / 1000.0
            for u, e in zip(SETUP["uids"], SETUP["entities"]) if e.get("layerId") == "su" and "Et" in e.get("attrs", {})}


def semi_axes(e: dict) -> tuple[float, float]:
    """An ellipse's semi-axes across (x) and up (y)."""
    m = math.hypot(e["major"]["x"], e["major"]["y"])
    return (m, m * e["ratio"]) if abs(e["major"]["x"]) > abs(e["major"]["y"]) else (m * e["ratio"], m)


class PipeProfileWalls(unittest.TestCase):
    def test_the_outer_walls_and_their_rings(self) -> None:
        warnings.simplefilter("ignore", cad.CommandWarning)
        doc = cad.Document.from_bytes(kr.container(kr.document(SETUP)))
        out = doc.run("cad.pipes.profile", INPUT)
        self.assertEqual(out["status"], "completed", out)
        o = out["output"]
        made = [doc.entity(u).entity.to_json() for u in o["uids"]]
        ellipses = [e for e in made if e["kind"] == "ellipse"]
        hatches = [e for e in made if e["kind"] == "hatch"]
        texts = [e for e in made if e["kind"] == "text"]
        walls, at, datum, k = own_walls(), INPUT["at"], o["datum"], o["exaggeration"]
        drawn = [c for c in o["crossings"] if "invert" in c and "diameter" in c]
        self.assertEqual(len(drawn), 3)
        self.assertEqual(len(hatches), 3)
        for c in drawn:
            w = walls[c["uid"]]
            self.assertAlmostEqual(c["wall"], w, delta=1e-12)
            d, inv = c["diameter"], c["invert"]
            cx, cy = at[0] + c["station"], at[1] + (inv - datum) * k + d / 2 * k
            here = [e for e in ellipses if abs(e["c"]["x"] - cx) < 1e-6 and abs(e["c"]["y"] - cy) < 1e-6]
            self.assertEqual(len(here), 2, c["name"])
            got = sorted(semi_axes(e) for e in here)
            want = sorted([(d / 2, d / 2 * k), (d / 2 + w, (d / 2 + w) * k)])
            for g, wv in zip(got, want):
                self.assertAlmostEqual(g[0], wv[0], delta=1e-9)
                self.assertAlmostEqual(g[1], wv[1], delta=1e-9)
            ring = next(h for h in hatches if any(abs(cv["c"]["x"] - cx) < 1e-6 for lp in h["boundary"]
                                                  for cv in lp["curves"] if cv))
            self.assertEqual(ring["pattern"]["type"], "solid")
            outer, inner = (semi_axes(lp["curves"][0]) for lp in ring["boundary"])
            self.assertAlmostEqual(outer[0], d / 2 + w, delta=1e-9)
            self.assertAlmostEqual(inner[0], d / 2, delta=1e-9)
            top = cy + (d / 2 + w) * k
            words = next(t for t in texts if t["text"].startswith(c["name"] + " "))
            self.assertGreater(words["p"]["y"], top)


if __name__ == "__main__":
    unittest.main()
