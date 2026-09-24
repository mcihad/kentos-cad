import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Symbol } from '../model/style';
import { compileSymbol, ExprCache, ExprRun, type CompileEnv } from '../style/compile';
import { styledGeometry } from '../style/geometry';
import { PrimitiveList, type FillPaint, type MarkerStyle, type PrimUnit, type StrokeStyle, type TileSource } from '../style/primitives';
import { drawShape } from './canvasShapes';
import { parseHex, resolveColor, type CanvasPalette } from './color';
import type { StyleSources } from './styledLayer';
import { applySvgParams } from './styledSink';
import { TEXT_BOX, type RGBA, type ShapeId } from './types';

/**
 * Symbols drawn with Canvas2D from the same primitives the GPU gets: the
 * style manager's thumbnails, the symbol designer's live view and legends.
 * A symbol is compiled on a sample geometry in paper millimetres (plot
 * scale 1:1000, so 1 world unit = 1 mm) and drawn at `pxPerMm`. Hatches,
 * shape patterns (with the shader's random scatter), tiles, strokes with
 * dashes and caps, and shape/SVG/text markers follow the GPU's rules and
 * draw order (by symbol level, then fills, strokes, markers).
 */

export type PreviewGeometry = 'point' | 'line' | 'bent' | 'area' | 'hole';

export interface SymbolPreviewOptions {
  palette: CanvasPalette;
  library: StyleSources;
  /** CSS px per paper mm; by default the symbol is fitted to the canvas. */
  pxPerMm?: number;
  /** Sample geometry; by default the symbol's own kind (point, straight line, rectangle). */
  geometry?: PreviewGeometry;
  /** Canvas background (a colour or theme token); transparent when null. */
  background?: string | null;
  /** Attributes of the sample object, for data-defined values. */
  attrs?: Record<string, string>;
  /** Called when an SVG or raster the drawing needs has loaded (draw again). */
  onLoad?: () => void;
}

/** Sample values for the attributes MPYY's parametric symbols read. */
export const SAMPLE_ATTRS: Record<string, string> = {
  Kat: '4',
  TAKS: '0.40',
  KAKS: '1.60',
  Emsal: '1.60',
  Yençok: '12.50',
  'Ön bahçe': '5',
  'Yan bahçe': '3',
  Parsel: '12',
  Ada: '1245',
  Genişlik: '15',
};

// ── Images ─────────────────────────────────────────────────────────────

type Loaded = HTMLImageElement | 'loading' | 'failed';
const images = new Map<string, Loaded>();
const waiting = new Map<string, Set<() => void>>();

function imageOf(key: string, src: () => string, onLoad?: () => void): HTMLImageElement | null {
  const hit = images.get(key);
  if (hit instanceof HTMLImageElement) return hit;
  if (onLoad) {
    let set = waiting.get(key);
    if (!set) waiting.set(key, (set = new Set()));
    set.add(onLoad);
  }
  if (hit) return null;
  images.set(key, 'loading');
  const img = new Image();
  img.onload = () => {
    images.set(key, img);
    const cbs = waiting.get(key);
    waiting.delete(key);
    cbs?.forEach((cb) => cb());
  };
  img.onerror = () => images.set(key, 'failed');
  img.src = src();
  return null;
}

const svgUrl = (svg: string) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(/\sxmlns\s*=/.test(svg) ? svg : svg.replace(/<svg\b/i, '<svg xmlns="http://www.w3.org/2000/svg"'))}`;

// ── Sample geometry (mm, centred on 0,0) ───────────────────────────────

function sampleEntity(kind: PreviewGeometry, w: number, h: number, attrs: Record<string, string>): Entity {
  const mx = Math.min(w * 0.12, 6);
  const my = Math.min(h * 0.14, 5);
  const x0 = -w / 2 + mx;
  const x1 = w / 2 - mx;
  const y0 = -h / 2 + my;
  const y1 = h / 2 - my;
  const base = { id: 1, layerId: 'preview', attrs };
  switch (kind) {
    case 'point':
      return { ...base, kind: 'point', p: { x: 0, y: 0 } };
    case 'line':
      return { ...base, kind: 'polyline', pts: [v(x0, 0), v(x1, 0)] };
    case 'bent':
      return { ...base, kind: 'polyline', pts: [v(x0, y0 * 0.55), v(x0 + (x1 - x0) * 0.42, y1 * 0.55), v(x1 - (x1 - x0) * 0.18, y0 * 0.2), v(x1, y1 * 0.5)] };
    case 'area':
      return { ...base, kind: 'polygon', pts: [v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1)] };
    case 'hole': {
      const hx = (x1 - x0) * 0.16;
      const hy = (y1 - y0) * 0.2;
      return {
        ...base,
        kind: 'polygon',
        pts: [v(x0, y0), v(x1, y0), v(x1, y1 * 0.35), v(x0 + (x1 - x0) * 0.55, y1), v(x0, y1)],
        holes: [{ pts: [v(x0 + hx * 2, y0 + hy), v(x0 + hx * 3.4, y0 + hy), v(x0 + hx * 3.4, y0 + hy * 2.4), v(x0 + hx * 2, y0 + hy * 2.4)] }],
      };
    }
  }
}

const v = (x: number, y: number): Vec2 => ({ x, y });

/** The preview geometry that shows a symbol best. */
export const defaultGeometry = (s: Symbol): PreviewGeometry => (s.type === 'marker' ? 'point' : s.type === 'line' ? 'line' : 'area');

// ── Drawing ────────────────────────────────────────────────────────────

const KIND_ORDER = { fill: 0, stroke: 1, marker: 2 } as const;

/** The shader's per-cell hash, so previews scatter like the map does. */
function hash3(x: number, y: number): [number, number, number] {
  const fract = (a: number) => a - Math.floor(a);
  let q0 = fract(x * 0.1031);
  let q1 = fract(y * 0.103);
  let q2 = fract(x * 0.0973);
  const d = q0 * (q1 + 33.33) + q1 * (q0 + 33.33) + q2 * (q2 + 33.33);
  q0 += d;
  q1 += d;
  q2 += d;
  return [fract((q0 + q1) * q2), fract((q0 + q2) * q1), fract((q1 + q2) * q0)];
}

/**
 * Draws `symbol` into `canvas` (its CSS size × devicePixelRatio). Returns
 * false when a referenced symbol is missing (nothing drawn).
 */
export function drawSymbolPreview(canvas: HTMLCanvasElement, symbol: Symbol, opts: SymbolPreviewOptions): boolean {
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || Number(canvas.getAttribute('width')) || 64;
  const cssH = canvas.clientHeight || Number(canvas.getAttribute('height')) || 40;
  if (canvas.width !== Math.round(cssW * dpr) || canvas.height !== Math.round(cssH * dpr)) {
    canvas.width = Math.round(cssW * dpr);
    canvas.height = Math.round(cssH * dpr);
  }
  const g = canvas.getContext('2d')!;
  g.setTransform(1, 0, 0, 1, 0, 0);
  g.clearRect(0, 0, canvas.width, canvas.height);
  const pal = opts.palette;
  if (opts.background) {
    g.fillStyle = resolveColor(opts.background, pal);
    g.fillRect(0, 0, canvas.width, canvas.height);
  }
  const kind = opts.geometry ?? defaultGeometry(symbol);
  const exprs = new ExprCache();
  const compile = (k: number) => {
    const entity = sampleEntity(kind, cssW / k, cssH / k, opts.attrs ?? SAMPLE_ATTRS);
    const env: CompileEnv = {
      plotScale: 1000,
      exprs,
      run: new ExprRun(exprs, { entities: [entity], layerName: () => 'Önizleme', plotScale: 1000 }),
      assetAspect: (a) => {
        const x = opts.library.asset(a);
        return x ? x.height / x.width : 1;
      },
    };
    const geom = styledGeometry(entity);
    const out = new PrimitiveList();
    if (geom) compileSymbol(symbol, geom, { entity, index: 1 }, env, out);
    return out;
  };
  // Legend-like size: a sample of 36 × 22 mm fitted to the canvas; a point symbol is fitted by its own size.
  let k = opts.pxPerMm ?? Math.min(cssW / 36, cssH / 22);
  let out = compile(k);
  if (opts.pxPerMm === undefined && kind === 'point') {
    const reach = markerReach(out);
    if (reach > 0) {
      k = Math.min(30, Math.max(1.5, (Math.min(cssW, cssH) * 0.4) / reach));
      out = compile(k);
    }
  }

  g.setTransform(dpr, 0, 0, dpr, 0, 0);
  const view: View = { g, k, cx: cssW / 2, cy: cssH / 2, pal, opts };
  type Op = { level: number; kind: number; seq: number; draw: () => void };
  const ops: Op[] = [];
  let seq = 0;
  for (const f of out.fills) ops.push({ level: f.paint.level, kind: KIND_ORDER.fill, seq: seq++, draw: () => drawFill(view, f.paint, f.rings) });
  for (const s of out.strokes) ops.push({ level: s.style.level, kind: KIND_ORDER.stroke, seq: seq++, draw: () => drawStroke(view, s.style, s.path, s.closed) });
  for (const m of out.markers) ops.push({ level: m.style.common.level, kind: KIND_ORDER.marker, seq: seq++, draw: () => drawMarker(view, m.style, m.at, m.angle) });
  ops.sort((a, b) => a.level - b.level || a.kind - b.kind || a.seq - b.seq);
  for (const op of ops) {
    g.save();
    op.draw();
    g.restore();
  }
  return true;
}

/** How far a point symbol reaches from its point, in mm (markers, their offsets, a text's rough width). */
function markerReach(out: PrimitiveList): number {
  let r = 0;
  for (const { style: m } of out.markers) {
    if (m.common.unit === 'px') continue;
    const w = m.kind === 'text' ? m.size * 0.6 * m.text.length : m.size;
    const h = m.kind === 'shape' ? m.height || m.size : m.kind === 'text' ? m.size * TEXT_BOX : m.size;
    r = Math.max(r, Math.hypot(m.common.offset[0], m.common.offset[1]) + Math.max(w, h) / 2);
  }
  return r;
}

interface View {
  g: CanvasRenderingContext2D;
  /** CSS px per mm. */
  k: number;
  cx: number;
  cy: number;
  pal: CanvasPalette;
  opts: SymbolPreviewOptions;
}

const px = (view: View, p: Vec2): [number, number] => [view.cx + p.x * view.k, view.cy - p.y * view.k];
/** A size in its unit as CSS px. */
const len = (view: View, v: number, unit: PrimUnit) => (unit === 'px' ? v : v * view.k);
const css = (view: View, color: string, opacity = 1) => {
  const c = parseHex(resolveColor(color, view.pal));
  return `rgba(${Math.round(c[0] * 255)},${Math.round(c[1] * 255)},${Math.round(c[2] * 255)},${c[3] * opacity})`;
};
const rgba = (view: View, color: string | null): RGBA | null => (color ? parseHex(resolveColor(color, view.pal)) : null);

function tracePath(view: View, pts: readonly Vec2[], closed: boolean): void {
  pts.forEach((p, i) => {
    const [x, y] = px(view, p);
    if (i) view.g.lineTo(x, y);
    else view.g.moveTo(x, y);
  });
  if (closed) view.g.closePath();
}

function drawStroke(view: View, s: StrokeStyle, path: readonly Vec2[], closed: boolean): void {
  const g = view.g;
  g.beginPath();
  tracePath(view, path, closed);
  g.strokeStyle = css(view, s.color, s.opacity);
  g.lineWidth = Math.max(len(view, s.width, s.unit), 0.75);
  g.lineCap = s.cap;
  g.lineJoin = 'round';
  if (s.dash) {
    g.setLineDash(s.dash.map((d) => len(view, d, s.unit)));
    g.lineDashOffset = len(view, s.dashOffset, s.unit);
  }
  // A soft edge (shadow): the canvas blur spreads about twice its radius, the shaders fade over the full width.
  const blur = s.blur ? len(view, s.blur, s.unit) / 2 : 0;
  if (blur > 0.5) g.filter = `blur(${blur.toFixed(2)}px)`;
  g.stroke();
  if (blur > 0.5) g.filter = 'none';
}

function clipRings(view: View, rings: readonly (readonly Vec2[])[]): { minX: number; minY: number; maxX: number; maxY: number } {
  const g = view.g;
  g.beginPath();
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const r of rings) {
    tracePath(view, r, true);
    for (const p of r) {
      minX = Math.min(minX, p.x);
      minY = Math.min(minY, p.y);
      maxX = Math.max(maxX, p.x);
      maxY = Math.max(maxY, p.y);
    }
  }
  return { minX, minY, maxX, maxY };
}

function drawFill(view: View, paint: FillPaint, rings: readonly (readonly Vec2[])[]): void {
  const g = view.g;
  const box = clipRings(view, rings);
  if (paint.kind === 'solid') {
    g.fillStyle = css(view, paint.color, paint.opacity);
    g.fill('evenodd');
    return;
  }
  g.clip('evenodd');
  // Work in mm from here; unit px sizes become mm through k.
  const toMm = (value: number, unit: PrimUnit) => (unit === 'px' ? value / view.k : value);
  const corners = [v(box.minX, box.minY), v(box.maxX, box.minY), v(box.maxX, box.maxY), v(box.minX, box.maxY)];
  if (paint.kind === 'hatch') {
    const sp = toMm(paint.spacing, paint.unit);
    if (!(sp > 0)) return;
    const dir = v(Math.cos(paint.angle), Math.sin(paint.angle));
    const n = v(-dir.y, dir.x);
    const us = corners.map((c) => c.x * n.x + c.y * n.y);
    const ss = corners.map((c) => c.x * dir.x + c.y * dir.y);
    const off = toMm(paint.offset, paint.unit);
    const s0 = Math.min(...ss);
    const s1 = Math.max(...ss);
    g.strokeStyle = css(view, paint.color, paint.opacity);
    g.lineWidth = Math.max(toMm(paint.width, paint.unit) * view.k, 0.6);
    if (paint.dash) {
      g.setLineDash(paint.dash.map((d) => toMm(d, paint.unit) * view.k));
      // Dashes anchored to the along-line distance 0, as on the map.
      g.lineDashOffset = (s0 + toMm(paint.dashOffset, paint.unit)) * view.k;
    }
    g.beginPath();
    for (let u = Math.ceil((Math.min(...us) - off) / sp) * sp + off; u <= Math.max(...us); u += sp) {
      const a = px(view, v(n.x * u + dir.x * s0, n.y * u + dir.y * s0));
      const b = px(view, v(n.x * u + dir.x * s1, n.y * u + dir.y * s1));
      g.moveTo(a[0], a[1]);
      g.lineTo(b[0], b[1]);
    }
    g.stroke();
    return;
  }
  const size: [number, number] = [toMm(paint.size[0], paint.unit), toMm(paint.size[1], paint.unit)];
  if (!(size[0] > 0 && size[1] > 0)) return;
  const cos = Math.cos(paint.angle);
  const sin = Math.sin(paint.angle);
  const offset = [toMm(paint.offset[0], paint.unit), toMm(paint.offset[1], paint.unit)];
  // Grid frame: world = R(angle)·(q + offset).
  const toWorld = (q: Vec2) => v(cos * (q.x + offset[0]) - sin * (q.y + offset[1]), sin * (q.x + offset[0]) + cos * (q.y + offset[1]));
  const qs = corners.map((c) => v(cos * c.x + sin * c.y - offset[0], -sin * c.x + cos * c.y - offset[1]));
  const qx0 = Math.min(...qs.map((q) => q.x)) - size[0];
  const qx1 = Math.max(...qs.map((q) => q.x)) + size[0];
  const qy0 = Math.min(...qs.map((q) => q.y)) - size[1];
  const qy1 = Math.max(...qs.map((q) => q.y)) + size[1];
  if (paint.kind === 'pattern') {
    const m = paint.mark;
    const unitMm = (value: number) => toMm(value, paint.unit);
    const w = unitMm(m.size);
    const h = unitMm(m.height || m.size);
    const sw = unitMm(m.strokeWidth);
    const ext = Math.hypot(w / 2, h / 2) + sw / 2;
    const jitter = [Math.max(0, size[0] - 2 * ext) * paint.jitter, Math.max(0, size[1] - 2 * ext) * paint.jitter];
    const look = { kind: 'shape' as const, shape: m.shape as ShapeId, fill: rgba(view, m.fill), stroke: rgba(view, m.stroke), strokeWidth: 0, params: m.params };
    g.globalAlpha = paint.opacity * m.common.opacity;
    for (let row = Math.floor(qy0 / size[1]); row * size[1] <= qy1; row++) {
      const sx = paint.stagger && ((row % 2) + 2) % 2 === 1 ? size[0] / 2 : 0;
      for (let col = Math.floor((qx0 - sx) / size[0]); col * size[0] + sx <= qx1; col++) {
        const hs = hash3(col + paint.seed * 17.31, row + paint.seed * 7.73);
        if (hs[2] > paint.coverage) continue;
        const q = v((col + 0.5) * size[0] + sx + (hs[0] - 0.5) * jitter[0] + unitMm(m.common.offset[0]), (row + 0.5) * size[1] + (hs[1] - 0.5) * jitter[1] + unitMm(m.common.offset[1]));
        const [x, y] = px(view, toWorld(q));
        g.save();
        g.translate(x, y);
        g.rotate(-(paint.angle + m.common.rotation));
        g.scale(1, -1);
        drawShape(g, look, w * view.k, h * view.k, Math.max(sw * view.k, 0.6));
        g.restore();
      }
    }
    return;
  }
  drawTile(view, paint.tile, size, toWorld, [qx0, qy0, qx1, qy1], paint.angle, paint.opacity, paint.unit);
}

function drawTile(view: View, tile: TileSource, size: [number, number], toWorld: (q: Vec2) => Vec2, box: number[], angle: number, opacity: number, unit: PrimUnit): void {
  const g = view.g;
  g.globalAlpha = opacity;
  if (tile.kind === 'asset') {
    const asset = view.opts.library.asset(tile.asset);
    if (!asset) return;
    const src = asset.format === 'svg' ? svgUrl(asset.data) : asset.data;
    const img = imageOf(`a|${asset.id}|${asset.data.length}`, () => src, view.opts.onLoad);
    if (!img) return;
    for (let y = Math.floor(box[1] / size[1]); y * size[1] <= box[3]; y++)
      for (let x = Math.floor(box[0] / size[0]); x * size[0] <= box[2]; x++) {
        const [px0, py0] = px(view, toWorld(v(x * size[0], (y + 1) * size[1])));
        g.save();
        g.translate(px0, py0);
        g.rotate(-angle);
        g.drawImage(img, 0, 0, size[0] * view.k, size[1] * view.k);
        g.restore();
      }
    return;
  }
  const rows = tile.stagger ? 2 : 1;
  for (let row = Math.floor(box[1] / size[1]); row * size[1] <= box[3]; row++) {
    const sx = rows === 2 && ((row % 2) + 2) % 2 === 1 ? size[0] / 2 : 0;
    for (let col = Math.floor(box[0] / size[0]) - 1; col * size[0] <= box[2]; col++) {
      const at = toWorld(v((col + 0.5) * size[0] + sx, (row + 0.5) * size[1]));
      for (const m of tile.markers) drawMarker(view, unit === 'px' && m.common.unit === 'world' ? m : m, at, angle);
    }
  }
}

function drawMarker(view: View, m: MarkerStyle, at: Vec2, angle: number): void {
  const g = view.g;
  const u = m.common.unit;
  const [x, y] = px(view, at);
  g.translate(x, y);
  g.rotate(-(angle + m.common.rotation));
  g.translate(len(view, m.common.offset[0], u), -len(view, m.common.offset[1], u));
  g.globalAlpha *= m.common.opacity;
  const anchor = ANCHOR[m.common.anchor] ?? [0, 0];
  if (m.kind === 'shape') {
    const w = len(view, m.size, u);
    const h = len(view, m.height || m.size, u);
    g.translate(-anchor[0] * w, anchor[1] * h);
    g.scale(1, -1);
    const look = { kind: 'shape' as const, shape: m.shape as ShapeId, fill: rgba(view, m.fill), stroke: rgba(view, m.stroke), strokeWidth: 0, params: m.params };
    drawShape(g, look, w, h, Math.max(len(view, m.strokeWidth, u), m.stroke || look.shape === 'cross' || look.shape === 'x' || look.shape === 'line' || look.shape === 'arrow' || look.shape === 'chevron' || look.shape === 'arc' ? 0.75 : 0));
    return;
  }
  if (m.kind === 'text') {
    const size = len(view, m.size, u);
    const family = m.font === 'serif' ? '"Times New Roman", "Liberation Serif", serif' : m.font === 'mono' ? '"IBM Plex Mono", monospace' : m.font === 'narrow' ? '"Arial Narrow", "Liberation Sans Narrow", sans-serif' : m.font === 'sans' ? (m.weight >= 900 ? '"Arial Black", Arial, "Liberation Sans", sans-serif' : 'Arial, "Liberation Sans", sans-serif') : 'Barlow, "Segoe UI", sans-serif';
    g.font = `${m.italic ? 'italic ' : ''}${m.weight} ${size}px ${family}`;
    const w = g.measureText(m.text).width;
    g.translate(-anchor[0] * w, anchor[1] * size * TEXT_BOX);
    g.textAlign = 'center';
    g.textBaseline = 'middle';
    if (m.halo) {
      g.lineJoin = 'round';
      g.lineWidth = len(view, m.halo.width, u) * 2;
      g.strokeStyle = css(view, m.halo.color);
      g.strokeText(m.text, 0, 0);
    }
    g.fillStyle = css(view, m.color);
    g.fillText(m.text, 0, 0);
    return;
  }
  const asset = view.opts.library.asset(m.asset);
  if (!asset) {
    const w = len(view, m.size, u);
    g.strokeStyle = '#E0457B';
    g.strokeRect(-w / 2, -w / 2, w, w);
    return;
  }
  const data = asset.format === 'svg' ? applySvgParams(asset.data, m.kind === 'svg' && m.fill ? resolveColor(m.fill, view.pal) : null, m.kind === 'svg' && m.stroke ? resolveColor(m.stroke, view.pal) : null) : asset.data;
  const img = imageOf(`m|${asset.id}|${data.length}|${data.slice(-64)}`, () => (asset.format === 'svg' ? svgUrl(data) : data), view.opts.onLoad);
  if (!img) return;
  const w = len(view, m.size, u);
  const h = w * (asset.height / Math.max(asset.width, 1e-9));
  g.translate(-anchor[0] * w, anchor[1] * h);
  g.drawImage(img, -w / 2, -h / 2, w, h);
}

const ANCHOR: Record<string, readonly [number, number]> = {
  center: [0, 0],
  top: [0, 0.5],
  bottom: [0, -0.5],
  left: [-0.5, 0],
  right: [0.5, 0],
  'top-left': [-0.5, 0.5],
  'top-right': [0.5, 0.5],
  'bottom-left': [-0.5, -0.5],
  'bottom-right': [0.5, -0.5],
};
