import { describe, expect, it } from 'vitest';
import { crsReadText, crsWriteProj, crsWriteWkt } from '../model/geom/crsText';
import type { System } from '../model/geom/crsTransform';

/**
 * Coordinate systems read from WKT and PROJ strings and written as them (docs/adr/0168 §5) against
 * fixtures/geodesy/v1/text.json (scripts/fixtures/crs_text_cases.py: PROJ reads every text there, its numbers and the
 * ADR's rules give the systems; the written texts are the rules', and PROJ reads them back), through the WASM core the
 * app calls: everything exactly. The core runs the same file natively (crates/shared/geometry-core/tests/all/crs_text.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  reads: { name: string; text: string; expect?: unknown; error?: { kind: string; detail: string } }[];
  writes: { name: string; definition: { name: string; system: System }; wkt: string | null; proj: string | null }[];
}

describe('crsReadText, crsWriteWkt, crsWriteProj', () => {
  const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/geodesy/v1/text.json', import.meta.url), 'utf8')) as File;

  it('reads the reference', () => expect(file.format).toBe('kentos.crs-text'));

  it('reads texts as PROJ and the rules read them', () => {
    for (const c of file.reads) {
      const got = crsReadText(c.text);
      expect(got, c.name).toEqual(c.error ? { error: c.error } : c.expect);
    }
    expect(file.reads.length).toBeGreaterThanOrEqual(25);
  });

  it('writes systems as the rules write them', () => {
    for (const c of file.writes) {
      const base = c.definition.system.kind === 'local' ? (c.wkt?.split('BASEPROJCRS["')[1]?.split('"')[0] ?? null) : null;
      expect(crsWriteWkt(c.definition.name, c.definition.system, base), `${c.name}: WKT`).toBe(c.wkt);
      expect(crsWriteProj(c.definition.system), `${c.name}: PROJ`).toBe(c.proj);
    }
  });
});
