import {
  STOPPED,
  type AnalysisReply,
  type AnalysisRequest,
  type AnalysisResult,
  type AnalysisWatch,
  type OpsRequest,
  type OpsResult,
  type PointRequest,
  type PointResult,
} from './rasterAnalysisProtocol';

/**
 * A raster analysis on the web (docs/adr/0231 §2, §11; a raster from points, docs/adr/0232): each job in a worker of its own
 * (io/rasterAnalysisWorker.ts), so that the raster workers drawing the tiles are never kept waiting and the job's
 * memory goes with it. Durdur terminates the worker at once.
 */

/** Runs `request` in a worker of its own: its `done` reply, or why not. */
function inWorker(request: AnalysisRequest | PointRequest | OpsRequest, watch: AnalysisWatch): Promise<Extract<AnalysisReply, { type: 'done' }>> {
  const worker = new Worker(new URL('./rasterAnalysisWorker.ts', import.meta.url), { type: 'module', name: 'KentOS raster çözümleme' });
  return new Promise((resolve, reject) => {
    // Durdur is heard between the worker's messages and a few times a second besides.
    const poll = setInterval(() => {
      if (!watch.canceled) return;
      end();
      reject(new Error(STOPPED));
    }, 100);
    const end = () => {
      clearInterval(poll);
      worker.terminate();
    };
    worker.onmessage = (e: MessageEvent<AnalysisReply>) => {
      const r = e.data;
      if (r.type === 'progress') {
        if (watch.canceled) {
          end();
          reject(new Error(STOPPED));
        } else watch.progress(r.share);
        return;
      }
      end();
      if (r.error !== undefined) reject(new Error(r.error));
      else resolve(r);
    };
    worker.onerror = (e) => {
      end();
      reject(new Error(e.message || 'Raster çözümleme işçisi başlatılamadı.'));
    };
    worker.postMessage(request);
  });
}

/** Runs `spec` (`kentos_raster::job::Spec` JSON) over the raster's bytes `blob` in a worker of its own. */
export async function analyzeRaster(blob: Blob, spec: string, watch: AnalysisWatch): Promise<AnalysisResult> {
  const r = await inWorker({ type: 'run', blob, spec }, watch);
  if (r.raster) return { raster: r.raster };
  if (r.lines) return { lines: r.lines };
  throw new Error('Çözümleme bir sonuç vermedi.');
}

/** Makes a raster from points or lines (docs/adr/0232) in a worker of its own. */
export async function analyzePoints(objects: string, values: string, spec: string, lines: boolean, watch: AnalysisWatch): Promise<PointResult> {
  const r = await inWorker({ type: 'points', objects, values, spec, lines }, watch);
  if (r.points) return r.points;
  throw new Error('Çözümleme bir sonuç vermedi.');
}

/**
 * Runs a raster operation (docs/adr/0233) over the inputs' files `sources` (the run's order; a text says why one cannot
 * be read) in a worker of its own.
 */
export async function analyzeOps(sources: (Blob | string)[], spec: string, shapes: string, watch: AnalysisWatch): Promise<OpsResult> {
  const r = await inWorker({ type: 'ops', sources, spec, shapes }, watch);
  if (r.ops) return r.ops;
  throw new Error('Çözümleme bir sonuç vermedi.');
}
