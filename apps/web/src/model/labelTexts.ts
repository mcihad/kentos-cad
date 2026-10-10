import type { LabelStyle } from '../contracts/generated/LabelStyle';
import type { LayerLabels } from '../contracts/generated/LayerLabels';
import type { Entity } from './entities';
import { compileExpression, type CompiledExpression, type ExprGeometry } from './expression/expression';
import { DEFAULT_LABELS } from './labelDefaults';
import type { LayerNode, LayerStore } from './layers';
import { fillTemplate } from './ops/labelText';

/**
 * The label engine's texts (docs/adr/0212 §3.1): each object's text for every class that labels it, worked out on
 * the page and handed to the geometry store, which places them. A layer's classes are its rules' (a class labels an
 * object its condition holds for), its single label style's or, without one, its objects' kinds' defaults; a class's
 * text is its expression (İfadeyle seç's language) or its template filled with the object's own label. A contour's
 * class gives its height too (its first vertex's). The desktop's is `kentos_native_application::label_texts`.
 */

/** Kinds the engine labels: points and blocks, lines, areas (texts, dimensions, leaders and tables have none). */
const LABELLED = new Set<Entity['kind']>(['point', 'insert', 'line', 'polyline', 'arc', 'spline', 'ellipse', 'polygon', 'circle', 'hatch', 'image', 'raster', 'pointcloud']);

/** One class as the page asks it: its condition, its text (an expression, or the template) and whether it is a contour. */
export interface TextClass {
  readonly when: CompiledExpression | null;
  readonly text: CompiledExpression | null;
  readonly template: string | undefined;
  readonly contour: boolean;
  /** Why the class labels nothing: its text or condition does not compile, or uses `$sıra` or `$ölçek`. */
  readonly error: string | null;
}

/** A layer's classes; `defaults`: its objects' kinds' defaults (index 0 each). */
export type LayerTexts = { readonly off: true } | { readonly off: false; readonly classes: readonly TextClass[]; readonly defaults: boolean };

const compiled = new Map<string, { expr: CompiledExpression | null; error: string | null }>();

/** An expression compiled once per source; `$sıra` and `$ölçek` refused (a label's text does not change with the run or the scale). */
export function labelExpression(source: string): { expr: CompiledExpression | null; error: string | null } {
  let c = compiled.get(source);
  if (!c) {
    const r = compileExpression(source);
    if (!r.ok) c = { expr: null, error: r.at > 1 ? `${r.at}. karakterde: ${r.error}` : r.error };
    else if (r.expr.needs.index) c = { expr: null, error: 'Etikette $sıra kullanılamaz.' };
    else if (r.expr.needs.scale) c = { expr: null, error: 'Etikette $ölçek kullanılamaz.' };
    else c = { expr: r.expr, error: null };
    if (compiled.size > 500) compiled.clear();
    compiled.set(source, c);
  }
  return c;
}

function textClass(style: LabelStyle, when?: string): TextClass {
  const w = when ? labelExpression(when) : { expr: null, error: null };
  const t = style.text ? labelExpression(style.text) : { expr: null, error: null };
  return { when: w.expr, text: t.expr, template: style.template, contour: style.line === 'contour', error: w.error ?? t.error };
}

/** A layer's classes as the page asks them. */
export function layerTexts(node: LayerNode | undefined): LayerTexts {
  const labels: LayerLabels | undefined = node?.style.labels;
  if (labels?.mode === 'off') return { off: true };
  if (labels?.mode === 'rules') return { off: false, classes: (labels.classes ?? []).map((c) => textClass(c.style, c.when)), defaults: false };
  const single = node?.style.label;
  return single ? { off: false, classes: [textClass(single)], defaults: false } : { off: false, classes: [], defaults: true };
}

/** A contour's height: its first vertex's elevation, else NaN. */
function heightOf(e: Entity): number {
  switch (e.kind) {
    case 'polyline':
      return e.zs?.find((z): z is number => z != null) ?? NaN;
    case 'line':
      return e.za ?? NaN;
    case 'point':
      return e.z ?? NaN;
    default:
      return NaN;
  }
}

/** The texts of a list of objects of one layer, ready for the geometry store's `setObjectLabels`. */
export interface PackedTexts {
  ids: Float64Array;
  from: Uint32Array;
  classes: Uint16Array;
  texts: string;
  lens: Uint32Array;
  zs: Float64Array;
}

/**
 * The texts of `list` (one layer's objects): for each object, each class's text (when its condition holds and the
 * text is not empty), in the classes' order. `layerName` names a layer (`$katman`); `geometry` is the store the
 * objects are in (their geometry values).
 */
export function objectTexts(list: readonly Entity[], t: LayerTexts, layerName: (id: string) => string, geometry: ExprGeometry): PackedTexts {
  const n = list.length;
  const out: string[][] = list.map(() => []);
  const cls: number[][] = list.map(() => []);
  const zs = new Float64Array(n).fill(NaN);
  if (!t.off) {
    const objects = { entities: list, layerName, geometry };
    if (t.defaults) {
      for (let i = 0; i < n; i++) {
        const e = list[i];
        const st = DEFAULT_LABELS[e.kind];
        if (st && e.label && LABELLED.has(e.kind)) {
          out[i].push(fillTemplate(st.template, e.label));
          cls[i].push(0);
        }
      }
    } else {
      let contour = false;
      t.classes.forEach((c, k) => {
        if (c.error) return;
        contour ||= c.contour;
        const met = c.when ? c.when.evaluateAll(objects, 'bool') : null;
        const texts = c.text ? c.text.evaluateAll(objects, 'text') : null;
        for (let i = 0; i < n; i++) {
          const e = list[i];
          if (!LABELLED.has(e.kind) || (met && met.value(i) !== true)) continue;
          const v = texts ? texts.value(i) : e.label ? fillTemplate(c.template, e.label) : null;
          const s = v == null ? '' : String(v);
          if (!s) continue;
          out[i].push(s);
          cls[i].push(k);
        }
      });
      if (contour) for (let i = 0; i < n; i++) zs[i] = heightOf(list[i]);
    }
  }
  const ids = new Float64Array(n);
  const from = new Uint32Array(n + 1);
  let total = 0;
  for (let i = 0; i < n; i++) total += out[i].length;
  const classes = new Uint16Array(total);
  const lens = new Uint32Array(total);
  const parts: string[] = [];
  let k = 0;
  for (let i = 0; i < n; i++) {
    ids[i] = list[i].id;
    from[i] = k;
    for (let j = 0; j < out[i].length; j++, k++) {
      classes[k] = cls[i][j];
      lens[k] = out[i][j].length;
      parts.push(out[i][j]);
    }
  }
  from[n] = k;
  return { ids, from, classes, texts: parts.join(''), lens, zs };
}

/**
 * The label engine's view of the layers (docs/adr/0212 §3.1): every layer drawn from its objects with its place in the
 * drawing order (the top first: the tree's first leaf is drawn last, over the others), its point symbol's size and its
 * label fields.
 */
export function labelLayers(layers: LayerStore): string {
  const leaves = layers.leaves().filter((l) => !l.service);
  return JSON.stringify(
    leaves.map((l, i) => ({
      id: l.id,
      rank: i,
      ...(l.style.point ? { point: l.style.point.size } : {}),
      ...(l.style.label ? { label: l.style.label } : {}),
      ...(l.style.labels ? { labels: l.style.labels } : {}),
    })),
  );
}

/** The kinds' default label styles, as the geometry store reads them. */
export const LABEL_DEFAULTS_JSON = JSON.stringify(DEFAULT_LABELS);
