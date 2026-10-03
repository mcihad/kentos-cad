import type { Bindable } from '../../contracts/generated/sheet/Bindable';
import type { Item } from '../../contracts/generated/sheet/Item';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import type { SheetHost } from './host';

/**
 * ƒ: a property's value from an expression (docs/sheet/design.md §7): what
 * may be bound and its unit are the engine's (`bindableProperties`), the
 * expression is compiled by the engine as it is typed (`checkExpression`,
 * kentos-expression: no language of its own here), and the names it may
 * read are listed to put in with a click: the sheet's and the project's
 * variables, the ready ones (@pafta_adi, @olcek, @tarih …) and an atlas
 * object's fields. Kaydet is one `SetItemProps` of the item's bindings;
 * “Bağı kaldır” takes the binding off and the property keeps its own value.
 */

const READY = ['@proje_adi', '@pafta_adi', '@sayfa', '@sayfa_sayisi', '@olcek', '@olcek_payda', '@kagit', '@tarih', '@kullanici', '@koordinat_sistemi'];

export function openExpressionDialog(host: SheetHost, sheetId: string, item: Item, property: Bindable): void {
  const engine = host.engine();
  const book = host.book()?.book;
  const sheet = book?.sheets.find((s) => s.id === sheetId);
  if (!engine || !book || !sheet) return;
  const bound = item.bindings.find((b) => b.property === property.property);
  // Esc leaves the window from the field too (nothing of the field's own to put back).
  const input = h('textarea', { class: 'sheet-field__area sheet-expr__input', rows: '4', spellcheck: 'false', 'aria-label': 'İfade' }) as HTMLTextAreaElement;
  input.value = bound?.expression ?? '';
  const status = h('div', { class: 'sheet-expr__status', role: 'status' });
  const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');
  const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
  const unbind = h('button', { class: 'btn btn--danger', type: 'button', hidden: !bound }, 'Bağı kaldır');
  const check = () => {
    const src = input.value.trim();
    if (!src) {
      replaceChildren(status, h('span', { class: 'sheet-insp__hint' }, 'Bir ifade yazın; aşağıdaki adlara tıklayınca yazılır.'));
      save.disabled = true;
      return;
    }
    const err = engine.checkExpression(src);
    save.disabled = !!err;
    replaceChildren(status, err ? h('span', { class: 'sheet-expr__bad' }, icon('error', 14), `${err.message} (${err.code})`) : h('span', { class: 'sheet-expr__ok' }, icon('success', 14), 'İfade geçerli.'));
  };
  const insert = (text: string) => {
    const at = input.selectionStart ?? input.value.length;
    const end = input.selectionEnd ?? at;
    input.value = input.value.slice(0, at) + text + input.value.slice(end);
    input.focus();
    input.setSelectionRange(at + text.length, at + text.length);
    check();
  };
  const chip = (text: string, title?: string) => {
    const b = h('button', { class: 'sheet-chip', type: 'button', title: title ?? null }, text);
    b.addEventListener('click', () => insert(text));
    return b;
  };
  const vars = [...sheet.variables, ...book.variables];
  const atlas = sheet.atlas?.layer;
  const dialog = new Dialog({
    title: `ƒ ${property.label}${property.unit ? ` (${property.unit})` : ''}: ${item.name}`,
    width: 560,
    className: 'sheet-dialog sheet-expr',
    content: [
      h('p', { class: 'sheet-insp__hint' }, `Değer bu ifadeden gelir (${property.value === 'number' ? 'sayı' : property.value === 'bool' ? 'evet/hayır' : 'metin'}${property.unit ? `, ${property.unit}` : ''}). Pafta her çizildiğinde yeniden hesaplanır; hesaplanamazsa ön denetim söyler.`),
      input,
      status,
      h('h3', { class: 'sheet-form__head' }, 'Hazır değişkenler'),
      h('div', { class: 'sheet-chips' }, READY.map((r) => chip(r))),
      vars.length ? h('h3', { class: 'sheet-form__head' }, 'Paftanın ve projenin değişkenleri') : null,
      vars.length ? h('div', { class: 'sheet-chips' }, vars.map((v) => chip(`@${v.name}`, v.label))) : null,
      atlas ? h('p', { class: 'sheet-insp__hint' }, `Atlas sayfasında “${atlas}” katmanının nesnesinin alanları da okunur: alan adını yalın ya da köşeli parantezle yazın ([Ada]).`) : null,
    ],
    footer: [unbind, h('div', { class: 'dialog__foot-spacer' }), cancel, save],
  });
  input.addEventListener('input', check);
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey) && !save.disabled) {
      e.preventDefault();
      save.click();
    }
  });
  const others = item.bindings.filter((b) => b.property !== property.property);
  cancel.addEventListener('click', () => dialog.close());
  unbind.addEventListener('click', () => host.apply([{ op: 'setItemProps', id: item.id, patch: { bindings: others } }], `Bağı kaldır: ${property.label}`) && dialog.close());
  save.addEventListener('click', () => host.apply([{ op: 'setItemProps', id: item.id, patch: { bindings: [...others, { property: property.property, expression: input.value.trim() }] } }], `Veriye bağla: ${property.label}`) && dialog.close());
  check();
  queueMicrotask(() => input.focus());
}
