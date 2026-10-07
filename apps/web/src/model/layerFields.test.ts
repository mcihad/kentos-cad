import { describe, expect, it } from 'vitest';
import type { LayerField } from './layerFields';
import { checkValue, compareDecimals, displayValue, fieldsProblem, inferFields, layerFieldsProblem } from './layerFields';

/**
 * A layer's fields (docs/adr/0199 §1, §3) against the independent reference (fixtures/layer-fields/v1/cases.json,
 * scripts/fixtures/layer_field_cases.py): a value's canonical text or why not, a field list's first problem, the fields
 * Verilerden al gives, a value's display; word for word as the contract (`kentos_contracts::fields`) gives them.
 */

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Cases {
  checks: { field: LayerField; text: string; want: unknown }[];
  problems: { name: string; fields: LayerField[]; want: string | null }[];
  infer: { name: string; rows: Record<string, string>[]; want: LayerField[] }[];
  display: { field: LayerField; value: string; want: string }[];
}

const cases = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/layer-fields/v1/cases.json', import.meta.url), 'utf8')) as Cases;

describe('layer fields (docs/adr/0199 §1)', () => {
  it('gives each value its canonical text or refuses it as the reference does', () => {
    expect(cases.checks.length).toBeGreaterThan(50);
    const off = cases.checks.flatMap((c) => {
      const got = checkValue(c.field, c.text);
      return JSON.stringify(got) === JSON.stringify(c.want) ? [] : [`${c.field.name} ← ${JSON.stringify(c.text)}: ${JSON.stringify(got)} ≠ ${JSON.stringify(c.want)}`];
    });
    expect(off).toEqual([]);
  });

  it('finds a field list’s first problem as the reference does', () => {
    for (const c of cases.problems) expect(fieldsProblem(c.fields), c.name).toBe(c.want);
    expect(layerFieldsProblem([])).not.toBeNull();
    expect(layerFieldsProblem([{ name: 'Ad', kind: 'text' }])).toBeNull();
  });

  it('gives Verilerden al’s fields in the natural order', () => {
    for (const c of cases.infer) expect(inferFields(c.rows), c.name).toEqual(c.want);
  });

  it('shows values as the reference does', () => {
    for (const c of cases.display) expect(displayValue(c.field, c.value), c.value).toBe(c.want);
  });

  it('compares decimals by their exact values', () => {
    expect(compareDecimals('0.5', '0.45')).toBe(1);
    expect(compareDecimals('12.50', '12.5')).toBe(0);
    expect(compareDecimals('-0.00', '0')).toBe(0);
    expect(compareDecimals('-2', '-10')).toBe(1);
    expect(compareDecimals('123456789012345678901234567890', '123456789012345678901234567891')).toBe(-1);
    expect(compareDecimals('1e3', '1')).toBeNull();
  });
});
