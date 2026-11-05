"""kentos.cad.review.notes and kentos.cad.drawing.take (docs/adr/1000) from Python on the real local engine: a note
bound to a line goes stale when the line moves, the difference taken back from a saved revision makes it current again,
undo takes the take back, a save–open keeps the notes, and validate/plan write nothing. The shared cases
(fixtures/commands/v1/cad.review.notes.*.json, cad.drawing.take.json) run in test_command_cases.py."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad

NOTE = "0192f5b0-5000-7000-8000-0000000000aa"


class ReviewNotes(unittest.TestCase):
    def test_a_note_goes_stale_and_taking_the_difference_makes_it_current(self) -> None:
        doc = cad.Document.new("İnceleme", workspace="cad", drawing_unit="m")
        line = cad.line.create(doc, layer_id=doc.active_layer, a=(0, 0), b=(4, 0))
        with tempfile.TemporaryDirectory() as folder:
            saved = Path(folder) / "rev-a.kcad"
            saved.write_bytes(doc.to_bytes())
            edited = cad.review.notes.edit(doc, change=cad.AddReviewNoteOp(
                id=NOTE, text="Uzunluk 4 m olmalı.", anchors=[line.uid], revision="A", author="Ayşe"))
            self.assertEqual((edited.id, edited.number, edited.notes), (NOTE, 1, 1))
            listed = cad.review.notes.list(doc)
            self.assertEqual(listed.open, 1)
            self.assertFalse(listed.notes[0].stale)

            cad.entities.transform(doc, uids=[line.uid], transform=cad.MoveTransform(dx=1.0, dy=0.0))
            listed = cad.review.notes.list(doc, stale_only=True)
            self.assertEqual(len(listed.notes), 1)
            self.assertEqual(listed.notes[0].anchor_states[0].state, cad.ReviewAnchorState.CHANGED)

            against = cad.FileCompareAgainst(path=str(saved))
            before = doc.info()
            plan = cad.drawing.take.plan(doc, against=against, uids=[line.uid])
            self.assertEqual(plan.replaced, [line.uid])
            self.assertEqual(doc.info(), before, "a plan writes nothing")
            taken = cad.drawing.take(doc, against=against, uids=[line.uid])
            self.assertEqual(taken.replaced, [line.uid])
            self.assertFalse(cad.review.notes.list(doc).notes[0].stale)
            self.assertEqual(doc.undo(), "Farkları al")
            self.assertTrue(cad.review.notes.list(doc).notes[0].stale)

            cad.review.notes.edit(doc, change=cad.ReplyReviewNoteOp(id=NOTE, text="Düzeltildi.", status="resolved"))
            reopened = cad.Document.from_bytes(doc.to_bytes())
            notes = cad.review.notes.list(reopened)
            self.assertEqual(notes.resolved, 1)
            self.assertEqual(notes.notes[0].note.replies[0].text, "Düzeltildi.")

    def test_refusals_come_back_with_their_codes_and_write_nothing(self) -> None:
        doc = cad.Document.new("Retler", workspace="cad", drawing_unit="m")
        before = doc.info()
        with self.assertRaises(cad.CommandError) as refused:
            cad.review.notes.edit(doc, change=cad.AddReviewNoteOp(text=" boşluklu"))
        self.assertEqual(refused.exception.code, "invalid_review_note")
        with self.assertRaises(cad.CommandError) as refused:
            cad.drawing.take(doc, against=cad.HistoryCompareAgainst(steps=1), uids=["0192f5b0-5000-7000-8000-000000000001"])
        self.assertEqual(refused.exception.code, "no_such_step")
        self.assertEqual(doc.info(), before)


if __name__ == "__main__":
    unittest.main()
