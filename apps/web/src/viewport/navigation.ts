import type { Bounds } from '../model/geometry';

/**
 * Genel bakış and Büyüteç (docs/adr/0181 §3, §4): where their cards stand in
 * the drawing area, the side the magnifier takes as the pointer nears it,
 * and the overview's fit — the drawing's extent in its picture, the view's
 * frame on it, a press turned into the view's new centre. The desktop keeps
 * the same rules (`kentos_geometry_core::tools::navigation`); both play
 * `fixtures/navigation/v1/cases.json` (scripts/fixtures/navigation_cases.py).
 */

/** From the drawing area's edges, CSS px. */
export const MARGIN = 8;
/** Between the two cards. */
export const GAP = 8;
/** How near the pointer comes before the magnifier moves to the other side. */
export const NEAR = 16;
/** Room under a CBS project's grid north at the top right. */
export const NORTH = 52;
/** The overview's picture, CSS px. */
export const OVERVIEW = { width: 240, height: 160 } as const;
/** The magnifier's picture. */
export const LENS = { width: 220, height: 220 } as const;
/** The overview's margin inside its picture. */
export const PAD = 6;
/** The magnifier's steps. */
export const ZOOMS = [2, 4, 8, 16] as const;
export type Zoom = (typeof ZOOMS)[number];

/** x, y, width, height. */
export type Rect = readonly [number, number, number, number];

/** A card and its content (below its title row, inside its 1 px frame). */
export interface Card {
  readonly card: Rect;
  readonly content: Rect;
}

export type Side = 'right' | 'left';

function card(x: number, y: number, content: { width: number; height: number }, header: number): Card {
  return { card: [x, y, content.width + 2, header + content.height + 2], content: [x + 1, y + 1 + header, content.width, content.height] };
}

export interface Cards {
  readonly overview: Card | null;
  readonly right: Card;
  readonly left: Card;
}

/**
 * Where the cards stand (§4): the overview at the top left; the magnifier at the top right (under a CBS project's grid
 * north), or at the left under the overview, beside it when the area is too low.
 */
export function cards(area: { width: number; height: number }, header: number, gis: boolean, overview: boolean): Cards {
  const lens = { width: LENS.width + 2, height: header + LENS.height + 2 };
  const right = card(area.width - MARGIN - lens.width, MARGIN + (gis ? NORTH : 0), LENS, header);
  let left: Card;
  if (overview) {
    const below = MARGIN + header + OVERVIEW.height + 2 + GAP;
    left = below + lens.height <= area.height - MARGIN ? card(MARGIN, below, LENS, header) : card(MARGIN + OVERVIEW.width + 2 + GAP, MARGIN, LENS, header);
  } else left = card(MARGIN, MARGIN, LENS, header);
  return { overview: overview ? card(MARGIN, MARGIN, OVERVIEW, header) : null, right, left };
}

const near = (r: Rect, p: { x: number; y: number }) => r[0] - NEAR <= p.x && p.x <= r[0] + r[2] + NEAR && r[1] - NEAR <= p.y && p.y <= r[1] + r[3] + NEAR;

/** The magnifier's side once the pointer is at `pointer`: the other side when the pointer nears its card, unless it is as near the other side's. */
export function nextSide(laid: Cards, side: Side, pointer: { x: number; y: number }): Side {
  const other: Side = side === 'right' ? 'left' : 'right';
  return near(laid[side].card, pointer) && !near(laid[other].card, pointer) ? other : side;
}

/** The extent in the overview's picture: its centre on the picture's, `k` pixels per metre. */
export interface Fit {
  readonly cx: number;
  readonly cy: number;
  readonly k: number;
}

/** The extent fitted in a picture of `size` CSS px, PAD inside its edges; an extent under 1 m wide or high counts as 1 m about its centre. */
export function fit(extent: Bounds, size: { width: number; height: number }): Fit {
  const cx = (extent.minX + extent.maxX) / 2;
  const cy = (extent.minY + extent.maxY) / 2;
  const w = Math.max(extent.maxX - extent.minX, 1);
  const h = Math.max(extent.maxY - extent.minY, 1);
  return { cx, cy, k: Math.min((size.width - 2 * PAD) / w, (size.height - 2 * PAD) / h) };
}

/** A drawing point in the picture, CSS px from its top left. */
export function toCard(f: Fit, size: { width: number; height: number }, x: number, y: number): { u: number; v: number } {
  return { u: size.width / 2 + (x - f.cx) * f.k, v: size.height / 2 - (y - f.cy) * f.k };
}

/** A point of the picture in the drawing. */
export function toWorld(f: Fit, size: { width: number; height: number }, u: number, v: number): { x: number; y: number } {
  return { x: f.cx + (u - size.width / 2) / f.k, y: f.cy - (v - size.height / 2) / f.k };
}

/**
 * The view's frame in the picture (u0, v0, u1, v1) for a view centred on `center` at `metresPerPixel` over `viewPx`,
 * and whether it is small enough to be drawn as a cross (under 6 px both ways).
 */
export function viewFrame(
  f: Fit,
  size: { width: number; height: number },
  center: { x: number; y: number },
  metresPerPixel: number,
  viewPx: { width: number; height: number },
): { frame: [number, number, number, number]; cross: boolean } {
  const hw = (viewPx.width * metresPerPixel) / 2;
  const hh = (viewPx.height * metresPerPixel) / 2;
  const a = toCard(f, size, center.x - hw, center.y + hh);
  const b = toCard(f, size, center.x + hw, center.y - hh);
  return { frame: [a.u, a.v, b.u, b.v], cross: b.u - a.u < 6 && b.v - a.v < 6 };
}
