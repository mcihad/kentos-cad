"""Calculated polar values remain points and every instance stays saveable."""
from pathlib import Path
import unittest
from kentos import cad
ROOT = Path(__file__).resolve().parents[2]


class BlockFormulaValues(unittest.TestCase):
    def source(self):
        doc = cad.Document.open(ROOT / 'fixtures/kcad/v2/block-parameter-groups.kcad')
        return doc, doc.blocks()[0]

    def dynamic(self, block, expression='@Genişlik * 2 + 0.1'):
        d = block.to_json()['dynamic']
        d['parameters'].append(dict(kind='polar', name='Uç', from_=dict(x=10., y=20.),
                                    to=dict(x=13., y=24.), rule=dict(expression=expression)))
        d['parameters'][-1]['from'] = d['parameters'][-1].pop('from_')
        return cad.DynamicBlock.from_json(d)

    def test_guarded_polar_formula_populates_default_instances_saves_and_undoes(self):
        doc, block = self.source()
        before = doc.to_bytes()
        records = list(doc.entities())
        args = dict(operation='dynamic', block=block.id, dynamic=self.dynamic(block))
        plan = cad.blocks.edit.answer(doc, **args, op='plan')
        self.assertEqual(plan.status, 'completed')
        self.assertEqual(doc.to_bytes(), before)
        cad.blocks.edit(doc, **args, expected=cad.Expected(plan=plan.plan_id))
        after = doc.to_bytes()
        self.assertEqual(cad.Document.from_bytes(after).to_bytes(), after)
        for uid, x, y in ((records[0].uid, 11.14, 21.52), (records[1].uid, 11.26, 21.68)):
            values = cad.blocks.parameters(doc, operation='get', uids=[uid]).inserts[0].values
            self.assertAlmostEqual(values['Uç']['x'], x)
            self.assertAlmostEqual(values['Uç']['y'], y)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), after)

    def test_bad_existing_instance_fails_without_resetting_width_or_flip(self):
        doc, block = self.source()
        before = doc.to_bytes()
        for op in ('validate', 'plan', 'execute'):
            answer = cad.blocks.edit.answer(doc, operation='dynamic', block=block.id,
                    dynamic=self.dynamic(block, '1 / abs(@Genişlik - 1)'), op=op)
            self.assertEqual(answer.status, 'failed')
            self.assertEqual(answer.error.code, 'invalid_parameter_value')
            self.assertEqual(doc.to_bytes(), before)

    def test_calculated_polar_is_read_only_and_its_inputs_drive_recomputed_points(self):
        doc, block = self.source()
        cad.blocks.edit(doc, operation='dynamic', block=block.id, dynamic=self.dynamic(block))
        uid = list(doc.entities())[1].uid
        before = doc.to_bytes()
        refused = cad.blocks.parameters.answer(doc, operation='set', uids=[uid], values={'Uç': {'x': 12., 'y': 22.}})
        self.assertEqual(refused.error.code, 'derived_parameter')
        self.assertEqual(doc.to_bytes(), before)
        cad.blocks.parameters(doc, operation='set', uids=[uid], values={'Genişlik': 1.05})
        values = cad.blocks.parameters(doc, operation='get', uids=[uid]).inserts[0].values
        self.assertAlmostEqual(values['Uç']['x'], 11.32)
        self.assertAlmostEqual(values['Uç']['y'], 21.76)
        self.assertEqual(cad.Document.from_bytes(doc.to_bytes()).to_bytes(), doc.to_bytes())
