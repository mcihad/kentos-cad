// Raster analysis in the browser (docs/adr/0231 §11): how long Yüzey analizi's jobs take in their own worker
// (io/rasterAnalysisWorker.ts, raster-wasm on one thread) over a 4096 × 4096 32-bit DEM, tiled and Deflate'd as the
// desktop's measurement makes it (crates/shared/raster/tests/all/timing.rs, the same hills): Eğim, Eş yükselti eğrileri
// (5 m, about 100 levels), and Güneşlenme over a 2048 × 2048 one (a year, 14 days, half an hour); then a raster from
// points (docs/adr/0232 §14): IDW, Doğal komşu and Kriging, 100 000 points onto a 2048 × 2048 grid; then the raster
// operations (docs/adr/0233 §15) over the 4096 × 4096 DEM: Raster hesaplayıcı, Yeniden sınıflandır, Yeniden örnekle
// (Ortalama, 2×), Komşuluk istatistiği (5 × 5 Ortalama), Histogram and Bölgesel istatistik (10 000 parcels). Starts its own Vite dev
// server and one headless Chrome; the DEMs are written once by GDAL into .run/perf (python3 with numpy and osgeo) and
// fetched by the page as a file the user gave. The worker's whole run is timed: its start, the module, reading the
// file's blocks, the job and the result's coding. Nothing else heavy may run meanwhile (docs/adr/0005).
//
//   node ../../scripts/wasm/ensure.mjs --release && node scripts/perf/raster.mjs [--runs 3] [--only surface|points|ops]
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { cpus } from 'node:os';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const runs = Number(args.includes('--runs') ? args[args.indexOf('--runs') + 1] : '3');
const only = args.includes('--only') ? args[args.indexOf('--only') + 1] : null;
const part = (name) => !only || only === name;
const dir = fileURLToPath(new URL('../../../../.run/perf/', import.meta.url));
mkdirSync(dir, { recursive: true });

/** A hilly DEM of n × n cells of 5 m (timing.rs's), tiled 256, Deflate, at `path`, unless it is there. */
function dem(n) {
  const path = `${dir}dem-${n}.tif`;
  if (existsSync(path)) return path;
  const script = `
import numpy as np
from osgeo import gdal, osr
gdal.UseExceptions()
n = ${n}
x = np.arange(n, dtype=np.float64)[None, :]
y = np.arange(n, dtype=np.float64)[:, None]
z = (400.0 + 120.0 * np.sin(x / 230.0) * np.cos(y / 310.0) + 0.05 * x + 3.0 * np.sin(x / 7.0) * np.cos(y / 9.0)).astype(np.float32)
d = gdal.GetDriverByName('GTiff').Create(${JSON.stringify(path)}, n, n, 1, gdal.GDT_Float32, ['TILED=YES', 'BLOCKXSIZE=256', 'BLOCKYSIZE=256', 'COMPRESS=DEFLATE', 'ZLEVEL=1'])
d.SetGeoTransform([500000.0, 5.0, 0.0, 4420000.0, 0.0, -5.0])
s = osr.SpatialReference(); s.ImportFromEPSG(5254); d.SetProjection(s.ExportToWkt())
d.GetRasterBand(1).WriteArray(z)
d = None
`;
  execFileSync('python3', ['-c', script], { stdio: 'inherit' });
  return path;
}

const TM30 = { kind: 'tm', datum: 'TUREF', centralMeridian: 30, scaleFactor: 1, falseEasting: 500000, falseNorthing: 0 };
const spec = (tool) => JSON.stringify({ tool, band: 1, affine: [500000, 5, 0, 4420000, 0, -5], epsg: 5254, system: TM30 });
const jobs = [
  { name: 'Eğim', n: 4096, budget: 6, spec: spec({ kind: 'slope', method: 'horn', unit: 'degrees', zFactor: 1 }) },
  { name: 'Eş yükselti (5 m)', n: 4096, budget: null, spec: spec({ kind: 'contours', interval: 5, base: 0, indexEvery: 5 }) },
  { name: 'Güneşlenme (yıl)', n: 2048, budget: null, spec: spec({ kind: 'insolation', firstDay: 1, lastDay: 365, dayStep: 14, hourStep: 0.5, transmissivity: 0.5, zFactor: 1 }) },
];

process.env.KENTOS_API_PORT = '9';
const server = await createServer({ server: { port: 0, strictPort: false, hmr: false, watch: null }, logLevel: 'error' });
await server.listen();
const b = await launch(`${server.resolvedUrls.local[0]}?start=0`, { width: 1280, height: 800, args: ['--use-angle=gl', '--ignore-gpu-blocklist'] });
try {
  await b.waitFor('!!window.kentos', 60000);
  console.log(`${cpus()[0].model}, ${cpus().length} çekirdek; işçi tek iş parçacığı`);
  for (const job of part('surface') ? jobs : []) {
    const path = dem(job.n);
    const times = [];
    for (let r = 0; r < runs; r++) {
      const ms = await b.eval(`(async () => {
        const { analyzeRaster } = await import('/src/io/rasterAnalysis.ts');
        const blob = await (await fetch('/@fs${path}')).blob();
        const t0 = performance.now();
        const out = await analyzeRaster(blob, ${JSON.stringify(job.spec)}, { progress() {}, canceled: false });
        const ms = performance.now() - t0;
        if (!('raster' in out) && !('lines' in out)) throw new Error('no result');
        return ms;
      })()`);
      times.push(ms / 1000);
    }
    times.sort((a, c) => a - c);
    const p50 = times[Math.floor(times.length / 2)];
    console.log(`${job.name.padEnd(20)} ${job.n}²  p50 ${p50.toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s${job.budget ? `  bütçe ${job.budget} s` : ''}`);
  }
  // İnterpolasyon (docs/adr/0232 §14): 100 000 random points over 2 km with a hill's heights (point_timing.rs's), a
  // 2048 × 2048 grid of a metre; the objects' JSON made in the page, the worker's whole run timed.
  const grid = { affine: [500000, 1, 0, 4422048, 0, -1], width: 2048, height: 2048 };
  const pointJobs = [
    { name: 'IDW (12 nokta)', tool: { kind: 'idw', power: 2, points: 12 } },
    { name: 'Doğal komşu', tool: { kind: 'naturalNeighbor' } },
    { name: 'Kriging (küresel, 12)', tool: { kind: 'kriging', model: 'spherical', variogram: { fit: 'manual', nugget: 0, sill: 1800, range: 400 }, points: 12 } },
  ];
  for (const job of part('points') ? pointJobs : []) {
    const times = [];
    for (let r = 0; r < Math.min(runs, 2); r++) {
      const ms = await b.eval(`(async () => {
        const { analyzePoints } = await import('/src/io/rasterAnalysis.ts');
        let s = 0x5eedn;
        const next = () => { s ^= s >> 12n; s ^= (s << 25n) & 0xffffffffffffffffn; s ^= s >> 27n; return Number(((s * 0x2545f4914f6cdd1dn) & 0xffffffffffffffffn) >> 11n) / 2 ** 53; };
        const pts = [];
        for (let i = 0; i < 100000; i++) {
          const x = next() * 2048, y = next() * 2048;
          pts.push({ kind: 'point', p: { x: 500000 + x, y: 4420000 + y }, z: 400 + 60 * Math.sin(x / 230) * Math.cos(y / 310) + 0.05 * x });
        }
        const objects = JSON.stringify(pts);
        const spec = JSON.stringify({ tool: ${JSON.stringify(job.tool)}, grid: ${JSON.stringify(grid)}, epsg: 5254 });
        const t0 = performance.now();
        const out = await analyzePoints(objects, 'null', spec, false, { progress() {}, canceled: false });
        const ms = performance.now() - t0;
        if (!out.bytes.length) throw new Error('no result');
        return ms;
      })()`);
      times.push(ms / 1000);
    }
    times.sort((a, c) => a - c);
    console.log(`${job.name.padEnd(22)} 2048², 10⁵ nokta  p50 ${times[Math.floor(times.length / 2)].toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s`);
  }
  // Raster işlemleri (docs/adr/0233 §15): the 4096 × 4096 DEM as the operations' one input; Bölgesel istatistik's
  // 10 000 parcels made in the page, 100 × 100 squares over the raster's 20 480 m (as ops_timing.rs's cover its own).
  const opsJobs = [
    { name: 'Hesaplayıcı (iki raster)', tool: { kind: 'calculator', expression: '[A] * 2 + [B]', empty: 'propagate', sample: 'f32' }, two: true },
    { name: 'Yeniden sınıflandır', tool: { kind: 'reclassify', band: 1, table: '* 350 1; 350 400 2; 400 450 3; 450 * 4', bounds: 'upperClosed', unmatched: 'keep', sample: 'u8' } },
    { name: 'Yeniden örnekle (2×)', tool: { kind: 'resample', cell: 10, method: 'mean' } },
    { name: 'Komşuluk (5 × 5)', tool: { kind: 'focalStatistics', band: 1, shape: 'rect', width: 5, height: 5, radius: 3, inner: 1, stat: 'mean', ignore: true } },
    { name: 'Histogram', tool: { kind: 'histogram', band: 1, bins: 64, min: null, max: null } },
    { name: 'Bölgesel (10⁴ parsel)', tool: { kind: 'zonalStatistics', band: 1, stat: 'mean' }, parcels: true },
  ];
  const path = dem(4096);
  for (const job of part('ops') ? opsJobs : []) {
    const times = [];
    for (let r = 0; r < runs; r++) {
      const ms = await b.eval(`(async () => {
        const { analyzeOps } = await import('/src/io/rasterAnalysis.ts');
        const blob = await (await fetch('/@fs${path}')).blob();
        const shapes = [];
        if (${!!job.parcels}) {
          const side = 20480 / 100;
          for (let j = 0; j < 100; j++) for (let i = 0; i < 100; i++) {
            const [x0, y0] = [500000 + i * side + 3, 4420000 - 20480 + j * side + 3];
            const [x1, y1] = [x0 + side - 6, y0 + side - 6];
            shapes.push({ kind: 'polygon', pts: [{ x: x0, y: y0 }, { x: x1, y: y0 }, { x: x1, y: y1 }, { x: x0, y: y1 }] });
          }
        }
        const names = ${job.two ? "['A', 'B']" : "['A']"};
        const spec = JSON.stringify({ tool: ${JSON.stringify(job.tool)}, inputs: names.map((name) => ({ affine: [500000, 5, 0, 4420000, 0, -5], name })), epsg: 5254 });
        const t0 = performance.now();
        const out = await analyzeOps(names.map(() => blob), spec, JSON.stringify(shapes), { progress() {}, canceled: false });
        const ms = performance.now() - t0;
        if (!out.bytes && !out.zones && !out.histogram) throw new Error('no result');
        return ms;
      })()`);
      times.push(ms / 1000);
    }
    times.sort((a, c) => a - c);
    console.log(`${job.name.padEnd(22)} 4096²  p50 ${times[Math.floor(times.length / 2)].toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s`);
  }
} finally {
  b.close();
  await server.close();
}
