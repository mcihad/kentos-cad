import type { Gen } from '../../../wasm/calls/harness';
import type { Cubic } from '../bezier';
import type { Matrix, PathNode, Pt, SubPath } from '../pathData';
import type { SvgShape } from '../svgModel';

/**
 * Test support for the SVG editor's geometry (docs/adr/0008 “SVG
 * düzenleyicisi”): random drawings as the editor holds them. Sub-paths
 * (lines and curves, open and closed, with repeated points, handles on
 * their nodes and grid-aligned coincidences), shapes of every kind with
 * every style field, matrices (moves, uniform and uneven scales, mirrors,
 * rotations, skews), cubics and path data in every command form. Drawings
 * stay the size of a symbol (0–100 units).
 */

/** A point of a symbol's drawing: free or on a coarse grid (coincidences). */
export function pt(g: Gen, at: Pt = [50, 50], spread = 50): Pt {
  if (g.chance(0.25)) return [at[0] + g.int(-4, 4) * (spread / 4), at[1] + g.int(-4, 4) * (spread / 4)];
  return [at[0] + g.num(-spread, spread), at[1] + g.num(-spread, spread)];
}

/** A handle near a node (or on it, which makes the segment's end direction come from the other control). */
function handle(g: Gen, p: Pt, spread: number): Pt {
  if (g.chance(0.08)) return [p[0], p[1]];
  return [p[0] + g.num(-spread, spread) * 0.6, p[1] + g.num(-spread, spread) * 0.6];
}

const TYPES = ['cusp', 'smooth', 'symmetric', 'auto'] as const;

/** A sub-path: 1–10 nodes, curved segments at random, sometimes a node repeated. */
export function subPath(g: Gen, at: Pt = [50, 50], spread = 40, closed = g.chance(0.6)): SubPath {
  const n = g.chance(0.05) ? 1 : g.int(2, g.chance(0.2) ? 10 : 5);
  const nodes: PathNode[] = [];
  for (let i = 0; i < n; i++) {
    const p = i > 0 && g.chance(0.06) ? ([nodes[i - 1].x, nodes[i - 1].y] as Pt) : pt(g, at, spread);
    const node: PathNode = { x: p[0], y: p[1] };
    if (g.chance(0.4)) node.in = handle(g, p, spread);
    if (g.chance(0.4)) node.out = handle(g, p, spread);
    if (g.chance(0.1)) node.type = g.pick(TYPES);
    nodes.push(node);
  }
  return { closed, nodes };
}

/** A convex-ish ring around a centre (a fill that booleans can overlap), lines or smooth curves. */
export function loop(g: Gen, at: Pt, r: number): SubPath {
  const n = g.int(3, 8);
  const curved = g.chance(0.5);
  const nodes: PathNode[] = [];
  for (let i = 0; i < n; i++) {
    const a = (2 * Math.PI * (i + g.num(0, 0.6))) / n;
    const d = r * g.num(0.5, 1);
    const x = at[0] + d * Math.cos(a);
    const y = at[1] + d * Math.sin(a);
    const node: PathNode = { x, y };
    if (curved) {
      const k = (d * 0.5) / n;
      node.in = [x + k * Math.sin(a), y - k * Math.cos(a)];
      node.out = [x - k * Math.sin(a), y + k * Math.cos(a)];
    }
    nodes.push(node);
  }
  return { closed: true, nodes: g.chance(0.5) ? nodes : nodes.reverse() };
}

/** Sub-paths of one shape: rings and free sub-paths, a hole now and then. */
export function subs(g: Gen, at: Pt = [50, 50], spread = 40): SubPath[] {
  const out: SubPath[] = [];
  const k = g.int(1, 3);
  for (let i = 0; i < k; i++) out.push(g.chance(0.5) ? loop(g, pt(g, at, spread / 3), spread * g.num(0.3, 1)) : subPath(g, at, spread));
  if (g.chance(0.25)) out.push(loop(g, at, spread * 0.25));
  return out;
}

const PAINTS = ['none', 'fill', 'stroke', '#AA3300', '#11223344'];

/** Style fields of a shape, each present or not. */
function style(g: Gen): Omit<SvgShape, 'kind' | 'id'> & Record<string, unknown> {
  const s: Record<string, unknown> = { fill: g.pick(PAINTS), stroke: g.pick(PAINTS), strokeWidth: g.pick([0, 0.5, 1, 2.5, 6, g.num(0.1, 8)]) };
  if (g.chance(0.3)) s.opacity = g.pick([1, 0.5, g.num(0, 1)]);
  if (g.chance(0.25)) s.dash = g.pick([[4, 2], [1], [3, 1, 0.5], [0, 2], [5, -1], [0, 0]]);
  if (g.chance(0.4)) s.cap = g.pick(['butt', 'round', 'square']);
  if (g.chance(0.4)) s.join = g.pick(['miter', 'round', 'bevel']);
  if (g.chance(0.4)) s.fillRule = g.pick(['nonzero', 'evenodd']);
  if (g.chance(0.3)) s.group = g.pick(['g1', 'g2', '']);
  if (g.chance(0.1)) s.hidden = g.chance(0.5);
  if (g.chance(0.1)) s.locked = true;
  if (g.chance(0.2)) s.name = g.pick(['Çerçeve', 'Işık', 'ağaç 2', '🌳']);
  return s as Omit<SvgShape, 'kind' | 'id'> & Record<string, unknown>;
}

/** A shape of any kind, with id `s<k>`. */
export function shape(g: Gen, k: number, at: Pt = [50, 50], spread = 40): SvgShape {
  const id = `s${k}`;
  const base = { id, ...style(g) };
  const kind = g.pick(['rect', 'rect', 'ellipse', 'ellipse', 'path', 'path', 'path', 'text']);
  const c = pt(g, at, spread / 2);
  if (kind === 'rect') {
    const r: Record<string, unknown> = { ...base, kind, x: c[0] - spread / 2, y: c[1] - spread / 3, w: spread * g.num(0.2, 1.2), h: spread * g.num(0.2, 1) };
    if (g.chance(0.4)) r.r = g.pick([0, 2, g.num(0, spread / 2), spread]);
    if (g.chance(0.4)) r.rotate = g.pick([0, 90, 45, g.num(-180, 180)]);
    return r as unknown as SvgShape;
  }
  if (kind === 'ellipse') {
    const e: Record<string, unknown> = { ...base, kind, cx: c[0], cy: c[1], rx: spread * g.num(0.1, 0.8), ry: spread * g.num(0.1, 0.8) };
    if (g.chance(0.4)) e.rotate = g.pick([0, 90, 30, g.num(-180, 180)]);
    return e as unknown as SvgShape;
  }
  if (kind === 'text') {
    const t: Record<string, unknown> = { ...base, kind, x: c[0], y: c[1], text: g.pick(['A', 'Parsel 12', 'Işık', '', 'ağaç ✓', '🌳 çam']), size: g.pick([3, 6, g.num(1, 12)]), weight: g.pick([400, 700, 900]), font: g.pick(['sans', 'serif']), anchor: g.pick(['start', 'middle', 'end']) };
    if (g.chance(0.4)) t.rotate = g.pick([0, 90, g.num(-180, 180)]);
    return t as unknown as SvgShape;
  }
  return { ...base, kind: 'path', subs: subs(g, c, spread) } as SvgShape;
}

/** Shapes overlapping around one place (booleans, cuts, combines). */
export function shapes(g: Gen, count = g.int(1, 4)): SvgShape[] {
  const at = pt(g, [50, 50], 20);
  return Array.from({ length: count }, (_, k) => shape(g, k, at, 30));
}

/** An affine map: identity, move, scales (uneven, mirrored), rotations (quarter turns and any), skews, anything. */
export function matrix(g: Gen): Matrix {
  const kind = g.int(0, 7);
  const dx = g.num(-50, 50);
  const dy = g.num(-50, 50);
  if (kind === 0) return [1, 0, 0, 1, 0, 0];
  if (kind === 1) return [1, 0, 0, 1, dx, dy];
  if (kind === 2) {
    const s = g.pick([2, 0.5, g.num(0.1, 3)]);
    return [s, 0, 0, s, dx, dy];
  }
  if (kind === 3) return [g.pick([1, -1, 2, 0.3]), 0, 0, g.pick([1, -1, 0.5, 3]), dx, dy];
  if (kind === 4 || kind === 5) {
    const a = g.pick([90, 180, -90, 45, g.num(-180, 180)]) * (Math.PI / 180);
    const s = kind === 5 ? g.num(0.2, 3) : 1;
    return [s * Math.cos(a), s * Math.sin(a), -s * Math.sin(a), s * Math.cos(a), dx, dy];
  }
  if (kind === 6) return [1, 0, g.num(-1, 1), 1, dx, dy];
  return [g.num(-2, 2), g.num(-2, 2), g.num(-2, 2), g.num(-2, 2), dx, dy];
}

/** A cubic: a curve, a straight one (controls on the ends), a loop or a point. */
export function cubic(g: Gen): Cubic {
  const a = pt(g);
  const d = pt(g);
  const k = g.int(0, 5);
  if (k === 0) return [a, a, d, d];
  if (k === 1) return [a, a, a, a];
  if (k === 2) return [a, [a[0], a[1]], pt(g), d];
  return [a, pt(g), pt(g), d];
}

/** A number as SVG writes it: integers, decimals, signs, leading dots and exponents. */
export const numberText = (g: Gen): string => {
  const v = g.pick([g.int(-20, 120), Number(g.num(-50, 150).toFixed(g.int(0, 4))), g.num(0, 1)]);
  const s = String(v);
  const r = g.int(0, 9);
  if (r === 0 && s.startsWith('0.')) return s.slice(1);
  if (r === 1 && s.startsWith('-0.')) return `-${s.slice(2)}`;
  if (r === 2) return `${v * 100}e-2`;
  if (r === 3 && v >= 0) return `+${s}`;
  return s;
};

/** Path data in every command form: relative and absolute, implicit repeats, compact arc flags, odd separators, a broken tail. */
export function pathData(g: Gen): string {
  const parts: string[] = [];
  const sep = () => g.pick([' ', ',', ', ', '  ', '\n', '']);
  const count = g.int(1, 12);
  parts.push(`${g.pick(['M', 'm'])}${numberText(g)}${sep()}${numberText(g)}`);
  for (let i = 0; i < count; i++) {
    const c = g.pick(['L', 'l', 'H', 'h', 'V', 'v', 'C', 'c', 'S', 's', 'Q', 'q', 'T', 't', 'A', 'a', 'Z', 'z', 'M', 'm']);
    const args =
      c.toUpperCase() === 'Z'
        ? 0
        : c.toUpperCase() === 'H' || c.toUpperCase() === 'V'
          ? 1
          : c.toUpperCase() === 'C'
            ? 6
            : c.toUpperCase() === 'S' || c.toUpperCase() === 'Q'
              ? 4
              : c.toUpperCase() === 'A'
                ? 7
                : 2;
    const repeat = args && g.chance(0.2) ? 2 : 1;
    let text = g.chance(0.1) && i > 0 ? '' : c;
    for (let r = 0; r < repeat; r++) {
      if (c.toUpperCase() === 'A') {
        const flags = g.chance(0.3) ? `${g.int(0, 1)}${g.int(0, 1)}` : `${g.int(0, 1)}${sep() || ' '}${g.int(0, 1)}`;
        text += `${sep()}${g.pick([0, g.num(1, 40).toFixed(2)])}${sep() || ' '}${g.num(1, 40).toFixed(2)}${sep() || ' '}${g.int(-90, 90)}${sep() || ' '}${flags}${sep() || ' '}${numberText(g)}${sep() || ' '}${numberText(g)}`;
      } else for (let k = 0; k < args; k++) text += `${k || r ? sep() || ' ' : sep()}${numberText(g)}`;
    }
    parts.push(text);
  }
  if (g.chance(0.1)) parts.push(g.pick(['L 3', 'C 1 2 3', 'x', 'A 5 5 0 1', '1e', '..5']));
  return parts.join(g.pick(['', ' ', '\n']));
}
