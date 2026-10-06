import type { Command } from '../core/commands';
import { ENTITY_KIND_LABEL, type EntityKind } from '../model/entities';
import { FILTER_KINDS, selectableIds } from '../tools/selectable';
import { ENTITY_KIND_ICON } from '../ui/kindIcons';
import type { AppContext } from './context';

/**
 * The selection's commands in Düzen › Seçim: Tümünü seç, Seçimi kaldır, Ters çevir, and those of docs/adr/0187:
 * Önceki seçim (§3), Sıradakini seç (§1, Shift+Boşluk) and Seçim süzgeci with its kinds (§5). The desktop runs the same
 * ids (`apps/desktop/src/selection_commands.rs`).
 */

export function selectionCommands(ctx: AppContext): Command[] {
  const { doc, selection, settings, log } = ctx;
  const E = 'Düzen';
  const visible = () => [...doc.all()].filter((e) => doc.layers.isVisible(e.layerId));
  /** Önceki seçim's objects still in the drawing and shown, in their order. */
  const previous = () => selection.previous.value.filter((id) => {
    const e = doc.get(id);
    return !!e && doc.layers.isVisible(e.layerId);
  });
  return [
    {
      id: 'edit.selectAll',
      title: 'Tümünü seç',
      category: E,
      icon: 'selectAll',
      run: () => {
        const ids = selectableIds(ctx, visible().map((e) => e.id));
        selection.set(ids);
        log.info(`${ids.length} nesne seçildi.`);
      },
    },
    { id: 'edit.deselect', title: 'Seçimi kaldır', category: E, icon: 'deselect', run: () => selection.clear(), isEnabled: () => selection.size > 0, watch: [selection.ids] },
    {
      id: 'edit.invertSelection',
      title: 'Seçimi ters çevir',
      category: E,
      icon: 'invertSelection',
      run: () => selection.set(selectableIds(ctx, visible().filter((e) => !selection.has(e.id)).map((e) => e.id))),
    },
    {
      id: 'edit.previousSelection',
      title: 'Önceki seçim',
      category: E,
      icon: 'selectPrevious',
      aliases: ['ONCEKISECIM', 'SELECTPREVIOUS'],
      description: 'Yerini yeni bir seçime bırakan ya da kaldırılan son seçimi geri getirir; yeniden basmak bir önceki hâle döner.',
      run: () => {
        const ids = previous();
        if (!ids.length) return void log.warn('Geri getirilecek önceki seçim yok.');
        selection.set(ids);
        log.info(`Önceki seçim geri geldi: ${ids.length} nesne.`);
      },
      isEnabled: () => previous().length > 0,
      watch: [selection.previous, selection.ids],
    },
    {
      id: 'edit.cycleSelection',
      title: 'Sıradakini seç',
      category: E,
      icon: 'selectCycle',
      aliases: ['SIRADAKINISEC', 'SELECTIONCYCLING'],
      description: 'Tıklanan yerde üst üste binen nesnelerden sıradakini seçer; tıklamanın yanındaki çip listeyi açar.',
      run: () => {
        const c = selection.cycle.value;
        if (c) selection.cycleTo(c.index + 1);
      },
      isEnabled: () => selection.cycle.value !== null,
      watch: [selection.cycle],
    },
    {
      id: 'edit.selectFilter',
      title: 'Seçim süzgeci',
      short: 'Süzgeç',
      category: E,
      icon: 'selectFilter',
      aliases: ['SUZGEC', 'SECIMSUZGECI', 'FILTER'],
      description: 'Açıkken yalnız işaretli türlerin nesneleri seçilir; türler hücrenin sağ tık menüsündedir.',
      run: () => {
        const on = !settings.selectFilter.value;
        settings.selectFilter.set(on);
        log.info(on ? `Seçim süzgeci açık: ${kindsText(settings.selectKinds.value)}.` : 'Seçim süzgeci kapalı.');
      },
      isChecked: () => settings.selectFilter.value,
      watch: [settings.selectFilter],
    },
    ...FILTER_KINDS.map(
      (kind): Command => ({
        id: `edit.selectFilter.${kind}`,
        title: `Seçim süzgecinde ${ENTITY_KIND_LABEL[kind]}`,
        short: ENTITY_KIND_LABEL[kind],
        category: E,
        icon: ENTITY_KIND_ICON[kind],
        // Ticking or unticking a kind turns the filter on (docs/adr/0187 §5).
        run: () => {
          const next = new Set(settings.selectKinds.value);
          if (!next.delete(kind)) next.add(kind);
          settings.selectKinds.set(next);
          settings.selectFilter.set(true);
        },
        isChecked: () => settings.selectKinds.value.has(kind),
        watch: [settings.selectKinds],
      }),
    ),
  ];
}

/** The kinds the filter holds as the log says them: all of them, none, or their names. */
function kindsText(kinds: ReadonlySet<EntityKind>): string {
  if (kinds.size === FILTER_KINDS.length) return 'bütün türler';
  if (!kinds.size) return 'hiçbir tür (hiçbir şey seçilmez)';
  return FILTER_KINDS.filter((k) => kinds.has(k))
    .map((k) => ENTITY_KIND_LABEL[k].toLocaleLowerCase('tr-TR'))
    .join(', ');
}
