"""Araç profilleri ve tarihli kapanmalar (docs/adr/0567) the whole way Python asks: the independent reference's grid
(fixtures/network/v1/access.json, scripts/fixtures/network_access_cases.py) drawn as a drawing, its network defined with
the profiles and the closures, built from the drawing as the İşlemler tools build it, and every question asked through
kentos.network with its profile and date."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import cad, network

ROOT = Path(__file__).resolve().parents[2]
FILE = json.loads((ROOT / "fixtures/network/v1/access.json").read_text(encoding="utf-8"))


def drawing() -> cad.Document:
    def layer(id: str, name: str) -> dict[str, Any]:
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []
    for e in FILE["edges"]:
        attrs = dict(e["attrs"])
        if e.get("closed"):
            attrs["kapali"] = "evet"
        entities.append({"kind": "polyline", "id": int(e["id"]), "layerId": "yol", "attrs": attrs,
                         "pts": [{"x": x, "y": y} for x, y in e["pts"]]})
    net = {**FILE["definition"], "closed": "kapali = 'evet'"}
    doc = {
        "format": "kentos.document",
        "version": 1,
        "name": "Araç profilleri",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow", "networks": [net]},
        "origin": {"x": 500000, "y": 4400000},
        "layers": [layer("yol", "Yol"), layer("diger", "Diğer")],
        "activeLayer": "diger",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(doc, ensure_ascii=False).encode("utf-8"))


class AccessTest(unittest.TestCase):
    def near(self, got: float, want: float, what: str) -> None:
        self.assertLessEqual(abs(got - want), 1e-9 * max(abs(want), 1.0), f"{what}: {got} ≠ {want}")

    def test_every_question_as_the_reference(self) -> None:
        net = network.Network(drawing(), "ag-1")
        self.assertEqual(net.problems, [])
        self.assertEqual([p.name for p in net.definition.profiles], ["Kamyon", "Yaya", "Otobüs", "Ambulans"])
        for c in FILE["cases"]:
            name = c["name"]
            net.profile = c["profile"]
            net.date = c["day"]
            self.assertEqual(net.closed["byProfile"], c["byProfile"], name)
            self.assertEqual(net.closed["byDate"], c["byDate"], name)
            self.assertEqual([int(i) for i in net.closed["unread"]], c["unread"], name)
            stops = [tuple(c["from"]), tuple(c["to"])]
            cost = net.costs[c["cost"]]
            if c.get("unreachable"):
                with self.assertRaises(network.NetworkError, msg=name) as e:
                    net.route(stops, cost=cost, reach=1.0)
                self.assertIn("duraklar arasında yol yok", str(e.exception))
                continue
            way = net.route(stops, cost=cost, reach=1.0)
            self.near(way.cost, c["total"], name)
            self.near(way.totals[0], c["totals"][0], name)
            self.near(way.totals[1], c["totals"][1], name)

    def test_an_unknown_profile_or_date_is_refused_and_keeps_the_access(self) -> None:
        net = network.Network(drawing(), "ag-1")
        net.profile = "Kamyon"
        for r in FILE["refusals"]:
            with self.assertRaises(network.NetworkError) as e:
                if "profile" in r:
                    net.profile = r["profile"]
                else:
                    net.date = r["day"]
            self.assertEqual(str(e.exception), r["error"])
        self.assertEqual((net.profile, net.date), ("Kamyon", None))


if __name__ == "__main__":
    unittest.main()
