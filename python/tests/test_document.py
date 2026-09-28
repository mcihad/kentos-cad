"""A drawing from Python end to end (docs/adr/0131; TODOS.md §14 acceptance):
polygon oluştur → ölç → katmana ata → kaydet, and what goes wrong on the way."""

from __future__ import annotations

import json
import tempfile
import unittest
import warnings
from pathlib import Path

import kentos.cad as cad

SQUARE = [(423500, 4512300), (423520, 4512300), (423520, 4512312.5), (423500, 4512312.5)]
ROOT = Path(__file__).resolve().parents[2]
# A drawing with a hidden layer (gizli) and a locked one (kilitli), the shared command cases' own.
FIXTURE = json.loads((ROOT / "fixtures/commands/v1/cad.circle.create.json").read_text(encoding="utf-8"))


def fixture_drawing() -> cad.Document:
    return cad.Document.from_bytes(json.dumps(FIXTURE["setup"]).encode())


def new() -> cad.Document:
    return cad.Document.new("Ada 101", srid=5256)


def other_layer(doc: cad.Document) -> cad.LayerNode:
    return next(n for n in doc.all_layers() if n.id != doc.active_layer and not n.locked and n.visible)


class Acceptance(unittest.TestCase):
    def test_polygon_measured_moved_saved_and_read_back(self) -> None:
        doc = new()
        info = doc.info()
        self.assertEqual(info.settings.srid, 5256)
        self.assertEqual(info.objects, 0)
        layer = doc.active_layer

        plan = cad.polygon.create.plan(doc, layer_id=layer, pts=SQUARE)
        self.assertIsInstance(plan, cad.PolygonPlan)
        self.assertIsInstance(plan.entity, cad.PolygonEntity)
        self.assertEqual(plan.entity.kind, "polygon")
        self.assertEqual(len(doc), 0, "a plan writes nothing")

        made = cad.polygon.create(doc, layer_id=layer, pts=SQUARE, expected_revision=plan.revision)
        self.assertIsInstance(made, cad.PolygonCreated)
        m = doc.measure(made.uid)
        self.assertEqual((m.kind, m.area, m.length), ("polygon", 250.0, 65.0))
        self.assertEqual(m.bounds, cad.Bounds(min_x=423500, min_y=4512300, max_x=423520, max_y=4512312.5))

        target = other_layer(doc)
        moved = cad.entities.set(doc, uids=[made.uid], operation=cad.PropertiesOperation.LAYER, layer_id=target.id)
        self.assertEqual(moved.changed, [made.uid])
        self.assertEqual(doc.entity(made.uid).entity.layer_id, target.id)

        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "ada.kcad"
            saved = doc.save(path)
            self.assertEqual(saved.path, path)
            self.assertGreater(saved.bytes, 0)
            self.assertFalse(doc.dirty)
            back = cad.Document.open(path)
            self.assertEqual(len(back), 1)
            self.assertEqual(back.measure(made.uid).area, 250.0)
            record = back.entity(made.uid)
            self.assertIsInstance(record.entity, cad.PolygonEntity)
            self.assertEqual(record.entity.layer_id, target.id)
            self.assertEqual(cad.Document.from_bytes(back.to_bytes()).measure(made.uid).area, 250.0)

    def test_a_plan_written_later_is_a_conflict(self) -> None:
        doc = new()
        plan = cad.polygon.create.plan(doc, layer_id=doc.active_layer, pts=SQUARE)
        cad.point.create(doc, layer_id=doc.active_layer, p=(1, 2))
        with self.assertRaises(cad.RevisionConflict) as caught:
            cad.polygon.create(doc, layer_id=doc.active_layer, pts=SQUARE, expected_revision=plan.revision)
        self.assertEqual(caught.exception.revision, doc.revision)
        self.assertEqual(len(doc), 1)


class Groups(unittest.TestCase):
    def test_a_failed_script_leaves_nothing_and_a_whole_one_is_one_step(self) -> None:
        doc = new()
        with self.assertRaises(ZeroDivisionError):
            with doc.group("Betik"):
                for _ in range(3):
                    cad.polygon.create(doc, layer_id=doc.active_layer, pts=SQUARE)
                1 / 0  # noqa: B018
        self.assertEqual(len(doc), 0)
        with doc.group("Betik"):
            for _ in range(3):
                cad.polygon.create(doc, layer_id=doc.active_layer, pts=SQUARE)
        self.assertEqual(len(doc), 3)
        self.assertEqual(doc.undo(), "Betik")
        self.assertEqual(len(doc), 0)
        self.assertEqual(doc.redo(), "Betik")
        self.assertEqual(len(doc), 3)

    def test_no_group_in_a_group_and_no_save_in_one(self) -> None:
        doc = new()
        with doc.group("Dış"):
            with self.assertRaises(cad.Busy):
                with doc.group("İç"):
                    pass
            with self.assertRaises(cad.Busy):
                doc.save(Path(tempfile.gettempdir()) / "kentos-hic-yazilmaz.kcad")


class Refusals(unittest.TestCase):
    def test_the_commands_refusal_raises_with_its_code_and_field(self) -> None:
        doc = new()
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.polygon.create(doc, layer_id="yok-boyle-katman", pts=SQUARE)
        self.assertEqual(caught.exception.code, "layer_not_found")
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.polygon.create(doc, layer_id=doc.active_layer, pts=SQUARE[:2])
        self.assertEqual(caught.exception.code, "too_few_corners")
        self.assertEqual(caught.exception.path, "pts")

    def test_run_gives_the_whole_answer_without_raising(self) -> None:
        doc = new()
        out = cad.polygon.create.run(doc, cad.PolygonCreate(layer_id="yok", pts=[cad.Vec2(0, 0)]))
        self.assertFalse(out.ok)
        self.assertEqual(out.status, "failed")
        self.assertIsInstance(out.error, cad.CommandFailed)

    def test_a_hidden_layer_is_written_with_a_warning_and_a_locked_one_not_at_all(self) -> None:
        doc = fixture_drawing()
        with warnings.catch_warnings(record=True) as seen:
            warnings.simplefilter("always")
            cad.circle.create(doc, layer_id="gizli", c=(487020, 4420015), r=7.5)
        notes = [w.message for w in seen if isinstance(w.message, cad.CommandWarning)]
        self.assertEqual([(n.code, n.command) for n in notes], [("layer_hidden", "cad.circle.create")])
        self.assertEqual(seen[0].filename, __file__, "the warning points at the caller")
        checked = cad.circle.create.validate(doc, layer_id="gizli", c=(0, 0), r=1)
        self.assertEqual([(n.code, n.path) for n in checked], [("layer_hidden", "layerId")])
        before = doc.revision
        with self.assertRaises(cad.CommandFailed) as caught:
            cad.circle.create(doc, layer_id="kilitli", c=(0, 0), r=1)
        self.assertEqual(caught.exception.code, "layer_locked")
        self.assertEqual(doc.revision, before)

    def test_an_old_file_is_read_and_saved_only_to_a_new_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            old = Path(tmp) / "eski.kcad"
            old.write_text(json.dumps(FIXTURE["setup"]), encoding="utf-8")
            doc = cad.Document.open(old)
            self.assertTrue(doc.info().legacy)
            with self.assertRaises(cad.FileError) as caught:
                doc.save()
            self.assertEqual(caught.exception.code, "legacy_file")
            saved = doc.save(Path(tmp) / "yeni.kcad")
            self.assertFalse(doc.info().legacy)
            self.assertEqual(doc.save().path, saved.path, "then it saves where it went")
            self.assertEqual(old.read_text(encoding="utf-8"), json.dumps(FIXTURE["setup"]), "the old file is as it was")

    def test_the_hosts_refusals_are_typed(self) -> None:
        doc = new()
        with self.assertRaises(cad.NotRunHere) as server:
            doc.run("project.rename", {"name": "x"})
        self.assertEqual(server.exception.code, "server_command")
        with self.assertRaises(cad.NotRunHere):
            doc.run("cad.yok", {})
        with self.assertRaises(cad.InvalidInput):
            doc.run("cad.polygon.create", {"layerId": 3})
        with self.assertRaises(cad.InvalidInput) as nan:
            cad.polygon.create(doc, layer_id=doc.active_layer, pts=[(0, 0), (float("nan"), 1), (2, 2)])
        self.assertEqual(nan.exception.code, "not_finite")
        with self.assertRaises(cad.UnknownObject):
            doc.measure("01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f")
        with self.assertRaises(cad.InvalidInput):
            doc.measure("bir-kimlik-degil")
        with self.assertRaises(cad.UnknownSystem):
            cad.Document.new(srid=4)
        with self.assertRaises(cad.FileError):
            cad.Document.open("/yok/boyle/bir/dosya.kcad")
        with self.assertRaises(cad.FileError):
            cad.Document.from_bytes(b"bu bir cizim degil")

    def test_a_new_drawing_has_nowhere_to_save_until_told(self) -> None:
        with self.assertRaises(cad.FileError) as caught:
            new().save()
        self.assertEqual(caught.exception.code, "no_path")


class Reading(unittest.TestCase):
    def test_pages_kinds_boxes_and_layers(self) -> None:
        doc = new()
        layer = doc.active_layer
        uids = [cad.polygon.create(doc, layer_id=layer, pts=SQUARE).uid for _ in range(5)]
        line = cad.line.create(doc, layer_id=layer, a=(0, 0), b=(10, 0))
        first = doc.page(limit=2)
        self.assertEqual([r.uid for r in first.items], uids[:2])
        self.assertIsNotNone(first.next)
        self.assertEqual([r.uid for r in doc.entities(page_size=2)], [*uids, line.uid])
        self.assertEqual([r.uid for r in doc.entities(kinds="line")], [line.uid])
        self.assertEqual([r.uid for r in doc.entities(bbox=(-1, -1, 11, 1))], [line.uid])
        self.assertEqual(len(list(doc.entities(layer=other_layer(doc)))), 0)
        self.assertEqual(doc.layer(layer).id, layer)
        with self.assertRaises(cad.UnknownObject):
            doc.layer("yok-boyle-katman")

    def test_every_kind_of_object_reads_as_its_class(self) -> None:
        doc = new()
        layer = doc.active_layer
        cad.point.create(doc, layer_id=layer, p=(1, 2))
        cad.circle.create(doc, layer_id=layer, c=(0, 0), r=5)
        cad.polyline.create(doc, layer_id=layer, pts=[(0, 0), (1, 1), (2, 0)])
        kinds = {type(r.entity).__name__: r.entity.kind for r in doc.entities()}
        self.assertEqual(kinds, {"PointEntity": "point", "CircleEntity": "circle", "PolylineEntity": "polyline"})


if __name__ == "__main__":
    unittest.main()
