"""Room identity review uses the existing read-only tag command and stable UUIDs."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from kentos import cad

ROOT=Path(__file__).resolve().parents[2]

class SpaceNumberCheck(unittest.TestCase):
    def drawing(self):
        cases=json.loads((ROOT/'fixtures/commands/v1/cad.tags.check.json').read_text())['cases']
        setup=next(c['setup']for c in cases if c['name'].startswith('Numarasız mahal opt-in'))
        doc=cad.Document.from_bytes(json.dumps(setup).encode())
        ids={r.entity.id:r.uid for r in doc.entities()}
        untagged=doc.to_bytes()
        cad.tags.create(doc,layer_id='etiket',tags=[cad.NewTag(of=u)for u in ids.values()])
        return doc,ids,untagged

    def findings(self,doc,**args):return [f.to_json()for f in cad.tags.check(doc,**args).findings]

    def test_opt_in_reports_tagged_rooms_and_preserves_the_old_exact_number_groups(self):
        doc,ids,_=self.drawing();before=doc.to_bytes()
        old=self.findings(doc)
        self.assertEqual([f['code']for f in old],['tag_duplicate_number'])
        self.assertEqual(old[0]['uids'],[ids[4],ids[5]])
        for option in [None,False]:self.assertEqual(self.findings(doc,missing_space_numbers=option),old)
        expected=old+[dict(code='tag_missing_space_number',message=f'“{name}” mahalının numarası yok ({level}).',uids=[ids[i]],at=dict(x=5*i+2.,y=1.5))
            for i,name,level in [(1,'Ana giriş','Zemin kat'),(2,'Çalışma','Kat atanmamış'),(3,'Depo','1. kat')]]
        args=dict(missing_space_numbers=True)
        self.assertEqual([f.to_json()for f in cad.tags.check.plan(doc,**args).findings],expected)
        self.assertEqual(cad.tags.check.validate(doc,**args),[])
        self.assertEqual(self.findings(doc,**args),expected)
        self.assertEqual(doc.to_bytes(),before)

    def test_explicit_layers_apply_to_missing_and_duplicate_findings_without_implicit_visibility_filter(self):
        doc,ids,_=self.drawing();before=doc.to_bytes()
        for layers,wanted in [([],[]),(['mahal'],[ids[1],ids[2]]),(['gizli'],[ids[3]]),(['kilitli'],[])]:
            with self.subTest(layers=layers):
                rows=self.findings(doc,layers=layers,missing_space_numbers=True)
                self.assertEqual([u for r in rows for u in r['uids']],wanted)
                self.assertTrue(all(r['code']=='tag_missing_space_number'for r in rows))
        for operation in [cad.tags.check,cad.tags.check.plan,cad.tags.check.validate]:
            with self.assertRaises(cad.CommandFailed)as failed:operation(doc,layers=['yok'],missing_space_numbers=True)
            self.assertEqual(failed.exception.code,'layer_not_found')
        self.assertEqual(doc.to_bytes(),before)

    def test_queries_add_no_history_and_repair_undo_redo_follow_existing_identity_fields(self):
        doc,ids,untagged=self.drawing()
        before={r.uid:r.entity.to_json()for r in doc.entities()}
        for _ in range(3):self.findings(doc,missing_space_numbers=True)
        cad.spaces.edit(doc,uids=[ids[1]],operation=cad.SetSpacesOperation(number='N-001'))
        after={r.uid:r.entity.to_json()for r in doc.entities()}
        self.assertEqual(len(self.findings(doc,missing_space_numbers=True)),3)
        for uid,e in before.items():
            if e['kind']=='space':
                actual=copy.deepcopy(after[uid]);actual.pop('number',None)
                expected=copy.deepcopy(e);expected.pop('number',None);self.assertEqual(actual,expected)
        doc.undo();self.assertEqual({r.uid:r.entity.to_json()for r in doc.entities()},before)
        self.assertEqual(len(self.findings(doc,missing_space_numbers=True)),4)
        doc.redo();self.assertEqual({r.uid:r.entity.to_json()for r in doc.entities()},after)
        doc.undo();doc.undo() # Repair and tag creation, never an intervening check.
        self.assertEqual(doc.to_bytes(),untagged)

    def test_real_kcad_roundtrip_and_units_do_not_normalize_room_numbers(self):
        doc,ids,_=self.drawing();expected=self.findings(doc,missing_space_numbers=True)
        cad.settings.set(doc,drawing_unit='mm',plot_scale=200)
        self.assertEqual(self.findings(doc,missing_space_numbers=True),expected)
        with tempfile.TemporaryDirectory()as folder:
            path=Path(folder)/'identities.kcad';doc.save(path);opened=cad.Document.open(path)
            self.assertEqual(self.findings(opened,missing_space_numbers=True),expected)
        self.assertEqual(doc.entity(ids[7]).entity.to_json()['number'],'Z-001 ')
        self.assertEqual(doc.entity(ids[9]).entity.to_json()['number'],'000000000000000000000000000001')

    def test_blank_text_stays_invalid_but_clearing_the_number_is_reported(self):
        doc,ids,_=self.drawing();before=doc.to_bytes()
        for number in ['', ' \u00a0\t']:
            with self.assertRaises(cad.CommandFailed)as failed:
                cad.spaces.edit(doc,uids=[ids[4]],operation=cad.SetSpacesOperation(number=number))
            self.assertEqual(failed.exception.code,'invalid_space');self.assertEqual(doc.to_bytes(),before)
        cad.spaces.edit(doc,uids=[ids[4]],operation=cad.SetSpacesOperation(number=None))
        rows=self.findings(doc,missing_space_numbers=True)
        self.assertEqual([u for r in rows for u in r['uids']],[ids[1],ids[2],ids[3],ids[4]])
        doc.undo();self.assertEqual(doc.to_bytes(),before)
