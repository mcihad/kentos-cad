import { describe, expect, it } from 'vitest';
import type { DatumTransform } from '../contracts/generated/DatumTransform';
import { buildChoice, epsgText, formOf, PAIRS, TEXTS, type ChoiceForm, type GridRef, type Pair, type Problems } from './choiceForm';

/**
 * Proje ayarları' Datum dönüşümleri (docs/adr/0168 §3, §6) against the shared cases (fixtures/crs/v1/choice-form.json,
 * written by scripts/fixtures/crs_choice_form_cases.py from the ADR's rules, not KentOS code): what is typed turns into
 * the project's choice or, field by field, what is wrong; a choice's form gives it back. The desktop reads the same
 * file (crates/native/project/src/choice_form.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  texts: typeof TEXTS;
  epsg: { pair: Pair; text: string }[];
  library: GridRef[];
  cases: { name: string; pair: Pair; form: ChoiceForm; choice?: DatumTransform | null; problems?: Problems }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/crs/v1/choice-form.json', import.meta.url), 'utf8')) as File;

describe('Datum dönüşümleri (docs/adr/0168 §3)', () => {
  it('builds the choices and says the problems as the shared cases say', () => {
    expect(file.format).toBe('kentos.crs-choice-form');
    expect(file.texts).toEqual(TEXTS);
    expect(file.epsg.map((e) => e.pair)).toEqual(PAIRS);
    for (const e of file.epsg) expect(epsgText(e.pair)).toBe(e.text);
    expect(file.cases.length).toBeGreaterThanOrEqual(8);
    for (const c of file.cases) {
      const got = buildChoice(c.pair, c.form, file.library);
      if (c.problems) expect(got, c.name).toEqual({ problems: c.problems });
      else {
        expect(got, c.name).toEqual({ choice: c.choice });
        // Its form gives it back.
        if (c.choice) expect(buildChoice(c.pair, formOf(c.pair, [c.choice]), file.library), c.name).toEqual({ choice: c.choice });
      }
    }
  });
});
