import { IDENTITY, multiply, parsePathData, type Matrix } from './pathData';
import { newDoc, shapeBox, shapeId, transformShape, type Paint, type SvgDoc, type SvgShape } from './svgModel';
import { PROPS, PX_PER, clamp01, isNearBlack, matches, nums, parseCss, parseDecls, parseLength, rawPaint, readColor, readTransform, toUser, viewBoxOf, viewBoxTransform, withAlpha, type Flat, type RawPaint, type Rule, type XmlNode } from './svgValues';

export { parseCss, readColor, readPaint, readTransform, viewBoxTransform, type XmlNode } from './svgValues';

/**
 * An SVG file read into the editor's model. The caller parses the XML
 * (DOMParser in the UI) into plain nodes; this walks them the way a
 * browser would: CSS `<style>` rules (class, id, tag, descendant and child
 * selectors), presentation attributes and `style` with inheritance, every
 * transform, nested `<svg>`, `<defs>`/`<symbol>`/`<use>`, viewBox with
 * units and preserveAspectRatio, and all basic shapes, paths and texts.
 * What the model cannot hold is simplified and counted in the report:
 * gradients and patterns become one flat colour, clip paths, masks,
 * filters, markers and images are left out. Pure.
 */

// ── The walk ───────────────────────────────────────────────────────────

/** Inherited properties as computed. */
interface Computed {
  fill: RawPaint;
  stroke: RawPaint;
  strokeWidth: number;
  fillOpacity: number;
  strokeOpacity: number;
  dash: number[] | null;
  cap: 'butt' | 'round' | 'square';
  join: 'miter' | 'round' | 'bevel';
  fillRule: 'nonzero' | 'evenodd';
  fontSize: number;
  fontFamily: string;
  fontWeight: number;
  anchor: 'start' | 'middle' | 'end';
  hidden: boolean;
}

const INITIAL: Computed = {
  fill: { kind: 'color', hex: '#000000', alpha: 1 },
  stroke: { kind: 'none' },
  strokeWidth: 1,
  fillOpacity: 1,
  strokeOpacity: 1,
  dash: null,
  cap: 'butt',
  join: 'miter',
  fillRule: 'nonzero',
  fontSize: 16,
  fontFamily: '',
  fontWeight: 400,
  anchor: 'start',
  hidden: false,
};

const FONT_SIZES: Record<string, number> = { 'xx-small': 9, 'x-small': 10, small: 13, medium: 16, large: 18, 'x-large': 24, 'xx-large': 32 };

/** What a file held that the drawing could not keep as it was. */
export interface ImportReport {
  shapes: number;
  /** `<use>` copies expanded. */
  uses: number;
  gradients: number;
  patterns: number;
  clips: number;
  masks: number;
  filters: number;
  markers: number;
  images: number;
  /** Shapes whose fill and stroke opacity could not both be kept. */
  approxOpacity: number;
  /** References to elements that are not in the file (or refer to themselves). */
  broken: number;
  /** The file paints with the symbol's colours itself (currentColor / param()). */
  symbolPaint: boolean;
}

export interface ColorUse {
  color: string;
  /** Rough area painted (fill: box area; stroke: box perimeter × width). */
  weight: number;
  count: number;
}

/** 'black': near-black colours; 'dominant': the colour painting the most; a hex; null: none. */
export type ColorTarget = 'black' | 'dominant' | string | null;

export interface ImportOptions {
  /** The colour that becomes the symbol's colour; default 'auto' (black, unless the file uses currentColor/param()). */
  symbolColor?: ColorTarget | 'auto';
  /** The colour that becomes the second colour (param(stroke)). */
  secondColor?: string | null;
  /** The editor's own source: element ids, `data-name`, `data-group` and hidden shapes are kept. */
  editor?: boolean;
}

/** A tracing reference kept in a drawing's file (docs/STYLE.md §7). */
export interface ReferenceSpec {
  href: string;
  x: number;
  y: number;
  width: number;
  height: number;
  opacity: number;
  locked: boolean;
  name: string;
}

export interface ImportResult {
  doc: SvgDoc;
  /** Element names that were left out. */
  skipped: string[];
  report: ImportReport;
  /** Fixed colours of the drawing (as read, before mapping), most painted first. */
  colors: ColorUse[];
  reference?: ReferenceSpec;
}

let groupSeq = 0;
const newGroup = () => `g${Date.now().toString(36)}${(groupSeq++).toString(36)}`;

const NOT_RENDERED = new Set(['defs', 'symbol', 'clippath', 'mask', 'marker', 'pattern', 'lineargradient', 'radialgradient', 'filter', 'style', 'title', 'desc', 'metadata', 'script', 'namedview', 'font', 'font-face', 'cursor', 'view', 'animate', 'animatemotion', 'animatetransform', 'set', 'mpath', 'foreignobject', 'solidcolor']);

export function docFromSvgTree(root: XmlNode, opts: ImportOptions = {}): ImportResult {
  const skipped = new Set<string>();
  const report: ImportReport = { shapes: 0, uses: 0, gradients: 0, patterns: 0, clips: 0, masks: 0, filters: 0, markers: 0, images: 0, approxOpacity: 0, broken: 0, symbolPaint: false };
  const seen = { gradients: new Set<string>(), patterns: new Set<string>(), clips: new Set<string>(), masks: new Set<string>(), filters: new Set<string>(), markers: new Set<string>() };
  const byId = new Map<string, XmlNode>();
  const styleTexts: string[] = [];
  let reference: ReferenceSpec | undefined;
  const scan = (n: XmlNode) => {
    if (n.attrs.id && !byId.has(n.attrs.id)) byId.set(n.attrs.id, n);
    if (n.tag.toLowerCase() === 'style' && (!n.attrs.type || /css/i.test(n.attrs.type))) styleTexts.push(textOf(n));
    if (n.tag === 'image' && n.attrs['data-kentos'] === 'reference') {
      const href = n.attrs.href ?? n.attrs['xlink:href'] ?? '';
      if (/^data:image\//.test(href))
        reference = { href, x: num(n.attrs.x), y: num(n.attrs.y), width: num(n.attrs.width, 1), height: num(n.attrs.height, 1), opacity: num(n.attrs.opacity, 0.5), locked: n.attrs['data-locked'] !== '0', name: n.attrs['data-name'] ?? 'Altlık' };
    }
    for (const c of n.children) scan(c);
  };
  scan(root);
  const rules: Rule[] = [];
  for (const t of styleTexts) rules.push(...parseCss(t, rules.length));

  // ── Root: canvas, units, physical size ──
  const vb = viewBoxOf(root.attrs.viewBox);
  const wl = parseLength(root.attrs.width);
  const hl = parseLength(root.attrs.height);
  const absolute = (l: { unit: string } | null) => !!l && l.unit !== '%' && l.unit !== 'em' && l.unit !== 'ex' && l.unit !== 'rem';
  let width: number;
  let height: number;
  let origin: Matrix = IDENTITY;
  let sizeMm: number | undefined;
  if (vb) {
    width = vb[2];
    height = vb[3];
    origin = [1, 0, 0, 1, -vb[0], -vb[1]];
    // A viewBox stretched onto a viewport of another shape keeps that stretch.
    const par = (root.attrs.preserveAspectRatio ?? '').trim();
    if (par.startsWith('none') && absolute(wl) && absolute(hl)) {
      const vw = wl!.value * PX_PER[wl!.unit];
      const vh = hl!.value * PX_PER[hl!.unit];
      const k = vh / vw / (height / width);
      if (Math.abs(k - 1) > 1e-6) {
        origin = multiply([1, 0, 0, k, 0, 0], origin);
        height *= k;
      }
    }
    if (absolute(wl) && wl!.unit !== '' && wl!.unit !== 'px') sizeMm = (wl!.value * PX_PER[wl!.unit]) / PX_PER.mm;
  } else if (absolute(wl) && absolute(hl) && wl!.unit !== '' && wl!.unit !== 'px') {
    // Without a viewBox user units are px; a size in mm (cm, in, pt) makes that unit the drawing's.
    const k = PX_PER[wl!.unit];
    width = wl!.value;
    height = (hl!.value * PX_PER[hl!.unit]) / k;
    origin = [1 / k, 0, 0, 1 / k, 0, 0];
    sizeMm = (wl!.value * k) / PX_PER.mm;
  } else {
    width = absolute(wl) ? wl!.value * PX_PER[wl!.unit] : 0;
    height = absolute(hl) ? hl!.value * PX_PER[hl!.unit] : 0;
  }
  const doc = newDoc(width || 100, height || 100);
  if (sizeMm && Number.isFinite(sizeMm) && sizeMm > 0) doc.sizeMm = Math.round(sizeMm * 1000) / 1000;
  const noSize = !vb && !width && !height;
  if (root.attrs['data-size-mm']) doc.sizeMm = num(root.attrs['data-size-mm']) || doc.sizeMm;
  if (root.attrs['data-background'] && readColor(root.attrs['data-background'])) doc.background = readColor(root.attrs['data-background'])!.hex;

  // ── Styles ──
  const specified = (n: XmlNode, path: readonly XmlNode[]): Record<string, string> => {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(n.attrs)) if (PROPS.has(k)) out[k] = v;
    const hits: { spec: number; order: number; decls: Rule['decls'] }[] = [];
    for (const r of rules) {
      let best = -1;
      for (const s of r.selectors) if (s.spec > best && matches(s, n, path)) best = s.spec;
      if (best >= 0) hits.push({ spec: best, order: r.order, decls: r.decls });
    }
    hits.sort((a, b) => a.spec - b.spec || a.order - b.order);
    const important: Record<string, string> = {};
    for (const hit of hits)
      for (const d of hit.decls) {
        if (d.important) important[d.prop] = d.value;
        else out[d.prop] = d.value;
      }
    for (const d of parseDecls(n.attrs.style ?? '')) out[d.prop] = d.value;
    return Object.assign(out, important);
  };

  const compute = (a: Record<string, string>, p: Computed): Computed => {
    const c = { ...p };
    const val = (k: string) => {
      const v = a[k]?.trim();
      return v && v !== 'inherit' ? v : undefined;
    };
    const fill = rawPaint(val('fill'));
    if (fill) c.fill = fill;
    const stroke = rawPaint(val('stroke'));
    if (stroke) c.stroke = stroke;
    const fontSize = val('font-size');
    if (fontSize) c.fontSize = FONT_SIZES[fontSize] ?? toUser(fontSize, p.fontSize, p.fontSize) ?? p.fontSize;
    const sw = toUser(val('stroke-width'), Math.hypot(width, height) / Math.SQRT2, c.fontSize);
    if (sw !== undefined && sw >= 0) c.strokeWidth = sw;
    const op = (v: string | undefined, prev: number) => {
      if (v === undefined) return prev;
      const x = v.endsWith('%') ? parseFloat(v) / 100 : parseFloat(v);
      return Number.isFinite(x) ? clamp01(x) : prev;
    };
    c.fillOpacity = op(val('fill-opacity'), p.fillOpacity);
    c.strokeOpacity = op(val('stroke-opacity'), p.strokeOpacity);
    const dash = val('stroke-dasharray');
    if (dash) {
      const d = dash === 'none' ? [] : dash.split(/[\s,]+/).filter(Boolean).map((t) => toUser(t, 100, c.fontSize) ?? NaN);
      c.dash = !d.length || d.some((x) => !(x >= 0)) || d.every((x) => x === 0) ? null : d.length % 2 ? [...d, ...d] : d;
    }
    const cap = val('stroke-linecap');
    if (cap === 'butt' || cap === 'round' || cap === 'square') c.cap = cap;
    const join = val('stroke-linejoin');
    if (join === 'round' || join === 'bevel') c.join = join;
    else if (join === 'miter' || join === 'miter-clip' || join === 'arcs') c.join = 'miter';
    const rule = val('fill-rule');
    if (rule === 'nonzero' || rule === 'evenodd') c.fillRule = rule;
    const family = val('font-family');
    if (family) c.fontFamily = family;
    const weight = val('font-weight');
    if (weight) c.fontWeight = weight === 'bold' || weight === 'bolder' ? 700 : weight === 'normal' || weight === 'lighter' ? 400 : Number(weight) || p.fontWeight;
    const anchor = val('text-anchor');
    if (anchor === 'start' || anchor === 'middle' || anchor === 'end') c.anchor = anchor;
    const vis = val('visibility');
    if (vis) c.hidden = vis === 'hidden' || vis === 'collapse';
    return c;
  };

  // ── Paint references (gradients and patterns become one flat colour) ──
  const stopsOf = (g: XmlNode, depth = 0): XmlNode[] => {
    const own = g.children.filter((c) => c.tag.toLowerCase() === 'stop');
    if (own.length || depth > 8) return own;
    const href = (g.attrs.href ?? g.attrs['xlink:href'] ?? '').replace(/^#/, '');
    const next = href ? byId.get(href) : undefined;
    return next && next !== g ? stopsOf(next, depth + 1) : own;
  };
  const flatten = (p: RawPaint, depth = 0): Flat => {
    if (p.kind !== 'url') return p;
    const el = byId.get(p.id);
    const tag = el?.tag.toLowerCase();
    if (el && (tag === 'lineargradient' || tag === 'radialgradient')) {
      seen.gradients.add(p.id);
      const stops = stopsOf(el);
      if (!stops.length) return { kind: 'none' };
      const stop = stops[Math.floor((stops.length - 1) / 2)];
      const a = specified(stop, []);
      const c = readColor(a['stop-color'] ?? 'black') ?? { hex: '#000000', alpha: 1 };
      const so = a['stop-opacity'] !== undefined ? clamp01(parseFloat(a['stop-opacity'])) : 1;
      return { kind: 'color', hex: c.hex, alpha: c.alpha * (Number.isFinite(so) ? so : 1) };
    }
    if (el && tag === 'pattern') {
      seen.patterns.add(p.id);
      let found: Flat | null = null;
      const look = (n: XmlNode) => {
        if (found) return;
        const f = rawPaint(specified(n, [])['fill']);
        if (f && f.kind !== 'none' && n.tag !== 'pattern' && depth < 4) found = flatten(f, depth + 1);
        n.children.forEach(look);
      };
      look(el);
      return found ?? { kind: 'color', hex: '#808080', alpha: 1 };
    }
    if (p.fallback) return flatten(p.fallback, depth + 1);
    report.broken++;
    return { kind: 'none' };
  };

  const noteRef = (a: Record<string, string>) => {
    const ref = (v: string | undefined, set: Set<string>) => {
      const m = /url\(\s*['"]?#([^'")\s]+)/.exec(v ?? '');
      if (m) set.add(m[1]);
    };
    ref(a['clip-path'], seen.clips);
    ref(a.mask, seen.masks);
    ref(a.filter, seen.filters);
    for (const k of ['marker', 'marker-start', 'marker-mid', 'marker-end']) ref(a[k], seen.markers);
  };

  // ── Shapes ──
  interface Ctx {
    m: Matrix;
    opacity: number;
    group?: string;
    path: XmlNode[];
    uses: Set<string>;
    depth: number;
  }

  const put = (s: SvgShape, c: Computed, ctx: Ctx, n: XmlNode, a: Record<string, string>) => {
    const scale = Math.sqrt(Math.abs(ctx.m[0] * ctx.m[3] - ctx.m[1] * ctx.m[2]));
    const fill = flatten(c.fill);
    const stroke = flatten(c.stroke);
    if (fill.kind === 'fill' || fill.kind === 'stroke' || stroke.kind === 'fill' || stroke.kind === 'stroke') report.symbolPaint = true;
    let opacity = ctx.opacity;
    const fa = fill.kind === 'color' ? fill.alpha * c.fillOpacity : c.fillOpacity;
    const sa = stroke.kind === 'color' ? stroke.alpha * c.strokeOpacity : c.strokeOpacity;
    const paint = (f: Flat, alpha: number): Paint => (f.kind === 'color' ? withAlpha(f.hex, alpha) : f.kind);
    // The symbol's colours carry no alpha: their opacity moves to the shape when only one of them paints.
    const symF = fill.kind === 'fill' || fill.kind === 'stroke';
    const symS = stroke.kind === 'fill' || stroke.kind === 'stroke';
    if (symF && fa < 0.999 && (stroke.kind === 'none' || !symS)) opacity *= fa;
    else if (symS && sa < 0.999 && fill.kind === 'none') opacity *= sa;
    else if ((symF && fa < 0.999) || (symS && sa < 0.999)) report.approxOpacity++;
    const out = s as SvgShape & Record<string, unknown>;
    out.fill = paint(fill, symF ? 1 : fa);
    out.stroke = paint(stroke, symS ? 1 : sa);
    out.strokeWidth = Math.round(c.strokeWidth * scale * 1e6) / 1e6;
    if (opacity < 0.999) out.opacity = Math.round(opacity * 1000) / 1000;
    if (stroke.kind !== 'none' && c.dash) out.dash = c.dash.map((d) => Math.round(d * scale * 1e6) / 1e6);
    const isPath = s.kind === 'path';
    // Paths default to round ends and evenodd in the model; SVG defaults are written out.
    if (stroke.kind !== 'none' && (isPath || c.cap !== 'butt')) out.cap = c.cap;
    if (stroke.kind !== 'none' && (isPath || c.join !== 'miter')) out.join = c.join;
    if (isPath && fill.kind !== 'none') out.fillRule = c.fillRule;
    if (ctx.group) out.group = ctx.group;
    if (opts.editor) {
      if (n.attrs.id && !/^\d/.test(n.attrs.id)) out.id = n.attrs.id;
      if (n.attrs['data-name']) out.name = n.attrs['data-name'];
      if (a.display === 'none') out.hidden = true;
    } else {
      const label = n.attrs['inkscape:label'] ?? n.children.find((k) => k.tag === 'title')?.children.find((k) => k.tag === '#text')?.text ?? n.children.find((k) => k.tag === 'title')?.text;
      if (label?.trim()) out.name = label.trim().slice(0, 60);
    }
    let t = transformShape(s, ctx.m);
    // A rectangle or ellipse sheared into a path keeps the SVG corners.
    if (t.kind === 'path' && s.kind !== 'path' && t.stroke !== 'none') t = { ...t, cap: t.cap ?? 'butt', join: t.join ?? 'miter' };
    doc.shapes.push(t);
    report.shapes++;
  };

  const lengthX = (v: string | undefined, fallback = 0) => toUser(v, width) ?? fallback;
  const lengthY = (v: string | undefined, fallback = 0) => toUser(v, height) ?? fallback;
  const lengthR = (v: string | undefined, fallback = 0) => toUser(v, Math.hypot(width, height) / Math.SQRT2) ?? fallback;

  const walk = (n: XmlNode, parent: Computed, ctx: Ctx) => {
    const tag = n.tag.toLowerCase();
    if (tag === '#text') return;
    if (NOT_RENDERED.has(tag) && !(tag === 'svg' || tag === 'symbol')) return;
    if (tag === 'symbol') return;
    const a = specified(n, ctx.path);
    if (a.display === 'none' && !opts.editor) return;
    noteRef(a);
    const c = compute(a, parent);
    const own = num(a.opacity, 1);
    const m = multiply(ctx.m, readTransform(n.attrs.transform));
    const next: Ctx = { ...ctx, m, opacity: ctx.opacity * clamp01(own), path: [...ctx.path, n], depth: ctx.depth + 1 };
    const visible = !c.hidden || opts.editor;
    switch (tag) {
      case 'svg': {
        if (ctx.depth > 0) {
          // A nested viewport: its own position, size and viewBox.
          const x = lengthX(n.attrs.x);
          const y = lengthY(n.attrs.y);
          const w = lengthX(n.attrs.width, width);
          const h = lengthY(n.attrs.height, height);
          const box = viewBoxOf(n.attrs.viewBox);
          next.m = multiply(multiply(m, [1, 0, 0, 1, x, y]), box ? viewBoxTransform(box, w, h, n.attrs.preserveAspectRatio) : IDENTITY);
        }
        for (const k of n.children) walk(k, c, next);
        return;
      }
      case 'g':
      case 'a':
      case 'switch': {
        // A group below the root keeps its shapes together (not an Inkscape layer, not a link).
        const layer = n.attrs['inkscape:groupmode'] === 'layer';
        if (tag === 'g' && ctx.depth > 0 && !layer && !ctx.group) next.group = (opts.editor && n.attrs['data-group']) || newGroup();
        const kids = tag === 'switch' ? n.children.filter((k) => k.tag !== '#text').slice(0, 1) : n.children;
        for (const k of kids) walk(k, c, next);
        return;
      }
      case 'use': {
        const id = (n.attrs.href ?? n.attrs['xlink:href'] ?? '').replace(/^#/, '');
        const target = id ? byId.get(id) : undefined;
        if (!target || ctx.uses.has(id) || ctx.path.includes(target)) {
          report.broken++;
          return;
        }
        report.uses++;
        const uses = new Set(ctx.uses).add(id);
        const at = multiply(m, [1, 0, 0, 1, lengthX(n.attrs.x), lengthY(n.attrs.y)]);
        const inner: Ctx = { ...next, m: at, uses, group: ctx.group ?? (ctx.depth > 0 || n.attrs.transform ? newGroup() : undefined) };
        const ttag = target.tag.toLowerCase();
        if (ttag === 'symbol' || ttag === 'svg') {
          const box = viewBoxOf(target.attrs.viewBox);
          const w = lengthX(n.attrs.width ?? target.attrs.width, box ? box[2] : width);
          const h = lengthY(n.attrs.height ?? target.attrs.height, box ? box[3] : height);
          if (ttag === 'svg') inner.m = multiply(inner.m, [1, 0, 0, 1, lengthX(target.attrs.x), lengthY(target.attrs.y)]);
          if (box) inner.m = multiply(inner.m, viewBoxTransform(box, w, h, target.attrs.preserveAspectRatio));
          const sc = compute(specified(target, next.path), c);
          for (const k of target.children) walk(k, sc, { ...inner, path: [...inner.path, target] });
        } else walk(target, c, inner);
        return;
      }
      case 'image':
        if (n.attrs['data-kentos'] !== 'reference') {
          report.images++;
          skipped.add('image');
        }
        return;
    }
    if (!visible) return;
    const base = { id: shapeId(), fill: 'none' as Paint, stroke: 'none' as Paint, strokeWidth: 1 };
    switch (tag) {
      case 'rect': {
        const w = lengthX(n.attrs.width);
        const h = lengthY(n.attrs.height);
        if (!(w > 0 && h > 0)) return;
        const x = lengthX(n.attrs.x);
        const y = lengthY(n.attrs.y);
        let rx = toUser(n.attrs.rx === 'auto' ? undefined : n.attrs.rx, width);
        let ry = toUser(n.attrs.ry === 'auto' ? undefined : n.attrs.ry, height);
        rx ??= ry ?? 0;
        ry ??= rx;
        rx = Math.min(Math.max(0, rx), w / 2);
        ry = Math.min(Math.max(0, ry), h / 2);
        if (Math.abs(rx - ry) < 1e-9) put({ ...base, kind: 'rect', x, y, w, h, r: rx || undefined }, c, next, n, a);
        else {
          // Elliptic corners: a path of four arcs.
          const d = `M${x + rx} ${y}H${x + w - rx}A${rx} ${ry} 0 0 1 ${x + w} ${y + ry}V${y + h - ry}A${rx} ${ry} 0 0 1 ${x + w - rx} ${y + h}H${x + rx}A${rx} ${ry} 0 0 1 ${x} ${y + h - ry}V${y + ry}A${rx} ${ry} 0 0 1 ${x + rx} ${y}Z`;
          put({ ...base, kind: 'path', subs: parsePathData(d) }, c, next, n, a);
        }
        return;
      }
      case 'circle':
      case 'ellipse': {
        const rx = tag === 'circle' ? lengthR(n.attrs.r) : lengthX(n.attrs.rx === 'auto' ? n.attrs.ry : n.attrs.rx, NaN);
        const ry = tag === 'circle' ? rx : lengthY(n.attrs.ry === 'auto' ? n.attrs.rx : n.attrs.ry, NaN);
        const rrx = Number.isFinite(rx) ? rx : ry;
        const rry = Number.isFinite(ry) ? ry : rx;
        if (rrx > 0 && rry > 0) put({ ...base, kind: 'ellipse', cx: lengthX(n.attrs.cx), cy: lengthY(n.attrs.cy), rx: rrx, ry: rry }, c, next, n, a);
        return;
      }
      case 'line':
        put({ ...base, kind: 'path', subs: parsePathData(`M${lengthX(n.attrs.x1)} ${lengthY(n.attrs.y1)}L${lengthX(n.attrs.x2)} ${lengthY(n.attrs.y2)}`) }, { ...c, fill: { kind: 'none' } }, next, n, a);
        return;
      case 'polyline':
      case 'polygon': {
        const p = nums(n.attrs.points);
        const pts: string[] = [];
        for (let i = 0; i + 1 < p.length; i += 2) pts.push(`${p[i]} ${p[i + 1]}`);
        if (pts.length > 1) put({ ...base, kind: 'path', subs: parsePathData(`M${pts.join('L')}${tag === 'polygon' ? 'Z' : ''}`) }, c, next, n, a);
        return;
      }
      case 'path': {
        const subs = n.attrs.d ? parsePathData(n.attrs.d) : [];
        if (subs.length) put({ ...base, kind: 'path', subs }, c, next, n, a);
        return;
      }
      case 'text':
        textShapes(n, c, next, a);
        return;
      default:
        if (!NOT_RENDERED.has(tag) && tag !== 'tspan' && tag !== 'textpath') skipped.add(n.tag);
    }
  };

  /** A text element's lines: every run placed anew (x, y, dx, dy) starts a text shape. */
  const textShapes = (n: XmlNode, c0: Computed, ctx: Ctx, a0: Record<string, string>) => {
    const lines: { x: number; y: number; text: string; c: Computed }[] = [];
    let cur: (typeof lines)[number] | null = null;
    let px = 0;
    let py = 0;
    const visit = (el: XmlNode, c: Computed, path: XmlNode[]) => {
      // Positions may carry units; em is the element's own font size.
      const list = (v: string | undefined, percent: number) => (v ?? '').split(/[\s,]+/).filter(Boolean).map((t) => toUser(t, percent, c.fontSize) ?? NaN).filter(Number.isFinite);
      const xs = list(el.attrs.x, width);
      const ys = list(el.attrs.y, height);
      const dx = list(el.attrs.dx, width);
      const dy = list(el.attrs.dy, height);
      if (xs.length) px = xs[0];
      if (ys.length) py = ys[0];
      if (dx.length) px += dx[0];
      if (dy.length) py += dy[0];
      if (xs.length || ys.length || dx.length || dy.length) cur = null;
      const runs = el.children.length ? el.children : el.text !== undefined ? [{ tag: '#text', attrs: {}, children: [], text: el.text }] : [];
      for (const k of runs) {
        if (k.tag === '#text') {
          const t = (k.text ?? '').replace(/\s+/g, ' ');
          if (!cur && !t.trim()) continue;
          if (!cur) lines.push((cur = { x: px, y: py, text: '', c }));
          cur.text += t;
          px += t.length * c.fontSize * 0.55;
        } else if (/^(tspan|textpath|a)$/i.test(k.tag)) {
          const ka = specified(k, path);
          if (ka.display === 'none') continue;
          visit(k, compute(ka, c), [...path, k]);
        }
      }
    };
    visit(n, c0, ctx.path);
    for (const l of lines) {
      const text = l.text.trim();
      if (!text || l.c.hidden) continue;
      const w = l.c.fontWeight;
      const family = l.c.fontFamily.toLowerCase();
      put(
        {
          id: shapeId(),
          kind: 'text',
          x: l.x,
          y: l.y,
          text,
          size: l.c.fontSize,
          weight: w >= 850 ? 900 : w >= 550 ? 700 : 400,
          font: family.includes('serif') && !family.includes('sans') ? 'serif' : 'sans',
          anchor: l.c.anchor,
          fill: 'none',
          stroke: 'none',
          strokeWidth: 1,
        },
        l.c,
        ctx,
        n,
        a0,
      );
    }
  };

  walk(root, INITIAL, { m: origin, opacity: 1, path: [], uses: new Set(), depth: 0 });

  // A group of one is no group.
  const members = new Map<string, number>();
  for (const s of doc.shapes) if (s.group) members.set(s.group, (members.get(s.group) ?? 0) + 1);
  doc.shapes = doc.shapes.map((s) => (s.group && members.get(s.group) === 1 ? { ...s, group: undefined } : s));

  // No size at all: the canvas takes the drawing's extent.
  if (noSize && doc.shapes.length) {
    let maxX = 0;
    let maxY = 0;
    for (const s of doc.shapes) {
      const b = shapeBox(s);
      maxX = Math.max(maxX, b.maxX);
      maxY = Math.max(maxY, b.maxY);
    }
    doc.width = Math.ceil(maxX) || 100;
    doc.height = Math.ceil(maxY) || 100;
  }

  report.gradients = seen.gradients.size;
  report.patterns = seen.patterns.size;
  report.clips = seen.clips.size;
  report.masks = seen.masks.size;
  report.filters = seen.filters.size;
  report.markers = seen.markers.size;
  const colors = colorUsage(doc);
  const target = opts.symbolColor === undefined || opts.symbolColor === 'auto' ? (report.symbolPaint ? null : 'black') : opts.symbolColor;
  const mapped = target || opts.secondColor ? mapColors(doc, target, opts.secondColor ?? null) : doc;
  return { doc: mapped, skipped: [...skipped], report, colors, reference };
}

function textOf(n: XmlNode): string {
  if (!n.children.length) return n.text ?? '';
  return n.children.map((k) => (k.tag === '#text' ? (k.text ?? '') : textOf(k))).join('');
}

const num = (v: string | undefined, fallback = 0) => {
  const x = parseFloat(v ?? '');
  return Number.isFinite(x) ? x : fallback;
};

// ── Colour mapping ─────────────────────────────────────────────────────

const rgbOf = (p: Paint) => (p.startsWith('#') ? p.slice(0, 7).toUpperCase() : null);

/** The fixed colours of a drawing, most painted first. */
export function colorUsage(doc: SvgDoc): ColorUse[] {
  const map = new Map<string, ColorUse>();
  const add = (p: Paint, weight: number) => {
    const c = rgbOf(p);
    if (!c) return;
    const u = map.get(c) ?? { color: c, weight: 0, count: 0 };
    u.weight += weight;
    u.count++;
    map.set(c, u);
  };
  for (const s of doc.shapes) {
    const b = shapeBox(s);
    const w = Math.max(0, b.maxX - b.minX);
    const h = Math.max(0, b.maxY - b.minY);
    add(s.fill, w * h);
    if (s.stroke !== 'none') add(s.stroke, 2 * (w + h) * s.strokeWidth);
  }
  return [...map.values()].sort((a, b) => b.weight - a.weight || b.count - a.count);
}

/**
 * Fixed colours turned into the symbol's colours: the chosen one (or all
 * near-black ones, or the dominant one) becomes the symbol colour, another
 * the second colour. A mapped colour's alpha moves to the shape's opacity
 * when the shape has no other paint.
 */
export function mapColors(doc: SvgDoc, symbol: ColorTarget, second: string | null): SvgDoc {
  const dominant = symbol === 'dominant' ? (colorUsage(doc)[0]?.color ?? null) : null;
  const toFill = (rgb: string) => (symbol === 'black' ? isNearBlack(rgb) : symbol === 'dominant' ? rgb === dominant : !!symbol && rgb === symbol.toUpperCase());
  const toStroke = (rgb: string) => !!second && rgb === second.toUpperCase() && !toFill(rgb);
  const shapes = doc.shapes.map((s) => {
    let opacity = s.opacity ?? 1;
    const one = (p: Paint, other: Paint): Paint => {
      const rgb = rgbOf(p);
      if (!rgb) return p;
      const to = toFill(rgb) ? 'fill' : toStroke(rgb) ? 'stroke' : null;
      if (!to) return p;
      if (p.length === 9 && other === 'none') opacity *= parseInt(p.slice(7), 16) / 255;
      return to;
    };
    const fill = one(s.fill, s.stroke);
    const stroke = one(s.stroke, s.fill);
    if (fill === s.fill && stroke === s.stroke) return s;
    return { ...s, fill, stroke, opacity: opacity < 0.999 ? Math.round(opacity * 1000) / 1000 : undefined };
  });
  return { ...doc, shapes };
}

// ── Summary ────────────────────────────────────────────────────────────

/** What an import did, in the words of the status line ("2 degrade düz renge çevrildi, 1 kırpma yolu atlandı"). */
export function importSummary(r: ImportReport): { done: string; lost: string[] } {
  const lost: string[] = [];
  const say = (n: number, text: string) => n && lost.push(`${n} ${text}`);
  say(r.gradients, 'degrade düz renge çevrildi');
  say(r.patterns, 'desen düz renge çevrildi');
  say(r.clips, 'kırpma yolu atlandı');
  say(r.masks, 'maske atlandı');
  say(r.filters, 'süzgeç (filtre) atlandı');
  say(r.markers, 'çizgi ucu işareti atlandı');
  say(r.images, 'görüntü atlandı');
  say(r.broken, 'kırık başvuru atlandı');
  say(r.approxOpacity, 'şeklin saydamlığı yaklaşık alındı');
  const done = `${r.shapes} şekil alındı${r.uses ? ` (${r.uses} kopya açıldı)` : ''}`;
  return { done, lost };
}
