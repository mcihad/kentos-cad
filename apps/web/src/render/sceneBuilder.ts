import type { Entity } from '../model/entities';
import { hatchLines } from '../model/geom/hatch';
import type { Bounds, Vec2 } from '../model/geometry';
import type { LayerStyle, LineType } from '../model/layers';
import { DrawnReader } from '../style/geometry';
import { parseHex, resolveColor, withAlpha, type CanvasPalette } from './color';
import { FillQueue } from './fillQueue';
import type { GeometrySource } from './styledLayer';
import type { LineBatch, PointBatch, RGBA, SceneLayer } from './types';

export const DASH_PATTERNS: Record<LineType, readonly number[] | null> = {
  continuous: null,
  dashed: [9, 5],
  dashdot: [14, 4, 2, 4],
  dotted: [2, 4],
};

class LineAccumulator {
  pos: number[] = [];
  dist: number[] = [];

  path(pts: readonly Vec2[], origin: Vec2, closed: boolean): void {
    let d = 0;
    const n = pts.length;
    const count = closed ? n : n - 1;
    for (let i = 0; i < count; i++) {
      const a = pts[i];
      const b = pts[(i + 1) % n];
      const ax = a.x - origin.x;
      const ay = a.y - origin.y;
      const bx = b.x - origin.x;
      const by = b.y - origin.y;
      const len = Math.hypot(bx - ax, by - ay);
      this.pos.push(ax, ay, bx, by);
      this.dist.push(d, d + len);
      d += len;
    }
  }

  batch(color: RGBA, dash: readonly number[] | null): LineBatch | null {
    if (!this.pos.length) return null;
    return { positions: new Float32Array(this.pos), distances: new Float32Array(this.dist), color, dash };
  }
}

export interface BuildOptions {
  origin: Vec2;
  palette: CanvasPalette;
  /** What the objects draw (the geometry store; rings as the objects have them). */
  geometry: GeometrySource;
  /** Force every batch into one colour (selection / hover highlight). */
  overrideColor?: RGBA;
  overrideFill?: RGBA | null;
  overrideDash?: readonly number[] | null;
  pointStyle?: Pick<PointBatch, 'size' | 'shape'>;
  /**
   * World box infinite construction lines are clipped to (a few view sizes
   * around the camera). Keeps GPU coordinates small; the viewport rebuilds
   * those layers when the view leaves it.
   */
  clip?: Bounds;
}

/** Converts entities of one layer (or a highlight set) into GPU-ready batches. */
export function buildSceneLayer(id: string, entities: readonly Entity[], style: LayerStyle, opts: BuildOptions): SceneLayer {
  const layer: SceneLayer = { id, lines: [], fills: [], points: [] };
  // Solid hatches get their own, stronger fill than the layer's polygon fill.
  const byColor = new Map<string, { lines: LineAccumulator; fill: number[]; solid: number[]; points: number[] }>();
  const bucket = (color: string) => {
    let b = byColor.get(color);
    if (!b) byColor.set(color, (b = { lines: new LineAccumulator(), fill: [], solid: [], points: [] }));
    return b;
  };
  const { origin } = opts;
  // Curves tessellated by the geometry store, all objects in one call; fills triangulated together.
  const drawn = new DrawnReader(opts.geometry.drawn(entities.map((e) => e.id), false, opts.clip));
  const fills = new FillQueue();

  for (const e of entities) {
    const g = drawn.read(e);
    const b = bucket(e.color ?? style.color);
    switch (e.kind) {
      case 'polygon':
        if (g?.cls !== 'fill') break;
        for (const r of g.rings) b.lines.path(r, origin, true);
        if (style.fill || opts.overrideFill) fills.add(b.fill, g.rings);
        break;
      case 'hatch':
        if (g?.cls !== 'fill') break;
        if (opts.overrideColor) {
          // Highlight: outline plus a light fill regardless of pattern.
          for (const r of g.rings) b.lines.path(r, origin, true);
          if (opts.overrideFill) fills.add(b.fill, g.rings);
        } else if (e.pattern.type === 'solid') fills.add(b.solid, g.rings);
        else {
          for (const [p, q] of hatchLines(e.ring, e.pattern.angle, e.pattern.spacing, e.holes).segments) b.lines.path([p, q], origin, false);
          if (e.pattern.type === 'cross')
            for (const [p, q] of hatchLines(e.ring, e.pattern.angle + 90, e.pattern.spacing, e.holes).segments) b.lines.path([p, q], origin, false);
        }
        break;
      case 'point':
        b.points.push(e.p.x - origin.x, e.p.y - origin.y);
        break;
      case 'text':
        break; // text is drawn by the overlay for now (SDF text is a later milestone)
      default:
        // Lines, paths, curves, construction lines (clipped) and dimensions (their layout lines).
        if (g?.cls === 'line') for (const p of g.paths) b.lines.path(p.pts, origin, p.closed);
    }
  }
  fills.run(origin);

  const dash = opts.overrideDash !== undefined ? opts.overrideDash : DASH_PATTERNS[style.lineType];
  for (const [color, b] of byColor) {
    const rgba = opts.overrideColor ?? parseHex(resolveColor(color, opts.palette));
    const lines = b.lines.batch(rgba, dash);
    if (lines) layer.lines.push(lines);
    if (b.fill.length) {
      const fill = opts.overrideFill ?? (style.fill ? parseHex(style.fill) : withAlpha(rgba, 0.12));
      layer.fills.push({ positions: new Float32Array(b.fill), color: fill });
    }
    if (b.solid.length) layer.fills.push({ positions: new Float32Array(b.solid), color: withAlpha(rgba, 0.45) });
    if (b.points.length) {
      const pb: PointBatch = {
        positions: new Float32Array(b.points),
        color: rgba,
        size: opts.pointStyle?.size ?? style.point?.size ?? 7,
        shape: opts.pointStyle?.shape ?? style.point?.symbol ?? 'ring',
      };
      layer.points.push(pb);
    }
  }
  return layer;
}
