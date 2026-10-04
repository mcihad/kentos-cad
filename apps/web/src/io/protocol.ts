import type { CoordReadOptions } from '../contracts/generated/CoordReadOptions';
import type { CoordWriteInput } from '../contracts/generated/CoordWriteInput';
import type { DxfReadOptions } from '../contracts/generated/DxfReadOptions';
import type { DxfWriteInput } from '../contracts/generated/DxfWriteInput';
import type { FieldCsvOptions } from '../contracts/generated/FieldCsvOptions';
import type { GeoJsonReadOptions } from '../contracts/generated/GeoJsonReadOptions';
import type { GeoJsonWriteInput } from '../contracts/generated/GeoJsonWriteInput';
import type { NczReadOptions } from '../contracts/generated/NczReadOptions';
import type { ShapefileReadOptions } from '../contracts/generated/ShapefileReadOptions';
import type { DrawingColumns, PackedDrawing } from './columns';
import type { KcadProgress } from './kcad';

/**
 * Messages between the page and the formats worker (formatsWorker.ts).
 * Files cross as ArrayBuffers, transferred (the page keeps its own copy
 * when it still needs one); results come back as UTF-8 JSON bytes, also
 * transferred, and the page parses them (CLAUDE.md §6.2 rule 6). A drawing
 * to save or one read crosses as typed columns (io/columns.ts, docs/adr/0030)
 * whose buffers are transferred: the side that sends them no longer has
 * them, and so does an imported drawing (DXF, Netcad NCZ; docs/adr/0138),
 * each read by a module of its own that the worker loads the first time such
 * a file is imported. A long KCAD read or write, and a drawing's import,
 * send progress messages before their reply.
 */

export type FormatsRequest =
  | { id: number; op: 'readCoords'; bytes: ArrayBuffer; options: CoordReadOptions }
  /** A field book (docs/adr/0169 §1, §6): an instrument's file (Leica GSI, Sokkia SDR, Topcon GTS-7, Nikon RAW, Trimble JobXML) by its content, else a text book with the user's mapping (null: its first line only). */
  | { id: number; op: 'readFieldBook'; bytes: ArrayBuffer; options: FieldCsvOptions | null }
  /** A GNSS file (docs/adr/0169 §1, §6): GPX 1.1 when it begins with “<”, else an NMEA 0183 log; WGS 84 positions as written. */
  | { id: number; op: 'readGnss'; bytes: ArrayBuffer }
  | { id: number; op: 'writeCoords'; input: CoordWriteInput }
  /** A DXF drawing to import (the DXF module, crates/wasm/dxf-wasm). */
  | { id: number; op: 'readDxf'; bytes: ArrayBuffer; options: DxfReadOptions }
  | { id: number; op: 'writeDxf'; input: DxfWriteInput }
  /** A Netcad NCZ drawing to import (the NCZ module, crates/wasm/ncz-wasm; docs/adr/0138). */
  | { id: number; op: 'readNcz'; bytes: ArrayBuffer; options: NczReadOptions }
  /** GeoJSON and Shapefile (docs/adr/0046); a Shapefile layer's files go together, the .shp required. */
  | { id: number; op: 'readGeoJson'; bytes: ArrayBuffer; options: GeoJsonReadOptions }
  | { id: number; op: 'writeGeoJson'; input: GeoJsonWriteInput }
  | { id: number; op: 'readShapefile'; files: ShapefileBuffers; options: ShapefileReadOptions }
  /** The persistent ids of a v1 drawing's objects (`V1Identities`, docs/adr/0014), from the drawing's text. */
  | { id: number; op: 'v1Identities'; text: string }
  /** A packed drawing to write as `.kcad` v2 (docs/specs/kcad-v2.md): its bytes come back verified. */
  | { id: number; op: 'encodeKcad'; drawing: PackedDrawing }
  /** A `.kcad` v2 file's bytes to read. */
  | { id: number; op: 'decodeKcad'; bytes: ArrayBuffer };

/** A Shapefile layer's files by extension, as transferred buffers. */
export interface ShapefileBuffers {
  shp: ArrayBuffer;
  shx?: ArrayBuffer;
  dbf?: ArrayBuffer;
  prj?: ArrayBuffer;
  cpg?: ArrayBuffer;
}

export type FormatsReply =
  /** A reader's result: JSON. */
  | { id: number; ok: true; json: ArrayBuffer }
  /** A writer's file and its report (ExportReport JSON). */
  | { id: number; ok: true; file: ArrayBuffer; report: string }
  /** A drawing's `.kcad` v2 bytes, read back and compared. */
  | { id: number; ok: true; kcad: ArrayBuffer }
  /** A `.kcad` v2 file's drawing, packed. */
  | { id: number; ok: true; drawing: PackedDrawing }
  /** An imported drawing: the `ImportResult` without its objects as JSON (`head`), the objects as columns. */
  | { id: number; ok: true; imported: { head: string; columns: DrawingColumns } }
  /** How far a KCAD read or write, or a drawing's import (`reading`), is; the reply follows. */
  | { id: number; progress: KcadProgress }
  /** `fatal`: the module trapped or did not load; the page starts a fresh worker. `code`: a KCAD error's (spec §9). */
  | { id: number; ok: false; message: string; fatal: boolean; code?: string };
