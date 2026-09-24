// Records the sample drawing as the document fixture the Rust contracts read
// (fixtures/document/v1/sample.json). Runs only on purpose:
//   GOLDEN_WRITE=1 npx vitest run scripts/fixtures/record-document.test.ts
import { mkdirSync, writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { toSnapshot } from '../../src/model/snapshot';
import { snapshotSampleDocument } from '../../src/model/snapshotSample';

it.runIf(!!process.env.GOLDEN_WRITE)('records the sample drawing snapshot', () => {
  const dir = new URL('../../../../fixtures/document/v1/', import.meta.url);
  mkdirSync(dir, { recursive: true });
  writeFileSync(new URL('sample.json', dir), `${JSON.stringify(toSnapshot(snapshotSampleDocument()), null, 2)}\n`);
});
