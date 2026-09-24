import { apply, emptyBox, growBox, multiply, parsePathData, pathDataOf, subPathsBox, transformSubPaths, type Box, type Matrix, type SubPath } from './pathData';

/**
 * The drawing model of KentOS's own SVG editor (docs/STYLE.md §7): a
 * canvas (viewBox) and shapes on it, back to front. Rectangles, ellipses
 * and texts keep their kind (so they stay editable as such); everything
 * else is a path of Bézier nodes. Paints are "the symbol's colour"
 * (currentColor, recoloured by the symbol), "the second colour"
 * (param(stroke)), a fixed colour or none. Pure: the editor's view and the
 * SVG file are both made from this.
 */

/** 'fill': the symbol's colour; 'stroke': the symbol's second colour; '#RRGGBB[AA]': fixed; 'none'. */
export type Paint = 'none' | 'fill' | 'stroke' | string;

interface ShapeBase {
  id: string;
  fill: Paint;
  stroke: Paint;
  strokeWidth: number;
  opacity?: number;
  /** Stroke dash lengths (on, off …); absent = continuous. */
  dash?: number[];
  /** Line ends and corners; absent = round (paths) or the SVG default (other shapes). */
  cap?: 'butt' | 'round' | 'square';
  join?: 'miter' | 'round' | 'bevel';
  /** Which parts of self-crossing or nested sub-paths are inside; absent = evenodd for paths. */
  fillRule?: 'nonzero' | 'evenodd';
  /** Shapes with the same group move and select together. */
  group?: string;
  hidden?: boolean;
  /** Not picked on the canvas nor moved by tools (the list and the panels still reach it). */
  locked?: boolean;
  name?: string;
}

export type SvgShape =
  | (ShapeBase & { kind: 'rect'; x: number; y: number; w: number; h: number; r?: number; rotate?: number })
  | (ShapeBase & { kind: 'ellipse'; cx: number; cy: number; rx: number; ry: number; rotate?: number })
  | (ShapeBase & { kind: 'path'; subs: SubPath[] })
  | (ShapeBase & { kind: 'text'; x: number; y: number; text: string; size: number; weight: 400 | 700 | 900; font: 'sans' | 'serif'; anchor: 'start' | 'middle' | 'end'; rotate?: number });

/** A guide: an endless line through (x, y) at `angle` degrees from the x axis (0 horizontal, 90 vertical). */
export interface Guide {
  id: string;
  x: number;
  y: number;
  angle: number;
}

export interface SvgDoc {
  width: number;
  height: number;
  shapes: SvgShape[];
  /** Intended width of the drawing on the map in mm (document properties; "1 birim = … mm"). */
  sizeMm?: number;
  /** Preview background (paper) colour kept with the drawing; absent = the theme's paper. */
  background?: string;
  /** Guide lines of the editor (snap targets); editing only, the SVG file does not keep them. */
  guides?: Guide[];
}

export const newDoc = (width = 100, height = 100): SvgDoc => ({ width, height, shapes: [] });

let seq = 0;
export const shapeId = () => `s${Date.now().toString(36)}${(seq++).toString(36)}`;

// ── To SVG ─────────────────────────────────────────────────────────────

/** A paint as an SVG attribute value (the symbol's colours as KentOS parameters). */
export const paintValue = (p: Paint): string => (p === 'fill' ? 'currentColor' : p === 'stroke' ? 'param(stroke) #000000' : p);

const n = (v: number) => {
  const s = (Math.round(v * 1000) / 1000).toString();
  return s === '-0' ? '0' : s;
};

const FONT: Record<'sans' | 'serif', string> = { sans: 'Arial, Helvetica, sans-serif', serif: '"Times New Roman", Times, serif' };

/** A shape as an element: tag and attributes (the view builds DOM from it, the file writes it). */
export function elementOf(s: SvgShape, paint: (p: Paint) => string = paintValue): { tag: string; attrs: Record<string, string>; text?: string } {
  const attrs: Record<string, string> = { fill: paint(s.fill), stroke: paint(s.stroke) };
  if (s.stroke !== 'none') attrs['stroke-width'] = n(s.strokeWidth);
  if (s.opacity !== undefined && s.opacity < 1) attrs.opacity = n(s.opacity);
  if (s.stroke !== 'none' && s.dash?.length) attrs['stroke-dasharray'] = s.dash.map(n).join(' ');
  if (s.cap) attrs['stroke-linecap'] = s.cap;
  if (s.join) attrs['stroke-linejoin'] = s.join;
  if (s.fillRule) attrs['fill-rule'] = s.fillRule;
  switch (s.kind) {
    case 'rect': {
      Object.assign(attrs, { x: n(s.x), y: n(s.y), width: n(s.w), height: n(s.h) });
      if (s.r) attrs.rx = n(s.r);
      if (s.rotate) attrs.transform = `rotate(${n(s.rotate)} ${n(s.x + s.w / 2)} ${n(s.y + s.h / 2)})`;
      return { tag: 'rect', attrs };
    }
    case 'ellipse':
      Object.assign(attrs, { cx: n(s.cx), cy: n(s.cy), rx: n(s.rx), ry: n(s.ry) });
      if (s.rotate) attrs.transform = `rotate(${n(s.rotate)} ${n(s.cx)} ${n(s.cy)})`;
      return { tag: 'ellipse', attrs };
    case 'path':
      attrs.d = pathDataOf(s.subs);
      attrs['fill-rule'] ??= 'evenodd';
      attrs['stroke-linejoin'] ??= 'round';
      attrs['stroke-linecap'] ??= 'round';
      return { tag: 'path', attrs };
    case 'text':
      Object.assign(attrs, { x: n(s.x), y: n(s.y), 'font-family': FONT[s.font], 'font-size': n(s.size), 'font-weight': String(s.weight), 'text-anchor': s.anchor });
      if (s.stroke === 'none') delete attrs.stroke;
      if (s.rotate) attrs.transform = `rotate(${n(s.rotate)} ${n(s.x)} ${n(s.y)})`;
      return { tag: 'text', attrs, text: s.text };
  }
}

const esc = (v: string) => v.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** The drawing as an SVG file (groups become <g>, hidden shapes are left out). */
export function serializeDoc(doc: SvgDoc): string {
  const out: string[] = [`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${n(doc.width)} ${n(doc.height)}" width="${n(doc.width)}" height="${n(doc.height)}">`];
  let open: string | undefined;
  for (const s of doc.shapes) {
    if (s.hidden) continue;
    if (s.group !== open) {
      if (open) out.push('</g>');
      if (s.group) out.push(`<g data-group="${esc(s.group)}">`);
      open = s.group;
    }
    const e = elementOf(s);
    const attrs = Object.entries(e.attrs)
      .map(([k, v]) => `${k}="${esc(v)}"`)
      .join(' ');
    out.push(e.text !== undefined ? `<${e.tag} ${attrs}>${esc(e.text)}</${e.tag}>` : `<${e.tag} ${attrs}/>`);
  }
  if (open) out.push('</g>');
  out.push('</svg>');
  return out.join('');
}

// ── Geometry ───────────────────────────────────────────────────────────

const rad = (deg: number) => (deg * Math.PI) / 180;
const rotateAbout = (deg: number, cx: number, cy: number): Matrix => {
  const c = Math.cos(rad(deg));
  const s = Math.sin(rad(deg));
  return [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy];
};

/** A rectangle or ellipse as a path (for transforms they cannot keep). */
export function toPath(s: SvgShape): Extract<SvgShape, { kind: 'path' }> | Extract<SvgShape, { kind: 'text' }> {
  if (s.kind === 'path' || s.kind === 'text') return s;
  const base = { id: s.id, fill: s.fill, stroke: s.stroke, strokeWidth: s.strokeWidth, opacity: s.opacity, dash: s.dash, cap: s.cap, join: s.join, fillRule: s.fillRule, group: s.group, hidden: s.hidden, name: s.name };
  let subs: SubPath[];
  if (s.kind === 'rect') {
    const r = Math.min(s.r ?? 0, s.w / 2, s.h / 2);
    const k = 0.5523 * r;
    const { x, y, w, h } = s;
    subs = [
      {
        closed: true,
        nodes: r
          ? [
              { x: x + r, y, in: [x + r - k, y] },
              { x: x + w - r, y, out: [x + w - r + k, y] },
              { x: x + w, y: y + r, in: [x + w, y + r - k] },
              { x: x + w, y: y + h - r, out: [x + w, y + h - r + k] },
              { x: x + w - r, y: y + h, in: [x + w - r + k, y + h] },
              { x: x + r, y: y + h, out: [x + r - k, y + h] },
              { x, y: y + h - r, in: [x, y + h - r + k] },
              { x, y: y + r, out: [x, y + r - k] },
            ]
          : [
              { x, y },
              { x: x + w, y },
              { x: x + w, y: y + h },
              { x, y: y + h },
            ],
      },
    ];
    if (s.rotate) subs = transformSubPaths(subs, rotateAbout(s.rotate, x + w / 2, y + h / 2));
  } else {
    const { cx, cy, rx, ry } = s;
    const kx = 0.5523 * rx;
    const ky = 0.5523 * ry;
    subs = [
      {
        closed: true,
        nodes: [
          { x: cx + rx, y: cy, in: [cx + rx, cy - ky], out: [cx + rx, cy + ky] },
          { x: cx, y: cy + ry, in: [cx + kx, cy + ry], out: [cx - kx, cy + ry] },
          { x: cx - rx, y: cy, in: [cx - rx, cy + ky], out: [cx - rx, cy - ky] },
          { x: cx, y: cy - ry, in: [cx - kx, cy - ry], out: [cx + kx, cy - ry] },
        ],
      },
    ];
    if (s.rotate) subs = transformSubPaths(subs, rotateAbout(s.rotate, cx, cy));
  }
  return { ...base, kind: 'path', subs };
}

/** Text is measured roughly (0.55 em per letter), like the drawing's text boxes. */
function textBox(s: Extract<SvgShape, { kind: 'text' }>): Box {
  const w = s.size * 0.55 * Math.max(1, s.text.length) * (s.weight >= 700 ? 1.08 : 1);
  const x0 = s.anchor === 'start' ? s.x : s.anchor === 'middle' ? s.x - w / 2 : s.x - w;
  const b = emptyBox();
  const corners: [number, number][] = [
    [x0, s.y - s.size * 0.75],
    [x0 + w, s.y - s.size * 0.75],
    [x0 + w, s.y + s.size * 0.22],
    [x0, s.y + s.size * 0.22],
  ];
  const m = s.rotate ? rotateAbout(s.rotate, s.x, s.y) : null;
  for (const [x, y] of corners) {
    const p = m ? apply(m, x, y) : [x, y];
    growBox(b, p[0], p[1]);
  }
  return b;
}

export function shapeBox(s: SvgShape): Box {
  if (s.kind === 'text') return textBox(s);
  if (s.kind === 'path') return subPathsBox(s.subs);
  if (!s.rotate) return s.kind === 'rect' ? { minX: s.x, minY: s.y, maxX: s.x + s.w, maxY: s.y + s.h } : { minX: s.cx - s.rx, minY: s.cy - s.ry, maxX: s.cx + s.rx, maxY: s.cy + s.ry };
  return shapeBox(toPath(s));
}

export function shapesBox(shapes: readonly SvgShape[]): Box | null {
  const b = emptyBox();
  for (const s of shapes) {
    const sb = shapeBox(s);
    growBox(b, sb.minX, sb.minY);
    growBox(b, sb.maxX, sb.maxY);
  }
  return Number.isFinite(b.minX) ? b : null;
}

/**
 * A shape under an affine map. Translations and scalings without rotation
 * keep rectangles and ellipses; rotations keep them when uniform (the
 * rotate property grows); anything else turns them into paths. Text moves
 * and scales its size.
 */
export function transformShape(s: SvgShape, m: Matrix): SvgShape {
  const sx = Math.hypot(m[0], m[1]);
  const sy = Math.hypot(m[2], m[3]);
  const angle = (Math.atan2(m[1], m[0]) * 180) / Math.PI;
  const axisAligned = Math.abs(m[1]) < 1e-12 && Math.abs(m[2]) < 1e-12;
  const uniform = Math.abs(sx - sy) < 1e-9 && Math.abs(m[0] * m[2] + m[1] * m[3]) < 1e-9 && m[0] * m[3] - m[1] * m[2] > 0;
  switch (s.kind) {
    case 'path':
      return { ...s, subs: transformSubPaths(s.subs, m) };
    case 'text': {
      const [x, y] = apply(m, s.x, s.y);
      return { ...s, x, y, size: s.size * Math.sqrt(Math.abs(m[0] * m[3] - m[1] * m[2])), rotate: axisAligned ? s.rotate : (s.rotate ?? 0) + angle };
    }
    case 'rect': {
      if (axisAligned && m[0] > 0 && m[3] > 0 && !s.rotate) {
        const [x, y] = apply(m, s.x, s.y);
        return { ...s, x, y, w: s.w * m[0], h: s.h * m[3], r: s.r ? s.r * Math.min(m[0], m[3]) : undefined };
      }
      if (uniform) {
        const [cx, cy] = apply(m, s.x + s.w / 2, s.y + s.h / 2);
        const w = s.w * sx;
        const h = s.h * sx;
        return { ...s, x: cx - w / 2, y: cy - h / 2, w, h, r: s.r ? s.r * sx : undefined, rotate: (s.rotate ?? 0) + angle };
      }
      return transformShape(toPath(s), m);
    }
    case 'ellipse': {
      if (axisAligned && !s.rotate) {
        const [cx, cy] = apply(m, s.cx, s.cy);
        return { ...s, cx, cy, rx: s.rx * Math.abs(m[0]), ry: s.ry * Math.abs(m[3]) };
      }
      if (uniform) {
        const [cx, cy] = apply(m, s.cx, s.cy);
        return { ...s, cx, cy, rx: s.rx * sx, ry: s.ry * sx, rotate: (s.rotate ?? 0) + angle };
      }
      return transformShape(toPath(s), m);
    }
  }
}

export const translate = (dx: number, dy: number): Matrix => [1, 0, 0, 1, dx, dy];
/** Maps box `a` onto box `b` (scaling about their corners). */
export function boxToBox(a: Box, b: Box): Matrix {
  const sx = (b.maxX - b.minX) / Math.max(a.maxX - a.minX, 1e-9);
  const sy = (b.maxY - b.minY) / Math.max(a.maxY - a.minY, 1e-9);
  return [sx, 0, 0, sy, b.minX - a.minX * sx, b.minY - a.minY * sy];
}
export const rotation = rotateAbout;
export { multiply };

/** A regular polygon (or star with an inner radius) as a closed path, first corner up. */
export function regularPolygon(cx: number, cy: number, r: number, sides: number, inner?: number): SubPath {
  const pts = inner ? sides * 2 : sides;
  return {
    closed: true,
    nodes: Array.from({ length: pts }, (_, i) => {
      const a = -Math.PI / 2 + (i * 2 * Math.PI) / pts;
      const rr = inner && i % 2 ? inner : r;
      return { x: cx + Math.cos(a) * rr, y: cy + Math.sin(a) * rr };
    }),
  };
}

/** Path data read into a path shape (imports). */
export const pathShape = (d: string, style: Omit<ShapeBase, 'id'>): SvgShape => ({ ...style, id: shapeId(), kind: 'path', subs: parsePathData(d) });
