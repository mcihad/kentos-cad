"""Project standards use the public Python command and survive KCAD saving."""

import json
from pathlib import Path
import sys
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/kcad"))
import kcad  # The independent KCAD reader.


class ProjectStandardDocuments(unittest.TestCase):
    def test_associate_save_reopen_and_remove(self):
        fixture = json.loads((ROOT / "fixtures/commands/v1/cad.blocks.find.json").read_text())
        document = cad.Document.from_bytes(json.dumps(fixture["setup"]).encode())
        source = json.loads((ROOT / "fixtures/kcad/v2/standard-docs.json").read_text())
        documents = [cad.ProjectStandardDocument(text=item["text"])
                     for item in source["settings"]["standardDocs"]]
        before = document.to_bytes()
        cad.settings.set.plan(document, standard_docs=documents)
        self.assertEqual(document.to_bytes(), before)
        changed = cad.settings.set(document, standard_docs=documents)
        self.assertIn("standardDocs", changed.changed)
        saved = document.to_bytes()
        self.assertEqual(kcad.read(saved)[1]["settings"]["standardDocs"],
                         source["settings"]["standardDocs"])
        reopened = cad.Document.from_bytes(saved)
        self.assertEqual([d.text for d in reopened.info().settings.standard_docs],
                         [d.text for d in documents])
        cad.settings.set(reopened, standard_docs=[])
        self.assertNotIn("standardDocs", kcad.read(reopened.to_bytes())[1]["settings"])


if __name__ == "__main__":
    unittest.main()
