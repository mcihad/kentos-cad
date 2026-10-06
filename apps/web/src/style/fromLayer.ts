import type { Entity, HatchPattern } from '../model/entities';
import { hatchPaints, type FamilyPaint } from '../model/ops/hatchPatterns';
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
 * Symbols for a layer's simple style; `color` is the object's own colour when it has one, `weight` its own
 * line weight (mm; docs/adr/0139). `hairlines`: line weights hidden (Kalınlık off), every line one pixel.
 */
export function symbolsOfLayerStyle(style: LayerStyle, color = style.color, hairlines = false, weight = style.lineWeight): SymbolSet {
  const line = lineSymbolOf(color, style.lineType, hairlines ? 0 : weight);
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

/**
 * A leader's look (docs/adr/0146 §5): its lines as its layer's simple line in its own colour and weight, its filled
 * arrowhead or dot solid in that colour. The desktop's is `leader_symbols_of` (crates/native/style/src/simple.rs).
 */
export function leaderSymbolsOf(style: LayerStyle, color: string, hairlines = false, weight = style.lineWeight): SymbolSet {
  return {
    line: lineSymbolOf(color, style.lineType, hairlines ? 0 : weight),
    fill: { type: 'fill', layers: [{ id: 's', type: 'simpleFill', color }] },
  };
}

/** The families of the patterns met so far, by the pattern's JSON (the core gives them once a pattern). */
const families = new Map<string, readonly FamilyPaint[]>();

/** A pattern's families, as the geometry core gives them (docs/adr/0186 §3). */
function familiesOf(pattern: HatchPattern): readonly FamilyPaint[] {
  const key = JSON.stringify(pattern);
  let out = families.get(key);
  if (!out) {
    // A drawing has a few patterns; a bound keeps a long session's edits from piling up.
    if (families.size > 256) families.clear();
    families.set(key, (out = hatchPaints(pattern)));
  }
  return out;
}

/**
 * A hatch object carries its own pattern: solid at 45 %, a gradient (docs/adr/0186 §3), or one hatch fill a family as
 * the geometry core gives the families (spacing in metres; a user-defined pattern's lines as they always were). The
 * desktop's is `hatch_symbol_of` (crates/native/style/src/simple.rs).
 */
export function hatchSymbolOf(e: Extract<Entity, { kind: 'hatch' }>, color: string): FillSymbol {
  const p = e.pattern;
  if (p.type === 'solid') return { type: 'fill', layers: [{ id: 's', type: 'simpleFill', color, opacity: 0.45 }] };
  if (p.type === 'gradient') {
    const g = p.gradient;
    return { type: 'fill', layers: [{ id: 'g', type: 'gradientFill', color, color2: g?.color2 ?? '#FFFFFF', shape: g?.shape ?? 'linear', angle: p.angle, inverted: g?.inverted === true }] };
  }
  const layers = familiesOf(p).map((f, i) => ({
    id: `h${i + 1}`,
    type: 'hatchFill' as const,
    angle: f.angle,
    spacing: f.spacing,
    width: 0,
    color,
    unit: 'm' as const,
    ...(f.offset !== 0 && { offset: f.offset }),
    ...(f.dash && { dash: f.dash, dashOffset: f.dashOffset }),
    ...(f.stagger !== 0 && { stagger: f.stagger }),
  }));
  return { type: 'fill', layers };
}
