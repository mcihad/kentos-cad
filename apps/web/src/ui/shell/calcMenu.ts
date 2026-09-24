import type { AppContext } from '../../app/context';
import { CALC_KINDS, startPointCalc } from '../../tools/pointCalc';
import type { MenuItem } from '../widgets/PopupMenu';

/**
 * Menu rows for the point calculator (command bar button, right-button
 * menu): an icon drawn like the construction, and a line saying what it
 * computes and where to click.
 */
export function calcMenuItems(ctx: AppContext): MenuItem[] {
  return [
    { kind: 'header', label: 'Komut nokta beklerken ölçülerden nokta hesaplar' },
    ...CALC_KINDS.map((k): MenuItem => ({ label: k.label, icon: k.icon, detail: k.description, hint: k.alias, run: () => startPointCalc(ctx, k.kind) })),
  ];
}
