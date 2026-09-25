// Builds the Rust WASM packages whose sources changed since their last build
// (docs/adr/0008): the geometry core (apps/web/src/wasm/pkg), which the app needs to
// start, the file formats (apps/web/src/io/pkg), which the formats worker loads
// only when a file is imported or exported, and the SVG editor's geometry
// (apps/web/src/style/svg/pkg), loaded with the editor (CLAUDE.md §20). `pnpm dev`,
// `test`, `build`, `e2e` and the perf scripts run this first. Each package
// has its own digest (the crates it is built from and the toolchain pins)
// and stamp, so an edit to the formats never rebuilds the core and nothing
// is built when nothing changed. Builds run niced (one heavy process at a
// time, ADR 0001).
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = new URL('../..', import.meta.url).pathname;
const PINS = ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo/config.toml'];
const PACKAGES = [
  { label: 'Geometri çekirdeği', script: 'rust:wasm', out: 'apps/web/src/wasm/pkg', lib: 'kentos_geometry_wasm', sources: ['crates/shared/geometry-core', 'crates/shared/style-core', 'crates/wasm/geometry-wasm', ...PINS] },
  // The formats use the core's own sampling of bulged rings (docs/adr/0009): a core edit rebuilds both.
  { label: 'Dosya biçimleri', script: 'rust:wasm:formats', out: 'apps/web/src/io/pkg', lib: 'kentos_formats_wasm', sources: ['crates/shared/formats', 'crates/wasm/formats-wasm', 'crates/shared/contracts', 'crates/shared/geometry-core', ...PINS] },
  // The SVG editor's geometry (loaded with the editor) runs on the core's overlay and writes numbers as the style core does.
  { label: 'SVG düzenleyicisi', script: 'rust:wasm:svg', out: 'apps/web/src/style/svg/pkg', lib: 'kentos_svg_wasm', sources: ['crates/shared/svg-core', 'crates/wasm/svg-wasm', 'crates/shared/geometry-core', 'crates/shared/style-core', ...PINS] },
];

function files(path) {
  const full = join(ROOT, path);
  if (!existsSync(full)) return [];
  if (!statSync(full).isDirectory()) return [full];
  return readdirSync(full, { recursive: true })
    .map((f) => join(full, f))
    .filter((f) => statSync(f).isFile())
    .sort();
}

function digest(sources) {
  const h = createHash('sha256');
  for (const f of sources.flatMap(files)) {
    h.update(relative(ROOT, f));
    h.update('\0');
    h.update(readFileSync(f));
    h.update('\0');
  }
  return h.digest('hex');
}

for (const pkg of PACKAGES) {
  const out = join(ROOT, pkg.out);
  const stamp = join(out, '.stamp');
  const want = digest(pkg.sources);
  const have = existsSync(stamp) ? readFileSync(stamp, 'utf8').trim() : '';
  const complete = [`${pkg.lib}.js`, `${pkg.lib}.d.ts`, `${pkg.lib}_bg.wasm`].every((f) => existsSync(join(out, f)));
  if (want === have && complete) continue;

  console.log(`${pkg.label} (WASM) derleniyor…`);
  const r = spawnSync('nice', ['-n', '10', 'pnpm', '-s', pkg.script], { cwd: ROOT, stdio: 'inherit' });
  if (r.status !== 0) {
    console.error(`${pkg.label} WASM paketi derlenemedi. Rust araç zinciri kurulu mu? (rust-toolchain.toml, wasm-bindgen-cli 0.2.128; CLAUDE.md §2)`);
    process.exit(r.status ?? 1);
  }
  writeFileSync(stamp, `${want}\n`);
}
