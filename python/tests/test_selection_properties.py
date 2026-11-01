"""CAD-ADV-03: typed selection/property contracts and semantic batch height edits."""
import json
from pathlib import Path
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing(count=30):
    raw = json.loads((ROOT / "fixtures/interaction/v1/objects.kcad").read_text())
    raw["settings"].update(workspace="cad", srid=0, drawingUnit="m", plotScale=50)
    raw["origin"] = {"x": 0, "y": 0}
    entities = []
    for i in range(count):
        base = dict(id=i + 1, layerId="kilitli" if i % 17 == 0 else "cizim", attrs={})
        x, y = (i % 250) * 3., (i // 250) * 3.
        if i % 5 == 0:
            geometry = dict(kind="text", p=dict(x=x, y=y), text=f"T{i}",
                            height=.3 if i % 2 == 0 else .5, rotation=0)
        else:
            geometry = dict(kind="line", a=dict(x=x, y=y), b=dict(x=x + 1, y=y))
        if i % 2 == 0:
            base["color"] = "#E5484D"
        entities.append(dict(**base, **geometry))
    raw["entities"] = entities
    return cad.Document.from_bytes(json.dumps(raw).encode())


class SelectionProperties(unittest.TestCase):
    def test_temporary_isolation_is_typed_unsaved_and_shared_with_processing(self):
        doc = drawing()
        original = doc.to_bytes()
        all_uids = cad.entities.select(doc, region=cad.AllSelectRegion()).uids
        action = cad.IsolateIsolationAction(uids=[all_uids[0]])
        self.assertEqual(cad.IsolationAction.from_json(action.to_json()).to_json(), action.to_json())
        planned = cad.entities.isolate.plan(doc, action=action)
        self.assertEqual(planned.retained, 1)
        self.assertFalse(cad.entities.isolate(doc, action=cad.QueryIsolationAction()).active)
        self.assertEqual(cad.entities.isolate(doc, action=action), planned)
        self.assertEqual(doc.to_bytes(), original)
        self.assertEqual(cad.entities.select(doc, region=cad.AllSelectRegion()).uids, all_uids[:1])
        result = cad.processing.run(doc, tool="selection.byExpression", parameters={
            "input":{"scope":"all"},"condition":"true"})
        self.assertEqual(result.selected, all_uids[:1])
        with self.assertRaises(cad.CommandFailed):
            cad.entities.isolate(doc, action=cad.IsolateIsolationAction(uids=[all_uids[0], "invalid"]))
        self.assertTrue(cad.entities.isolate(doc, action=cad.QueryIsolationAction()).active)
        reopened = cad.Document.from_bytes(doc.to_bytes())
        self.assertFalse(cad.entities.isolate(reopened, action=cad.QueryIsolationAction()).active)
        self.assertEqual(cad.entities.select(reopened, region=cad.AllSelectRegion()).uids, all_uids)
        ended = cad.entities.isolate(doc, action=cad.EndIsolationAction())
        self.assertFalse(ended.active)
        self.assertEqual(cad.entities.select(doc, region=cad.AllSelectRegion()).uids, all_uids)
        self.assertEqual(doc.to_bytes(), original)

    def test_processing_combines_property_hits_with_selection_without_writing(self):
        doc = drawing()
        before = doc.to_bytes()
        all_uids = cad.entities.select(doc, region=cad.AllSelectRegion()).uids
        hits = set(cad.entities.select(doc, region=cad.PropertiesSelectRegion(
            source=cad.AllSelectRegion(), query=cad.SelectProperties(text_height_min=.3, text_height_max=.3))).uids)
        current = set(all_uids[:15])
        for mode, expected in [("new", hits), ("add", current | hits),
                               ("remove", current - hits), ("within", current & hits)]:
            result = cad.processing.run(doc, tool="selection.byExpression", selection=list(current),
                parameters={"input":{"scope":"all"},"condition":"true","cadFilter":True,
                            "textHeightMin":.3,"textHeightMax":.3,"mode":mode})
            self.assertEqual(set(result.selected), expected, mode)
            self.assertEqual(doc.to_bytes(), before)

    def test_typed_query_round_trip_and_geometric_source(self):
        doc = drawing()
        source = cad.WindowSelectRegion(a=(0, -1), b=(31, 1))
        region = cad.PropertiesSelectRegion(source=source,
            query=cad.SelectProperties(text_height_min=.3, text_height_max=.3))
        self.assertEqual(cad.SelectRegion.from_json(region.to_json()).to_json(), region.to_json())
        before = doc.to_bytes()
        planned = cad.entities.select.plan(doc, region=region)
        selected = cad.entities.select(doc, region=region)
        self.assertEqual(selected.uids, planned.uids)
        self.assertEqual(len(selected.uids), 2)  # text at x=0 and x=30
        self.assertEqual(doc.to_bytes(), before)
        with self.assertRaises(cad.CommandFailed):
            cad.entities.select(doc, region=cad.PropertiesSelectRegion(source=region,
                query=cad.SelectProperties()))
        self.assertEqual(doc.to_bytes(), before)

    def test_50000_objects_processing_and_command_choose_the_same_5000_texts(self):
        doc = drawing(50000)
        before = doc.to_bytes()
        region = cad.PropertiesSelectRegion(source=cad.AllSelectRegion(),
            query=cad.SelectProperties(text_height_min=.3, text_height_max=.3))
        selected = cad.entities.select(doc, region=region)
        result = cad.processing.run(doc, tool="selection.byExpression", parameters={
            "input": {"scope": "all"}, "condition": "true", "cadFilter": True,
            "textHeightMin": .3, "textHeightMax": .3,
        })
        self.assertEqual(len(selected.uids), 5000)
        self.assertEqual(set(selected.uids), set(result.selected))
        self.assertEqual(doc.to_bytes(), before)
        records = {r.uid: r.entity.to_json() for r in doc.entities()}
        editable = [u for u in selected.uids if records[u]["layerId"] != "kilitli"]
        self.assertEqual(len(editable), 4705)
        isolated = cad.entities.isolate(doc, action=cad.IsolateIsolationAction(uids=selected.uids))
        self.assertEqual((isolated.hidden, isolated.retained), (45000, 5000))
        self.assertEqual(set(cad.entities.select(doc, region=cad.AllSelectRegion()).uids), set(selected.uids))
        self.assertEqual(doc.to_bytes(), before)
        edit = cad.HeightTextEdit(uids=editable, height=.35)
        cad.texts.edit.plan(doc, edit=edit)
        self.assertEqual(doc.to_bytes(), before)
        changed = cad.texts.edit(doc, edit=edit)
        self.assertEqual(len(changed.changed), 4705)
        after = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
        self.assertEqual(doc.undo(), "Yazı yüksekliği")
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), after)
        cad.entities.isolate(doc, action=cad.EndIsolationAction())
        self.assertEqual(len(cad.entities.select(doc, region=cad.AllSelectRegion()).uids), 50000)
        self.assertEqual(doc.to_bytes(), after)

    def test_lock_invalid_height_and_revision_conflict_never_partially_write(self):
        doc = drawing()
        uids = cad.entities.select(doc, region=cad.PropertiesSelectRegion(
            source=cad.AllSelectRegion(), query=cad.SelectProperties(text_height_min=.3))).uids
        before = doc.to_bytes()
        for edit in [cad.HeightTextEdit(uids=uids, height=.4),
                     cad.HeightTextEdit(uids=uids, height=0)]:
            with self.assertRaises(cad.CommandFailed):
                cad.texts.edit(doc, edit=edit)
            self.assertEqual(doc.to_bytes(), before)
        editable = [r.uid for r in doc.entities() if r.entity.to_json()["kind"] == "text"
                    and r.entity.to_json()["layerId"] != "kilitli"]
        with self.assertRaises(cad.RevisionConflict):
            cad.texts.edit(doc, edit=cad.HeightTextEdit(uids=editable, height=.4),
                           expected=cad.Expected(revision="999999"))
        self.assertEqual(doc.to_bytes(), before)


if __name__ == "__main__":
    unittest.main()
