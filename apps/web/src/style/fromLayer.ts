import type { Entity } from '../model/entities';
import type { LayerStyle, LineType } from '../model/layers';
import type { FillSymbol, LineSymbol, MarkerSymbol, ShapeName, SymbolSet } from '../model/style';

/**
 * The simple look of a layer (colour, line type, weight, fill, point
 * symbol) as symbols, so layers without a renderer go through the same
 * drawing path. Line types are paper millimetres like CAD linetypes; the
 * point symbols stay in screen pixels as before.
 */

/** Dash patterns of the layer line types, paper mm. */
export const LINE_TYPE_DASH: Record<LineType, readonly number[] | null> = {
  continuous: null,
  dashed: [3, 1.5],
  dashdot: [5, 1.2, 0.6, 1.2],
  dotted: [0.6, 1.2],
};

const POINT_SHAPE: Record<string, ShapeName> = { ring: 'ring', cross: 'cross', triangle: 'triangle' };

export function lineSymbolOf(color: string, lineType: LineType, weight: number): LineSymbol {
  return { type: 'line', layers: [{ id: 'l', type: 'simpleLine', color, width: weight, dash: LINE_TYPE_DASH[lineType], unit: 'mm', cap: 'butt', join: 'miter' }] };
}

/**
 * Symbols for a layer's simple style; `color` is the object's own colour when it has one. `hairlines`: line
 * weights hidden (Kalınlık off), every line one pixel.
 */
export function symbolsOfLayerStyle(style: LayerStyle, color = style.color, hairlines = false): SymbolSet {
  const line = lineSymbolOf(color, style.lineType, hairlines ? 0 : style.lineWeight);
  const fill: FillSymbol = {
    type: 'fill',
    layers: [...(style.fill ? [{ id: 'f', type: 'simpleFill' as const, color: style.fill }] : []), { ...line.layers[0], id: 'o' }],
  };
  const p = style.point ?? { symbol: 'ring', size: 7 };
  const marker: MarkerSymbol = {
    type: 'marker',
    layers: [{ id: 'p', type: 'shape', shape: POINT_SHAPE[p.symbol] ?? 'ring', size: p.size, unit: 'px', stroke: color, strokeWidth: 1.3, fill: p.symbol === 'triangle' || p.symbol === 'ring' ? null : null }],
  };
  return { line, fill, marker };
}

/** A hatch object carries its own pattern: solid, lines or crossed lines (spacing in metres). */
export function hatchSymbolOf(e: Extract<Entity, { kind: 'hatch' }>, color: string): FillSymbol {
  const { type, angle, spacing } = e.pattern;
  if (type === 'solid') return { type: 'fill', layers: [{ id: 's', type: 'simpleFill', color, opacity: 0.45 }] };
  const lines = (id: string, a: number) => ({ id, type: 'hatchFill' as const, angle: a, spacing, width: 0, color, unit: 'm' as const });
  return { type: 'fill', layers: type === 'cross' ? [lines('h1', angle), lines('h2', angle + 90)] : [lines('h1', angle)] };
}
