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


class TerminalsTest(unittest.TestCase):
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
