"""Ulaşılamayanın nedeni (docs/adr/0570) the whole way Python asks: the independent reference's chains
(fixtures/network/v1/blocked.json, scripts/fixtures/network_blocked_cases.py) drawn as a drawing, every question asked
through kentos.network; a way where the reference has one, else a refusal whose words name what shuts the way."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import cad, network

ROOT = Path(__file__).resolve().parents[2]
FILE = json.loads((ROOT / "fixtures/network/v1/blocked.json").read_text(encoding="utf-8"))
WHY = {"expression": "kapalı kenarlar ifadesiyle kapalı", "shut": "yön alanı kapalı", "oneWay": "tek yön, ters yönde",
       "profile": "araç profiline kapalı", "date": "o gün kapalı", "hours": "o saatte kapalı", "cost": "maliyetin değeri yok"}


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
    doc = {
        "format": "kentos.document",
        "version": 1,
        "name": "Ulaşılamayan",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow",
                     "networks": [{**FILE["definition"], "closed": "kapali = 'evet'"}]},
        "origin": {"x": 600000, "y": 4300000},
        "layers": [layer("yol", "Yol"), layer("diger", "Diğer")],
        "activeLayer": "diger",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(doc, ensure_ascii=False).encode("utf-8"))


class BlockedTest(unittest.TestCase):
    def test_every_question_as_the_reference(self) -> None:
        net = network.Network(drawing(), "ag-1")
        self.assertEqual(net.problems, [])
        for c in FILE["cases"]:
            name = c["name"]
            net.profile = c["profile"]
            net.date = c["day"]
            net.time = None if c["time"] is None else f"{int(c['time']) // 60:02d}:{int(c['time']) % 60:02d}"
            stops = [tuple(c["from"]), tuple(c["to"])]
            barriers = [tuple(b) for b in c["barriers"]]
            cost = net.costs[c["cost"]]
            if c.get("reachable"):
                net.route(stops, barriers=barriers, cost=cost, reach=1.0)
                continue
            with self.assertRaises(network.NetworkError, msg=name) as e:
                net.route(stops, barriers=barriers, cost=cost, reach=1.0)
            words = str(e.exception)
            self.assertIn("1. ve 2. duraklar arasında yol yok", words, name)
            b = c["blocked"]
            if b["kind"] == "apart":
                self.assertIn("birbirine bağlanmayan parçalarında", words, name)
                continue
            said = [f"kenar {x['id']} ({WHY[x['why']]})" for x in b["edges"]] + [f"{i + 1}. engel" for i in b["barriers"]]
            self.assertIn("yolu açmak için en az kalkması gerekenler: " + ", ".join(said), words, name)


if __name__ == "__main__":
    unittest.main()
