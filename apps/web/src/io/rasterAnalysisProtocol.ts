/**
 * What the page and the raster analysis worker say to each other (io/rasterAnalysisWorker.ts,
 * io/rasterAnalysis.ts; docs/adr/0231 §2). A job is one worker's: the raster comes as a Blob (a File the user chose,
 * or an embedded raster's bytes; cloning one passes a reference), its settings as the raster core's `Spec` JSON. The
 * result comes back as transferred buffers: a GeoTIFF's bytes with its kind and look, or the lines as typed arrays.
 */

export interface AnalysisRequest {
  type: 'run';
  blob: Blob;
  /** `kentos_raster::job::Spec` as JSON. */
  spec: string;
}

/** A raster result: the whole GeoTIFF, its bands, samples (`f32`, `u8`) and look (`RasterStyle` JSON). */
export interface AnalysisRaster {
  bytes: Uint8Array;
  bands: number;
  sample: string;
  style: string;
}

/** Lines, one after another: each one's level, whether an Ana eğri, its point count and Kot text; all points x, y. */
export interface AnalysisLines {
  values: Float64Array;
  main: Uint8Array;
  sizes: Uint32Array;
  points: Float64Array;
  texts: string[];
}

export type AnalysisReply =
  | { type: 'progress'; share: number }
  | { type: 'done'; raster?: AnalysisRaster; lines?: AnalysisLines; error?: string };

export type AnalysisResult = { raster: AnalysisRaster } | { lines: AnalysisLines };

/** How a job tells its share and asks whether it is to stop. */
export interface AnalysisWatch {
  progress(share: number): void;
  readonly canceled: boolean;
}

/** A stopped job's error: the run ends saying nothing more. */
export const STOPPED = 'durduruldu';
