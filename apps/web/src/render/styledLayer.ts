import type { Entity } from '../model/entities';
import { exprTable, type ExprNeeds, type ExprTable } from '../model/expression/expression';
import type { Bounds, Vec2 } from '../model/geometry';
import type { LayerStyle } from '../model/layers';
import type { LayerRenderer, LibraryAsset, Rule, Symbol, SymbolSet } from '../model/style';
import { hatchSymbolOf, symbolsOfLayerStyle } from '../style/fromLayer';
import { CoreStyleProgram } from '../wasm/core';
import type { CanvasPalette } from './color';
import { styledBatches } from './styledBatches';
import type { SceneLayer } from './types';

/**
 * A document layer through the style engine (docs/STYLE.md §6), in one
 * call to the style core next to the geometry store
 * (crates/shared/style-core/src/style/build.rs, docs/adr/0008 “Stil
 * derleyicisi”): each object gets its own symbol, else its layer's
 * renderer, else the layer's simple look; symbols are compiled on the
 * store's geometry and packed into GPU batches there. This side says how
 * each object is drawn, gives the symbols and the objects' values for the
 * expressions, and turns the batches' colours and images into what the
 * GPU draws (render/styledBatches.ts).
 */

export interface StyleSources {
  symbol(id: string): Symbol | undefined;
  asset(id: string): LibraryAsset | undefined;
}

/** The geometry store's answers for layer builds (viewport/picking.ts; docs/adr/0008, S2). */
export interface GeometrySource {
  /** What these objects draw, one record each (style/geometry.ts `DrawnReader`); `oriented`: rings turned for the style engine; `clip`: the box construction lines are clipped to. */
  drawn(ids: readonly number[], oriented: boolean, clip?: Bounds): Float64Array;
  /** A styled layer's batches (`CoreStore.buildStyled`). */
  styled(program: CoreStyleProgram, ids: readonly number[], objects: Int32Array, table: ExprTable, clip: Bounds | null, origin: Vec2, plotScale: number): { json: string; data: Float32Array };
}

export interface StyledBuildOptions {
  origin: Vec2;
  palette: CanvasPalette;
  plotScale: number;
  library: StyleSources;
  layerName(id: string): string;
  geometry: GeometrySource;
  /** Box construction lines are clipped to (see ViewportController). */
  clip?: Bounds;
}

/** How the core draws an object (style/build.rs `MODE_*`). */
const SKIP = 0;
const DIMENSION = 1;
const SET = 2;
const OWN = 3;
const RENDERER = 4;

/** The library symbols a renderer's sets refer to. */
function rendererRefs(r: LayerRenderer, out: Set<string>): void {
  const set = (s: SymbolSet | undefined) => {
    for (const ref of [s?.marker, s?.line, s?.fill]) if (ref && 'ref' in ref) out.add(ref.ref);
  };
  const rules = (list: readonly Rule[]) => {
    for (const r of list) {
      set(r.symbols);
      if (r.children) rules(r.children);
    }
  };
  switch (r.type) {
    case 'single':
      return set(r.symbols);
    case 'categorized':
      r.categories.forEach((c) => set(c.symbols));
      return set(r.other);
    case 'graduated':
      return r.classes.forEach((c) => set(c.symbols));
    case 'rules':
      return rules(r.rules);
  }
}

/** Image assets the symbols tile (their sizes go to the core: tiles keep the images' proportions). */
function assetsOf(value: unknown, out: Set<string>): void {
  if (Array.isArray(value)) for (const v of value) assetsOf(v, out);
  else if (value && typeof value === 'object') {
    const o = value as Record<string, unknown>;
    if (o.type === 'imageFill' && typeof o.asset === 'string') out.add(o.asset);
    for (const v of Object.values(o)) assetsOf(v, out);
  }
}

/** Width and height of the library's image assets among `ids` (what the core reads as `assets`). */
export function assetSizes(ids: Iterable<string>, asset: (id: string) => LibraryAsset | undefined): Record<string, readonly [number, number]> {
  const out: Record<string, readonly [number, number]> = {};
  for (const id of ids) {
    const a = asset(id);
    if (a) out[id] = [a.width, a.height];
  }
  return out;
}

const NEEDS: readonly (keyof ExprNeeds)[] = ['measured', 'vertices', 'kind', 'layer', 'label', 'index', 'id', 'scale'];

export function buildStyledLayer(id: string, entities: readonly Entity[], style: LayerStyle, opts: StyledBuildOptions): SceneLayer {
  // Sets by content (the simple look per colour, hatches' own patterns), colours and own symbols by index.
  const sets: SymbolSet[] = [];
  const setIndex = new Map<string, number>();
  const setOf = (s: SymbolSet) => {
    const key = JSON.stringify(s);
    let k = setIndex.get(key);
    if (k === undefined) {
      setIndex.set(key, (k = sets.length));
      sets.push(s);
    }
    return k;
  };
  const colors: string[] = [];
  const colorIndex = new Map<string, number>();
  const refs: string[] = [];
  const refIndex = new Map<string, number>();
  const simple = new Map<string, number>();
  const objects = new Int32Array(4 * entities.length);
  entities.forEach((e, i) => {
    const color = e.color ?? style.color;
    let c = colorIndex.get(color);
    if (c === undefined) {
      colorIndex.set(color, (c = colors.length));
      colors.push(color);
    }
    let s = simple.get(color);
    if (s === undefined) simple.set(color, (s = setOf(symbolsOfLayerStyle(style, color))));
    let mode = RENDERER;
    let a = 0;
    if (e.kind === 'text') mode = SKIP;
    else if (e.kind === 'dimension') mode = DIMENSION;
    else if (e.kind === 'hatch') {
      mode = SET;
      a = setOf({ fill: hatchSymbolOf(e, color) });
    } else if (e.symbol) {
      mode = OWN;
      let r = refIndex.get(e.symbol);
      if (r === undefined) {
        refIndex.set(e.symbol, (r = refs.length));
        refs.push(e.symbol);
      }
      a = r;
    } else if (!style.renderer) {
      mode = SET;
      a = s;
    }
    objects.set([mode, a, s, c], 4 * i);
  });
  const used = new Set(refs);
  if (style.renderer) rendererRefs(style.renderer, used);
  const symbols: Record<string, Symbol> = {};
  for (const ref of used) {
    const sym = opts.library.symbol(ref);
    if (sym) symbols[ref] = sym;
  }
  const tiled = new Set<string>();
  assetsOf([symbols, sets, style.renderer ?? null], tiled);
  const assets = assetSizes(tiled, (a) => opts.library.asset(a));
  const program = new CoreStyleProgram(JSON.stringify({ symbols, renderer: style.renderer ?? null, sets, refs, colors, assets }));
  try {
    const needs = Object.fromEntries(NEEDS.map((k, i) => [k, !!(program.needs & (1 << i))])) as unknown as ExprNeeds;
    const table = exprTable(program.fields, needs, entities, opts.layerName);
    const out = opts.geometry.styled(program, entities.map((e) => e.id), objects, table, opts.clip ?? null, opts.origin, opts.plotScale);
    return { id, lines: [], fills: [], points: [], styled: styledBatches(out.json, out.data, { palette: opts.palette, plotScale: opts.plotScale, asset: (a) => opts.library.asset(a) }) };
  } finally {
    program.free();
  }
}
