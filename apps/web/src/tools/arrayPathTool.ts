import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { Affine } from '../model/geom/affine';
import { pathOf } from '../model/ops/path';
import { pathArrayTransforms } from '../model/ops/transform';
import type { ViewTransform } from '../viewport/Camera';
import { parseNumber } from './coordinateInput';
import { SelectionFirstTool } from './modifyTools';
import { strokeGeometry } from './preview';
import { entityGeometry } from '../model/entities';
import type { ToolPointer } from './Tool';

/** Places past this are refused by the command. */
const MAX_PLACES = 10_000;
const NONE = 1e-9;

/** What Yol boyunca dizi follows: a line, an arc, a circle or a polyline. */
const isPath = (e: Entity) => e.kind === 'line' || e.kind === 'arc' || e.kind === 'circle' || e.kind === 'polyline';

/**
 * Yol boyunca dizi (docs/adr/0140): copies of the objects along a path. Objects are
 * picked before or after; then the path (a line, arc, circle or polyline; a click, any
 * layer, it is only read); then the count (Adet, default 5, typed) or, with `A` Aralık,
 * a spacing (typed; as many copies as fit the path); `H` Hizala turns the copies with
 * the path (default on). The copies are previewed live; Enter (or right click, or a
 * click in the drawing; the Uygula chip) writes them through `cad.entities.array`, layout `path`, one
 * undo step, “Yol boyunca dizi”. Where the copies go is the shared core's
 * (`pathArrayTransforms`). Esc steps back: the path, then the objects.
 */
export class PathArrayTool extends SelectionFirstTool {
  readonly id = 'arrayPath';
  protected readonly label = 'Yol boyunca dizi';
  private static last = { count: 5, spacing: 10, bySpacing: false, align: true };
  /** The path picked. */
  private path: Entity | null = null;
  /** The path under the cursor, while it is being picked. */
  private over: Entity | null = null;

  protected begin(): void {
    this.path = null;
    this.over = null;
  }

  // ── The places ────────────────────────────────────────────────────────

  /** How many places the path holds at the kept value: the count, or as many as the spacing fits. */
  private places(path: Entity): number | 'none' {
    const l = PathArrayTool.last;
    if (!l.bySpacing) return l.count;
    const p = pathOf(path);
    if (!p || !(p.length > NONE)) return 'none';
    const ratio = p.length / l.spacing;
    const n = p.closed ? Math.ceil(ratio - 1e-9) : Math.floor(ratio + 1e-9) + 1;
    return n >= 2 ? n : 'none';
  }

  private layoutFor(path: Entity, uid: string): { kind: 'path'; path: string; count: number; spacing?: number; align: boolean } | null {
    const count = this.places(path);
    if (count === 'none') return null;
    const l = PathArrayTool.last;
    return { kind: 'path', path: uid, count, ...(l.bySpacing && { spacing: l.spacing }), align: l.align };
  }

  private maps(): Affine[] {
    const path = this.path;
    if (!path) return [];
    const count = this.places(path);
    if (count === 'none' || count > MAX_PLACES) return [];
    const l = PathArrayTool.last;
    return pathArrayTransforms(path, count, l.bySpacing ? l.spacing : null, l.align) ?? [];
  }

  // ── Prompt ────────────────────────────────────────────────────────────

  protected stagePrompt(): string {
    const n = this.ctx.selection.size;
    if (!this.path) return `${n} nesne için yolu seçin: çizgiye, yaya, daireye ya da çoklu çizgiye tıklayın`;
    const l = PathArrayTool.last;
    const f = this.ctx.format;
    const align = `Hizala (H): ${l.align ? 'açık' : 'kapalı'}`;
    const places = this.places(this.path);
    if (l.bySpacing) return `aralığı yazın [aralık ${f.length(l.spacing)}${typeof places === 'number' ? `; ${places} adet` : ''}; Adet (N) / ${align} / Uygula (Enter)]`;
    return `adedi yazın [${l.count} adet; Aralık (A) / ${align} / Uygula (Enter)]`;
  }

  // ── Pointer ───────────────────────────────────────────────────────────

  override pointerMove(p: ToolPointer): void {
    if (this.picking || this.path) return super.pointerMove(p);
    this.over = this.ctx.view.pickEdge(p.screen, isPath);
    this.ctx.selection.hover.set(this.over?.id ?? null);
    this.hover = p.raw;
    this.ctx.view.requestOverlay();
  }

  override pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.picking) return super.pointerDown(p);
    if (!this.path) {
      const e = this.ctx.view.pickEdge(p.screen, isPath);
      if (!e) return void this.ctx.log.warn('Yol olarak bir çizgiye, yaya, daireye ya da çoklu çizgiye tıklayın.');
      if (this.ctx.selection.has(e.id)) return void this.ctx.log.warn('Yol, çoğaltılacak nesnelerden biri olamaz; başka bir çizgi, yay, daire ya da çoklu çizgi seçin.');
      const length = pathOf(e)?.length ?? 0;
      if (!(length > NONE)) return void this.ctx.log.warn('Bu nesnenin uzunluğu yok; başka bir yol seçin.');
      this.path = e;
      this.ctx.selection.hover.set(null);
      this.refresh();
      return;
    }
    // A click in the drawing writes, as right click does.
    this.write();
  }

  protected point(_p: Vec2): void {}

  // ── Typed input, options ──────────────────────────────────────────────

  override input(text: string): boolean {
    if (this.picking || !this.path) return super.input(text);
    const t = text.trim().toLocaleUpperCase('tr-TR');
    const l = PathArrayTool.last;
    const { log } = this.ctx;
    if (t === 'A') l.bySpacing = true;
    else if (t === 'N') l.bySpacing = false;
    else if (t === 'H') l.align = !l.align;
    else {
      const n = parseNumber(text);
      if (n === null || /[,;@<]/.test(text)) return false;
      if (l.bySpacing) {
        if (!(n > 0)) {
          log.warn('Aralık sıfırdan büyük olmalı.');
          return true;
        }
        l.spacing = this.ctx.format.toMetres(n);
      } else {
        if (!Number.isInteger(n) || n < 2 || n > MAX_PLACES) {
          log.warn('Adet 2 ile 10 000 arasında bir tam sayı olmalı.');
          return true;
        }
        l.count = n;
      }
    }
    this.refresh();
    return true;
  }

  override confirm(): void {
    if (this.picking || !this.path) return super.confirm();
    this.write();
  }

  cancel(): boolean {
    // One step back: from the value to the path, from the path to the objects.
    if (this.picking) return false;
    if (this.path) this.path = null;
    else this.picking = true;
    this.over = null;
    this.ctx.selection.hover.set(null);
    this.refresh();
    return true;
  }

  private write(): void {
    const { doc, log } = this.ctx;
    const path = this.path;
    if (!path) return;
    const uid = doc.uidOf(path.id);
    const layout = uid !== undefined ? this.layoutFor(path, uid) : null;
    if (!layout) return void log.warn(`Aralık yolun boyundan (${this.ctx.format.length(pathOf(path)?.length ?? 0)}) uzun; en az iki adet gerekir. Daha küçük bir aralık yazın.`);
    if (layout.count > MAX_PLACES) return void log.warn('Aralık çok küçük; 10 000’den fazla adet oluşacak. Daha büyük bir aralık yazın.');
    const n = this.arraySelection(layout);
    if (n === null) return;
    log.success(`Yol boyunca dizi: ${layout.count} adet${layout.spacing !== undefined ? `, ${this.ctx.format.length(layout.spacing)} aralıkla` : ''}, ${n} yeni nesne.`);
    // Done: the manager asks `cancel` before it leaves, and a tool already back at picking lets it.
    this.picking = true;
    this.path = null;
    this.ctx.tools.exit();
  }

  // ── Preview ───────────────────────────────────────────────────────────

  protected override previewTransforms(): Affine[] {
    return this.maps();
  }

  protected override previewTag(): string[] {
    const path = this.path;
    if (!path) return [];
    const places = this.places(path);
    const l = PathArrayTool.last;
    if (places === 'none') return ['Aralık yola sığmıyor'];
    return [`${places} adet${l.align ? ' · hizalı' : ''}`, 'Tıklayın: uygula'];
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    const shown = this.path ?? this.over;
    if (shown) strokeGeometry(g, view, entityGeometry(shown), { color: pal.snap, width: 2.5 });
    if (this.path) super.draw(g, view);
  }
}
