"""Source floor scope, shared writes and independent full binary expectations."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/kcad'))
import kcad


class TagLevels(unittest.TestCase):
    def drawing(self, upper=False):
        setup = json.loads((ROOT/'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
        if upper:
            setup['entities'][1]['arch']['level']='kat-1'
            setup['entities'][3].setdefault('arch',{})['level']='kat-1'
        return cad.Document.from_bytes(json.dumps(setup).encode())

    def test_nested_null_scope_roundtrips_without_conflating_absent_empty_or_unassigned(self):
        for value in [{"kinds":["space"]}, {"kinds":["space"],"levels":None},
                {"kinds":["space"],"levels":[]}, {"kinds":["space"],"levels":[None]},
                {"kinds":["space"],"levels":["zemin",None,"zemin"]}]:
            self.assertEqual(cad.TagAll.from_json(value).to_json(),value)

    def test_scope_matrix_independent_sources_complete_tags_and_one_undo(self):
        for levels, source_ids, upper in [(['zemin'], [1,3],False), ([None], [2,4,5,6,8,9],False),
                (['zemin',None], [1,2,3,4,5,6,8,9],False), (['zemin','zemin'], [1,3],False),
                (['kat-1'],[2,4],True), (['zemin','kat-1'],[1,2,3,4],True), ([None],[5,6,8,9],True)]:
            with self.subTest(levels=levels):
                doc=self.drawing(upper=upper);before=doc.to_bytes();expected=kcad.read(before)[1]
                al=cad.TagAll(kinds=list(cad.TaggedKind),levels=levels,content='Kat testi',size=3.)
                plan=cad.tags.create.plan(doc,layer_id='etiket',all=al)
                self.assertEqual(doc.to_bytes(),before)
                out=cad.tags.create(doc,layer_id='etiket',all=al,expected_revision=plan.revision)
                self.assertEqual(len(out.uids),len(source_ids))
                anchors={1:(5.,0.),2:(5.,5.),3:(22.,1.5),4:(26.5,1.),5:(40.,0.),6:(45.,0.),8:(50.,3.),9:(65.,0.)}
                for uid, id_, source in zip(out.uids,out.ids,source_ids):
                    x,y=anchors[source];p=dict(x=x,y=y)
                    entity=dict(kind='tag',id=id_,layerId='etiket',attrs={},of=expected['uids'][source-1],
                                at=dict(x=x,y=y+(0. if source in [3,4] else .6)),anchor=p,
                                content='Kat testi',size=3.,text='Kat testi')
                    if source in [3,4]:entity['shape']='rectangle'
                    expected['uids'].append(uid);expected['entities'].append(entity)
                after=doc.to_bytes();self.assertEqual(kcad.read(after)[1],expected)
                doc.undo();self.assertEqual(doc.to_bytes(),before)
                doc.redo();self.assertEqual(doc.to_bytes(),after)
                with tempfile.TemporaryDirectory() as folder:
                    path=Path(folder)/'levels.kcad';doc.save(path)
                    self.assertEqual(cad.Document.open(path).to_bytes(),after)

    def test_missing_floor_atomic_even_with_explicit_tags_and_unknown_host_excluded(self):
        doc=self.drawing();before=doc.to_bytes();room=list(doc.entities())[2].uid
        for levels in [[],['kat-1'],['missing'],[None,'missing']]:
            out=cad.tags.create.answer(doc,layer_id='etiket',all=cad.TagAll(kinds=list(cad.TaggedKind),levels=levels))
            self.assertFalse(out.ok);self.assertEqual(doc.to_bytes(),before)
            self.assertEqual(out.error.code,'level_not_found' if 'missing' in levels else 'no_objects')
        out=cad.tags.create.answer(doc,layer_id='etiket',tags=[cad.NewTag(of=room)],
            all=cad.TagAll(kinds=[cad.TaggedKind.SPACE],levels=['missing']))
        self.assertFalse(out.ok);self.assertEqual(out.error.code,'level_not_found')
        self.assertEqual(doc.to_bytes(),before)

    def test_default_null_same_and_explicit_tags_independent(self):
        doc=self.drawing()
        args=dict(layer_id='etiket')
        a=cad.tags.create.plan(doc,**args,all=cad.TagAll(kinds=[cad.TaggedKind.SPACE]))
        b=cad.tags.create.plan(doc,**args,all=cad.TagAll(kinds=[cad.TaggedKind.SPACE],levels=None))
        self.assertEqual(a,b)
        room=list(doc.entities())[3].uid
        out=cad.tags.create(doc,**args,tags=[cad.NewTag(of=room)],
            all=cad.TagAll(kinds=[cad.TaggedKind.SPACE],levels=[]))
        self.assertEqual(len(out.uids),1)

    def test_refresh_only_selected_floor_preserves_manual_place_and_other_tags(self):
        doc=self.drawing();rooms=[r.uid for r in doc.entities() if r.entity.to_json()['kind']=='space']
        cad.tags.create(doc,layer_id='etiket',tags=[cad.NewTag(of=u,at=cad.Vec2(-4.,8.+i),content='{name}',leader=True)for i,u in enumerate(rooms)])
        before=doc.to_bytes();expected=kcad.read(before)[1]
        out=cad.tags.create(doc,layer_id='etiket',all=cad.TagAll(kinds=[cad.TaggedKind.SPACE],levels=['zemin'],refresh=True,content='{number}'))
        self.assertEqual(len(out.refreshed),1);self.assertFalse(out.uids)
        for row in expected['entities']:
            if row['kind']=='tag' and row['of']==rooms[0]:row.update(content='{number}',text='101')
        self.assertEqual(kcad.read(doc.to_bytes())[1],expected)
        doc.undo();self.assertEqual(doc.to_bytes(),before)
