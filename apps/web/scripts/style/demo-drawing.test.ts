// The web's demo drawing (the sample project and, below its sheet, the showcase of every system
// symbol, app/startContent.ts) written as a .kcad v1 JSON file, so the desktop draws the same drawing
// and the two platforms' pictures can be set side by side (docs/adr/0090). Runs only on purpose,
// into a file outside the repository:
//   KENTOS_DEMO_OUT=/path/demo.json pnpm -C apps/web exec vitest run scripts/style/demo-drawing.test.ts
// Outside src/ so the app's type check does not need Node's types.
import { writeFileSync } from 'node:fs';
import { it } from 'vitest';
import { createSampleProject } from '../../src/model/sampleProject';
import { toSnapshot } from '../../src/model/snapshot';
import { buildShowcase } from '../../src/style/showcase';
import { SYSTEM_LIBRARY } from '../../src/style/system';

it.runIf(!!process.env.KENTOS_DEMO_OUT)('writes the web’s demo drawing', () => {
  const doc = createSampleProject();
  if (doc.homeView) buildShowcase(doc, SYSTEM_LIBRARY.items, SYSTEM_LIBRARY.categories, { x: doc.homeView.minX, y: doc.homeView.minY - 80 });
  writeFileSync(process.env.KENTOS_DEMO_OUT!, JSON.stringify(toSnapshot(doc)));
});
