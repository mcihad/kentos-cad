"""Terminals and subnetworks (docs/adr/0557) through Python against the independent reference's cases
(fixtures/network/v1/terminals.json, scripts/fixtures/network_terminals_cases.py): each scene drawn as a drawing (its pipes
as lines whose attributes name the terminals their ends are attached to, its junctions on layers of their own, the devices'
junctions on their device layer), its network defined in the project with the devices' terminals and paths, and then
every trace asked through ``kentos.network`` and every subnetwork through ``kentos.cad.subnetworks.find``, the same
numbers the geometry core gives natively."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import cad, network

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/network/v1/terminals.json").read_text(encoding="utf-8"))


def layer(id_: str, name: str) -> dict[str, Any]:
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def drawing(scene: dict[str, Any], reach: str = "downstream", names: dict[int, str] | None = None) -> cad.Document:
    """A scene as a v1 drawing whose objects keep the scene's ids, its network ``ag-1``. `names` gives sources a name."""
    rules = scene["rules"]
    devices = (rules.get("terminals") or {}).get("devices", [])
    entities: list[dict[str, Any]] = []
    for e in scene["edges"]:
        attrs = {}
        for key, value in zip(("uc_bas", "uc_son"), e["terminals"]):
            if value is not None:
                attrs[key] = value
        if e.get("closed"):
            attrs["kapali"] = "evet"
        (ax, ay), (bx, by) = e["pts"][0], e["pts"][-1]
        entities.append({"kind": "line", "id": int(e["id"]), "layerId": "hat", "attrs": attrs, "a": {"x": ax, "y": ay}, "b": {"x": bx, "y": by}})
    layers = [layer("hat", "Hat")]
    junction_layers: list[dict[str, Any]] = []
    for k, j in enumerate(scene["junctions"]):
        device = j.get("device")
        if device is not None:
            layer_id = devices[device]["layer"]
            if layer_id not in {n["id"] for n in layers}:
                layers.append(layer(layer_id, layer_id))
                junction_layers.append({"layer": layer_id, "role": "junction"})
        else:
            layer_id = f"j{k}"
            layers.append(layer(layer_id, f"Düğüm {k}"))
            junction_layers.append({"layer": layer_id, "role": j["role"], "closed": "durum = 'kapalı'"})
        attrs = {"durum": "kapalı"} if j.get("closed") else {}
        # A device's configuration is named by the attribute `by` of its point (docs/adr/0562 §2).
        if device is not None and j.get("config") is not None and devices[device].get("by"):
            attrs[devices[device]["by"]] = j["config"]
        if names and k in names:
            attrs["Ad"] = names[k]
        entities.append({"kind": "point", "id": int(j["id"]), "layerId": layer_id, "attrs": attrs, "p": {"x": j["p"][0], "y": j["p"][1]}})
    net: dict[str, Any] = {
        "id": "ag-1", "name": scene["name"], "kind": "utility",
        "edges": [{"layer": "hat"}], "junctions": junction_layers,
        "connect": rules["connect"], "tolerance": rules["tolerance"], "direction": rules["direction"],
        "closed": "kapali = 'evet'",
        "subnetworks": {"name": "Ad", "attribute": "Alt ağ", "reach": reach},
    }
    if devices:
        net["terminals"] = {"start": "uc_bas", "end": "uc_son", "devices": devices}
    doc = {
        "format": "kentos.document", "version": 1, "name": scene["name"],
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow", "networks": [net]},
        "origin": {"x": 0, "y": 0},
        "layers": [*layers, layer("diger", "Diğer")],
        "activeLayer": "diger", "entities": entities, "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(doc, ensure_ascii=False).encode("utf-8"))


def where(scene: dict[str, Any], loc: dict[str, Any]) -> tuple[float, float]:
    """A reference location (an edge and the fraction of its length, or a node) as a point."""
    if "node" in loc:
        return (float(loc["node"][0]), float(loc["node"][1]))
    (ax, ay), (bx, by) = scene["edges"][loc["edge"]]["pts"][0], scene["edges"][loc["edge"]]["pts"][-1]
    t = loc.get("at", 0.5)
    return (ax + t * (bx - ax), ay + t * (by - ay))


def length_of(line: Any) -> float:
    """The length of a service line's straight stretches."""
    return sum(((a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2) ** 0.5 for a, b in zip(line.pts, line.pts[1:]))


class TerminalsTest(unittest.TestCase):
    def test_routes_matrices_and_areas_cross_devices_only_along_their_paths(self) -> None:
        routes = matrices = areas = 0
        for case in CASES["cases"]:
            scene = case["scene"]
            net = network.Network(drawing(scene), "ag-1")
            for q in case["routes"]:
                stops = [where(scene, s) for s in q["query"]["stops"]]
                title = f"{scene['name']}: {q['query']}"
                if q["cost"] is None:
                    with self.assertRaises(network.NetworkError, msg=title):
                        net.route(stops, reach=1.0)
                else:
                    got = net.route(stops, reach=1.0)
                    self.assertLessEqual(abs(got.cost - q["cost"]), 1e-9 * max(q["cost"], 1.0), title)
                routes += 1
            for q in case["matrices"]:
                query = q["query"]
                origins = [where(scene, p) for p in query["origins"]]
                targets = [where(scene, p) for p in query["targets"]]
                title = f"{scene['name']}: {query}"
                rows = net.od_matrix(origins, targets, reach=1.0)
                for row, want in zip(rows, q["from"]):
                    self.assertEqual(sorted(r.target for r in row), [j for j, c in enumerate(want) if c is not None], title)
                    for r in row:
                        self.assertLessEqual(abs(r.cost - want[r.target]), 1e-9 * max(want[r.target], 1.0), title)
                for k in q["closest"]:
                    rows = net.closest_facility(origins, targets, count=k["k"], cutoff=k["cutoff"], reach=1.0)
                    for row, want in zip(rows, k["rows"]):
                        self.assertEqual(len(row), len(want), title)
                        for r, (_, cost) in zip(row, want):
                            self.assertLessEqual(abs(r.cost - cost), 1e-9 * max(cost, 1.0), title)
                matrices += 1
            for q in case["areas"]:
                query = q["query"]
                places = [where(scene, p) for p in query["facilities"]]
                title = f"{scene['name']}: {query}"
                got = net.service_area(places, [query["break"]], toward=bool(query.get("toward")), areas=False, reach=1.0)
                total = sum(length_of(x.line) for x in got.lines)
                want = sum(v for _, v in q["lengths"])
                self.assertLessEqual(abs(total - want), 1e-6 * max(want, 1.0), title)
                areas += 1
        self.assertGreaterEqual((routes, matrices, areas), (20, 6, 15))

    def test_every_trace_as_the_reference(self) -> None:
        asked = 0
        for case in CASES["cases"]:
            scene = case["scene"]
            net = network.Network(drawing(scene), "ag-1")
            wanted = {(int(u[0]), tuple(u[1])) for u in case["unterminated"]}
            got = {(int(p.ids[0]), (p.x, p.y)) for p in net.check().problems if p.kind == "terminal"}
            self.assertEqual(got, wanted, scene["name"])
            for q in case["traces"]:
                query = q["query"]
                if "edge" in query:
                    (ax, ay), (bx, by) = scene["edges"][query["edge"]]["pts"][0], scene["edges"][query["edge"]]["pts"][-1]
                    at = ((ax + bx) / 2, (ay + by) / 2)
                else:
                    at = tuple(query["node"])
                t = net.trace([at], kind=query["kind"], reach=1.0)
                title = f"{scene['name']}: {query}"
                self.assertEqual(sorted(t.objects), sorted(int(i) for i in q["edges"]), title)
                self.assertEqual(sorted(t.junctions), sorted(int(scene["junctions"][k]["id"]) for k in q["junctions"]), title)
                asked += 1
        self.assertGreaterEqual(asked, 20)

    def test_every_subnetwork_as_the_reference(self) -> None:
        asked = 0
        for case in CASES["cases"]:
            scene = case["scene"]
            for q in case["subnets"]:
                query = q["query"]
                members = [k for g in query["groups"] for k in g]
                if any(scene["junctions"][k]["role"] != "source" for k in members):
                    continue
                names = {k: f"G{g}" for g, group in enumerate(query["groups"]) for k in group}
                # The sources not asked about keep their own names: each its own subnetwork would change the stops.
                if any(j["role"] == "source" and k not in names for k, j in enumerate(scene["junctions"])):
                    continue
                doc = drawing(scene, reach=query["reach"], names=names)
                found = cad.subnetworks.find(doc, network="ag-1")
                title = f"{scene['name']}: {query}"
                by_name = {s.name: s for s in found.subnetworks}
                self.assertEqual(sorted(by_name), [f"G{g}" for g in range(len(query["groups"]))], title)
                for g, r in enumerate(q["reached"]):
                    s = by_name[f"G{g}"]
                    self.assertEqual(s.edges, len(r["edges"]), title)
                    self.assertEqual(s.junctions, len(r["junctions"]), title)
                    self.assertLessEqual(abs(s.length - r["length"]), 1e-9 * max(r["length"], 1.0), title)
                asked += 1
        self.assertGreaterEqual(asked, 6)

    def test_update_writes_the_names_in_one_undo_step(self) -> None:
        case = next(c for c in CASES["cases"] if c["scene"]["name"].startswith("İki kaynak: ortak"))
        doc = drawing(case["scene"], names={0: "Merkez A", 1: "Merkez B"})
        done = cad.subnetworks.update(doc, network="ag-1")
        self.assertGreater(done.written, 0)
        records = {r.entity.id: r.entity for r in doc.entities(page_size=1000)}
        self.assertEqual(records[1].attrs.get("Alt ağ"), "Merkez A")
        self.assertEqual(records[4].attrs.get("Alt ağ"), "Merkez B")
        again = cad.subnetworks.find(doc, network="ag-1")
        self.assertEqual(again.stale.missing.count + again.stale.changed.count + again.stale.left_over.count, 0)


if __name__ == "__main__":
    unittest.main()
