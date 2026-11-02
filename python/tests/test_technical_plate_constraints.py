"""CAD-ADV-05 combined acceptance: tangent, equality, driving/reference and partial grip."""
import math
import unittest
from pathlib import Path
from kentos import cad

ROOT=Path(__file__).resolve().parents[2]
BASE=(ROOT/'fixtures/interaction/v1/constraint-technical-plate.kcad').read_bytes()

class TechnicalPlateConstraints(unittest.TestCase):
    def drawing(self):
        return cad.Document.from_bytes(BASE)

    def intact(self,doc,count=24,dof=4):
        q=cad.constraints.query(doc)
        self.assertEqual(len(q.constraints),count)
        self.assertTrue(all(c.holds for c in q.constraints))
        self.assertEqual(q.diagnosis.dof,dof)
        self.assertFalse(q.diagnosis.conflicting)
        self.assertFalse(q.diagnosis.broken)
        raw=doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(raw).to_bytes(),raw)
        return [r.entity.to_json() for r in doc.entities()]

    def test_drivers_follow_reference_equality_and_tangent_without_branch_jump(self):
        for name,value in [('delik_yaricapi','1.5'),('taban','7'),('delik_araligi','8')]:
            with self.subTest(parameter=name):
                d=self.drawing();edit=cad.ParametersEdit(edits=[cad.SetParameterEdit(name=name,expression=value)])
                cad.parameters.edit.plan_input(d,edit);self.assertEqual(d.to_bytes(),BASE)
                cad.parameters.edit.execute_input(d,edit)
                es=self.intact(d);left,right,guide=es[4:7]
                self.assertAlmostEqual(left['r'],right['r'],places=9)
                self.assertGreater(right['c']['x'],left['c']['x'])
                self.assertGreater(guide['a']['y'],left['c']['y'],'upper tangent must remain upper')
                self.assertAlmostEqual(guide['a']['y']-left['c']['y'],left['r'],places=8)
                self.assertAlmostEqual(guide['a']['y'],guide['b']['y'],places=8)
                self.assertAlmostEqual(guide['a']['x'],left['c']['x'],places=8)
                self.assertAlmostEqual(guide['b']['x'],right['c']['x'],places=8)
                self.assertAlmostEqual(es[0]['b']['x']-es[0]['a']['x'],14 if name=='taban' else 12,places=8)
                q=cad.constraints.query(d)
                ref=next(c for c in q.constraints if c.constraint.rule.to_json().get('parameter')=='kosegen')
                self.assertTrue(ref.constraint.reference)
                self.assertAlmostEqual(ref.measured,math.hypot(14 if name=='taban' else 12,8),places=8)
                d.undo();self.assertEqual(d.to_bytes(),BASE)

    def test_partial_grip_preserves_the_independent_plate_and_permanent_diagnosis(self):
        d=self.drawing();before=[r.entity.to_json() for r in d.entities()]
        change=cad.EllipseAxisEntityEdit(uid=list(d.entities())[7].uid,index=1,at=cad.Vec2(x=487019,y=4420007))
        cad.entities.edit.plan(d,operation='grip',changes=[change]);self.assertEqual(d.to_bytes(),BASE)
        cad.entities.edit(d,operation='grip',changes=[change]);es=self.intact(d)
        self.assertEqual(es[:7],before[:7]);self.assertNotEqual(es[8]['a'],before[8]['a'])
        self.assertAlmostEqual(math.hypot(es[7]['major']['x'],es[7]['major']['y'])*es[7]['ratio'],3,places=8)
        d.undo();self.assertEqual(d.to_bytes(),BASE)

    def test_lock_conflict_and_explicit_relax_remain_atomic_and_one_undo(self):
        d=self.drawing();cad.layers.set(d,operation='locked',layer='cizim',value=True);locked=d.to_bytes()
        with self.assertRaises(cad.CommandFailed) as failure:
            cad.parameters.edit(d,edits=[cad.SetParameterEdit(name='delik_yaricapi',expression='1.5')])
        self.assertEqual(failure.exception.code,'layer_locked');self.assertEqual(d.to_bytes(),locked)
        cad.layers.set(d,operation='locked',layer='cizim',value=False)
        self.assertEqual(d.to_bytes(),BASE)
        first=list(d.entities())[0];g=first.entity.to_json();g={k:g[k] for k in ['kind','a','b']};g['a']={'x':487001.,'y':4420000.}
        change=cad.UpdateEntityEdit.from_json(dict(kind='update',uid=first.uid,geometry=g))
        with self.assertRaises(cad.CommandFailed) as failure:cad.entities.edit(d,operation='grip',changes=[change])
        self.assertEqual(failure.exception.code,'constraint_conflict');self.assertEqual(d.to_bytes(),BASE)
        with d.relaxed(),self.assertWarns(cad.CommandWarning):cad.entities.edit(d,operation='grip',changes=[change])
        es=self.intact(d,count=21,dof=9)
        self.assertEqual(es[0]['a'],g['a']);self.assertEqual(es[1:],[r.entity.to_json() for r in self.drawing().entities()][1:])
        d.undo();self.assertEqual(d.to_bytes(),BASE)
