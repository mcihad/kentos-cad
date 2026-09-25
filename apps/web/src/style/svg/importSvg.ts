import { svgOp } from './core';
import { shapeId, type SvgDoc } from './svgModel';
import type { XmlNode } from './svgValues';

export { parseCss, readColor, readPaint, readTransform, viewBoxTransform, type XmlNode } from './svgValues';

/**
 * An SVG file read into the editor's model. The caller parses the XML
 * (DOMParser in the UI) into plain nodes; the SVG core walks them the way
 * a browser would (crates/shared/svg-core `import.rs`): CSS `<style>` rules
 * (class, id, tag, descendant and child selectors), presentation
 * attributes and `style` with inheritance, every transform, nested
 * `<svg>`, `<defs>`/`<symbol>`/`<use>`, viewBox with units and
 * preserveAspectRatio, and all basic shapes, paths and texts. What the
 * model cannot hold is simplified and counted in the report: gradients and
 * patterns become one flat colour, clip paths, masks, filters, markers and
 * images are left out. The tree crosses flat (a deep file is no deep
 * JSON); the core names new shapes and groups "\u0001<k>" and "\u0002<k>"
 * in the order they are made, and their ids are made here.
 */

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

/** An element as it crosses: tag, attributes, text, and its parent's place in the list (document order). */
type FlatNode = [string, Record<string, string>, string | null, number];

const flatTree = (n: XmlNode, parent: number, out: FlatNode[]): FlatNode[] => {
  const at = out.length;
  out.push([n.tag, n.attrs, n.text ?? null, parent]);
  for (const k of n.children) flatTree(k, at, out);
  return out;
};

const importCore = svgOp<(nodes: FlatNode[], opts: ImportOptions) => ImportResult & { ids: number; groups: number }>('docFromSvgTree');

export function docFromSvgTree(root: XmlNode, opts: ImportOptions = {}): ImportResult {
  const r = importCore(flatTree(root, -1, []), opts);
  const ids = Array.from({ length: r.ids }, () => shapeId());
  const groups = Array.from({ length: r.groups }, () => newGroup());
  const fresh = (v: string) => (v.charCodeAt(0) === 1 ? ids[Number(v.slice(1))] : v.charCodeAt(0) === 2 ? groups[Number(v.slice(1))] : v);
  for (const s of r.doc.shapes) {
    s.id = fresh(s.id);
    if (s.group !== undefined) s.group = fresh(s.group);
  }
  return { doc: r.doc, skipped: r.skipped, report: r.report, colors: r.colors, reference: r.reference };
}

// ── Colour mapping ─────────────────────────────────────────────────────

/** The fixed colours of a drawing, most painted first. */
export const colorUsage = svgOp<(doc: SvgDoc) => ColorUse[]>('colorUsage');

/**
 * Fixed colours turned into the symbol's colours: the chosen one (or all
 * near-black ones, or the dominant one) becomes the symbol colour, another
 * the second colour. A mapped colour's alpha moves to the shape's opacity
 * when the shape has no other paint.
 */
export const mapColors = svgOp<(doc: SvgDoc, symbol: ColorTarget, second: string | null) => SvgDoc>('mapColors');

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
