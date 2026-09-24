import { NO_SHAPE_PARAMS, type ShapeParams } from '../style/primitives';
import type { MarkerLook, RGBA, ShapeId } from './types';

/**
 * Marker shapes for Canvas2D, matching the shaders' distance fields: the
 * atlas draws pattern tiles with them, and style previews and legends use
 * the same code. Coordinates: centre (0, 0), y up, half sizes hw/hh.
 */

/** Shapes drawn as lines only (their "fill" colour, if any, strokes them). */
export const OPEN_SHAPES: ReadonlySet<ShapeId> = new Set(['cross', 'x', 'line', 'arrow', 'chevron', 'arc']);

function polygon(n: number, r: number, rot = Math.PI / 2): [number, number][] {
  return Array.from({ length: n }, (_, i) => [Math.cos(rot + (i * 2 * Math.PI) / n) * r, Math.sin(rot + (i * 2 * Math.PI) / n) * r]);
}

/**
 * Gear outline (as the shaders' field): a body circle with square teeth,
 * one centred on +x; each tooth is half a pitch wide at mid depth.
 */
function gearPath(p: Path2D, r: number, teeth: number, depth: number): void {
  const n = Math.max(3, Math.round(teeth));
  const rb = r * (1 - depth);
  const a = (2 * Math.PI) / n;
  const hw = 0.25 * a * (rb + (r - rb) / 2);
  const foot = Math.sqrt(Math.max(rb * rb - hw * hw, 0));
  const side = Math.atan2(hw, foot);
  const rot = (k: number, x: number, y: number): [number, number] => [x * Math.cos(k * a) - y * Math.sin(k * a), x * Math.sin(k * a) + y * Math.cos(k * a)];
  for (let k = 0; k < n; k++) {
    const pts = [rot(k, foot, -hw), rot(k, r, -hw), rot(k, r, hw), rot(k, foot, hw)];
    pts.forEach(([x, y], i) => (k === 0 && i === 0 ? p.moveTo(x, y) : p.lineTo(x, y)));
    p.arc(0, 0, rb, k * a + side, (k + 1) * a - side);
  }
  p.closePath();
}

/** Outline of a shape as a Path2D (y up: callers flip the context); `params` as the shaders read them. */
export function shapePath(shape: ShapeId, hw: number, hh: number, params: ShapeParams = NO_SHAPE_PARAMS): Path2D {
  const r = Math.min(hw, hh);
  const p = new Path2D();
  const poly = (pts: [number, number][]) => {
    pts.forEach(([x, y], i) => (i ? p.lineTo(x, y) : p.moveTo(x, y)));
    p.closePath();
  };
  switch (shape) {
    case 'circle':
    case 'ring':
      p.arc(0, 0, r, 0, Math.PI * 2);
      break;
    case 'square':
      p.rect(-r, -r, 2 * r, 2 * r);
      break;
    case 'rectangle':
      p.rect(-hw, -hh, 2 * hw, 2 * hh);
      break;
    case 'diamond':
      poly([
        [0, hh],
        [hw, 0],
        [0, -hh],
        [-hw, 0],
      ]);
      break;
    case 'triangle': {
      // Equilateral, side = width, centred on its centroid (as the shader's field).
      const s = 2 * r;
      const h = (s * Math.sqrt(3)) / 2;
      poly([
        [0, (2 * h) / 3],
        [s / 2, -h / 3],
        [-s / 2, -h / 3],
      ]);
      break;
    }
    case 'pentagon':
      poly(polygon(5, r / Math.cos(Math.PI / 5)));
      break;
    case 'hexagon':
      poly(polygon(6, r / Math.cos(Math.PI / 6))); // pointed top, flat sides
      break;
    case 'octagon':
      poly(polygon(8, r / Math.cos(Math.PI / 8), Math.PI / 8));
      break;
    case 'star': {
      const pts: [number, number][] = [];
      for (let i = 0; i < 10; i++) {
        const a = Math.PI / 2 + (i * Math.PI) / 5;
        const rr = i % 2 ? r * 0.4 : r;
        pts.push([Math.cos(a) * rr, Math.sin(a) * rr]);
      }
      poly(pts);
      break;
    }
    case 'semicircle':
      p.arc(0, 0, r, 0, Math.PI);
      p.closePath();
      break;
    case 'quartercircle':
      p.moveTo(0, 0);
      p.arc(0, 0, r, 0, Math.PI / 2);
      p.closePath();
      break;
    case 'arrowhead':
      poly([
        [hw, 0],
        [-hw, hh * 0.8],
        [-hw, -hh * 0.8],
      ]);
      break;
    case 'chevron':
      // Open V pointing along +x (the two sides of an arrowhead without its base).
      p.moveTo(-hw, hh);
      p.lineTo(hw, 0);
      p.lineTo(-hw, -hh);
      break;
    case 'cross':
      p.moveTo(-r, 0);
      p.lineTo(r, 0);
      p.moveTo(0, -r);
      p.lineTo(0, r);
      break;
    case 'x': {
      const d = r * Math.SQRT1_2;
      p.moveTo(-d, -d);
      p.lineTo(d, d);
      p.moveTo(-d, d);
      p.lineTo(d, -d);
      break;
    }
    case 'line':
      p.moveTo(-hw, 0);
      p.lineTo(hw, 0);
      break;
    case 'arrow':
      p.moveTo(-hw, 0);
      p.lineTo(hw, 0);
      p.moveTo(hw - hw * 0.5, hh * 0.4);
      p.lineTo(hw, 0);
      p.lineTo(hw - hw * 0.5, -hh * 0.4);
      break;
    case 'gear':
      gearPath(p, r, params[1], params[3]);
      break;
    case 'arc': {
      // Open arc centred on the top: from the right end over the top to the left end.
      const half = Math.min(Math.PI, params[2] / 2);
      p.arc(0, 0, r, Math.PI / 2 - half, Math.PI / 2 + half);
      break;
    }
  }
  // A round hole in a closed shape; the even-odd fill leaves it empty and the stroke rings it.
  if (params[0] > 0 && !OPEN_SHAPES.has(shape)) {
    const hr = params[0] * r;
    p.moveTo(hr, 0);
    p.arc(0, 0, hr, 0, 2 * Math.PI);
    p.closePath();
  }
  return p;
}

export const cssColor = (c: RGBA) => `rgba(${Math.round(c[0] * 255)},${Math.round(c[1] * 255)},${Math.round(c[2] * 255)},${c[3]})`;

/**
 * Draws a shape look with its centre at the context origin; `w`, `h` and
 * the stroke width are in context units. The context's y axis must point
 * up (scale(1, -1)) for triangles and arrows to face the right way.
 */
export function drawShape(g: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D, look: Extract<MarkerLook, { kind: 'shape' }>, w: number, h: number, strokeWidth: number): void {
  const path = shapePath(look.shape, w / 2, (h || w) / 2, look.params);
  const open = OPEN_SHAPES.has(look.shape);
  const strokeColor = look.stroke ?? (open ? look.fill : null);
  if (!open && look.fill) {
    g.fillStyle = cssColor(look.fill);
    g.fill(path, 'evenodd');
  }
  if (strokeColor) {
    g.strokeStyle = cssColor(strokeColor);
    g.lineWidth = Math.max(strokeWidth, 0.0001);
    g.lineJoin = 'miter';
    g.stroke(path);
  }
  if (look.shape === 'ring' && strokeColor) {
    g.beginPath();
    g.arc(0, 0, Math.max(strokeWidth, Math.min(w, h || w) * 0.08), 0, Math.PI * 2);
    g.fillStyle = cssColor(strokeColor);
    g.fill();
  }
}
