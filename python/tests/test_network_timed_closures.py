"""Saatli kapanmalar (docs/adr/0568) the whole way Python asks: the independent reference's grid
(fixtures/network/v1/timed-closures.json, scripts/fixtures/network_timed_closures_cases.py) drawn as a drawing, its
network defined with the closures' weekday and hour fields, built from the drawing as the İşlemler tools build it, and
every question with a whole minute asked through kentos.network with its date and time, in both orders."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import cad, network

ROOT = Path(__file__).resolve().parents[2]
FILE = json.loads((ROOT / "fixtures/network/v1/timed-closures.json").read_text(encoding="utf-8"))


def drawing() -> cad.Document:
    def layer(id: str, name: str) -> dict[str, Any]:
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = [{"kind": "polyline", "id": int(e["id"]), "layerId": "yol", "attrs": dict(e["attrs"]),
                 "pts": [{"x": x, "y": y} for x, y in e["pts"]]} for e in FILE["edges"]]
    doc = {
        "format": "kentos.document",
        "version": 1,
        "name": "Saatli kapanmalar",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow", "networks": [FILE["definition"]]},
        "origin": {"x": 400000, "y": 4500000},
        "layers": [layer("yol", "Yol"), layer("diger", "Diğer")],
        "activeLayer": "diger",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(doc, ensure_ascii=False).encode("utf-8"))


def clock(minutes: float | None) -> str | None:
    return None if minutes is None else f"{int(minutes) // 60:02d}:{int(minutes) % 60:02d}"


class TimedClosuresTest(unittest.TestCase):
    def near(self, got: float, want: float, what: str) -> None:
        self.assertLessEqual(abs(got - want), 1e-9 * max(abs(want), 1.0), f"{what}: {got} ≠ {want}")

    def test_every_whole_minute_question_as_the_reference(self) -> None:
        net = network.Network(drawing(), "ag-1")
        self.assertEqual(net.problems, [])
        asked = 0
        for c in FILE["cases"]:
            name, t = c["name"], c["time"]
            if t is not None and t != int(t):
                continue
            asked += 1
            # The date before the time and the time before the date answer alike.
            for date_first in (True, False):
                if date_first:
                    net.date = c["day"]
                    net.time = clock(t)
                else:
                    net.time = clock(t)
                    net.date = c["day"]
                self.assertEqual(net.closed["byDate"], c["byDate"], name)
                self.assertEqual(net.closed["byHours"], c["byHours"], name)
                self.assertEqual(net.closed["hoursUnasked"], c["hoursUnasked"], name)
                self.assertEqual([int(i) for i in net.closed["unread"]], c["unread"], name)
                way = net.route([tuple(c["from"]), tuple(c["to"])], cost=net.costs[c["cost"]], reach=1.0)
                self.near(way.cost, c["total"], name)
                self.near(way.totals[0], c["totals"][0], name)
                self.near(way.totals[1], c["totals"][1], name)
        self.assertEqual(asked, 19)


if __name__ == "__main__":
    unittest.main()
