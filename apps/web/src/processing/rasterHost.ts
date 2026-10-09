import type { AnalysisResult, AnalysisWatch, OpsResult, PointResult } from '../io/rasterAnalysisProtocol';
import type { RasterEntity } from '../model/entities';

/**
 * Where the raster tools (Yüzey analizi, docs/adr/0231 §2) read a raster, run their job and keep their result on the
 * web: the page's (app/rasterAnalysis.ts sets it when the app starts). A worker, or a test that gives none, has no
 * host, and such a tool refuses saying why, as the desktop's tools do without files.
 */
export interface RasterRunHost {
  /** The raster object's bytes this session; a reason when there are none (a linked file not given again). */
  source(raster: RasterEntity): Blob | { refused: string };
  /** Runs a job (`kentos_raster::job::Spec` JSON) in a worker of its own; rejects with `STOPPED` when stopped. */
  analyze(blob: Blob, spec: string, watch: AnalysisWatch): Promise<AnalysisResult>;
  /**
   * Makes a raster from points or lines (docs/adr/0232): the objects' JSON, their value texts' JSON, the
   * `PointSpec` JSON; in a worker of its own, rejecting with `STOPPED` when stopped.
   */
  analyzePoints(objects: string, values: string, spec: string, lines: boolean, watch: AnalysisWatch): Promise<PointResult>;
  /**
   * Runs a raster operation (docs/adr/0233) over several rasters' files (the run's order; a text says why one cannot
   * be read, refused only when the run reads it), the `OpsSpec` JSON and the mask's or the zones' objects as JSON; in a
   * worker of its own, rejecting with `STOPPED` when stopped.
   */
  analyzeOps(sources: (Blob | string)[], spec: string, shapes: string, watch: AnalysisWatch): Promise<OpsResult>;
  /**
   * Keeps a result GeoTIFF named `name` (`width` × `height`): embedded in the project's library when it is small
   * enough, else the session's file of that name, downloaded. What the raster object names, and a line for the log
   * (none when empty).
   */
  keep(bytes: Uint8Array, name: string, width: number, height: number): Promise<{ asset?: string; file?: string; note: string }>;
}

let host: RasterRunHost | null = null;

/** The page's host (app/rasterAnalysis.ts); null takes it away (tests). */
export function setRasterRunHost(h: RasterRunHost | null): void {
  host = h;
}

export function rasterRunHost(): RasterRunHost | null {
  return host;
}
