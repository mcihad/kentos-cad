import type { AppContext } from '../../../app/context';
import type { Finding } from '../../../contracts/generated/sheet/Finding';
import type { Fix } from '../../../contracts/generated/sheet/Fix';
import type { DisposableStore } from '../../../core/disposable';
import { h, type Child } from '../../dom';
import { icon } from '../../icons';
import { tooltip } from '../../widgets/tooltip';
import type { SheetHost } from '../host';

/**
 * Denetçi's Ön denetim tab (docs/sheet/design.md §9): what the engine finds
 * wrong with the sheet in front before it is exported, errors first, each
 * with its item (Göster chooses it), the engine's sentence on how to put it
 * right, and its fixes as buttons: operations the engine gives (one undo
 * step), or an action of the program's (choose a coordinate system, take a
 * map's place from the view, open the sheet's values). A value only the
 * user can give is marked so.
 */

const ICON: Record<Finding['severity'], string> = { error: 'error', warning: 'warning', info: 'info' };
const WORD: Record<Finding['severity'], string> = { error: 'Hata', warning: 'Uyarı', info: 'Bilgi' };

/** The program's side of a fix (the engine names it, the host does it). */
function runAction(ctx: AppContext, host: SheetHost, f: Finding, fix: Fix): void {
  switch (fix.action) {
    case 'project.crs':
      ctx.commands.execute('crs.set');
      return;
    case 'sheet.variables':
      ctx.commands.execute('sheet.variables');
      return;
    case 'sheet.atlas':
      ctx.commands.execute('sheet.atlas');
      return;
    case 'map.placeFromView':
      if (f.item) host.apply([{ op: 'setItemProps', id: f.item, patch: { kind: { view: { center: host.mapPlace().center } } } }], 'Harita merkezi: görünümden');
      return;
    default:
      ctx.log.warn(`Bu düzeltme (${fix.action}) bu sürümde yok: ${f.fix}`);
  }
}

export function preflightTab(ctx: AppContext, host: SheetHost, sheetId: string, findings: readonly Finding[], d: DisposableStore): Child[] {
  const errors = findings.filter((f) => f.severity === 'error').length;
  const warnings = findings.filter((f) => f.severity === 'warning').length;
  const infos = findings.length - errors - warnings;
  const sheet = host.book()?.book.sheets.find((s) => s.id === sheetId);
  const name = (id: string) => sheet?.items.find((i) => i.id === id)?.name ?? id;
  if (!findings.length)
    return [h('div', { class: 'empty' }, icon('success', 20), h('p', { class: 'empty__title' }, 'Ön denetim temiz'), h('p', { class: 'empty__text' }, 'Sayfa dışında kalan, örtülen, bağı kopuk öğe, sığmayan yazı ya da düşük çözünürlük yok.'))];
  return [
    h('div', { class: 'sheet-pf__sum' }, h('span', { class: 'sheet-pf__count sheet-pf__count--error' }, `${errors} hata`), h('span', { class: 'sheet-pf__count sheet-pf__count--warning' }, `${warnings} uyarı`), h('span', { class: 'sheet-pf__count' }, `${infos} bilgi`)),
    h(
      'ul',
      { class: 'sheet-findings' },
      findings.map((f) => {
        const show = f.item ? h('button', { class: 'btn btn--small btn--ghost', type: 'button' }, 'Göster') : null;
        if (show && f.item) {
          const id = f.item;
          show.addEventListener('click', () => host.state.select([id]));
          d.add(tooltip(show, () => ({ title: `“${name(id)}” öğesini seç`, description: 'Öğe paftada ve denetçide seçilir.' })));
        }
        const fixes = f.fixes.map((fix) => {
          const b = h('button', { class: 'btn btn--small', type: 'button', disabled: !!host.whyReadOnly() }, fix.label);
          b.addEventListener('click', () => (fix.ops.length ? host.apply(fix.ops, fix.label) : runAction(ctx, host, f, fix)));
          return b;
        });
        return h(
          'li',
          { class: `sheet-finding sheet-finding--${f.severity}`, dataset: { code: f.code } },
          h('div', { class: 'sheet-finding__head' }, icon(ICON[f.severity], 16), h('span', { class: 'sheet-finding__word' }, WORD[f.severity]), f.item ? h('span', { class: 'sheet-finding__item' }, name(f.item)) : null, f.placeholder ? h('span', { class: 'tbadge' }, 'değer bekliyor') : null),
          h('p', { class: 'sheet-finding__msg' }, f.message),
          f.fix ? h('p', { class: 'sheet-finding__fix' }, f.fix) : null,
          show || fixes.length ? h('div', { class: 'sheet-finding__actions' }, show, ...fixes) : null,
        );
      }),
    ),
  ];
}
