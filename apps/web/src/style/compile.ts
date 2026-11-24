import { ENTITY_KIND_LABEL, type Entity } from '../model/entities';
import { vertexCount } from '../model/expression/expression';
import type { Symbol } from '../model/style';
import { op } from '../wasm/core';
import type { Primitives } from './primitives';

/**
 * Symbol × geometry × object → drawing primitives, by the style core
 * (crates/shared/style-core/src/style/compile.rs, docs/adr/0008 “Stil
 * derleyicisi”): units converted (mm on paper → metres at the plot scale),
 * data-defined values evaluated, parallel offsets and marker places along
 * lines computed in float64. A layer is compiled whole next to the geometry
 * store (render/styledLayer.ts); this is one symbol on one object, for the
 * previews, the legend and tests.
 */

export interface CompileOptions {
  /** Denominator of the plot scale (1000 for 1:1000; $ölçek). */
  readonly plotScale?: number;
  /** The object's layer name ($katman). */
  readonly layerName?: string;
  /** Width and height of image assets (tiles keep their proportions; a square when missing). */
  readonly assets?: Readonly<Record<string, readonly [number, number]>>;
}

const styleCompile = op<(input: unknown) => Primitives>('styleCompile');

/**
 * Draws a symbol on an object's geometry. A symbol of another class adapts,
 * so any symbol can be given to any object: a line symbol on an area draws
 * its edges, a marker symbol sits at an area's inside point or a line's
 * middle, a fill symbol fills a closed line; a fill symbol on an open line
 * or a point, and a line symbol on a point, draw nothing.
 */
export function compileSymbol(symbol: Symbol, entity: Entity, opts: CompileOptions = {}): Primitives {
  return styleCompile({
    symbol,
    entity,
    layerName: opts.layerName ?? '',
    kindLabel: ENTITY_KIND_LABEL[entity.kind],
    vertices: vertexCount(entity),
    plotScale: opts.plotScale ?? 1000,
    assets: opts.assets ?? {},
  });
}
