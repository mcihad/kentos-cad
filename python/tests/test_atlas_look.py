"""Atlas sayfasının görünüşü (`file.export.atlas`'s page `look` and the changes' `look`; docs/adr/4820; plan/map-cad.md
MAP-18) through KentOS's Python, held by this test's own reading of the sheet and the record (no KentOS code in the
expectations).

On the shared cases' atlas-map-only drawing (scripts/fixtures/atlas_cases.py's `plain` setup), whose sheet prints the
page's own parcel's Ada, Parsel and Durum and whose map draws the parcels without labels:

- an execute writes each page's look into the record, 64 hex digits; the same drawing made twice gives the same looks;
- parcel 2's owner (the form hides it, nothing prints it) changes: the pages the record says list parcel 2 changed,
  every one looking the same;
- its Ada changes: its own page (the record's page whose object it is) looks other, the other pages listing it the
  same.
"""

from __future__ import annotations

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


class AtlasLook(unittest.TestCase):
    def test_a_page_looks_other_only_where_it_draws_the_change(self) -> None:
        doc = drawing()
        atlas = cad.command("file.export.atlas")
        folder = Path(tempfile.mkdtemp())

        def run(name: str, previous: Path | None = None):
            asked = {"path": str(folder / name), "sheet": "atlas-paftasi"}
            if previous is not None:
                asked["previous"] = str(previous)
            answer = atlas.run(doc, atlas.input_type.from_json(asked), op="execute")
            self.assertIsNone(answer.error, answer.error and answer.error.message)
            return answer.output

        run("atlas.pdf")
        record_path = folder / "atlas.kaynaklar.json"
        record = json.loads(record_path.read_text("utf-8"))
        looks = [p["look"] for p in record["pages"]]
        self.assertTrue(all(len(l) == 64 and all(c in "0123456789abcdef" for c in l) for l in looks))
        again = run("atlas-again.pdf")
        self.assertEqual([p.look for p in again.pages], looks, "the same drawing made twice looks the same")

        ids = {doc.entity(r.uid).entity.to_json()["id"]: r.uid for r in doc.entities()}
        parcel = ids[2]
        listed_it = {p["feature"] for p in record["pages"] if parcel in p["sources"]}
        self.assertGreater(len(listed_it), 1)

        cad.entities.set(doc, uids=[parcel], operation="attributes", attrs={"Malik": "Yeni Malik"})
        owner = run("atlas-2.pdf", record_path).changes
        changed = [p for p in owner.pages if p.state.value == "changed"]
        self.assertEqual({p.feature for p in changed}, listed_it)
        self.assertTrue(all(p.look.value == "same" for p in changed))

        cad.entities.set(doc, uids=[parcel], operation="attributes", attrs={"Ada": "A9"})
        island = run("atlas-3.pdf", record_path).changes
        for page in island.pages:
            if page.state.value != "changed":
                continue
            self.assertEqual(page.look.value, "changed" if page.feature == parcel else "same", page.feature)


if __name__ == "__main__":
    unittest.main()
