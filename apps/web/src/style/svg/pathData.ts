/**
 * SVG path data as editable nodes: every command (lines, cubic and
 * quadratic Béziers, smooth variants, elliptic arcs) becomes nodes with
 * optional cubic handles, in absolute coordinates. A segment from node i
 * to i+1 is a cubic with controls (i.out ?? i) and (i+1.in ?? i+1): both
 * missing makes it a straight line. Pure, for the SVG editor and imports.
 */

export type Pt = readonly [number, number];

export interface PathNode {
  x: number;
  y: number;
  /** Incoming handle (control point before this node), absolute. */
  in?: Pt;
  /** Outgoing handle (control point after this node), absolute. */
  out?: Pt;
  /**
   * How the node editor keeps its handles: cusp (free), smooth (in line),
   * symmetric (in line, equal), auto (from the neighbours). Absent: read
   * from the handles. Editing state only; the file does not keep it.
   */
  type?: 'cusp' | 'smooth' | 'symmetric' | 'auto';
}

export interface SubPath {
  nodes: PathNode[];
  closed: boolean;
}

/** An affine matrix as SVG writes it: x' = a·x + c·y + e, y' = b·x + d·y + f. */
export type Matrix = readonly [number, number, number, number, number, number];

export const IDENTITY: Matrix = [1, 0, 0, 1, 0, 0];

export const multiply = (m: Matrix, n: Matrix): Matrix => [
  m[0] * n[0] + m[2] * n[1],
  m[1] * n[0] + m[3] * n[1],
  m[0] * n[2] + m[2] * n[3],
  m[1] * n[2] + m[3] * n[3],
  m[0] * n[4] + m[2] * n[5] + m[4],
  m[1] * n[4] + m[3] * n[5] + m[5],
];

export const apply = (m: Matrix, x: number, y: number): [number, number] => [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];

// ── Parsing ────────────────────────────────────────────────────────────

const TOKEN = /([MmLlHhVvCcSsQqTtAaZz])|([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)/g;

/** Nodes of every subpath in `d`; unknown text is skipped, a broken tail ends the path. */
export function parsePathData(d: string): SubPath[] {
  // Tokens stay text until read: compact arc flags ("011") need their digits.
  const tokens: string[] = [];
  for (const m of d.matchAll(TOKEN)) tokens.push(m[1] ?? m[2]);
  const isCmd = (t: string | undefined) => !!t && /^[A-Za-z]$/.test(t);
  const subs: SubPath[] = [];
  let cur: SubPath | null = null;
  let x = 0;
  let y = 0;
  let startX = 0;
  let startY = 0;
  let cmd = '';
  let lastCubic: Pt | null = null;
  let lastQuad: Pt | null = null;
  let i = 0;
  const num = () => {
    const t = tokens[i];
    if (t === undefined || isCmd(t)) throw new Error('number expected');
    i++;
    return Number(t);
  };
  const flag = () => {
    const t = tokens[i];
    if (t === undefined || isCmd(t)) throw new Error('flag expected');
    if (t.length > 1 && (t[0] === '0' || t[0] === '1') && /^\d/.test(t.slice(1))) {
      tokens[i] = t.slice(1);
      return t[0] === '1';
    }
    i++;
    return Number(t) !== 0;
  };
  const ensure = () => {
    if (!cur) {
      cur = { nodes: [{ x, y }], closed: false };
      subs.push(cur);
    }
    return cur;
  };
  const lineTo = (nx: number, ny: number) => {
    ensure().nodes.push({ x: nx, y: ny });
    x = nx;
    y = ny;
  };
  const cubicTo = (c1x: number, c1y: number, c2x: number, c2y: number, nx: number, ny: number) => {
    const sp = ensure();
    const prev = sp.nodes[sp.nodes.length - 1];
    prev.out = [c1x, c1y];
    sp.nodes.push({ x: nx, y: ny, in: [c2x, c2y] });
    x = nx;
    y = ny;
  };
  try {
    while (i < tokens.length) {
      const t = tokens[i];
      if (isCmd(t)) {
        cmd = t;
        i++;
      } else if (!cmd) break;
      const rel = cmd === cmd.toLowerCase() && cmd !== 'z' ? true : false;
      const ox = rel ? x : 0;
      const oy = rel ? y : 0;
      let cubic: Pt | null = null;
      let quad: Pt | null = null;
      switch (cmd.toUpperCase()) {
        case 'M': {
          const nx = ox + num();
          const ny = oy + num();
          cur = { nodes: [{ x: nx, y: ny }], closed: false };
          subs.push(cur);
          x = startX = nx;
          y = startY = ny;
          // Pairs after a moveto are linetos.
          cmd = rel ? 'l' : 'L';
          break;
        }
        case 'L':
          lineTo(ox + num(), oy + num());
          break;
        case 'H':
          lineTo(ox + num(), y);
          break;
        case 'V':
          lineTo(x, oy + num());
          break;
        case 'C': {
          const c1x = ox + num();
          const c1y = oy + num();
          const c2x = ox + num();
          const c2y = oy + num();
          cubicTo(c1x, c1y, c2x, c2y, ox + num(), oy + num());
          cubic = [c2x, c2y];
          break;
        }
        case 'S': {
          const c1: Pt = lastCubic ? [2 * x - lastCubic[0], 2 * y - lastCubic[1]] : [x, y];
          const c2x = ox + num();
          const c2y = oy + num();
          cubicTo(c1[0], c1[1], c2x, c2y, ox + num(), oy + num());
          cubic = [c2x, c2y];
          break;
        }
        case 'Q': {
          const qx = ox + num();
          const qy = oy + num();
          const nx = ox + num();
          const ny = oy + num();
          cubicTo(x + (2 / 3) * (qx - x), y + (2 / 3) * (qy - y), nx + (2 / 3) * (qx - nx), ny + (2 / 3) * (qy - ny), nx, ny);
          quad = [qx, qy];
          break;
        }
        case 'T': {
          const q: Pt = lastQuad ? [2 * x - lastQuad[0], 2 * y - lastQuad[1]] : [x, y];
          const nx = ox + num();
          const ny = oy + num();
          cubicTo(x + (2 / 3) * (q[0] - x), y + (2 / 3) * (q[1] - y), nx + (2 / 3) * (q[0] - nx), ny + (2 / 3) * (q[1] - ny), nx, ny);
          quad = q;
          break;
        }
        case 'A': {
          const rx = num();
          const ry = num();
          const rot = num();
          const large = flag();
          const sweep = flag();
          const nx = ox + num();
          const ny = oy + num();
          for (const c of arcToCubics(x, y, rx, ry, rot, large, sweep, nx, ny)) cubicTo(c[0], c[1], c[2], c[3], c[4], c[5]);
          x = nx;
          y = ny;
          break;
        }
        case 'Z': {
          if (cur) {
            const sp: SubPath = cur;
            const first = sp.nodes[0];
            const last = sp.nodes[sp.nodes.length - 1];
            // A closing segment that ends on the start merges into it.
            if (sp.nodes.length > 1 && Math.abs(last.x - first.x) < 1e-9 && Math.abs(last.y - first.y) < 1e-9) {
              if (last.in) first.in = last.in;
              sp.nodes.pop();
            }
            sp.closed = true;
          }
          x = startX;
          y = startY;
          cur = null;
          cmd = '';
          break;
        }
        default:
          i++;
      }
      lastCubic = cubic;
      lastQuad = quad;
    }
  } catch {
    // A malformed tail: keep what was read.
  }
  return subs.filter((s) => s.nodes.length > 0);
}

/**
 * Cubic segments approximating an SVG arc (endpoint form, F.6.5 of the SVG
 * spec), each spanning at most 90°: [c1x, c1y, c2x, c2y, x, y] each.
 */
export function arcToCubics(x1: number, y1: number, rxIn: number, ryIn: number, rotDeg: number, large: boolean, sweep: boolean, x2: number, y2: number): number[][] {
  let rx = Math.abs(rxIn);
  let ry = Math.abs(ryIn);
  if ((x1 === x2 && y1 === y2) || rx === 0 || ry === 0) return [[x1, y1, x2, y2, x2, y2]];
  const phi = (rotDeg * Math.PI) / 180;
  const cos = Math.cos(phi);
  const sin = Math.sin(phi);
  const dx = (x1 - x2) / 2;
  const dy = (y1 - y2) / 2;
  const x1p = cos * dx + sin * dy;
  const y1p = -sin * dx + cos * dy;
  const lambda = (x1p * x1p) / (rx * rx) + (y1p * y1p) / (ry * ry);
  if (lambda > 1) {
    rx *= Math.sqrt(lambda);
    ry *= Math.sqrt(lambda);
  }
  const num = rx * rx * ry * ry - rx * rx * y1p * y1p - ry * ry * x1p * x1p;
  const den = rx * rx * y1p * y1p + ry * ry * x1p * x1p;
  const coef = (large !== sweep ? 1 : -1) * Math.sqrt(Math.max(0, num / den));
  const cxp = (coef * rx * y1p) / ry;
  const cyp = (-coef * ry * x1p) / rx;
  const cx = cos * cxp - sin * cyp + (x1 + x2) / 2;
  const cy = sin * cxp + cos * cyp + (y1 + y2) / 2;
  const angle = (ux: number, uy: number, vx: number, vy: number) => {
    const a = Math.atan2(ux * vy - uy * vx, ux * vx + uy * vy);
    return a;
  };
  const t1 = angle(1, 0, (x1p - cxp) / rx, (y1p - cyp) / ry);
  let dt = angle((x1p - cxp) / rx, (y1p - cyp) / ry, (-x1p - cxp) / rx, (-y1p - cyp) / ry);
  if (!sweep && dt > 0) dt -= 2 * Math.PI;
  else if (sweep && dt < 0) dt += 2 * Math.PI;
  const n = Math.max(1, Math.ceil(Math.abs(dt) / (Math.PI / 2) - 1e-9));
  const step = dt / n;
  const k = (4 / 3) * Math.tan(step / 4);
  const point = (t: number): [number, number] => [cx + rx * Math.cos(t) * cos - ry * Math.sin(t) * sin, cy + rx * Math.cos(t) * sin + ry * Math.sin(t) * cos];
  const deriv = (t: number): [number, number] => [-rx * Math.sin(t) * cos - ry * Math.cos(t) * sin, -rx * Math.sin(t) * sin + ry * Math.cos(t) * cos];
  const out: number[][] = [];
  for (let s = 0; s < n; s++) {
    const a = t1 + s * step;
    const b = a + step;
    const p0 = point(a);
    const p3 = s === n - 1 ? [x2, y2] : point(b);
    const d0 = deriv(a);
    const d3 = deriv(b);
    out.push([p0[0] + k * d0[0], p0[1] + k * d0[1], p3[0] - k * d3[0], p3[1] - k * d3[1], p3[0], p3[1]]);
  }
  return out;
}

// ── Writing ────────────────────────────────────────────────────────────

const f = (v: number, digits: number) => {
  const s = (Math.round(v * 10 ** digits) / 10 ** digits).toString();
  return s === '-0' ? '0' : s;
};

/** Path data of subpaths, absolute, lines as L and curves as C. */
export function pathDataOf(subs: readonly SubPath[], digits = 3): string {
  const parts: string[] = [];
  for (const sp of subs) {
    const n = sp.nodes.length;
    if (!n) continue;
    const p = (x: number, y: number) => `${f(x, digits)} ${f(y, digits)}`;
    parts.push(`M${p(sp.nodes[0].x, sp.nodes[0].y)}`);
    const seg = (a: PathNode, b: PathNode) => (a.out || b.in ? `C${p(...(a.out ?? [a.x, a.y]))} ${p(...(b.in ?? [b.x, b.y]))} ${p(b.x, b.y)}` : `L${p(b.x, b.y)}`);
    for (let i = 1; i < n; i++) parts.push(seg(sp.nodes[i - 1], sp.nodes[i]));
    if (sp.closed) {
      const last = sp.nodes[n - 1];
      const first = sp.nodes[0];
      if (n > 1 && (last.out || first.in)) parts.push(seg(last, first));
      parts.push('Z');
    }
  }
  return parts.join('');
}

// ── Geometry ───────────────────────────────────────────────────────────

export function transformSubPaths(subs: readonly SubPath[], m: Matrix): SubPath[] {
  const t = (p: Pt): Pt => apply(m, p[0], p[1]);
  return subs.map((sp) => ({
    closed: sp.closed,
    nodes: sp.nodes.map((n) => {
      const [x, y] = apply(m, n.x, n.y);
      return { x, y, ...(n.in ? { in: t(n.in) } : {}), ...(n.out ? { out: t(n.out) } : {}), ...(n.type ? { type: n.type } : {}) };
    }),
  }));
}

export interface Box {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

export const emptyBox = (): Box => ({ minX: Infinity, minY: Infinity, maxX: -Infinity, maxY: -Infinity });
export const growBox = (b: Box, x: number, y: number) => {
  if (x < b.minX) b.minX = x;
  if (y < b.minY) b.minY = y;
  if (x > b.maxX) b.maxX = x;
  if (y > b.maxY) b.maxY = y;
};

/** Points along every segment (curves sampled), for bounds and hit tests. */
export function flattenSubPath(sp: SubPath, steps = 16): [number, number][] {
  const out: [number, number][] = [];
  const n = sp.nodes.length;
  if (!n) return out;
  out.push([sp.nodes[0].x, sp.nodes[0].y]);
  const segs = sp.closed ? n : n - 1;
  for (let i = 0; i < segs; i++) {
    const a = sp.nodes[i];
    const b = sp.nodes[(i + 1) % n];
    if (!a.out && !b.in) {
      out.push([b.x, b.y]);
      continue;
    }
    const c1 = a.out ?? [a.x, a.y];
    const c2 = b.in ?? [b.x, b.y];
    for (let s = 1; s <= steps; s++) {
      const t = s / steps;
      const u = 1 - t;
      out.push([u * u * u * a.x + 3 * u * u * t * c1[0] + 3 * u * t * t * c2[0] + t * t * t * b.x, u * u * u * a.y + 3 * u * u * t * c1[1] + 3 * u * t * t * c2[1] + t * t * t * b.y]);
    }
  }
  return out;
}

export function subPathsBox(subs: readonly SubPath[]): Box {
  const b = emptyBox();
  for (const sp of subs) for (const [x, y] of flattenSubPath(sp)) growBox(b, x, y);
  return b;
}
