import { MARKER_STRIDE, type BatchEffect, type LayerComposeFrame, type MarkerBatch, type StyledBatch } from './types';

/**
 * Katman çizimi and draw effects (docs/adr/0226 §2–§4): a frame's composing passes, made the same way the desktop
 * makes them (crates/render/wgpu/src/compose.rs, `Planner`) and run by either backend (webgpu/composer.ts,
 * webgl2/composer.ts) with the shared shader (shaders/wgsl/compose, its GLSL twin).
 *
 * A layer whose opacity is below 1, whose mode is not normal or which a mask cuts into is drawn into a picture of
 * its own (`layer`), then laid over the drawing so far or mixed with it (the drawing copied into `backdrop`). An
 * effect group's batches (one symbol layer with effects, over every object) are drawn into a picture of their own
 * (`group`) and laid over their target with their effects in one pass, in the fixed order: shadow, outer glow, the
 * group (blurred, with Bulanıklaştır), inner shadow, inner glow. Each effect reads its own map of the group's alpha,
 * spread or shrunk, shrunk to a half or a quarter for a large blur, blurred; the maps are packed, four pixels a
 * texel (column-packed along x, row-packed along y). A mask is the union of its sources' boxes, grown with round
 * corners. Pure: no GPU, no DOM.
 */

/**
 * The pictures a step reads and writes; the maps by level (1, 2, 4) and which: row-packed (`rows`, four pixels of a
 * row a texel), column-packed (`cols`) and a blurred picture's (`wide`, a pixel a texel).
 */
export type Tex = 'layer' | 'group' | 'backdrop' | 'mask' | 'none' | `rows:${number}:${number}` | `cols:${number}:${number}` | `wide:${number}:${number}`;

/** Where styled batches or a layer go. */
export type Into = 'picture' | 'layer';

export type MapPipe = 'pack' | 'rows' | 'columns' | 'h' | 'v' | 'down' | 'downWide' | 'blurWide';

/** The pictures a pass reads: srcA to srcF. */
export type Reads = readonly [Tex, Tex, Tex, Tex, Tex, Tex];

/** One step of a composed frame, in order; `params` is the pass's `Params` (32 floats). */
export type ComposeStep =
  /** The batches `from` … `to` of the frame's layer `layer` (and its plain parts at its ends) into `into`. */
  | { op: 'batches'; layer: number; from: number; to: number; into: Into }
  | { op: 'layerClear' }
  /** An effect group's batches into its picture, cleared first. */
  | { op: 'group'; layer: number; from: number; to: number }
  | { op: 'map'; pipe: MapPipe; target: Tex; reads: Reads; params: Float32Array }
  | { op: 'effect'; into: Into; reads: Reads; params: Float32Array }
  /**
   * The mask cleared and these boxes drawn, seven floats each: middle x, y and half size in the target's pixels from
   * its top left, the cosine and sine of its turn as the screen turns (made here in double precision: a GPU's own
   * sine may be coarse), growth.
   */
  | { op: 'boxes'; boxes: Float32Array; params: Float32Array }
  /** The drawing copied for a mode's mix. */
  | { op: 'backdrop' }
  /** The layer laid over the drawing (`blend` false) or mixed with it. */
  | { op: 'layer'; blend: boolean; reads: Reads; params: Float32Array };

/** A layer of the frame as the planner sees it. */
export interface PlanLayer {
  id: string;
  batches: readonly StyledBatch[];
  /** Whether batch `i` draws this frame (its prepare said so). */
  drawn(i: number): boolean;
  /** Its plain fills, lines or points (drawn with the whole layer). */
  plain: boolean;
  compose?: LayerComposeFrame;
}

export interface PlanView {
  /** The target's size, device pixels. */
  size: readonly [number, number];
  /** The camera centre from the document's origin, metres. */
  cam: readonly [number, number];
  /** Device pixels per metre and per CSS pixel. */
  pxPerM: number;
  dpr: number;
}

export interface ComposePlan {
  steps: ComposeStep[];
  /** The pictures the steps need. */
  needs: Set<Tex>;
}

/** The largest blur (σ) and spread, device pixels (docs/adr/0226 §2). */
const MAX_SIGMA = 64;
const MAX_SPREAD = 64;

/** The level a blur of σ device pixels is made at: whole up to 8, half up to 32, a quarter beyond. */
export function levelOf(sigma: number): number {
  return sigma <= 8 ? 1 : sigma <= 32 ? 2 : 4;
}

/** The Gaussian weights' sum for σ over ±⌈3σ⌉ (the blur passes divide by it). */
export function gaussSum(sigma: number): number {
  if (sigma <= 0) return 1;
  const r = Math.ceil(3 * sigma);
  let s = 0;
  for (let k = -r; k <= r; k++) s += Math.exp(-(k * k) / (2 * sigma * sigma));
  return s;
}

/** A map's size at `level`: the picture's, halved or quartered, rounded up. */
export function levelSize(size: readonly [number, number], level: number): [number, number] {
  return [Math.max(1, Math.ceil(size[0] / level)), Math.max(1, Math.ceil(size[1] / level))];
}

/** A row-packed map's size in texels at `level`: four pixels of a row a texel. */
export function rowsSize(size: readonly [number, number], level: number): [number, number] {
  const [w, h] = levelSize(size, level);
  return [Math.ceil(w / 4), h];
}

/** A column-packed one's: four pixels of a column a texel. */
export function colsSize(size: readonly [number, number], level: number): [number, number] {
  const [w, h] = levelSize(size, level);
  return [w, Math.ceil(h / 4)];
}

/** A picture's size in texels: a map's by its form and level, the target's for the others. */
export function texSize(t: Tex, size: readonly [number, number]): [number, number] {
  const [form, level] = t.split(':');
  if (form === 'rows') return rowsSize(size, Number(level));
  if (form === 'cols') return colsSize(size, Number(level));
  if (form === 'wide') return levelSize(size, Number(level));
  return [size[0], size[1]];
}

/** A pass's numbers: its first `vec4f`s from `a`. */
function numbers(a: readonly number[]): Float32Array {
  const out = new Float32Array(32);
  out.set(a, 0);
  return out;
}

/**
 * A map pass's numbers: its work (2 a blur, 3 the largest, 4 the smallest), σ or n, a blur's weights' sum (for σ as
 * the shader takes it, in single precision); the map's size in pixels, its source's in texels.
 */
function work(op: number, value: number, size: readonly [number, number], source: readonly [number, number]): Float32Array {
  const v = Math.fround(value);
  return numbers([op, v, op === 2 ? gaussSum(v) : 1, 0, size[0], size[1], source[0], source[1]]);
}

/**
 * One of an effect group's alpha maps (docs/adr/0226 §2): its kind (0 the shadow, 1 the outer glow, 2 the inner
 * shadow, 3 the inner glow), its square filter's n (+ spread, − shrink; 0 none), its blur's σ and the level the blur is
 * made at, device pixels.
 */
interface Chain {
  kind: number;
  n: number;
  sigma: number;
  level: number;
}

function chainOf(kind: number, spread: number, sigma: number): Chain {
  const n = Math.round(Math.min(Math.abs(spread), MAX_SPREAD));
  const s = Math.min(sigma, MAX_SIGMA);
  return { kind, n: n >= 1 ? (spread < 0 ? -n : n) : 0, sigma: s, level: levelOf(s) };
}

/** A batch's effect group: its id and effects, or null. */
function effectOf(b: StyledBatch | undefined): { group: number; effect: BatchEffect } | null {
  return b && b.group && b.group > 0 && b.effect ? { group: b.group, effect: b.effect } : null;
}

/**
 * The boxes of a marker batch's markers as the marker shader places them (`markerVs`), grown by `grow` device
 * pixels: a mask's shapes (docs/adr/0226 §3). `aspect` is an image's height over its width (the atlas's).
 */
export function markerBoxes(b: MarkerBatch, aspect: number, view: PlanView, grow: number, out: number[]): void {
  const k = b.unit === 'world' ? view.pxPerM : view.dpr;
  const fit = b.look.kind === 'image' ? (b.look.fit === 'height' ? 2 : 1) : 0;
  const a = Math.max(aspect, 1e-6);
  const origin = b.origin ?? [0, 0];
  const tile = [view.cam[0] - origin[0], view.cam[1] - origin[1]];
  const [vw, vh] = view.size;
  const d = b.instances;
  for (let i = 0; i + MARKER_STRIDE <= d.length; i += MARKER_STRIDE) {
    let w = d[i + 3] * k;
    let h = d[i + 4] * k;
    if (fit === 1) h = w * a;
    else if (fit === 2) w = h / a;
    else if (h <= 0) h = w;
    const qx = -b.anchor[0] * w + b.offset[0] * k;
    const qy = -b.anchor[1] * h + b.offset[1] * k;
    const turn = d[i + 2];
    const s = Math.sin(turn);
    const c = Math.cos(turn);
    const x = vw / 2 + (d[i] - tile[0]) * view.pxPerM + (c * qx - s * qy);
    const y = vh / 2 - (d[i + 1] - tile[1]) * view.pxPerM - (s * qx + c * qy);
    const reach = Math.max(w, h) + grow + 2;
    if (x < -reach || y < -reach || x > vw + reach || y > vh + reach) continue;
    // Turned as the screen turns (y down): the other way.
    out.push(x, y, w / 2, h / 2, c, -s, grow);
  }
}

class Planner {
  readonly steps: ComposeStep[] = [];
  readonly needs = new Set<Tex>();
  private readonly view: PlanView;

  constructor(view: PlanView) {
    this.view = view;
  }

  private need(t: Tex): void {
    if (t !== 'none') this.needs.add(t);
  }

  batches(layer: number, from: number, to: number, into: Into): void {
    if (into === 'layer') this.need('layer');
    this.steps.push({ op: 'batches', layer, from, to, into });
  }

  layerClear(): void {
    this.need('layer');
    this.steps.push({ op: 'layerClear' });
  }

  private px(unit: BatchEffect['unit'], v: number): number {
    return Math.max(v * (unit === 'world' ? this.view.pxPerM : this.view.dpr), 0);
  }

  private map(pipe: MapPipe, from: Tex, to: Tex, params: Float32Array): void {
    this.need(to);
    this.need(from);
    this.steps.push({ op: 'map', pipe, target: to, reads: [from, 'none', 'none', 'none', 'none', 'none'], params });
  }

  private size(level: number): [number, number] {
    return levelSize(this.view.size, level);
  }

  /** A column-packed map's filter along x, row-packed, along y into `to`, at `level`; between them the level's scratch maps. */
  private filtered(from: Tex, scratch: number, to: Tex, op: number, value: number, level: number): void {
    const between: Tex = `cols:${level}:${scratch}`;
    const rows: Tex = `rows:${level}:${scratch}`;
    const size = this.size(level);
    const cols = colsSize(this.view.size, level);
    this.map('h', from, between, work(op, value, size, cols));
    this.map('rows', between, rows, work(0, 0, size, cols));
    this.map('v', rows, to, work(op, value, size, rowsSize(this.view.size, level)));
  }

  private alphaRowsMade = false;

  /** The group's alpha row-packed, made once a group from its column-packed form. */
  private alphaRows(): Tex {
    if (!this.alphaRowsMade) {
      this.map('rows', 'cols:1:0', 'rows:1:0', work(0, 0, this.size(1), colsSize(this.view.size, 1)));
      this.alphaRowsMade = true;
    }
    return 'rows:1:0';
  }

  /**
   * An effect's row-packed map of the group's alpha (column-packed whole, `cols:1:0`): its square filter whole, the map
   * halved to its level, its blur there. The map: the effect's own, or the alpha itself.
   */
  private chain(c: Chain): Tex {
    const own = (level: number): Tex => `rows:${level}:${4 + c.kind}`;
    let rows: Tex | null = null;
    if (c.n !== 0) {
      const to = c.sigma > 0 ? 'rows:1:2' : own(1);
      this.filtered('cols:1:0', 1, to, c.n > 0 ? 3 : 4, Math.abs(c.n), 1);
      if (c.sigma <= 0) return to;
      rows = to;
    } else if (c.sigma <= 0) return this.alphaRows();
    const level = c.level;
    let cols: Tex;
    if (level > 1) {
      let from = rows ?? this.alphaRows();
      for (let at = 1; at < level; at *= 2) {
        const next: Tex = `rows:${at * 2}:1`;
        this.map('down', from, next, work(0, 0, this.size(at * 2), rowsSize(this.view.size, at)));
        from = next;
      }
      cols = `cols:${level}:1`;
      this.map('columns', from, cols, work(0, 0, this.size(level), rowsSize(this.view.size, level)));
    } else if (rows) {
      cols = 'cols:1:2';
      this.map('columns', rows, cols, work(0, 0, this.size(1), rowsSize(this.view.size, 1)));
    } else cols = 'cols:1:0';
    this.filtered(cols, 3, own(level), 2, c.sigma / level, level);
    return own(level);
  }

  /** The group's picture blurred by σ at its level (Bulanıklaştır), four channels a pixel. */
  private blurred(sigma: number): [Tex, number] {
    sigma = Math.min(sigma, MAX_SIGMA);
    const level = levelOf(sigma);
    let from: Tex = 'group';
    for (let at = 1; at < level; at *= 2) {
      const next: Tex = `wide:${at * 2}:0`;
      this.map('downWide', from, next, work(0, 0, this.size(at * 2), this.size(at)));
      from = next;
    }
    const s = Math.fround(sigma / level);
    const sum = gaussSum(s);
    const [w, h] = this.size(level);
    this.map('blurWide', from, `wide:${level}:1`, numbers([0, s, sum, 0, 1, 0, w, h]));
    this.map('blurWide', `wide:${level}:1`, `wide:${level}:2`, numbers([0, s, sum, 0, 0, 1, w, h]));
    return [`wide:${level}:2`, level];
  }

  /** The scale from the target's uv to a map's of `level`. */
  private mapScale(level: number): [number, number] {
    const [w, h] = this.view.size;
    const s = levelSize(this.view.size, level);
    return [w / (level * s[0]), h / (level * s[1])];
  }

  /** An effect group's batches and its effects over `into` (docs/adr/0226 §2). */
  group(layer: number, from: number, to: number, effect: BatchEffect, into: Into): void {
    this.need('group');
    if (into === 'layer') this.need('layer');
    this.steps.push({ op: 'group', layer, from, to });
    const unit = effect.unit;
    const k = unit === 'world' ? this.view.pxPerM : this.view.dpr;
    // A shadow's offset in device pixels: x right, y down (the effect's y is up).
    const shift = (dx: number, dy: number): [number, number] => [dx * k, -dy * k];
    const params = new Float32Array(32);
    const chains: Chain[] = [];
    if (effect.shadow) {
      const s = effect.shadow;
      chains.push(chainOf(0, 0, this.px(unit, s.blur) / 3));
      params.set(shift(s.dx, s.dy), 0);
      params.set(s.color, 8);
    }
    if (effect.glow) {
      const g = effect.glow;
      chains.push(chainOf(1, this.px(unit, g.spread), this.px(unit, g.blur) / 3));
      params.set(g.color, 12);
    }
    if (effect.innerShadow) {
      const s = effect.innerShadow;
      chains.push(chainOf(2, 0, this.px(unit, s.blur) / 3));
      params.set(shift(s.dx, s.dy), 2);
      params.set(s.color, 16);
    }
    if (effect.innerGlow) {
      const g = effect.innerGlow;
      chains.push(chainOf(3, -this.px(unit, g.spread), this.px(unit, g.blur) / 3));
      params.set(g.color, 20);
    }
    // The group's alpha column-packed once; each effect's map from it, read from srcB … srcE.
    const reads: Tex[] = ['group', 'none', 'none', 'none', 'none', 'none'];
    if (chains.length) {
      this.map('pack', 'group', 'cols:1:0', work(0, 0, this.size(1), this.view.size));
      this.alphaRowsMade = false;
      for (const c of chains) {
        reads[1 + c.kind] = this.chain(c);
        // A map without a blur is whole.
        params[4 + c.kind] = c.sigma > 0 ? c.level : 1;
      }
    }
    const blur = effect.blur ?? 0;
    if (blur > 0 && this.px(unit, blur) >= 0.5) {
      const [map, level] = this.blurred(this.px(unit, blur) / 3);
      reads[5] = map;
      const scale = this.mapScale(level);
      params.set([level, 0, scale[0], scale[1]], 24);
    }
    params.set(this.view.size, 28);
    for (const r of reads) this.need(r);
    this.steps.push({ op: 'effect', into, reads: reads as unknown as Reads, params });
  }

  /** The layer's picture over the drawing: faded, masked by `boxes`, mixed by `blend`. */
  layer(opacity: number, blend: number, boxes: number[] | null): void {
    const masked = !!boxes && boxes.length > 0;
    const [w, h] = this.view.size;
    if (masked) {
      // The boxes draw into the mask: it is their target, not a picture they read.
      this.need('mask');
      this.steps.push({ op: 'boxes', boxes: new Float32Array(boxes), params: numbers([w, h, 0, 0]) });
    }
    const mixed = blend !== 0;
    if (mixed) {
      this.need('backdrop');
      this.steps.push({ op: 'backdrop' });
    }
    this.need('layer');
    this.steps.push({
      op: 'layer',
      blend: mixed,
      reads: ['layer', mixed ? 'backdrop' : 'none', masked ? 'mask' : 'none', 'none', 'none', 'none'],
      params: numbers([opacity, masked ? 1 : 0, blend, 0]),
    });
  }
}

/** A stretch of a layer's batches split into its plain runs and its effect groups' runs. */
function runs(p: Planner, li: number, layer: PlanLayer, into: Into): void {
  const n = layer.batches.length;
  if (n === 0) {
    // Its plain parts only.
    if (layer.plain) p.batches(li, 0, 0, into);
    return;
  }
  let k = 0;
  while (k < n) {
    const start = k;
    const e = effectOf(layer.batches[k]);
    if (!e) {
      while (k < n && !effectOf(layer.batches[k])) k++;
      p.batches(li, start, k, into);
      continue;
    }
    // One symbol layer's group (the core gives a group one id for its level and effects), batch after batch.
    while (k < n && effectOf(layer.batches[k])?.group === e.group) k++;
    let drawn = false;
    for (let j = start; j < k && !drawn; j++) drawn = layer.drawn(j);
    if (!drawn) {
      // Nothing of it shows; the layer's plain parts at its ends still draw (a step draws a layer's plain fills
      // when it starts at its first batch, its plain lines and points when it ends at its last).
      if (layer.plain) p.batches(li, start, k, into);
      continue;
    }
    if (layer.plain && start === 0) p.batches(li, 0, 0, into);
    p.group(li, start, k, e.effect, into);
    if (layer.plain && k === n) p.batches(li, n, n, into);
  }
}

/**
 * The composed frame's steps from the layers in draw order (docs/adr/0226): the plain stretches straight into the
 * picture, each composed layer into its own picture and laid over or mixed in, each effect group into its own with
 * its effects. `markers` adds the boxes of a layer's drawn markers, grown by `grow` device pixels. Null when nothing
 * composes: the frame draws as it always did.
 */
export function planCompose(layers: readonly PlanLayer[], view: PlanView, markers: (id: string, grow: number, out: number[]) => void): ComposePlan | null {
  const composes = layers.some((l) => {
    if (l.compose) return true;
    for (let i = 0; i < l.batches.length; i++) if (effectOf(l.batches[i]) && l.drawn(i)) return true;
    return false;
  });
  if (!composes) return null;
  const p = new Planner(view);
  const [w, h] = view.size;
  layers.forEach((layer, li) => {
    const c = layer.compose;
    if (!c) {
      runs(p, li, layer, 'picture');
      return;
    }
    let drawn = layer.plain;
    for (let i = 0; i < layer.batches.length && !drawn; i++) drawn = layer.drawn(i);
    if (!drawn) return;
    p.layerClear();
    runs(p, li, layer, 'layer');
    let boxes: number[] | null = null;
    if (c.masks.length) {
      boxes = [];
      for (const m of c.masks) {
        const grow = m.grow * view.dpr;
        const l = m.labels;
        for (let i = 0; i + 5 <= l.length; i += 5) {
          const turn = (-l[i + 4] * Math.PI) / 180;
          boxes.push(
            w / 2 + (l[i] - view.cam[0]) * view.pxPerM,
            h / 2 - (l[i + 1] - view.cam[1]) * view.pxPerM,
            (l[i + 2] * view.dpr) / 2,
            (l[i + 3] * view.dpr) / 2,
            Math.cos(turn),
            Math.sin(turn),
            grow,
          );
        }
        if (m.markers !== null) markers(m.markers, grow, boxes);
      }
    }
    p.layer(c.opacity, c.blend, boxes);
  });
  return { steps: p.steps, needs: p.needs };
}

/**
 * A key of the frame's compositions: a kept picture is drawn again when it changes (a new opacity, mode or mask, a
 * label placed elsewhere).
 */
export function composeKey(compositions: ReadonlyMap<string, LayerComposeFrame> | undefined): string {
  if (!compositions?.size) return '';
  const parts: string[] = [];
  for (const [id, c] of compositions) {
    let sum = 0;
    for (const m of c.masks) {
      const l = m.labels;
      for (let i = 0; i < l.length; i++) sum = (sum * 31 + l[i] * (i + 1)) % 1e15;
      sum += m.grow + (m.markers ? m.markers.length : 0);
    }
    parts.push(`${id}:${c.opacity}:${c.blend}:${c.masks.length}:${c.masks.map((m) => `${m.markers ?? ''}/${m.labels.length}`).join(',')}:${sum}`);
  }
  return parts.join('|');
}
