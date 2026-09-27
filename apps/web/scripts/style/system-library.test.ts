// The system style library (MPYY and the basic symbols, src/style/system) as the desktop reads it:
// one .kstil file, crates/native/style/assets/system-library.kstil, embedded in the desktop build.
// The TypeScript is the single source; this test fails when the file no longer matches it.
// Rewrite the file on purpose, then read the diff:
//   KENTOS_WRITE_SYSTEM_STYLES=1 pnpm -C apps/web exec vitest run scripts/style/system-library.test.ts
// Outside src/ so the app's type check does not need Node's types.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { expect, it } from 'vitest';
import { STYLE_FORMAT, STYLE_VERSION } from '../../src/style/file';
import { SYSTEM_LIBRARY } from '../../src/style/system';

const OUT = new URL('../../../../crates/native/style/assets/system-library.kstil', import.meta.url);

/** The library as a .kstil file, the format the desktop's style files read. `exported` is fixed, so the file changes only with the symbols. */
function text(): string {
  const file = { format: STYLE_FORMAT, version: STYLE_VERSION, exported: 'system', items: SYSTEM_LIBRARY.items, categories: SYSTEM_LIBRARY.categories };
  return `${JSON.stringify(file)}\n`;
}

it('the desktop’s copy of the system library is the TypeScript’s', () => {
  const want = text();
  if (process.env.KENTOS_WRITE_SYSTEM_STYLES) writeFileSync(OUT, want);
  const have = existsSync(OUT) ? readFileSync(OUT, 'utf8') : '';
  expect(have === want, 'crates/native/style/assets/system-library.kstil is stale: KENTOS_WRITE_SYSTEM_STYLES=1 pnpm -C apps/web exec vitest run scripts/style/system-library.test.ts').toBe(true);
});
