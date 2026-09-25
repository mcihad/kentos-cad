// Renders every KentOS UI showcase scenario without a window and compares
// the images with a recorded set (TODOS.md UI-01; docs/baseline/2026-09-25).
// Rendering uses the software rasterizer (tiny-skia), which gives the same
// bytes for the same code, fonts and dependencies, so a match proves that a
// move, rename or dependency change left the interface as it was. A mismatch
// is a change to look at (the images are kept in --out), not a failure to
// hide: record a new set only after reading the differences.
//
// Four scenes draw the wall clock (a creation time; progress and toast timer
// bars that move with elapsed time), so their bytes can change from run to
// run; they are listed separately and do not fail the comparison. Fixing the
// clock in snapshot mode is an open item (docs/adr/0016).
//
// The reference is apps/ui-showcase/snapshots.json. `--write` records the
// current images as the new reference, after a deliberate change whose
// differences were read (the pre-import record stays in docs/baseline).
//
//   node scripts/ui/snapshots.mjs [--bin target/debug/kentos-ui-showcase]
//     [--compare apps/ui-showcase/snapshots.json] [--out .run/ui-snapshots] [--write]
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1] : def);
const bin = resolve(ROOT, opt('bin', 'target/debug/kentos-ui-showcase'));
const compare = resolve(ROOT, opt('compare', 'apps/ui-showcase/snapshots.json'));
const write = args.includes('--write');
const out = resolve(ROOT, opt('out', '.run/ui-snapshots'));
mkdirSync(out, { recursive: true });

const recorded = JSON.parse(readFileSync(compare, 'utf8'));
const WALL_CLOCK = new Set(['onay', 'bildirimler', 'galeri-yerlesim', 'galeri-geri-bildirim']);
const sha = (buf) => createHash('sha256').update(buf).digest('hex');
const same = [];
const differ = [];
const clocked = [];
const rendered = [];
for (const image of recorded.images) {
  const file = join(out, `${image.name}.png`);
  execFileSync(bin, ['snapshot', file, ...image.args], { env: { ...process.env, KENTOS_SNAPSHOT_BACKEND: 'tiny-skia' }, stdio: ['ignore', 'ignore', 'inherit'] });
  const bytes = readFileSync(file);
  rendered.push({ name: image.name, args: image.args, bytes: bytes.length, sha256: sha(bytes) });
  if (sha(bytes) === image.sha256) same.push(image.name);
  else (WALL_CLOCK.has(image.name) ? clocked : differ).push(image.name);
}
if (write) {
  // No commit id: the file's own git history says when the reference moved.
  const reference = { source: 'apps/ui-showcase', backend: 'tiny-skia', command: 'node scripts/ui/snapshots.mjs --write', wallClock: [...WALL_CLOCK], images: rendered };
  writeFileSync(compare, `${JSON.stringify(reference, null, 2)}\n`);
  console.log(`${rendered.length} görüntü yeni başvuru olarak yazıldı: ${compare}`);
  process.exit(0);
}
console.log(`${same.length}/${recorded.images.length} görüntü başvuruyla bayt bayt aynı (${recorded.commit ?? compare.replace(ROOT, '')}, ${recorded.backend}).`);
if (clocked.length) console.log(`Saat gösterdiği için farklı (beklenen): ${clocked.join(', ')}.`);
if (differ.length) {
  console.log(`Farklı: ${differ.join(', ')}. Görüntüler: ${out}`);
  process.exit(1);
}
