import type { Template } from '../../contracts/generated/sheet/Template';
import type { Variable } from '../../contracts/generated/sheet/Variable';
import type { VariableValue } from '../../contracts/generated/sheet/VariableValue';
import type { VarValue } from '../../contracts/generated/sheet/VarValue';
import { h } from '../dom';
import { toggleSwitch } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { missingMark } from '../../product/sheet/marks';
import { field } from './widgets/form';

/**
 * A template's questions, asked before a sheet is made from it
 * (docs/sheet/design.md §12: `Template.variables`, “kullanırken
 * sorulacaklar”: ada, parsel, mahalle …): one field per question, its
 * kind's (a line or several, a number, a date, yes/no), filled with the
 * template's own value. What is left empty is written “‹ad?›” on the paper
 * and the preflight says so; the Değişkenler window fills it later. Enter
 * makes the sheet (Ctrl+Enter in a text of several lines), Esc leaves it.
 * Resolves with the answers, or null when the user left.
 */

export const QUESTION_TEXTS = {
  title: 'Paftanın bilgileri',
  intro: (name: string) => `“${name}” şablonu bunları soruyor. Boş bıraktıklarınız paftada “${missingMark('ad')}” diye yazılır ve ön denetim hatırlatır; sonra Değişkenler'den doldurabilirsiniz.`,
  make: 'Paftayı oluştur',
  cancel: 'Vazgeç',
  empty: 'boş',
} as const;

interface Asked {
  readonly variable: Variable;
  read(): VarValue;
}

function ask(v: Variable, submit: () => void): { el: HTMLElement; asked: Asked; focus: HTMLElement } {
  const label = `${v.label || v.name} · @${v.name}`;
  const text = (x: VarValue) => (x === null || x === undefined ? '' : String(x));
  switch (v.kind) {
    case 'bool': {
      let on = v.value === true;
      const sw = toggleSwitch({ label, checked: on, onChange: (x) => (on = x) });
      return { el: field(label, h('div', { class: 'sheet-ask-q__bool' }, sw)), asked: { variable: v, read: () => on }, focus: sw };
    }
    case 'number': {
      const input = h('input', { class: 'sheet-field__input num', type: 'number', step: 'any', value: text(v.value), placeholder: QUESTION_TEXTS.empty, 'aria-label': label });
      input.addEventListener('keydown', (e) => e.key === 'Enter' && (e.preventDefault(), submit()));
      const read = () => (input.value.trim() === '' || !Number.isFinite(Number(input.value)) ? null : Number(input.value));
      return { el: field(label, h('div', { class: 'sheet-field__box' }, input)), asked: { variable: v, read }, focus: input };
    }
    case 'date': {
      const input = h('input', { class: 'sheet-field__input', type: 'date', value: text(v.value), 'aria-label': label });
      input.addEventListener('keydown', (e) => e.key === 'Enter' && (e.preventDefault(), submit()));
      return { el: field(label, h('div', { class: 'sheet-field__box' }, input)), asked: { variable: v, read: () => input.value || null }, focus: input };
    }
    default: {
      const long = typeof v.value === 'string' && v.value.includes('\n');
      if (long) {
        const area = h('textarea', { class: 'sheet-field__area', rows: '4', spellcheck: 'false', 'aria-label': label }) as HTMLTextAreaElement;
        area.value = text(v.value);
        area.addEventListener('keydown', (e) => e.key === 'Enter' && e.ctrlKey && (e.preventDefault(), submit()));
        return { el: field(label, area), asked: { variable: v, read: () => (area.value.trim() ? area.value : null) }, focus: area };
      }
      const input = h('input', { class: 'sheet-field__input', value: text(v.value), placeholder: QUESTION_TEXTS.empty, spellcheck: 'false', 'aria-label': label });
      input.addEventListener('keydown', (e) => e.key === 'Enter' && (e.preventDefault(), submit()));
      return { el: field(label, h('div', { class: 'sheet-field__box' }, input)), asked: { variable: v, read: () => (input.value.trim() ? input.value.trim() : null) }, focus: input };
    }
  }
}

/** Asks a template's questions; the answers in the template's order, or null when the window was left. A template that asks nothing answers []. */
export function askTemplateValues(template: Template, o: { stack?: boolean } = {}): Promise<VariableValue[] | null> {
  if (!template.variables.length) return Promise.resolve([]);
  return new Promise((resolve) => {
    let done = false;
    const finish = (v: VariableValue[] | null) => {
      if (done) return;
      done = true;
      dialog.close();
      resolve(v);
    };
    const submit = () => finish(asked.map((a) => ({ name: a.variable.name, value: a.read() })));
    const parts = template.variables.map((v) => ask(v, submit));
    const asked = parts.map((p) => p.asked);
    const make = h('button', { class: 'btn btn--primary', type: 'button' }, QUESTION_TEXTS.make);
    const cancel = h('button', { class: 'btn', type: 'button' }, QUESTION_TEXTS.cancel);
    make.addEventListener('click', submit);
    cancel.addEventListener('click', () => finish(null));
    const dialog = new Dialog({
      title: QUESTION_TEXTS.title,
      width: 520,
      className: 'sheet-ask-q',
      stack: o.stack,
      content: [h('p', { class: 'sheet-ask-q__intro' }, QUESTION_TEXTS.intro(template.meta.name)), h('div', { class: 'sheet-form sheet-ask-q__list' }, parts.map((p) => p.el))],
      footer: [h('div', { class: 'dialog__foot-spacer' }), cancel, make],
      onClose: () => {
        if (!done) {
          done = true;
          resolve(null);
        }
      },
    });
    queueMicrotask(() => parts[0]?.focus.focus());
  });
}
