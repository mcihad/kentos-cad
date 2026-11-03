"""Shared type values through the guarded Python/AI surface, not instance overrides."""
from pathlib import Path
import unittest
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]
class BlockTypeValues(unittest.TestCase):
    def drawing(self):
        doc=cad.Document.open(ROOT/'fixtures/kcad/v2/block-type-values.kcad')
        return doc,doc.blocks()[0]
    def test_guarded_common_edit_nested_roundtrip_and_exact_undo(self):
        doc,block=self.drawing();before=doc.to_bytes();records=[r.entity.to_json() for r in doc.entities()]
        dynamic=block.to_json()['dynamic'];dynamic['typeValues']['Genişlik']=1.2
        args=dict(operation='dynamic',block=block.id,dynamic=cad.DynamicBlock.from_json(dynamic))
        plan=cad.blocks.edit.answer(doc,**args,op='plan');self.assertEqual(plan.status,'completed');self.assertEqual(doc.to_bytes(),before)
        cad.blocks.edit(doc,**args,expected=cad.Expected(plan=plan.plan_id));after=doc.to_bytes()
        self.assertEqual([r.entity.to_json() for r in doc.entities()],records)
        self.assertEqual(doc.blocks()[0].to_json()['dynamic']['typeValues'],{'Genişlik':1.2})
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(),after)
        doc.undo();self.assertEqual(doc.to_bytes(),before);doc.redo();self.assertEqual(doc.to_bytes(),after)
    def test_common_parameter_and_lookup_refuse_instance_set_and_named_reset(self):
        doc,block=self.drawing();before=doc.to_bytes();uid=next(doc.entities()).uid
        for args in (dict(operation='set',values={'Genişlik':1.0}),dict(operation='set',values={'Tip':'Dar'}),dict(operation='reset',names=['Genişlik']),dict(operation='reset',names=['Tip'])):
            answer=cad.blocks.parameters.answer(doc,uids=[uid],**args);self.assertEqual(answer.error.code,'invalid_parameter_value');self.assertEqual(doc.to_bytes(),before)
    def test_demotion_materializes_every_common_value_and_units_preserve_scope(self):
        doc,block=self.drawing();before=doc.to_bytes();dynamic=block.to_json()['dynamic'];dynamic.pop('typeParameters');dynamic.pop('typeValues')
        cad.blocks.edit(doc,operation='dynamic',block=block.id,dynamic=cad.DynamicBlock.from_json(dynamic))
        self.assertEqual(next(doc.entities()).entity.to_json()['parameters']['Genişlik'],1.1)
        self.assertEqual(doc.blocks()[1].to_json()['entities'][0]['parameters']['Genişlik'],1.1)
        doc.undo();self.assertEqual(doc.to_bytes(),before)
        cad.blocks.edit(doc,operation='units',block=block.id,units='mm',rescale=True)
        d=doc.blocks()[0].to_json()['dynamic'];self.assertEqual(d['typeParameters'],['Genişlik']);self.assertAlmostEqual(d['typeValues']['Genişlik'],.0011)
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(),doc.to_bytes());doc.undo();self.assertEqual(doc.to_bytes(),before)
    def test_changed_type_plan_is_not_authorization_for_another_common_value(self):
        doc,block=self.drawing();before=doc.to_bytes();dynamic=block.to_json()['dynamic'];dynamic['typeValues']['Genişlik']=1.2
        args=dict(operation='dynamic',block=block.id,dynamic=cad.DynamicBlock.from_json(dynamic));plan=cad.blocks.edit.answer(doc,**args,op='plan')
        dynamic['typeValues']['Genişlik']=1.0;args['dynamic']=cad.DynamicBlock.from_json(dynamic)
        a=cad.blocks.edit.answer(doc,**args,expected=cad.Expected(plan=plan.plan_id));self.assertNotEqual(a.status,'completed');self.assertEqual(doc.to_bytes(),before)
    def test_same_publication_ids_do_not_couple_distinct_drawing_types_and_selected_replacement(self):
        doc,source=self.drawing();uid=next(doc.entities()).uid
        target=cad.blocks.edit(doc,operation='duplicate',block=source.id,name='Ayrı çizim tipi').added[0]
        dynamic=source.to_json()['dynamic'];dynamic['typeValues']['Genişlik']=1.2
        cad.blocks.edit(doc,operation='dynamic',block=target,dynamic=cad.DynamicBlock.from_json(dynamic))
        self.assertEqual(doc.blocks()[0].to_json()['dynamic']['typeValues']['Genişlik'],1.1)
        before=doc.to_bytes();cad.blocks.replace(doc,from_=source.id,to=target,uids=[uid])
        self.assertEqual(cad.blocks.parameters(doc,operation='get',uids=[uid]).inserts[0].values['Genişlik'],1.2)
        self.assertEqual(doc.blocks()[0].to_json()['dynamic']['typeValues']['Genişlik'],1.1)
        doc.undo();self.assertEqual(doc.to_bytes(),before)
        # Replacing with an instance-scoped target materializes the old common input.
        dynamic.pop('typeParameters');dynamic.pop('typeValues')
        cad.blocks.edit(doc,operation='dynamic',block=target,dynamic=cad.DynamicBlock.from_json(dynamic))
        cad.blocks.replace(doc,from_=source.id,to=target,uids=[uid])
        self.assertEqual(next(doc.entities()).entity.to_json()['parameters']['Genişlik'],1.1)
    def test_static_dxf_preserves_common_width_through_nested_references_and_reports_parameter_loss(self):
        import tempfile
        doc,block=self.drawing();before=doc.to_bytes()
        proxy=cad.blocks.proxy(doc,uids=[next(doc.entities()).uid]).inserts[0]
        self.assertTrue(any(e.kind=='arc' and abs(e.to_json()['r']-1.1)<1e-12 for e in proxy.entities))
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'common.dxf';plan=cad.file.export.dxf.plan(doc,path=str(path));self.assertFalse(path.exists())
            self.assertTrue(any(n['what']=='Dinamik blok' and n['severity']=='warning' for n in plan.report['notes']))
            result=cad.file.export.dxf(doc,path=str(path));self.assertEqual(result.report,plan.report);self.assertEqual(doc.to_bytes(),before)
            back=cad.Document.open(path)
            self.assertTrue(all(b.dynamic is cad.UNSET or b.dynamic is None for b in back.blocks()))
            definitions={b.id:b for b in back.blocks()}
            def radii(block_id):
                # proxy returns a definition's direct pieces; nested INSERTs retain
                # their identity. Follow that graph instead of assuming flattening.
                for entity in definitions[block_id].entities:
                    data=entity.to_json()
                    if entity.kind=='arc':
                        yield data['r']
                    elif entity.kind=='insert':
                        yield from radii(data['block'])
            for record in back.entities():
                actual=list(radii(record.entity.to_json()['block']))
                self.assertEqual(len(actual),1)
                self.assertAlmostEqual(actual[0],1.1)
    def test_whole_project_units_scale_common_lengths_and_points_but_not_angles_or_flags(self):
        doc,block=self.drawing();dynamic=block.to_json()['dynamic']
        dynamic['parameters'].extend([
            dict(kind='point',name='Konum',at=dict(x=1.,y=2.)),
            dict(kind='angle',name='Açı',center=dict(x=0.,y=0.),base=.3),
        ])
        dynamic['typeParameters'].extend(['Konum','Açı'])
        dynamic['typeValues'].update({'Konum':dict(x=1.,y=2.),'Açı':.3})
        cad.blocks.edit(doc,operation='dynamic',block=block.id,dynamic=cad.DynamicBlock.from_json(dynamic));before=doc.to_bytes()
        cad.units.change(doc,from_='m',to='mm',geometry='scale')
        values=doc.blocks()[0].to_json()['dynamic']['typeValues']
        self.assertAlmostEqual(values['Genişlik'],.0011);self.assertEqual(values['Konum'],dict(x=.001,y=.002));self.assertEqual(values['Açı'],.3)
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(),doc.to_bytes());doc.undo();self.assertEqual(doc.to_bytes(),before)
    def test_selected_publication_upgrade_adopts_target_common_values_and_keeps_old_nested_type(self):
        import json
        doc,block=self.drawing();uid=next(doc.entities()).uid
        library=json.loads(cad.blocks.edit(doc,operation='export').library)
        incoming=next(b for b in library['blocks'] if b['id']==block.id)
        incoming['card']['publication']['release']+=1;incoming['dynamic']['typeValues']['Genişlik']=1.2
        before=doc.to_bytes();old_blocks=[b.to_json() for b in doc.blocks()]
        args=dict(from_=block.id,to=block.id,library=json.dumps(library,ensure_ascii=False),uids=[uid])
        plan=cad.blocks.replace.answer(doc,**args,op='plan');self.assertEqual(plan.status,'completed');self.assertEqual(doc.to_bytes(),before)
        cad.blocks.replace(doc,**args,expected=cad.Expected(plan=plan.plan_id))
        self.assertEqual([b.to_json() for b in doc.blocks()][:2],old_blocks)
        self.assertEqual(cad.blocks.parameters(doc,operation='get',uids=[uid]).inserts[0].values['Genişlik'],1.2)
        self.assertEqual(doc.blocks()[0].to_json()['dynamic']['typeValues']['Genişlik'],1.1)
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(),doc.to_bytes());doc.undo();self.assertEqual(doc.to_bytes(),before)

    def test_library_geometry_metadata_requires_version_three_in_both_import_paths(self):
        import json
        doc, block = self.drawing()
        library = json.loads(cad.blocks.edit(doc, operation='export').library)
        self.assertEqual(library['version'], 3)
        nested = json.loads(cad.blocks.edit(doc, operation='export', blocks=[doc.blocks()[1].id]).library)
        self.assertEqual(nested['version'], 3)
        self.assertEqual(len(nested['blocks']), 2)
        legacy = cad.Document.open(ROOT/'fixtures/kcad/v2/block-publication.kcad')
        self.assertEqual(json.loads(cad.blocks.edit(legacy, operation='export').library)['version'], 2)
        before = doc.to_bytes()
        other = cad.Document.new(workspace='cad')
        cad.blocks.edit(other, operation='import', library=json.dumps(library, ensure_ascii=False))
        self.assertEqual(other.blocks()[0].dynamic.to_json()['typeValues']['Genişlik'], 1.1)
        incoming = next(b for b in library['blocks'] if b['id'] == block.id)
        incoming['card']['publication']['release'] += 1
        library['version'] = 2
        text = json.dumps(library, ensure_ascii=False)
        for op in ['validate', 'plan', 'execute']:
            for command, args in [
                (cad.blocks.edit, dict(operation='import', replace=True, library=text)),
                (cad.blocks.replace, dict(from_=block.id, to=block.id, uids=[next(doc.entities()).uid], library=text)),
            ]:
                answer = command.answer(doc, **args, op=op)
                self.assertEqual(answer.status, 'failed')
                self.assertEqual(answer.error.code, 'invalid_library')
                self.assertIn('3', answer.error.message)
                self.assertEqual(doc.to_bytes(), before)
