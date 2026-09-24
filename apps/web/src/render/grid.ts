import type { Bounds, Vec2 } from '../model/geometry';
import type { CanvasPalette } from './color';
import type { SceneLayer } from './types';

/** Picks a 1-2-5 spacing so minor lines stay ≥ `minPx` apart on screen. */
export function gridSpacing(scale: number, minPx = 14): { minor: number; major: number } {
  const target = minPx / scale;
  const p = Math.pow(10, Math.floor(Math.log10(target)));
  const m = [1, 2, 5, 10].find((k) => k * p >= target) ?? 10;
  const minor = m * p;
  // 1 → 5, 2 → 10, 5 → 20: majors always land on round values.
  return { minor, major: minor * (m === 5 ? 4 : 5) };
}

/**
 * What a grid on the GPU was built for: the world box its lines span, their
 * spacing, the origin they are relative to and the colours.
 */
export interface GridExtent {
  readonly box: Bounds;
  readonly spacing: { readonly minor: number; readonly major: number };
  readonly origin: Vec2;
  readonly palette: CanvasPalette;
}

/**
 * The grid a view (absolute world box) needs: `built` again while it still
 * covers the view with this zoom's spacing, origin and colours, so panning
 * and zooming inside it upload nothing; else a new extent three view sizes
 * around the view.
 */
export function gridExtent(view: Bounds, scale: number, origin: Vec2, palette: CanvasPalette, built: GridExtent | null): GridExtent {
  const spacing = gridSpacing(scale);
  if (
    built &&
    built.spacing.minor === spacing.minor &&
    built.origin.x === origin.x &&
    built.origin.y === origin.y &&
    built.palette === palette &&
    view.minX >= built.box.minX &&
    view.minY >= built.box.minY &&
    view.maxX <= built.box.maxX &&
    view.maxY <= built.box.maxY
  )
    return built;
  const w = view.maxX - view.minX;
  const h = view.maxY - view.minY;
  return { box: { minX: view.minX - w, minY: view.minY - h, maxX: view.maxX + w, maxY: view.maxY + h }, spacing, origin: { x: origin.x, y: origin.y }, palette };
}

/**
 * Grid lines across the extent's box. World-aligned in absolute coordinates
 * (so grid lines fall on round TM values), emitted relative to the origin.
 */
export function buildGrid(extent: GridExtent): SceneLayer {
  const { box, origin, palette } = extent;
  const { minor, major } = extent.spacing;
  const x0 = Math.floor(box.minX / minor) * minor;
  const y0 = Math.floor(box.minY / minor) * minor;
  const minorPos: number[] = [];
  const majorPos: number[] = [];
  const isMajor = (v: number) => Math.abs(v / major - Math.round(v / major)) < 1e-6;
  const ly0 = box.minY - origin.y;
  const ly1 = box.maxY - origin.y;
  const lx0 = box.minX - origin.x;
  const lx1 = box.maxX - origin.x;
  for (let i = 0, x = x0; x <= box.maxX; x = x0 + ++i * minor) (isMajor(x) ? majorPos : minorPos).push(x - origin.x, ly0, x - origin.x, ly1);
  for (let i = 0, y = y0; y <= box.maxY; y = y0 + ++i * minor) (isMajor(y) ? majorPos : minorPos).push(lx0, y - origin.y, lx1, y - origin.y);
  const batch = (pos: number[], color: typeof palette.gridMinor) => ({
    positions: new Float32Array(pos),
    distances: new Float32Array(pos.length / 2),
    color,
    dash: null,
  });
  return { id: '__grid', lines: [batch(minorPos, palette.gridMinor), batch(majorPos, palette.gridMajor)], fills: [], points: [] };
}
