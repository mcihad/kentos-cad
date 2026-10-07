import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/sources/v1/cases.json?raw';
import { SOURCE_LABELS, sourceKind, sourceListing, type SourceEntry } from './sources';

/**
 * Kaynaklar's folder listing (docs/adr/0199 §7) on the shared cases fixtures/sources/v1/cases.json
 * (scripts/fixtures/source_list_cases.py, no KentOS code; the natural order by the WASM core). The desktop plays the
 * same file (crates/native/interaction/tests/all/sources.rs).
 */
const f = JSON.parse(text);

describe('Kaynaklar: a folder’s listing (fixtures/sources/v1)', () => {
  it('shows the folders and the files it adds, Shapefiles with their parts', () => {
    for (const c of f.cases) expect(sourceListing(c.entries as SourceEntry[]), c.name).toEqual(c.expect);
  });

  it('names the kinds as the cases do', () => {
    expect(SOURCE_LABELS).toEqual(f.labels);
    expect([sourceKind('a.SHP'), sourceKind('.dxf'), sourceKind('b'), sourceKind('c.kcad')]).toEqual(['shapefile', null, null, null]);
  });
});
