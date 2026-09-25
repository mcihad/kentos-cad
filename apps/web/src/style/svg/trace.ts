import { readResult, writeArgs } from '../../wasm/core';
import { svgOp } from './core';
import type { SubPath } from './pathData';
import { inkMask as maskOfBytes, inkMaskNumbers, traceBitmap as traceBytes, traceBitmapNumbers, traceContours as contours } from './pkg/kentos_svg_wasm.js';

/**
 * Trace bitmap (Inkscape's, simplified): a raster picture into filled
 * paths with holes, for redrawing the regulation's raster pictograms.
 * Brightness threshold → ink mask; marching squares give the outlines on
 * the pixel lattice with ink always on the same side, so outer rings and
 * holes differ by the sign of their area; rings are nested into outers
 * with their holes and specks below an area go; Douglas–Peucker picks the
 * corners (chamfered pixel corners are recovered first) and the runs
 * between them are fitted with cubics. Computed by the SVG core
 * (crates/shared/svg-core `trace.rs`); the pixels cross as bytes.
 * Coordinates are pixels from the image's top-left corner.
 */

/** RGBA pixels, row by row (ImageData or the like). */
export interface Bitmap {
  width: number;
  height: number;
  data: ArrayLike<number>;
}

export interface TraceOptions {
  /** 0–255: pixels darker than this are ink. */
  threshold: number;
  /** Light pixels are ink instead. */
  invert: boolean;
  /** Specks and pinholes smaller than this (px²) are dropped. */
  speckle: number;
  /** Douglas–Peucker tolerance (px). */
  tolerance: number;
  /** Turns sharper than this (degrees) stay corners. */
  corner: number;
  /** 0: straight edges only; above 0: fitted curves, looser (fewer nodes, rounder) as it grows to 1. */
  smooth: number;
}

export const TRACE_DEFAULTS: TraceOptions = { threshold: 128, invert: false, speckle: 6, tolerance: 1, corner: 60, smooth: 1 };

export type Ring = [number, number][];

export interface TracedShape {
  outer: SubPath;
  holes: SubPath[];
}

export interface TraceResult {
  shapes: TracedShape[];
  nodes: number;
  holes: number;
  /** Specks and pinholes dropped. */
  removed: number;
}

/** Pixel bytes as the core reads them (a byte array without a copy), or null for other numbers. */
const bytesOf = (d: ArrayLike<number>): Uint8Array | null => (d instanceof Uint8Array || d instanceof Uint8ClampedArray ? new Uint8Array(d.buffer, d.byteOffset, d.length) : null);

/** Ink (1) where the pixel, laid on white paper, is darker than the threshold. */
export function inkMask(img: Bitmap, threshold: number, invert = false): Uint8Array {
  const bytes = bytesOf(img.data);
  return bytes ? maskOfBytes(img.width, img.height, bytes, threshold, invert) : inkMaskNumbers(img.width, img.height, Float64Array.from(img.data), threshold, invert);
}

/**
 * Outlines of the ink by marching squares on pixel centres: every ring
 * closes, ink lies on the same side of every segment (outer rings have a
 * positive shoelace area in image coordinates, holes a negative one), and
 * diagonal pixels of ink connect.
 */
export function traceContours(mask: ArrayLike<number>, w: number, h: number): Ring[] {
  return readResult(contours(Uint8Array.from(mask), w, h)) as Ring[];
}

/** Signed shoelace area (image coordinates, y down): outer rings positive. */
export const ringArea = svgOp<(r: readonly (readonly [number, number])[]) => number>('ringArea');

/** Outer rings with the holes they hold (the smallest outer around each hole); rings under `minArea` go. */
export const nestRings = svgOp<(rings: Ring[], minArea?: number) => { shapes: { outer: Ring; holes: Ring[] }[]; removed: number }>('nestRings');

/** Douglas–Peucker on a closed ring, anchored at two extreme points (real corners, not the arbitrary start). */
export const simplifyRing = svgOp<(r: Ring, tol: number) => Ring>('simplifyRing');

/** The whole trace: mask, outlines, nesting, simplification and curves. */
export function traceBitmap(img: Bitmap, opts: Partial<TraceOptions> = {}): TraceResult {
  const o = writeArgs({ ...TRACE_DEFAULTS, ...opts });
  const bytes = bytesOf(img.data);
  return readResult(bytes ? traceBytes(img.width, img.height, bytes, o) : traceBitmapNumbers(img.width, img.height, Float64Array.from(img.data), o)) as TraceResult;
}
