"""Every object of the project file's samples reaches Python as it is (plan/cad.md ``CAD-DATA-22``, docs/adr/0265):
each valid sample of ``fixtures/kcad/v2`` (the drawings ``expected.json`` names, with the kinds and fields every schema
added) is opened by the native module, each object is read into the typed layer (``kentos.cad``'s ``Entity``) and
written back, and must equal the sample's object, with its persistent id."""

from __future__ import annotations

import json
import math
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad._runtime import dumps

DIR = Path(__file__).resolve().parents[2] / "fixtures" / "kcad" / "v2"


def same(a: object, b: object) -> bool:
    """Equal as JSON values: numbers by value (1.0 and 1 alike), NaN never written."""
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    if isinstance(a, (int, float)) and isinstance(b, (int, float)) and not isinstance(a, bool):
        return a == b or (math.isnan(a) and math.isnan(b))
    return a == b


class CarriageTests(unittest.TestCase):
    def test_every_sample_object_reaches_the_typed_layer_and_back(self) -> None:
        expected = json.loads((DIR / "expected.json").read_text(encoding="utf-8"))
        files = objects = 0
        kinds: set[str] = set()
        for case in expected["files"]:
            content = case.get("content")
            if not content:
                continue
            sample = json.loads((DIR / content).read_text(encoding="utf-8"))
            doc = cad.Document.open(DIR / case["file"])
            records = list(doc.entities())
            self.assertEqual([r.uid for r in records], sample.get("uids", [r.uid for r in records]), content)
            for record, want in zip(records, sample["entities"]):
                got = json.loads(dumps(record.entity.to_json()))
                with self.subTest(file=content, kind=want["kind"]):
                    self.assertTrue(same(got, want), f"{got}\n≠\n{want}")
                kinds.add(want["kind"])
                objects += 1
            files += 1
        self.assertGreaterEqual(files, 50)
        self.assertGreaterEqual(objects, 200)
        self.assertGreaterEqual(len(kinds), 20)


if __name__ == "__main__":
    unittest.main()
