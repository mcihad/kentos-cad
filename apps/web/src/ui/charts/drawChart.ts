import type { ChartLayout, ChartPalette, ChartPrim, ChartRole } from '../../model/ops/charts';

/**
 * A chart's drawing list on a canvas (docs/adr/0227 §5): the core lays the chart out, this only paints its primitives
 * with the theme's colours (DESIGN.md §3; the chart tokens `--chart-1` … `--chart-8`, `--chart-other`), so the web and
 * the desktop draw the same chart. The hovered element is outlined in the first ink.
 */

/** The theme's colours of the roles, read from the CSS tokens. */
export interface RoleColors {
  surface: string;
  grid: string;
  axis: string;
  ink1: string;
  ink2: string;
  ink3: string;
  series: string[];
  other: string;
  selection: string;
}

/** The colours of the roles as the page's tokens give them now (the theme and the accent may change). */
export function roleColors(el: Element = document.documentElement): RoleColors {
  const css = getComputedStyle(el);
  const v = (name: string) => css.getPropertyValue(name).trim();
  return {
    surface: v('--c-panel'),
    grid: v('--c-line'),
    axis: v('--c-line-strong'),
    ink1: v('--c-text'),
    ink2: v('--c-text-2'),
    ink3: v('--c-text-3'),
    series: [1, 2, 3, 4, 5, 6, 7, 8].map((k) => v(`--chart-${k}`)),
    other: v('--chart-other'),
    selection: v('--c-accent'),
  };
}

export function roleColor(c: RoleColors, role: ChartRole): string {
  if (role.startsWith('series')) return c.series[Number(role.slice(6))] ?? c.ink2;
  return (c as unknown as Record<string, string>)[role] ?? c.ink2;
}

/** The light theme's colours on white paper (a saved picture, docs/adr/0227 §7). */
export const PAPER: Omit<ChartPalette, 'font'> = {
  background: '#ffffff',
  surface: '#ffffff',
  grid: '#d3d9df',
  axis: '#b5bfc9',
  ink1: '#1b232c',
  ink2: '#4d5966',
  ink3: '#7a8591',
  series: ['#2a78d6', '#eb6834', '#1baf7a', '#eda100', '#e87ba4', '#008300', '#4a3aa7', '#e34948'],
  other: '#9aa3ad',
  selection: '#1f4a96',
};

function roundRect(g: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, rt: number, rb: number): void {
  rt = Math.min(rt, w / 2, h);
  rb = Math.min(rb, w / 2, h);
  g.beginPath();
  g.moveTo(x, y + rt);
  if (rt > 0) g.arcTo(x, y, x + rt, y, rt);
  g.lineTo(x + w - rt, y);
  if (rt > 0) g.arcTo(x + w, y, x + w, y + rt, rt);
  g.lineTo(x + w, y + h - rb);
  if (rb > 0) g.arcTo(x + w, y + h, x + w - rb, y + h, rb);
  g.lineTo(x + rb, y + h);
  if (rb > 0) g.arcTo(x, y + h, x, y + h - rb, rb);
  g.closePath();
}

function wedge(g: CanvasRenderingContext2D, p: Extract<ChartPrim, { t: 'wedge' }>): void {
  // The core's angles turn clockwise from twelve o'clock; the canvas's from three o'clock.
  const a0 = p.a0 - Math.PI / 2;
  const a1 = p.a1 - Math.PI / 2;
  g.beginPath();
  g.arc(p.cx, p.cy, p.r1, a0, a1, false);
  g.arc(p.cx, p.cy, p.r0, a1, a0, true);
  g.closePath();
}

const FONT_STACK = (family: string) => `'${family}', system-ui, sans-serif`;

/**
 * Paints the drawing on a 2D context already scaled to CSS pixels. `hovered` (an element) is outlined; `points` are the
 * layout's bulk points.
 */
export function drawChart(g: CanvasRenderingContext2D, layout: ChartLayout, points: Float32Array, pointElems: Uint32Array, c: RoleColors, family: string, hovered = -1): void {
  g.lineCap = 'round';
  g.lineJoin = 'round';
  for (const p of layout.prims) {
    switch (p.t) {
      case 'rect':
        g.globalAlpha = p.a;
        g.fillStyle = roleColor(c, p.fill);
        roundRect(g, p.x, p.y, p.w, p.h, p.rt, p.rb);
        g.fill();
        break;
      case 'line':
        g.globalAlpha = p.a;
        g.strokeStyle = roleColor(c, p.stroke);
        g.lineWidth = p.w;
        g.beginPath();
        // A hairline sits on the pixel's middle so it stays one pixel wide.
        if (p.w <= 1 && (p.x1 === p.x2 || p.y1 === p.y2)) {
          const sx = p.x1 === p.x2 ? Math.round(p.x1) + 0.5 : p.x1;
          const sy = p.y1 === p.y2 ? Math.round(p.y1) + 0.5 : p.y1;
          g.moveTo(sx, sy);
          g.lineTo(p.x1 === p.x2 ? sx : p.x2, p.y1 === p.y2 ? sy : p.y2);
        } else {
          g.moveTo(p.x1, p.y1);
          g.lineTo(p.x2, p.y2);
        }
        g.stroke();
        break;
      case 'poly':
        g.globalAlpha = p.a;
        g.strokeStyle = roleColor(c, p.stroke);
        g.lineWidth = p.w;
        g.beginPath();
        for (let i = 0; i + 1 < p.pts.length; i += 2) i ? g.lineTo(p.pts[i], p.pts[i + 1]) : g.moveTo(p.pts[i], p.pts[i + 1]);
        if (p.closed) g.closePath();
        g.stroke();
        break;
      case 'circle':
        g.globalAlpha = p.a;
        g.beginPath();
        g.arc(p.x, p.y, p.r, 0, Math.PI * 2);
        if (p.fill) {
          g.fillStyle = roleColor(c, p.fill);
          g.fill();
        }
        if (p.stroke) {
          g.strokeStyle = roleColor(c, p.stroke);
          g.lineWidth = p.w;
          g.stroke();
        }
        break;
      case 'wedge':
        g.globalAlpha = p.a;
        g.fillStyle = roleColor(c, p.fill);
        wedge(g, p);
        g.fill();
        break;
      case 'cross':
        g.globalAlpha = 1;
        g.strokeStyle = roleColor(c, p.stroke);
        g.lineWidth = p.w;
        g.beginPath();
        g.moveTo(p.x - p.s, p.y - p.s);
        g.lineTo(p.x + p.s, p.y + p.s);
        g.moveTo(p.x - p.s, p.y + p.s);
        g.lineTo(p.x + p.s, p.y - p.s);
        g.stroke();
        break;
      case 'text':
        g.globalAlpha = 1;
        g.fillStyle = roleColor(c, p.color);
        g.font = `${p.weight} ${p.size}px ${FONT_STACK(family)}`;
        g.textAlign = p.align === 'start' ? 'left' : p.align === 'end' ? 'right' : 'center';
        g.textBaseline = p.baseline;
        g.fillText(p.text, p.x, p.y);
        break;
      case 'points': {
        const from = p.start * 2;
        const to = (p.start + p.count) * 2;
        g.globalAlpha = p.a;
        if (p.ring) {
          g.fillStyle = c.surface;
          g.beginPath();
          for (let i = from; i < to; i += 2) {
            g.moveTo(points[i] + p.r + 2, points[i + 1]);
            g.arc(points[i], points[i + 1], p.r + 2, 0, Math.PI * 2);
          }
          g.fill();
        }
        g.fillStyle = roleColor(c, p.fill);
        g.beginPath();
        for (let i = from; i < to; i += 2) {
          g.moveTo(points[i] + p.r, points[i + 1]);
          g.arc(points[i], points[i + 1], p.r, 0, Math.PI * 2);
        }
        g.fill();
        break;
      }
    }
  }
  g.globalAlpha = 1;
  if (hovered >= 0) outline(g, layout, points, pointElems, c, hovered);
}

/** The hovered element outlined in the first ink: its bars, slices and markers, or its point. */
function outline(g: CanvasRenderingContext2D, layout: ChartLayout, points: Float32Array, pointElems: Uint32Array, c: RoleColors, e: number): void {
  g.strokeStyle = c.ink1;
  g.lineWidth = 1.5;
  let found = false;
  for (const p of layout.prims) {
    if (p.t === 'rect' && p.e === e) roundRect(g, p.x, p.y, p.w, p.h, p.rt, p.rb);
    else if (p.t === 'wedge' && p.e === e) wedge(g, p);
    else if (p.t === 'circle' && p.e === e) {
      g.beginPath();
      g.arc(p.x, p.y, p.r + 2, 0, Math.PI * 2);
    } else continue;
    g.stroke();
    found = true;
  }
  if (found) return;
  // A scatter's point: the bulk points carry their elements in order.
  const k = pointElems.indexOf(e);
  if (k < 0) return;
  const r = layout.prims.find((p): p is Extract<ChartPrim, { t: 'points' }> => p.t === 'points' && k >= p.start && k < p.start + p.count)?.r ?? 4;
  g.beginPath();
  g.arc(points[k * 2], points[k * 2 + 1], r + 3, 0, Math.PI * 2);
  g.stroke();
}

/** Fonts and the frame's text size: the interface's `--fs-xs` and typeface. */
export function chartFont(el: Element = document.documentElement): { size: number; family: string } {
  const css = getComputedStyle(el);
  const size = parseFloat(css.getPropertyValue('--fs-xs')) || 12;
  const family = css.getPropertyValue('--font-ui').split(',')[0].trim().replace(/^['"]|['"]$/g, '') || 'Plus Jakarta Sans';
  return { size, family };
}
