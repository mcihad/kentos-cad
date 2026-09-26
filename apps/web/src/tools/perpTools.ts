import type { AppContext } from '../app/context';
import type { CreateOperation } from '../contracts/generated/CreateOperation';
import { Signal } from '../core/signal';
import type { Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { closestOnEdge, type Edge } from '../model/geom/intersect';
import { sideOffsets, sidePoint } from '../model/geom/survey';
import { entityEdges } from '../model/ops/edges';
import type { ViewTransform } from '../viewport/Camera';
import { radialPoint } from './constructions';
import { parseNumber } from './coordinateInput';
import { writeObjects } from './createCommand';
import { drawTag, strokeGeometry, strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { pointFromText } from './tracking';

/**
 * Netcad "Dik in" and "Dik çık": perpendiculars to a reference line, in
 * surveying terms (dik ayak along the line from its start, dik boy square
 * to it, positive to the right). The reference is the straight segment
 * clicked; its start is the end nearer to the click. Dik in also accepts
 * an arc or circle (the perpendicular runs through the centre).
 */

type Ref = { kind: 'seg'; a: Vec2; b: Vec2; entity: Entity } | { kind: 'arc'; edge: Extract<Edge, { kind: 'arc' }>; entity: Entity };

/** The edge of `e` nearest to `p`, as a reference oriented from the end nearer to `p`. */
function refAt(e: Entity, p: Vec2, straightOnly: boolean): Ref | null {
  let best: { edge: Edge; d: number } | null = null;
  for (const edge of entityEdges(e)) {
    if (straightOnly && edge.kind !== 'seg') continue;
    const d = closestOnEdge(edge, p).d;
    if (!best || d < best.d) best = { edge, d };
  }
  if (!best) return null;
  const edge = best.edge;
  if (edge.kind === 'arc') return { kind: 'arc', edge, entity: e };
  const [a, b] = dist(edge.a, p) <= dist(edge.b, p) ? [edge.a, edge.b] : [edge.b, edge.a];
  return dist(a, b) > 1e-9 ? { kind: 'seg', a, b, entity: e } : null;
}

/** Foot of the perpendicular from p (on the unbounded line, or on the circle through the centre). */
function footOn(ref: Ref, p: Vec2): Vec2 | null {
  if (ref.kind === 'seg') {
    const o = sideOffsets(ref.a, ref.b, p);
    return o ? sidePoint(ref.a, ref.b, o.absis, 0) : null;
  }
  return radialPoint(ref.edge.c, ref.edge.r, p);
}

/** Small square at the foot showing the right angle. */
function drawRightAngle(g: CanvasRenderingContext2D, view: ViewTransform, foot: Vec2, along: Vec2, up: Vec2, color: string): void {
  const s = view.worldToScreen(foot);
  const u = unitScreen(view, foot, along);
  const v = unitScreen(view, foot, up);
  if (!u || !v) return;
  const k = 8;
  g.save();
  g.strokeStyle = color;
  g.lineWidth = 1;
  g.beginPath();
  g.moveTo(s.x + u.x * k, s.y + u.y * k);
  g.lineTo(s.x + (u.x + v.x) * k, s.y + (u.y + v.y) * k);
  g.lineTo(s.x + v.x * k, s.y + v.y * k);
  g.stroke();
  g.restore();
}

function unitScreen(view: ViewTransform, from: Vec2, to: Vec2): Vec2 | null {
  const a = view.worldToScreen(from);
  const b = view.worldToScreen(to);
  const l = Math.hypot(b.x - a.x, b.y - a.y);
  return l < 1e-9 ? null : { x: (b.x - a.x) / l, y: (b.y - a.y) / l };
}

/** Shared flow: pick the reference line first, then work against it. */
abstract class PerpendicularTool implements Tool {
  abstract readonly id: string;
  protected abstract readonly label: string;
  protected abstract readonly straightOnly: boolean;
  /** The undo step's name in the command's terms: “Dik in” or “Dik çık”. */
  protected abstract readonly operation: CreateOperation;
  readonly prompt = new Signal('');
  readonly cursor = 'cross' as const;
  protected ref: Ref | null = null;
  protected hoverEntity: Entity | null = null;
  protected hover: Vec2 | null = null;
  protected readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  get snaps(): boolean {
    return !!this.ref;
  }

  activate(): void {
    this.refresh();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  protected abstract stepPrompt(): string;

  protected refresh(): void {
    const kinds = this.straightOnly ? 'çizgi ya da çoklu çizgi kenarı' : 'çizgi, çoklu çizgi kenarı, yay ya da daire';
    this.prompt.set(`${this.label}: ${this.ref ? `${this.stepPrompt()} [Başka hat (H)]` : `referans hatta tıklayın (${kinds})`}`);
    this.ctx.view.requestOverlay();
  }

  pointerMove(p: ToolPointer): void {
    if (!this.ref) {
      this.hoverEntity = this.ctx.view.pickEdge(p.screen);
      this.ctx.selection.hover.set(this.hoverEntity?.id ?? null);
      return;
    }
    this.hover = p.world;
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.ref) return this.point(p.world);
    const e = this.ctx.view.pickEdge(p.screen);
    const ref = e && refAt(e, p.raw, this.straightOnly);
    if (!ref) return this.ctx.log.warn(this.straightOnly ? 'Düz bir kenara tıklayın: çizgi ya da çoklu çizgi.' : 'Bir çizgiye, çoklu çizgi kenarına, yaya ya da daireye tıklayın.');
    this.ref = ref;
    this.ctx.selection.hover.set(null);
    this.picked();
    this.refresh();
  }

  /** Called once a reference line is chosen. */
  protected picked(): void {}
  protected abstract point(p: Vec2): void;

  input(text: string): boolean {
    if (text.trim().toLocaleUpperCase('tr-TR') === 'H') {
      this.ref = null;
      this.refresh();
      return true;
    }
    if (!this.ref) return false;
    const pt = pointFromText(this.ctx, text, null, this.hover);
    if (!pt) return false;
    this.point(pt);
    return true;
  }

  acceptPoint(p: Vec2): boolean {
    if (!this.ref) return false;
    this.point(p);
    return true;
  }

  confirm(): void {
    this.ctx.tools.exit();
  }

  /**
   * The perpendicular, written through `cad.entities.create` (docs/adr/0057)
   * as one undo step named after the tool. `short`: it has no length and
   * nothing was written; `refused`: the command said why.
   */
  protected addLine(a: Vec2, b: Vec2): 'written' | 'short' | 'refused' {
    if (dist(a, b) < 1e-9) return 'short';
    return writeObjects(this.ctx, [{ kind: 'line', a, b }], this.operation) ? 'written' : 'refused';
  }

  protected drawRef(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const ref = this.ref;
    if (!ref) return;
    const pal = this.ctx.view.palette;
    if (ref.kind === 'arc') return strokeGeometry(g, view, { kind: 'circle', c: ref.edge.c, r: ref.edge.r }, { color: pal.snap, dash: [4, 4] });
    // The line runs on past its ends: the foot may fall on the extension.
    const far = (Math.hypot(view.width, view.height) / view.scale) * 2;
    const l = dist(ref.a, ref.b);
    const ux = (ref.b.x - ref.a.x) / l;
    const uy = (ref.b.y - ref.a.y) / l;
    strokePath(g, view, [{ x: ref.a.x - ux * far, y: ref.a.y - uy * far }, { x: ref.a.x + ux * far, y: ref.a.y + uy * far }], { color: pal.snap, dash: [4, 4] });
    strokePath(g, view, [ref.a, ref.b], { color: pal.snap, width: 2 });
    const s = view.worldToScreen(ref.a);
    g.save();
    g.fillStyle = pal.snap;
    g.font = '600 11px Barlow, system-ui, sans-serif';
    g.fillText('A', s.x + 6, s.y - 6);
    g.restore();
  }
}

// ── Dik in ─────────────────────────────────────────────────────────────

/** Perpendiculars dropped from points onto the reference (each click one line, foot on the line). */
export class PerpendicularInTool extends PerpendicularTool {
  readonly id = 'perpIn';
  protected readonly label = 'Dik in';
  protected readonly straightOnly = false;
  protected readonly operation = 'perpendicularIn';

  protected stepPrompt(): string {
    return 'dik inilecek noktaları gösterin, bitince sağ tıklayın';
  }

  protected point(p: Vec2): void {
    const foot = this.ref && footOn(this.ref, p);
    if (!foot) return this.ctx.log.warn('Bu noktadan dik inilemez (nokta dairenin merkezinde).');
    const added = this.addLine(p, foot);
    if (added === 'short') return this.ctx.log.warn('Nokta zaten hattın üzerinde.');
    if (added === 'written') this.ctx.log.success(`Dik inildi: dik boy ${this.ctx.format.length(dist(p, foot))}.`);
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    this.drawRef(g, view);
    const ref = this.ref;
    const p = this.hover;
    if (!ref || !p) return;
    const foot = footOn(ref, p);
    if (!foot) return;
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    strokePath(g, view, [p, foot], { color: pal.accent, width: 1.5 });
    const lines = [`Dik boy ${f.length(dist(p, foot))}`];
    if (ref.kind === 'seg') {
      const o = sideOffsets(ref.a, ref.b, p)!;
      lines.push(`Dik ayak ${f.length(o.absis)}`);
      drawRightAngle(g, view, foot, ref.b, p, pal.accent);
    }
    drawTag(g, view.worldToScreen(p), lines, pal.accent, pal.labelHalo);
  }
}

// ── Dik çık ────────────────────────────────────────────────────────────

/** From a point on the reference (dik ayak), a perpendicular of the given length (dik boy, + right). */
export class PerpendicularOutTool extends PerpendicularTool {
  readonly id = 'perpOut';
  protected readonly label = 'Dik çık';
  protected readonly straightOnly = true;
  protected readonly operation = 'perpendicularOut';
  private absis: number | null = null;

  protected override picked(): void {
    this.absis = null;
  }

  protected stepPrompt(): string {
    return this.absis === null
      ? 'dik çıkılacak yeri hat üzerinde gösterin ya da A ucundan uzaklığı (dik ayak) yazın'
      : 'dik boyu fareyle gösterin ya da yazın (sağa artı, sola eksi)';
  }

  override input(text: string): boolean {
    const n = parseNumber(text);
    if (this.ref?.kind === 'seg' && n !== null && !/[,;@<]/.test(text)) {
      if (this.absis === null) {
        this.absis = n;
        this.refresh();
      } else this.place(n);
      return true;
    }
    return super.input(text);
  }

  protected point(p: Vec2): void {
    const ref = this.ref;
    if (ref?.kind !== 'seg') return;
    const o = sideOffsets(ref.a, ref.b, p)!;
    if (this.absis === null) {
      this.absis = o.absis;
      return this.refresh();
    }
    this.place(o.ordinat);
  }

  private place(ordinat: number): void {
    const ref = this.ref;
    if (ref?.kind !== 'seg' || this.absis === null) return;
    const foot = sidePoint(ref.a, ref.b, this.absis, 0)!;
    const end = sidePoint(ref.a, ref.b, this.absis, ordinat)!;
    const added = this.addLine(foot, end);
    if (added === 'short') return this.ctx.log.warn('Dik boy sıfır olamaz.');
    if (added === 'refused') return;
    const f = this.ctx.format;
    this.ctx.log.success(`Dik çıkıldı: dik ayak ${f.length(this.absis)}, dik boy ${f.length(ordinat)}.`);
    // Next perpendicular on the same line.
    this.absis = null;
    this.refresh();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    this.drawRef(g, view);
    const ref = this.ref;
    const p = this.hover;
    if (ref?.kind !== 'seg' || !p) return;
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    const o = sideOffsets(ref.a, ref.b, p)!;
    const absis = this.absis ?? o.absis;
    const foot = sidePoint(ref.a, ref.b, absis, 0)!;
    if (this.absis === null) {
      strokePath(g, view, [ref.a, foot], { color: pal.accent, width: 2 });
      drawTag(g, view.worldToScreen(p), [`Dik ayak ${f.length(absis)}`], pal.accent, pal.labelHalo);
      return;
    }
    const end = sidePoint(ref.a, ref.b, absis, o.ordinat)!;
    strokePath(g, view, [foot, end], { color: pal.accent, width: 1.5 });
    drawRightAngle(g, view, foot, ref.b, end, pal.accent);
    drawTag(g, view.worldToScreen(p), [`Dik ayak ${f.length(absis)}`, `Dik boy ${f.length(o.ordinat)} (${o.ordinat >= 0 ? 'sağ' : 'sol'})`], pal.accent, pal.labelHalo);
  }
}

