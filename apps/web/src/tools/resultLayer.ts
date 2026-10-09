import type { AppContext } from '../app/context';
import { foldTurkish } from '../core/text';
import type { EntityGeometry as NewGeometry } from '../contracts/generated/EntityGeometry';
import type { EntityGeometry } from '../model/entities';
import type { LayerStyle } from '../model/layers';
import { entitiesCreate } from '../product/entitiesCreate';

/**
 * The layers the network tools write their results to (docs/adr/0209 §5–§7): Rota, Hizmet alanı and its lines, En
 * yakın tesis. A layer of that name (compared trimmed and Turkish-folded, as İşlemler's new layers are) is written to;
 * without one it is opened, with its own look, at the top of the tree, in the write's undo step. The objects take the
 * layer's look (no colour of their own). The desktop's is `kentos_interaction::network::result_layer`.
 */
export interface ResultLayer {
  readonly name: string;
  readonly style: LayerStyle;
}

export const ROUTE_LAYER: ResultLayer = { name: 'Rota', style: { color: '#D32F2F', lineType: 'continuous', lineWeight: 0.5 } };
export const AREA_LAYER: ResultLayer = { name: 'Hizmet alanı', style: { color: '#1976D2', lineType: 'continuous', lineWeight: 0.25, fill: '#1976D233' } };
export const AREA_LINES_LAYER: ResultLayer = { name: 'Hizmet alanı çizgileri', style: { color: '#1565C0', lineType: 'continuous', lineWeight: 0.35 } };
export const CLOSEST_LAYER: ResultLayer = { name: 'En yakın tesis', style: { color: '#7B1FA2', lineType: 'continuous', lineWeight: 0.5 } };

/** The id a result layer opens with: `ag-` and its folded name, numbered when taken. */
function freshId(ctx: AppContext, name: string): string {
  const base = `ag-${foldTurkish(name).toLowerCase().replace(/[^a-z0-9]+/g, '-')}`;
  let id = base;
  for (let k = 2; ctx.doc.layers.get(id); k++) id = `${base}-${k}`;
  return id;
}

/** The drawing's layer of a result layer's name, if it has one. */
export function resultLayerOf(ctx: AppContext, layer: ResultLayer): string | null {
  const key = foldTurkish(layer.name);
  return ctx.doc.layers.leaves().find((l) => foldTurkish(l.name) === key)?.id ?? null;
}

/** One layer's objects of a write. */
export interface ResultWrite {
  readonly layer: ResultLayer;
  readonly objects: readonly { geometry: EntityGeometry; attrs: Record<string, string> }[];
}

/**
 * Writes the objects to their result layers in one undo step named `step`, opening the layers the drawing lacks in it;
 * nothing stays when one is refused (its words are said). The ids written, layer by layer, or null.
 */
export function writeResults(ctx: AppContext, step: string, writes: readonly ResultWrite[]): number[] | null {
  const { doc } = ctx;
  const ids: number[] = [];
  const opened: string[] = [];
  let refused: string | null = null;
  try {
    doc.transact(step, () => {
      for (const w of writes) {
        if (!w.objects.length) continue;
        let layerId = resultLayerOf(ctx, w.layer);
        if (!layerId) {
          layerId = freshId(ctx, w.layer.name);
          doc.addLayer({ id: layerId, name: w.layer.name, style: { ...w.layer.style } }, null);
          opened.push(w.layer.name);
        }
        const objects = w.objects.map((o) => ({ geometry: o.geometry as unknown as NewGeometry, attrs: o.attrs }));
        const r = entitiesCreate.execute({ doc }, { layerId, objects });
        if (r.status !== 'completed') {
          refused = 'error' in r ? r.error.message : `${step} yazılamadı.`;
          throw new Error(refused);
        }
        for (const warning of r.warnings) ctx.log.warn(warning.message);
        ids.push(...r.output.ids);
      }
    });
  } catch (e) {
    ctx.log.warn(refused ?? (e as Error).message);
    return null;
  }
  for (const name of opened) ctx.log.info(`“${name}” katmanı çizimde yoktu; ${step.toLocaleLowerCase('tr-TR')} için açıldı.`);
  return ids;
}
