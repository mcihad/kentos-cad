import { op } from '../../wasm/core';
import type { Vec2 } from '../geometry';

/**
 * What the TypeScript side keeps of the expression language (expression.ts):
 * the value type, the layout of the geometry store's `measures` answer, and
 * the catalog of functions and variables for the expression field's menus.
 * The language itself, its values and its rules, are the Rust style core's
 * (crates/shared/style-core/src/expr, docs/adr/0008 “İfade dili”).
 */

export type ExprValue = number | string | boolean | null;

/** An object's geometry values for expressions: `$uzunluk`, `$alan`, and the anchor behind `$y` and `$x`. */
export interface Measured {
  readonly length: number | null;
  readonly area: number | null;
  /** Null for a path without vertices. */
  readonly anchor: Vec2 | null;
}

/** Numbers per object in a geometry store `measures` answer: flags, length, area, anchor x and y, spare. */
export const MEASURE_STRIDE = 6;

/**
 * The geometry values of object `i` in a geometry store `measures` answer
 * (crates/shared/geometry-core/src/store/draw.rs).
 */
export function measuredAt(values: Float64Array, i: number): Measured {
  const k = i * MEASURE_STRIDE;
  const flags = values[k];
  return {
    length: flags & 1 ? values[k + 1] : null,
    area: flags & 2 ? values[k + 2] : null,
    anchor: flags & 4 ? { x: values[k + 3], y: values[k + 4] } : null,
  };
}

export interface ExprFunctionInfo {
  readonly name: string;
  /** English (QGIS) and spelling variants. */
  readonly aliases: readonly string[];
  readonly signature: string;
  readonly description: string;
}

export interface ExprVariableInfo {
  /** Name without "$", as shown. */
  readonly name: string;
  readonly aliases: readonly string[];
  readonly description: string;
}

export interface ExprCatalog {
  readonly functions: readonly ExprFunctionInfo[];
  readonly variables: readonly ExprVariableInfo[];
}

const catalogOp = op<() => ExprCatalog>('exprCatalog');
let catalog: ExprCatalog | null = null;

/** The functions and variables of the language, from the core (asked once). */
export const exprCatalog = (): ExprCatalog => (catalog ??= catalogOp());
