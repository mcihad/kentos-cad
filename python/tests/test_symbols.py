"""Mimari işaretler through the public Python host (docs/adr/0721 §3–§5): the north arrow (the library's block added
with it; the true north refused in a local project), a revision's cloud and delta with its date and description, the
Revizyon listesi's table the document keeps, and the material legend of the walls; the file says the list's columns
(schema 122, read by the independent KCAD reader)."""
from pathlib import Path
import sys
import tempfile
import unittest

from kentos import cad
from kentos.cad.errors import CommandFailed

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools/kcad'))
import kcad  # Independent saved-file reader.

PLAN = (b'{"format":"kentos.document","version":1,"name":"Plan","settings":{"srid":0,"lengthDecimals":3,'
        b'"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100.0,"workspace":"cad"},'
        b'"origin":{"x":0.0,"y":0.0},"layers":[{"id":"plan","name":"Plan","type":"layer",'
        b'"visible":true,"locked":false,"expanded":true,"style":{"color":"fg","lineType":"continuous",'
        b'"lineWeight":0.25},"children":[]}],"activeLayer":"plan","entities":[],"styles":{"items":[],"categories":[]}}')


def entity(doc, uid):
    return doc.entity(uid).entity


class Symbols(unittest.TestCase):
    def plan(self):
        doc = cad.Document.from_bytes(PLAN)
        cad.walls.create(doc, layer_id='plan', walls=[
            cad.NewWall(pts=[cad.Vec2(x=0, y=0), cad.Vec2(x=8, y=0)], style='İç duvar 15')])
        return doc

    def test_a_north_arrow_comes_with_its_block_and_a_local_project_has_no_true_north(self):
        doc = self.plan()
        made = cad.north.create(doc, at=cad.Vec2(x=9, y=4), layer_id='plan')
        arrow = entity(doc, made.uid)
        self.assertTrue(made.added)
        self.assertEqual((arrow.p.x, arrow.p.y, arrow.rotation), (9.0, 4.0, 0.0))
        self.assertEqual(next(b.name for b in doc.blocks() if b.id == arrow.block), 'Kuzey oku')
        # 15 mm across at 1:100: 1.5 m, the library's arrow 1 m across.
        self.assertAlmostEqual(arrow.scale, 1.5)
        with self.assertRaises(CommandFailed) as true_north:
            cad.north.create(doc, at=cad.Vec2(x=9, y=6), layer_id='plan', north=cad.TrueNorthDirection())
        self.assertEqual(true_north.exception.code, 'no_true_north')

    def test_a_revision_and_its_list_and_the_material_legend(self):
        doc = self.plan()
        marked = cad.revisions.mark(doc, layer_id='plan', frame=cad.RectangleRevcloudFrame(
            a=cad.Vec2(x=2, y=-2), b=cad.Vec2(x=5, y=-0.5)), arc=0.3, number='R1', date='2026-10-27',
            description='Duvar uzatıldı')
        delta = entity(doc, marked.tag)
        self.assertEqual((delta.attrs['REV'], delta.attrs['Tarih'], delta.attrs['Açıklama']),
                         ('R1', '2026-10-27', 'Duvar uzatıldı'))
        self.assertEqual(entity(doc, marked.cloud).attrs['Revizyon'], 'R1')
        # Revizyon listesi: a block schedule of the deltas by REV with Tarih and Açıklama; the document writes it.
        made = cad.entities.create(doc, layer_id='plan', operation='table', objects=[cad.NewObject(
            geometry=cad.TableEntityGeometry(p=cad.Vec2(x=10, y=0), rotation=0.0, height=0.25, rows=[0.5],
                                             columns=[1.0], cells=[['x']], header=True,
                                             source=cad.BlocksTableSource(names=['Revizyon'], by='REV',
                                                                          columns=['Tarih', 'Açıklama'])))])
        table = made.created[0]
        self.assertEqual(entity(doc, table).cells, [['REV', 'Tarih', 'Açıklama', 'Adet'],
                                                    ['R1', '2026-10-27', 'Duvar uzatıldı', '1'],
                                                    ['Toplam', '', '', '1']])
        legend = cad.legends.materials(doc, at=cad.Vec2(x=-3, y=4), layer_id='plan')
        self.assertEqual([(m.name, m.walls) for m in legend.materials], [('Sıva', 1), ('Tuğla', 1)])
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'isaretler.kcad'
            doc.save(path)
            data = path.read_bytes()
            self.assertEqual(kcad.decode_payload(kcad.read_header(data)[1])['version'], 122)
            content = kcad.read(data)[1]
            sources = [e['source'] for e in content['entities'] if e['kind'] == 'table']
            self.assertEqual(sources, [{'kind': 'blocks', 'names': ['Revizyon'], 'by': 'REV',
                                        'columns': ['Tarih', 'Açıklama']}])
            back = cad.Document.open(path)
            self.assertEqual(entity(back, table).cells[1], ['R1', '2026-10-27', 'Duvar uzatıldı', '1'])


if __name__ == '__main__':
    unittest.main()
