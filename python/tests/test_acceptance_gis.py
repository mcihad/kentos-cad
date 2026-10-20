"""The first end-to-end GIS acceptance project from Python (plan/gis.md CBS-44, docs/adr/0554): the shared scenario
(fixtures/acceptance/v1/parcels.json, worked out by scripts/fixtures/acceptance_parcels.py without KentOS code) replayed
through kentos.cad: parcels and buildings with typed fields, a wrong value taken back, Alan hesapla, Konuma göre seç, a
save and reopen, five formats written and the GeoPackage opened in a new project with nothing lost. With
``KENTOS_WRITE_ACCEPTANCE=1`` the files go to fixtures/acceptance/v1/out, where ``acceptance_parcels.py --files`` reads
them back with GDAL's OGR."""

from __future__ import annotations

import json
import os
import shutil
import tempfile
import unittest
from pathlib import Path
from typing import Any

import kentos.cad as cad

from test_acceptance import matches

ROOT = Path(__file__).resolve().parents[2]
SCENARIO = json.loads((ROOT / "fixtures/acceptance/v1/parcels.json").read_text(encoding="utf-8"))
KEPT = ROOT / "fixtures/acceptance/v1/out"


def bind(value: Any, names: dict[str, str], out: Path) -> Any:
    if isinstance(value, str) and value.startswith("$out:"):
        return str(out / value[5:])
    if isinstance(value, str) and value.startswith("$"):
        return names[value[1:]]
    if isinstance(value, dict):
        return {k: bind(v, names, out) for k, v in value.items()}
    if isinstance(value, list):
        return [bind(v, names, out) for v in value]
    return value


class GisAcceptance(unittest.TestCase):
    def test_the_parcels_are_made_checked_written_and_opened_elsewhere_as_the_scenario_says(self) -> None:
        p = SCENARIO["project"]

        def new() -> cad.Document:
            return cad.Document.new(p["name"], workspace=p["workspace"], srid=p["srid"])

        doc = new()
        names: dict[str, str] = {}
        tolerance = SCENARIO["tolerance"]
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            for step in SCENARIO["steps"]:
                name = step["name"]
                output: dict[str, Any] = {}
                if step.get("fresh"):
                    doc = new()
                if "command" in step:
                    answer = doc.run(step["command"], bind(step["input"], names, out))
                    self.assertEqual(answer.get("status"), "completed", f"{name}: {answer}")
                    output = answer.get("output") or {}
                    if isinstance(step.get("as"), str):
                        names[step["as"]] = output["uid"]
                    for key, field in step.get("bind", {}).items():
                        names[key] = output[field]
                elif "undo" in step:
                    self.assertEqual(doc.undo(), step["undo"], name)
                elif step.get("reopen"):
                    doc = cad.Document.from_bytes(doc.to_bytes())
                problems: list[str] = []
                expect = step.get("expect", {})
                for key, want in expect.get("objects", {}).items():
                    problems += matches(doc.entity(names[key]).entity.to_json(), want, tolerance, key)
                for key, want in expect.get("measures", {}).items():
                    problems += matches({"area": doc.measure(names[key]).area}, want, tolerance, f"{key} ölçü")
                if "count" in expect:
                    problems += matches(len(doc), expect["count"], 0, "nesne sayısı")
                if "selected" in expect:
                    want = sorted(names[n] for n in expect["selected"])
                    problems += matches(sorted(output.get("selected", [])), want, 0, "seçilenler")
                for layer, spec in expect.get("layers", {}).items():
                    problems += self.layer_problems(doc, layer, spec, tolerance)
                self.assertEqual(problems, [], name)
            if os.environ.get("KENTOS_WRITE_ACCEPTANCE"):
                shutil.rmtree(KEPT, ignore_errors=True)
                shutil.copytree(out, KEPT)

    def layer_problems(self, doc: cad.Document, layer: str, spec: dict[str, Any], tolerance: float) -> list[str]:
        """A layer of an opened file: found by its name, its rows by the key attribute, every attribute and area."""
        leaves = [n for n in walk(doc.layers()) if n.get("name") == layer and n.get("type") == "layer"]
        if len(leaves) != 1:
            return [f"{layer}: {len(leaves)} katman"]
        rows = {r.entity.to_json()["attrs"].get(spec["key"]): r for r in doc.entities(layer=leaves[0]["id"])}
        problems = matches(sorted(k for k in rows if k is not None), sorted(spec["rows"]), 0, f"{layer} satırları")
        for key, attrs in spec["rows"].items():
            record = rows.get(key)
            if record is None:
                continue
            have = record.entity.to_json()["attrs"]
            problems += matches({k: have.get(k) for k in attrs}, attrs, 0, f"{layer}/{key}")
            problems += matches(doc.measure(record.uid).area, spec["areas"][key], tolerance, f"{layer}/{key} alan")
        return problems


def walk(nodes: Any) -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for n in nodes:
        n = n.to_json() if hasattr(n, "to_json") else n
        out.append(n)
        out += walk(n.get("children", []))
    return out


if __name__ == "__main__":
    unittest.main()
