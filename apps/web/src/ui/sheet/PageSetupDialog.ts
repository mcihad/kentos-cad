import type { AppContext } from '../../app/context';
import type { Orientation } from '../../contracts/generated/sheet/Orientation';
import type { Page } from '../../contracts/generated/sheet/Page';
import type { Paper } from '../../contracts/generated/sheet/Paper';
import { DisposableStore } from '../../core/disposable';
import { mm, um } from '../../product/sheet/adapter';
import { h, replaceChildren } from '../dom';
import { segmented, toggleSwitch } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import type { SheetHost } from './host';
import { numberField } from './widgets/fields';
import { choice, field } from './widgets/form';

/**
 * Sayfa ayarları (docs/sheet/design.md §3.2): the paper (A and B sizes from
 * the engine's table, or a size of one's own), its way round, its margins,
 * and whether the items follow their constraints onto it (on by default:
 * that is what makes a template usable on any paper). Uygula is one
 * `SetPage`, one undo step; the engine says why when it refuses.
 */
export function openPageSetup(_ctx: AppContext, host: SheetHost, sheetId: string): void {
  const engine = host.engine();
  const sheet = host.book()?.book.sheets.find((s) => s.id === sheetId);
  if (!engine || !sheet) return;
  const papers = engine.paperSizes();
  const p0 = sheet.page;
  const s = {
    paper: p0.paper as Paper,
    orientation: p0.orientation as Orientation,
    width: mm(p0.size.width),
    height: mm(p0.size.height),
    margins: { top: mm(p0.margins.top), right: mm(p0.margins.right), bottom: mm(p0.margins.bottom), left: mm(p0.margins.left) },
    relayout: true,
  };
  const d = new DisposableStore();
  /** The paper's size as it lies: the table's (turned for landscape), or the typed one for Özel. */
  const size = () => {
    const t = papers.find((x) => x.id === s.paper);
    if (!t) return { width: s.width, height: s.height };
    const [a, b] = [mm(t.width), mm(t.height)];
    return s.orientation === 'landscape' ? { width: Math.max(a, b), height: Math.min(a, b) } : { width: Math.min(a, b), height: Math.max(a, b) };
  };
  const apply = h('button', { class: 'btn btn--primary', type: 'button' }, 'Uygula');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const body = h('div', { class: 'sheet-form' });
  const render = () => {
    d.dispose();
    const custom = s.paper === 'custom';
    const sz = size();
    const num = (label: string, key: string, value: number, take: (v: number) => void, ro: string | null = null) =>
      numberField({ label, key, value, unit: 'mm', decimals: 1, min: label.includes('boşluk') ? 0 : 10, readOnly: ro, onCommit: (v) => (take(v), render()) }, d);
    replaceChildren(
      body,
      choice({ label: 'Kâğıt', value: s.paper, options: [...papers.map((x) => ({ value: x.id, label: x.name, detail: `${mm(x.width)} × ${mm(x.height)} mm` })), { value: 'custom' as Paper, label: 'Özel boy' }], onChange: (v) => ((s.paper = v), render()) }, d),
      field('Yön', segmented<Orientation>({ label: 'Yön', value: s.orientation, options: [{ value: 'portrait', label: 'Dikey' }, { value: 'landscape', label: 'Yatay' }], onChange: (v) => ((s.orientation = v), custom && ([s.width, s.height] = [s.height, s.width]), render()) })),
      h(
        'div',
        { class: 'sheet-fields' },
        num('Genişlik', 'w', sz.width, (v) => (s.width = v), custom ? null : 'Tablodaki kâğıdın boyu; başka bir boy için Özel boy seçin.'),
        num('Yükseklik', 'h', sz.height, (v) => (s.height = v), custom ? null : 'Tablodaki kâğıdın boyu; başka bir boy için Özel boy seçin.'),
      ),
      h('span', { class: 'sheet-field__label' }, 'Kenar boşlukları'),
      h(
        'div',
        { class: 'sheet-fields' },
        num('Sol boşluk', 'ml', s.margins.left, (v) => (s.margins.left = v)),
        num('Üst boşluk', 'mt', s.margins.top, (v) => (s.margins.top = v)),
        num('Sağ boşluk', 'mr', s.margins.right, (v) => (s.margins.right = v)),
        num('Alt boşluk', 'mb', s.margins.bottom, (v) => (s.margins.bottom = v)),
      ),
      h(
        'div',
        { class: 'sheet-insp__row' },
        h('span', { class: 'sheet-insp__label' }, 'Öğeler kısıtlarıyla yeni kâğıda yerleşsin'),
        toggleSwitch({ label: 'Öğeler kısıtlarıyla yeni kâğıda yerleşsin', checked: s.relayout, onChange: (v) => ((s.relayout = v), render()) }),
      ),
      h('p', { class: 'sheet-insp__hint' }, s.relayout ? 'Her öğe kısıtına göre taşınır: sağa bağlı antet sağda kalır, iki kenara bağlı harita esner; harita ölçeğini korur, kapsamını büyütür.' : 'Öğeler oldukları yerde kalır; yeni kâğıdın dışında kalan ön denetimde görünür.'),
    );
  };
  render();
  const dialog = new Dialog({ title: 'Sayfa ayarları', width: 460, className: 'sheet-dialog', content: [body], footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, apply], onClose: () => d.dispose() });
  cancel.addEventListener('click', () => dialog.close());
  apply.addEventListener('click', () => {
    const sz = size();
    const page: Page = {
      ...p0,
      paper: s.paper,
      orientation: s.orientation,
      size: { width: um(sz.width), height: um(sz.height) },
      margins: { top: um(s.margins.top), right: um(s.margins.right), bottom: um(s.margins.bottom), left: um(s.margins.left) },
    };
    if (host.apply([{ op: 'setPage', owner: { kind: 'sheet', id: sheetId }, page, relayout: s.relayout }], 'Sayfa ayarları')) dialog.close();
  });
}
