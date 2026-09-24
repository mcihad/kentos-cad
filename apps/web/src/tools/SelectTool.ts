import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { dist, type Vec2 } from '../model/geometry';
import type { Entity } from '../model/entities';
import { entityGrips, moveGrip } from '../model/ops/grips';
import type { ViewTransform } from '../viewport/Camera';
import { drawSelectionBox, drawTag, strokeGeometry, strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { constrainPoint, drawTracking, pointFromText, type Tracking } from './tracking';

const DRAG_THRESHOLD = 4;

interface GripEdit {
  id: number;
  index: number;
  origin: Vec2;
  downScreen: Vec2;
  /** Clicked without dragging: the next click places it. */
  hot: boolean;
}

/**
 * Default tool. Click picks the most specific entity; dragging draws a
 * window (left→right, fully inside) or crossing (right→left, touching) box.
 * Dragging a grip of a selected entity edits that vertex (snaps apply).
 */
export class SelectTool implements Tool {
  readonly id = 'select';
  readonly prompt = new Signal('Komut');
  readonly cursor = 'pick' as const;
  private start: { screen: Vec2; world: Vec2 } | null = null;
  private current: { screen: Vec2; world: Vec2 } | null = null;
  private dragging = false;
  private grip: GripEdit | null = null;
  private gripPoint: Vec2 | null = null;
  private tracking: Tracking | null = null;
  private lastClick: { id: number; time: number } | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  /** Double click on text or a dimension opens the inline editor. */
  private maybeEditText(id: number): boolean {
    const now = performance.now();
    const prev = this.lastClick;
    this.lastClick = { id, time: now };
    if (!prev || prev.id !== id || now - prev.time > 450) return false;
    const e = this.ctx.doc.get(id);
    if (!e || (e.kind !== 'text' && e.kind !== 'dimension')) return false;
    if (this.ctx.doc.layers.isLocked(e.layerId)) {
      this.ctx.log.warn('Kilitli katmandaki yazı düzenlenemez.');
      return true;
    }
    this.lastClick = null;
    this.ctx.view.requestTextEdit(id);
    return true;
  }

  /** Object snaps only while a grip is being moved. */
  get snaps(): boolean {
    return !!this.grip;
  }

  snapFrom(): Vec2 | null {
    return this.grip?.origin ?? null;
  }

  activeGrip(): { id: number; index: number } | null {
    return this.grip;
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  cancel(): boolean {
    if (!this.grip) return false;
    this.endGrip();
    return true;
  }

  private endGrip(): void {
    this.grip = this.gripPoint = this.tracking = null;
    this.prompt.set('Komut');
    this.ctx.view.requestOverlay();
  }

  private commitGrip(p: Vec2): void {
    const g = this.grip!;
    const { doc, log } = this.ctx;
    const e = doc.get(g.id);
    const moved = e ? moveGrip(e, g.index, p) : null;
    if (!moved) log.warn('Bu konum geçersiz bir şekil oluşturuyor; tutamaç yerinde bırakıldı.');
    else if (dist(g.origin, p) > 1e-9) doc.transact('Tutamaçla düzenle', () => doc.update(g.id, moved as Partial<Entity>));
    this.endGrip();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.grip?.hot) return this.commitGrip(this.gripPoint ?? p.world);
    const hit = this.ctx.view.gripAt(p.screen);
    if (hit) {
      const e = this.ctx.doc.get(hit.id)!;
      this.grip = { ...hit, origin: entityGrips(e)[hit.index], downScreen: p.screen, hot: false };
      this.gripPoint = this.grip.origin;
      this.ctx.selection.hover.set(null);
      this.prompt.set('Tutamaç: yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)');
      return;
    }
    this.start = { screen: p.screen, world: p.raw };
    this.current = this.start;
    this.dragging = false;
  }

  pointerMove(p: ToolPointer): void {
    if (this.grip) {
      const r = constrainPoint(this.ctx, this.grip.origin, p);
      this.gripPoint = r.point;
      this.tracking = r.tracking;
      this.ctx.view.requestOverlay();
      return;
    }
    if (this.start) {
      this.current = { screen: p.screen, world: p.raw };
      if (!this.dragging && Math.hypot(p.screen.x - this.start.screen.x, p.screen.y - this.start.screen.y) > DRAG_THRESHOLD) {
        this.dragging = true;
        this.ctx.selection.hover.set(null);
      }
      if (this.dragging) this.ctx.view.requestOverlay();
      return;
    }
    const hit = this.ctx.view.pick(p.screen);
    this.ctx.selection.hover.set(hit?.id ?? null);
  }

  pointerUp(p: ToolPointer): void {
    if (this.grip && !this.grip.hot) {
      const moved = Math.hypot(p.screen.x - this.grip.downScreen.x, p.screen.y - this.grip.downScreen.y) > DRAG_THRESHOLD;
      if (moved) this.commitGrip(this.gripPoint ?? p.world);
      else this.grip.hot = true;
      return;
    }
    if (!this.start) return;
    const { selection } = this.ctx;
    if (this.dragging && this.current) {
      const a = this.start.world;
      const b = this.current.world;
      const crossing = this.current.screen.x < this.start.screen.x;
      const ids = this.ctx.view.pickRect(
        { minX: Math.min(a.x, b.x), minY: Math.min(a.y, b.y), maxX: Math.max(a.x, b.x), maxY: Math.max(a.y, b.y) },
        crossing,
      );
      p.shift ? selection.add(ids) : selection.set(ids);
    } else {
      const hit = this.ctx.view.pick(p.screen);
      if (hit && !p.shift && this.maybeEditText(hit.id)) selection.set([hit.id]);
      else if (hit) p.shift ? selection.toggle(hit.id) : selection.set([hit.id]);
      else if (!p.shift) selection.clear();
    }
    this.start = this.current = null;
    this.dragging = false;
    this.ctx.view.requestOverlay();
  }

  input(text: string): boolean {
    if (!this.grip) return false;
    const pt = pointFromText(this.ctx, text, this.grip.origin, this.gripPoint);
    if (!pt) return false;
    this.commitGrip(pt);
    return true;
  }

  acceptPoint(p: Vec2): boolean {
    if (!this.grip) return false;
    this.commitGrip(p);
    return true;
  }

  confirm(): void {
    if (this.grip) return this.commitGrip(this.gripPoint ?? this.grip.origin);
    this.ctx.tools.repeatLast();
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.grip && this.gripPoint) {
      const pal = this.ctx.view.palette;
      const e = this.ctx.doc.get(this.grip.id);
      const moved = e ? moveGrip(e, this.grip.index, this.gripPoint) : null;
      if (moved) strokeGeometry(g, view, moved, { color: pal.accent, dash: [4, 3], width: 1.5 });
      strokePath(g, view, [this.grip.origin, this.gripPoint], { color: pal.accent, dash: [2, 3] });
      drawTag(g, view.worldToScreen(this.gripPoint), [this.ctx.format.length(dist(this.grip.origin, this.gripPoint))], pal.accent, pal.labelHalo);
      if (this.tracking) drawTracking(g, view, this.tracking, this.gripPoint, pal.accent, pal.labelHalo);
      return;
    }
    if (!this.dragging || !this.start || !this.current) return;
    drawSelectionBox(g, this.start.screen, this.current.screen, this.ctx.view.palette.snap);
  }
}

export class PanTool implements Tool {
  readonly id = 'pan';
  readonly prompt = new Signal('Kaydır: sürükleyerek görünümü taşıyın. Çıkmak için Esc');
  readonly cursor = 'grab' as const;
  readonly snaps = false;
  private last: Vec2 | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button === 0) this.last = p.screen;
  }

  pointerMove(p: ToolPointer): void {
    if (!this.last) return;
    this.ctx.view.camera.panBy(p.screen.x - this.last.x, p.screen.y - this.last.y);
    this.last = p.screen;
  }

  pointerUp(): void {
    this.last = null;
  }
}

/** Drag (or click twice) a rectangle and fit the view to it. */
export class ZoomWindowTool implements Tool {
  readonly id = 'zoomWindow';
  readonly prompt = new Signal('Pencere yakınlaştır: ilk köşeyi belirtin');
  readonly cursor = 'cross' as const;
  readonly snaps = false;
  private a: { world: Vec2; screen: Vec2 } | null = null;
  private b: Vec2 | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (!this.a) {
      this.a = { world: p.raw, screen: p.screen };
      this.prompt.set('Pencere yakınlaştır: karşı köşeyi belirtin');
    } else this.finish(p.raw);
  }

  pointerMove(p: ToolPointer): void {
    this.b = p.screen;
    if (this.a) this.ctx.view.requestOverlay();
  }

  pointerUp(p: ToolPointer): void {
    if (this.a && Math.hypot(p.screen.x - this.a.screen.x, p.screen.y - this.a.screen.y) > DRAG_THRESHOLD) this.finish(p.raw);
  }

  private finish(w: Vec2): void {
    const a = this.a!.world;
    if (Math.abs(w.x - a.x) > 1e-6 && Math.abs(w.y - a.y) > 1e-6)
      this.ctx.view.camera.fit({ minX: Math.min(a.x, w.x), minY: Math.min(a.y, w.y), maxX: Math.max(a.x, w.x), maxY: Math.max(a.y, w.y) }, 0);
    this.ctx.tools.exit();
  }

  draw(g: CanvasRenderingContext2D): void {
    if (!this.a || !this.b) return;
    const a = this.a.screen;
    const b = this.b;
    g.save();
    g.strokeStyle = this.ctx.view.palette.accent;
    g.setLineDash([5, 4]);
    g.strokeRect(Math.min(a.x, b.x) + 0.5, Math.min(a.y, b.y) + 0.5, Math.abs(b.x - a.x), Math.abs(b.y - a.y));
    g.restore();
  }
}
