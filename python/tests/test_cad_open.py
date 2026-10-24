"""External CAD opens directly; its units are independent of any receiving project."""
from pathlib import Path
import tempfile
import unittest
from kentos import cad

ROOT = Path(__file__).resolve().parents[2]

class CadOpening(unittest.TestCase):
    def test_direct_open_and_native_save_preserve_the_source(self):
        for name, unit in (('dxf-units/mm.dxf', 'mm'), ('dxf-units/inches.dxf', 'in'),
                           ('dwg/mm.dwg', 'mm'), ('ncz/01-her-tur.ncz', 'm')):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as tmp:
                source = ROOT / 'fixtures/formats/v1' / name
                before = source.read_bytes()
                doc = cad.Document.open(source)
                info = doc.info()
                self.assertEqual(info.settings.workspace.value, 'cad')
                self.assertEqual(info.units.drawing.value, unit)
                self.assertIsNone(info.path)
                self.assertFalse(info.can_undo)
                self.assertTrue(info.dirty)
                self.assertIsNotNone(info.opened_cad)
                self.assertEqual(info.opened_cad['path'], str(source))
                self.assertFalse(any('milimetreye çevrildi' in n['reason']
                                     for n in info.opened_cad['report']['notes']))
                with self.assertRaises(cad.HostError):
                    doc.save(source)
                output = Path(tmp) / 'room.kcad'
                doc.save(output)
                back = cad.Document.open(output)
                self.assertEqual(back.info().objects, info.objects)
                self.assertEqual(back.info().units, info.units)
                self.assertEqual(source.read_bytes(), before)

    def test_default_cad_template_is_metric_full_size(self):
        doc = cad.Document.new(workspace='cad')
        self.assertEqual(doc.info().units.drawing.value, 'mm')
        self.assertEqual(doc.info().settings.plot_scale, 1.0)
