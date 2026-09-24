import type { AppContext } from '../../app/context';
import type { Entity } from '../../model/entities';

/** What an export writes: the selection, the objects on visible layers, or everything. */
export type ExportScope = 'selection' | 'visible' | 'all';

export const SCOPE_LABEL: Record<ExportScope, string> = {
  selection: 'Seçili',
  visible: 'Görünen katmanlar',
  all: 'Tümü',
};

export function scopeEntities(ctx: AppContext, scope: ExportScope): Entity[] {
  const { doc } = ctx;
  if (scope === 'selection') return [...ctx.selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
  const all = [...doc.all()];
  return scope === 'all' ? all : all.filter((e) => doc.layers.isVisible(e.layerId));
}
