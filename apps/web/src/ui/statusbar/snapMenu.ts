import type { AppContext } from '../../app/context';
import { commandItem } from '../../app/menus';
import type { MenuItem } from '../widgets/PopupMenu';

/** The snap kinds' commands as the menu lists them: the old ones in the one-shot menu's order, then the additions. */
const KINDS = [
  'draft.snap.endpoint',
  'draft.snap.midpoint',
  'draft.snap.intersection',
  'draft.snap.center',
  'draft.snap.perpendicular',
  'draft.snap.tangent',
  'draft.snap.node',
  'draft.snap.nearest',
  'draft.snap.centroid',
  'draft.snap.extension',
  'draft.snap.parallel',
  'draft.snap.grid',
] as const;

/** Karelaj's usual spacings, metres: the same east and north (Netcad's Karelaj, AutoCAD's SNAP). */
export const GRID_SPACINGS = [0.1, 0.25, 0.5, 1, 2, 5, 10, 20, 50, 100] as const;

/** A spacing as set, not to the drawing's decimals: “0.25 m”, “1 m”. */
export const spacing = (v: number) => `${v} m`;

/**
 * The Kenet cell's right-click menu (docs/adr/0163 §6): the snap kinds, each a tick; Çizilmekte olan nesneye; Karelaj
 * aralığı's usual spacings (choosing one turns Karelaj on) and the rest on the settings' Kenetleme page. The desktop's
 * is `apps/desktop/src/snap_menu.rs`.
 */
export function snapMenu(ctx: AppContext): MenuItem[] {
  const p = ctx.prefs;
  const [east, north] = [p.snapGridEast.value, p.snapGridNorth.value];
  const spacings: MenuItem[] = [
    ...GRID_SPACINGS.map(
      (v): MenuItem => ({
        label: spacing(v),
        radio: true,
        checked: east === v && north === v,
        run: () => {
          p.snapGridEast.set(v);
          p.snapGridNorth.set(v);
          p.snapGrid.set(true);
        },
      }),
    ),
    { kind: 'separator' },
    { label: 'Farklı aralık…', icon: 'settings', run: () => ctx.commands.execute('tools.options', 'snap') },
  ];
  return [
    { kind: 'header', label: 'Kenet türleri' },
    // The command's short name, “Uç nokta”, not “Kenet: Uç nokta”, under the header; its marker beside the tick.
    ...KINDS.map((id) => {
      const cmd = ctx.commands.get(id);
      return commandItem(ctx, id, { label: cmd?.short, icon: cmd?.icon });
    }),
    { kind: 'separator' },
    commandItem(ctx, 'draft.snap.self', { label: 'Çizilmekte olan nesneye' }),
    { label: 'Karelaj aralığı', icon: 'snapGrid', hint: `${spacing(east)} × ${spacing(north)}`, items: spacings },
    { label: 'Kenet ayarları…', icon: 'settings', run: () => ctx.commands.execute('tools.options', 'snap') },
  ];
}
