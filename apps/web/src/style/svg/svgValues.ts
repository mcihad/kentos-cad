import { IDENTITY, multiply, type Matrix } from './pathData';
import type { Paint } from './svgModel';

/**
 * The values of an SVG file as the importer reads them: colours (every
 * CSS syntax and keyword, with alpha), paints (the symbol's colours,
 * references), transforms, lengths with units, viewBox mapping with
 * preserveAspectRatio, and `<style>` sheets with the selectors icon files
 * use (class, id, tag, attribute, descendant and child). Pure.
 */

export interface XmlNode {
  tag: string;
  attrs: Record<string, string>;
  children: XmlNode[];
  /** Text content: of a text element (without children), of a `#text` node, of a `<style>`. */
  text?: string;
}

// ── Colours ────────────────────────────────────────────────────────────

/** CSS colour keywords (CSS Color 4), name then hex. */
const NAMED_LIST =
  'aliceblue f0f8ff antiquewhite faebd7 aqua 00ffff aquamarine 7fffd4 azure f0ffff beige f5f5dc bisque ffe4c4 black 000000 blanchedalmond ffebcd blue 0000ff blueviolet 8a2be2 brown a52a2a burlywood deb887 cadetblue 5f9ea0 chartreuse 7fff00 chocolate d2691e coral ff7f50 cornflowerblue 6495ed cornsilk fff8dc crimson dc143c cyan 00ffff darkblue 00008b darkcyan 008b8b darkgoldenrod b8860b darkgray a9a9a9 darkgreen 006400 darkgrey a9a9a9 darkkhaki bdb76b darkmagenta 8b008b darkolivegreen 556b2f darkorange ff8c00 darkorchid 9932cc darkred 8b0000 darksalmon e9967a darkseagreen 8fbc8f darkslateblue 483d8b darkslategray 2f4f4f darkslategrey 2f4f4f darkturquoise 00ced1 darkviolet 9400d3 deeppink ff1493 deepskyblue 00bfff dimgray 696969 dimgrey 696969 dodgerblue 1e90ff firebrick b22222 floralwhite fffaf0 forestgreen 228b22 fuchsia ff00ff gainsboro dcdcdc ghostwhite f8f8ff gold ffd700 goldenrod daa520 gray 808080 green 008000 greenyellow adff2f grey 808080 honeydew f0fff0 hotpink ff69b4 indianred cd5c5c indigo 4b0082 ivory fffff0 khaki f0e68c lavender e6e6fa lavenderblush fff0f5 lawngreen 7cfc00 lemonchiffon fffacd lightblue add8e6 lightcoral f08080 lightcyan e0ffff lightgoldenrodyellow fafad2 lightgray d3d3d3 lightgreen 90ee90 lightgrey d3d3d3 lightpink ffb6c1 lightsalmon ffa07a lightseagreen 20b2aa lightskyblue 87cefa lightslategray 778899 lightslategrey 778899 lightsteelblue b0c4de lightyellow ffffe0 lime 00ff00 limegreen 32cd32 linen faf0e6 magenta ff00ff maroon 800000 mediumaquamarine 66cdaa mediumblue 0000cd mediumorchid ba55d3 mediumpurple 9370db mediumseagreen 3cb371 mediumslateblue 7b68ee mediumspringgreen 00fa9a mediumturquoise 48d1cc mediumvioletred c71585 midnightblue 191970 mintcream f5fffa mistyrose ffe4e1 moccasin ffe4b5 navajowhite ffdead navy 000080 oldlace fdf5e6 olive 808000 olivedrab 6b8e23 orange ffa500 orangered ff4500 orchid da70d6 palegoldenrod eee8aa palegreen 98fb98 paleturquoise afeeee palevioletred db7093 papayawhip ffefd5 peachpuff ffdab9 peru cd853f pink ffc0cb plum dda0dd powderblue b0e0e6 purple 800080 rebeccapurple 663399 red ff0000 rosybrown bc8f8f royalblue 4169e1 saddlebrown 8b4513 salmon fa8072 sandybrown f4a460 seagreen 2e8b57 seashell fff5ee sienna a0522d silver c0c0c0 skyblue 87ceeb slateblue 6a5acd slategray 708090 slategrey 708090 snow fffafa springgreen 00ff7f steelblue 4682b4 tan d2b48c teal 008080 thistle d8bfd8 tomato ff6347 turquoise 40e0d0 violet ee82ee wheat f5deb3 white ffffff whitesmoke f5f5f5 yellow ffff00 yellowgreen 9acd32';
const NAMED = new Map<string, string>();
{
  const parts = NAMED_LIST.split(' ');
  for (let i = 0; i + 1 < parts.length; i += 2) NAMED.set(parts[i], `#${parts[i + 1].toUpperCase()}`);
}

/** A paint as written, before references are followed. */
export type RawPaint = { kind: 'none' } | { kind: 'fill' } | { kind: 'stroke' } | { kind: 'color'; hex: string; alpha: number } | { kind: 'url'; id: string; fallback?: RawPaint };
/** A paint once gradients and patterns are flattened. */
export type Flat = { kind: 'none' } | { kind: 'fill' } | { kind: 'stroke' } | { kind: 'color'; hex: string; alpha: number };

export const hex2 = (v: number) => Math.round(Math.max(0, Math.min(255, v))).toString(16).padStart(2, '0').toUpperCase();
export const clamp01 = (v: number) => Math.max(0, Math.min(1, v));
const NUM = /[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g;
export const nums = (v: string | undefined): number[] => (v?.match(NUM) ?? []).map(Number);

function hsl(h: number, s: number, l: number): [number, number, number] {
  const k = (n: number) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n: number) => l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1));
  return [f(0) * 255, f(8) * 255, f(4) * 255];
}

/** A CSS colour as hex (#RRGGBB) and alpha; undefined when it is not a colour. */
export function readColor(v: string): { hex: string; alpha: number } | undefined {
  const s = v.trim().toLowerCase();
  let m: RegExpExecArray | null;
  if ((m = /^#([0-9a-f]{3,4})$/.exec(s))) {
    const d = m[1];
    return { hex: `#${d[0]}${d[0]}${d[1]}${d[1]}${d[2]}${d[2]}`.toUpperCase(), alpha: d.length === 4 ? parseInt(d[3] + d[3], 16) / 255 : 1 };
  }
  if ((m = /^#([0-9a-f]{6})([0-9a-f]{2})?$/.exec(s))) return { hex: `#${m[1].toUpperCase()}`, alpha: m[2] ? parseInt(m[2], 16) / 255 : 1 };
  const named = NAMED.get(s);
  if (named) return { hex: named, alpha: 1 };
  if ((m = /^(rgba?|hsla?)\(([^)]*)\)$/.exec(s))) {
    const parts = m[2].split(/[\s,/]+/).filter(Boolean);
    if (parts.length < 3) return undefined;
    const pct = (t: string, full: number) => (t.endsWith('%') ? (parseFloat(t) * full) / 100 : parseFloat(t));
    const alpha = parts[3] === undefined ? 1 : clamp01(pct(parts[3], 1));
    let rgb: [number, number, number];
    if (m[1].startsWith('rgb')) rgb = [pct(parts[0], 255), pct(parts[1], 255), pct(parts[2], 255)];
    else rgb = hsl(((parseFloat(parts[0]) % 360) + 360) % 360, clamp01(parseFloat(parts[1]) / 100), clamp01(parseFloat(parts[2]) / 100));
    if (rgb.some((c) => !Number.isFinite(c))) return undefined;
    return { hex: `#${hex2(rgb[0])}${hex2(rgb[1])}${hex2(rgb[2])}`, alpha };
  }
  return undefined;
}

export function rawPaint(v: string | undefined): RawPaint | undefined {
  if (v === undefined) return undefined;
  const s = v.trim();
  const low = s.toLowerCase();
  if (!s || low === 'inherit') return undefined;
  if (low === 'none' || low === 'transparent') return { kind: 'none' };
  if (low === 'currentcolor' || low.startsWith('param(fill')) return { kind: 'fill' };
  if (low.startsWith('param(stroke')) return { kind: 'stroke' };
  const url = /^url\(\s*['"]?#([^'")\s]+)['"]?\s*\)\s*(.*)$/i.exec(s);
  if (url) return { kind: 'url', id: url[1], fallback: url[2] ? rawPaint(url[2]) : undefined };
  if (low.startsWith('url(')) return { kind: 'none' };
  const c = readColor(s);
  return c ? { kind: 'color', ...c } : undefined;
}

export const withAlpha = (hex: string, alpha: number): string => (alpha < 0.999 ? `${hex}${hex2(alpha * 255)}` : hex);

/** Near-black colours (the "black" of icon sets: #000, #1D1D1B, #231F20 …) that become the symbol colour. */
export const isNearBlack = (hex: string) => {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return Math.max(r, g, b) <= 48;
};

/** A paint value as the model's paint (black is the symbol colour); undefined when it cannot be read. */
export function readPaint(v: string | undefined): Paint | undefined {
  const p = rawPaint(v);
  if (!p || p.kind === 'url') return undefined;
  if (p.kind !== 'color') return p.kind;
  return p.hex === '#000000' && p.alpha >= 0.999 ? 'fill' : withAlpha(p.hex, p.alpha);
}

// ── Transforms and lengths ─────────────────────────────────────────────

/** The transform attribute as a matrix (functions applied left to right, as SVG does). */
export function readTransform(v: string | undefined): Matrix {
  if (!v) return IDENTITY;
  let m: Matrix = IDENTITY;
  for (const f of v.matchAll(/(matrix|translate|scale|rotate|skewX|skewY)\s*\(([^)]*)\)/g)) {
    const a = nums(f[2]);
    let t: Matrix = IDENTITY;
    switch (f[1]) {
      case 'matrix':
        if (a.length === 6) t = a as unknown as Matrix;
        break;
      case 'translate':
        t = [1, 0, 0, 1, a[0] ?? 0, a[1] ?? 0];
        break;
      case 'scale':
        t = [a[0] ?? 1, 0, 0, a[1] ?? a[0] ?? 1, 0, 0];
        break;
      case 'rotate': {
        const r = ((a[0] ?? 0) * Math.PI) / 180;
        const c = Math.cos(r);
        const s = Math.sin(r);
        const cx = a[1] ?? 0;
        const cy = a[2] ?? 0;
        t = [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy];
        break;
      }
      case 'skewX':
        t = [1, 0, Math.tan(((a[0] ?? 0) * Math.PI) / 180), 1, 0, 0];
        break;
      case 'skewY':
        t = [1, Math.tan(((a[0] ?? 0) * Math.PI) / 180), 0, 1, 0, 0];
        break;
    }
    m = multiply(m, t);
  }
  return m;
}

/** Pixels per unit (CSS: 96 px to the inch). */
export const PX_PER: Record<string, number> = { '': 1, px: 1, mm: 96 / 25.4, cm: 96 / 2.54, in: 96, pt: 96 / 72, pc: 16, q: 96 / 101.6 };

export function parseLength(v: string | undefined): { value: number; unit: string } | null {
  const m = /^\s*([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)\s*(px|mm|cm|in|pt|pc|q|em|ex|rem|%)?\s*$/i.exec(v ?? '');
  return m ? { value: Number(m[1]), unit: (m[2] ?? '').toLowerCase() } : null;
}

/** A length in user units; % against `percent`, em against `em`. */
export function toUser(v: string | undefined, percent: number, em = 16): number | undefined {
  const l = parseLength(v);
  if (!l) return undefined;
  if (l.unit === '%') return (l.value * percent) / 100;
  if (l.unit === 'em' || l.unit === 'rem') return l.value * em;
  if (l.unit === 'ex') return l.value * em * 0.5;
  return l.value * (PX_PER[l.unit] ?? 1);
}

/** The viewBox → viewport map with preserveAspectRatio (SVG 1.1 §7.8). */
export function viewBoxTransform(vb: readonly number[], width: number, height: number, par = ''): Matrix {
  const [x, y, w, h] = vb;
  const words = par.trim().split(/\s+/).filter((t) => t && t !== 'defer');
  const align = words[0] ?? 'xMidYMid';
  let sx = width / w;
  let sy = height / h;
  let tx = 0;
  let ty = 0;
  if (align !== 'none') {
    const s = words[1] === 'slice' ? Math.max(sx, sy) : Math.min(sx, sy);
    sx = sy = s;
    const ax = align.includes('xMid') ? 0.5 : align.includes('xMax') ? 1 : 0;
    const ay = align.includes('YMid') ? 0.5 : align.includes('YMax') ? 1 : 0;
    tx = (width - w * s) * ax;
    ty = (height - h * s) * ay;
  }
  return [sx, 0, 0, sy, tx - x * sx, ty - y * sy];
}

export const viewBoxOf = (v: string | undefined) => {
  const a = nums(v);
  return a.length === 4 && a[2] > 0 && a[3] > 0 ? a : null;
};

// ── CSS ────────────────────────────────────────────────────────────────

interface Compound {
  tag?: string;
  id?: string;
  classes: string[];
  attrs: { name: string; value?: string }[];
}
interface Selector {
  /** Right to left: the element itself first, then its ancestors with the combinator between. */
  parts: { c: Compound; child: boolean }[];
  spec: number;
}
export interface Rule {
  selectors: Selector[];
  decls: { prop: string; value: string; important: boolean }[];
  order: number;
}

export function parseDecls(text: string): { prop: string; value: string; important: boolean }[] {
  const out: { prop: string; value: string; important: boolean }[] = [];
  for (const decl of text.split(';')) {
    const i = decl.indexOf(':');
    if (i <= 0) continue;
    let value = decl.slice(i + 1).trim();
    const important = /!\s*important\s*$/i.test(value);
    if (important) value = value.replace(/!\s*important\s*$/i, '').trim();
    out.push({ prop: decl.slice(0, i).trim().toLowerCase(), value, important });
  }
  return out;
}

function parseSelector(text: string): Selector | null {
  const tokens = text.trim().replace(/\s*>\s*/g, ' > ').split(/\s+/).filter(Boolean);
  const parts: { c: Compound; child: boolean }[] = [];
  let spec = 0;
  let child = false;
  for (const t of tokens) {
    if (t === '>') {
      child = true;
      continue;
    }
    if (/[+~:]/.test(t)) return null;
    const c: Compound = { classes: [], attrs: [] };
    const re = /([#.]?)(-?[_a-zA-Z][\w-]*)|\[\s*([\w:-]+)\s*(?:=\s*["']?([^"'\]]*)["']?)?\s*\]|(\*)/g;
    let pos = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(t))) {
      if (m.index !== pos) return null;
      pos = re.lastIndex;
      if (m[5]) continue;
      if (m[3]) {
        c.attrs.push({ name: m[3], value: m[4] });
        spec += 100;
      } else if (m[1] === '#') {
        c.id = m[2];
        spec += 10000;
      } else if (m[1] === '.') {
        c.classes.push(m[2]);
        spec += 100;
      } else {
        c.tag = m[2].toLowerCase();
        spec += 1;
      }
    }
    if (pos !== t.length) return null;
    parts.unshift({ c, child: false });
    if (parts.length > 1) parts[1].child = child;
    child = false;
  }
  return parts.length ? { parts, spec } : null;
}

/** Rules of a style sheet; at-rule blocks (@media, @font-face …) are left out. */
export function parseCss(text: string, orderFrom = 0): Rule[] {
  const src = text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/<!--|-->/g, '');
  const rules: Rule[] = [];
  let i = 0;
  let order = orderFrom;
  while (i < src.length) {
    const open = src.indexOf('{', i);
    if (open < 0) break;
    const head = src.slice(i, open).trim();
    // Find the matching close brace (at-rules may nest).
    let depth = 1;
    let j = open + 1;
    while (j < src.length && depth) {
      if (src[j] === '{') depth++;
      else if (src[j] === '}') depth--;
      j++;
    }
    const body = src.slice(open + 1, j - 1);
    i = j;
    if (head.startsWith('@')) continue;
    const selectors = head
      .split(',')
      .map(parseSelector)
      .filter((s): s is Selector => !!s);
    if (selectors.length) rules.push({ selectors, decls: parseDecls(body), order: order++ });
  }
  return rules;
}

function matchCompound(c: Compound, n: XmlNode): boolean {
  if (c.tag && n.tag.toLowerCase() !== c.tag) return false;
  if (c.id && n.attrs.id !== c.id) return false;
  if (c.classes.length) {
    const cls = (n.attrs.class ?? '').split(/\s+/);
    if (!c.classes.every((k) => cls.includes(k))) return false;
  }
  return c.attrs.every((a) => (a.value === undefined ? a.name in n.attrs : n.attrs[a.name] === a.value));
}

/** Does the selector match `n` whose ancestors are `path` (root first)? */
export function matches(sel: Selector, n: XmlNode, path: readonly XmlNode[]): boolean {
  if (!matchCompound(sel.parts[0].c, n)) return false;
  let at = path.length - 1;
  for (let k = 1; k < sel.parts.length; k++) {
    const { c } = sel.parts[k];
    const child = sel.parts[k - 1].child;
    if (child) {
      if (at < 0 || !matchCompound(c, path[at])) return false;
      at--;
    } else {
      while (at >= 0 && !matchCompound(c, path[at])) at--;
      if (at < 0) return false;
      at--;
    }
  }
  return true;
}

/** Properties read from presentation attributes, rules and `style`. */
export const PROPS = new Set([
  'fill', 'stroke', 'stroke-width', 'stroke-opacity', 'fill-opacity', 'opacity', 'stroke-dasharray', 'stroke-linecap', 'stroke-linejoin', 'fill-rule',
  'font-size', 'font-family', 'font-weight', 'text-anchor', 'display', 'visibility', 'color', 'clip-path', 'mask', 'filter', 'stop-color', 'stop-opacity',
  'marker-start', 'marker-mid', 'marker-end', 'marker',
]);
