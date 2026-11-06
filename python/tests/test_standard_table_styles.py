"""CAD-ADV-12, ADR 1126: an office standard's table styles through the public commands.

The drawing is the shared cases' (fixtures/commands/v1/cad.standards.install.tables.json, written by
scripts/fixtures/standards_table_styles_cases.py): two bound standards' table styles, a differing CETVEL a table
follows, a same SADE, a missing BASLIK whose id the drawing has. Install shows every row, replaces CETVEL in place
(its id kept, so its table follows), adds BASLIK as ofis-baslik-2, and one undo step takes it back; the audit's purge
never removes a table style a table follows.
"""
import json
import unittest
from pathlib import Path

from kentos import cad

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / 'fixtures/commands/v1/cad.standards.install.tables.json').read_text())


class StandardTableStyles(unittest.TestCase):
    def drawing(self):
        return cad.Document.from_bytes(json.dumps(CASES['setup']).encode())

    def rows(self, doc, choices=(), op='plan'):
        install = cad.command('cad.standards.install')
        out = install.run(doc, install.input_type.from_json(dict(choices=list(choices))), op=op).unwrap().to_json()
        return [(r['name'], r['status'], r['action']) for r in out['rows']]

    def unused(self, doc, purge=()):
        audit = cad.command('cad.drawing.audit')
        out = audit.run(doc, audit.input_type.from_json(dict(purge=list(purge)))).unwrap().to_json()
        return sorted((u['id'], u['name']) for u in out['unused'] if u['kind'] == 'tableStyle')

    def test_install_lists_replaces_and_adds_table_styles(self):
        doc = self.drawing()
        cetvel = [dict(kind='tableStyle', name='CETVEL', action='replace')]
        self.assertEqual(self.rows(doc, cetvel), [
            ('CETVEL', 'differs', 'replace'), ('BASLIK', 'missing', 'add'), ('sade', 'same', 'keepLocal'),
            ('PLAN-LISTE', 'missing', 'add')])
        before = doc.to_bytes()
        self.rows(doc, cetvel, op='execute')
        # Installed: the same answer now says every one is the drawing's.
        self.assertEqual([s for _, s, _ in self.rows(doc)], ['same'] * 4)
        self.assertIn(('ofis-baslik-2', 'BASLIK'), self.unused(doc), 'added under a fresh id')
        tables = [r.entity.to_json() for r in doc.entities(kinds=['table'])]
        self.assertEqual(tables[0].get('tableStyle'), 'ofis-cetvel', 'replaced in place: the table still follows it')
        doc.undo()
        self.assertEqual(doc.to_bytes(), before)

    def test_a_used_table_style_is_never_purged(self):
        doc = self.drawing()
        self.assertEqual(self.unused(doc, ['tableStyles']), [('ofis-baslik', 'YEREL')])
        self.assertEqual(self.unused(doc), [], 'purged; the two a table follows stay')
        self.assertEqual([s for _, s, _ in self.rows(doc)], ['differs', 'missing', 'same', 'missing'])


if __name__ == '__main__':
    unittest.main()
