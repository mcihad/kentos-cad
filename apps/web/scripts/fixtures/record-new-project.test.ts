// Writes the new-project fixture (fixtures/project/v1/new-project.json) from
// src/model/newProject.ts and src/model/standardLayers.ts: the drawing a new
// project starts as, for a few coordinate systems, scales and modes, as a
// `.kcad` v1 snapshot. Runs only on purpose, after a deliberate change:
//   GOLDEN_WRITE=1 npx vitest run scripts/fixtures/record-new-project.test.ts
// src/model/newProject.test.ts fails while the file and the code differ; the
// desktop builds the same drawings (apps/desktop/src/project/new.rs) and its
// test compares them with this file.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { newProjectFixture } from '../../src/model/newProjectFixture';

const FILE = new URL('../../../../fixtures/project/v1/new-project.json', import.meta.url);

it.runIf(!!process.env.GOLDEN_WRITE)('records the new project drawings', () => {
  writeFileSync(FILE, `${JSON.stringify(newProjectFixture(), null, 2)}\n`);
});
