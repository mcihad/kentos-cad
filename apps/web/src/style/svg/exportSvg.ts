import { svgOp } from './core';
import type { ReferenceSpec } from './importSvg';
import { crc32 as crcOfBytes, withPngDpi as dpiOfBytes } from './pkg/kentos_svg_wasm.js';
import type { SvgDoc } from './svgModel';

/**
 * The drawing written out (docs/STYLE.md §7): as a symbol SVG (the
 * symbol's colours stay currentColor and param(stroke), as the library
 * keeps it), as a plain SVG for other programs (colours resolved to the
 * preview colours, alpha as fill/stroke-opacity, the size in mm), or as
 * the editor's source view (one element per line with ids, names and
 * hidden shapes, and where each shape's element sits). Selection only
 * crops the canvas to the chosen shapes. Also the pixel size of a PNG
 * export and its DPI chunk. Written by the SVG core (crates/shared/svg-core
 * `export.rs`); the PNG crosses as bytes.
 */

export interface SvgTextOptions {
  /** Resolve the symbol's colours to these (a plain SVG); absent keeps them as parameters. */
  colors?: { ink: string; second: string };
  /** Only these shapes, on a canvas cropped to them. */
  only?: ReadonlySet<string>;
  /** One element per line, indented. */
  pretty?: boolean;
  /** A tracing reference kept in the file (never drawn: it sits in `<defs>`). */
  reference?: ReferenceSpec | null;
}

type Box = { x: number; y: number; w: number; h: number };
const exportBoxCore = svgOp<(doc: SvgDoc, only?: string[]) => Box>('exportBox');

/** The canvas an export covers: the whole drawing, or the chosen shapes with their strokes. */
export const exportBox = (doc: SvgDoc, only?: ReadonlySet<string>): Box => exportBoxCore(doc, only && [...only]);

interface Written {
  text: string;
  /** Character range of each shape's element. */
  spans: Map<string, [number, number]>;
}

const svgTextCore = svgOp<(doc: SvgDoc, opts: Omit<SvgTextOptions, 'only'> & { only?: string[] }) => string>('svgText');

/** The drawing as an SVG file: a symbol SVG, or a plain one with `colors`. */
export function svgText(doc: SvgDoc, opts: SvgTextOptions = {}): string {
  return svgTextCore(doc, { ...opts, only: opts.only && [...opts.only] });
}

const sourceCore = svgOp<(doc: SvgDoc) => { text: string; spans: [string, number, number][] }>('sourceText');

/** The editor's source view: one element per line with ids, names and hidden shapes, and each shape's range. */
export function sourceText(doc: SvgDoc): Written {
  const r = sourceCore(doc);
  return { text: r.text, spans: new Map(r.spans.map(([id, from, to]) => [id, [from, to]])) };
}

// ── PNG ────────────────────────────────────────────────────────────────

/**
 * Pixel size of a PNG export of a `w × h` canvas: a width in pixels, or a
 * DPI with the drawing's width in mm (without one, a unit is a CSS pixel,
 * 96 dpi).
 */
export const pngSize = svgOp<(w: number, h: number, spec: { px: number } | { dpi: number; widthMm?: number }) => { width: number; height: number }>('pngSize');

export const crc32 = (bytes: Uint8Array): number => crcOfBytes(bytes);

/** A PNG with its resolution recorded (a pHYs chunk after IHDR, replacing one that was there), so print programs size it right. */
export const withPngDpi = (png: Uint8Array, dpi: number): Uint8Array => dpiOfBytes(png, dpi) ?? png;
