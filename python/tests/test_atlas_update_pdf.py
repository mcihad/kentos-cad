"""Atlasın güncelleme PDF'i (`file.export.atlas` `changedOnly`; docs/adr/4400; plan/map-cad.md MAP-18) through KentOS's
Python, held by independent readers of the PDF (pypdf's page labels, poppler's pdftotext) and this test's own reading
of the previous record.

On the shared cases' atlas-map-only drawing (scripts/fixtures/atlas_cases.py's `plain` setup): the whole atlas is
written with its record; parcel 2's owner changes; the update's PDF then holds exactly the pages the record says listed
parcel 2, in the atlas's order:

- its page count is theirs, and `written` names their indices;
- each page's label (the PDF's /PageLabels, read by pypdf) is its own number in the atlas;
- each page's text (pdftotext) says “Sayfa N / 100”, its own number and the atlas's count, and its parcel's name;
- the record beside it still holds every page (the next update is told from it) and says which pages it holds.
"""

from __future__ import annotations

import json
import subprocess
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


def text_of(pdf: Path, page: int) -> str:
    return subprocess.run(["pdftotext", "-f", str(page), "-l", str(page), "-layout", str(pdf), "-"],
                          check=True, capture_output=True, text=True).stdout


class AtlasUpdatePdf(unittest.TestCase):
    def test_only_the_changed_pages_with_their_own_numbers(self) -> None:
        from pypdf import PdfReader

        doc = drawing()
        atlas = cad.command("file.export.atlas")
        folder = Path(tempfile.mkdtemp())
        whole = atlas.run(doc, atlas.input_type.from_json({"path": str(folder / "atlas.pdf"), "sheet": "atlas-paftasi"}),
                          op="execute")
        self.assertIsNone(whole.error, whole.error and whole.error.message)
        record = json.loads((folder / "atlas.kaynaklar.json").read_text("utf-8"))
        ids = {doc.entity(r.uid).entity.to_json()["id"]: r.uid for r in doc.entities()}
        parcel = ids[2]
        cad.entities.set(doc, uids=[parcel], operation="attributes", attrs={"Malik": "Yeni Malik"})
        # The record says which pages listed parcel 2: those, in the atlas's order.
        want = [p for p in record["pages"] if parcel in p["sources"]]
        self.assertGreater(len(want), 1)
        out = folder / "guncelleme.pdf"
        update = atlas.run(doc, atlas.input_type.from_json({"path": str(out), "sheet": "atlas-paftasi",
                                                            "previous": str(folder / "atlas.kaynaklar.json"),
                                                            "changedOnly": True}), op="execute")
        self.assertIsNone(update.error, update.error and update.error.message)
        self.assertEqual(list(update.output.written), [p["index"] for p in want])
        reader = PdfReader(str(out))
        self.assertEqual(len(reader.pages), len(want))
        self.assertEqual(list(reader.page_labels), [str(p["index"]) for p in want])
        for k, page in enumerate(want, start=1):
            text = text_of(out, k)
            self.assertIn(f"Sayfa {page['index']} / 100", text)
            self.assertIn(page["name"], text)
        again = json.loads((folder / "guncelleme.kaynaklar.json").read_text("utf-8"))
        self.assertEqual(len(again["pages"]), 100)
        self.assertEqual(again["written"], [p["index"] for p in want])

    def test_the_whole_atlas_has_no_page_labels(self) -> None:
        from pypdf import PdfReader

        doc = drawing()
        atlas = cad.command("file.export.atlas")
        folder = Path(tempfile.mkdtemp())
        written = atlas.run(doc, atlas.input_type.from_json({"path": str(folder / "atlas.pdf"), "sheet": "atlas-paftasi"}),
                            op="execute")
        self.assertIsNone(written.error, written.error and written.error.message)
        reader = PdfReader(str(folder / "atlas.pdf"))
        self.assertNotIn("/PageLabels", reader.trailer["/Root"])


if __name__ == "__main__":
    unittest.main()
