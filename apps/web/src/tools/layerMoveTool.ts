import type { AppContext } from '../app/context';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import type { NewObject } from '../contracts/generated/NewObject';
import type { Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import { entitiesCreate } from '../product/entitiesCreate';
import { geometryOf } from '../product/entitiesEdit';
import { entitiesSet } from '../product/entitiesSet';
import type { ViewTransform } from '../viewport/Camera';
import { uidOf } from './editCommand';
import { SelectionFirstTool } from './modifyTools';
import { drawTag } from './preview';
import type { ToolPointer } from './Tool';

/**
 * Katmanı eşle and Katmana kopyala (docs/adr/0177 §2; the desktop's `kentos_interaction::layer_move`): the selected
 * objects (picked first when nothing is selected) go to, or are copied to, a target layer: the layer of an object
 * clicked, or the active one (Etkin katman, E).
 *
 * One undo step named after the tool: Katmanı eşle through `cad.entities.set`'s layer, Katmana kopyala through
 * `cad.entities.create` (the copies in place with every property, as Özgün koordinatlara yapıştır writes them), the
 * copies selected. Objects on a locked layer are left out and counted; a locked target refuses; the commands' own
 * warnings (a hidden target) are said.
 */
export class LayerMoveTool extends SelectionFirstTool {
  readonly id: string;
  protected readonly label: string;
  override readonly snaps = false;
  private readonly copy: boolean;
  /** The object under the cursor while the target is asked for. */
  private target: Entity | null = null;

  constructor(ctx: AppContext, copy: boolean) {
    super(ctx);
    this.copy = copy;
    this.id = copy ? 'copyToLayer' : 'layerMatch';
    this.label = copy ? 'Katmana kopyala' : 'Katmanı eşle';
  }

  protected begin(): void {
    this.target = null;
  }

  protected stagePrompt(): string {
    return 'hedef katmanın bir nesnesine tıklayın [Etkin katman (E)]';
  }

  /** A typed point: the object there names the layer. */
  protected point(p: Vec2): void {
    this.toLayerOf(this.ctx.view.pick(this.ctx.view.camera.worldToScreen(p)));
  }

  override pointerDown(p: ToolPointer): void {
    if (this.picking || p.button !== 0) return super.pointerDown(p);
    this.toLayerOf(this.ctx.view.pick(p.screen));
  }

  override pointerMove(p: ToolPointer): void {
    if (this.picking) return super.pointerMove(p);
    this.hover = p.raw;
    this.target = this.ctx.view.pick(p.screen);
    this.ctx.selection.hover.set(this.target?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  override input(text: string): boolean {
    if (!this.picking && text.trim().toLocaleUpperCase('tr-TR') === 'E') {
      this.toLayer(this.ctx.doc.layers.active.value);
      return true;
    }
    return super.input(text);
  }

  private toLayerOf(hit: Entity | null): void {
    if (!hit) return void this.ctx.log.warn('Hedef katmanın bir nesnesine tıklayın ya da Etkin katman (E) seçin.');
    this.toLayer(hit.layerId);
  }

  /** Moves or copies the selection to `layerId`, then the tool leaves; a locked target keeps it waiting. */
  private toLayer(layerId: string): void {
    const { doc, log } = this.ctx;
    const name = doc.layers.get(layerId)?.name ?? layerId;
    if (doc.layers.isLocked(layerId)) {
      return void log.warn(`“${name}” katmanı kilitli; nesneler oraya ${this.copy ? 'kopyalanamaz' : 'geçemez'}. Kilidini Katmanlar panelinden açın.`);
    }
    const all = this.targets();
    const free = all.filter((e) => !doc.layers.isLocked(e.layerId));
    if (free.length < all.length) log.warn(`${all.length - free.length} nesne kilitli katmanda olduğu için atlandı.`);
    const objects = this.copy ? free : free.filter((e) => e.layerId !== layerId);
    if (!objects.length) {
      if (!this.copy && free.length) log.info(`Seçili nesneler zaten “${name}” katmanında.`);
    } else if (this.copy) this.copyTo(objects, layerId, name);
    else this.moveTo(objects, layerId, name);
    queueMicrotask(() => this.ctx.tools.exit());
  }

  private moveTo(objects: Entity[], layerId: string, name: string): void {
    const { doc, log } = this.ctx;
    const uids = objects.map((e) => uidOf(this.ctx, e));
    const result = doc.transact('Katmanı eşle', () => entitiesSet.execute({ doc }, { uids, layerId, operation: 'layer' }));
    if (result.status !== 'completed') return void ('error' in result && log.warn(result.error.message));
    for (const w of result.warnings) log.warn(w.message);
    log.success(`${result.output.changed.length} nesne “${name}” katmanına geçti.`);
  }

  private copyTo(objects: Entity[], layerId: string, name: string): void {
    const { doc, log, selection } = this.ctx;
    const objectOf = (e: Entity): NewObject => ({
      geometry: geometryOf(e as unknown as EditGeometry) as unknown as EditGeometry,
      ...(e.color !== undefined && { color: e.color }),
      ...(e.lineWeight !== undefined && { lineWeight: e.lineWeight }),
      attrs: { ...e.attrs },
      ...(e.label !== undefined && { label: e.label }),
      ...(e.symbol !== undefined && { symbol: e.symbol }),
    });
    const result = doc.transact('Katmana kopyala', () => entitiesCreate.execute({ doc }, { layerId, objects: objects.map(objectOf) }));
    if (result.status !== 'completed') return void ('error' in result && log.warn(result.error.message));
    for (const w of result.warnings) log.warn(w.message);
    selection.set(result.output.ids);
    log.success(`${result.output.ids.length} nesnenin kopyası “${name}” katmanına yazıldı.`);
  }

  override draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    if (this.picking) return super.draw(g, view);
    const pal = this.ctx.view.palette;
    if (this.target && this.hover) {
      const name = this.ctx.doc.layers.get(this.target.layerId)?.name ?? this.target.layerId;
      drawTag(g, view.worldToScreen(this.hover), [`Hedef: ${name}`, this.copy ? 'Tıklayın: kopyala' : 'Tıklayın: taşı'], pal.accent, pal.labelHalo);
    }
  }
}
