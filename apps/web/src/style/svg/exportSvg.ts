import type { ReferenceSpec } from './importSvg';
import { elementOf, shapesBox, type Paint, type SvgDoc, type SvgShape } from './svgModel';

/**
 * The drawing written out (docs/STYLE.md §7): as a symbol SVG (the
 * symbol's colours stay currentColor and param(stroke), as the library
 * keeps it), as a plain SVG for other programs (colours resolved to the
 * preview colours, alpha as fill/stroke-opacity, the size in mm), or as
 * the editor's source view (one element per line with ids, names and
 * hidden shapes, and where each shape's element sits). Selection only
 * crops the canvas to the chosen shapes. Also the pixel size of a PNG
 * export and its DPI chunk. Pure.
 */

export interface SvgTextOptions {
  /** Resolve the symbol's colours to these (a plain SVG); absent keeps them as parameters. */
  colors?: { ink: string; second: string };
  /** Only these shapes, on a canvas cropped to them. */
  only?: ReadonlySet<string>;
  /** One element per line, indented. */
  pretty?: boolean;
  /** A tracing reference kept in the file (never drawn: it sits in `<defs>`). */
  reference?: ReferenceSpec | null;
}

const n = (v: number) => {
  const s = (Math.round(v * 1000) / 1000).toString();
  return s === '-0' ? '0' : s;
};
const esc = (v: string) => v.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

/** The canvas an export covers: the whole drawing, or the chosen shapes with their strokes. */
export function exportBox(doc: SvgDoc, only?: ReadonlySet<string>): { x: number; y: number; w: number; h: number } {
  if (!only) return { x: 0, y: 0, w: doc.width, h: doc.height };
  const shapes = doc.shapes.filter((s) => only.has(s.id) && !s.hidden);
  const b = shapesBox(shapes);
  if (!b) return { x: 0, y: 0, w: doc.width, h: doc.height };
  const pad = Math.max(0, ...shapes.map((s) => (s.stroke !== 'none' ? s.strokeWidth / 2 : 0)));
  return { x: b.minX - pad, y: b.minY - pad, w: Math.max(b.maxX - b.minX + 2 * pad, 1e-3), h: Math.max(b.maxY - b.minY + 2 * pad, 1e-3) };
}

interface Written {
  text: string;
  /** Character range of each shape's element. */
  spans: Map<string, [number, number]>;
}

function write(doc: SvgDoc, opts: SvgTextOptions, source: boolean): Written {
  const plain = !!opts.colors;
  const pretty = source || !!opts.pretty;
  const box = exportBox(doc, opts.only);
  const root: string[] = ['xmlns="http://www.w3.org/2000/svg"', `viewBox="${n(box.x)} ${n(box.y)} ${n(box.w)} ${n(box.h)}"`];
  if (plain && doc.sizeMm) {
    // The physical size: 1 unit = sizeMm / width mm.
    const k = doc.sizeMm / doc.width;
    root.push(`width="${n(box.w * k)}mm"`, `height="${n(box.h * k)}mm"`);
  } else root.push(`width="${n(box.w)}"`, `height="${n(box.h)}"`);
  if (!plain && doc.sizeMm) root.push(`data-size-mm="${n(doc.sizeMm)}"`);
  if (!plain && doc.background) root.push(`data-background="${esc(doc.background)}"`);
  const paint = (p: Paint): string => (!opts.colors ? (p === 'fill' ? 'currentColor' : p === 'stroke' ? 'param(stroke) #000000' : p) : p === 'fill' ? opts.colors.ink : p === 'stroke' ? opts.colors.second : p);
  let text = `<svg ${root.join(' ')}>`;
  const spans = new Map<string, [number, number]>();
  const line = (depth: number, s: string) => {
    text += pretty ? `\n${'  '.repeat(depth)}${s}` : s;
  };
  const ref = opts.reference;
  if (ref && !plain && !source) {
    const attrs = [`data-kentos="reference"`, `data-name="${esc(ref.name)}"`, `data-locked="${ref.locked ? 1 : 0}"`, `href="${esc(ref.href)}"`, `x="${n(ref.x)}"`, `y="${n(ref.y)}"`, `width="${n(ref.width)}"`, `height="${n(ref.height)}"`, `opacity="${n(ref.opacity)}"`, 'preserveAspectRatio="none"'];
    line(1, `<defs><image ${attrs.join(' ')}/></defs>`);
  }
  let open: string | undefined;
  for (const s of doc.shapes) {
    if ((s.hidden && !source) || (opts.only && !opts.only.has(s.id))) continue;
    if (s.group !== open) {
      if (open) line(1, '</g>');
      if (s.group) line(1, plain ? '<g>' : `<g data-group="${esc(s.group)}">`);
      open = s.group;
    }
    const at = text.length + (pretty ? 1 + 2 * (s.group ? 2 : 1) : 0);
    line(s.group ? 2 : 1, element(s, paint, plain, source));
    spans.set(s.id, [at, text.length]);
  }
  if (open) line(1, '</g>');
  line(0, '</svg>');
  return { text: pretty ? `${text}\n` : text, spans };
}

function element(s: SvgShape, paint: (p: Paint) => string, plain: boolean, source: boolean): string {
  const e = elementOf(s, paint);
  let attrs = Object.entries(e.attrs);
  if (plain) {
    // #RRGGBBAA is not read everywhere: the alpha goes to fill-opacity / stroke-opacity.
    const out: [string, string][] = [];
    for (const [k, v] of attrs) {
      if ((k === 'fill' || k === 'stroke') && /^#[0-9a-f]{8}$/i.test(v)) {
        out.push([k, v.slice(0, 7)], [`${k}-opacity`, n(parseInt(v.slice(7), 16) / 255)]);
      } else out.push([k, v]);
    }
    attrs = out;
  }
  if (source) {
    attrs = [['id', s.id], ...(s.name ? ([['data-name', s.name]] as [string, string][]) : []), ...attrs];
    if (s.hidden) attrs.push(['display', 'none']);
  }
  const a = attrs.map(([k, v]) => `${k}="${esc(v)}"`).join(' ');
  return e.text !== undefined ? `<${e.tag} ${a}>${esc(e.text)}</${e.tag}>` : `<${e.tag} ${a}/>`;
}

/** The drawing as an SVG file: a symbol SVG, or a plain one with `colors`. */
export function svgText(doc: SvgDoc, opts: SvgTextOptions = {}): string {
  return write(doc, opts, false).text;
}

/** The editor's source view: one element per line with ids, names and hidden shapes, and each shape's range. */
export function sourceText(doc: SvgDoc): Written {
  return write(doc, {}, true);
}

// ── PNG ────────────────────────────────────────────────────────────────

/**
 * Pixel size of a PNG export of a `w × h` canvas: a width in pixels, or a
 * DPI with the drawing's width in mm (without one, a unit is a CSS pixel,
 * 96 dpi).
 */
export function pngSize(w: number, h: number, spec: { px: number } | { dpi: number; widthMm?: number }): { width: number; height: number } {
  const clamp = (v: number) => Math.max(1, Math.min(8192, Math.round(v)));
  const width = 'px' in spec ? spec.px : ((spec.widthMm ?? (w * 25.4) / 96) / 25.4) * spec.dpi;
  return { width: clamp(width), height: clamp((width * h) / w) };
}

let crcTable: Uint32Array | null = null;
export function crc32(bytes: Uint8Array): number {
  if (!crcTable) {
    crcTable = new Uint32Array(256);
    for (let i = 0; i < 256; i++) {
      let c = i;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      crcTable[i] = c >>> 0;
    }
  }
  let c = 0xffffffff;
  for (const b of bytes) c = crcTable[(c ^ b) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

/** A PNG with its resolution recorded (a pHYs chunk after IHDR, replacing one that was there), so print programs size it right. */
export function withPngDpi(png: Uint8Array, dpi: number): Uint8Array {
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  const type = (at: number) => String.fromCharCode(png[at + 4], png[at + 5], png[at + 6], png[at + 7]);
  if (png.length < 33 || type(8) !== 'IHDR') return png;
  const ppm = Math.round(dpi / 0.0254);
  const chunk = new Uint8Array(21);
  const cv = new DataView(chunk.buffer);
  cv.setUint32(0, 9);
  chunk.set([0x70, 0x48, 0x59, 0x73], 4); // pHYs
  cv.setUint32(8, ppm);
  cv.setUint32(12, ppm);
  chunk[16] = 1; // per metre
  cv.setUint32(17, crc32(chunk.subarray(4, 17)));
  // Chunks after IHDR, without an old pHYs.
  const rest: Uint8Array[] = [];
  let at = 33;
  while (at + 12 <= png.length) {
    const len = view.getUint32(at);
    const end = at + 12 + len;
    if (type(at) !== 'pHYs') rest.push(png.subarray(at, end));
    at = end;
  }
  const out = new Uint8Array(33 + 21 + rest.reduce((s, r) => s + r.length, 0));
  out.set(png.subarray(0, 33), 0);
  out.set(chunk, 33);
  let o = 54;
  for (const r of rest) {
    out.set(r, o);
    o += r.length;
  }
  return out;
}
