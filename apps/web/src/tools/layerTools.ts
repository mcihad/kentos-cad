import type { AppContext } from '../app/context';
import { Signal } from '../core/signal';
import { entityGeometry, type Entity } from '../model/entities';
import type { Vec2 } from '../model/geometry';
import type { ViewTransform } from '../viewport/Camera';
import { drawTag, strokeGeometry } from './preview';
import type { Tool, ToolPointer } from './Tool';

/**
 * Layer actions by an object (docs/adr/0177 §1; the desktop's `kentos_interaction::layer_tools`): a click on an object
 * acts on its layer. Katmanı gizle and Katmanı kilitle go on, a layer a click; Katmanı etkin yap leaves after its click;
 * Katmanı yalıt gathers the clicked objects' layers and isolates them on Enter or a right click, remembering what it hid
 * for Yalıtımı kaldır (`ToolManager.isolatedLayers`, the drawing's: another drawing forgets it). With objects selected
 * when it starts, it acts on their layers at once (Katmanı etkin yap on the first one's) and leaves.
 *
 * These are the layer tree's own changes, as the Katmanlar panel's eye, lock and active layer: the drawing changes, no
 * undo step. A hidden layer's objects leave the selection.
 */

export type LayerAction = 'off' | 'isolate' | 'lock' | 'active';

/** The tools' ids: their commands are `tool.<id>`. */
export const LAYER_TOOL_ID: Record<LayerAction, string> = { off: 'layerOff', isolate: 'layerIsolate', lock: 'layerLock', active: 'layerMakeActive' };

const LABEL: Record<LayerAction, string> = { off: 'Katmanı gizle', isolate: 'Katmanı yalıt', lock: 'Katmanı kilitle', active: 'Katmanı etkin yap' };
const STEP: Record<LayerAction, string> = {
  off: 'katmanı gizlenecek nesneye tıklayın',
  isolate: 'yalıtılacak katmanların nesnelerine tıklayın',
  lock: 'katmanı kilitlenecek nesneye tıklayın',
  active: 'katmanı etkin yapılacak nesneye tıklayın',
};

/** “3 katman ve 1 grup”, “1 grup”: what a count of layers and groups says. */
export function nodesText(layers: number, groups: number): string {
  const parts = [layers ? `${layers} katman` : '', groups ? `${groups} grup` : ''].filter(Boolean);
  return parts.join(' ve ');
}

export class LayerTool implements Tool {
  readonly id: string;
  readonly prompt = new Signal('');
  readonly cursor = 'pick' as const;
  readonly snaps = false;
  private readonly ctx: AppContext;
  private readonly action: LayerAction;
  /** Katmanı yalıt's layers so far, by id, in click order, and the objects clicked for them. */
  private gathered: string[] = [];
  private picked: number[] = [];
  private hover: Entity | null = null;
  private mouse: Vec2 | null = null;

  constructor(ctx: AppContext, action: LayerAction) {
    this.ctx = ctx;
    this.action = action;
    this.id = LAYER_TOOL_ID[action];
  }

  /** Katmanı yalıt's gathered layers are its steps: Ctrl+Z takes back the newest. */
  get pointCount(): number {
    return this.gathered.length;
  }

  activate(): void {
    const { doc, selection } = this.ctx;
    const selected = [...selection.ids.value].flatMap((id) => {
      const e = doc.get(id);
      return e ? [e] : [];
    });
    if (selected.length) {
      // Selected first: their layers at once, and the tool leaves (as Okunur yap does).
      const layers = [...new Set(selected.map((e) => e.layerId))];
      this.act(this.action === 'active' ? layers.slice(0, 1) : layers);
      queueMicrotask(() => this.ctx.tools.exit());
      return;
    }
    this.refresh();
  }

  deactivate(): void {
    this.ctx.selection.hover.set(null);
  }

  private name(id: string): string {
    return this.ctx.doc.layers.get(id)?.name ?? id;
  }

  private refresh(): void {
    const label = LABEL[this.action];
    const step = STEP[this.action];
    const bracket =
      this.action === 'isolate'
        ? [this.gathered.length ? `${this.gathered.length} katman: ${this.gathered.map((id) => this.name(id)).join(', ')}` : '', 'Uygula (Enter)'].filter(Boolean).join('; ')
        : this.action === 'active'
          ? ''
          : 'Bitir (Enter)';
    this.prompt.set(`${label}: ${step}${bracket ? ` [${bracket}]` : ''}`);
    this.ctx.view.requestOverlay();
  }

  pointerMove(p: ToolPointer): void {
    this.mouse = p.raw;
    this.hover = this.ctx.view.pick(p.screen);
    this.ctx.selection.hover.set(this.hover?.id ?? null);
    this.ctx.view.requestOverlay();
  }

  pointerDown(p: ToolPointer): void {
    if (p.button !== 0) return;
    const hit = this.ctx.view.pick(p.screen);
    if (!hit) return void this.ctx.log.warn(`Bir nesneye tıklayın: ${LABEL[this.action]} onun katmanında çalışır.`);
    if (this.action === 'isolate') {
      if (!this.gathered.includes(hit.layerId)) this.gathered.push(hit.layerId);
      if (!this.picked.includes(hit.id)) this.picked.push(hit.id);
      return this.refresh();
    }
    this.act([hit.layerId]);
    if (this.action === 'active') queueMicrotask(() => this.ctx.tools.exit());
    else this.refresh();
  }

  confirm(): void {
    if (this.action === 'isolate' && this.gathered.length) this.act(this.gathered);
    this.gathered = [];
    this.picked = [];
    this.ctx.tools.exit();
  }

  cancel(): boolean {
    // Katmanı yalıt steps back to no layer gathered; then Esc leaves.
    if (this.action !== 'isolate' || !this.gathered.length) return false;
    this.gathered = [];
    this.picked = [];
    this.refresh();
    return true;
  }

  undoStep(): boolean {
    // Ctrl+Z while gathering takes back the newest layer.
    if (this.action !== 'isolate' || !this.gathered.length) return false;
    const layer = this.gathered.pop();
    this.picked = this.picked.filter((id) => this.ctx.doc.get(id)?.layerId !== layer);
    this.refresh();
    return true;
  }

  /** The action on these layers (ids, in order, each once). */
  private act(ids: string[]): void {
    if (this.action === 'off') this.hide(ids);
    else if (this.action === 'isolate') this.isolate(ids);
    else if (this.action === 'lock') this.lock(ids);
    else this.makeActive(ids[0]);
  }

  private list(ids: string[]): string {
    return ids.length === 1 ? `“${this.name(ids[0])}” katmanı` : `${ids.length} katman (${ids.map((id) => this.name(id)).join(', ')})`;
  }

  private hide(ids: string[]): void {
    const { doc, log } = this.ctx;
    const layers = doc.layers;
    const shown = ids.filter((id) => layers.get(id)?.visible);
    if (!shown.length) return void log.info(`${this.list(ids)} zaten gizli.`);
    for (const id of shown) layers.setVisible(id, false);
    this.dropHidden();
    const active = shown.includes(layers.active.value) ? ' Etkin katman gizli: yeni nesneler gizli katmana çizilir.' : '';
    log.success(`${this.list(shown)} gizlendi.${active}`);
  }

  private isolate(ids: string[]): void {
    const { doc, log, tools } = this.ctx;
    const layers = doc.layers;
    const shownBefore = layers.all().filter((n) => n.visible).map((n) => n.id);
    if (!layers.isolateLayers(ids)) return void log.info(`Yalnız ${this.list(ids)} zaten görünüyor; değişen olmadı.`);
    const hidden = shownBefore.filter((id) => !layers.get(id)?.visible);
    // What isolations hid since the last Yalıtımı kaldır, in order: it shows them all again.
    for (const id of hidden) if (!tools.isolatedLayers.includes(id)) tools.isolatedLayers.push(id);
    this.dropHidden();
    const groups = hidden.filter((id) => layers.get(id)?.type === 'group').length;
    log.success(`${this.list(ids)} yalıtıldı; öbür ${nodesText(hidden.length - groups, groups)} gizlendi. Yalıtımı kaldır onları geri getirir.`);
  }

  private lock(ids: string[]): void {
    const { doc, log } = this.ctx;
    const layers = doc.layers;
    const open = ids.filter((id) => !layers.isLocked(id));
    if (!open.length) return void log.info(`${this.list(ids)} zaten kilitli.`);
    for (const id of open) layers.toggleLocked(id);
    log.success(`${this.list(open)} kilitlendi.`);
  }

  private makeActive(id: string | undefined): void {
    if (id === undefined) return;
    const { doc, log } = this.ctx;
    const layers = doc.layers;
    if (layers.active.value === id) return void log.info(`“${this.name(id)}” katmanı zaten etkin.`);
    if (layers.isLocked(id)) return void log.warn(`“${this.name(id)}” katmanı kilitli; etkin yapılmadı. Kilidini Katmanlar panelinden açın.`);
    layers.setActive(id);
    log.success(`“${this.name(id)}” katmanı etkin yapıldı.`);
  }

  /** The objects of layers now hidden leave the selection. */
  private dropHidden(): void {
    const { doc, selection } = this.ctx;
    const ids = [...selection.ids.value];
    const kept = ids.filter((id) => {
      const e = doc.get(id);
      return !!e && doc.layers.isVisible(e.layerId);
    });
    if (kept.length < ids.length) selection.set(kept);
  }

  draw(g: CanvasRenderingContext2D, view: ViewTransform): void {
    const pal = this.ctx.view.palette;
    for (const id of this.picked) {
      const e = this.ctx.doc.get(id);
      if (e) strokeGeometry(g, view, entityGeometry(e), { color: pal.accent, width: 2.5 });
    }
    if (!this.hover || !this.mouse) return;
    strokeGeometry(g, view, entityGeometry(this.hover), { color: pal.snap, dash: [5, 3], width: 2 });
    drawTag(g, view.worldToScreen(this.mouse), [`Katman: ${this.name(this.hover.layerId)}`, `Tıklayın: ${LABEL[this.action].replace('Katmanı ', '')}`], pal.accent, pal.labelHalo);
  }
}
