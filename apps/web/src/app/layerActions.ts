import type { AppContext } from './context';
import { entitiesCreate } from '../product/entitiesCreate';
import { entitiesSet } from '../product/entitiesSet';
import { newObjectOf } from '../tools/layerMoveTool';
import { nodesText } from '../tools/layerTools';
import { treeLocked } from '../ui/layers/treeRights';

/**
 * The layer commands that are no tool (docs/adr/0177; the desktop's `app.rs` and `layer_merge.rs`). Yalıtımı kaldır
 * (§1) shows again the layers and groups Katmanı yalıt hid since the last time (`ToolManager.isolatedLayers`) and still
 * hidden; those changed by hand meanwhile stay as they are. A tree change, as the eye: no undo step.
 */
export function unisolateLayers(ctx: AppContext): void {
  const { doc, log, tools } = ctx;
  const layers = doc.layers;
  const remembered = [...tools.isolatedLayers];
  tools.isolatedLayers.length = 0;
  if (!remembered.length) return void log.info('Yalıtılmış katman yok: Katmanı yalıt bu çizimde katman gizlemedi.');
  const back = remembered.filter((id) => layers.get(id)?.visible === false);
  if (!back.length) return void log.info('Katmanı yalıt’ın gizlediği katmanlar zaten görünüyor.');
  for (const id of back) layers.setVisible(id, true);
  const groups = back.filter((id) => layers.get(id)?.type === 'group').length;
  log.success(`Yalıtım kaldırıldı: ${nodesText(back.length - groups, groups)} yeniden gösterildi.`);
}

/** What the dialog shows above the list, each line with its kind, and whether Birleştir may write. */
export interface MergeSummary {
  readonly ready: boolean;
  readonly lines: readonly (readonly ['info' | 'warn', string])[];
}

/**
 * Katmanları birleştir's words (docs/adr/0177 §3; the desktop's `layer_merge::summary`): what keeps it from writing, or
 * what it will do.
 */
export function mergeSummary(ctx: AppContext, sources: readonly string[], target: string): MergeSummary {
  const { doc } = ctx;
  const layers = doc.layers;
  const locked = treeLocked(ctx);
  if (locked) return { ready: false, lines: [['warn', locked]] };
  const into = layers.get(target);
  if (!into || into.type !== 'layer') return { ready: false, lines: [['warn', 'Hedef bir katman olmalı; listeden bir katman seçin.']] };
  if (layers.isLocked(target)) return { ready: false, lines: [['warn', `“${into.name}” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın.`]] };
  const from = sources.filter((id) => id !== target && layers.get(id)?.type === 'layer');
  if (!from.length) return { ready: false, lines: [['info', 'Birleşecek katmanları işaretleyin: nesneleri hedef katmana geçer, katmanları silinir.']] };
  const objects = from.reduce((n, id) => n + doc.byLayer(id).length, 0);
  const lines: (readonly ['info' | 'warn', string])[] = [['info', `${from.length} katman “${into.name}” katmanına birleşir: ${objects} nesne taşınır, katmanlar silinir. Tek adımda geri alınır.`]];
  const active = layers.active.value;
  if (from.includes(active)) lines.push(['info', `Etkin katman (“${layers.get(active)?.name ?? active}”) birleşiyor: “${into.name}” etkin katman olur.`]);
  return { ready: true, lines };
}

/**
 * Katmanları birleştir (`layer.merge`, docs/adr/0177 §3): the sources' objects go to the target and the sources are
 * removed, in one undo step “Katmanları birleştir”; a source that is the active layer gives that to the target first (a
 * tree change, as the eye: outside the step). Refused, with what mergeSummary says, when it may not write. Whether it
 * wrote.
 */
export function mergeLayers(ctx: AppContext, sources: readonly string[], target: string): boolean {
  const { doc, log } = ctx;
  const said = mergeSummary(ctx, sources, target);
  if (!said.ready) return (log.warn(said.lines[0][1]), false);
  const layers = doc.layers;
  const into = layers.get(target)!;
  const from = sources.filter((id) => id !== target && layers.get(id)?.type === 'layer');
  // A source on a locked layer stops it: its objects could not move.
  const locked = from.find((id) => layers.isLocked(id));
  if (locked) return (log.warn(`“${layers.get(locked)?.name}” katmanı kilitli; birleştirilemez. Kilidini Katmanlar panelinden açın.`), false);
  const uids = from.flatMap((id) => doc.byLayer(id)).flatMap((e) => (e.uid ? [e.uid] : []));
  if (from.includes(layers.active.value)) layers.setActive(target);
  const warnings: string[] = [];
  try {
    doc.transact('Katmanları birleştir', () => {
      if (uids.length) {
        const r = entitiesSet.execute({ doc }, { uids, layerId: target, operation: 'layer' });
        if (r.status !== 'completed') throw new Error('error' in r ? r.error.message : 'Nesneler taşınamadı.');
        warnings.push(...r.warnings.map((w) => w.message));
      }
      for (const id of from) doc.removeLayer(id);
    });
  } catch (e) {
    log.warn(e instanceof Error ? e.message : String(e));
    return false;
  }
  for (const w of warnings) log.warn(w);
  log.success(`${from.length} katman “${into.name}” katmanına birleştirildi: ${uids.length} nesne taşındı.`);
  return true;
}

/**
 * Kopyasını oluştur (`layer.duplicate`, docs/adr/0177 §3): a new layer last in the same group, named “<ad> kopyası” (one
 * of its kind in the tree), with the same style (and layer style) and visibility, unlocked, and the copies of its
 * objects; one undo step “Katmanı kopyala”. Without an id, the active layer's. A locked layer is not copied (nor are its
 * objects, CLAUDE.md §7); a project without project.edit cannot change its tree.
 */
export function duplicateLayer(ctx: AppContext, id?: string): void {
  const { doc, log } = ctx;
  const locked = treeLocked(ctx);
  if (locked) return void log.warn(locked);
  const source = doc.layers.get(id ?? doc.layers.active.value);
  if (!source || source.type !== 'layer') return void log.warn('Kopyası oluşturulacak bir katman seçin; grubun kopyası oluşturulmaz.');
  if (doc.layers.isLocked(source.id)) return void log.warn(`“${source.name}” katmanı kilitli; kopyası oluşturulmaz. Kilidini Katmanlar panelinden açın.`);
  const name = doc.layers.uniqueName(`${source.name} kopyası`);
  const objects = doc.byLayer(source.id);
  const warnings: string[] = [];
  let made = 0;
  try {
    doc.transact('Katmanı kopyala', () => {
      const node = doc.addLayer({ name, type: 'layer', visible: source.visible, style: structuredClone(source.style) }, source.id);
      if (!objects.length) return;
      const r = entitiesCreate.execute({ doc }, { layerId: node.id, objects: objects.map(newObjectOf) });
      if (r.status !== 'completed') throw new Error('error' in r ? r.error.message : 'Nesneler kopyalanamadı.');
      warnings.push(...r.warnings.map((w) => w.message));
      made = r.output.ids.length;
    });
  } catch (e) {
    return void log.warn(e instanceof Error ? e.message : String(e));
  }
  for (const w of warnings) log.warn(w);
  log.success(`“${source.name}” katmanı “${name}” olarak kopyalandı: ${made} nesne.`);
}
