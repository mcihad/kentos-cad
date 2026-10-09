// Raster analysis in the browser (docs/adr/0231 §11): how long Yüzey analizi's jobs take in their own worker
// (io/rasterAnalysisWorker.ts, raster-wasm on one thread) over a 4096 × 4096 32-bit DEM, tiled and Deflate'd as the
// desktop's measurement makes it (crates/shared/raster/tests/all/timing.rs, the same hills): Eğim, Eş yükselti eğrileri
// (5 m, about 100 levels), and Güneşlenme over a 2048 × 2048 one (a year, 14 days, half an hour). Starts its own Vite dev
// server and one headless Chrome; the DEMs are written once by GDAL into .run/perf (python3 with numpy and osgeo) and
// fetched by the page as a file the user gave. The worker's whole run is timed: its start, the module, reading the
// file's blocks, the job and the result's coding. Nothing else heavy may run meanwhile (docs/adr/0005).
//
//   node ../../scripts/wasm/ensure.mjs --release && node scripts/perf/raster.mjs [--runs 3]
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync } from 'node:fs';
import { cpus } from 'node:os';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { launch } from '../e2e/cdp.mjs';

const args = process.argv.slice(2);
const runs = Number(args.includes('--runs') ? args[args.indexOf('--runs') + 1] : '3');
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
  for (const job of jobs) {
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
} finally {
  b.close();
  await server.close();
}
