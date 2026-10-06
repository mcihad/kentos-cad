import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import type { Entity, EntityGeometry } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { parseLength, parseNumber } from './coordinateInput';
import { edgeShift, edgeShiftForArea, edgeShiftPick, type EdgeShiftPicked } from './constructions';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { drawTag, strokeGeometry, strokePath } from './preview';
import type { Tool, ToolCursor, ToolPointer } from './Tool';

/**
 * Paralel kaydır (docs/adr/0191; Netcad's Paralel Kaydır and Alan Düzeltme (Paralel); the desktop's
 * `kentos_interaction::edge_shift`): an area's or a polyline's straight edge moved parallel to itself, its neighbours
 * lengthened or shortened to meet it (the core's `ops::edge_shift`). The edge is clicked; it follows the cursor (the
 * parallel line through the cursor's point, snaps on), the distance and an area's new size beside it. A click there, a
 * typed distance, or Alan (A) and a target size writes it through `cad.entities.edit` (step “Paralel kaydır”) and the
 * tool waits for the next edge. Esc and Ctrl+Z let the edge go; Esc with none leaves.
 */

export const LABEL = 'Paralel kaydır';
/** A click where no area's or polyline's edge is. */
export const NO_EDGE_HERE = 'Tıklanan yerde kaydırılacak kenar yok; kilitsiz bir alanın ya da çoklu çizginin kenarına tıklayın.';
const NOT_AREA = 'Hedef alan yalnız alanlarda yazılır.';

interface Picking {
  readonly id: number;
  readonly entity: Entity;
  readonly picked: EdgeShiftPicked;
  /** The area's size then (null: a polyline). */
  readonly area: number | null;
}

/** The cursor's distance and the object moved there, or why not. */
interface At {
  readonly p: Vec2;
  readonly d: number;
  readonly moved: Entity | null;
  readonly area: number | null;
  readonly problem: string | null;
}

const signed = (text: string, v: number): string => (v > 0 ? `+${text}` : text);

export class EdgeShiftTool implements Tool {
  readonly id = 'edgeShift';
  readonly prompt = new Signal('');
  private readonly ctx: AppContext;
  private edge: Picking | null = null;
  private asking = false;
  private at: At | null = null;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  /** The cursor snaps once the edge is picked: the edge goes through the point. */
  get snaps(): boolean {
    return this.edge !== null;
  }

  get cursor(): ToolCursor {
    return this.edge ? 'cross' : 'pick';
  }

  get pointCount(): number {
    return this.edge ? 1 : 0;
  }

  activate(): void {
    this.refresh();
  }

  private refresh(): void {
    const f = this.ctx.format;
    let step: string;
    if (this.asking) step = `hedef alanı yazın (${f.areaUnitLabel})`;
    else if (!this.edge) step = 'kaydırılacak kenara tıklayın (alan ya da çoklu çizgi)';
    else step = `uzaklığı yazın ya da yerine tıklayın (${this.edge.area !== null ? 'dışarı artı' : 'sağa artı'})${this.edge.area !== null ? ' [Alan (A)]' : ''}`;
    this.prompt.set(`${LABEL}: ${step}`);
    this.ctx.view.requestOverlay();
  }

  private editable(e: Entity): boolean {
    return (e.kind === 'polygon' || e.kind === 'polyline') && !this.ctx.doc.layers.isLocked(e.layerId);
  }

  private pickAt(p: ToolPointer): Entity | null {
    return this.ctx.view.pickEdge(p.screen, (e) => this.editable(e));
  }

  /** The edge under the click, or why none is taken. */
  private pick(p: ToolPointer): void {
    const e = this.pickAt(p);
    if (!e) return void this.ctx.log.warn(NO_EDGE_HERE);
    const got = edgeShiftPick(e, p.raw);
    if (!got.picked) return void this.ctx.log.warn(got.problem ?? NO_EDGE_HERE);
    const area = edgeShift(e, got.picked.ring, got.picked.edge, 0).area ?? null;
    this.edge = { id: e.id, entity: e, picked: got.picked, area };
    this.ctx.selection.hover.set(null);
  }

  private distanceAt(p: Vec2): number {
    const { a, normal } = this.edge!.picked;
    return (p.x - a.x) * normal.x + (p.y - a.y) * normal.y;
  }

  /** Writes the edge moved `d` and says so; the tool waits for the next edge. Whether it was written. */
  private write(d: number): boolean {
    const edge = this.edge;
    if (!edge) return false;
    const got = edgeShift(edge.entity, edge.picked.ring, edge.picked.edge, d);
    if (!got.entity) {
      this.ctx.log.warn(got.problem ?? NO_EDGE_HERE);
      return false;
    }
    const geometry = editGeometry(got.entity as EntityGeometry);
    if (!writeEdit(this.ctx, 'edgeShift', [{ kind: 'update', uid: uidOf(this.ctx, edge.id), geometry }])) return false;
    const f = this.ctx.format;
    this.ctx.log.success(
      got.area !== null && got.area !== undefined ? `${LABEL}: kenar ${f.length(d)} kaydırıldı, alan ${f.area(got.area)}.` : `${LABEL}: kenar ${f.length(d)} kaydırıldı.`,
    );
    this.edge = null;
    this.at = null;
    this.refresh();
    return true;
  }

  pointerMove(p: ToolPointer): void {
    if (!this.edge) {
      this.ctx.selection.hover.set(this.pickAt(p)?.id ?? null);
      return;
    }
    const d = this.distanceAt(p.world);
    const got = edgeShift(this.edge.entity, this.edge.picked.ring, this.edge.picked.edge, d);
    this.at = { p: p.world, d, moved: got.entity ?? null, area: got.area ?? null, problem: got.problem ?? null };
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0 || this.asking) return;
    if (!this.edge) this.pick(p);
    else this.write(this.distanceAt(p.world));
    this.refresh();
  }

  input(text: string): boolean {
    const t = text.trim();
    const edge = this.edge;
    if (!edge) return false;
    if (this.asking) {
      const n = parseNumber(t);
      if (n !== null && n > 0 && Number.isFinite(n)) {
        const got = edgeShiftForArea(edge.entity, edge.picked.ring, edge.picked.edge, this.ctx.format.areaToSquareMetres(n));
        if (got.distance !== null && got.distance !== undefined) {
          this.asking = false;
          this.write(got.distance);
        } else this.ctx.log.warn(got.problem ?? NOT_AREA);
      } else this.ctx.log.warn(`Hedef alan sıfırdan büyük bir sayı olmalı; “${t}” yazıldı.`);
      this.refresh();
      return true;
    }
    if (t.toLocaleUpperCase('tr-TR') === 'A') {
      if (edge.area !== null) this.asking = true;
      else this.ctx.log.warn(NOT_AREA);
      this.refresh();
      return true;
    }
    const d = parseLength(this.ctx.format, t);
    if (d === null || !Number.isFinite(d)) return false;
    this.write(d);
    this.refresh();
    return true;
  }

  /** Enter: Alan's question ends; else the tool leaves (a click or a typed distance writes). */
  confirm(): void {
    if (this.asking) {
      this.asking = false;
      return this.refresh();
    }
    this.ctx.tools.exit();
  }

  /** Esc: Alan's question, then the edge go first. */
  cancel(): boolean {
    if (this.asking) {
      this.asking = false;
      this.refresh();
      return true;
    }
    return this.undoStep();
  }

  /** Ctrl+Z: the edge goes; with none the drawing is undone. */
  undoStep(): boolean {
    this.asking = false;
    this.at = null;
    if (!this.edge) return false;
    this.edge = null;
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const edge = this.edge;
    if (!edge) return;
    const pal = this.ctx.view.palette;
    const f = this.ctx.format;
    strokePath(g, view, [edge.picked.a, edge.picked.b], { color: pal.snap, width: 2 });
    const at = this.at;
    if (!at) return;
    const lines = [f.length(at.d)];
    let tone = pal.accent;
    if (at.moved) {
      strokeGeometry(g, view, at.moved as EntityGeometry, { color: pal.accent, dash: [6, 4] });
      if (edge.area !== null && at.area !== null) lines.push(`${f.area(at.area)} (${signed(f.area(at.area - edge.area, false), at.area - edge.area)})`);
    } else if (at.problem) {
      lines.push(at.problem);
      tone = pal.danger;
    }
    drawTag(g, view.worldToScreen(at.p), lines, tone, pal.labelHalo);
  }
}
