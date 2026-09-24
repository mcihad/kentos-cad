import type { Entity } from '../model/entities';
import type { Bounds, Vec2 } from '../model/geometry';
import type { LayerStyle } from '../model/layers';
import type { LibraryAsset, Symbol, SymbolSet } from '../model/style';
import { compileSymbol, ExprRun, type CompileEnv, type ExprCache } from '../style/compile';
import { hatchSymbolOf, symbolsOfLayerStyle } from '../style/fromLayer';
import { DrawnReader, type GeometryClass } from '../style/geometry';
import { resolveRenderer, symbolOf, type ResolvedSet } from '../style/resolve';
import type { CanvasPalette } from './color';
import { StyledSink } from './styledSink';
import type { SceneLayer } from './types';

/**
 * A document layer through the style engine (docs/STYLE.md §6): each
 * object gets its own symbol, else its layer's renderer, else the layer's
 * simple look; the symbols are compiled and packed into GPU batches.
 * A symbol missing from the library (or no symbol for the object's kind of
 * geometry) falls back to the simple look, so an object never silently
 * disappears; an area without a fill symbol draws its edges with the line
 * symbol. Dimensions keep their hairlines. What each object draws (curves
 * tessellated, rings oriented) and the geometry values expressions ask for
 * come from the geometry store, the whole layer in one call each.
 */

export interface StyleSources {
  symbol(id: string): Symbol | undefined;
  asset(id: string): LibraryAsset | undefined;
}

/** The geometry store's answers for a layer build (viewport/picking.ts; docs/adr/0008, S2). */
export interface GeometrySource {
  /** What these objects draw, one record each (style/geometry.ts `DrawnReader`); `oriented`: rings turned for the style engine; `clip`: the box construction lines are clipped to. */
  drawn(ids: readonly number[], oriented: boolean, clip?: Bounds): Float64Array;
  /** Their geometry values for expressions (model/expression/expressionLib.ts `measuredAt`). */
  measures(ids: readonly number[]): Float64Array;
}

export interface StyledBuildOptions {
  origin: Vec2;
  palette: CanvasPalette;
  plotScale: number;
  library: StyleSources;
  exprs: ExprCache;
  layerName(id: string): string;
  geometry: GeometrySource;
  /** Box construction lines are clipped to (see ViewportController). */
  clip?: Bounds;
}

/** Symbol levels across classes: every fill before any line, every line before any marker. */
const LEVEL_BASE: Record<GeometryClass, number> = { fill: 0, line: 1000, marker: 2000 };
/** An object's symbol may be of another class than its geometry (compileSymbol adapts it). */
const SYMBOL_CLASS = { fill: 'fill', line: 'line', marker: 'marker' } as const;

export function buildStyledLayer(id: string, entities: readonly Entity[], style: LayerStyle, opts: StyledBuildOptions): SceneLayer {
  const sink = new StyledSink({ origin: opts.origin, palette: opts.palette, plotScale: opts.plotScale, asset: (a) => opts.library.asset(a) });
  const ids = entities.map((e) => e.id);
  // Each expression is evaluated for the whole layer the first time an object needs it; its
  // geometry values come from the store, for the whole layer, the first time one is read.
  const run = new ExprRun(opts.exprs, { entities, layerName: opts.layerName, plotScale: opts.plotScale, measures: () => opts.geometry.measures(ids) });
  const env: CompileEnv = {
    plotScale: opts.plotScale,
    exprs: opts.exprs,
    run,
    assetAspect: (a) => {
      const asset = opts.library.asset(a);
      return asset ? asset.height / asset.width : 1;
    },
  };
  const simple = new Map<string, SymbolSet>();
  const simpleFor = (color: string) => {
    let s = simple.get(color);
    if (!s) simple.set(color, (s = symbolsOfLayerStyle(style, color)));
    return s;
  };
  const lookup = (ref: string) => opts.library.symbol(ref);
  const drawn = new DrawnReader(opts.geometry.drawn(ids, true, opts.clip));

  entities.forEach((e, i) => {
    // Every object has a record, also those drawn elsewhere (text).
    const geom = drawn.read(e);
    if (e.kind === 'text') return;
    const color = e.color ?? style.color;
    if (e.kind === 'dimension') {
      // Dimensions keep their own hairline look (the dimension style is a later step): their layout lines.
      if (geom?.cls !== 'line') return;
      const hair = { color, opacity: 1, width: 0, unit: 'px' as const, dash: null, dashOffset: 0, cap: 'butt' as const, join: 'miter' as const, blur: 0, level: LEVEL_BASE.line + 500 };
      for (const p of geom.paths) sink.stroke(hair, p.pts, false);
      return;
    }
    if (!geom) return;
    const target = { entity: e, index: i + 1 };
    let sets: ResolvedSet[];
    if (e.kind === 'hatch') sets = [{ symbols: { fill: hatchSymbolOf(e, color) } }];
    else if (e.symbol) sets = [{ symbols: { [geom.cls]: { ref: e.symbol } } }];
    else if (style.renderer) sets = resolveRenderer(style.renderer, e, i + 1, env);
    else sets = [{ symbols: simpleFor(color) }];
    // No matching rule or category: the renderer leaves the object out (as in QGIS).
    let drew = false;
    for (const r of sets) {
      sink.setScale(r);
      const own = symbolOf(r.symbols[geom.cls], lookup);
      if (own) {
        compileSymbol(own, geom, target, env, sink, LEVEL_BASE[SYMBOL_CLASS[own.type]]);
        drew = true;
      } else if (geom.cls === 'fill') {
        // An area without a fill symbol takes the line symbol on its edges.
        const edge = symbolOf(r.symbols.line, lookup);
        if (!edge) continue;
        compileSymbol(edge, geom, target, env, sink, LEVEL_BASE.line);
        drew = true;
      }
    }
    // Matched, but nothing for this kind of geometry (or the symbol is gone): the simple look, never nothing.
    if (sets.length && !drew) {
      sink.setScale(sets[0]);
      const fallback = symbolOf(simpleFor(color)[geom.cls], lookup);
      if (fallback) compileSymbol(fallback, geom, target, env, sink, LEVEL_BASE[geom.cls]);
    }
  });
  return { id, lines: [], fills: [], points: [], styled: sink.finish() };
}
