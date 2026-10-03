import type { LayoutVariant } from '../../../contracts/generated/sheet/LayoutVariant';
import type { Sheet } from '../../../contracts/generated/sheet/Sheet';
import type { VariantCondition } from '../../../contracts/generated/sheet/VariantCondition';
import type { DisposableStore } from '../../../core/disposable';
import { fixed } from '../../../core/displayNumber';
import { h, type Child } from '../../dom';
import { icon } from '../../icons';
import { tooltip } from '../../widgets/tooltip';
import type { SheetHost } from '../host';
import { askText } from '../widgets/askText';

/**
 * Yerleşim düzenleri (docs/sheet/design.md §3.2a): the arrangements a sheet
 * keeps per paper (a title block across the foot on an upright paper, down
 * the side on a landscape one). Which one is in force is the engine's
 * choice for the paper (`activeVariant`: the first whose condition the paper
 * meets, else the base layout); while one is, moving and resizing write
 * into it. Here: the one in force, “Bu kâğıt için ayrı düzen oluştur”, and
 * each layout with its condition, its name (renamed) and Sil.
 */

const mm = (um: number) => fixed(um / 1000, 0);

/** A condition in words: “Dikey; genişlik 210–297 mm”. */
export function conditionText(w: VariantCondition): string {
  const parts = [w.orientation === 'landscape' ? 'Yatay' : 'Dikey'];
  const range = (label: string, a?: number, b?: number) => {
    if (a === undefined && b === undefined) return;
    if (a !== undefined && b !== undefined) parts.push(a === b ? `${label} ${mm(a)} mm` : `${label} ${mm(a)}–${mm(b)} mm`);
    else if (a !== undefined) parts.push(`${label} en az ${mm(a)} mm`);
    else parts.push(`${label} en çok ${mm(b!)} mm`);
  };
  range('genişlik', w.minWidth, w.maxWidth);
  range('yükseklik', w.minHeight, w.maxHeight);
  return parts.join('; ');
}

/** The name of the layout in force, or the base layout's. */
export function activeLayout(sheet: Sheet): { name: string; variant: LayoutVariant | null } {
  const v = sheet.activeVariant ? sheet.variants.find((x) => x.id === sheet.activeVariant) : undefined;
  return v ? { name: v.name, variant: v } : { name: 'Temel düzen', variant: null };
}

export function variantsSection(host: SheetHost, sheet: Sheet, d: DisposableStore, paperName: string): Child[] {
  const readOnly = host.whyReadOnly();
  const owner = { kind: 'sheet' as const, id: sheet.id };
  const active = activeLayout(sheet);
  const page = sheet.page;
  const orient = page.orientation === 'landscape' ? 'yatay' : 'dikey';
  // A layout for exactly this paper: its way round and its size.
  const make = h('button', { class: 'btn btn--small', type: 'button', disabled: readOnly !== null }, icon('plus', 14), 'Bu kâğıt için ayrı düzen oluştur');
  make.addEventListener('click', () => {
    const when: VariantCondition = { orientation: page.orientation, minWidth: page.size.width, maxWidth: page.size.width, minHeight: page.size.height, maxHeight: page.size.height };
    host.apply([{ op: 'addVariant', owner, id: host.newId(), name: `${paperName} ${orient}`, when, index: 0 }], 'Yerleşim düzeni ekle');
  });
  d.add(tooltip(make, () => ({ title: 'Bu kâğıt için ayrı düzen', description: 'Öğelerin şimdiki yerleri bu kâğıdın düzeni olur; bundan sonra bu kâğıtta yapılan taşıma ve boyutlandırma yalnız bu düzene yazılır, öbür kâğıtlarınki bozulmaz.', note: readOnly ?? undefined })));
  const rows = sheet.variants.map((v, i) => {
    const rename = h('button', { class: 'ibtn', type: 'button', 'aria-label': `${v.name}: ad ver`, disabled: readOnly !== null }, icon('edit', 14));
    rename.addEventListener('click', () => void askText({ title: 'Düzene ad ver', label: 'Ad', value: v.name }).then((name) => name && name !== v.name && host.apply([{ op: 'setVariant', owner, id: v.id, name }], 'Düzene ad ver')));
    const remove = h('button', { class: 'ibtn', type: 'button', 'aria-label': `${v.name}: sil`, disabled: readOnly !== null }, icon('trash', 14));
    remove.addEventListener('click', () => host.apply([{ op: 'removeVariant', owner, id: v.id }], `Düzeni sil: ${v.name}`));
    const up = h('button', { class: 'ibtn', type: 'button', 'aria-label': `${v.name}: öne al`, disabled: readOnly !== null || i === 0 }, icon('chevronUp', 14));
    up.addEventListener('click', () => host.apply([{ op: 'setVariant', owner, id: v.id, index: i - 1 }], 'Düzenin sırası'));
    d.add(tooltip(up, () => ({ title: 'Öne al', description: 'Kâğıdın koşuluna uyan ilk düzen geçerli olur: sıra önemlidir.' })));
    return h(
      'div',
      { class: 'sheet-variant', dataset: v.id === sheet.activeVariant ? { active: '' } : {} },
      h('div', { class: 'sheet-variant__text' }, h('span', { class: 'sheet-variant__name' }, v.name, v.id === sheet.activeVariant ? h('span', { class: 'tbadge tbadge--synced' }, 'geçerli') : null), h('span', { class: 'sheet-insp__hint' }, conditionText(v.when))),
      h('span', { class: 'sheet-variant__tools' }, up, rename, remove),
    );
  });
  return [
    h('div', { class: 'sheet-insp__row' }, h('span', { class: 'sheet-insp__label' }, 'Yerleşim düzeni'), h('span', null, active.variant ? `${active.name} (kendiliğinden)` : `${active.name}`)),
    rows.length ? h('div', { class: 'sheet-variants' }, rows) : null,
    h('div', { class: 'sheet-insp__actions' }, make),
    h('p', { class: 'sheet-insp__hint' }, 'Kâğıt değişince koşuluna uyan ilk düzen kendiliğinden geçerli olur; hiçbiri uymazsa temel düzen.'),
  ];
}
