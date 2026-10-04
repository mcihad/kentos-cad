import { describe, expect, it } from 'vitest';
import type { CrsDefinition } from '../contracts/generated/CrsDefinition';
import { definitionProj, definitionWkt, READ_TEXTS, readDefinition } from '../model/definitionForm';
import type { System } from '../model/geom/crsTransform';
import { definitionFrom, definitionSystem } from '../model/projectCrs';

/**
 * Özel koordinat sistemi's WKT and PROJ (docs/adr/0168 §5, §6) through the WASM core the app calls: a text read as the
 * shared cases say (fixtures/crs/v1/definition-text.json, scripts/fixtures/crs_definition_text_cases.py: the systems PROJ
 * reads in fixtures/geodesy/v1/text.json, the registry and the ADR's rules), every system read a definition the
 * transforms read back as it was, a definition copied as the core's texts. The desktop runs the same files
 * (crates/native/project/src/definition_form.rs, systems.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const read = <T>(path: string): T => JSON.parse(fs.readFileSync(new URL(path, import.meta.url), 'utf8')) as T;

interface Cases {
  format: string;
  texts: Record<string, string>;
  reads: { name: string; text: string; definition?: CrsDefinition; same?: number; note?: string; problem?: string }[];
}

interface Texts {
  reads: { name: string; text: string; expect?: { name: string; system: System } }[];
  writes: { name: string; definition: { name: string; system: System }; wkt: string | null; proj: string | null }[];
}

describe('Özel koordinat sistemi: WKT and PROJ', () => {
  const cases = read<Cases>('../../../../fixtures/crs/v1/definition-text.json');
  const texts = read<Texts>('../../../../fixtures/geodesy/v1/text.json');

  it('says what the shared cases say', () => {
    expect(cases.format).toBe('kentos.crs-definition-text');
    for (const [key, text] of Object.entries(READ_TEXTS)) expect(cases.texts[key], key).toBe(text);
    expect(cases.texts.base).toBe('{name} tabanı');
  });

  it('reads texts as the shared cases', () => {
    for (const c of cases.reads) {
      const got = readDefinition(c.text);
      if (c.problem !== undefined) expect(got, c.name).toEqual({ problem: c.problem });
      else expect(got, c.name).toEqual({ definition: c.definition, same: c.same ?? null, note: c.note ?? null });
    }
    expect(cases.reads.length).toBeGreaterThanOrEqual(27);
  });

  it('reads every system back as it was, a base the registry lacks as a definition of its own', () => {
    let n = 0;
    for (const c of texts.reads) {
      if (!c.expect) continue;
      const d = definitionFrom(c.expect.name, c.expect.system);
      expect(d, c.name).not.toBeNull();
      expect(definitionSystem(d!), c.name).toEqual(c.expect.system);
      n++;
    }
    expect(n).toBeGreaterThanOrEqual(19);
    const base: System = { kind: 'tm', datum: 'TUREF', centralMeridian: 30, scaleFactor: 1, falseEasting: 400000, falseNorthing: 0 };
    const local: System = { kind: 'local', base, plane: { kind: 'similarity', east: 1, north: 2, rotation: 0.5, scale: 1 } };
    const d = definitionFrom('Şantiye', local)!;
    expect(d.system.kind === 'local' && d.system.base.srid).toBe(undefined);
    expect(d.system.kind === 'local' && d.system.base.definition?.name).toBe('Şantiye tabanı');
    expect(definitionSystem(d)).toEqual(local);
    expect(definitionFrom('Web', { kind: 'mercator' })).toBeNull();
  });

  it('copies a definition as the core writes its system', () => {
    for (const c of texts.writes) {
      const d = definitionFrom(c.definition.name, c.definition.system)!;
      // A base the registry does not have is named as the case names it.
      if (d.system.kind === 'local' && d.system.base.definition) {
        const name = c.wkt?.split('BASEPROJCRS["')[1]?.split('"')[0] ?? '';
        d.system.base.definition = { ...d.system.base.definition, name };
      }
      expect(definitionWkt(d), `${c.name}: WKT`).toBe(c.wkt);
      expect(definitionProj(d), `${c.name}: PROJ`).toBe(c.proj);
    }
  });
});
