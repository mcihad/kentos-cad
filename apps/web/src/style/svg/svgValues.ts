import { svgOp } from './core';
import type { Matrix } from './pathData';
import type { Paint } from './svgModel';

/**
 * The values of an SVG file as the importer reads them: colours (every
 * CSS syntax and keyword, with alpha), paints (the symbol's colours,
 * references), transforms, lengths with units, viewBox mapping with
 * preserveAspectRatio, and `<style>` sheets with the selectors icon files
 * use (class, id, tag, attribute, descendant and child). Read by the SVG
 * core (crates/shared/svg-core `values.rs`), where the importer uses them.
 */

export interface XmlNode {
  tag: string;
  attrs: Record<string, string>;
  children: XmlNode[];
  /** Text content: of a text element (without children), of a `#text` node, of a `<style>`. */
  text?: string;
}

// ── Colours ────────────────────────────────────────────────────────────

const readColorCore = svgOp<(v: string) => { hex: string; alpha: number } | null>('readColor');

/** A CSS colour as hex (#RRGGBB) and alpha; undefined when it is not a colour. */
export const readColor = (v: string): { hex: string; alpha: number } | undefined => readColorCore(v) ?? undefined;

/** A colour with its alpha as #RRGGBBAA (opaque colours stay #RRGGBB). */
export const withAlpha = svgOp<(hex: string, alpha: number) => string>('withAlpha');

/** Near-black colours (the "black" of icon sets: #000, #1D1D1B, #231F20 …) that become the symbol colour. */
export const isNearBlack = svgOp<(hex: string) => boolean>('isNearBlack');

const readPaintCore = svgOp<(v: string | undefined) => Paint | null>('readPaint');

/** A paint value as the model's paint (black is the symbol colour); undefined when it cannot be read. */
export const readPaint = (v: string | undefined): Paint | undefined => readPaintCore(v) ?? undefined;

// ── Transforms and lengths ─────────────────────────────────────────────

/** The transform attribute as a matrix (functions applied left to right, as SVG does). */
export const readTransform = svgOp<(v: string | undefined) => Matrix>('readTransform');

export const parseLength = svgOp<(v: string | undefined) => { value: number; unit: string } | null>('parseLength');

const toUserCore = svgOp<(v: string | undefined, percent: number, em: number) => number | null>('toUser');

/** A length in user units; % against `percent`, em against `em`. */
export const toUser = (v: string | undefined, percent: number, em = 16): number | undefined => toUserCore(v, percent, em) ?? undefined;

/** The viewBox → viewport map with preserveAspectRatio (SVG 1.1 §7.8). */
export const viewBoxTransform = svgOp<(vb: readonly number[], width: number, height: number, par?: string) => Matrix>('viewBoxTransform');

export const viewBoxOf = svgOp<(v: string | undefined) => number[] | null>('viewBoxOf');

// ── CSS ────────────────────────────────────────────────────────────────

interface Compound {
  tag?: string;
  id?: string;
  classes: string[];
  attrs: { name: string; value?: string }[];
}
interface Selector {
  /** Right to left: the element itself first, then its ancestors with the combinator between. */
  parts: { c: Compound; child: boolean }[];
  spec: number;
}
export interface Rule {
  selectors: Selector[];
  decls: { prop: string; value: string; important: boolean }[];
  order: number;
}

/** Rules of a style sheet; at-rule blocks (@media, @font-face …) are left out. */
export const parseCss = svgOp<(text: string, orderFrom?: number) => Rule[]>('parseCss');
