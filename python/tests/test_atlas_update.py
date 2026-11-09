"""Atlasın güncelleme yayını (`file.export.atlas` `previous`; docs/adr/4000; plan/map-cad.md MAP-18) through KentOS's
Python, held by this test's own reading of the record (no KentOS code in the expectations).

On the shared cases' atlas-map-only drawing (scripts/fixtures/atlas_cases.py's `plain` setup):

- an execute writes the record beside the PDF: `atlas.kaynaklar.json`, `kentos.atlas-sources/1`, the PDF's name and
  SHA-256 (the PDF's own bytes hashed here), each page with its sources and each listed object's digest;
- parcel 2's owner changes: against that record, a page changed exactly when the record says it listed parcel 2,
  by that object alone; the others are the same; nothing is gone;
- an unreadable record is refused (`no_previous`).
"""

from __future__ import annotations

import hashlib
import json
import tempfile
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/atlas/v1/cases.json").read_text("utf-8"))


def drawing() -> cad.Document:
    s = CASES["setups"]["plain"]
    doc = cad.Document.from_bytes(json.dumps(s["drawing"]).encode())
    edit = cad.command("cad.sheets.edit")
    for sheet in s["book"]["sheets"]:
        added = edit.run(doc, edit.input_type.from_json({"ops": [{"op": "addSheet", "sheet": sheet}]}), op="execute")
        assert added.error is None, added.error
    return doc


class AtlasUpdate(unittest.TestCase):
    def test_the_record_tells_the_pages_that_changed(self) -> None:
        doc = drawing()
        atlas = cad.command("file.export.atlas")
        folder = Path(tempfile.mkdtemp())
        written = atlas.run(doc, atlas.input_type.from_json({"path": str(folder / "atlas.pdf"), "sheet": "atlas-paftasi"}),
                            op="execute")
        self.assertIsNone(written.error, written.error and written.error.message)
        record_path = folder / "atlas.kaynaklar.json"
        self.assertEqual(written.output.record, str(record_path))
        record = json.loads(record_path.read_text("utf-8"))
        self.assertEqual(record["format"], "kentos.atlas-sources/1")
        self.assertEqual(record["pdf"], "atlas.pdf")
        self.assertEqual(record["sha256"], hashlib.sha256((folder / "atlas.pdf").read_bytes()).hexdigest())
        self.assertEqual(len(record["pages"]), 100)

        ids = {doc.entity(r.uid).entity.to_json()["id"]: r.uid for r in doc.entities()}
        parcel = ids[2]
        cad.entities.set(doc, uids=[parcel], operation="attributes", attrs={"Malik": "Yeni Malik"})
        planned = atlas.run(doc, atlas.input_type.from_json(
            {"path": str(folder / "atlas-2.pdf"), "sheet": "atlas-paftasi", "previous": str(record_path)}), op="plan")
        self.assertIsNone(planned.error, planned.error and planned.error.message)
        changes = planned.output.changes
        listed_it = {p["feature"] for p in record["pages"] if parcel in p["sources"]}
        self.assertGreater(len(listed_it), 1)
        for page in changes.pages:
            if page.feature in listed_it:
                self.assertEqual((page.state.value, list(page.changed)), ("changed", [parcel]), page.feature)
            else:
                self.assertEqual(page.state.value, "same", page.feature)
        self.assertFalse(changes.gone)

    def test_a_record_that_is_none_is_refused(self) -> None:
        doc = drawing()
        atlas = cad.command("file.export.atlas")
        folder = Path(tempfile.mkdtemp())
        (folder / "bozuk.kaynaklar.json").write_text("{}", "utf-8")
        answer = atlas.run(doc, atlas.input_type.from_json(
            {"path": str(folder / "atlas.pdf"), "sheet": "atlas-paftasi", "previous": str(folder / "bozuk.kaynaklar.json")}),
            op="plan")
        self.assertEqual((answer.error.code, answer.error.path), ("no_previous", "previous"))


if __name__ == "__main__":
    unittest.main()
