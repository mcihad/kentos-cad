import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import type { MenuItem } from '../widgets/PopupMenu';
import type { SheetHost } from './host';

/**
 * The menus the sheet tabs, the Paftalar list, the item tree and the paper
 * share (docs/sheet/design.md §11): what “+” offers, what a sheet's tab
 * offers (Ad ver, Çoğalt, Sola taşı, Sağa taşı, Sil) and what an item
 * offers. Every row is a command (app/sheet/commands.ts), so a row that
 * cannot run now is dimmed and its command says why in its tooltip and hint.
 */

/** “+”: a new sheet, from a template, or a `.kpafta` file. */
export function newSheetItems(ctx: AppContext): MenuItem[] {
  return [
    commandItem(ctx, 'sheet.new', { detail: 'Çalışma modunun varsayılan şablonundan' }),
    commandItem(ctx, 'sheet.fromTemplate', { detail: 'Pafta şablonları: sistem, benim, kurumum, paylaşılan' }),
    { kind: 'separator' },
    commandItem(ctx, 'sheet.importKpafta', { detail: '.kpafta dosyasındaki paftalar ve varlıkları' }),
  ];
}

/** A sheet's tab (and its row in Paftalar): the commands on that sheet. */
export function sheetItems(ctx: AppContext, host: SheetHost, id: string): MenuItem[] {
  const sheets = host.state.book.value.sheets;
  const at = sheets.findIndex((s) => s.id === id);
  const sheet = sheets[at];
  if (!sheet) return [];
  const on = (cmd: string, extra: Partial<MenuItem> = {}): MenuItem => commandItem(ctx, cmd, { run: () => ctx.commands.execute(cmd, { id }), ...extra });
  const engine = host.state.whyNoEngine();
  return [
    { kind: 'header', label: sheet.name },
    on('sheet.open', { label: 'Aç', disabled: host.state.open.value === id }),
    { kind: 'separator' },
    on('sheet.rename'),
    on('sheet.duplicate'),
    on('sheet.moveLeft', { disabled: !!engine || at === 0 }),
    on('sheet.moveRight', { disabled: !!engine || at === sheets.length - 1 }),
    { kind: 'separator' },
    on('sheet.delete'),
  ];
}

/** An item's menu (the tree, the paper): the commands on the chosen items. */
export function itemItems(ctx: AppContext, host: SheetHost): MenuItem[] {
  const chosen = host.state.chosen;
  const hidden = chosen.length > 0 && chosen.every((i) => i.hidden);
  const locked = chosen.length > 0 && chosen.every((i) => i.locked);
  return [
    { kind: 'header', label: chosen.length === 1 ? chosen[0].name : `${chosen.length} öğe` },
    commandItem(ctx, 'sheet.renameItem', { disabled: chosen.length !== 1 || !ctx.commands.isEnabled('sheet.renameItem') }),
    commandItem(ctx, 'sheet.zoomSelection'),
    commandItem(ctx, 'sheet.duplicateItems'),
    { kind: 'separator' },
    commandItem(ctx, 'sheet.hideItems', { label: hidden ? 'Göster' : 'Gizle', icon: hidden ? 'eye' : 'eyeOff' }),
    commandItem(ctx, 'sheet.lockItems', { label: locked ? 'Kilidi aç' : 'Kilitle', icon: locked ? 'unlock' : 'lock' }),
    { kind: 'separator' },
    commandItem(ctx, 'sheet.order.front'),
    commandItem(ctx, 'sheet.order.forward'),
    commandItem(ctx, 'sheet.order.backward'),
    commandItem(ctx, 'sheet.order.back'),
    { kind: 'separator' },
    commandItem(ctx, 'sheet.group'),
    commandItem(ctx, 'sheet.ungroup'),
    { kind: 'separator' },
    commandItem(ctx, 'sheet.deleteItems'),
  ];
}

/** The paper with nothing under the pointer: the view and the selection. */
export function paperItems(ctx: AppContext): MenuItem[] {
  return [commandItem(ctx, 'sheet.zoomPage'), commandItem(ctx, 'sheet.zoomReal'), { kind: 'separator' }, commandItem(ctx, 'sheet.selectAll'), { kind: 'separator' }, commandItem(ctx, 'sheet.pageSetup')];
}
