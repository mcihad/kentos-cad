import { entityGeometry, type Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { breakEntity } from '../model/ops/break';
import { divisionPoints, nearestS, pathOf, type Path } from '../model/ops/path';
import { insertVertex, nearestSegment, nearHole, removeVertex } from '../model/ops/vertex';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { EdgePickTool } from './edgeTools';
import { editGeometry, uidOf, writeEdit } from './editCommand';
import { drawTag, strokeGeometry } from './preview';
import type { ToolPointer } from './Tool';

// ── Kır ────────────────────────────────────────────────────────────────

/** Break: pick the object at the first point, then the second point (Enter = split at the first). */
export class BreakTool extends EdgePickTool {
  readonly id = 'break';
  override get snaps(): boolean {
    return true;
  }
  private target: { entity: Entity; p1: Vec2 } | null = null;
  private p2: Vec2 | null = null;
  protected override editable = (e: Entity) => ['line', 'polyline', 'polygon', 'arc', 'circle', 'ellipse', 'xline', 'ray'].includes(e.kind) && !this.ctx.doc.layers.isLocked(e.layerId);

  protected refresh(): void {
    this.prompt.set(
      this.target
        ? 'Kır: ikinci kırılma noktasını belirtin [Aynı noktadan böl (Enter)]'
        : 'Kır: nesneyi ilk kırılma noktasından seçin (kesişim ve uç kenetleri çalışır)',
    );
  }

  override pointerMove(p: ToolPointer): void {
    if (!this.target) return super.pointerMove(p);
    this.p2 = p.world;
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (!this.target) {
      // The object under the cursor; the (snapped) click is the first break point.
      const e = this.ctx.view.pickEdge(p.screen, this.editable) ?? (p.snap ? this.snappedEntity(p.snap.entityId) : null);
      if (!e) return this.ctx.log.warn('Kırılacak düzenlenebilir bir çizgi, çoklu çizgi, yay ya da daireye tıklayın.');
      if (this.refuseHoled(e, 'kırma')) return;
      this.target = { entity: e, p1: p.world };
      this.ctx.selection.hover.set(null);
      return this.refresh();
    }
    this.commit(p.world);
  }

  private snappedEntity(id: number): Entity | null {
    const e = this.ctx.doc.get(id);
    return e && this.editable(e) ? e : null;
  }

  private commit(p2: Vec2): void {
    const t = this.target!;
    const r = breakEntity(t.entity, t.p1, p2);
    if ('error' in r) this.ctx.log.warn(r.error);
    else if (this.replace('break', t.entity, r.pieces)) this.ctx.log.success(dist(t.p1, p2) < 1e-9 ? `Nesne bölündü: ${r.pieces.length} parça.` : 'Aradaki kısım silindi.');
    this.target = null;
    this.p2 = null;
    this.refresh();
    this.ctx.view.requestOverlay();
  }

  confirm(): void {
    if (this.target) return this.commit(this.target.p1);
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    if (!this.target) return false;
    this.target = null;
    this.refresh();
    this.ctx.view.requestOverlay();
    return true;
  }

  snapFrom(): Vec2 | null {
    return this.target?.p1 ?? null;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.target || !this.p2) return;
    const pal = this.ctx.view.palette;
    const r = breakEntity(this.target.entity, this.target.p1, this.p2);
    if ('error' in r) return;
    strokeGeometry(g, view, entityGeometry(this.target.entity), { color: pal.danger, dash: [5, 3], width: 2 });
    for (const piece of r.pieces) strokeGeometry(g, view, piece, { color: pal.accent, width: 1.5 });
  }
}

// ── Böl ────────────────────────────────────────────────────────────────

/** Places points along an object: n equal parts, or every d metres (A option). */
export class DivideTool extends EdgePickTool {
  readonly id = 'divide';
  private static parts = 4;
  private static step = 10;
  private static byStep = false;
  private target: { entity: Entity; path: Path; fromEnd: boolean } | null = null;
  protected override editable = (e: Entity) => ['line', 'polyline', 'polygon', 'arc', 'circle', 'ellipse', 'spline'].includes(e.kind);

  protected refresh(): void {
    const mode = DivideTool.byStep ? `aralık ${this.ctx.format.length(DivideTool.step)}` : `${DivideTool.parts} parça`;
    const other = DivideTool.byStep ? 'Parça sayısı (P)' : 'Aralık (A)';
    this.prompt.set(
      this.target
        ? `Böl: ${DivideTool.byStep ? 'aralığı' : 'parça sayısını'} yazın ya da Enter (${mode}) [${other}]`
        : `Böl: noktaların konacağı nesneyi seçin [${mode}; ${other}]`,
    );
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const e = this.ctx.view.pickEdge(p.screen, this.editable);
    if (!e) return this.ctx.log.warn('Çizgi, çoklu çizgi, yay, daire ya da eğriye tıklayın.');
    const path = pathOf(e);
    if (!path || path.length < 1e-9) return this.ctx.log.warn('Bu nesne bölünemez.');
    // Measuring starts from the end nearer to the click.
    this.target = { entity: e, path, fromEnd: !path.closed && nearestS(path, p.raw) > path.length / 2 };
    this.ctx.selection.hover.set(null);
    this.refresh();
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    const t = text.trim().toLocaleUpperCase('tr-TR');
    if (t === 'A' || t === 'P') {
      DivideTool.byStep = t === 'A';
      this.refresh();
      this.ctx.view.requestOverlay();
      return true;
    }
    const n = parseNumber(text);
    if (n === null || n <= 0) return false;
    if (DivideTool.byStep) DivideTool.step = n;
    else {
      if (!Number.isInteger(n) || n < 2 || n > 10_000) {
        this.ctx.log.warn('Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.');
        return true;
      }
      DivideTool.parts = n;
    }
    if (this.target) this.commit();
    else this.refresh();
    return true;
  }

  private points(): Vec2[] {
    if (!this.target) return [];
    const { entity, fromEnd } = this.target;
    // One call for all of them (up to 10 000 in the preview); an ellipse's points land on the true curve.
    return divisionPoints(entity, DivideTool.byStep ? { step: DivideTool.step } : { parts: DivideTool.parts }, fromEnd);
  }

  private commit(): void {
    const pts = this.points();
    if (!pts.length) {
      this.ctx.log.warn('Aralık nesne boyundan uzun; nokta konmadı.');
    } else if (pts.length > 10_000) {
      this.ctx.log.warn('10 000’den fazla nokta oluşacak; daha büyük bir aralık girin.');
    } else {
      const { doc } = this.ctx;
      const layerId = doc.layers.active.value;
      if (doc.layers.isLocked(layerId)) this.ctx.log.warn(`“${doc.layers.get(layerId)?.name}” katmanı kilitli; noktalar etkin katmana konur.`);
      else {
        doc.transact('Böl', () => {
          for (const p of pts) doc.add({ kind: 'point', p, layerId, color: this.ctx.settings.color.value ?? undefined, attrs: {} });
        });
        this.ctx.log.success(`${pts.length} nokta kondu.`);
      }
    }
    this.target = null;
    this.refresh();
    this.ctx.view.requestOverlay();
  }

  confirm(): void {
    if (this.target) return this.commit();
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    if (!this.target) return false;
    this.target = null;
    this.refresh();
    this.ctx.view.requestOverlay();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (!this.target) return;
    const pal = this.ctx.view.palette;
    strokeGeometry(g, view, entityGeometry(this.target.entity), { color: pal.accent, width: 1.5 });
    const pts = this.points();
    if (pts.length > 2000) return;
    g.save();
    g.strokeStyle = pal.accent;
    for (const p of pts) {
      const s = view.worldToScreen(p);
      g.beginPath();
      g.arc(s.x, s.y, 3.5, 0, Math.PI * 2);
      g.stroke();
    }
    g.restore();
  }
}

// ── Köşe ekle/sil ──────────────────────────────────────────────────────

const VERTEX_PX = 7;

/** Click near a vertex to remove it, anywhere else on an edge to add one there. */
export class VertexTool extends EdgePickTool {
  readonly id = 'vertex';
  private action: { entity: Entity; remove: number | null; seg: number; at: Vec2 } | null = null;
  protected override editable = (e: Entity) => (e.kind === 'line' || e.kind === 'polyline' || e.kind === 'polygon') && !this.ctx.doc.layers.isLocked(e.layerId);

  protected refresh(): void {
    this.prompt.set('Köşe ekle/sil: kenara tıklayın köşe eklensin, köşeye tıklayın silinsin. Çıkmak için Esc');
  }

  override pointerMove(p: ToolPointer): void {
    super.pointerMove(p);
    this.action = this.hover ? this.plan(this.hover.entity, p) : null;
  }

  private plan(e: Entity, p: ToolPointer): { entity: Entity; remove: number | null; seg: number; at: Vec2 } {
    const tol = this.ctx.view.worldTolerance(VERTEX_PX);
    if (e.kind === 'polyline' || e.kind === 'polygon') {
      const i = e.pts.findIndex((q) => dist(q, p.raw) <= tol);
      if (i >= 0) return { entity: e, remove: i, seg: -1, at: e.pts[i] };
    }
    return { entity: e, remove: null, seg: e.kind === 'line' ? 0 : nearestSegment(e, p.raw), at: p.raw };
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const e = this.ctx.view.pickEdge(p.screen, this.editable);
    if (!e) return this.ctx.log.warn('Düzenlenebilir bir çizgi, çoklu çizgi ya da kapalı alana tıklayın.');
    if (e.kind === 'polygon' && nearHole(e, p.raw)) return this.ctx.log.warn('İç halkanın köşeleri tutamaçla taşınır; köşe eklemek ya da silmek için alanı Patlat ile halkalarına ayırın.');
    const a = this.plan(e, p);
    const r = a.remove !== null ? removeVertex(e, a.remove) : insertVertex(e, a.seg, a.at);
    if ('error' in r) return this.ctx.log.warn(r.error);
    const operation = a.remove !== null ? 'vertexRemove' : 'vertexAdd';
    // The whole geometry is written (docs/adr/0047): a closed area keeps its holes, the core's path is the outer ring.
    const geometry = r.geometry.kind === 'polygon' && e.kind === 'polygon' && e.holes?.length ? { ...r.geometry, holes: e.holes } : r.geometry;
    const written =
      r.geometry.kind !== e.kind ? this.replace(operation, e, [r.geometry]) : writeEdit(this.ctx, operation, [{ kind: 'update', uid: uidOf(this.ctx, e), geometry: editGeometry(geometry) }]) !== null;
    if (written) this.ctx.log.success(a.remove !== null ? 'Köşe silindi.' : 'Köşe eklendi.');
    this.action = null;
    this.ctx.view.requestOverlay();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const a = this.action;
    if (!a) return;
    const pal = this.ctx.view.palette;
    const s = view.worldToScreen(a.at);
    g.save();
    g.lineWidth = 2;
    g.strokeStyle = a.remove !== null ? pal.danger : pal.accent;
    g.beginPath();
    if (a.remove !== null) {
      g.moveTo(s.x - 5, s.y - 5);
      g.lineTo(s.x + 5, s.y + 5);
      g.moveTo(s.x + 5, s.y - 5);
      g.lineTo(s.x - 5, s.y + 5);
    } else {
      g.moveTo(s.x - 6, s.y);
      g.lineTo(s.x + 6, s.y);
      g.moveTo(s.x, s.y - 6);
      g.lineTo(s.x, s.y + 6);
    }
    g.stroke();
    g.restore();
    drawTag(g, s, [a.remove !== null ? 'Köşeyi sil' : 'Köşe ekle'], a.remove !== null ? pal.danger : pal.accent, pal.labelHalo);
  }
}
