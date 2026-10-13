"""The command protocol from Python (docs/adr/0253): a call's summary and
stamps, a plan the write must equal, what the call expects, a request sent
again, a command run as a job, the journal replayed, a change set and the
commands that write files."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import kentos.cad as cad
from kentos.cad import Document, Expected, RevisionConflict


def drawing() -> tuple[Document, str]:
    doc = Document.new("Protokol", workspace="cad", coords="local", drawing_unit="m")
    layer = doc.active_layer
    return doc, layer


def square(doc: Document, layer: str, x: float = 0.0) -> str:
    made = cad.polygon.create(doc, layer_id=layer, pts=[(x, 0), (x + 4, 0), (x + 4, 3), (x, 3)])
    return made.uid


class Protocol(unittest.TestCase):
    def test_a_write_says_what_it_touched_and_its_stamps(self) -> None:
        doc, layer = drawing()
        uid = square(doc, layer)
        answer = cad.entities.transform.answer(doc, uids=[uid], transform=cad.MoveTransform(dx=1.0, dy=0.0))
        self.assertTrue(answer.ok)
        s = answer.summary
        self.assertEqual(s.changed, [uid])
        self.assertEqual(s.stamps.objects[uid], doc.entity(uid).stamp)
        self.assertNotEqual(s.sources.objects[uid], s.stamps.objects[uid])
        self.assertIsNotNone(s.bounds)
        self.assertEqual(s.algorithm["policy"], "kentos.numeric/1")
        self.assertIsNotNone(answer.call)
        info = doc.info()
        self.assertEqual(len(info.stamps.layers), 16)

    def test_a_plan_is_written_as_planned_or_not_at_all(self) -> None:
        doc, layer = drawing()
        uid = square(doc, layer)
        move = cad.MoveTransform(dx=2.0, dy=0.0)
        plan = cad.entities.transform.answer(doc, uids=[uid], transform=move, op="plan")
        self.assertEqual(len(plan.plan_id), 32)
        written = cad.entities.transform.answer(doc, uids=[uid], transform=move, expected=Expected(plan=plan.plan_id))
        self.assertTrue(written.ok)
        self.assertEqual(written.plan_id, plan.plan_id)
        # Planned again, then the object moves: the plan no longer holds and nothing is written.
        again = cad.entities.transform.answer(doc, uids=[uid], transform=move, op="plan")
        cad.entities.transform(doc, uids=[uid], transform=cad.MoveTransform(dx=0.5, dy=0.0))
        revision = doc.revision
        with self.assertRaises(RevisionConflict) as e:
            cad.entities.transform(doc, uids=[uid], transform=move, expected=Expected(plan=again.plan_id))
        self.assertEqual(e.exception.code, "plan_changed")
        self.assertTrue(e.exception.retryable)
        self.assertEqual(doc.revision, revision)

    def test_a_stale_stamp_writes_nothing_and_names_its_object(self) -> None:
        doc, layer = drawing()
        uid = square(doc, layer)
        stamp = doc.entity(uid).stamp
        cad.entities.transform(doc, uids=[uid], transform=cad.MoveTransform(dx=1.0, dy=0.0), expected=Expected(objects={uid: stamp}))
        with self.assertRaises(RevisionConflict) as e:
            cad.entities.delete(doc, uids=[uid], expected=Expected(objects={uid: stamp}))
        self.assertEqual(e.exception.code, "stale_state")
        self.assertEqual(e.exception.uids, (uid,))
        self.assertEqual(len(doc), 1)

    def test_a_request_sent_again_is_applied_once(self) -> None:
        doc, layer = drawing()
        first = cad.point.create.answer(doc, layer_id=layer, p=(1.0, 1.0), request_id="betik:nokta-1")
        again = cad.point.create.answer(doc, layer_id=layer, p=(1.0, 1.0), request_id="betik:nokta-1")
        self.assertFalse(first.replayed)
        self.assertTrue(again.replayed)
        self.assertEqual(again.output, first.output)
        self.assertEqual(len(doc), 1)
        other = cad.point.create.answer(doc, layer_id=layer, p=(2.0, 2.0), request_id="betik:nokta-1")
        self.assertEqual(other.error.code, "request_reused")

    def test_a_command_runs_as_a_job(self) -> None:
        doc, layer = drawing()
        uid = square(doc, layer)
        job = doc.start("cad.entities.transform", {"uids": [uid], "transform": {"kind": "move", "dx": 3.0, "dy": 0.0}})
        status = job.wait()
        self.assertEqual(status.phase, "done")
        self.assertTrue(status.answer.ok)
        with self.assertRaises(cad.HostError) as e:
            job.cancel()
        self.assertEqual(e.exception.code, "already_committed")

    def test_the_journal_replays_on_another_drawing(self) -> None:
        doc, layer = drawing()
        uid = square(doc, layer)
        cad.entities.set(doc, uids=[uid], operation="color", color="#E5484D")
        journal = doc.journal()
        self.assertEqual(journal["format"], "kentos.replay/1")
        other, _ = drawing()
        report = other.replay(journal)
        self.assertEqual((report.same, report.failed), (2, 0))
        made = report.mapped[uid]
        self.assertEqual(other.entity(made).entity.color, "#E5484D")

    def test_a_change_set_is_one_write(self) -> None:
        doc, layer = drawing()
        applied = cad.changeset.apply(
            doc,
            label="İki oda",
            steps=[
                cad.ChangesetStep(
                    command="cad.polygon.create",
                    as_="a",
                    input={"layerId": layer, "pts": [{"x": 0, "y": 0}, {"x": 4, "y": 0}, {"x": 4, "y": 3}]},
                ),
                cad.ChangesetStep(
                    command="cad.entities.set", input={"uids": ["$a"], "operation": "color", "color": "#30A46C"}
                ),
            ],
        )
        self.assertEqual(applied.label, "İki oda")
        self.assertEqual(len(applied.steps), 2)
        self.assertEqual(doc.undo(), "İki oda")
        self.assertEqual(len(doc), 0)

    def test_files_are_planned_and_written_whole(self) -> None:
        doc, layer = drawing()
        square(doc, layer)
        with tempfile.TemporaryDirectory() as d:
            path = str(Path(d) / "oda.dxf")
            planned = cad.file.export.dxf.plan(doc, path=path)
            self.assertFalse(Path(path).exists())
            written = cad.file.export.dxf(doc, path=path)
            self.assertEqual(written.sha256, planned.sha256)
            self.assertEqual(Path(path).stat().st_size, written.bytes)
            with self.assertRaises(cad.CommandFailed) as e:
                cad.file.export.dxf(doc, path=path)
            self.assertEqual(e.exception.code, "file_exists")
            rows = doc.journal()["calls"]
            self.assertEqual(rows[-1]["effects"][0]["sha256"], written.sha256)


if __name__ == "__main__":
    unittest.main()
