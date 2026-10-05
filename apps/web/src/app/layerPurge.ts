import type { Entity } from '../model/entities';
import { purgeFound, purgeRemoved, type PurgeFound, type PurgeIds, type PurgeObject, type PurgeSource } from '../model/layerPurge';
import { treeLocked } from '../ui/layers/treeRights';
import type { AppContext } from './context';

/**
 * Kullanılmayanları temizle (`layer.purge`, docs/adr/0177 §5; the desktop's `layer_purge.rs`): what the drawing and the
 * project library hold that nothing uses, by the shared rule (`model/layerPurge.ts`). The checked layers, groups and
 * block definitions go in one undo step “Kullanılmayanları temizle”; the project library's symbols and assets go from
 * the library, which keeps no undo (docs/adr/0092). A project without project.edit may not change its tree.
 */

const objectOf = (e: { layerId: string; symbol?: string; kind?: string; block?: string }): PurgeObject => ({
  layer: e.layerId,
  ...(e.symbol && { symbol: e.symbol }),
  ...(e.kind === 'insert' && e.block && { block: e.block }),
});

/** What the rule reads of the open drawing and the library. */
export function purgeSource(ctx: AppContext): PurgeSource {
  const { doc } = ctx;
  return {
    tree: doc.layers.tree,
    active: doc.layers.active.value,
    objects: [...doc.all()].map((e: Entity) => objectOf(e as Entity & { block?: string })),
    blocks: doc.blocks.value.map((b) => ({ id: b.id, name: b.name, objects: b.entities.map((e) => objectOf(e as typeof e & { block?: string })) })),
    library: ctx.styles.library.items().map((it) => ({
      id: it.id,
      kind: it.kind,
      source: it.source,
      name: it.name,
      ...(it.kind === 'symbol' && { symbol: it.symbol }),
      ...(it.kind === 'template' && { template: it.template }),
    })),
  };
}

/** What Kullanılmayanları temizle lists for the open drawing. */
export function purgeFoundNow(ctx: AppContext): PurgeFound {
  return purgeFound(purgeSource(ctx));
}

/** “2 katman, 1 grup, 3 sembol”: the counts that are not naught. */
function countsText(removed: PurgeIds): string {
  const words: [number, string][] = [
    [removed.layers.length, 'katman'],
    [removed.groups.length, 'grup'],
    [removed.blocks.length, 'blok'],
    [removed.symbols.length, 'sembol'],
    [removed.assets.length, 'varlık'],
  ];
  return words
    .filter(([n]) => n > 0)
    .map(([n, w]) => `${n} ${w}`)
    .join(', ');
}

/**
 * Removes the checked ones nothing staying uses: the definitions (in rounds), the layers and the groups (deepest first)
 * in one undo step, then the library's symbols and assets. Says what went, and how many checked ones stayed because
 * something staying uses them. Whether anything went.
 */
export function purgeUnused(ctx: AppContext, checked: Partial<PurgeIds>): boolean {
  const { doc, log } = ctx;
  const locked = treeLocked(ctx);
  if (locked) return (log.warn(locked), false);
  const { removed, kept } = purgeRemoved(purgeSource(ctx), checked);
  const inDrawing = removed.blocks.length + removed.layers.length + removed.groups.length;
  if (inDrawing)
    try {
      doc.transact('Kullanılmayanları temizle', () => {
        for (const id of removed.blocks) doc.removeBlock(id);
        for (const id of removed.layers) doc.removeLayer(id);
        for (const id of removed.groups) doc.removeLayer(id);
      });
    } catch (e) {
      log.warn(e instanceof Error ? e.message : String(e));
      return false;
    }
  for (const id of [...removed.symbols, ...removed.assets]) ctx.styles.library.remove(id);
  const gone = countsText(removed);
  if (gone) log.success(`Kullanılmayanlar temizlendi: ${gone}.`);
  else log.info('Silinecek bir şey kalmadı: işaretlenenleri kalanlar kullanıyor.');
  if (gone && kept) log.info(`İşaretlenen ${kept} öğe, kalanlar kullandığı için silinmedi.`);
  return !!gone;
}
