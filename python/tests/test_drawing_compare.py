"""kentos.cad.drawing.compare (docs/adr/0576 §3) from Python: the drawing against its own undo history and against
a drawing file, on the interaction traces' drawing (fixtures/interaction/v1/edits.kcad) and the shared revision files
(fixtures/compare/v1/revisions). The shared cases (fixtures/commands/v1/cad.drawing.compare.json) run in
test_command_cases.py; this is the same questions through the plain API, and the read only promise."""

from __future__ import annotations

import unittest
from pathlib import Path

import kentos.cad as cad

ROOT = Path(__file__).resolve().parents[2]
EDITS = ROOT / "fixtures/interaction/v1/edits.kcad"
REVISIONS = ROOT / "fixtures/compare/v1/revisions"


class DrawingCompare(unittest.TestCase):
    def test_a_moved_object_is_changed_against_its_history_and_nothing_is_written(self) -> None:
        doc = cad.Document.from_bytes(EDITS.read_bytes())
        first = next(iter(doc.entities(page_size=1000))).uid
        cad.entities.transform(doc, uids=[first], transform=cad.MoveTransform(dx=5.0, dy=0.0))
        before = doc.info()
        got = cad.drawing.compare(doc, against=cad.HistoryCompareAgainst(steps=1))
        self.assertEqual((got.changed, got.added, got.removed), (1, 0, 0))
        self.assertEqual([o.uid for o in got.changed_objects], [first])
        self.assertEqual(got.changed_objects[0].status, "geometry")
        # Read only: the revision, the history and the dirt are as they were.
        self.assertEqual(doc.info(), before)

    def test_a_file_and_the_refusals(self) -> None:
        doc = cad.Document.from_bytes((REVISIONS / "ayni.kcad").read_bytes())
        got = cad.drawing.compare(doc, against=cad.FileCompareAgainst(path=str(REVISIONS / "tasindi.kcad")))
        self.assertEqual((got.changed, got.added, got.removed, got.same), (1, 1, 1, 5))
        cases = [
            (cad.HistoryCompareAgainst(steps=1), "no_such_step"),
            (cad.FileCompareAgainst(path="goreli.kcad"), "invalid_file_path"),
            (cad.FileCompareAgainst(path=str(REVISIONS / "yok.kcad")), "file_not_found"),
        ]
        for against, code in cases:
            with self.subTest(code=code):
                with self.assertRaises(cad.CommandError) as e:
                    cad.drawing.compare(doc, against=against)
                self.assertEqual(e.exception.code, code)


if __name__ == "__main__":
    unittest.main()
