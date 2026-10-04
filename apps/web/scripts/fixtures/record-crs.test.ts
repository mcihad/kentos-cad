// Writes the CRS registry fixture (fixtures/crs/v1/registry.json) from
// src/geo/crs.ts, the single source of CRS metadata (CLAUDE.md §5). Runs only
// on purpose, after a deliberate change to the registry:
//   GOLDEN_WRITE=1 npx vitest run scripts/fixtures/record-crs.test.ts
// src/geo/crs.test.ts fails while the file and the registry differ; the Rust
// side checks the file against EPSG facts (crates/shared/contracts/tests/all/crs.rs).
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { CRS_REGISTRY, DEFAULT_SRID } from '../../src/geo/crs';
import { crsFixture } from '../../src/geo/crsFixture';
import { provincesFixture } from '../../src/geo/provinces';

const FILE = new URL('../../../../fixtures/crs/v1/registry.json', import.meta.url);
// The provinces (src/geo/provinces.ts, docs/adr/0165 §3): one per line, as the list reads.
const PROVINCES = new URL('../../../../fixtures/crs/v1/provinces.json', import.meta.url);

it.runIf(!!process.env.GOLDEN_WRITE)('records the CRS registry', () => {
  writeFileSync(FILE, `${JSON.stringify(crsFixture(CRS_REGISTRY, DEFAULT_SRID), null, 2)}\n`);
  const { provinces, ...head } = provincesFixture();
  const rows = provinces.map((x) => `    ${JSON.stringify(x)}`).join(',\n');
  const text = JSON.stringify(head, null, 2).replace(/\n}$/, `,\n  "provinces": [\n${rows}\n  ]\n}`);
  writeFileSync(PROVINCES, `${text}\n`);
});
