"""Each known point's own place in the project (`map.survey.query`'s `places`, docs/adr/4120; plan/map-cad.md MAP-03),
held by the test's own computation (no KentOS code in the expectations).

The drawings are the shared cases' (fixtures/commands/v1/map.survey.query.json: the network's archive, BN1 and BN2
published in TUREF at 2005.0, the campaign's epoch decision 2005.0). The test reads them as a surveyor would:

- BN1 published at 2026.84 with its own Earth-centred velocity V goes back to 2005.0 linearly, X(t) = X(t₀) + V·(t − t₀):
  its shift east, north, up is R(φ, λ)·V·Δt, worked out here; and since V is the grid's own velocity there and the
  2026.84 coordinates came from the 2005.0 ones along the grid, it lands within a millimetre of its 2005.0 place;
- N1, both a known point and an accepted point, published 6 mm east, 4 mm south and 12 mm up of the accepted
  result: the place minus the point is that far (the grid turns east and north by the meridian's convergence, the
  distance stays), the height 12 mm;
- without a velocity and without a grid the point at another epoch has no place, and the answer says why;
- without `places` nothing of it.
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

import kcad_v2_reference as kr  # noqa: E402

CASES = json.loads((ROOT / "fixtures/commands/v1/map.survey.query.json").read_text("utf-8"))


def opened(prefix: str):
    case = next(c for c in CASES["cases"] if c["name"].startswith(prefix))
    setup = case["setup"]
    return cad.Document.from_bytes(kr.container(kr.document(setup))), setup


def places(doc) -> list[dict]:
    out = cad.map.survey.query(doc, places=True).to_json()
    return out.get("knownPlaces", [])


def enu_rows(lat: float, lon: float) -> list[list[float]]:
    la, lo = math.radians(lat), math.radians(lon)
    return [[-math.sin(lo), math.cos(lo), 0.0],
            [-math.sin(la) * math.cos(lo), -math.sin(la) * math.sin(lo), math.cos(la)],
            [math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la)]]


class KnownPlaces(unittest.TestCase):
    def test_a_point_at_another_epoch_goes_back_by_its_own_velocity(self) -> None:
        doc, setup = opened("BN1 2026,84'te, kendi hızıyla")
        k = setup["settings"]["mapSurvey"]["known"][0]
        row = places(doc)[0]
        moved = row["moved"]
        self.assertEqual((moved["by"], moved["from"]), ("velocity", 2026.84))
        years = 2005.0 - 2026.84
        self.assertAlmostEqual(moved["years"], years, delta=1e-12)
        r = enu_rows(k["lat"], k["lon"])
        for i in range(3):
            want = sum(r[i][j] * k["velocity"][j] * years for j in range(3))
            self.assertAlmostEqual(moved["shift"][i], want, delta=1e-6, msg=f"shift[{i}]")
        # Back where it was published at 2005.0, within a millimetre (the velocity is the grid's, rounded).
        base, _ = opened("Bilinen noktaların projedeki kendi yerleri")
        then = places(base)[0]
        self.assertLess(math.hypot(row["x"] - then["x"], row["y"] - then["y"]), 0.001)
        self.assertEqual(row["epoch"], 2005.0)

    def test_the_place_minus_the_accepted_point(self) -> None:
        doc, _ = opened("N1 hem bilinen nokta hem kabul noktası")
        row = next(p for p in places(doc) if p["mark"] == "N1")
        self.assertEqual(row["point"], "kn-1")
        # Published to 1e-9° (about 0.1 mm) and 0.1 mm: the distance and the height within 0.2 mm.
        self.assertAlmostEqual(math.hypot(row["dx"], row["dy"]), math.hypot(0.006, 0.004), delta=2e-4)
        self.assertAlmostEqual(row["dz"], 0.012, delta=2e-4)
        self.assertGreater(row["dx"], 0.0)
        self.assertLess(row["dy"], 0.0)

    def test_no_velocity_and_no_grid_says_why(self) -> None:
        doc, _ = opened("BN1 2026,84'te, ne kendi hızı ne hız ızgarası")
        row = places(doc)[0]
        self.assertNotIn("x", row)
        self.assertIn("2026,84", row["why"])
        self.assertIn("kendi hızı", row["why"])
        self.assertIn("x", places(doc)[1], "BN2 at the decision has its place")

    def test_without_places_nothing(self) -> None:
        doc, _ = opened("Bilinen noktaların projedeki kendi yerleri")
        self.assertNotIn("knownPlaces", cad.map.survey.query(doc).to_json())


if __name__ == "__main__":
    unittest.main()
