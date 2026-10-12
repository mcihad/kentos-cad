// Katman çizimi and draw effects in a real browser (docs/adr/0226 §2–§4): the scenes of fixtures/compose/v1/cases.json
// (written by scripts/fixtures/effects_cases.py from the ADR's definitions with numpy, without KentOS code) drawn by
// the app's own WebGPU and WebGL2 backends, single-sampled and with 4× multisampling, read back and compared pixel by
// pixel within the fixture's tolerance. The desktop's twin is crates/render/wgpu/tests/gpu/compose.rs.
//
// Headless Chrome gets WebGPU on SwiftShader and WebGL2 on ANGLE's Vulkan with the flags of ./cdp.mjs: a check of
// the method, not a GPU benchmark (CLAUDE.md §9.5). With --time, Chrome draws on the machine's GPU (WebGPU on its
// Vulkan adapter, WebGL2 through ANGLE's Vulkan) and the desktop timing's scene is drawn instead (docs/adr/0226 §7:
// the composed frame's cost against the budget, 2.5 times the desktop's 8 ms).
//
//   (cd apps/web && node scripts/e2e/compose.mjs [--time])
import { createServer } from 'vite';
import { launch, WEBGPU_ARGS } from './cdp.mjs';

const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const url = server.resolvedUrls.local[0];
const timing = process.argv.includes('--time');
const GPU_ARGS = ['--enable-unsafe-webgpu', '--enable-features=Vulkan', '--use-angle=vulkan', '--ignore-gpu-blocklist'];
const b = await launch(`${url}scripts/e2e/compose/index.html`, { args: timing ? GPU_ARGS : WEBGPU_ARGS });
const failures = [];
if (timing) {
  try {
    await b.waitFor('typeof window.composeTime === "function"', 30000);
    console.log('ADR 0226 §7: 1440 × 900, the base drawn again a frame and waited for; medians of 60 frames, the scenes in turn');
    for (const kind of ['webgpu', 'webgl2']) {
      for (const samples of [1, 4]) {
        const r = await b.eval(`window.composeTime(${JSON.stringify(kind)}, ${samples})`);
        if (r.error) {
          console.log(`✗ ${kind} ${samples}×: ${r.error}`);
          failures.push(`${kind} ${samples}×`);
          continue;
        }
        const base = r.rows[0][1];
        const added = r.rows[1][1] - base;
        console.log(`${kind} ${samples}× (${r.label}): ${added.toFixed(2)} ms   bütçe 20 ms  ${added <= 20 ? '✓' : '✗'}   (yalın ${base.toFixed(2)} ms, birleştirmeyle ${r.rows[1][1].toFixed(2)} ms)`);
        for (const [name, ms] of r.rows.slice(2)) console.log(`    ${name}: ${(ms - base).toFixed(2)} ms`);
        if (added > 20) failures.push(`${kind} ${samples}× over the budget`);
      }
    }
  } finally {
    await b.close();
    await server.close();
  }
  process.exit(failures.length ? 1 : 0);
}
try {
  await b.waitFor('typeof window.composeRun === "function"', 30000);
  for (const kind of ['webgpu', 'webgl2']) {
    for (const samples of [1, 4]) {
      const r = await b.eval(`window.composeRun(${JSON.stringify(kind)}, ${samples})`);
      if (r.error) {
        console.log(`✗ ${kind} ${samples}×: ${r.error}`);
        failures.push(`${kind} ${samples}×`);
        continue;
      }
      console.log(`${kind} ${samples}× (${r.label})`);
      for (const c of r.results) {
        const ok = c.off === 0;
        console.log(`  ${ok ? '✓' : '✗'} ${c.name}: largest difference ${c.worst}/255 at (${c.at.join(', ')})${ok ? '' : `, ${c.off} channels over the tolerance: ${c.got.join(' ')} drawn, ${c.want.join(' ')} expected`}`);
        if (!ok) failures.push(`${kind} ${samples}× ${c.name}`);
      }
    }
  }
} finally {
  await b.close();
  await server.close();
}
if (failures.length) {
  console.log(`\n${failures.length} failed: ${failures.join('; ')}`);
  process.exit(1);
}
console.log('\nall scenes match the reference');
