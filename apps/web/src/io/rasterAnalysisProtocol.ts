/**
 * What the page and the raster analysis worker say to each other (io/rasterAnalysisWorker.ts,
 * io/rasterAnalysis.ts; docs/adr/0231 §2, docs/adr/0232, docs/adr/0233). A job is one worker's: the raster comes as a Blob (a File the user chose,
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

/**
 * A raster made from points or lines (docs/adr/0232): the objects as JSON (their places and elevations; Çizgi
 * yoğunluğu's their geometry), each one's value or weight text as a JSON array (or `null`), the raster core's
 * `PointSpec` JSON.
 */
export interface PointRequest {
  type: 'points';
  objects: string;
  values: string;
  spec: string;
  lines: boolean;
}

/**
 * A point job's result: the GeoTIFF, its grid (the affine's six numbers, width, height), its bands, samples and each
 * band's look (`RasterStyle` JSON), what the run met (JSON: taken, merged, unread, noElevation, empty, outside, radius,
 * variogram, cross), and the cross-validation's rows (each point's place among the points, its object, and five numbers a row:
 * x, y, measured, predicted, standard error; NaN for none).
 */
export interface PointResult {
  bytes: Uint8Array;
  grid: number[];
  bands: number;
  /** The samples as the contract names them: `f32`, Rasterleştir's its own (docs/adr/0234 §3). */
  sample: string;
  styles: [string, string];
  notes: string;
  crossPoint: Uint32Array;
  crossObject: Uint32Array;
  crossValues: Float64Array;
}

/**
 * A raster operation (docs/adr/0233): its inputs' Blobs in the run's order (each a TIFF read by its slices, a PNG, a
 * JPEG), the raster core's `OpsSpec` JSON, and the mask's or the zones' objects as JSON (an array; `[]` for none).
 */
export interface OpsRequest {
  type: 'ops';
  /** Each input's file in the run's order, or why it cannot be read (said only when the run reads it). */
  sources: (Blob | string)[];
  spec: string;
  shapes: string;
}

/**
 * A raster operation's result: a raster (the GeoTIFF's bytes, its grid, bands, samples and look), or a table's figures
 * (each zone's seven numbers: cells with a value, the statistic asked for, sum, mean, least, largest, standard
 * deviation; NaN for none), or the histogram (JSON), or a vectorizing run's features; what the run met (JSON: cells,
 * emptyCells).
 */
export interface OpsResult {
  /** The inputs the run read (`opsReads`): Raster hesaplayıcı's those its expression names, the first the grid's. */
  reads: number[];
  bytes?: Uint8Array;
  grid: number[];
  bands: number;
  sample: string;
  style: string;
  notes: string;
  zones?: Float64Array;
  histogram?: string;
  features?: AnalysisFeatures;
}

/**
 * A vectorizing run's features (docs/adr/0234): `polygons`, `lines` or `points`; each one's value (NaN: none), text and
 * tag (a point: 1 a peak, 2 a pit; Alan kapat's area: 1 written unsimplified), each area's ring count, each ring's,
 * line's or point's vertex count, and every vertex's x, y in order.
 */
export interface AnalysisFeatures {
  kind: 'polygons' | 'lines' | 'points';
  values: Float64Array;
  texts: string[];
  tags: Uint8Array;
  rings: Uint32Array;
  sizes: Uint32Array;
  xy: Float64Array;
}

export type AnalysisReply =
  | { type: 'progress'; share: number }
  | { type: 'done'; raster?: AnalysisRaster; lines?: AnalysisLines; points?: PointResult; ops?: OpsResult; error?: string };

export type AnalysisResult = { raster: AnalysisRaster } | { lines: AnalysisLines };

/** How a job tells its share and asks whether it is to stop. */
export interface AnalysisWatch {
  progress(share: number): void;
  readonly canceled: boolean;
}

/** A stopped job's error: the run ends saying nothing more. */
export const STOPPED = 'durduruldu';
