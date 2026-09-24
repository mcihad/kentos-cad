// Records the TypeScript reference results into the shared golden file
// (fixtures/geometry/v1/cases.json). Runs only on purpose:
//   GOLDEN_WRITE=1 npx vitest run scripts/fixtures/record-geometry.test.ts
// Outside src/ so the app's type check does not need Node's types.
import { readFileSync, writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { run, type GoldenFile } from '../../src/model/geom/goldenCases';

const FILE = new URL('../../../../fixtures/geometry/v1/cases.json', import.meta.url);

it.runIf(!!process.env.GOLDEN_WRITE)('records the TypeScript reference into the golden file', () => {
  const file = JSON.parse(readFileSync(FILE, 'utf8')) as GoldenFile;
  for (const c of file.cases) c.expected = run(c);
  writeFileSync(FILE, `${JSON.stringify(file, null, 2)}\n`);
});
