"""The outside pieces' numbers on leaders when they do not fit (`cad.readjustment.sheets`' `numberLeader`, docs/adr/4680;
plan/map-cad.md MAP-11), held by the test's own reading: the font's advances by fontTools (not the reference's hand
reader, not KentOS), the text's box by ADR 0145's rule, the leader's landing and note as ADR 0146 lays them out.

At 6 mm on paper (6 m at 1:1000) with Kılavuz:

- 101/10's two pieces, 4 m wide, do not take their number at the full height, horizontal nor turned along their long
  sides (the test finds it on its own); each number is a leader instead, not stepped down: its tip the piece's label
  place (inside it), its line running east to one text height past the piece's east side, the note 6 m tall beyond its
  2 h landing and 0.5 h gap, wholly outside the piece;
- without Kılavuz the same numbers step down inside (docs/adr/4280).
"""

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

from fontTools.ttLib import TTFont  # noqa: E402

from kentos import cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402
import readjustment_scope_cases as rs  # noqa: E402
import readjustment_sheets_cases as sc  # noqa: E402

P = rs.P_
TEN = [P(10)]
NUMBERS = {rs.uid(P(10) + 1): ["101/24", "101/25"]}
FONT = TTFont(ROOT / "apps/desktop/assets/fonts/drawing/Barlow-500.ttf")
UPEM, CMAP, HMTX = FONT["head"].unitsPerEm, FONT.getBestCmap(), FONT["hmtx"]


def width(text: str, height: float) -> float:
    return sum(round(HMTX[CMAP[ord(c)]][0] * 1000 / UPEM) for c in text) / 1000 * height


def box(cx: float, cy: float, w: float, h: float, turn: float) -> list[tuple[float, float]]:
    """ADR 0145's box of a middle-centred one-line text, turned with it."""
    r = math.radians(turn)
    c, s = math.cos(r), math.sin(r)
    return [(cx + c * x - s * y, cy + s * x + c * y) for x, y in ((-w / 2, -0.73 * h), (w / 2, -0.73 * h),
                                                                 (w / 2, 0.65 * h), (-w / 2, 0.65 * h))]


def inside(q: tuple[float, float], ring: list[tuple[float, float]]) -> bool:
    sides = [(b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]) for a, b in zip(ring, ring[1:] + ring[:1])]
    return all(v >= -1e-9 for v in sides) or all(v <= 1e-9 for v in sides)


def drawing() -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(rs.setup(rs.MAIN))))


class SeparationNumberLeader(unittest.TestCase):
    def written(self, leader: bool) -> list[dict]:
        doc = drawing()
        scope = cad.ReadjustmentScope.from_json(sc.scope_input(TEN))
        out = cad.readjustment.sheets(doc, scope=scope, numbers=NUMBERS, number_height=6.0, number_leader=leader)
        return [doc.entity(u).entity.to_json() for u in out.uids]

    def test_the_numbers_too_big_go_outside_on_leaders(self) -> None:
        records = self.written(True)
        pieces = [r for r in records if r["kind"] == "polygon"]
        leaders = [r for r in records if r["kind"] == "leader"]
        self.assertEqual(len(leaders), 2)
        self.assertFalse([r for r in records if r["kind"] == "text"], "no text inside")
        for piece, lead in zip(pieces, leaders):
            ring = [(p["x"], p["y"]) for p in piece["pts"]]
            xs, ys = [q[0] for q in ring], [q[1] for q in ring]
            h, number = lead["height"], lead["text"]
            self.assertAlmostEqual(h, 6.0, delta=1e-9, msg="the full height, not stepped down")
            self.assertEqual(number, piece["attrs"]["ParselNo"])
            w = width(number, h)
            # The test's own check that the full height fits nowhere: horizontal or along the long side.
            mid = ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)
            for turn in (0.0, 90.0):
                self.assertFalse(all(inside(q, ring) for q in box(*mid, w, h, turn)), turn)
            tip, start = (lead["pts"][0]["x"], lead["pts"][0]["y"]), (lead["pts"][-1]["x"], lead["pts"][-1]["y"])
            self.assertTrue(inside(tip, ring), "the tip in the piece")
            self.assertAlmostEqual(tip[1], (min(ys) + max(ys)) / 2, delta=1e-9)
            self.assertAlmostEqual(start[0], max(xs) + h, delta=1e-9, msg="one text height past the east side")
            self.assertAlmostEqual(start[1], tip[1], delta=1e-9)
            self.assertEqual(lead["rotation"], 0.0)
            # ADR 0146: the note's middle-left 2.5 h past the landing's start; its box wholly east of the piece.
            note_left = start[0] + 2.5 * h
            self.assertGreater(note_left, max(xs))

    def test_without_the_leader_they_step_down_inside(self) -> None:
        records = self.written(False)
        self.assertFalse([r for r in records if r["kind"] == "leader"])
        texts = [r for r in records if r["kind"] == "text"]
        self.assertEqual(len(texts), 2)
        self.assertTrue(all(t["height"] < 6.0 for t in texts))

    def test_a_leader_without_a_height_is_refused(self) -> None:
        command = cad.command("cad.readjustment.sheets")
        input_ = {"scope": sc.scope_input(TEN), "numbers": NUMBERS, "numberLeader": True}
        answer = command.run(drawing(), command.input_type.from_json(input_), op="plan")
        self.assertEqual((answer.error.code, answer.error.path), ("invalid_input", "numberLeader"))


if __name__ == "__main__":
    unittest.main()
