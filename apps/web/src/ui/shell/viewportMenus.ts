import { commandItem, resolveMenu } from '../../app/menus';
import type { AppContext } from '../../app/context';
import type { Disposable } from '../../core/disposable';
import { SNAP_LABEL, type SnapKind } from '../../viewport/picking';
import { canCalcPoint } from '../../tools/pointCalc';
import { calcMenuItems } from './calcMenu';
import { gripItems } from './gripMenu';
import { choiceItems, parsePrompt, runPromptOption } from '../promptOptions';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';

/** Snap kinds offered as one-shot overrides, in the order surveyors reach for them. */
const SNAP_ORDER: SnapKind[] = ['endpoint', 'midpoint', 'intersection', 'center', 'perpendicular', 'tangent', 'quadrant', 'node', 'nearest'];

/** Menu icons drawn like the snap markers on the canvas. */
const SNAP_ICON: Record<SnapKind, string> = {
  endpoint: 'snapEndpoint',
  midpoint: 'snapMidpoint',
  center: 'snapCenter',
  node: 'snapNode',
  quadrant: 'snapQuadrant',
  intersection: 'snapIntersection',
  perpendicular: 'snapPerpendicular',
  tangent: 'snapTangent',
  nearest: 'snapNearest',
};

/**
 * Right-button menus over the drawing: the idle menu (with grip actions
 * when a grip is under the cursor), the command menu (hold the right
 * button while a command runs) and the one-shot snap menu (Shift + right).
 */
export function bindViewportMenus(ctx: AppContext): Disposable {
  return ctx.view.events.on('contextmenu', ({ clientX, clientY, screen, kind }) => {
    const at = { x: clientX, y: clientY };
    if (kind === 'snap') return PopupMenu.open(snapItems(ctx, true), at, { minWidth: 220 });
    if (kind === 'command') return PopupMenu.open(commandItems(ctx), at, { minWidth: 240 });
    PopupMenu.open([...gripItems(ctx, screen), ...idleItems(ctx)], at, { minWidth: 220 });
  });
}

function snapItems(ctx: AppContext, withHeader: boolean): MenuItem[] {
  const current = ctx.view.snapOverride.value;
  return [
    ...(withHeader ? [{ kind: 'header' as const, label: 'Tek seferlik kenet (sonraki tık)' }] : []),
    ...SNAP_ORDER.map((k): MenuItem => ({ label: SNAP_LABEL[k], icon: SNAP_ICON[k], checked: current === k, run: () => ctx.view.snapOverride.set(k) })),
    { kind: 'separator' },
    { label: 'Kenet ayarları…', icon: 'settings', run: () => ctx.commands.execute('tools.options') },
  ];
}

function commandItems(ctx: AppContext): MenuItem[] {
  const p = parsePrompt(ctx.tools.prompt.value);
  const options = p.options.filter((o) => o.key !== 'Enter' && o.key !== 'Esc');
  return [
    { kind: 'header', label: p.tool || 'Komut' },
    { label: 'Onayla / bitir', icon: 'check', shortcut: 'Enter', run: () => ctx.commands.execute('tool.confirm') },
    { label: 'İptal', icon: 'close', shortcut: 'Esc', run: () => ctx.commands.execute('tool.cancel') },
    ...(options.length
      ? [
          { kind: 'separator' as const },
          // An option that offers values (Yazı's Hiza, docs/adr/0145 §6) opens them as a submenu.
          ...options.map((o): MenuItem => {
            const label = o.value ? `${o.label}: ${o.value}` : o.label;
            const items = choiceItems(ctx, o.key);
            return items ? { label, hint: o.key, items } : { label, hint: o.key, run: () => runPromptOption(ctx, o.key) };
          }),
        ]
      : []),
    { kind: 'separator' },
    ...(canCalcPoint(ctx)
      ? [{ label: 'Nokta hesapla', icon: 'calc', items: () => calcMenuItems(ctx) }]
      : []),
    { label: 'Tek seferlik kenet', icon: 'snap', items: () => snapItems(ctx, false) },
    commandItem(ctx, 'draft.snap'),
    commandItem(ctx, 'draft.ortho'),
    commandItem(ctx, 'draft.polar'),
    commandItem(ctx, 'draft.tracking'),
    commandItem(ctx, 'draft.topology'),
    { kind: 'separator' },
    commandItem(ctx, 'view.zoomExtents'),
  ];
}

function idleItems(ctx: AppContext): MenuItem[] {
  const last = ctx.tools.lastToolLabel;
  const items = resolveMenu(ctx, [
    ...(last ? ['tool.repeat'] : []),
    '-',
    'view.zoomExtents',
    'view.zoomSelection',
    'tool.pan',
    '-',
    'edit.selectAll',
    'edit.deselect',
    '-',
    'tool.move',
    'tool.copy',
    'edit.copy',
    'edit.paste',
    'tool.erase',
    '-',
    'view.coords',
  ]).filter((it, i, arr) => !(it.kind === 'separator' && (i === 0 || arr[i - 1].kind === 'separator')));
  if (last && items[0]) items[0].label = `Yinele: ${last}`;
  return items;
}
