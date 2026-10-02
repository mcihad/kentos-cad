import type { AppContext } from '../app/context';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import { Signal } from '../core/signal';
import type { Entity } from '../model/entities';
import { dist, type Vec2 } from '../model/geometry';
import { entityGrips, moveGrip } from '../model/ops/grips';
import { withoutElevations } from '../product/elevation';
import { geometryOf } from '../product/entitiesEdit';
import { gripElevation, hasVertexElevation, nearestVertex } from '../product/elevationValues';
import { writeEdit } from './editCommand';
import { neighbours, sayNeighbours } from './neighbours';
import type { ViewTransform } from '../viewport/Camera';
import { GRIP_HIT_PX } from '../viewport/overlay';
import { drawSelectionBox, drawTag, strokeGeometry, strokePath } from './preview';
import type { Tool, ToolPointer } from './Tool';
import { constrainPoint, drawTracking, pointFromText, type Tracking } from './tracking';

const DRAG_THRESHOLD = 4;

interface GripEdit {
  id: number;
  /** The object's persistent id: the command names it by that. */
  uid: string;
  index: number;
  origin: Vec2;
  downScreen: Vec2;
  /** Clicked without dragging: the next click places it. */
  hot: boolean;
}

/**
 * Default tool. Click picks the most specific entity; dragging draws a
 * window (left→right, fully inside) or crossing (right→left, touching) box.
 * Dragging a grip of a selected entity edits that vertex (snaps apply); with
 * Topolojik düzenleme on, the shared corners and edges of the objects around
 * it go with it in the same step (docs/adr/0160, `neighbours`).
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
  /** Where the pointer rests over the drawing, for the tag of a grip that has an elevation. */
  private hoverScreen: Vec2 | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  /** Double click on text, a dimension or a leader (its note, docs/adr/0146 §7) opens the inline editor. */
  private maybeEditText(id: number): boolean {
    const now = performance.now();
    const prev = this.lastClick;
    this.lastClick = { id, time: now };
    if (!prev || prev.id !== id || now - prev.time > 450) return false;
    const e = this.ctx.doc.get(id);
    if (!e || (e.kind !== 'text' && e.kind !== 'dimension' && e.kind !== 'leader')) return false;
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
    this.hoverScreen = null;
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

  /**
   * Writes the moved grip through the product command `cad.entities.edit`,
   * operation `grip` (step “Tutamaçla düzenle”): the object by its persistent
   * id, its whole new geometry explicit (TODOS.md CMD-07). The command's
   * refusal is said, as a layer locked while the grip waited.
   */
  private commitGrip(p: Vec2): void {
    const g = this.grip!;
    const { doc, log } = this.ctx;
    const e = doc.byUid(g.uid);
    const moved = e ? moveGrip(e, g.index, p) : null;
    if (!e) log.warn('Tutamacın nesnesi artık çizimde yok (silinmiş ya da geri alınmış); tutamaç bırakıldı.');
    else if (!moved) log.warn('Bu konum geçersiz bir şekil oluşturuyor; tutamaç yerinde bırakıldı.');
    else if (dist(g.origin, p) > 1e-9) {
      // The neighbours sharing what moved go in the same step (docs/adr/0160 §6).
      const follow = neighbours(this.ctx, e, moved);
      // No elevations go with it: the moved vertex keeps its own by place, as every other does (docs/adr/0142, 0143).
      const geometry = withoutElevations(geometryOf(moved as unknown as EditGeometry)) as unknown as EditGeometry;
      if (writeEdit(this.ctx, 'grip', [{ kind: 'update', uid: g.uid, geometry }, ...(follow?.changes ?? [])])) sayNeighbours(this.ctx, follow);
    }
    this.endGrip();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    this.hoverScreen = null;
    if (this.grip?.hot) return this.commitGrip(this.gripPoint ?? p.world);
    const hit = this.ctx.view.gripAt(p.screen);
    if (hit) {
      const e = this.ctx.doc.get(hit.id)!;
      this.grip = { ...hit, uid: this.ctx.doc.uidOf(hit.id) ?? '', origin: entityGrips(e)[hit.index], downScreen: p.screen, hot: false };
      this.gripPoint = this.grip.origin;
      this.ctx.selection.hover.set(null);
      this.prompt.set('Tutamaç: yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)');
      return;
    }
    this.start = { screen: p.screen, world: p.raw };
    this.current = this.start;
    this.dragging = false;
  }

  /**
   * The tag of the grip under `at` when the vertex it stands on has an elevation (docs/adr/0142): `Kot 105.250 m`
   * beside the pointer. A vertex without one has none, and neither has a mid grip or a grip of another kind. The
   * nearest vertex of the selected objects within the grips' aperture is the one (as `gripAt` finds a grip), looked
   * for among the vertices themselves: `gripAt` builds every grip of the selection, which the pointer must not
   * cost on each move over a selection of contours.
   */
  private gripTagAt(at: Vec2): string | null {
    const { doc, selection, view, format } = this.ctx;
    // A selection too big to have grips (`gripAt`'s limit) has no tags.
    if (selection.size === 0 || selection.size > 150) return null;
    const editable: Entity[] = [];
    for (const id of selection.ids.value) {
      const e = doc.get(id);
      if (e && !doc.layers.isLocked(e.layerId)) editable.push(e);
    }
    // A selection with no elevation costs nothing more.
    if (!editable.some(hasVertexElevation)) return null;
    // In metres, at the pointer: no vertex of the selection is brought to the screen.
    const camera = view.camera;
    const where = camera.screenToWorld(at);
    let reach = GRIP_HIT_PX / camera.scale;
    let z: number | null = null;
    for (const e of editable) {
      const v = nearestVertex(e, where, reach);
      if (v) {
        reach = v.d;
        z = v.z;
      }
    }
    return z === null ? null : `Kot ${format.length(z)}`;
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
    this.hoverScreen = p.screen;
    // On a grip that has a tag the pointer is on the vertex, not on the object: no highlight, and no card over the tag.
    const hit = this.gripTagAt(p.screen) === null ? this.ctx.view.pick(p.screen) : null;
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
      // The object and the neighbours that follow it (docs/adr/0160 §5), alike.
      const follow = e && moved ? (neighbours(this.ctx, e, moved)?.shapes ?? []) : [];
      for (const shape of moved ? [moved, ...follow] : []) strokeGeometry(g, view, shape, { color: pal.accent, dash: [4, 3], width: 1.5 });
      strokePath(g, view, [this.grip.origin, this.gripPoint], { color: pal.accent, dash: [2, 3] });
      // The vertex keeps its elevation as it moves (docs/adr/0142): the tag says which.
      const z = e ? gripElevation(e, this.grip.index) : null;
      const lines = [this.ctx.format.length(dist(this.grip.origin, this.gripPoint)), ...(z === null ? [] : [`Kot ${this.ctx.format.length(z)}`])];
      drawTag(g, view.worldToScreen(this.gripPoint), lines, pal.accent, pal.labelHalo);
      if (this.tracking) drawTracking(g, view, this.tracking, this.gripPoint, pal.accent, pal.labelHalo);
      return;
    }
    if (this.start) {
      if (this.dragging && this.current) drawSelectionBox(g, this.start.screen, this.current.screen, this.ctx.view.palette.snap);
      return;
    }
    // The pointer left the drawing: no tag.
    const at = this.ctx.view.cursorWorld.value ? this.hoverScreen : null;
    const tag = at && this.gripTagAt(at);
    if (at && tag) {
      const pal = this.ctx.view.palette;
      drawTag(g, at, [tag], pal.accent, pal.labelHalo);
    }
  }
}

export class PanTool implements Tool {
  readonly id = 'pan';
  readonly prompt = new Signal('Kaydır: sürükleyerek görünümü taşıyın. Çıkmak için Esc');
  readonly cursor = 'grab' as const;
  readonly snaps = false;
  private last: Vec2 | null = null;
  /** Whether the drag has kept the view it leaves (Önceki görünüm, docs/adr/0141). */
  private kept = false;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    this.last = p.screen;
    this.kept = false;
  }

  pointerMove(p: ToolPointer): void {
    if (!this.last) return;
    // The start of a pan keeps the view it leaves: at the drag's first move, so a click that goes nowhere keeps nothing.
    if (!this.kept) {
      this.kept = true;
      this.ctx.view.rememberView();
    }
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
      this.ctx.view.zoomToBox({ minX: Math.min(a.x, w.x), minY: Math.min(a.y, w.y), maxX: Math.max(a.x, w.x), maxY: Math.max(a.y, w.y) });
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
