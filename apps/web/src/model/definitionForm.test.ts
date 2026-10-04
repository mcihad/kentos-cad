import { describe, expect, it } from 'vitest';
import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import { buildDefinition, ELLIPSOIDS, formOf, TEXTS, type DefinitionForm, type Problems } from './definitionForm';

/**
 * The Özel koordinat sistemi window's rules (docs/adr/0168 §1–§2, §6) against the shared cases
 * (fixtures/crs/v1/definition-form.json, written by scripts/fixtures/crs_definition_form_cases.py from the ADR's rules
 * and the registry, not KentOS code): what is typed turns into the definition and its note, or the problems by field;
 * a definition's form gives it back. The desktop reads the same file (crates/native/project/src/definition_form.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  texts: typeof TEXTS;
  ellipsoids: { name: string; semiMajor: number; inverseFlattening: number }[];
  cases: { name: string; form: DefinitionForm; definition?: CrsDefinition; note?: string; problems?: Problems }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/crs/v1/definition-form.json', import.meta.url), 'utf8')) as File;

describe('Özel koordinat sistemi (docs/adr/0168 §1)', () => {
  it('builds the definitions and says the problems as the shared cases say', () => {
    expect(file.format).toBe('kentos.crs-definition-form');
    expect(file.texts).toEqual(TEXTS);
    expect(file.ellipsoids.map((e) => [e.name, e.semiMajor, e.inverseFlattening])).toEqual(ELLIPSOIDS);
    expect(file.cases.length).toBeGreaterThanOrEqual(13);
    for (const c of file.cases) {
      const got = buildDefinition(c.form);
      if (c.problems) expect(got, c.name).toEqual({ problems: c.problems });
      else {
        expect(got, c.name).toEqual(c.note ? { definition: c.definition, note: c.note } : { definition: c.definition });
        // Its form gives it back.
        const again = buildDefinition(formOf(c.definition!));
        expect('definition' in again ? again.definition : again, c.name).toEqual(c.definition);
      }
    }
  });
});
