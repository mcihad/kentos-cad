import { describe, expect, it } from 'vitest';
import type { LayerNode } from '../model/layers';
import { layerListCsv, layerListRows, layerListTsv } from './layerList';

/**
 * Katman listesi (docs/adr/0177 §6) against the shared cases (fixtures/layers/v1/list.json, written by
 * scripts/fixtures/layer_list_cases.py from the ADR, not KentOS code). The desktop reads the same file (layer_list.rs).
 */
const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

interface File {
  format: string;
  cases: { name: string; tree: LayerNode[]; active: string; counts: Record<string, number>; rows: string[][]; csv: string; tsv: string }[];
}

const file = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/layers/v1/list.json', import.meta.url), 'utf8')) as File;

describe('Katman listesi (docs/adr/0177 §6)', () => {
  it('writes the rows, the CSV and the clipboard text as the shared cases say', () => {
    expect(file.format).toBe('kentos.layer-list-cases');
    for (const c of file.cases) {
      const rows = layerListRows(c.tree, c.active, (id) => c.counts[id] ?? 0);
      expect(rows, c.name).toEqual(c.rows);
      expect(layerListCsv(rows), c.name).toBe(c.csv);
      expect(layerListTsv(rows), c.name).toBe(c.tsv);
    }
  });
});
