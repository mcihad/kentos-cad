"""CAD-ADV-06: attribute identity migrations never overwrite instance values."""
from pathlib import Path
import unittest
from kentos import cad
ROOT=Path(__file__).resolve().parents[2]

def drawing():
    d=cad.Document.from_bytes((ROOT/'fixtures/interaction/v1/blocks.kcad').read_bytes())
    block=next(b for b in d.blocks() if b.name=='Rögar')
    a=dict(tag='A',value='default-A',height=.3,rotation=0,p=dict(x=0,y=0),movable=True)
    b=dict(tag='B',value='default-B',height=.4,rotation=0,p=dict(x=1,y=0),movable=True)
    cad.blocks.edit(d,operation='attributes',block=block.id,attributes=[cad.AttributeDefinition.from_json(a),cad.AttributeDefinition.from_json(b)])
    for i,r in enumerate(list(d.entities())):
        if r.entity.to_json().get('block')==block.id:
            cad.entities.set(d,uids=[r.uid],operation='attributes',attrs={'A':f'alpha-{i}','B':f'beta-{i}'})
    return d,block.id,a,b

class BlockAttributeRenames(unittest.TestCase):
    def test_values_and_custom_places_swap_simultaneously_and_undo_exactly(self):
        d,bid,a,b=drawing()
        target=next(r for r in d.entities() if r.entity.to_json().get('block')==bid)
        cad.blocks.edit(d,operation='place',block=bid,uids=[target.uid],tag='A',at=cad.Vec2(x=487011,y=4420002))
        cad.blocks.edit(d,operation='place',block=bid,uids=[target.uid],tag='B',at=cad.Vec2(x=487012,y=4420003))
        before=d.to_bytes()
        cad.blocks.edit.plan(d,operation='sync',block=bid,renames={'A':'B','B':'A'})
        self.assertEqual(d.to_bytes(),before)
        cad.blocks.edit(d,operation='sync',block=bid,renames={'A':'B','B':'A'})
        old=cad.Document.from_bytes(before)
        for previous,now in zip(old.entities(),d.entities()):
            p=previous.entity.to_json();n=now.entity.to_json()
            if p.get('block')==bid:
                self.assertEqual(n['attrs']['A'],p['attrs']['B']);self.assertEqual(n['attrs']['B'],p['attrs']['A'])
                if 'attributePlaces' in p:
                    self.assertEqual(n['attributePlaces']['A'],p['attributePlaces']['B'])
                    self.assertEqual(n['attributePlaces']['B'],p['attributePlaces']['A'])
        raw=d.to_bytes();self.assertEqual(cad.Document.from_bytes(raw).to_bytes(),raw)
        d.undo();self.assertEqual(d.to_bytes(),before)

    def test_unrelated_target_value_refuses_validate_plan_execute_without_data_loss(self):
        d,bid,a,b=drawing();before=d.to_bytes()
        input=cad.BlocksEdit(operation='attributes',block=bid,attributes=[cad.AttributeDefinition.from_json(dict(a,tag='B'))],renames={'A':'B'})
        for op in ['validate','plan','execute']:
            result=cad.blocks.edit.run(d,input,op=op)
            self.assertEqual(result.error.code,'duplicate_tag');self.assertIn('üzerine yazılmadı',result.error.message)
            self.assertEqual(d.to_bytes(),before)

    def test_definition_rename_preserves_values_defaults_and_single_undo(self):
        d,bid,a,b=drawing();before=d.to_bytes();a=dict(a,tag='KOD',value='new-default')
        cad.blocks.edit(d,operation='attributes',block=bid,attributes=[cad.AttributeDefinition.from_json(a),cad.AttributeDefinition.from_json(b)],renames={'A':'KOD'})
        for previous,now in zip(cad.Document.from_bytes(before).entities(),d.entities()):
            p=previous.entity.to_json();n=now.entity.to_json()
            if p.get('block')==bid:
                self.assertEqual(n['attrs']['KOD'],p['attrs']['A']);self.assertNotIn('A',n['attrs']);self.assertEqual(n['attrs']['B'],p['attrs']['B'])
        d.undo();self.assertEqual(d.to_bytes(),before)

    def test_explicit_sync_keeps_its_existing_target_replacement_contract(self):
        d,bid,a,b=drawing();before=d.to_bytes()
        cad.blocks.edit(d,operation='sync',block=bid,renames={'A':'B'})
        for previous,now in zip(cad.Document.from_bytes(before).entities(),d.entities()):
            p=previous.entity.to_json();n=now.entity.to_json()
            if p.get('block')==bid:
                self.assertEqual(n['attrs']['B'],p['attrs']['A']);self.assertNotIn('A',n['attrs'])
        d.undo();self.assertEqual(d.to_bytes(),before)
