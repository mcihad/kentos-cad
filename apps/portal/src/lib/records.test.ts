import { describe, expect, it } from 'vitest';
import type { FeatureRecord, LayerField } from '../api/types';
import { byLabel, byLayer, valueText, withheldWords } from './records';

const durum: LayerField = { name: 'Durum', kind: 'text', values: [{ code: 'iyi', label: 'İyi' }, { code: 'kotu', label: 'Kötü' }] };
const not: LayerField = { name: 'Not', kind: 'text' };

const record = (layerId: string, attrs: Record<string, string>, label?: string): FeatureRecord =>
  ({ id: `${layerId}-${label ?? Object.keys(attrs).length}`, version: '1', entity: { kind: 'point', id: 0, layerId, attrs, label } }) as unknown as FeatureRecord;

describe('the project page says a record (docs/adr/9194)', () => {
  it('a value never written apart from an empty text, and a code by its label', () => {
    expect(valueText(not, undefined)).toBeNull();
    expect(valueText(not, '')).toBe('');
    expect(valueText(not, 'NULL')).toBe('NULL');
    expect(valueText(durum, 'kotu')).toBe('Kötü');
    expect(valueText(durum, 'bilinmeyen')).toBe('bilinmeyen');
    expect(valueText({ name: 'Kontrol', kind: 'boolean' }, 'true')).toBe('Evet');
    expect(valueText({ ...durum, multiple: true }, 'iyi; kotu')).toBe('İyi, Kötü');
  });

  it('the fields the role does not read by name, and the records by layer', () => {
    expect(withheldWords(['Not', 'Ekip'])).toBe('Rolünüz bu alanları okumuyor, gösterilmiyor: Not, Ekip');
    const by = byLayer([record('saha', { Durum: 'iyi' }), record('yol', {}), record('saha', {})]);
    expect([...by.keys()]).toEqual(['saha', 'yol']);
    expect(by.get('saha')?.length).toBe(2);
  });

  it('the records by their labels, in natural order, those without one last', () => {
    const labels = byLabel([record('saha', {}, 'D-10'), record('saha', {}), record('saha', {}, 'D-2'), record('saha', {}, 'D-1')]).map(
      (r) => (r.entity as unknown as { label?: string }).label ?? '—',
    );
    expect(labels).toEqual(['D-1', 'D-2', 'D-10', '—']);
  });
});
