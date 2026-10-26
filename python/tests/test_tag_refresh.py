"""Bulk changes preserve existing labels and share one explicit history step."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/kcad'))
import kcad


class TagRefresh(unittest.TestCase):
    def drawing(self):
        setup = json.loads((ROOT / 'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
        doc = cad.Document.from_bytes(json.dumps(setup).encode())
        rooms = [r.uid for r in doc.entities() if r.entity.to_json()['kind'] == 'space']
        tags = cad.tags.create(doc, layer_id='etiket', tags=[
            cad.NewTag(of=rooms[0], at=cad.Vec2(-4., 8.), content='{name}', size=4., shape=cad.TagShape.ROUNDED, leader=True),
            cad.NewTag(of=rooms[0], at=cad.Vec2(-4., 10.), content='{area}', size=2., shape=cad.TagShape.NONE),
        ]).uids
        return doc, rooms, tags

    def test_mixed_new_and_existing_full_model_and_one_undo_reopen(self):
        doc, rooms, tags = self.drawing()
        before = doc.to_bytes()
        model = kcad.read(before)[1]
        al = cad.TagAll(kinds=[cad.TaggedKind.SPACE], refresh=True, content='{number} / {name}')
        plan = cad.tags.create.plan(doc, layer_id='etiket', all=al)
        self.assertEqual(doc.to_bytes(), before)
        self.assertEqual(len(plan.tags), 1)
        self.assertEqual(len(plan.refreshed), 2)
        out = cad.tags.create(doc, layer_id='etiket', all=al, expected_revision=plan.revision)
        self.assertEqual(out.refreshed, tags)
        self.assertEqual(len(out.uids), 1)
        expected = copy.deepcopy(model)
        for uid in tags:
            i = expected['uids'].index(uid)
            expected['entities'][i]['content'] = '{number} / {name}'
            expected['entities'][i]['text'] = '101 / Salon'
        # Geometry is computed independently; only the new randomly allocated identity
        # and next slot are obtained from the write result.
        expected['uids'].append(out.uids[0])
        expected['entities'].append(dict(kind='tag', id=out.ids[0], layerId='etiket', attrs={},
            of=rooms[1], at=dict(x=26.5,y=1.), anchor=dict(x=26.5,y=1.),
            content='{number} / {name}', text='102 / Mutfak', shape='rectangle'))
        after = doc.to_bytes()
        self.assertEqual(kcad.read(after)[1], expected)
        doc.undo(); self.assertEqual(doc.to_bytes(), before)
        doc.redo(); self.assertEqual(doc.to_bytes(), after)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'refresh.kcad'; doc.save(path)
            self.assertEqual(cad.Document.open(path).to_bytes(), after)
        repeated = cad.tags.create.answer(doc, layer_id='etiket', all=al)
        self.assertFalse(repeated.ok)
        self.assertEqual(repeated.error.code, 'no_objects')
        self.assertEqual(doc.to_bytes(), after)

    def test_absent_null_false_preserve_old_output_and_other_tag_layers_stay_untouched(self):
        for flag in [cad.UNSET, None, False]:
            doc, rooms, tags = self.drawing()
            old = {uid:doc.entity(uid).entity.to_json() for uid in tags}
            al = cad.TagAll(kinds=[cad.TaggedKind.SPACE], refresh=flag, content='{usage}')
            plan = cad.tags.create.plan(doc, layer_id='etiket', all=al)
            self.assertNotIn('refreshed', plan.to_json())
            out = cad.tags.create(doc, layer_id='etiket', all=al)
            self.assertNotIn('refreshed', out.to_json())
            self.assertEqual({uid:doc.entity(uid).entity.to_json() for uid in tags}, old)
        doc, rooms, tags = self.drawing()
        cad.tags.create(doc, layer_id='cizim', tags=[cad.NewTag(of=rooms[1], content='{name}')])
        old = doc.to_bytes()
        al = cad.TagAll(kinds=[cad.TaggedKind.SPACE], refresh=True, content='{number}')
        out = cad.tags.create(doc, layer_id='etiket', all=al)
        self.assertEqual(out.uids, [])
        self.assertEqual(out.refreshed, tags)
        old_model = kcad.read(old)[1]
        expected = copy.deepcopy(old_model)
        for uid in tags:
            e=expected['entities'][expected['uids'].index(uid)]; e.update(content='{number}',text='101')
        self.assertEqual(kcad.read(doc.to_bytes())[1], expected)

    def test_refusals_and_revision_are_atomic_and_explicit_list_is_separate(self):
        doc, rooms, tags = self.drawing()
        before = doc.to_bytes()
        for extra in [dict(content='{bad}'),dict(size=0.),dict(size=51.)]:
            al=cad.TagAll.from_json(dict(kinds=['space'],refresh=True,**extra))
            for op in ['plan','validate','execute']:
                answer=cad.tags.create.answer(doc, layer_id='etiket',all=al,op=op)
                self.assertFalse(answer.ok);self.assertEqual(doc.to_bytes(),before)
        al=cad.TagAll(kinds=[cad.TaggedKind.SPACE],refresh=True,content='{usage}')
        stale=cad.tags.create.plan(doc,layer_id='etiket',all=al).revision
        cad.spaces.edit(doc,uids=[rooms[0]],operation=cad.SetSpacesOperation(name='Çalışma'))
        changed=doc.to_bytes()
        out=cad.tags.create.answer(doc,layer_id='etiket',all=al,expected_revision=stale)
        self.assertFalse(out.ok);self.assertEqual(out.error.code,'revision_conflict');self.assertEqual(doc.to_bytes(),changed)
        out=cad.tags.create.answer(doc,layer_id='kilitli',all=al)
        self.assertFalse(out.ok);self.assertEqual(out.error.code,'layer_locked');self.assertEqual(doc.to_bytes(),changed)
        prior={uid:doc.entity(uid).entity.to_json() for uid in tags}
        out=cad.tags.create(doc,layer_id='etiket',tags=[cad.NewTag(of=rooms[0],content='{name}')],all=al)
        self.assertEqual(len(out.uids),2);self.assertNotIn('refreshed',out.to_json())
        self.assertEqual({uid:doc.entity(uid).entity.to_json() for uid in tags},prior)

    def test_locked_existing_labels_refuse_mixed_write_without_partial_new_label(self):
        doc, rooms, tags=self.drawing()
        cad.layers.set(doc,operation='locked',layer='etiket',value=True)
        before=doc.to_bytes()
        al=cad.TagAll(kinds=[cad.TaggedKind.SPACE],refresh=True,content='{number}')
        for op in ['validate','plan','execute']:
            out=cad.tags.create.answer(doc,layer_id='etiket',all=al,op=op)
            self.assertFalse(out.ok);self.assertEqual(out.error.code,'layer_locked')
            self.assertEqual(doc.to_bytes(),before)

    def test_total_cap_counts_existing_and_new_labels_together(self):
        doc, rooms, tags=self.drawing()
        # 9,998 more labels of the first room: 10,000 existing labels plus
        # one untagged room must be refused before any replacement or addition.
        cad.tags.create(doc,layer_id='etiket',tags=[cad.NewTag(of=rooms[0],content='{name}') for _ in range(9998)])
        before=doc.to_bytes()
        al=cad.TagAll(kinds=[cad.TaggedKind.SPACE],refresh=True,content='{number}')
        out=cad.tags.create.answer(doc,layer_id='etiket',all=al)
        self.assertFalse(out.ok);self.assertEqual(out.error.code,'invalid_tag')
        self.assertEqual(out.error.path,'all');self.assertEqual(doc.to_bytes(),before)

    def test_replayed_request_does_not_apply_the_same_mixed_operation_twice(self):
        doc, rooms, tags=self.drawing()
        before=doc.to_bytes()
        al=cad.TagAll(kinds=[cad.TaggedKind.SPACE],refresh=True,content='{number}')
        first=cad.tags.create.answer(doc,layer_id='etiket',all=al,request_id='tag-refresh-once')
        self.assertTrue(first.ok);self.assertFalse(first.replayed)
        after=doc.to_bytes()
        second=cad.tags.create.answer(doc,layer_id='etiket',all=al,request_id='tag-refresh-once')
        self.assertTrue(second.ok);self.assertTrue(second.replayed)
        self.assertEqual(second.output,first.output);self.assertEqual(doc.to_bytes(),after)
        # The existing request ledger survives Undo and marks the receipt undone,
        # so a replay cannot silently execute an undone request a second time.
        # Hold every drawing field to the original, and every receipt field to
        # the executed receipt except that specified history status.
        receipts=copy.deepcopy(kcad.read(after)[1]['requests'])
        self.assertEqual(len(receipts),1)
        self.assertEqual(receipts[0]['requestId'],'tag-refresh-once')
        self.assertEqual(receipts[0]['status'],'done')
        expected=kcad.read(before)[1]
        receipts[0]['status']='undone';expected['requests']=receipts
        doc.undo();self.assertEqual(kcad.read(doc.to_bytes())[1],expected)
        doc.redo();self.assertEqual(doc.to_bytes(),after)
