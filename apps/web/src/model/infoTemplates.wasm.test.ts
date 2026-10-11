import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/expression/v2/template.json?raw';
import { op } from '../wasm/core';
import { compileTemplate, templateTarget, templateTexts } from './infoTemplates';
import type { Entity } from './entities';

/**
 * Templates (docs/adr/0225 §6) through the WASM core on the independent reference's cases
 * (fixtures/expression/v2/template.json, scripts/fixtures/template_cases.py): the pieces and the errors, the filled
 * texts, the kinds of targets. The core runs the same file natively (`kentos_expression::template`).
 */
const f = JSON.parse(text) as {
  parse: { template: string; pieces?: unknown[]; encoded?: boolean; error?: { at: number; message: string } }[];
  fill: { template: string; values: (string | null)[]; place?: [number, number]; text: string }[];
  target: { target: string; kind: string }[];
};
const parse = op<(t: string) => { pieces: unknown[]; encoded: boolean; error?: { at: number; message: string } }>('templateParse');
const fill = op<(t: string, values: (string | null)[], place: number[]) => string>('templateFill');

describe('templates (fixtures/expression/v2/template.json)', () => {
  it('splits every template as the reference says', () => {
    for (const c of f.parse) {
      const got = parse(c.template);
      if (c.error) expect(got.error, c.template).toEqual(c.error);
      else {
        expect(got.error, c.template).toBeUndefined();
        expect(got.pieces, c.template).toEqual(c.pieces);
        expect(got.encoded, c.template).toBe(c.encoded);
      }
    }
  });

  it('fills every template as the reference says', () => {
    for (const c of f.fill) expect(fill(c.template, c.values, c.place ?? []), c.template).toBe(c.text);
  });

  it('says what every target is', () => {
    for (const c of f.target) expect(templateTarget(c.target), JSON.stringify(c.target)).toBe(c.kind);
  });

  it('compiles a template with its expressions and fills it for objects, a batch at a time', () => {
    const parcel = (id: number, ada: string): Entity => ({ kind: 'point', id, layerId: 'parsel', attrs: { Ada: ada, 'Tapu alanı': '1250.40' }, p: { x: 0, y: 0 } });
    const t = compileTemplate('https://a.org/{Ada}?alan={[Tapu alanı]}&yer={$enlem}');
    expect(t.ok).toBe(true);
    if (!t.ok) return;
    const texts = templateTexts(t.template, [parcel(1, '101 A'), parcel(2, 'ş')], () => 'Parsel', undefined, (i) => (i === 0 ? [41, 29] : null));
    expect(texts).toEqual(['https://a.org/101%20A?alan=1250.40&yer=41.00000000', 'https://a.org/%C5%9F?alan=1250.40&yer=']);
    expect(compileTemplate('Ada {Ada +}')).toMatchObject({ ok: false });
    expect(compileTemplate('{ $sıra }')).toEqual({ ok: false, error: '2. karakterde: Şablonda $sıra kullanılamaz.' });
  });
});
