import type { AlignSide, AlignTo, Distribute } from '../../style/svg/arrange';
import { h } from '../dom';
import { checkbox, row, select } from '../style/designerFields';
import type { EditActions } from './svgActions';
import { svgIcon } from './svgIcons';

/**
 * The Hizala tab of the SVG editor (Inkscape's Align and Distribute):
 * what to align against (the selection, the first or last chosen, the
 * biggest or smallest, the canvas), edges and centres, and even spacing of
 * edges, centres or gaps. A group counts as one shape.
 */

const TO: { value: AlignTo; label: string }[] = [
  { value: 'selection', label: 'Seçimin kutusu' },
  { value: 'first', label: 'İlk seçilen' },
  { value: 'last', label: 'Son seçilen' },
  { value: 'biggest', label: 'En büyük' },
  { value: 'smallest', label: 'En küçük' },
  { value: 'canvas', label: 'Tuval' },
];

export function alignTab(edit: EditActions, count: number): HTMLElement {
  const btn = (iconName: string, label: string, run: () => void, disabled = false) => {
    const b = h('button', { class: 'ibtn svgp__act svgp__act--big', type: 'button', title: label, 'aria-label': label, disabled }, svgIcon(iconName, 20));
    b.addEventListener('click', run);
    return b;
  };
  const a = (side: AlignSide, iconName: string, label: string) => btn(iconName, label, () => edit.align(side), !count);
  const d = (how: Distribute, iconName: string, label: string) => btn(iconName, label, () => edit.distribute(how), count < 3);
  return h(
    'div',
    { class: 'sdf__form' },
    h('div', { class: 'svgp__title' }, 'Hizala ve dağıt'),
    row('Göre', select(edit.ui.alignTo, TO, (v) => (edit.ui.alignTo = v), 'Neye göre hizalanır')),
    checkbox(edit.ui.alignAsOne, (v) => (edit.ui.alignAsOne = v), 'Seçimi tek parça olarak taşı'),
    h('div', { class: 'svgp__group' }, h('div', { class: 'sdf__grouptitle' }, 'Hizala'), h('div', { class: 'svgp__acts' }, a('left', 'alignLeft', 'Sol kenarlar'), a('hcenter', 'alignHCenter', 'Yatay ortalar'), a('right', 'alignRight', 'Sağ kenarlar')), h('div', { class: 'svgp__acts' }, a('top', 'alignTop', 'Üst kenarlar'), a('vcenter', 'alignVCenter', 'Dikey ortalar'), a('bottom', 'alignBottom', 'Alt kenarlar'))),
    h(
      'div',
      { class: 'svgp__group' },
      h('div', { class: 'sdf__grouptitle' }, 'Dağıt'),
      h('div', { class: 'svgp__acts' }, d('left', 'distLeft', 'Sol kenarlar eşit aralıkta'), d('hcenter', 'distHCenter', 'Yatay ortalar eşit aralıkta'), d('right', 'distRight', 'Sağ kenarlar eşit aralıkta'), d('hgap', 'distHGap', 'Yatayda eşit boşluk')),
      h('div', { class: 'svgp__acts' }, d('top', 'distTop', 'Üst kenarlar eşit aralıkta'), d('vcenter', 'distVCenter', 'Dikey ortalar eşit aralıkta'), d('bottom', 'distBottom', 'Alt kenarlar eşit aralıkta'), d('vgap', 'distVGap', 'Dikeyde eşit boşluk')),
    ),
    h('div', { class: 'sdf__hint' }, !count ? 'Şekil seçin; tek şekil tuvale hizalanır.' : count < 3 ? 'Dağıtmak için en az üç şekil (ya da grup) seçin; en dıştakiler yerinde kalır.' : 'En dıştakiler yerinde kalır, aradakiler eşit aralanır. Grup tek şekil sayılır.'),
  );
}
