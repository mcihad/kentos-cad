import { STOPPED, type AnalysisReply, type AnalysisRequest, type AnalysisResult, type AnalysisWatch } from './rasterAnalysisProtocol';

/**
 * A raster analysis on the web (docs/adr/0231 §2, §11): each job in a worker of its own
 * (io/rasterAnalysisWorker.ts), so that the raster workers drawing the tiles are never kept waiting and the job's
 * memory goes with it. Durdur terminates the worker at once.
 */

/** Runs `spec` (`kentos_raster::job::Spec` JSON) over the raster's bytes `blob` in a worker of its own. */
export function analyzeRaster(blob: Blob, spec: string, watch: AnalysisWatch): Promise<AnalysisResult> {
  const worker = new Worker(new URL('./rasterAnalysisWorker.ts', import.meta.url), { type: 'module', name: 'KentOS raster çözümleme' });
  return new Promise<AnalysisResult>((resolve, reject) => {
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
      else if (r.raster) resolve({ raster: r.raster });
      else if (r.lines) resolve({ lines: r.lines });
      else reject(new Error('Çözümleme bir sonuç vermedi.'));
    };
    worker.onerror = (e) => {
      end();
      reject(new Error(e.message || 'Raster çözümleme işçisi başlatılamadı.'));
    };
    worker.postMessage({ type: 'run', blob, spec } satisfies AnalysisRequest);
  });
}
