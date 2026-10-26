"""Room elevations through public commands; independent Decimal expectations."""
import copy
from decimal import Decimal, ROUND_HALF_UP
import json
from pathlib import Path
import tempfile
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
CONTENT = '{number} {name}\n{area}\nZ {floorElevation}\nT {ceilingElevation}'


class TagElevations(unittest.TestCase):
    def drawing(self, unit='m', height=2.75, level='zemin'):
        setup = json.loads((ROOT/'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
        setup['settings']['drawingUnit'] = unit
        setup['settings']['arch']['levels'][0]['elevation'] = -3.125
        room = copy.deepcopy(setup['entities'][2])
        room['id'] = 1
        room['arch'] = dict(level=level) if level else {}
        if height is not None:
            room['height'] = height
        setup['entities'] = [room]
        doc = cad.Document.from_bytes(json.dumps(setup).encode())
        source = next(doc.entities()).uid
        args = dict(layer_id='etiket', tags=[cad.NewTag.from_json(dict(
            of=source, at=dict(x=-5,y=8), content=CONTENT, leader=True))])
        original = doc.to_bytes()
        planned = cad.tags.create.plan(doc, **args)
        self.assertEqual(doc.to_bytes(), original)
        tag = cad.tags.create(doc, **args, expected_revision=planned.revision).uids[0]
        stored = doc.entity(tag).entity.to_json()
        expected = planned.tags[0].to_json()
        expected["id"] = stored["id"]  # A plan has slot zero until inserted.
        self.assertEqual(stored, expected)
        return doc, source, tag

    def rows(self, doc):
        return {v.uid:v.entity.to_json() for v in doc.entities()}

    def expect_text(self, unit, elevation, height, decimals=3):
        divisor = dict(m='1',mm='.001',cm='.01',**{'in':'.0254','ft':'.3048'})[unit]
        def number(v):
            return str((v/Decimal(divisor)).quantize(Decimal(1).scaleb(-decimals), rounding=ROUND_HALF_UP))
        floor = number(Decimal(str(elevation)))
        ceiling = number(Decimal(str(elevation))+Decimal(str(height))) if height is not None else ''
        return f'101 Salon\n12.00 m²\nZ {floor}\nT {ceiling}'.rstrip()

    def test_twenty_units_heights_and_saved_source_identity(self):
        for unit in ['m','mm','cm','in','ft']:
            for height in [.125,2.75,3.125,12.5]:
                with self.subTest(unit=unit,height=height):
                    doc, source, tag = self.drawing(unit,height)
                    rows = self.rows(doc)
                    self.assertEqual(rows[tag]['text'],self.expect_text(unit,-3.125,height))
                    self.assertEqual(rows[tag]['of'],source)
                    with tempfile.TemporaryDirectory() as folder:
                        path=Path(folder)/'elevation.kcad';doc.save(path)
                        self.assertEqual(self.rows(cad.Document.open(path)),rows)

    def test_unknown_height_is_not_floor_to_floor_and_unknown_level_not_zero(self):
        doc, _, tag = self.drawing(height=None)
        self.assertEqual(self.rows(doc)[tag]['text'],self.expect_text('m',-3.125,None))
        for level in [None,'missing']:
            doc, _, tag = self.drawing(level=level)
            self.assertEqual(self.rows(doc)[tag]['text'],'101 Salon\n12.00 m²\nZ\nT')

    def test_source_height_follow_move_single_history_and_cancel(self):
        doc, source, tag = self.drawing()
        cad.entities.transform(doc,uids=[tag],transform=cad.MoveTransform(dx=-2,dy=3))
        before = self.rows(doc)
        args=dict(uids=[source],operation=cad.SetSpacesOperation(height=3.125))
        plan=cad.spaces.edit.plan(doc,**args)
        self.assertEqual(self.rows(doc),before)
        cad.spaces.edit(doc,**args,expected_revision=plan.revision)
        after=self.rows(doc)
        self.assertEqual(after[tag]['text'],self.expect_text('m',-3.125,3.125))
        for key in ['at','anchor','of','content','leader']:
            self.assertEqual(after[tag][key],before[tag][key])
        doc.undo();self.assertEqual(self.rows(doc),before)
        doc.redo();self.assertEqual(self.rows(doc),after)
        with self.assertRaisesRegex(RuntimeError,'cancel'):
            with doc.group('Cancel elevation edit'):
                cad.spaces.edit(doc,uids=[source],operation=cad.SetSpacesOperation(height=2.))
                raise RuntimeError('cancel')
        self.assertEqual(self.rows(doc),after)

    def test_source_level_change_uses_its_assigned_level_and_single_undo(self):
        doc, source, tag = self.drawing()
        before = self.rows(doc)
        cad.spaces.edit(doc,uids=[source],operation=cad.SetSpacesOperation(level='kat-1'))
        self.assertEqual(self.rows(doc)[tag]['text'],self.expect_text('m',3.,2.75))
        self.assertEqual(self.rows(doc)[tag]['at'],before[tag]['at'])
        doc.undo();self.assertEqual(self.rows(doc),before)

    def test_settings_and_later_undo_use_current_elevation_and_unit(self):
        doc,source,tag=self.drawing()
        cad.spaces.edit(doc,uids=[source],operation=cad.SetSpacesOperation(height=3.125))
        before=self.rows(doc)
        arch=doc.settings.to_json()['arch']
        arch['levels'][0]['elevation']=12.375
        cad.settings.set(doc,arch=cad.ArchSettings.from_json(arch),drawing_unit='mm',length_decimals=2)
        after=self.rows(doc)
        self.assertEqual(after[source],before[source])
        self.assertEqual(after[tag]['text'],self.expect_text('mm',12.375,3.125,2))
        self.assertEqual(after[tag]['at'],before[tag]['at'])
        doc.undo()  # Settings remain as documented; source height undo still must refresh words.
        self.assertEqual(self.rows(doc)[tag]['text'],self.expect_text('mm',12.375,2.75,2))
        doc.redo();self.assertEqual(self.rows(doc),after)
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'current-settings.kcad';doc.save(path)
            self.assertEqual(self.rows(cad.Document.open(path)),after)

    def test_source_deleted_keeps_last_words_and_tag_edit_retains_position(self):
        doc,source,tag=self.drawing()
        before=self.rows(doc)
        cad.tags.edit(doc,uids=[tag],change=cad.TagChange.from_json(dict(content='{floorElevation}/{ceilingElevation}')))
        after=self.rows(doc)
        self.assertEqual(after[tag]['text'],'-3.125/-0.375')
        self.assertEqual(after[tag]['at'],before[tag]['at'])
        doc.undo();self.assertEqual(self.rows(doc),before)
        cad.entities.delete(doc,uids=[source])
        broken=self.rows(doc)[tag]
        self.assertEqual(broken['text'],before[tag]['text'])
        self.assertEqual(broken['broken'],'targetGone')
        doc.undo();self.assertEqual(self.rows(doc),before)
