import type { AppContext } from '../../app/context';
import type { Op } from '../../contracts/generated/sheet/Op';
import type { Variable } from '../../contracts/generated/sheet/Variable';
import type { VarValue } from '../../contracts/generated/sheet/VarValue';
import { DisposableStore } from '../../core/disposable';
import { h, replaceChildren } from '../dom';
import { note, toggleSwitch } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import type { SheetHost } from './host';
import { askText } from './widgets/askText';
import { numberField } from './widgets/fields';
import { missingMark } from '../../product/sheet/marks';
import { field, textInput } from './widgets/form';

/**
 * Değişkenler (docs/sheet/design.md §7): the values written `[% @ad %]` in the
 * sheet's texts and title block cells: the sheet's own (ada, parsel,
 * mahalle … a template's questions) and the project's (proje no, idare …),
 * the sheet's found first. One with no value is written ‹ad?› on the paper
 * and the preflight says so. Kaydet is one step for both lists.
 */
export function openVariables(_ctx: AppContext, host: SheetHost, sheetId: string): void {
  const book = host.book()?.book;
  const sheet = book?.sheets.find((s) => s.id === sheetId);
  if (!book || !sheet) return;
  const own = sheet.variables.map((v) => ({ ...v }));
  const project = book.variables.map((v) => ({ ...v }));
  const d = new DisposableStore();
  const body = h('div', { class: 'sheet-form sheet-vars' });

  const valueField = (v: Variable, key: string) => {
    const set = (x: VarValue) => {
      v.value = x;
      render();
    };
    const label = `${v.label || v.name} · @${v.name}`;
    switch (v.kind) {
      case 'number':
        return numberField({ label, key, value: typeof v.value === 'number' ? v.value : null, decimals: 2, readOnly: null, wide: true, onCommit: set }, d);
      case 'bool':
        return field(label, toggleSwitch({ label, checked: v.value === true, onChange: (x) => set(x) }));
      case 'date': {
        const input = h('input', { class: 'sheet-field__input', type: 'date', value: typeof v.value === 'string' ? v.value : '', 'aria-label': label, dataset: { key } });
        input.addEventListener('change', () => set(input.value || null));
        return field(label, h('div', { class: 'sheet-field__box' }, input));
      }
      default:
        return textInput({ label, key, value: typeof v.value === 'string' ? v.value : v.value === null ? '' : String(v.value), placeholder: 'değeri yok', onCommit: (x) => set(x.trim() ? x : null) }, d);
    }
  };
  const list = (title: string, vars: Variable[], scope: string, empty: string) => [
    h('h3', { class: 'sheet-form__head' }, title),
    vars.length ? h('div', { class: 'sheet-vars__list' }, vars.map((v) => valueField(v, `${scope}.${v.name}`))) : h('p', { class: 'sheet-insp__hint' }, empty),
  ];
  const addButton = (target: Variable[], where: string) => {
    const b = h('button', { class: 'btn btn--small', type: 'button' }, `${where} değişkeni ekle…`);
    b.addEventListener('click', async () => {
      const name = await askText({ title: `${where} değişkeni`, label: 'Ad (harf, rakam ve _; metinde @ad)', value: '', stack: true });
      if (!name) return;
      const clean = name.replace(/^@/, '');
      if (target.some((v) => v.name === clean)) return;
      target.push({ name: clean, label: clean, kind: 'text', value: null });
      render();
    });
    return b;
  };
  const missing = () => [...own, ...project].filter((v) => v.value === null).length;
  const render = () => {
    d.dispose();
    const n = missing();
    replaceChildren(
      body,
      n ? note('warn', `${n} değişkenin değeri yok: kâğıda ${missingMark('ad')} yazılır ve ön denetim hata verir.`) : null,
      ...list(`Paftanın değişkenleri (${sheet.name})`, own, 's', 'Bu paftanın kendi değişkeni yok. Şablondan yapılan paftalar sorularını burada taşır.'),
      h('div', null, addButton(own, 'Pafta')),
      ...list('Projenin değişkenleri', project, 'p', 'Projenin değişkeni yok: bütün paftalarda aynı olan değerler (proje no, idare …) için ekleyin.'),
      h('div', null, addButton(project, 'Proje')),
      h('p', { class: 'sheet-insp__hint' }, 'Hazır değişkenler kendiliğinden dolar: @proje_adi, @pafta_adi, @sayfa, @sayfa_sayisi, @olcek, @tarih, @kullanici, @koordinat_sistemi.'),
    );
  };
  render();
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const dialog = new Dialog({ title: 'Değişkenler', width: 520, className: 'sheet-dialog', content: [body], footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, save], onClose: () => d.dispose() });
  cancel.addEventListener('click', () => dialog.close());
  save.addEventListener('click', () => {
    // A field still being typed in is taken first.
    (document.activeElement as HTMLElement | null)?.blur?.();
    const ops: Op[] = [
      { op: 'saveVariables', sheet: sheetId, variables: own },
      { op: 'saveVariables', variables: project },
    ];
    if (host.apply(ops, 'Değişkenler')) dialog.close();
  });
}
