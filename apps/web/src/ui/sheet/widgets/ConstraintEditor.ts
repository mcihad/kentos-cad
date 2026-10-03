import type { DisposableStore } from '../../../core/disposable';
import type { AnchorBox, AnchorsView, HAnchor, VAnchor } from '../../../product/sheet/view';
import { h } from '../../dom';
import { Dropdown } from '../../widgets/Dropdown';
import type { MenuItem } from '../../widgets/PopupMenu';
import { tooltip } from '../../widgets/tooltip';
import { BOX_ORDER, BOX_TEXT, H_ORDER, H_TEXT, V_ORDER, V_TEXT, clickPin, commonAnchors, fromH, fromV, pinState, toH, toV, type Pin } from './anchors';

/**
 * Kısıtlar (docs/sheet/design.md §3.2, §11): which distances an item keeps
 * when the paper changes, as a square drawing (the box, the item in it, a pin
 * on each of its four sides and its two middle lines) and three drop-downs
 * (Yatay, Düşey, Göre). A click on a pin takes that side; Shift+click keeps
 * both sides (the item stretches). With several items chosen a pin some of
 * them have is drawn dashed and a drop-down whose values differ says “—”;
 * a choice is given to all of them. The keys work as on any button (Tab,
 * Enter, Space; Shift+Enter is Shift+click).
 */

export interface ConstraintEditorOptions {
  /** The chosen items' anchors: one item's, or several. */
  readonly anchors: readonly AnchorsView[];
  /** The items are in a group: “Grup” may be the box. */
  readonly inGroup: boolean;
  /** Why the anchors cannot be changed now; null when they can. */
  readonly readOnly: string | null;
  onChange(patch: Partial<AnchorsView>): void;
}

const PIN_TEXT: Record<'h' | 'v', Record<Pin, { label: string; tip: string }>> = {
  h: {
    start: { label: 'Sol kenara bağla', tip: 'Sol kenara uzaklığı korunur. Shift ile sağ kenarla birlikte: öğe genişler.' },
    end: { label: 'Sağ kenara bağla', tip: 'Sağ kenara uzaklığı korunur. Shift ile sol kenarla birlikte: öğe genişler.' },
    center: { label: 'Yatayda ortaya bağla', tip: 'Ortasının kutunun ortasına yatay uzaklığı korunur.' },
  },
  v: {
    start: { label: 'Üst kenara bağla', tip: 'Üst kenara uzaklığı korunur. Shift ile alt kenarla birlikte: öğe uzar.' },
    end: { label: 'Alt kenara bağla', tip: 'Alt kenara uzaklığı korunur. Shift ile üst kenarla birlikte: öğe uzar.' },
    center: { label: 'Düşeyde ortaya bağla', tip: 'Ortasının kutunun ortasına düşey uzaklığı korunur.' },
  },
};

export function constraintEditor(o: ConstraintEditorOptions, d: DisposableStore): HTMLElement {
  const common = commonAnchors(o.anchors);
  const hs = o.anchors.map((a) => fromH[a.h]);
  const vs = o.anchors.map((a) => fromV[a.v]);
  const disabled = o.readOnly !== null;

  const pin = (axis: 'h' | 'v', p: Pin, cls: string) => {
    const state = pinState(axis === 'h' ? hs : vs, p);
    const t = PIN_TEXT[axis][p];
    const b = h('button', {
      class: `cedit__pin ${cls}`,
      type: 'button',
      'aria-label': t.label,
      'aria-pressed': state === 'mixed' ? 'mixed' : String(state),
      disabled,
      dataset: { pin: `${axis}-${p}` },
    });
    const take = (extend: boolean) => {
      if (axis === 'h') o.onChange({ h: toH[clickPin(common.h === null ? null : fromH[common.h], p, extend)] });
      else o.onChange({ v: toV[clickPin(common.v === null ? null : fromV[common.v], p, extend)] });
    };
    b.addEventListener('click', (e) => take(e.shiftKey));
    // Shift+Enter is Shift+click (the browser's Enter click carries no Shift).
    b.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && e.shiftKey) {
        e.preventDefault();
        take(true);
      }
    });
    d.add(tooltip(b, () => ({ title: t.label, description: t.tip, note: o.readOnly ?? undefined })));
    return b;
  };

  const square = h(
    'div',
    { class: 'cedit__square', role: 'group', 'aria-label': 'Kısıtların çizimi' },
    h('div', { class: 'cedit__item' }),
    pin('h', 'start', 'cedit__pin--h cedit__pin--left'),
    pin('h', 'end', 'cedit__pin--h cedit__pin--right'),
    pin('v', 'start', 'cedit__pin--v cedit__pin--top'),
    pin('v', 'end', 'cedit__pin--v cedit__pin--bottom'),
    pin('h', 'center', 'cedit__pin--cx'),
    pin('v', 'center', 'cedit__pin--cy'),
  );

  const choose = <T extends string>(label: string, order: readonly T[], text: Record<T, { label: string; detail: string }>, current: T | null, take: (v: T) => void, key: string) => {
    const dd = new Dropdown({
      ariaLabel: label,
      className: 'dropdown--cell',
      items: (): MenuItem[] => order.map((v) => ({ label: text[v].label, detail: text[v].detail, radio: true, checked: v === current, run: () => take(v) })),
    });
    dd.set(h('span', { class: 'dropdown__text' }, current === null ? '—' : text[current].label));
    dd.el.dataset.key = key;
    dd.el.disabled = disabled;
    if (current === null) dd.el.title = 'Seçili öğelerde farklı değerler var';
    const field = h('div', { class: 'sheet-field' }, h('span', { class: 'sheet-field__label' }, label), dd.el);
    // A drop-down that cannot be changed now does not open (the list's own press would open it, disabled or not).
    field.addEventListener(
      'pointerdown',
      (e) => {
        if (!dd.el.disabled) return;
        e.preventDefault();
        e.stopPropagation();
      },
      true,
    );
    if (o.readOnly) d.add(tooltip(field, () => ({ title: label, note: o.readOnly ?? undefined })));
    return field;
  };
  const boxes = o.inGroup ? BOX_ORDER : BOX_ORDER.filter((b) => b !== 'group');
  const fields = h(
    'div',
    { class: 'cedit__fields' },
    choose<HAnchor>('Yatay', H_ORDER, H_TEXT, common.h, (v) => o.onChange({ h: v }), 'h'),
    choose<VAnchor>('Düşey', V_ORDER, V_TEXT, common.v, (v) => o.onChange({ v }), 'v'),
    choose<AnchorBox>('Göre', boxes, BOX_TEXT, common.box, (v) => o.onChange({ box: v }), 'box'),
  );
  return h('div', { class: 'cedit', title: o.readOnly ?? null }, square, fields);
}
