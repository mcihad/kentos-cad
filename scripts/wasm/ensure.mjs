// Builds the Rust WASM packages whose sources changed since their last build
// (docs/adr/0008): the geometry core (apps/web/src/wasm/pkg), which the app needs to
// start, the file formats (apps/web/src/io/pkg), which the formats worker loads
// only when a file is imported or exported or a drawing opened or saved, DXF and
// Netcad NCZ (apps/web/src/io/dxf/pkg, io/ncz/pkg), loaded only for such a file,
// the SVG editor's geometry
// (apps/web/src/style/svg/pkg), loaded with the editor, the sheet core
// (apps/web/src/product/sheet/pkg), loaded when the sheet mode opens, the map
// services (apps/web/src/io/services/pkg), loaded when a drawing shows one, and the
// raster analyses (apps/web/src/io/raster/pkg), loaded in their worker when a job
// starts (CLAUDE.md §20). `pnpm dev`,
// `test`, `build`, `e2e` and the perf scripts run this first. Each package
// has its own digest (the crates it is built from, the toolchain pins and the
// profile) and stamp, so an edit to the formats never rebuilds the core and
// nothing is built when nothing changed. Builds run niced (one heavy process
// at a time, ADR 0001). The dev server, tests and e2e take the `wasm-dev`
// profile (no whole-program LTO, parallel code generation: a core edit costs a
// fraction of the time); `--release` (`pnpm build`, the perf scripts) takes
// `wasm`, the shipped one (Cargo.toml).
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { spawnSync } from 'node:child_process';

const ROOT = new URL('../..', import.meta.url).pathname;
const PROFILE = process.argv.includes('--release') ? 'wasm' : 'wasm-dev';
const PINS = ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', '.cargo/config.toml'];
const PACKAGES = [
  { label: 'Geometri çekirdeği', script: 'rust:wasm', out: 'apps/web/src/wasm/pkg', lib: 'kentos_geometry_wasm', sources: ['crates/shared/geometry-core', 'crates/shared/expression', 'crates/shared/style-core', 'crates/wasm/geometry-wasm', ...PINS] },
  // The formats use the core's own sampling of bulged rings (docs/adr/0009): a core edit rebuilds both.
  { label: 'Dosya biçimleri', script: 'rust:wasm:formats', out: 'apps/web/src/io/pkg', lib: 'kentos_formats_wasm', sources: ['crates/shared/formats', 'crates/shared/kcad', 'crates/wasm/formats-wasm', 'crates/shared/contracts', 'crates/shared/geometry-core', ...PINS] },
  // DXF and Netcad NCZ, each in a module of its own that the formats worker loads only when such a
  // file is imported or exported (the objects cross as the drawing's typed columns, crates/shared/kcad).
  { label: 'DXF', script: 'rust:wasm:dxf', out: 'apps/web/src/io/dxf/pkg', lib: 'kentos_dxf_wasm', sources: ['crates/shared/formats', 'crates/shared/kcad', 'crates/wasm/dxf-wasm', 'crates/shared/contracts', 'crates/shared/geometry-core', ...PINS] },
  { label: 'Netcad NCZ', script: 'rust:wasm:ncz', out: 'apps/web/src/io/ncz/pkg', lib: 'kentos_ncz_wasm', sources: ['crates/shared/ncz', 'crates/shared/formats', 'crates/shared/kcad', 'crates/wasm/ncz-wasm', 'crates/shared/contracts', 'crates/shared/geometry-core', ...PINS] },
  // The SVG editor's geometry (loaded with the editor) runs on the core's overlay and writes numbers as the style core does.
  { label: 'SVG düzenleyicisi', script: 'rust:wasm:svg', out: 'apps/web/src/style/svg/pkg', lib: 'kentos_svg_wasm', sources: ['crates/shared/svg-core', 'crates/wasm/svg-wasm', 'crates/shared/geometry-core', 'crates/shared/expression', 'crates/shared/style-core', ...PINS] },
  // Sheet layouts (docs/sheet/design.md), loaded when the sheet mode opens: the core with its templates, metrics and profiles (data it embeds).
  { label: 'Pafta çekirdeği', script: 'rust:wasm:sheet', out: 'apps/web/src/product/sheet/pkg', lib: 'kentos_sheet_wasm', sources: ['crates/shared/sheet', 'crates/wasm/sheet-wasm', 'crates/shared/expression', 'crates/shared/geometry-core', 'crates/shared/contracts', ...PINS] },
  // Map services (docs/adr/0208), loaded when a drawing shows a service or a services window opens: requests,
  // capabilities, tiles and their meshes, vector tiles into the style engine's batches, labels and features.
  { label: 'Harita servisleri', script: 'rust:wasm:services', out: 'apps/web/src/io/services/pkg', lib: 'kentos_services_wasm', sources: ['crates/shared/services', 'crates/wasm/services-wasm', 'crates/shared/formats', 'crates/shared/style-core', 'crates/shared/geometry-core', 'crates/shared/expression', 'crates/shared/contracts', 'fixtures/services/v1/presets.json', 'fixtures/crs/v1/registry.json', ...PINS] },
  // Raster analyses (docs/adr/0231), loaded in their worker when a job starts: the raster core's jobs over the formats' reader.
  { label: 'Raster çözümleme', script: 'rust:wasm:raster', out: 'apps/web/src/io/raster/pkg', lib: 'kentos_raster_wasm', sources: ['crates/shared/raster', 'crates/wasm/raster-wasm', 'crates/shared/formats', 'crates/shared/geometry-core', 'crates/shared/contracts', ...PINS] },
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
  h.update(`profile:${PROFILE}\0`);
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

  console.log(`${pkg.label} (WASM, ${PROFILE}) derleniyor…`);
  const r = spawnSync('nice', ['-n', '10', 'pnpm', '-s', pkg.script], { cwd: ROOT, stdio: 'inherit', env: { ...process.env, KENTOS_WASM_PROFILE: PROFILE } });
  if (r.status !== 0) {
    console.error(`${pkg.label} WASM paketi derlenemedi. Rust araç zinciri kurulu mu? (rust-toolchain.toml, wasm-bindgen-cli 0.2.128; CLAUDE.md §2)`);
    process.exit(r.status ?? 1);
  }
  writeFileSync(stamp, `${want}\n`);
}
