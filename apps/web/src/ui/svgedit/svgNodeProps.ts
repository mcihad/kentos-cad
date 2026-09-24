import { nodeTypeOf } from '../../style/svg/nodeOps';
import { h, type Child } from '../dom';
import { numberInput } from '../style/designerFields';
import type { EditActions } from './svgActions';
import type { SvgCanvas } from './svgCanvas';
import { svgIcon } from './svgIcons';

/**
 * The node tool's box on top of the SVG editor's properties: what is
 * chosen, node types, adding, deleting, joining and breaking, segments to
 * lines or curves, corners rounded or cut (by dragging on the canvas or a
 * typed size) and nodes lined up. Every button says its key.
 */

export function nodeBox(edit: EditActions, canvas: SvgCanvas, refresh: () => void): HTMLElement {
  const tool = canvas.nodes;
  const s = tool.shape;
  const sel = tool.selected;
  const btn = (iconName: string, label: string, key: string, run: () => void, opts: { pressed?: boolean; disabled?: boolean } = {}) => {
    const b = h(
      'button',
      { class: 'ibtn svgp__act', type: 'button', title: key ? `${label} (${key})` : label, 'aria-label': label, disabled: opts.disabled, 'aria-pressed': opts.pressed === undefined ? null : String(opts.pressed) },
      svgIcon(iconName, 16),
    );
    b.addEventListener('click', run);
    return b;
  };
  const types = sel.length && s ? new Set(sel.map((r) => nodeTypeOf(s.subs[r.sub], r.index))) : new Set<string>();
  const type = types.size === 1 ? [...types][0] : null;
  const few = sel.length < 2;
  const count = s ? s.subs.reduce((k, sp) => k + sp.nodes.length, 0) : 0;
  const corner = (mode: 'fillet' | 'chamfer') => {
    tool.setMode(tool.mode === mode ? null : mode);
    refresh();
  };
  const parts: Child[] = [
    h('div', { class: 'sdf__grouptitle' }, 'Düğüm aracı'),
    h('div', { class: 'sdf__hint' }, sel.length ? `${sel.length} / ${count} düğüm seçili.` : `${count} düğüm. Tıklayın ya da boşlukta kutu çizin; Shift ekler.`),
    h(
      'div',
      { class: 'svgp__acts', role: 'group', 'aria-label': 'Düğüm türü' },
      btn('nodeCusp', 'Köşe düğüm', 'Shift+C', () => edit.nodeType('cusp'), { pressed: type === 'cusp', disabled: !sel.length }),
      btn('nodeSmooth', 'Yumuşak düğüm', 'Shift+S', () => edit.nodeType('smooth'), { pressed: type === 'smooth', disabled: !sel.length }),
      btn('nodeSymmetric', 'Simetrik düğüm', 'Shift+Y', () => edit.nodeType('symmetric'), { pressed: type === 'symmetric', disabled: !sel.length }),
      btn('nodeAuto', 'Otomatik düğüm', 'Shift+A', () => edit.nodeType('auto'), { pressed: type === 'auto', disabled: !sel.length }),
    ),
    h(
      'div',
      { class: 'svgp__acts', role: 'group', 'aria-label': 'Düğümler' },
      btn('nodeInsert', 'Seçili parçaların ortasına düğüm ekle', 'Insert', () => edit.nodeInsert(), { disabled: few }),
      btn('nodeDelete', 'Düğümü sil (biçim korunur; Ctrl+Delete korumadan)', 'Delete', () => edit.nodeDelete(true), { disabled: !sel.length }),
      btn('nodeJoin', 'Uç düğümleri birleştir', 'Shift+J', () => edit.nodeJoin(true), { disabled: few }),
      btn('nodeJoinSeg', 'Uçları parçayla birleştir', 'Shift+K', () => edit.nodeJoin(false), { disabled: few }),
      btn('nodeBreak', 'Düğümde kır', 'Shift+B', () => edit.nodeBreak(), { disabled: !sel.length }),
      btn('segDelete', 'İki düğüm arasındaki parçayı sil', 'Alt+Delete', () => edit.nodeDeleteSegment(), { disabled: few }),
      btn('segLine', 'Parçaları düz yap', 'Shift+L', () => edit.nodeSegments('line'), { disabled: few }),
      btn('segCurve', 'Parçaları eğri yap', 'Shift+U', () => edit.nodeSegments('curve'), { disabled: few }),
    ),
    h('div', { class: 'sdf__grouptitle' }, 'Köşe'),
    h(
      'div',
      { class: 'svgp__acts' },
      btn('fillet', 'Köşe yuvarla: köşeye basıp çekin', '', () => corner('fillet'), { pressed: tool.mode === 'fillet' }),
      btn('chamfer', 'Pah kır: köşeye basıp çekin', '', () => corner('chamfer'), { pressed: tool.mode === 'chamfer' }),
      h('span', { class: 'svgp__sep' }),
      numberInput(edit.ui.corner, (v) => (edit.ui.corner = Math.max(0, v)), { label: 'Yarıçap ya da pah boyu', step: 0.5, min: 0 }),
      btn('fillet', 'Seçili köşeleri bu yarıçapla yuvarla', '', () => edit.nodeCorner('fillet'), { disabled: !sel.length }),
      btn('chamfer', 'Seçili köşelere bu boyda pah kır', '', () => edit.nodeCorner('chamfer'), { disabled: !sel.length }),
    ),
    h('div', { class: 'sdf__hint' }, tool.mode ? 'Köşe halkayla işaretlenir; basıp kenar boyunca çekin, bırakınca uygulanır. Esc biter.' : 'Fareyle: düğmeyi basılı yapıp köşeye basın ve çekin. Kesin değer: yazıp yandaki düğme.'),
    h('div', { class: 'sdf__grouptitle' }, 'Düğümleri hizala'),
    h(
      'div',
      { class: 'svgp__acts' },
      btn('alignLeft', 'Solda hizala', '', () => edit.nodeAlign('x', 'min'), { disabled: few }),
      btn('alignHCenter', 'Yatayda ortala', '', () => edit.nodeAlign('x', 'mid'), { disabled: few }),
      btn('alignRight', 'Sağda hizala', '', () => edit.nodeAlign('x', 'max'), { disabled: few }),
      btn('alignTop', 'Üstte hizala', '', () => edit.nodeAlign('y', 'min'), { disabled: few }),
      btn('alignVCenter', 'Dikeyde ortala', '', () => edit.nodeAlign('y', 'mid'), { disabled: few }),
      btn('alignBottom', 'Altta hizala', '', () => edit.nodeAlign('y', 'max'), { disabled: few }),
      btn('distHCenter', 'Yatayda eşit dağıt', '', () => edit.nodeDistribute('x'), { disabled: sel.length < 3 }),
      btn('distVCenter', 'Dikeyde eşit dağıt', '', () => edit.nodeDistribute('y'), { disabled: sel.length < 3 }),
    ),
  ];
  return h('div', { class: 'svgp__group svgp__group--tool' }, parts);
}
