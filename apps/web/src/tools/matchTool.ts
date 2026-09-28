import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { entitiesSet } from '../product/entitiesSet';
import type { ViewTransform } from '../viewport/Camera';
import { uidOf } from './editCommand';
import { drawSelectionBox, drawTag, strokeGeometry } from './preview';
import type { Tool, ToolPointer } from './Tool';

/**
 * Özellik kopyala (docs/adr/0140): click the object whose look is wanted,
 * then the objects that take it — one click, or a window, is one step — and
 * finish with a right click or Enter. The targets take the source's layer,
 * colour, line weight and symbol (what it has of its own: an object drawn in
 * its layer's colour gives that to its targets too); attributes and label
 * are not copied.
 *
 * It writes through `cad.entities.set`, one call for all the targets of a
 * click, as the undo step “Özellik kopyala”. A target on a locked layer, or a
 * locked layer the source lies on, is refused by the command and said.
 */
export class MatchPropertiesTool implements Tool {
  readonly id = 'matchProperties';
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private source: number | null = null;
  private hover: Entity | null = null;
  private mouse: Vec2 | null = null;
  private box: { a: Vec2; aw: Vec2; b: Vec2; bw: Vec2; dragging: boolean } | null = null;
  private readonly ctx: AppContext;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
  }

  activate(): void {
    this.refresh();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  private get sourceEntity(): Entity | null {
    return this.source === null ? null : (this.ctx.doc.get(this.source) ?? null);
  }

  private refresh(): void {
    const s = this.sourceEntity;
    this.prompt.set(
      s
        ? 'Özellik kopyala: özellikleri alacak nesnelere tıklayın ya da pencereyle seçin [Bitir (Enter)]'
        : 'Özellik kopyala: özellikleri alınacak kaynak nesneye tıklayın',
    );
    this.ctx.view.requestOverlay();
  }

  /** What a target takes from `e`, in words: "“Yol” katmanı, renk #E5484D, 0.50 mm, sembol". */
  private what(e: Entity): string {
    const layer = this.ctx.doc.layers.get(e.layerId)?.name ?? e.layerId;
    return [`“${layer}” katmanı`, e.color ? `renk ${e.color}` : 'katman rengi', e.lineWeight !== undefined ? `${e.lineWeight.toFixed(2)} mm` : 'katman kalınlığı', e.symbol ? 'sembol' : 'katman stili'].join(', ');
  }

  pointerMove(p: ToolPointer): void {
    this.mouse = p.raw;
    if (this.box) {
      this.box.b = p.screen;
      this.box.bw = p.raw;
      if (!this.box.dragging && Math.hypot(p.screen.x - this.box.a.x, p.screen.y - this.box.a.y) > 4) this.box.dragging = true;
      if (this.box.dragging) this.hover = null;
    } else this.hover = this.ctx.view.pick(p.screen);
    this.ctx.selection.hover.set(this.hover?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    if (this.source === null) return this.pickSource(p);
    this.box = { a: p.screen, aw: p.raw, b: p.screen, bw: p.raw, dragging: false };
  }

  pointerUp(p: ToolPointer): void {
    const box = this.box;
    if (!box) return;
    this.box = null;
    if (box.dragging) {
      const { aw, bw } = box;
      const r = { minX: Math.min(aw.x, bw.x), minY: Math.min(aw.y, bw.y), maxX: Math.max(aw.x, bw.x), maxY: Math.max(aw.y, bw.y) };
      // Left to right takes what lies inside, right to left what it touches (as selecting does).
      this.apply(this.ctx.view.pickRect(r, box.b.x < box.a.x));
    } else {
      const hit = this.ctx.view.pick(p.screen);
      if (hit) this.apply([hit.id]);
      else this.ctx.log.warn('Özellik alacak bir nesneye tıklayın ya da pencereyle seçin.');
    }
  }

  private pickSource(p: ToolPointer): void {
    const e = this.ctx.view.pick(p.screen);
    if (!e) return this.ctx.log.warn('Özellikleri alınacak bir nesneye tıklayın.');
    this.source = e.id;
    this.hover = null;
    this.ctx.selection.hover.set(null);
    this.ctx.log.info(`Kaynak: ${this.what(e)}. Alacak nesnelere tıklayın; sağ tıklayınca biter.`);
    this.refresh();
  }

  /** Gives the targets the source's look: one command call, one undo step. */
  private apply(ids: number[]): void {
    const { doc, log } = this.ctx;
    const src = this.sourceEntity;
    if (!src) {
      log.warn('Kaynak nesne çizimde artık yok; yeni bir kaynak seçin.');
      this.source = null;
      return this.refresh();
    }
    const targets = ids.map((id) => doc.get(id)).filter((e): e is Entity => !!e && e.id !== src.id);
    if (!targets.length) return void log.warn('Kaynağın kendisi hedef olamaz; özellik alacak başka bir nesne seçin.');
    // A window skips what is locked and says so; a click on a locked object gets the command's own refusal.
    const editable = targets.length > 1 ? targets.filter((e) => !doc.layers.isLocked(e.layerId)) : targets;
    if (editable.length < targets.length) log.warn(`${targets.length - editable.length} nesne kilitli katmanda olduğu için atlandı.`);
    if (!editable.length) return;
    const result = doc.transact('Özellik kopyala', () =>
      entitiesSet.execute(
        { doc },
        {
          uids: editable.map((e) => uidOf(this.ctx, e)),
          layerId: src.layerId,
          color: src.color ?? null,
          lineWeight: src.lineWeight ?? null,
          symbol: src.symbol ?? null,
          operation: 'layer',
        },
      ),
    );
    if (result.status !== 'completed') return void ('error' in result && log.warn(result.error.message));
    for (const w of result.warnings) log.warn(w.message);
    const n = result.output.changed.length;
    log.success(n ? `${n} nesne kaynağın özelliklerini aldı.` : 'Hedefler zaten kaynakla aynı özelliklere sahip; bir şey değişmedi.');
  }

  confirm(): void {
    // Finished: the manager asks `cancel` before it leaves, and a tool back at its first step lets it.
    this.source = null;
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    // One step back: from the targets to the source.
    if (this.source === null) return false;
    this.source = null;
    this.hover = null;
    this.ctx.selection.hover.set(null);
    this.refresh();
    return true;
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    const src = this.sourceEntity;
    if (src) strokeGeometry(g, view, entityGeometry(src), { color: pal.accent, width: 2.5 });
    if (this.box?.dragging) drawSelectionBox(g, this.box.a, this.box.b, pal.snap);
    if (!this.mouse || this.box?.dragging) return;
    if (!src) {
      if (this.hover) drawTag(g, view.worldToScreen(this.mouse), ['Kaynak: bu nesne', 'Tıklayın: seç'], pal.accent, pal.labelHalo);
      return;
    }
    if (this.hover && this.hover.id !== src.id) {
      strokeGeometry(g, view, entityGeometry(this.hover), { color: pal.snap, dash: [5, 3], width: 2 });
      const layer = this.ctx.doc.layers.get(src.layerId)?.name ?? src.layerId;
      drawTag(g, view.worldToScreen(this.mouse), [`Katman: ${layer}`, 'Tıklayın: özellikleri ver'], pal.accent, pal.labelHalo);
    }
  }
}
