// Build inventory (CLAUDE.md §20.1): builds the app with Vite's API and an
// inline plugin that records every output file, which modules it holds and
// what it imports. Reports, separately:
//   - what the first page requests (entry JS and its static imports, CSS),
//   - lazy chunks (dynamic imports) and their largest modules,
//   - the total size of dist.
// Sizes are raw, gzip and brotli. Writes docs/perf/bundle-<date>.{json,md}.
//
//   node scripts/perf/bundle.mjs [--out docs/perf] [--label baseline]
import { fileURLToPath } from 'node:url';
import { build } from 'vite';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { execSync } from 'node:child_process';

const args = process.argv.slice(2);
const opt = (name, def) => (args.includes(`--${name}`) ? args[args.indexOf(`--${name}`) + 1] : def);
const outDir = resolve(opt('out', fileURLToPath(new URL('../../../../docs/perf', import.meta.url))));
const label = opt('label', 'baseline');

const sizes = (buf) => ({
  raw: buf.length,
  gzip: gzipSync(buf, { level: 9 }).length,
  brotli: brotliCompressSync(buf, { params: { [constants.BROTLI_PARAM_QUALITY]: 11 } }).length,
});

const files = [];
await build({
  logLevel: 'warn',
  build: { manifest: true, emptyOutDir: true },
  plugins: [
    {
      name: 'kentos-inventory',
      generateBundle(_opts, bundle) {
        for (const [file, out] of Object.entries(bundle)) {
          const buf = Buffer.from(out.type === 'chunk' ? out.code : out.source);
          const modules = out.type === 'chunk' ? Object.entries(out.modules ?? {}).map(([id, m]) => ({ id: id.replace(process.cwd() + '/', ''), size: m.renderedLength ?? 0 })) : [];
          files.push({
            file,
            type: out.type,
            isEntry: !!out.isEntry,
            isDynamicEntry: !!out.isDynamicEntry,
            imports: out.imports ?? [],
            dynamicImports: out.dynamicImports ?? [],
            css: out.viteMetadata ? [...(out.viteMetadata.importedCss ?? [])] : [],
            modules: modules.sort((a, b) => b.size - a.size),
            size: sizes(buf),
          });
        }
      },
    },
  ],
});

const byFile = new Map(files.map((f) => [f.file, f]));
// First page: the entry chunks, everything they import statically, and their CSS.
const initial = new Set();
const visit = (file) => {
  if (initial.has(file) || !byFile.has(file)) return;
  initial.add(file);
  const f = byFile.get(file);
  f.imports.forEach(visit);
  f.css.forEach((c) => initial.add(c));
};
files.filter((f) => f.isEntry).forEach((f) => visit(f.file));
const sum = (list) => list.reduce((s, f) => ({ raw: s.raw + f.size.raw, gzip: s.gzip + f.size.gzip, brotli: s.brotli + f.size.brotli }), { raw: 0, gzip: 0, brotli: 0 });
const initialFiles = files.filter((f) => initial.has(f.file));
const report = {
  label,
  date: new Date().toISOString(),
  commit: execSync('git rev-parse --short HEAD').toString().trim(),
  // The reports themselves do not count: a re-run on the same commit is still a clean measurement.
  dirtyTree: execSync("git status --porcelain -- ':(top)' ':(exclude,top)docs/perf'").toString().trim().length > 0,
  initial: {
    js: sum(initialFiles.filter((f) => f.file.endsWith('.js'))),
    css: sum(initialFiles.filter((f) => f.file.endsWith('.css'))),
    files: initialFiles.map((f) => f.file),
  },
  lazy: files
    .filter((f) => f.type === 'chunk' && !initial.has(f.file))
    .map((f) => ({ file: f.file, dynamicEntry: f.isDynamicEntry, size: f.size, topModules: f.modules.slice(0, 8) })),
  total: sum(files),
  entryTopModules: files.filter((f) => f.isEntry).flatMap((f) => f.modules).sort((a, b) => b.size - a.size).slice(0, 25),
  fileCount: files.length,
};

const kb = (n) => `${(n / 1024).toFixed(1)} KB`;
const day = report.date.slice(0, 10);
mkdirSync(outDir, { recursive: true });
writeFileSync(`${outDir}/bundle-${label}-${day}.json`, `${JSON.stringify(report, null, 2)}\n`);
const md = [
  `# Build envanteri: ${label} (${day}, ${report.commit}${report.dirtyTree ? ', çalışma ağacı temiz değil' : ''})`,
  '',
  `Ölçüm: \`node scripts/perf/bundle.mjs --label ${label}\`. Boyutlar ham / gzip (9) / brotli (11).`,
  '',
  '| Kapsam | Ham | gzip | brotli |',
  '|---|---|---|---|',
  `| İlk sayfa JS | ${kb(report.initial.js.raw)} | ${kb(report.initial.js.gzip)} | ${kb(report.initial.js.brotli)} |`,
  `| İlk sayfa CSS | ${kb(report.initial.css.raw)} | ${kb(report.initial.css.gzip)} | ${kb(report.initial.css.brotli)} |`,
  `| Bütün dist (${report.fileCount} dosya) | ${kb(report.total.raw)} | ${kb(report.total.gzip)} | ${kb(report.total.brotli)} |`,
  '',
  '## Giriş chunk\'ının en büyük modülleri (işlenmiş boyut)',
  '',
  '| Modül | Boyut |',
  '|---|---|',
  ...report.entryTopModules.map((m) => `| \`${m.id}\` | ${kb(m.size)} |`),
  '',
  '## İsteğe bağlı (lazy) chunk\'lar',
  '',
  '| Dosya | gzip | En büyük modüller |',
  '|---|---|---|',
  ...report.lazy.map((l) => `| \`${l.file}\` | ${kb(l.size.gzip)} | ${l.topModules.slice(0, 3).map((m) => `\`${m.id.split('/').slice(-2).join('/')}\``).join(', ')} |`),
  '',
].join('\n');
writeFileSync(`${outDir}/bundle-${label}-${day}.md`, md);
console.log(md.split('\n').slice(0, 10).join('\n'));
