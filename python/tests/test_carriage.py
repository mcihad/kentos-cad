"""Every object of the project file's samples reaches Python as it is (plan/cad.md ``CAD-DATA-22``, docs/adr/0265):
each valid sample of ``fixtures/kcad/v2`` (the drawings ``expected.json`` names, with the kinds and fields every schema
added) is opened by the native module, each object is read into the typed layer (``kentos.cad``'s ``Entity``) and
written back, and must equal the sample's object, with its persistent id."""

from __future__ import annotations

import json
import math
import tempfile
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

    def test_every_sample_is_saved_and_opened_again_as_it_was(self) -> None:
        """A project opened from any sample and saved again comes back with the same settings, layers and objects: the
        writer picks the schema that holds what it has, and a key it writes must be one that schema's reader reads
        (docs/adr/0558's ek to 0265: a project with value lists and no form was saved in a schema that could not read them)."""
        expected = json.loads((DIR / "expected.json").read_text(encoding="utf-8"))
        saved = 0
        with tempfile.TemporaryDirectory() as tmp:
            for case in expected["files"]:
                if not case.get("content"):
                    continue
                doc = cad.Document.open(DIR / case["file"])
                path = Path(tmp) / "kopya.kcad"
                doc.save(path)
                again = cad.Document.open(path)
                with self.subTest(file=case["file"]):
                    self.assertTrue(same(self.held(again), self.held(doc)))
                saved += 1
        self.assertGreaterEqual(saved, 100)

    def test_the_profile_projects_are_saved_and_opened_again(self) -> None:
        """fixtures/profiles/v1: a municipality's project (value lists, linked fields, relations), another institution's
        names for it and the park project: saved to a file and opened, as the desktop's Save does."""
        folder = DIR.parents[1] / "profiles" / "v1"
        with tempfile.TemporaryDirectory() as tmp:
            for name in ("belediye", "sivas", "park-r1"):
                doc = cad.Document.from_bytes((folder / f"{name}.json").read_bytes())
                path = Path(tmp) / f"{name}.kcad"
                doc.save(path)
                again = cad.Document.open(path)
                with self.subTest(project=name):
                    self.assertTrue(same(self.held(again), self.held(doc)))
                    self.assertFalse(again.info().dirty)

    @staticmethod
    def held(doc: cad.Document) -> dict[str, object]:
        """What a project holds, as JSON: its settings, its layers and its objects."""
        settings = json.loads(dumps(doc.settings.to_json()))
        # Save writes where a bound file is from the drawing's folder (`relative`, docs/adr/0506): not carried, made.
        for source in settings.get("sources", []):
            source.pop("relative", None)
        return {
            "settings": settings,
            "layers": [json.loads(dumps(n.to_json())) for n in doc.layers()],
            "entities": [json.loads(dumps(r.entity.to_json())) for r in doc.entities(page_size=10_000)],
        }


if __name__ == "__main__":
    unittest.main()
