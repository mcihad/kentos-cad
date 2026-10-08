/**
 * What the page and a raster worker say to each other (io/rasterWorker.ts,
 * render/rasterService.ts; docs/adr/0204). A raster is named by its scene
 * key (`asset:<id>`, `file:<name>`) and comes with its bytes as a Blob (a
 * File the user chose, or an embedded raster's bytes): cloning one passes a
 * reference, not the bytes. Pixels come back as transferred buffers.
 */

/** A raster's bytes and names, as every request about it carries them. */
export interface RasterRef {
  key: string;
  blob: Blob;
  /** Its file's name (the pyramid file's key takes it). */
  name: string;
}

export type RasterRequest =
  /** Raster ekle: the chosen files (the image and its world file), read for the window. */
  | { type: 'inspect'; id: number; files: File[]; srid: number; confirmed: boolean; view: number[] }
  /** A tile's colours by `look` (the contract's JSON) for a raster placed by `affine`. */
  | ({ type: 'tile'; id: number; look: string; affine: number[]; level: number; tx: number; ty: number } & RasterRef)
  /** The bands' statistics (Raster stili). */
  | ({ type: 'stats'; id: number } & RasterRef)
  /** The bands' values at pixel (i, j) of level 0 (Koordinat oku). */
  | ({ type: 'values'; id: number; i: number; j: number } & RasterRef)
  /** Raster oturt's resampling: `points` five numbers each (column, row, x, y, used). */
  | ({ type: 'warp'; id: number; points: number[]; method: string; pixel: number; nearest: boolean; epsg: number } & RasterRef)
  | { type: 'stopWarp'; id: number }
  | { type: 'stopPyramid'; key: string }
  /** Every raster but these is let go (another drawing, a raster gone). */
  | { type: 'forget'; keep: string[] }
  /** This raster is let go (its file given again). */
  | { type: 'drop'; key: string };

export type RasterReply =
  /** A request's answer: its value, its bytes, why not, or (a coarse level while the pyramid is made) to ask again later. */
  | { type: 'done'; id: number; value?: unknown; bytes?: Uint8Array; error?: string; wait?: boolean }
  /** A resampling's share. */
  | { type: 'progress'; id: number; share: number }
  /** A pyramid's pass: its share, or null when it ended (made or stopped). */
  | { type: 'pyramid'; key: string; name: string; done: number | null }
  /** Tiles that waited may be asked again (a pyramid made or taken). */
  | { type: 'refresh' };

/** What Raster ekle reads of the chosen files. */
export interface RasterInspected {
  name: string;
  world: string | null;
  info: RasterFileInfo;
  style: import('../model/entities').RasterStyle;
  placement: RasterPlacement;
  /** The placement once an unknown system is said to be the project's. */
  confirmedPlacement: RasterPlacement;
}

/** The formats core's `RasterInfo`. */
export interface RasterFileInfo {
  width: number;
  height: number;
  bands: number;
  sample: import('../model/entities').RasterSample;
  color: string;
  alpha: boolean;
  compression: string;
  tiled: boolean;
  big: boolean;
  overviews: number;
  levels: number;
  affine?: number[];
  placedBy: 'geotiff' | 'world' | 'none';
  epsg?: number;
  geographic: boolean;
  nodata?: number;
  needsPyramid: boolean;
}

/** The core's `place::Rule` and, when the raster goes, where. */
export type RasterPlacement = ({ rule: 'same'; srid: number } | { rule: 'unknown' } | { rule: 'other'; srid: number } | { rule: 'unplaced' }) & {
  affine?: number[];
  srid?: number;
};

/** A band's statistics (Raster stili). */
export interface BandStats {
  min: number | null;
  max: number | null;
  low: number | null;
  high: number | null;
}
