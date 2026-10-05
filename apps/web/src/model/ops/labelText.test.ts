import { describe, expect, it } from 'vitest';
import { fillTemplate, labelTexts, type LabelItem, type LabelTexts } from './labelText';

/**
 * Etiketleri yazıya çevir (docs/adr/0175 §1) through the WASM core, against the independent reference in
 * fixtures/label-text/v1/cases.json (scripts/fixtures/label_text_cases.py, no KentOS code), the cases the core runs
 * natively in crates/shared/geometry-core/tests/all/label_text.rs: places and heights within 1e-9 m, rotations within
 * 1e-9°, a missing field null.
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface Case {
  name: string;
  items: LabelItem[];
  scale: number;
  thin: boolean;
  expected: LabelTexts;
}

function differ(a: unknown, e: unknown, path: string): string | null {
  if (typeof a === 'number' && typeof e === 'number') return Math.abs(a - e) <= 1e-9 ? null : `${path}: ${a} ≠ ${e}`;
  if (Array.isArray(a) || Array.isArray(e)) {
    if (!Array.isArray(a) || !Array.isArray(e) || a.length !== e.length) return `${path}: ${JSON.stringify(a)?.slice(0, 80)} ≠ ${JSON.stringify(e)?.slice(0, 80)}`;
    for (let i = 0; i < a.length; i++) {
      const d = differ(a[i], e[i], `${path}[${i}]`);
      if (d) return d;
    }
    return null;
  }
  if (a && e && typeof a === 'object' && typeof e === 'object') {
    for (const k of new Set([...Object.keys(a), ...Object.keys(e)])) {
      const d = differ((a as Record<string, unknown>)[k] ?? null, (e as Record<string, unknown>)[k] ?? null, `${path}.${k}`);
      if (d) return d;
    }
    return null;
  }
  return (a ?? null) === (e ?? null) ? null : `${path}: ${String(a)} ≠ ${String(e)}`;
}

describe('Etiketleri yazıya çevir', () => {
  it('fills a template as the core does: the first {label}, literally', () => {
    expect(fillTemplate(undefined, '101')).toBe('101');
    expect(fillTemplate('', '101')).toBe('101');
    expect(fillTemplate('No: {label}', '101')).toBe('No: 101');
    expect(fillTemplate('{label}/{label}', '7')).toBe('7/{label}');
    expect(fillTemplate('Ada', '7')).toBe('Ada');
    // String.replace would write the template's text for `$&`.
    expect(fillTemplate('[{label}]', 'a$&b')).toBe('[a$&b]');
  });

  it('writes every case as the reference writes it', () => {
    const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/label-text/v1/cases.json', import.meta.url), 'utf8')) as { format: string; version: number; cases: Case[] };
    expect([file.format, file.version]).toEqual(['kentos.label-text-fixtures', 1]);
    expect(file.cases.length).toBeGreaterThanOrEqual(80);
    const off = file.cases.flatMap((c) => {
      const d = differ(labelTexts(c.items, c.scale, c.thin), c.expected, c.name);
      return d ? [d] : [];
    });
    expect(off).toEqual([]);
  });
});
