"""kentos.network (docs/adr/0209 §11) against the independent reference's cases (fixtures/network/v1/cases.json,
scripts/fixtures/network_cases.py), the whole way Python asks: each scene drawn as a drawing (edges on ``yol``, their
direction and costs as attributes, a closed edge by the network's expression; each junction on a layer of its own
with its role, a closed valve by its layer's expression), opened as a Document, its network defined in the project
and built from the drawing as the İşlemler tools build it, and every question asked through kentos.network. The web
plays the same file through its worker (apps/web/src/app/networks.wasm.test.ts), the core natively."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import cad, network

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/network/v1/cases.json").read_text(encoding="utf-8"))
REACH = 5.0


def drawing(scene: dict[str, Any]) -> cad.Document:
    """A scene as a v1 drawing whose objects keep the scene's ids, its network ``ag-1``."""
    rules = scene["rules"]
    field = rules["direction"].get("field") if rules["direction"]["kind"] == "field" else None
    junctions = scene.get("junctions", [])

    def layer(id: str, name: str) -> dict[str, Any]:
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    entities = []
    for e in scene["edges"]:
        attrs: dict[str, str] = {}
        if field and e["direction"] is not None:
            attrs[field] = e["direction"]
        for i, c in enumerate(rules.get("costs", [])):
            if i < len(e["costs"]) and e["costs"][i] is not None:
                attrs[c["field"]] = e["costs"][i]
        if e["closed"]:
            attrs["kapali"] = "evet"
        out = {"kind": "polyline", "id": int(e["id"]), "layerId": "yol", "attrs": attrs, "pts": [{"x": x, "y": y} for x, y in e["pts"]]}
        if e.get("bulges"):
            out["bulges"] = e["bulges"]
        entities.append(out)
    for j in junctions:
        entities.append({"kind": "point", "id": int(j["id"]), "layerId": f"j{int(j['id'])}", "attrs": {"durum": "kapalı"} if j["closed"] else {},
                         "p": {"x": j["p"][0], "y": j["p"][1]}})
    net: dict[str, Any] = {
        "id": "ag-1",
        "name": scene["name"],
        "kind": "road",
        "edges": [{"layer": "yol"}],
        "junctions": [{"layer": f"j{int(j['id'])}", "role": j["role"], "closed": "durum = 'kapalı'"} for j in junctions],
        "connect": rules["connect"],
        "tolerance": rules["tolerance"],
        "direction": rules["direction"],
        "closed": "kapali = 'evet'",
    }
    if rules.get("costs"):
        net["costs"] = rules["costs"]
    if not junctions:
        del net["junctions"]
    doc = {
        "format": "kentos.document",
        "version": 1,
        "name": scene["name"],
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow", "networks": [net]},
        "origin": {"x": 500000, "y": 4400000},
        "layers": [layer("yol", "Yol"), *(layer(f"j{int(j['id'])}", f"Düğüm {int(j['id'])}") for j in junctions), layer("diger", "Diğer")],
        "activeLayer": "diger",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(doc, ensure_ascii=False).encode("utf-8"))


class CasesTest(unittest.TestCase):
    def near(self, got: float, want: float, what: str) -> None:
        self.assertLessEqual(abs(got - want), 1e-9 * max(abs(want), 1.0), f"{what}: {got} ≠ {want}")

    def test_every_case_as_the_reference(self) -> None:
        for case in CASES["cases"]:
            scene = case["scene"]
            name = scene["name"]
            doc = drawing(scene)
            net = network.Network(doc, "ag-1")
            self.assertEqual((net.counts["nodes"], net.counts["pieces"]), (len(case["graph"]["nodes"]), len(case["graph"]["pieces"])), name)
            self.assertEqual(net.problems, [], name)

            def at(k: str) -> tuple[float, float]:
                return (case["places"][k]["x"], case["places"][k]["y"])

            def cost_of(q: dict[str, Any]) -> str:
                return net.costs[q.get("cost", 0)]

            for q in case["questions"]:
                title = f"{name}: {q['title']}"
                want = q["expect"]
                barriers = [at(k) for k in q.get("barriers", [])]
                with self.subTest(title):
                    if q["kind"] == "route":
                        if "error" in want:
                            with self.assertRaises(network.NetworkError, msg=title):
                                net.route([at(k) for k in q["stops"]], cost=cost_of(q), reorder=q["reorder"], barriers=barriers, reach=REACH)
                            continue
                        r = net.route([at(k) for k in q["stops"]], cost=cost_of(q), reorder=q["reorder"], barriers=barriers, reach=REACH)
                        self.assertEqual(r.order, want["order"], title)
                        self.near(r.cost, want["cost"], title)
                        for i, t in enumerate(want["totals"]):
                            if t is None:
                                self.assertIsNone(r.totals[i], title)
                            else:
                                self.near(r.totals[i], t, f"{title}: cost {i}")
                        self.assertGreater(len(r.line.pts), 1, title)
                    elif q["kind"] == "area":
                        a = net.service_area([at(k) for k in q["facilities"]], q["breaks"], cost=cost_of(q), toward=q["toward"], separate=q["separate"],
                                             trim=q["trim"], barriers=barriers, reach=REACH, areas=False)
                        key = lambda f: -1 if f is None else f  # noqa: E731
                        got = sorted((key(line.facility), line.band) for line in a.lines)
                        wanted = sorted((key(line["facility"]), line["band"]) for line in want["lines"])
                        self.assertEqual(got, wanted, title)
                    elif q["kind"] in ("closest", "matrix"):
                        origins, targets = [at(k) for k in q["origins"]], [at(k) for k in q["targets"]]
                        if q["kind"] == "closest":
                            rows = net.closest_facility(origins, targets, count=q["k"], cutoff=q.get("cutoff"), cost=cost_of(q),
                                                        direction="fromFacility" if q["reverse"] else "toFacility", barriers=barriers, reach=REACH)
                        else:
                            rows = net.od_matrix(origins, targets, cost=cost_of(q), barriers=barriers, reach=REACH)
                        self.assertEqual(len(rows), len(want), title)
                        for row, w in zip(rows, want, strict=True):
                            self.assertEqual([f.target for f in row], [x["target"] for x in w], title)
                            for f, x in zip(row, w, strict=True):
                                self.near(f.cost, x["cost"], title)
                                if q["kind"] == "closest":
                                    self.assertIsNotNone(f.line, title)
                    elif q["kind"] == "trace":
                        t = net.trace([at(k) for k in q["starts"]], kind=q["trace"], barriers=barriers, reach=REACH)
                        self.near(t.length, want["length"], title)
                        ids = lambda edges: list(dict.fromkeys(int(scene["edges"][i]["id"]) for i in edges))  # noqa: E731
                        self.assertEqual(t.objects, ids(want["edges"]), title)
                        if q["trace"] == "isolation":
                            self.assertEqual([v.id for v in t.valves], [int(scene["junctions"][i]["id"]) for i in want["valves"]], title)
                            if "unfed" in want:
                                self.assertEqual(t.unfed, ids(want["unfedEdges"]), title)
                                self.near(t.unfed_length, want["unfedLength"], title)
                    elif q["kind"] == "check":
                        k = net.check()
                        self.assertEqual([k.nodes, k.pieces, k.dead_ends], [want["nodes"], want["pieces"], want["deadEnds"]], title)
                        self.near(k.length, want["length"], title)
                        self.assertEqual([p.kind for p in k.problems], [p["kind"] for p in want["problems"]], title)
                        for p, w in zip(k.problems, want["problems"], strict=True):
                            self.assertEqual(p.ids, [int(i) for i in w["ids"]], title)
                    else:
                        self.fail(f"unknown question {q['kind']}")


class DocumentTest(unittest.TestCase):
    def test_define_ask_and_remove(self) -> None:
        doc = drawing(CASES["cases"][0]["scene"])
        self.assertEqual([n.id for n in network.networks(doc)], ["ag-1"])
        defined = network.define(doc, {**network.networks(doc)[0].to_json(), "id": "ag-2", "name": "Kopya"})
        self.assertEqual([n.id for n in defined.networks], ["ag-1", "ag-2"])
        copy = network.Network(doc, "ag-2")
        self.assertEqual(copy.definition.name, "Kopya")
        self.assertEqual(copy.costs, ["Uzunluk", "Süre", "Ücret"])
        with self.assertRaises(network.NetworkError) as e:
            copy.route([(500050.0, 4400001.0), (500350.0, 4400151.0)], cost="Para")
        self.assertIn("“Para” maliyeti “Kopya” ağında yok", str(e.exception))
        with self.assertRaises(network.NetworkError) as e:
            copy.route([(500050.0, 4400001.0), (499000.0, 4399000.0)])
        self.assertEqual(str(e.exception), "2. durak ağa 100.00 m içinde değil.")
        # A way written as the apps write it: one polyline, its arc kept.
        way = network.route(doc, "ag-1", [(500050.0, 4400001.0), (500350.0, 4400151.0)], cost="Süre")
        self.assertTrue(any(b != 0.0 for b in way.line.bulges))
        written = cad.entities.create(doc, layer_id="diger", objects=[cad.types.NewObject(geometry=way.line.geometry(), attrs={"Ağ": "yollar"})])
        self.assertEqual(len(written.ids), 1)
        self.assertEqual(network.check(doc, "ag-1").pieces, 15)
        network.remove(doc, "ag-2")
        with self.assertRaises(network.NetworkError) as e:
            network.Network(doc, "ag-2")
        self.assertEqual(str(e.exception), "“ag-2” ağı projede yok; Ağlar penceresinden tanımlayın.")


if __name__ == "__main__":
    unittest.main()
