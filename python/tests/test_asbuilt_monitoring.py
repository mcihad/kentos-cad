"""A monitoring job's two epochs (docs/adr/3620; plan/map-cad.md MAP-19), shown without KentOS code: the marks are
matched by name alone, each movement is the later epoch's coordinate minus the earlier one's, a mark beyond a limit has
moved, and a control point beyond a limit says the epochs do not rest on one datum.

The drawing is scripts/fixtures/asbuilt_cases.py's, its epochs `donem1` and `donem2` (seven marks each: M5 measured
only at the first, M6 only at the second). Python's own arithmetic gives the movements and the limits' verdict; the
check run the other way round negates every movement and swaps the new and the unmeasured mark; a design check's
inputs (a surface, lines, a search radius) are refused in a monitoring job.
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from kentos import cad  # noqa: E402
from kentos.cad.errors import CommandFailed  # noqa: E402

import asbuilt_cases as asb  # noqa: E402
import kcad_v2_reference as kr  # noqa: E402

H, V = 0.008, 0.01


def epoch(rows) -> dict[str, tuple[float, float, float]]:
    return {r[0]: (r[1], r[2], r[3]) for r in rows}


class Monitoring(unittest.TestCase):
    def setUp(self) -> None:
        self.doc = cad.Document.from_bytes(kr.container(kr.document(asb.setup())))

    def watch(self, measured: str, design: str, **more):
        return cad.asbuilt.check(self.doc, kind="monitoring", measured=measured, design=design, **more)

    def test_each_mark_s_movement_is_the_later_epoch_minus_the_earlier(self) -> None:
        r = self.watch("donem2", "donem1", horizontal=H, vertical=V, control=["K1", "K2"])
        then, now = epoch(asb.EPOCH1), epoch(asb.EPOCH2)
        moved = []
        for p in r.points:
            if p.name not in then:
                self.assertEqual((p.name, p.status), ("M6", "unmatched"))
                continue
            self.assertEqual((p.by, p.design_name), ("name", p.name))
            de, dn, dh = (now[p.name][k] - then[p.name][k] for k in range(3))
            self.assertAlmostEqual(p.de, de, places=9)
            self.assertAlmostEqual(p.dn, dn, places=9)
            self.assertAlmostEqual(p.dh, dh, places=9)
            self.assertAlmostEqual(p.horizontal, math.hypot(de, dn), places=9)
            beyond = math.hypot(de, dn) > H or abs(dh) > V
            self.assertEqual(p.status, "outside" if beyond else "within", p.name)
            if beyond:
                moved.append(p.name)
        self.assertEqual(moved, ["M2", "M3", "K2"])
        self.assertEqual([m.name for m in r.missing], ["M5"])
        self.assertEqual(r.kind, "monitoring")
        self.assertEqual((r.control.checked, r.control.moved, r.control.missing), (["K1", "K2"], ["K2"], []))
        # No design surface, line or place in a monitoring job.
        self.assertTrue(all(p.surface is cad.UNSET or p.surface is None for p in r.points))
        self.assertTrue(all(p.by != "place" for p in r.points))

    def test_the_other_way_round_negates_every_movement(self) -> None:
        ahead = {p.name: p for p in self.watch("donem2", "donem1").points}
        back = self.watch("donem1", "donem2", control=["K1", "M5"])
        for p in back.points:
            if p.name == "M5":
                self.assertEqual(p.status, "unmatched")
                continue
            self.assertAlmostEqual(p.de, -ahead[p.name].de, places=9)
            self.assertAlmostEqual(p.dh, -ahead[p.name].dh, places=9)
        self.assertEqual([m.name for m in back.missing], ["M6"])
        self.assertEqual((back.control.checked, back.control.missing), (["K1"], ["M5"]))

    def test_a_design_check_s_inputs_are_refused(self) -> None:
        for extra, path in (({"surface": "Tasarım"}, "surface"), ({"lines": "bordur"}, "lines"), ({"search": 0.5}, "search")):
            with self.subTest(path=path), self.assertRaises(CommandFailed) as refused:
                self.watch("donem2", "donem1", **extra)
            self.assertEqual((refused.exception.code, refused.exception.path), ("invalid_asbuilt_check", path))
        with self.assertRaises(CommandFailed) as refused:
            cad.asbuilt.check(self.doc, measured="olcum", design="tasarim", control=["K1"])
        self.assertEqual(refused.exception.path, "control")


if __name__ == "__main__":
    unittest.main()
