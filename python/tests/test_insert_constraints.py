"""Insertion anchors share UI/Python/MCP units, placement preservation and atomic Undo."""
import json
import unittest
from pathlib import Path
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]


def drawing():
    d = cad.Document.from_bytes((ROOT / 'fixtures/interaction/v1/constraint-block-anchors.kcad').read_bytes())
    uids = [r.uid for r in d.entities()]
    def part(i, place):
        return dict(uid=uids[i], element=dict(kind='point', place=place))
    a = part(0, dict(kind='node', part=0))
    b = part(1, dict(kind='node', part=0))
    l = part(2, dict(kind='start'))
    edit = cad.AddConstraintEdit.from_json(dict(kind='add', constraints=[
        dict(rule=dict(kind='fix', a=a)),
        dict(rule=dict(kind='horizontal', a=a, b=b)),
        dict(rule=dict(kind='distance', a=a, b=b, parameter='yerlesim'), value='6m'),
        dict(rule=dict(kind='coincident', a=b, b=l)),
    ]))
    cad.constraints.edit(d, edit=edit)
    return d, uids


class InsertConstraints(unittest.TestCase):
    def test_anchor_moves_with_transform_and_definition_preserved_and_one_undo(self):
        d, uids = drawing()
        before = d.to_bytes()
        old = d.entity(uids[1]).entity.to_json()
        edits = [cad.SetParameterEdit(name='yerlesim', expression='8m')]
        cad.parameters.edit.plan(d, edits=edits)
        self.assertEqual(d.to_bytes(), before)
        written = cad.parameters.edit(d, edits=edits)
        expected = dict(old, p=dict(x=8., y=0.))
        self.assertEqual(d.entity(uids[1]).entity.to_json(), expected)
        self.assertEqual(d.entity(uids[2]).entity.to_json()['a'], dict(x=8., y=0.))
        q = cad.constraints.query(d).to_json()
        a, b = q['diagnosis'], written.diagnosis.to_json()
        a.pop('iterations', None)
        b.pop('iterations', None)
        self.assertEqual(a, b)
        self.assertEqual(a['dof'], 2)
        self.assertTrue(all(row['holds'] for row in q['constraints']))
        saved = d.to_bytes()
        self.assertEqual(cad.Document.from_bytes(saved).to_bytes(), saved)
        d.undo()
        self.assertEqual(d.to_bytes(), before)

    def test_temporary_first_object_hold_matches_the_persistent_query(self):
        d = cad.Document.from_bytes((ROOT / 'fixtures/interaction/v1/constraint-block-anchors.kcad').read_bytes())
        uids = [r.uid for r in d.entities()]
        def part(i):
            return dict(uid=uids[i], element=dict(kind='point', place=dict(kind='node', part=0)))
        cad.constraints.edit(d, edit=cad.AddConstraintEdit.from_json(dict(kind='add', constraints=[dict(rule=dict(kind='fix', a=part(0)))])))
        before = d.to_bytes()
        edit = cad.AddConstraintEdit.from_json(dict(kind='add', constraints=[dict(rule=dict(kind='coincident', a=part(0), b=part(1)))]))
        planned = cad.constraints.edit.plan(d, edit=edit, hold=[uids[0]])
        self.assertEqual(d.to_bytes(), before)
        written = cad.constraints.edit(d, edit=edit, hold=[uids[0]])
        self.assertEqual(planned.diagnosis.to_json(), written.diagnosis.to_json())
        q = cad.constraints.query(d).diagnosis.to_json()
        q['iterations'] = written.diagnosis.iterations
        self.assertEqual(q, written.diagnosis.to_json())
        self.assertEqual(q['status'], 'fully')
        self.assertNotIn('redundant', q)
        d.undo()
        self.assertEqual(d.to_bytes(), before)

    def test_geometry_update_keeps_moved_attribute_places_and_undo(self):
        d = cad.Document.from_bytes((ROOT / 'fixtures/interaction/v1/constraint-block-anchors.kcad').read_bytes())
        uid = list(d.entities())[1].uid
        before = d.to_bytes()
        old = d.entity(uid).entity.to_json()
        self.assertTrue(old['attributePlaces'])
        geometry = {key: old[key] for key in ['kind', 'block', 'p', 'scale', 'yScale', 'rotation', 'mirror']}
        geometry['p'] = dict(x=8., y=2.)
        changes = [cad.UpdateEntityEdit(uid=uid, geometry=cad.InsertEntityGeometry.from_json(geometry))]
        cad.entities.edit.plan(d, operation='grip', changes=changes)
        self.assertEqual(d.to_bytes(), before)
        cad.entities.edit(d, operation='grip', changes=changes)
        self.assertEqual(d.entity(uid).entity.to_json(), dict(old, p=dict(x=8., y=2.)))
        d.undo()
        self.assertEqual(d.to_bytes(), before)


if __name__ == '__main__':
    unittest.main()
