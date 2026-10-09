// Katman süzgeci's commands (docs/adr/0211 §4; the desktop's `layer_filters.rs`): the window, Seçimden süzgeç and
// Süzgeci kaldır, and the one message a new object its layer's filter leaves out takes (§3). The window loads on first
// use (CLAUDE.md §20).

import type { LayerFilter } from '../contracts/generated/LayerFilter';
import type { LayersFilter } from '../contracts/generated/LayersFilter';
import type { LayerNode } from '../model/layers';
import { layersFilter } from '../product/layersFilter';
import type { AppContext } from './context';

/** Seçimden süzgeç's undo step. */
export const FROM_SELECTION_LABEL = 'Seçimden süzgeç';

const failed = (ctx: AppContext) => (e: Error) => ctx.log.error(`Pencere yüklenemedi: ${e.message}. Bağlantıyı denetleyip yeniden deneyin.`);

/** The layer a command means: the one the tree's menu named, else the active one; a group or a service layer is none. */
function layerAtHand(ctx: AppContext, args: unknown): LayerNode | undefined {
  const layers = ctx.doc.layers;
  const node = layers.get(typeof args === 'string' ? args : layers.active.value);
  return node?.type === 'layer' ? node : undefined;
}

/** What a filter keeps, said in a few words: “ifade ve 12 nesnelik liste”. */
export function filterText(f: LayerFilter): string {
  const list = f.objects?.length ? `${f.objects.length} nesnelik liste` : '';
  return f.expression != null ? (list ? `“${f.expression}” ve ${list}` : `“${f.expression}”`) : list;
}

/**
 * Seçimden süzgeç (docs/adr/0211 §1): each layer of the selected objects (only `only`, the layer tree's menu's) gets
 * the list of its selected objects; a condition it has stays. One undo step, all layers or none.
 */
export function filterFromSelection(ctx: AppContext, only?: string): void {
  const { doc, selection, log } = ctx;
  const byLayer = new Map<string, string[]>();
  for (const id of selection.ids.value) {
    const e = doc.get(id);
    const uid = e && doc.uidOf(id);
    if (!e || !uid || (only !== undefined && e.layerId !== only)) continue;
    const list = byLayer.get(e.layerId);
    if (list) list.push(uid);
    else byLayer.set(e.layerId, [uid]);
  }
  if (!byLayer.size) return void log.warn('Seçili nesne yok; önce süzgeçte kalacak nesneleri seçin.');
  const inputs: LayersFilter[] = [...byLayer].map(([layer, objects]) => {
    const expression = doc.layers.get(layer)?.filter?.expression;
    return { layer, filter: { ...(expression != null && { expression }), objects } };
  });
  for (const input of inputs) {
    const checked = layersFilter.validate({ doc }, input);
    if (checked.status !== 'completed') return void ('error' in checked && log.warn(checked.error.message));
  }
  let changed = 0;
  doc.transact(FROM_SELECTION_LABEL, () => {
    for (const input of inputs) {
      const r = layersFilter.execute({ doc }, input);
      if (r.status === 'completed' && r.output.changed) changed++;
    }
  });
  const names = inputs.map((i) => `“${doc.layers.get(i.layer)?.name ?? i.layer}” (${i.filter!.objects!.length})`);
  if (!changed) log.info(`${names.join(', ')} katmanlarının süzgeci zaten bu nesneler.`);
  else log.success(`Süzgeç seçimden yazıldı: ${names.join(', ')}. Yalnız bu nesneler görünür; Süzgeci kaldır ile hepsi döner.`);
}

/** Süzgeci kaldır: the layer's filter taken away, one undo step “Katman süzgeci”. */
export function clearFilter(ctx: AppContext, layer: LayerNode): void {
  const { doc, log } = ctx;
  if (!layer.filter) return void log.info(`“${layer.name}” katmanının süzgeci yok.`);
  const r = layersFilter.execute({ doc }, { layer: layer.id });
  if (r.status !== 'completed') return void ('error' in r && log.warn(r.error.message));
  log.success(`“${layer.name}” katmanının süzgeci kaldırıldı; ${r.output.total} nesnesinin hepsi görünür.`);
}

/**
 * A new object its layer's filter leaves out does not show (docs/adr/0211 §3): drawn, pasted or brought in, it is said
 * once for the change, by layer. Another editor's objects are not.
 */
function sayHiddenNew(ctx: AppContext): void {
  const { doc, log } = ctx;
  doc.events.on('touched', ({ added, external }) => {
    if (external || !added.length) return;
    // After the change's other listeners: the geometry store takes the new objects first.
    queueMicrotask(() => {
      const hidden = new Map<string, number>();
      for (const id of added) {
        const e = doc.get(id);
        if (!e || !doc.layers.get(e.layerId)?.filter || ctx.view.geometry.filterShown(id)) continue;
        hidden.set(e.layerId, (hidden.get(e.layerId) ?? 0) + 1);
      }
      for (const [layer, n] of hidden)
        log.warn(`“${doc.layers.get(layer)?.name ?? layer}” katmanının süzgecinden geçmeyen ${n} yeni nesne görünmüyor; Süzgeç… ya da Süzgeci kaldır.`);
    });
  });
}

export function registerLayerFilterCommands(ctx: AppContext): void {
  const { log } = ctx;
  const noLayer = 'Süzgeç katmanın olur: etkin katman bir grup ya da servisten çizilen bir katman. Nesneleri olan bir katmanı etkin yapın.';
  ctx.commands.registerAll([
    {
      id: 'layer.filter',
      title: 'Katman süzgeci…',
      short: 'Katman süzgeci',
      category: 'Katman',
      icon: 'layerFilter',
      description:
        'Katmanın yalnız bir kısmıyla çalışmak için: ifadeye (İfadeyle seç’in dili) ya da bir nesne listesine uymayan nesneler katmanda yokmuş gibidir; çizilmez, seçilmez, kenetlenmez, Öznitelik tablosunda ve İşlemler’de görünmez. Çizim değişmez; kayıt ve dışa aktarma bütün nesneleri yazar.',
      aliases: ['KATMANSUZGECI', 'TANIMSORGUSU', 'LAYERFILTER'],
      run: (args) => {
        const layer = layerAtHand(ctx, args);
        if (!layer || layer.service) return void log.warn(noLayer);
        void import('../ui/layers/LayerFilterDialog').then((m) => m.openLayerFilter(ctx, layer.id), failed(ctx));
      },
    },
    {
      id: 'layer.filterFromSelection',
      title: 'Seçimden süzgeç',
      category: 'Katman',
      icon: 'layerFilterSelection',
      description: 'Seçili nesnelerin her katmanına yalnız o katmanın seçili nesnelerini gösteren bir süzgeç yazar; katmanın ifadesi kalır. Tek adımda geri alınır.',
      aliases: ['SECIMDENSUZGEC'],
      run: (args) => filterFromSelection(ctx, typeof args === 'string' ? args : undefined),
    },
    {
      id: 'layer.filterClear',
      title: 'Süzgeci kaldır',
      category: 'Katman',
      icon: 'layerFilterClear',
      description: 'Etkin katmanın (Katmanlar panelinde sağ tıklanan katmanın) süzgecini kaldırır; bütün nesneleri yeniden görünür.',
      aliases: ['SUZGECKALDIR'],
      run: (args) => {
        const layer = layerAtHand(ctx, args);
        if (!layer) return void log.warn(noLayer);
        clearFilter(ctx, layer);
      },
    },
  ]);
  sayHiddenNew(ctx);
}
