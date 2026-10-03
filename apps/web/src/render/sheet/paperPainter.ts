import type { Guide } from '../../contracts/generated/sheet/Guide';
import type { MarginsMm, RectMm } from '../../product/sheet/view';
import { toScreen, type PaperViewport } from './paperView';

/**
 * The desk and what the workspace draws over a sheet (docs/sheet/design.md
 * §11): the desk in the theme's surface colour, the paper (white in both
 * themes) with its shadow, the margins, the snapping grid and the ruler
 * guides while designing, and over the sheet the item under the pointer,
 * the choice with its handles and the box being dragged out, all in the
 * theme's one accent (“bir vurgu, bir anlam”). The sheet itself is the
 * engine's plan, painted by painter.ts. Sharp on any screen: the canvas has
 * device pixels and every hairline falls on one.
 */

/** Colours of the workspace, from CSS tokens (ui/sheet/sheet.css): none is written in code. */
export interface PaperColors {
  desk: string;
  paper: string;
  paperEdge: string;
  shadow: string;
  margin: string;
  grid: string;
  accent: string;
  handleFill: string;
  /** A box dragged left to right (whole items) and right to left (touched items), as on the drawing (DESIGN.md §8). */
  window: string;
  crossing: string;
  /** The badges' ground and letters (distances, the angle). */
  badge: string;
  badgeText: string;
  font: string;
}

export function readPaperColors(el: Element): PaperColors {
  const cs = getComputedStyle(el);
  const v = (name: string, fallback: string) => cs.getPropertyValue(name).trim() || fallback;
  return {
    desk: v('--sheet-desk', '#181e26'),
    paper: v('--sheet-paper', '#ffffff'),
    paperEdge: v('--sheet-paper-edge', 'rgba(0,0,0,0.18)'),
    shadow: v('--sheet-shadow', 'rgba(0,0,0,0.45)'),
    margin: v('--sheet-margin', 'rgba(31,111,196,0.45)'),
    grid: v('--sheet-grid', 'rgba(31,111,196,0.18)'),
    accent: v('--canvas-accent', '#f2b632'),
    handleFill: v('--sheet-paper', '#ffffff'),
    window: v('--c-info', '#6db3f2'),
    crossing: v('--canvas-snap', '#6fd08c'),
    badge: v('--sheet-badge', '#f2b632'),
    badgeText: v('--sheet-badge-text', '#1b1300'),
    font: v('--font-ui', 'system-ui, sans-serif'),
  };
}

/** A chosen or hovered item's frame on the paper: millimetres and degrees. */
export interface FrameMm {
  readonly frame: RectMm;
  readonly rotation: number;
}

/** Handle size (CSS px): the SVG editor's 8 px squares (DESIGN.md §7.14). */
export const HANDLE = 8;
/** The round handle's distance above the top edge (CSS px); the select tool asks the engine with it. */
export const ROTATE_OFFSET = 18;

/** A position snapped to the middle of a device pixel, so a one-pixel line is sharp. */
export const crisp = (v: number, dpr: number) => (Math.round(v * dpr) + 0.5) / dpr;

export function hairRect(g: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, dpr: number): void {
  const x0 = crisp(x, dpr);
  const y0 = crisp(y, dpr);
  g.strokeRect(x0, y0, crisp(x + w, dpr) - x0, crisp(y + h, dpr) - y0);
}

/** The desk and the paper on it with its shadow and edge (CSS px; the canvas is scaled by `dpr`). */
export function paintDesk(g: CanvasRenderingContext2D, width: number, height: number, dpr: number, view: PaperViewport, paper: { width: number; height: number }, c: PaperColors): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  g.fillStyle = c.desk;
  g.fillRect(0, 0, width, height);
  const p = toScreen(view, { x: 0, y: 0 });
  const pw = paper.width * view.scale;
  const ph = paper.height * view.scale;
  // Blur and offset are device pixels, outside the transform.
  g.save();
  g.shadowColor = c.shadow;
  g.shadowBlur = 16 * dpr;
  g.shadowOffsetY = 3 * dpr;
  g.fillStyle = c.paper;
  g.fillRect(p.x, p.y, pw, ph);
  g.restore();
}

/** The paper's edge, over what is painted on it. */
export function paintPaperEdge(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, paper: { width: number; height: number }, c: PaperColors): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  const p = toScreen(view, { x: 0, y: 0 });
  g.strokeStyle = c.paperEdge;
  g.lineWidth = 1 / dpr;
  g.setLineDash([]);
  hairRect(g, p.x, p.y, paper.width * view.scale, paper.height * view.scale, dpr);
}

/** Margins (dashed), the snapping grid when shown, the ruler guides: design aids, never printed. */
export function paintAids(
  g: CanvasRenderingContext2D,
  dpr: number,
  view: PaperViewport,
  paper: { width: number; height: number; margins: MarginsMm },
  aids: { guides: readonly Guide[]; grid: number | null },
  c: PaperColors,
): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  const o = toScreen(view, { x: 0, y: 0 });
  const pw = paper.width * view.scale;
  const ph = paper.height * view.scale;
  g.save();
  g.beginPath();
  g.rect(o.x, o.y, pw, ph);
  g.clip();
  if (aids.grid && aids.grid * view.scale >= 6) {
    g.fillStyle = c.grid;
    const step = aids.grid * view.scale;
    for (let x = 0; x <= pw + 0.5; x += step) for (let y = 0; y <= ph + 0.5; y += step) g.fillRect(Math.round((o.x + x) * dpr) / dpr, Math.round((o.y + y) * dpr) / dpr, 1 / dpr * 1.5, 1 / dpr * 1.5);
  }
  const m = paper.margins;
  if (m.left + m.right + m.top + m.bottom > 0) {
    g.strokeStyle = c.margin;
    g.lineWidth = 1;
    g.setLineDash([4, 3]);
    hairRect(g, o.x + m.left * view.scale, o.y + m.top * view.scale, (paper.width - m.left - m.right) * view.scale, (paper.height - m.top - m.bottom) * view.scale, dpr);
  }
  g.strokeStyle = c.window;
  g.lineWidth = 1;
  g.setLineDash([]);
  for (const gd of aids.guides) {
    g.beginPath();
    if (gd.axis === 'x') {
      const x = crisp(o.x + (gd.at / 1000) * view.scale, dpr);
      g.moveTo(x, o.y);
      g.lineTo(x, o.y + ph);
    } else {
      const y = crisp(o.y + (gd.at / 1000) * view.scale, dpr);
      g.moveTo(o.x, y);
      g.lineTo(o.x + pw, y);
    }
    g.stroke();
  }
  g.restore();
}

type Pt = { x: number; y: number };

/** The eight handles of a box: corners, then edge middles (screen px), turned with it. */
function corners(f: FrameMm, view: PaperViewport): Pt[] {
  const a = toScreen(view, { x: f.frame.left, y: f.frame.top });
  const w = f.frame.width * view.scale;
  const h = f.frame.height * view.scale;
  const pts = [
    { x: a.x, y: a.y },
    { x: a.x + w, y: a.y },
    { x: a.x + w, y: a.y + h },
    { x: a.x, y: a.y + h },
    { x: a.x + w / 2, y: a.y },
    { x: a.x + w, y: a.y + h / 2 },
    { x: a.x + w / 2, y: a.y + h },
    { x: a.x, y: a.y + h / 2 },
  ];
  if (!f.rotation) return pts;
  const c = { x: a.x + w / 2, y: a.y + h / 2 };
  const t = (f.rotation * Math.PI) / 180;
  return pts.map((p) => turn(p, c, t));
}

const turn = (p: Pt, c: Pt, t: number): Pt => ({ x: c.x + (p.x - c.x) * Math.cos(t) - (p.y - c.y) * Math.sin(t), y: c.y + (p.x - c.x) * Math.sin(t) + (p.y - c.y) * Math.cos(t) });

/** A frame's outline, turned with it. */
function outline(g: CanvasRenderingContext2D, f: FrameMm, view: PaperViewport, color: string, dash: number[], dpr: number, alpha = 1): void {
  const a = toScreen(view, { x: f.frame.left, y: f.frame.top });
  const w = f.frame.width * view.scale;
  const h = f.frame.height * view.scale;
  g.save();
  g.globalAlpha = alpha;
  g.strokeStyle = color;
  g.setLineDash(dash);
  if (f.rotation) {
    const c = { x: a.x + w / 2, y: a.y + h / 2 };
    g.translate(c.x, c.y);
    g.rotate((f.rotation * Math.PI) / 180);
    g.lineWidth = 1.25;
    g.strokeRect(-w / 2, -h / 2, w, h);
  } else {
    g.lineWidth = 1;
    hairRect(g, a.x, a.y, w, h, dpr);
  }
  g.restore();
}

/** The union of frames (turns left aside), or null for none. */
export function boundsOf(frames: readonly FrameMm[]): RectMm | null {
  if (!frames.length) return null;
  let l = Infinity;
  let t = Infinity;
  let r = -Infinity;
  let b = -Infinity;
  for (const { frame: f } of frames) {
    l = Math.min(l, f.left);
    t = Math.min(t, f.top);
    r = Math.max(r, f.left + f.width);
    b = Math.max(b, f.top + f.height);
  }
  return { left: l, top: t, width: r - l, height: b - t };
}

/**
 * The item under the pointer (a thin line), the chosen ones (dashed for one,
 * their box for several), the handles (`handles`: one unlocked item gets
 * the round handle above it too).
 */
export function paintChoice(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, s: { hover: FrameMm | null; chosen: readonly FrameMm[]; handles: 'none' | 'resize' | 'rotate' }, c: PaperColors): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  if (s.hover) outline(g, s.hover, view, c.accent, [], dpr, 0.85);
  for (const f of s.chosen) outline(g, f, view, c.accent, s.chosen.length > 1 ? [] : [5, 3], dpr);
  const box = boundsOf(s.chosen);
  if (!box) return;
  if (s.chosen.length > 1) outline(g, { frame: box, rotation: 0 }, view, c.accent, [5, 3], dpr);
  if (s.handles === 'none') return;
  const single = s.chosen.length === 1 ? s.chosen[0] : { frame: box, rotation: 0 };
  const pts = corners(single, view);
  g.save();
  g.lineWidth = 1;
  g.setLineDash([]);
  g.strokeStyle = c.accent;
  g.fillStyle = c.handleFill;
  if (s.handles === 'rotate' && s.chosen.length === 1) {
    const top = pts[4];
    const f = single.frame;
    const centre = toScreen(view, { x: f.left + f.width / 2, y: f.top + f.height / 2 });
    const knob = turn({ x: centre.x, y: centre.y - (f.height * view.scale) / 2 - ROTATE_OFFSET }, centre, (single.rotation * Math.PI) / 180);
    g.beginPath();
    g.moveTo(top.x, top.y);
    g.lineTo(knob.x, knob.y);
    g.stroke();
    g.beginPath();
    g.arc(knob.x, knob.y, HANDLE / 2, 0, Math.PI * 2);
    g.fill();
    g.stroke();
  }
  for (const p of pts) {
    const x = Math.round((p.x - HANDLE / 2) * dpr) / dpr + 0.5 / dpr;
    const y = Math.round((p.y - HANDLE / 2) * dpr) / dpr + 0.5 / dpr;
    g.fillRect(x, y, HANDLE, HANDLE);
    g.strokeRect(x, y, HANDLE, HANDLE);
  }
  g.restore();
}

/** A box being dragged out to choose items (paper mm); dashed and green when right to left (it takes what it touches). */
export function paintMarquee(g: CanvasRenderingContext2D, dpr: number, view: PaperViewport, box: RectMm, crossing: boolean, c: PaperColors): void {
  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  const a = toScreen(view, { x: box.left, y: box.top });
  const color = crossing ? c.crossing : c.window;
  g.save();
  g.fillStyle = color;
  g.globalAlpha = 0.1;
  g.fillRect(a.x, a.y, box.width * view.scale, box.height * view.scale);
  g.globalAlpha = 1;
  g.strokeStyle = color;
  g.lineWidth = 1;
  g.setLineDash(crossing ? [5, 3] : []);
  hairRect(g, a.x, a.y, box.width * view.scale, box.height * view.scale, dpr);
  g.restore();
}
