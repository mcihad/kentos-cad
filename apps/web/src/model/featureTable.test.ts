import { describe, expect, it } from 'vitest';
import type { Entity } from './entities';
import { featureTableModel } from './featureTable';
import type { LayerField } from './layerFields';

/**
 * The attribute table's columns and cells (docs/adr/0199 §4): the same table the desktop's model test reads
 * (crates/native/interaction/src/feature_table.rs): the fields in order by their aliases, then the other keys in the
 * natural order; each cell as its field shows it and its sort key, a refused value as written with why.
 */

const point = (attrs: Record<string, string>): Entity => ({ id: 1, kind: 'point', layerId: 'a', attrs, p: { x: 0, y: 0 } }) as unknown as Entity;

describe('Öznitelik tablosu: model', () => {
  it('has the fields, then the other keys, and shows the cells as the fields do', () => {
    const fields: LayerField[] = [
      { name: 'Kullanım', kind: 'text', values: [{ code: 'K', label: 'Konut' }, { code: 'T', label: 'Ticaret' }] },
      { name: 'Kat', alias: 'Kat sayısı', kind: 'integer' },
      { name: 'Tarih', kind: 'date' },
    ];
    const a = point({ Kat: '+03', Kullanım: 'T', Tarih: '7.10.2026', 'not 10': 'x', 'Not 2': ' ' });
    const b = point({ Kat: '3a', Ada: '101' });
    const t = featureTableModel(fields, [a, b], () => true, () => false, null);
    expect(t.columns.map((c) => c.label)).toEqual(['Tür', 'Kullanım', 'Kat sayısı', 'Tarih', 'Ada', 'Not 2', 'not 10']);
    expect(t.columns.map((c) => c.order)).toEqual(['text', 'text', 'number', 'date', 'text', 'text', 'text']);
    expect(t.rows[0].cells.map((c) => [c.shown, c.key ?? null])).toEqual([
      ['Nokta', 'Nokta'],
      ['Ticaret', 'Ticaret'],
      ['3', '3'],
      ['07.10.2026', '2026-10-07'],
      ['', null],
      [' ', null],
      ['x', 'x'],
    ]);
    expect([t.rows[1].cells[2].shown, t.rows[1].cells[2].key ?? null]).toEqual(['3a', null]);
    expect(t.problems.get('1:2')).toContain('tam sayı ister');
    expect(t.problems.size).toBe(1);
    expect([t.rows[0].selected, t.rows[0].inView, t.rows[0].passes]).toEqual([true, false, true]);
  });
});
