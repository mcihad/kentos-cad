"""Wire-version selection without a network: a recorder sees the exact request.

Ordinary project.changes scripts must keep speaking v1 to existing servers;
recipe metadata opts into v2, including the empty-list removal tombstone.
"""
from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest
from typing import Any

ROOT = Path(__file__).resolve().parents[2]

from kentos.cad._runtime import ServerCommand
from kentos.cad.protocol import Expected, Stamps, Summary


class Input:
    def __init__(self, payload: dict[str, Any]):
        self.payload = payload

    def to_json(self) -> dict[str, Any]:
        return copy.deepcopy(self.payload)


class Output:
    @classmethod
    def from_json(cls, raw: dict[str, Any]) -> dict[str, Any]:
        return raw


class Changes(ServerCommand[Input, Output]):
    id = "project.changes"
    version = 1
    title = "Recorded changes"
    scope = "project"
    input_type = Input
    output_type = Output


class RecordingServer:
    def __init__(self):
        self.requests: list[dict[str, Any]] = []

    def _command(self, command, version, payload, expected_versions, idempotency_key):
        self.requests.append(dict(command=command, version=version, payload=payload,
                                  expected_versions=expected_versions, idempotency_key=idempotency_key))
        return {"received": True}


class ServerVersions(unittest.TestCase):
    def test_ordinary_payloads_keep_v1_and_recipe_presence_selects_v2(self):
        cases = [
            ({"features": []}, 1),
            ({"features": [], "project": None}, 1),
            ({"features": [], "project": {"name": "A"}}, 1),
            ({"features": [], "project": {"arrays": []}}, 1),
            ({"features": [], "project": {"dimensionSets": []}}, 2),
            ({"features": [], "project": {"dimensionSets": [{"id": "recipe"}]}}, 2),
            ({"features": [], "project": {"dimensionSets": None}}, 2),
        ]
        for payload, version in cases:
            with self.subTest(payload=payload):
                target = RecordingServer()
                original = copy.deepcopy(payload)
                result = Changes().execute_input(target, Input(payload), expected_versions={"@project": "3"}, idempotency_key="same-key")
                self.assertEqual(result, {"received": True})
                sent = target.requests[0]
                self.assertEqual(sent["version"], version)
                self.assertEqual(sent["payload"], original)
                self.assertEqual(sent["expected_versions"], {"@project": "3"})
                self.assertEqual(sent["idempotency_key"], "same-key")
                self.assertEqual(payload, original)

    def test_other_commands_keep_their_declared_version(self):
        class Other(Changes):
            id = "project.rename"
            version = 7
        target = RecordingServer()
        Other().execute_input(target, Input({"project": {"dimensionSets": []}}))
        self.assertEqual(target.requests[0]["version"], 7)

    def test_generator_has_one_stable_wrapper_per_id_and_explicit_supported_versions(self):
        spec = importlib.util.spec_from_file_location("kentos_sdk_generator_version_test", ROOT / "scripts/python/sdk.py")
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        try:
            spec.loader.exec_module(module)
            catalog = json.loads((ROOT / "apps/web/src/contracts/generated/commandCatalog.json").read_text())
            v1 = copy.deepcopy(next(c for c in catalog["commands"] if c["id"] == "project.changes" and c["version"] == 1))
            v2 = copy.deepcopy(v1)
            v2["version"] = 2
            for entries in [[v1, v2], [v2, v1]]:
                commands = module.commands_of({"commands": entries})
                self.assertEqual(len(commands), 1)
                self.assertEqual(commands[0].id, "project.changes")
                self.assertEqual(commands[0].version, 1)
                self.assertEqual(commands[0].versions, (1, 2))
            # Every generated source must parse, including decimal defaults
            # and wrappers whose nonempty defaults contain enum values.
            _, schemas = module.load()
            model = module.Model(schemas)
            for path, source in module.emit_modules(model, module.commands_of(catalog)).items():
                compile(source, str(path), "exec")
            # A future incompatible shape must not reuse one wrapper silently.
            v2["input"]["title"] = "DifferentProjectChanges"
            with self.assertRaises(module.Fail):
                module.commands_of({"commands": [v1, v2]})
        finally:
            sys.modules.pop(spec.name, None)

    def test_dimension_recipe_stamps_and_changes_survive_manual_protocol_helpers(self):
        self.assertEqual(Expected().to_json(), {})
        self.assertEqual(Expected(dimension_sets="0123456789abcdef", plan="plan-id").to_json(),
                         {"dimensionSets": "0123456789abcdef", "plan": "plan-id"})
        self.assertIsNone(Stamps.from_json({}).dimension_sets)
        self.assertEqual(Stamps.from_json({"dimensionSets": "0123456789abcdef"}).dimension_sets, "0123456789abcdef")
        summary = Summary.from_json({"dimensionSets": {"added": ["set-1"], "changed": [], "removed": []},
                                     "sources": {"dimensionSets": "before"}, "stamps": {"dimensionSets": "after"}})
        self.assertEqual(summary.dimension_sets["added"], ["set-1"])
        self.assertEqual(summary.sources.dimension_sets, "before")
        self.assertEqual(summary.stamps.dimension_sets, "after")
        self.assertEqual(Summary.from_json({}).dimension_sets, {})


if __name__ == "__main__":
    unittest.main()
