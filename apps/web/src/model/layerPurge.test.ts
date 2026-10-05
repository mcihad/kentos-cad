import { describe, expect, it } from 'vitest';
import { purgeFound, purgeRemoved, type PurgeFound, type PurgeIds, type PurgeSource } from './layerPurge';

/**
 * Kullanılmayanları temizle's rule (docs/adr/0177 §5) against the shared cases (fixtures/layers/v1/purge.json, written
 * by scripts/fixtures/layer_purge_cases.py from the ADR, not KentOS code). The desktop reads the same file
 * (crates/native/interaction/src/layer_purge.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  cases: { name: string; document: PurgeSource; found: PurgeFound; applies: { name: string; checked: Partial<PurgeIds>; removed: PurgeIds; kept: number }[] }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/layers/v1/purge.json', import.meta.url), 'utf8')) as File;

describe('Kullanılmayanları temizle (docs/adr/0177 §5)', () => {
  it('finds and removes as the shared cases say', () => {
    expect(file.format).toBe('kentos.layer-purge-cases');
    expect(file.cases.length).toBeGreaterThanOrEqual(2);
    for (const c of file.cases) {
      expect(purgeFound(c.document), c.name).toEqual(c.found);
      for (const a of c.applies) expect(purgeRemoved(c.document, a.checked), `${c.name}: ${a.name}`).toEqual({ removed: a.removed, kept: a.kept });
    }
  });
});
