"""Project suggestions stay advisory through the public Python host and saved files."""
import copy
from dataclasses import replace
import json
from pathlib import Path
import sys
import tempfile
import unittest

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/kcad'))
import kcad  # Independent saved-file reader.


class RoomChoices(unittest.TestCase):
    def drawing(self):
        setup = json.loads((ROOT / 'fixtures/commands/v1/cad.tags.create.json').read_text())['setup']
        return cad.Document.from_bytes(json.dumps(setup).encode())

    def choices(self, doc):
        return replace(doc.info().settings.arch,
                       space_names=['Mutfak', 'Çalışma', 'mutfak'],
                       space_usages=['Konut', 'Ofis'],
                       space_number_prefixes={'zemin': 'G-', 'kat-1': '01-'})

    def test_preferences_change_only_arch_and_keep_existing_source_identity(self):
        doc = self.drawing()
        before = kcad.read(doc.to_bytes())[1]
        arch = self.choices(doc)
        self.assertEqual(cad.ArchSettings.from_json(arch.to_json()), arch)
        for operation in [cad.settings.set.plan, cad.settings.set.validate]:
            operation(doc, arch=arch)
            self.assertEqual(kcad.read(doc.to_bytes())[1], before)
        cad.settings.set(doc, arch=arch)
        expected = copy.deepcopy(before)
        expected['settings']['arch'] = arch.to_json()
        self.assertEqual(kcad.read(doc.to_bytes())[1], expected)
        self.assertEqual(cad.settings.set(doc, arch=arch).changed, [])
        # An advisory list never coerces the original room's name/usage/number.
        self.assertEqual(list(doc.entities())[0].entity.to_json(), before['entities'][0])

    def test_geometry_undo_keeps_current_preferences_and_single_edit_updates_tag(self):
        doc = self.drawing()
        room = next(r.uid for r in doc.entities() if r.entity.to_json()['kind'] == 'space')
        tag = cad.tags.create(doc, layer_id='etiket', tags=[cad.NewTag(of=room)]).uids[0]
        cad.settings.set(doc, arch=self.choices(doc))
        before = doc.to_bytes()
        # Values outside both suggestion lists remain valid free text.
        cad.spaces.edit(doc, uids=[room], operation=cad.SetSpacesOperation(
            name='Serbest çalışma', usage='Atölye', number='G-001'))
        edited = doc.to_bytes()
        self.assertIn('Serbest çalışma', doc.entity(tag).entity.text)
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)
        doc.redo()
        self.assertEqual(doc.to_bytes(), edited)
        doc.undo()
        doc.undo()  # The tag creation, not an intervening preference edit.
        self.assertNotIn(tag, [r.uid for r in doc.entities()])
        self.assertEqual(doc.info().settings.arch, self.choices(doc))

    def test_invalid_lists_or_floor_bindings_are_atomic_in_all_operations(self):
        doc = self.drawing()
        before = doc.to_bytes()
        for key, value in [
            ('spaceNames', ['Mutfak', 'Mutfak']), ('spaceNames', [' ']),
            ('spaceUsages', ['Konut\nOfis']), ('spaceUsages', ['x'] * 257),
            ('spaceNumberPrefixes', {'missing': 'X-'}),
            ('spaceNumberPrefixes', {'zemin': 'Ö' * 254}),
        ]:
            arch = self.choices(doc).to_json()
            arch[key] = value
            args = cad.SettingsSet.from_json(dict(arch=arch))
            for op in ['validate', 'plan', 'execute']:
                with self.subTest(key=key, value=value, op=op):
                    answer = cad.settings.set.run(doc, args, op=op)
                    self.assertFalse(answer.ok)
                    self.assertEqual(answer.error.code, 'invalid_arch')
                    self.assertTrue(answer.error.path.startswith('arch.'))
                    self.assertEqual(doc.to_bytes(), before)

    def test_real_kcad_keeps_lists_order_and_unlisted_source_values(self):
        doc = self.drawing()
        cad.settings.set(doc, arch=self.choices(doc))
        expected = {r.uid: r.entity.to_json() for r in doc.entities()}
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'room-choices.kcad'
            doc.save(path)
            _, payload = kcad.read_header(path.read_bytes())
            self.assertEqual(kcad.decode_payload(payload)['version'], 107)
            back = cad.Document.open(path)
            self.assertEqual(back.info().settings.arch, doc.info().settings.arch)
            self.assertEqual({r.uid: r.entity.to_json() for r in back.entities()}, expected)

    def test_dxf_explicitly_reports_project_preference_loss_and_keeps_its_old_drawing_bytes(self):
        doc = self.drawing()
        with tempfile.TemporaryDirectory() as folder:
            original = Path(folder) / 'original.dxf'
            old_report = doc.export_dxf(original).report
            cad.settings.set(doc, arch=self.choices(doc))
            before = doc.to_bytes()
            exported = doc.export_dxf(Path(folder) / 'choices.dxf')
            warnings = [n for n in exported.report['notes'] if n['what'] == 'Mahal proje tercihleri']
            self.assertEqual(len(warnings), 1)
            self.assertEqual(warnings[0]['severity'], 'warning')
            self.assertIn('KCAD', warnings[0]['reason'])
            self.assertFalse(any(n['what'] == 'Mahal proje tercihleri' for n in old_report['notes']))
            self.assertEqual(exported.path.read_bytes(), original.read_bytes())
            self.assertEqual(doc.to_bytes(), before)
            back = cad.Document.open(exported.path)
            self.assertNotIn('arch', back.info().settings.to_json())
            self.assertEqual([(r.entity.name, r.entity.usage, r.entity.number) for r in back.entities()
                              if r.entity.kind == 'space'],
                             [(r.entity.name, r.entity.usage, r.entity.number) for r in doc.entities()
                              if r.entity.kind == 'space'])

    def test_lists_can_be_cleared_without_erasing_other_architectural_settings(self):
        doc = self.drawing()
        initial = doc.info().settings.arch
        original_bytes = doc.to_bytes()
        before = {r.uid: r.entity for r in doc.entities()}
        cad.settings.set(doc, arch=self.choices(doc))
        cad.settings.set(doc, arch=initial)
        self.assertEqual(doc.info().settings.arch, initial)
        self.assertEqual({r.uid: r.entity for r in doc.entities()}, before)
        self.assertNotIn('spaceNames', kcad.read(doc.to_bytes())[1]['settings']['arch'])
        self.assertEqual(doc.to_bytes(), original_bytes)
