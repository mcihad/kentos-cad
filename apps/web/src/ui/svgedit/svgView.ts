import type { NodeRef } from '../../style/svg/nodeOps';
import type { Pt } from '../../style/svg/pathData';
import type { SnapKind } from '../../style/svg/snapping';
import type { SvgDoc } from '../../style/svg/svgModel';

/**
 * What the SVG editor's canvas and its tool modules share: the tool ids,
 * the canvas options, what the canvas asks of the editor (`CanvasHost`)
 * and what a tool module asks of the canvas (`CanvasView`).
 */

export type ToolId = 'select' | 'node' | 'rect' | 'ellipse' | 'polygon' | 'line' | 'pen' | 'text' | 'measure';

export interface CanvasOptions {
  grid: number;
  snapGrid: boolean;
  /** Snapping to shapes, guides and the canvas at all (the kinds below choose which). */
  snapObjects: boolean;
  snapKinds: readonly SnapKind[];
  rulers: boolean;
  tile: boolean;
  sides: number;
  star: boolean;
  /** Preview colours: the symbol's colour, its second colour, the paper. */
  ink: string;
  /** The ink follows the theme until a colour is picked. */
  inkAuto: boolean;
  second: string;
  paper: string;
}

/** Everything but the box middles, which crowd small drawings. */
export const DEFAULT_SNAPS: readonly SnapKind[] = ['cusp', 'smooth', 'mid', 'intersection', 'bboxCorner', 'bboxCentre', 'centre', 'perpendicular', 'tangent', 'guide', 'page'];

export function defaultOptions(doc: SvgDoc, ink: string, paper: string): CanvasOptions {
  return { grid: Math.max(1, Math.round(doc.width / 20)), snapGrid: true, snapObjects: true, snapKinds: DEFAULT_SNAPS, rulers: true, tile: false, sides: 6, star: false, ink, inkAuto: true, second: '#2B83BA', paper };
}

export interface CanvasHost {
  readonly doc: SvgDoc;
  readonly selection: ReadonlySet<string>;
  readonly tool: ToolId;
  readonly nodeEdit: string | null;
  readonly options: CanvasOptions;
  /** Remember the drawing before an interactive change (one undo step). */
  begin(): void;
  /** The interactive change is over. */
  commit(label: string): void;
  /** Redraw after the drawing changed (during a drag). */
  changed(): void;
  select(ids: string[]): void;
  setTool(t: ToolId): void;
  editNodes(id: string | null): void;
  status(text: string, kind?: 'ok' | 'warn'): void;
  /** Draws under the drawing, on the paper (the tracing reference), in drawing units. */
  underlay?(world: SVGGElement): void;
  /** The view's scale changed (fit, zoom buttons, wheel): the zoom label follows. */
  zoomed?(scale: number): void;
  /** The chosen nodes changed (the node panel follows). */
  nodesChanged?(): void;
}

/** What tool modules (nodes, rulers, measure) use of the canvas. */
export interface CanvasView {
  readonly host: CanvasHost;
  readonly scale: number;
  readonly stage: HTMLElement;
  toDoc(e: { clientX: number; clientY: number }): Pt;
  toScreen(p: Pt): Pt;
  /** A point snapped as the options say; `from` is where the drag started (perpendicular, tangent). */
  snap(p: Pt, o?: SnapOptions): Pt;
  render(): void;
}

export interface SnapOptions {
  /** Shapes moving with the pointer. */
  exclude?: ReadonlySet<string>;
  /** Nodes moving with the pointer. */
  nodes?: { shape: string; refs: readonly NodeRef[] };
  from?: Pt | null;
  /** No grid fallback (a handle, a measure point). */
  noGrid?: boolean;
  /** A guide being dragged (never snaps to itself). */
  guide?: string;
}

const SVGNS = 'http://www.w3.org/2000/svg';

export const el = (tag: string, attrs: Record<string, string | number> = {}): SVGElement => {
  const e = document.createElementNS(SVGNS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  return e as SVGElement;
};

/** A number for labels: at most three decimals, no trailing zeros. */
export const fmtNum = (v: number, digits = 3) => {
  const s = (Math.round(v * 10 ** digits) / 10 ** digits).toString();
  return s === '-0' ? '0' : s;
};

/** A label in screen space with a halo (overlay text). */
export function tag(x: number, y: number, text: string, cls = 'svge__tag'): SVGElement {
  const t = el('text', { x, y, class: cls });
  t.textContent = text;
  return t;
}
