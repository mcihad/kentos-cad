// Dependency direction check (TODOS.md ARCH-01, docs/adr/0010): every Rust
// crate depends only in the allowed direction, and no runtime of another
// platform reaches it, not even through a dependency of a dependency.
//
// Each crate is checked on the targets it is built for (the pure libraries
// for the host and wasm32, the browser bindings for wasm32, the server for
// the host) with `cargo tree`: the graph as Cargo builds it, features and
// target filters applied, normal and build dependencies (dev-dependencies do
// not ship). `cargo metadata`'s resolve is not used: it also lists optional
// dependencies no enabled feature asks for.
//
// A crate under a path no group claims stops the check: the first desktop,
// UI, renderer or native crate must be placed here deliberately.
//
//   node scripts/arch/deps.mjs      (runs at the end of pnpm rust:test)
import { execFileSync } from 'node:child_process';
import { relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const cargo = (args) => execFileSync('cargo', args, { cwd: ROOT, encoding: 'utf8', maxBuffer: 64 << 20, stdio: ['ignore', 'pipe', 'pipe'] });
const HOST = execFileSync('rustc', ['-vV'], { encoding: 'utf8' }).match(/^host: (.+)$/m)[1];
const WASM = 'wasm32-unknown-unknown';

/**
 * Groups by path: the targets they are built for, the workspace groups they
 * may use, and the external crates (a name, or a prefix ending in `*`) that
 * must not appear anywhere below them. Paths not in use yet follow TODOS.md §2.1.
 */
const RUNTIMES = ['tokio', 'sqlx*', 'axum*', 'hyper*', 'tower*', 'reqwest'];
const BROWSER = ['wasm-bindgen*', 'js-sys', 'web-sys'];
const DESKTOP = ['iced*', 'wgpu*', 'winit', 'naga'];
const GROUPS = [
  { name: 'shared', path: 'crates/shared/', targets: [HOST, WASM], uses: ['shared'], forbid: [...RUNTIMES, ...BROWSER, ...DESKTOP, 'pyo3*', 'gdal*', 'proj', 'proj-sys'] },
  { name: 'wasm', path: 'crates/wasm/', targets: [WASM], uses: ['shared'], forbid: [...RUNTIMES, ...DESKTOP, 'pyo3*'] },
  // The native drawing document (docs/adr/0020): innermost native layer, pure like the shared
  // libraries but never built for the browser (the web keeps its TypeScript document). Listed
  // before `native`, whose path contains it. Only the contracts and other shared libraries below it.
  { name: 'domain', path: 'crates/native/domain/', targets: [HOST], uses: ['shared'], forbid: [...RUNTIMES, ...BROWSER, ...DESKTOP, 'pyo3*', 'gdal*', 'proj', 'proj-sys'] },
  // The native tool session (docs/adr/0021): tools, prompts and typed input over the document.
  // Pure like the document: no Iced, window system or GPU (the desktop feeds it input and
  // draws its preview), no runtime, no browser. Listed before `native`, whose path contains it.
  { name: 'interaction', path: 'crates/native/interaction/', targets: [HOST], uses: ['shared', 'domain'], forbid: [...RUNTIMES, ...BROWSER, ...DESKTOP, 'pyo3*', 'gdal*', 'proj', 'proj-sys'] },
  { name: 'native', path: 'crates/native/', targets: [HOST], uses: ['shared', 'domain', 'native'], forbid: ['sqlx*', 'axum*', ...BROWSER, ...DESKTOP, 'pyo3*'] },
  { name: 'server', path: 'crates/server/', targets: [HOST], uses: ['shared', 'domain', 'native', 'server'], forbid: [...BROWSER, ...DESKTOP] },
  { name: 'api', path: 'apps/api/', targets: [HOST], uses: ['shared', 'domain', 'native', 'server'], forbid: [...BROWSER, ...DESKTOP] },
  // The UI component library (docs/adr/0016): widgets, theme, icons; no domain, no runtime of its own.
  { name: 'ui', path: 'crates/ui/', targets: [HOST], uses: [], forbid: [...RUNTIMES, ...BROWSER, 'pyo3*'] },
  // The native renderer (docs/adr/0019): wgpu on the host's device, fed by the shared core.
  // No Iced or window system (the desktop app plugs it into Iced), no runtime, no browser.
  { name: 'render', path: 'crates/render/', targets: [HOST], uses: ['shared'], forbid: [...RUNTIMES, ...BROWSER, 'iced*', 'winit', 'pyo3*'] },
  // Desktop programs: Iced's executor may be tokio; no server framework, no browser bindings.
  { name: 'desktop', path: 'apps/desktop/', targets: [HOST], uses: ['shared', 'domain', 'interaction', 'native', 'ui', 'render'], forbid: ['axum*', ...BROWSER] },
  { name: 'desktop', path: 'apps/ui-showcase/', targets: [HOST], uses: ['shared', 'domain', 'interaction', 'native', 'ui', 'render'], forbid: ['axum*', ...BROWSER] },
];

const meta = JSON.parse(cargo(['metadata', '--format-version', '1', '--locked', '--no-deps']));
const members = meta.packages.map((p) => ({ name: p.name, path: relative(ROOT, p.manifest_path) }));
const groupOfPath = (path) => GROUPS.find((g) => path.startsWith(g.path));
const matches = (name, pattern) => (pattern.endsWith('*') ? name.startsWith(pattern.slice(0, -1)) : name === pattern);

const problems = [];
let checked = 0;
for (const member of members) {
  const group = groupOfPath(member.path);
  if (!group) {
    problems.push(`${member.name}: ${member.path} hiçbir gruba ait değil; scripts/arch/deps.mjs'e grubunu ve kurallarını ekleyin (ADR 0010).`);
    continue;
  }
  for (const target of group.targets) {
    checked++;
    const tree = cargo(['tree', '--locked', '-p', member.name, '-e', 'normal,build', '--target', target, '--prefix', 'depth', '--format', '{p}']);
    const stack = [];
    for (const line of tree.split('\n')) {
      // "3serde v1.0.229", "1kentos-contracts v0.1.0 (/abs/path) (*)", "2serde_derive v1.0.229 (proc-macro)"
      const m = line.match(/^(\d+)(\S+) v\S+(?: \(([^)]*)\))?/);
      if (!m) continue;
      const depth = Number(m[1]);
      stack.length = depth;
      stack.push(m[2]);
      if (depth === 0) continue;
      const where = `${stack.join(' → ')} [${target === HOST ? 'makine' : target}]`;
      const local = m[3]?.startsWith('/') ? relative(ROOT, m[3]) : null;
      if (local) {
        const used = groupOfPath(`${local}/`);
        if (!used) problems.push(`${where}: ${local} hiçbir gruba ait değil.`);
        else if (!group.uses.includes(used.name)) problems.push(`${where}: ${group.name} grubu ${used.name} grubuna bağlanamaz.`);
      } else if (group.forbid.some((p) => matches(m[2], p))) {
        problems.push(`${where}: ${group.name} grubunda ${m[2]} bulunamaz.`);
      }
    }
  }
}

if (problems.length) {
  console.error(`Bağımlılık yönü bozuldu (ADR 0010, TODOS.md ARCH-01):\n${[...new Set(problems)].map((p) => `  ${p}`).join('\n')}`);
  process.exit(1);
}
console.log(`Bağımlılık yönü temiz: ${members.length} crate, ${checked} crate × hedef denetlendi.`);
