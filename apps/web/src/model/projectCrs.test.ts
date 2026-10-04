import { describe, expect, it } from 'vitest';
import type { DatumChoice } from './geom/crsTransform';
import { datumChoices, ownSystem, secondSystem, type CrsSettings, type NamedSystem } from './projectCrs';

/**
 * The project's systems and datum choices (docs/adr/0168 §1–§3) against the shared cases (fixtures/geodesy/v1/project.json,
 * written by scripts/fixtures/project_crs_cases.py from the ADR's rules and the registry, not KentOS code): names,
 * titles and codes exactly, the systems and choices as the core reads them. The desktop reads the same file
 * (crates/native/project/src/systems.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  cases: { name: string; settings: CrsSettings; own: NamedSystem | null; second: NamedSystem | null; choices: DatumChoice[] }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/project.json', import.meta.url), 'utf8')) as File;

describe("the project's systems (docs/adr/0168)", () => {
  it('are named and read as the shared cases say', () => {
    expect(file.format).toBe('kentos.project-crs');
    expect(file.cases.length).toBeGreaterThanOrEqual(9);
    for (const c of file.cases) {
      expect(ownSystem(c.settings), c.name).toEqual(c.own);
      expect(secondSystem(c.settings), c.name).toEqual(c.second);
      expect(datumChoices(c.settings), c.name).toEqual(c.choices);
    }
  });
});
