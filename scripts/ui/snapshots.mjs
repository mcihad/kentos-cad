// Renders every KentOS UI showcase scenario without a window and compares
// the images with a recorded set (TODOS.md UI-01; docs/baseline/2026-09-25).
// Rendering uses the software rasterizer (tiny-skia), which gives the same
// bytes for the same code, fonts and dependencies, so a match proves that a
// move, rename or dependency change left the interface as it was. A mismatch
// is a change to look at (the images are kept in --out), not a failure to
// hide: record a new set only after reading the differences.
//
// Three scenes draw the wall clock (a creation time, progress bars that move
// with elapsed time), so their bytes change from run to run; they are listed
// separately and do not fail the comparison. Fixing the clock in snapshot mode
// is an open item (docs/adr/0016).
//
//   node scripts/ui/snapshots.mjs [--bin target/debug/kentos-ui-showcase]
//     [--compare docs/baseline/2026-09-25/kentos-rc-showcase.json] [--out .run/ui-snapshots]
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1] : def);
const bin = resolve(ROOT, opt('bin', 'target/debug/kentos-ui-showcase'));
const compare = resolve(ROOT, opt('compare', 'docs/baseline/2026-09-25/kentos-rc-showcase.json'));
const out = resolve(ROOT, opt('out', '.run/ui-snapshots'));
mkdirSync(out, { recursive: true });

const recorded = JSON.parse(readFileSync(compare, 'utf8'));
const WALL_CLOCK = new Set(['onay', 'galeri-yerlesim', 'galeri-geri-bildirim']);
const sha = (buf) => createHash('sha256').update(buf).digest('hex');
const same = [];
const differ = [];
const clocked = [];
for (const image of recorded.images) {
  const file = join(out, `${image.name}.png`);
  execFileSync(bin, ['snapshot', file, ...image.args], { env: { ...process.env, KENTOS_SNAPSHOT_BACKEND: 'tiny-skia' }, stdio: ['ignore', 'ignore', 'inherit'] });
  if (sha(readFileSync(file)) === image.sha256) same.push(image.name);
  else (WALL_CLOCK.has(image.name) ? clocked : differ).push(image.name);
}
console.log(`${same.length}/${recorded.images.length} görüntü kayıtla bayt bayt aynı (${recorded.commit}, ${recorded.backend}).`);
if (clocked.length) console.log(`Saat gösterdiği için farklı (beklenen): ${clocked.join(', ')}.`);
if (differ.length) {
  console.log(`Farklı: ${differ.join(', ')}. Görüntüler: ${out}`);
  process.exit(1);
}
