"""Each atlas page's sources (`file.export.atlas`, docs/adr/3600; plan/map-cad.md MAP-18) through KentOS's Python, held
by the shared cases scripts/fixtures/atlas_cases.py writes (the pages' grounds and the box test its own; no KentOS code
in the expectations).

- With the atlas map alone, every page lists its own parcel first, then what its ground meets, as the case says (the
  drawing's object numbers made persistent ids here); the first page's four objects' digests are the case's.
- With the overview map following the atlas map, every page meets the whole drawing (109 objects).
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/atlas/v1/cases.json").read_text("utf-8"))


def drawing(setup: str) -> cad.Document:
    s = CASES["setups"][setup]
    doc = cad.Document.from_bytes(json.dumps(s["drawing"]).encode())
    edit = cad.command("cad.sheets.edit")
    for sheet in s["book"]["sheets"]:
        added = edit.run(doc, edit.input_type.from_json({"ops": [{"op": "addSheet", "sheet": sheet}]}), op="execute")
        assert added.error is None, added.error
    return doc


def resolved(doc: cad.Document, value):
    """`$uidOf:N` as the persistent id the drawing's object N got when it was read."""
    ids = {doc.entity(r.uid).entity.to_json()["id"]: r.uid for r in doc.entities()}
    if isinstance(value, str) and value.startswith("$uidOf:"):
        return ids[int(value.split(":")[1])]
    if isinstance(value, list):
        return [resolved(doc, v) for v in value]
    if isinstance(value, dict):
        return {resolved(doc, k): resolved(doc, v) for k, v in value.items()}
    return value


def case(prefix: str) -> dict:
    return next(c for c in CASES["cases"] if c["name"].startswith(prefix))["steps"][0]["result"]["output"]


class AtlasPageSources(unittest.TestCase):
    def plan(self, doc: cad.Document):
        atlas = cad.command("file.export.atlas")
        path = str(Path(tempfile.mkdtemp()) / "atlas.pdf")
        answer = atlas.run(doc, atlas.input_type.from_json({"path": path, "sheet": "atlas-paftasi"}), op="plan")
        self.assertIsNone(answer.error, answer.error and answer.error.message)
        return answer.output

    def test_a_page_with_the_atlas_map_alone_lists_what_its_ground_meets(self) -> None:
        doc = drawing("plain")
        out = self.plan(doc)
        want = resolved(doc, case("Plan: haritası tek olan sayfa"))
        self.assertEqual([list(p.sources) for p in out.pages], [p["sources"] for p in want["pages"]])
        self.assertTrue(all(p.sources[0] == p.feature for p in out.pages))
        for uid, digest in want["objects"].items():
            self.assertEqual(out.objects[uid], digest)

    def test_the_overview_map_meets_the_whole_drawing_on_every_page(self) -> None:
        out = self.plan(drawing("main"))
        self.assertEqual({len(p.sources) for p in out.pages}, {109})


if __name__ == "__main__":
    unittest.main()
