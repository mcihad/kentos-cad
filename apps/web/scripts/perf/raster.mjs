// Raster analysis in the browser (docs/adr/0231 §11): how long Yüzey analizi's jobs take in their own worker
// (io/rasterAnalysisWorker.ts, raster-wasm on one thread) over a 4096 × 4096 32-bit DEM, tiled and Deflate'd as the
// desktop's measurement makes it (crates/shared/raster/tests/all/timing.rs, the same hills): Eğim, Eş yükselti eğrileri
// (5 m, about 100 levels), and Güneşlenme over a 2048 × 2048 one (a year, 14 days, half an hour); then a raster from
// points (docs/adr/0232 §14): IDW, Doğal komşu and Kriging, 100 000 points onto a 2048 × 2048 grid; then the raster
// operations (docs/adr/0233 §15) over the 4096 × 4096 DEM: Raster hesaplayıcı, Yeniden sınıflandır, Yeniden örnekle
// (Ortalama, 2×), Komşuluk istatistiği (5 × 5 Ortalama), Histogram and Bölgesel istatistik (10 000 parcels); then raster and
// vector (docs/adr/0234 §11, vector_timing.rs's work): Rasterleştir (10 000 parcels onto 4096²), Rasterden alan (a 4096²
// class raster of some 50 000 regions), Rasterden çizgi (4096², about 3 % line cells), Rasterden nokta (Adım 10), Çizgi
// yakala and Alan kapat on an 8192² scanned sheet, Eğrilere kot ver over 10 000 curves (in the page); then hydrology
// (docs/adr/0235 §12, hydro_timing.rs's work): every tool over its 4096² DEM with pits and flats; then distance and cost
// (docs/adr/0236 §8, distance_timing.rs's work): each tool over its 4096² cost raster with lakes, the sources' raster and
// hydrology's DEM as the surface, Uzaklık yüzeyi from 2000 points and 100 lines; then suitability (docs/adr/0237 §11,
// suitability_timing.rs's work): four 4096² criteria, each tool's run and ROC with 10 000 presence cells. Starts its own Vite dev
// server and one headless Chrome; the DEMs are written once by GDAL into .run/perf (python3 with numpy and osgeo) and
// fetched by the page as a file the user gave. The worker's whole run is timed: its start, the module, reading the
// file's blocks, the job and the result's coding. Nothing else heavy may run meanwhile (docs/adr/0005).
//
//   node ../../scripts/wasm/ensure.mjs --release && node scripts/perf/raster.mjs [--runs 3] [--only surface|points|ops|vector|hydro|distance|suitability]
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

/** hydro_timing.rs's DEM: the same hills with a little noise (pits and flats as a real model has them), 5 m, at `path`. */
function hydroDem(n) {
  const path = `${dir}dem-hidro-${n}.tif`;
  if (existsSync(path)) return path;
  const script = `
import numpy as np
from osgeo import gdal, osr
gdal.UseExceptions()
n = ${n}
i = np.arange(n, dtype=np.uint32)[None, :]
j = np.arange(n, dtype=np.uint32)[:, None]
h = (i * np.uint32(0x9e3779b9)) ^ (j * np.uint32(0x85ebca6b))
h ^= h >> np.uint32(15)
h = h * np.uint32(0x2c1b3c6d)
h ^= h >> np.uint32(12)
noise = (h % np.uint32(1000)).astype(np.float64) / 1000.0
x = np.arange(n, dtype=np.float64)[None, :]
y = np.arange(n, dtype=np.float64)[:, None]
z = (400.0 + 120.0 * np.sin(x / 230.0) * np.cos(y / 310.0) + 0.05 * x + 3.0 * np.sin(x / 7.0) * np.cos(y / 9.0) + 0.8 * noise).astype(np.float32)
d = gdal.GetDriverByName('GTiff').Create(${JSON.stringify(path)}, n, n, 1, gdal.GDT_Float32, ['TILED=YES', 'BLOCKXSIZE=256', 'BLOCKYSIZE=256', 'COMPRESS=DEFLATE', 'ZLEVEL=1'])
d.SetGeoTransform([500000.0, 5.0, 0.0, 4420000.0, 0.0, -5.0])
s = osr.SpatialReference(); s.ImportFromEPSG(5254); d.SetProjection(s.ExportToWkt())
d.GetRasterBand(1).WriteArray(z)
d = None
`;
  execFileSync('python3', ['-c', script], { stdio: 'inherit' });
  return path;
}

/** distance_timing.rs's rasters of n × n cells of 5 m: the cost (dearer up the hills, lakes without a value) and the sources'. */
function distanceRasters(n) {
  const cost = `${dir}maliyet-${n}.tif`;
  const wells = `${dir}kaynaklar-${n}.tif`;
  if (existsSync(cost) && existsSync(wells)) return { cost, wells };
  const script = `
import numpy as np
from osgeo import gdal, osr
gdal.UseExceptions()
n = ${n}
def hash(i, j):
    h = (i * np.uint32(0x9e3779b9)) ^ (j * np.uint32(0x85ebca6b))
    h ^= h >> np.uint32(15)
    h = h * np.uint32(0x2c1b3c6d)
    return h ^ (h >> np.uint32(12))
i = np.arange(n, dtype=np.uint32)[None, :]
j = np.arange(n, dtype=np.uint32)[:, None]
x = np.arange(n, dtype=np.float64)[None, :]
y = np.arange(n, dtype=np.float64)[:, None]
lake = np.sin(x / 97.0) * np.cos(y / 131.0) + 0.3 * np.sin((x + y) / 41.0)
cost = 1.0 + 4.0 * np.abs(np.sin(x / 230.0) * np.cos(y / 310.0)) + (hash(i, j) % np.uint32(100)).astype(np.float64) / 100.0
cost = np.where(lake > 1.05, np.nan, cost).astype(np.float32)
wells = np.where(hash(i, j) % np.uint32(20000) == 0, (hash(j, i) % np.uint32(50)).astype(np.float64) + 1.0, np.nan).astype(np.float32)
for path, z in ((${JSON.stringify(cost)}, cost), (${JSON.stringify(wells)}, wells)):
    d = gdal.GetDriverByName('GTiff').Create(path, n, n, 1, gdal.GDT_Float32, ['TILED=YES', 'BLOCKXSIZE=256', 'BLOCKYSIZE=256', 'COMPRESS=DEFLATE', 'ZLEVEL=1'])
    d.SetGeoTransform([500000.0, 5.0, 0.0, 4420000.0, 0.0, -5.0])
    s = osr.SpatialReference(); s.ImportFromEPSG(5254); d.SetProjection(s.ExportToWkt())
    d.GetRasterBand(1).SetNoDataValue(float('nan'))
    d.GetRasterBand(1).WriteArray(z)
    d = None
`;
  execFileSync('python3', ['-c', script], { stdio: 'inherit' });
  return { cost, wells };
}

/** suitability_timing.rs's criteria of n × n cells of 5 m: a slope with holes, a distance, classes and a membership. */
function suitabilityRasters(n) {
  const names = ['egim', 'uzaklik', 'ortu', 'uyelik'].map((k) => `${dir}olcut-${k}-${n}.tif`);
  if (names.every((f) => existsSync(f))) return names;
  const script = `
import numpy as np
from osgeo import gdal, osr
gdal.UseExceptions()
n = ${n}
def hash(i, j):
    h = (i * np.uint32(0x9e3779b9)) ^ (j * np.uint32(0x85ebca6b))
    h ^= h >> np.uint32(15)
    h = h * np.uint32(0x2c1b3c6d)
    return h ^ (h >> np.uint32(12))
i = np.arange(n, dtype=np.uint32)[None, :]
j = np.arange(n, dtype=np.uint32)[:, None]
x = np.arange(n, dtype=np.float64)[None, :]
y = np.arange(n, dtype=np.float64)[:, None]
hole = np.sin(x / 97.0) * np.cos(y / 131.0) + 0.3 * np.sin((x + y) / 41.0) > 1.15
slope = 40.0 * np.abs(np.sin(x / 230.0) * np.cos(y / 310.0)) + (hash(i, j) % np.uint32(100)).astype(np.float64) / 20.0
slope = np.where(hole, np.nan, slope)
dist = 5.0 * (np.hypot(x - 1800.0, y - 2300.0) % 900.0)
cls = (1 + (i // np.uint32(37) + j // np.uint32(53) + hash(i // np.uint32(37), j // np.uint32(53))) % np.uint32(5)).astype(np.float64)
mem = (hash(j, i) % np.uint32(1001)).astype(np.float64) / 1000.0
for path, z in zip(${JSON.stringify(names)}, (slope, dist + 0 * y, cls, mem)):
    d = gdal.GetDriverByName('GTiff').Create(path, n, n, 1, gdal.GDT_Float32, ['TILED=YES', 'BLOCKXSIZE=256', 'BLOCKYSIZE=256', 'COMPRESS=DEFLATE', 'ZLEVEL=1'])
    d.SetGeoTransform([500000.0, 5.0, 0.0, 4420000.0, 0.0, -5.0])
    s = osr.SpatialReference(); s.ImportFromEPSG(5254); d.SetProjection(s.ExportToWkt())
    d.GetRasterBand(1).SetNoDataValue(float('nan'))
    d.GetRasterBand(1).WriteArray(z.astype(np.float32))
    d = None
`;
  execFileSync('python3', ['-c', script], { stdio: 'inherit' });
  return names;
}

/** A raster of n × n cells of 2 m written by `body` (numpy: `img`, uint8, `n`), tiled 256, Deflate, at `path`, unless it is there. */
function made(name, n, bands, body) {
  const path = `${dir}${name}-${n}.tif`;
  if (existsSync(path)) return path;
  const script = `
import numpy as np
from osgeo import gdal
gdal.UseExceptions()
n = ${n}
j = np.arange(n, dtype=np.int64)[:, None]
i = np.arange(n, dtype=np.int64)[None, :]
${body}
d = gdal.GetDriverByName('GTiff').Create(${JSON.stringify(path)}, n, n, ${bands}, gdal.GDT_Byte, ['TILED=YES', 'BLOCKXSIZE=256', 'BLOCKYSIZE=256', 'COMPRESS=DEFLATE', 'ZLEVEL=1'])
d.SetGeoTransform([500000.0, 2.0, 0.0, 4420000.0, 0.0, -2.0])
for k in range(${bands}):
    d.GetRasterBand(k + 1).WriteArray(img if ${bands} == 1 else img[:, :, k])
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
  // Raster ve vektör (docs/adr/0234 §11): vector_timing.rs's work on the same sizes.
  if (part('vector')) {
    const classes = made('siniflar', 4096, 1, `h = ((i // 18) * 2654435761 ^ (j // 18) * 2246822507) & 0xffffffff
h ^= h >> 15
img = (h % 50).astype(np.uint8)`);
    const lines = made('cizgiler', 4096, 1, `k = np.round(j / 100.0) * 100.0
centre = k + 20.0 * np.sin(i / 180.0 + k / 100.0)
img = (np.abs(j - centre) <= 1.0).astype(np.uint8)`);
    const sheet = made('pafta', 8192, 3, `grid = (i % 400 < 3) | (j % 400 < 3)
contour = np.abs(j - 4100.0 - 900.0 * np.sin(i / 1300.0)) <= 1.5
img = np.empty((n, n, 3), dtype=np.uint8)
img[...] = 250
img[np.broadcast_to(grid, (n, n))] = (25, 25, 25)
img[np.broadcast_to(contour, (n, n))] = (150, 80, 30)`);
    const opsRun = (file, tool, cell) => `(async () => {
      const { analyzeOps } = await import('/src/io/rasterAnalysis.ts');
      const blob = await (await fetch('/@fs${file}')).blob();
      const spec = JSON.stringify({ tool: ${JSON.stringify(tool)}, inputs: [{ affine: [500000, ${cell}, 0, 4420000, 0, -${cell}], name: 'A' }], epsg: 5254 });
      const t0 = performance.now();
      const out = await analyzeOps([blob], spec, '[]', { progress() {}, canceled: false });
      const ms = performance.now() - t0;
      if (!out.features) throw new Error('no features');
      return ms;
    })()`;
    const at = (i, j) => ({ x: 500000 + 2 * i + 1, y: 4420000 - 2 * j - 1 });
    const vectorJobs = [
      { name: 'Rasterden alan', size: '4096²', budget: 8, run: opsRun(classes, { kind: 'toPolygons', band: 1, connect: 'four' }, 2) },
      { name: 'Rasterden çizgi', size: '4096²', budget: 8, run: opsRun(lines, { kind: 'toLines', band: 1, select: 'nonZero', spur: 3, simplify: 1 }, 2) },
      { name: 'Rasterden nokta (Adım 10)', size: '4096²', budget: 2, run: opsRun(path, { kind: 'toPoints', band: 1, mode: 'step', step: 10 }, 5) },
      {
        name: 'Çizgi yakala (eğri)',
        size: '8192²',
        budget: 1.5,
        run: opsRun(sheet, { kind: 'captureLine', ...at(3000, 4100 + 900 * Math.sin(3000 / 1300)), tolerance: 60, spur: 5, simplify: 1 }, 2),
      },
      { name: 'Alan kapat', size: '8192²', budget: 1.5, run: opsRun(sheet, { kind: 'closeArea', ...at(1810, 1190), tolerance: 60, holes: 'fill', simplify: 1 }, 2) },
      {
        name: 'Rasterleştir (10⁴ parsel)',
        size: '4096²',
        budget: 4,
        run: `(async () => {
          const { analyzePoints } = await import('/src/io/rasterAnalysis.ts');
          const shapes = [], values = [];
          for (let j = 0; j < 100; j++) for (let i = 0; i < 100; i++) {
            const [x0, y0] = [500000 + i * 81.92 + 3, 4420000 - 8192 + j * 81.92 + 3];
            shapes.push({ kind: 'polygon', pts: [{ x: x0, y: y0 }, { x: x0 + 70, y: y0 + 5 }, { x: x0 + 75, y: y0 + 72 }, { x: x0 - 2, y: y0 + 66 }] });
            values.push(String(shapes.length - 1));
          }
          const spec = JSON.stringify({ tool: { kind: 'rasterize', value: 1, overlap: 'last', sample: 'i32' }, grid: { affine: [500000, 2, 0, 4420000, 0, -2], width: 4096, height: 4096 }, epsg: 5254 });
          const t0 = performance.now();
          const out = await analyzePoints(JSON.stringify(shapes), JSON.stringify(values), spec, true, { progress() {}, canceled: false });
          const ms = performance.now() - t0;
          if (!out.bytes.length) throw new Error('no result');
          return ms;
        })()`,
      },
      {
        name: 'Eğrilere kot ver (10⁴ eğri)',
        size: 'sayfada',
        budget: 0.5,
        run: `(async () => {
          const { contourElevations } = await import('/src/model/ops/contourElevations.ts');
          const curves = [];
          for (let k = 0; k < 10000; k++) {
            const pts = [];
            for (let q = 0; q < 60; q++) pts.push({ x: 500000 + q * 50, y: 4420000 + k * 2 + 0.6 * Math.sin(q * 50 / 300 + k * 0.01) });
            curves.push({ kind: 'polyline', pts });
          }
          const t0 = performance.now();
          const z = contourElevations(curves, { x: 501234, y: 4419995 }, { x: 501500, y: 4440010 }, 100, 1);
          const ms = performance.now() - t0;
          if (z.filter((v) => v !== null).length !== 10000) throw new Error('not every curve');
          return ms;
        })()`,
      },
    ];
    for (const job of vectorJobs) {
      const times = [];
      for (let r = 0; r < runs; r++) times.push((await b.eval(job.run)) / 1000);
      times.sort((a, c) => a - c);
      console.log(`${job.name.padEnd(26)} ${job.size}  p50 ${times[Math.floor(times.length / 2)].toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s  bütçe ${job.budget} s`);
    }
  }
  // Hidroloji (docs/adr/0235 §12): hydro_timing.rs's jobs over its DEM, the whole worker run each (the result written or
  // the objects made); Noktadan havza with its ten points.
  if (part('hydro')) {
    const file = hydroDem(4096);
    const points = Array.from({ length: 10 }, (_, k) => ({ kind: 'point', p: { x: 500000 + 2000 * k + 777, y: 4420000 - 1500 * (k % 7) - 999 } }));
    const hydroJobs = [
      { name: 'Çukur doldur', budget: 6, tool: { kind: 'fill', band: 1, slope: 0, result: 'filled' } },
      { name: 'Akış yönü', budget: 8, tool: { kind: 'flowDirection', band: 1, fill: true, coding: 'esri' } },
      { name: 'Akış birikimi, D8', budget: 10, tool: { kind: 'flowAccumulation', band: 1, fill: true, method: 'd8', exponent: 0, unit: 'cells' } },
      { name: 'Akış birikimi, Çoklu yön', budget: 16, tool: { kind: 'flowAccumulation', band: 1, fill: true, method: 'mfd', exponent: 0, unit: 'cells' } },
      { name: 'Akış birikimi, D∞', budget: 16, tool: { kind: 'flowAccumulation', band: 1, fill: true, method: 'dinf', exponent: 0, unit: 'cells' } },
      { name: 'Topografik nemlilik indisi', budget: 18, tool: { kind: 'wetness', band: 1, fill: true, method: 'mfd', exponent: 0, slope: 0.1 } },
      { name: 'Havzalar, ana havzalar', budget: 12, tool: { kind: 'basins', band: 1, fill: true, mode: 'main', threshold: 0, least: 0 } },
      { name: 'Dere ağı', budget: 12, tool: { kind: 'streams', band: 1, fill: true, threshold: 0, simplify: 1 } },
      { name: 'Noktadan havza, 10 nokta', budget: 10, tool: { kind: 'watershed', band: 1, fill: true, snap: 50 }, shapes: points },
    ];
    for (const job of hydroJobs) {
      const times = [];
      for (let r = 0; r < runs; r++) {
        const ms = await b.eval(`(async () => {
          const { analyzeOps } = await import('/src/io/rasterAnalysis.ts');
          const blob = await (await fetch('/@fs${file}')).blob();
          const spec = JSON.stringify({ tool: ${JSON.stringify(job.tool)}, inputs: [{ affine: [500000, 5, 0, 4420000, 0, -5], name: 'A' }], epsg: 5254 });
          const t0 = performance.now();
          const out = await analyzeOps([blob], spec, ${JSON.stringify(JSON.stringify(job.shapes ?? []))}, { progress() {}, canceled: false });
          const ms = performance.now() - t0;
          if (!out.bytes && !out.features) throw new Error('no result');
          return ms;
        })()`);
        times.push(ms / 1000);
      }
      times.sort((a, c) => a - c);
      console.log(`${job.name.padEnd(28)} 4096²  p50 ${times[Math.floor(times.length / 2)].toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s  bütçe ${job.budget} s`);
    }
  }
  // Uzaklık ve maliyet (docs/adr/0236 §8): distance_timing.rs's jobs, the whole worker run each (the result written or
  // the paths made); the surface the hydrology DEM; Uzaklık yüzeyi from objects as the point job.
  if (part('distance')) {
    const { cost, wells } = distanceRasters(4096);
    const surface = hydroDem(4096);
    const at = (x, y) => ({ kind: 'point', p: { x, y } });
    const sources = Array.from({ length: 10 }, (_, k) => at(500000 + 2000 * k + 777, 4420000 - 1500 * (k % 7) - 999));
    const far = (k) => at(500000 + 20000 - 1700 * k - 333, 4420000 - 19000 + 2100 * k);
    const path = [at(500777, 4419001), ...Array.from({ length: 5 }, (_, k) => far(k))];
    const ends = [at(500777, 4419001), at(519777, 4400667)];
    const net = { band: 1, neighbours: '16', surfaceLength: false, slope: 0 };
    const distanceJobs = [
      { name: 'Uzaklık yüzeyi, rasterden', budget: 3, files: [wells], tool: { kind: 'distance', band: 1, max: 0, result: 'distance' } },
      { name: 'Uzaklık yüzeyi, en yakın kaynak', budget: 3, files: [wells], tool: { kind: 'distance', band: 1, max: 0, result: 'allocation' } },
      { name: 'Birikimli maliyet, 8 komşu', budget: 10, files: [cost], tool: { kind: 'costDistance', ...net, neighbours: '8', max: 0, result: 'cost', sample: 'f32' }, shapes: sources },
      { name: 'Birikimli maliyet, 16 komşu', budget: 12, files: [cost], tool: { kind: 'costDistance', ...net, max: 0, result: 'cost', sample: 'f32' }, shapes: sources },
      { name: 'Birikimli maliyet, kaynak', budget: 13, files: [cost], tool: { kind: 'costDistance', ...net, max: 0, result: 'allocation', sample: 'f32' }, shapes: sources },
      {
        name: 'Birikimli maliyet, yüzey ve eğim',
        budget: 15,
        files: [cost, surface],
        tool: { kind: 'costDistance', ...net, surfaceLength: true, slope: 30, max: 0, result: 'cost', sample: 'f32' },
        shapes: sources,
      },
      { name: 'En düşük maliyetli yol, 5 varış', budget: 12, files: [cost], tool: { kind: 'costPath', ...net, simplify: 1, first: 1 }, shapes: path },
      { name: 'Maliyet koridoru', budget: 18, files: [cost], tool: { kind: 'costCorridor', ...net, first: 1, threshold: 'percent', value: 5, sample: 'f32' }, shapes: ends },
    ];
    for (const job of distanceJobs) {
      const times = [];
      for (let r = 0; r < runs; r++) {
        const ms = await b.eval(`(async () => {
          const { analyzeOps } = await import('/src/io/rasterAnalysis.ts');
          const files = ${JSON.stringify(job.files)};
          const blobs = await Promise.all(files.map(async (f) => (await fetch('/@fs' + f)).blob()));
          const spec = JSON.stringify({ tool: ${JSON.stringify(job.tool)}, inputs: files.map((_, k) => ({ affine: [500000, 5, 0, 4420000, 0, -5], name: 'AB'[k] })), epsg: 5254 });
          const t0 = performance.now();
          const out = await analyzeOps(blobs, spec, ${JSON.stringify(JSON.stringify(job.shapes ?? []))}, { progress() {}, canceled: false });
          const ms = performance.now() - t0;
          if (!out.bytes && !out.features) throw new Error('no result');
          return ms;
        })()`);
        times.push(ms / 1000);
      }
      times.sort((a, c) => a - c);
      console.log(`${job.name.padEnd(34)} 4096²  p50 ${times[Math.floor(times.length / 2)].toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s  bütçe ${job.budget} s`);
    }
    // From 2000 points and 100 lines onto the 4096² grid: the point job.
    const h = (i, j) => {
      let x = (Math.imul(i, 0x9e3779b9) ^ Math.imul(j, 0x85ebca6b)) >>> 0;
      x = (x ^ (x >>> 15)) >>> 0;
      x = Math.imul(x, 0x2c1b3c6d) >>> 0;
      return (x ^ (x >>> 12)) >>> 0;
    };
    const objects = [
      ...Array.from({ length: 2000 }, (_, k) => at(500000 + (h(k, 1) % 20480) + 0.37, 4420000 - (h(k, 2) % 20480) - 0.41)),
      ...Array.from({ length: 100 }, (_, k) => ({
        kind: 'line',
        a: { x: 500000 + (h(k, 3) % 20480) + 0.13, y: 4420000 - (h(k, 4) % 20480) - 0.29 },
        b: { x: 500000 + (h(k, 5) % 20480) + 0.71, y: 4420000 - (h(k, 6) % 20480) - 0.53 },
      })),
    ];
    const pointSpec = JSON.stringify({ tool: { kind: 'distance', max: 0, result: 'distance', margin: 0 }, grid: { affine: [500000, 5, 0, 4420000, 0, -5], width: 4096, height: 4096 } });
    const times = [];
    for (let r = 0; r < runs; r++) {
      const ms = await b.eval(`(async () => {
        const { analyzePoints } = await import('/src/io/rasterAnalysis.ts');
        const t0 = performance.now();
        const out = await analyzePoints(${JSON.stringify(JSON.stringify(objects))}, 'null', ${JSON.stringify(pointSpec)}, true, { progress() {}, canceled: false });
        const ms = performance.now() - t0;
        if (!out.bytes) throw new Error('no result');
        return ms;
      })()`);
      times.push(ms / 1000);
    }
    times.sort((a, c) => a - c);
    console.log(`${'Uzaklık yüzeyi, 2000 nokta, 100 çizgi'.padEnd(34)} 4096²  p50 ${times[Math.floor(times.length / 2)].toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s  bütçe 4 s`);
  }
  // Uygunluk analizi (docs/adr/0237 §11): suitability_timing.rs's jobs, the whole worker run each.
  if (part('suitability')) {
    const [slope, dist, cls, mem] = suitabilityRasters(4096);
    const names = ['Eğim', 'Uzaklık', 'Örtü', 'Üyelik'];
    const square = [{ kind: 'polygon', pts: [[505000.1, 4413500.2], [505499.9, 4413500.2], [505499.9, 4413999.9], [505000.1, 4413999.9]].map(([x, y]) => ({ x, y })) }];
    const suitJobs = [
      { name: 'Bulanık üyelik, Gauss', budget: 3, files: [slope], names: ['Eğim'], tool: { kind: 'fuzzyMembership', band: 1, function: 'gaussian', midpoint: 10, spread: 0.01, sample: 'f32' } },
      { name: 'Bulanık çakıştırma, dört raster, Gamma', budget: 6, files: [mem, mem, mem, mem], names: ['A', 'B', 'C', 'D'], tool: { kind: 'fuzzyOverlay', band: 1, op: 'gamma', gamma: 0.9, sample: 'f32' } },
      {
        name: 'Ağırlıklı toplam, dört raster',
        budget: 6,
        files: [slope, dist, cls, mem],
        names,
        tool: { kind: 'weightedSum', band: 1, weights: { Eğim: -0.4, Uzaklık: 0.001, Örtü: 0.5, Üyelik: 2 }, sample: 'f32' },
      },
      {
        name: 'Ağırlıklı çakıştırma, sınıf tablolarıyla',
        budget: 6,
        files: [slope, dist, cls, mem],
        names,
        tool: {
          kind: 'weightedOverlay',
          band: 1,
          low: 1,
          high: 9,
          influence: { Eğim: 40, Uzaklık: 25, Örtü: 20, Üyelik: 15 },
          classes: { Eğim: '* 5 9; 5 15 6; 15 30 3; 30 * kısıt', Uzaklık: '* 500 9; 500 1500 5; 1500 * 1', Örtü: '1 9; 2 7; 3 5; 4 3; 5 kısıt', Üyelik: '* 0,25 1; 0,25 0,5 4; 0,5 0,75 7; 0,75 * 9' },
          bounds: 'upperClosed',
        },
      },
      { name: 'ROC, bütün hücreler, 10 000 varlık', budget: 6, files: [slope], names: ['Eğim'], tool: { kind: 'roc', band: 1, first: 1, absence: false, higher: true }, shapes: square },
    ];
    for (const job of suitJobs) {
      const times = [];
      for (let r = 0; r < runs; r++) {
        const ms = await b.eval(`(async () => {
          const { analyzeOps } = await import('/src/io/rasterAnalysis.ts');
          const files = ${JSON.stringify(job.files)};
          const names = ${JSON.stringify(job.names)};
          const blobs = await Promise.all(files.map(async (f) => (await fetch('/@fs' + f)).blob()));
          const spec = JSON.stringify({ tool: ${JSON.stringify(job.tool)}, inputs: files.map((_, k) => ({ affine: [500000, 5, 0, 4420000, 0, -5], name: names[k] })), epsg: 5254 });
          const t0 = performance.now();
          const out = await analyzeOps(blobs, spec, ${JSON.stringify(JSON.stringify(job.shapes ?? []))}, { progress() {}, canceled: false });
          const ms = performance.now() - t0;
          if (!out.bytes && !out.roc) throw new Error('no result');
          return ms;
        })()`);
        times.push(ms / 1000);
      }
      times.sort((a, c) => a - c);
      console.log(`${job.name.padEnd(40)} 4096²  p50 ${times[Math.floor(times.length / 2)].toFixed(3)} s  en yavaş ${times[times.length - 1].toFixed(3)} s  bütçe ${job.budget} s`);
    }
  }
} finally {
  b.close();
  await server.close();
}
