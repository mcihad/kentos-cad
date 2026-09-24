import { refKey } from '../../style/svg/nodeOps';
import type { Pt } from '../../style/svg/pathData';
import { SnapIndex, type SnapHit, type SnapKind } from '../../style/svg/snapping';
import { el, tag, type CanvasHost, type SnapOptions } from './svgView';

/**
 * Snapping on the SVG editor's canvas: an index of the drawing's snap
 * points (style/svg/snapping.ts), built once per drag, the grid when no
 * point is near, and the marker drawn where the pointer snapped: the main
 * CAD's shapes (square node, triangle middle, circle centre, cross
 * crossing, right angle, tangent circle) with the kind's name beside it.
 */

const RADIUS_PX = 8;

export class Snapper {
  private readonly host: CanvasHost;
  private index: SnapIndex | null = null;
  private key = '';
  /** The last snap, drawn until the pointer moves off it. */
  hit: SnapHit | null = null;

  constructor(host: CanvasHost) {
    this.host = host;
  }

  /** Forget the index (the drawing, the guides or the options changed). */
  reset(): void {
    this.index = null;
    this.key = '';
    this.hit = null;
  }

  snap(p: Pt, zoom: number, o: SnapOptions = {}): Pt {
    const host = this.host;
    const opt = host.options;
    this.hit = null;
    if (opt.snapObjects && opt.snapKinds.length) {
      const moving = o.nodes ? new Set(o.nodes.refs.map(refKey)) : null;
      const key = `${[...(o.exclude ?? [])].join(',')}|${o.nodes ? `${o.nodes.shape}:${[...moving!].join(',')}` : ''}|${o.guide ?? ''}`;
      if (!this.index || key !== this.key) {
        this.key = key;
        this.index = new SnapIndex({
          shapes: host.doc.shapes,
          guides: o.guide ? host.doc.guides?.filter((g) => g.id !== o.guide) : host.doc.guides,
          page: host.doc,
          kinds: new Set(opt.snapKinds),
          exclude: o.exclude,
          skipNode: moving ? (shape, sub, index) => shape === o.nodes!.shape && moving.has(`${sub}:${index}`) : undefined,
        });
      }
      const hit = this.index.query(p, RADIUS_PX / zoom, o.from ?? null);
      if (hit) {
        this.hit = hit;
        return hit.p;
      }
    }
    if (o.noGrid || !opt.snapGrid || !(opt.grid > 0)) return p;
    return [Math.round(p[0] / opt.grid) * opt.grid, Math.round(p[1] / opt.grid) * opt.grid];
  }

  /** The marker of the last snap, in screen space. */
  draw(g: SVGElement, toScreen: (p: Pt) => Pt): void {
    const hit = this.hit;
    if (!hit) return;
    const [x, y] = toScreen(hit.p);
    g.append(el('path', { d: markPath(hit.kind, x, y), class: 'svge__snap' }));
    g.append(tag(x + 9, y - 9, hit.label, 'svge__tag svge__tag--snap'));
  }
}

/** The marker's outline, 5 px round the point (the shapes of DESIGN.md §8). */
function markPath(kind: SnapKind, x: number, y: number): string {
  const r = 5;
  switch (kind) {
    case 'cusp':
    case 'bboxCorner':
      return `M${x - r} ${y - r}h${2 * r}v${2 * r}h${-2 * r}Z`;
    case 'smooth':
      return `M${x} ${y - r - 1}L${x + r + 1} ${y}L${x} ${y + r + 1}L${x - r - 1} ${y}Z`;
    case 'mid':
    case 'bboxMid':
      return `M${x} ${y - r - 1}L${x + r + 1} ${y + r}H${x - r - 1}Z`;
    case 'centre':
    case 'bboxCentre':
      return `M${x - r} ${y}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0M${x - 1.5} ${y}h3`;
    case 'intersection':
      return `M${x - r} ${y - r}L${x + r} ${y + r}M${x + r} ${y - r}L${x - r} ${y + r}`;
    case 'perpendicular':
      return `M${x - r} ${y - r}V${y + r}H${x + r}M${x - r} ${y}H${x}V${y + r}`;
    case 'tangent':
      return `M${x - r} ${y}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0M${x - r - 2} ${y - r}H${x + r + 2}`;
    case 'guide':
      return `M${x - r} ${y - r}L${x + r} ${y + r}H${x - r}L${x + r} ${y - r}Z`;
    case 'page':
      return `M${x - r} ${y - r}h${2 * r}v${2 * r}h${-2 * r}ZM${x - r} ${y}H${x + r}M${x} ${y - r}V${y + r}`;
  }
}
