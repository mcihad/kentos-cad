import type { ExprField, ExprSection } from '../../model/expression/builder';
import type { FlowEdit, FlowNode, FlowType } from '../../model/expression/flow';
import { h } from '../dom';
import { icon } from '../icons';
import { highlighted } from './highlight';

/**
 * The selected node of the expression's flow (DESIGN.md §7.16, docs/adr/0101),
 * over the help in the builder's right column: a value to write, a field or
 * a `$` value to choose, another operator of the same kind, another
 * function, `değil` and `benzer`, an input to add or take away; the node's
 * inputs and what is wrong with it. Each change goes to the core as an edit.
 */
export interface FlowInspectorOptions {
  readonly fields: () => readonly ExprField[];
  /** The builder's tree, for the variables and functions to choose from. */
  readonly catalog: () => readonly ExprSection[];
  readonly edit: (change: FlowEdit) => void;
}

const TYPE_NAME: Record<FlowType, string> = { any: 'değer', number: 'sayı', text: 'metin', bool: 'koşul' };
const KIND_NAME: Record<FlowNode['kind'], string> = {
  result: 'Sonuç',
  field: 'Alan',
  variable: 'Değişken',
  number: 'Sayı',
  text: 'Metin',
  constant: 'Sabit',
  function: 'İşlev',
  operator: 'İşleç',
  keyword: 'Sözcük',
};

/** Operators that can take each other's place (the same two inputs, the same kind of answer). */
const FAMILIES: readonly (readonly string[])[] = [
  ['=', '!=', '<', '<=', '>', '>='],
  ['+', '-', '*', '/', '%', '^'],
  ['ve', 'veya'],
];

/** 'it''s' → it's */
const unquote = (s: string) => s.slice(1, -1).replace(/''/g, "'");

export class FlowInspector {
  readonly el: HTMLElement;
  private readonly opts: FlowInspectorOptions;
  private value: HTMLInputElement | null = null;

  constructor(opts: FlowInspectorOptions) {
    this.opts = opts;
    this.el = h('div', { class: 'xfi', hidden: true });
  }

  /** Puts the cursor in the value's field (a double click on a value node). */
  focusValue(): void {
    this.value?.focus();
    this.value?.select();
  }

  show(n: FlowNode | null): void {
    this.value = null;
    if (!n) {
      this.el.hidden = true;
      this.el.replaceChildren();
      return;
    }
    this.el.hidden = false;
    const parts: (Node | null)[] = [
      h('div', { class: 'xfi__kind' }, KIND_NAME[n.kind]),
      h('div', { class: 'xfi__title' }, h('code', null, ...highlighted(n.kind === 'result' ? 'Sonuç' : n.title)), h('span', { class: 'xfi__type', 'data-type': n.type }, TYPE_NAME[n.type])),
    ];
    parts.push(this.editor(n));
    if (n.error || n.warnings.length) {
      parts.push(
        h(
          'div',
          { class: 'xfi__problems' },
          n.error ? h('p', { class: 'xfi__error' }, icon('error', 14), h('span', null, n.error)) : null,
          ...n.warnings.map((w) => h('p', { class: 'xfi__warn' }, icon('warning', 14), h('span', null, w))),
        ),
      );
    }
    if (n.ports.length) parts.push(this.ports(n));
    if (n.kind === 'result') {
      parts.push(h('p', { class: 'xfi__hint' }, n.text ? 'Sonuç, girişine bağlı ifadenin değeridir; Tamam bu metni yazar.' : 'Bir düğümün çıkışını buraya bağlayın.'));
    } else {
      const remove = h('button', { class: 'btn btn--small xfi__remove', type: 'button' }, icon('trash', 14), 'Düğümü sil');
      remove.addEventListener('click', () => this.opts.edit({ op: 'remove', node: n.id }));
      parts.push(h('div', { class: 'xfi__actions' }, remove));
    }
    this.el.replaceChildren(...parts.filter((p): p is Node => p !== null));
  }

  /** What can be changed in place. */
  private editor(n: FlowNode): HTMLElement | null {
    const edit = this.opts.edit;
    if (n.kind === 'number' || n.kind === 'text') {
      const number = n.kind === 'number';
      const input = h('input', { class: 'field xfi__input', type: 'text', value: number ? n.title : unquote(n.title), spellcheck: 'false', 'aria-label': number ? 'Sayı' : 'Metin' });
      const msg = h('div', { class: 'xfi__msg', 'aria-live': 'polite' });
      // Enter and the field's change (on leaving it) both write: once.
      let written = false;
      const commit = () => {
        if (written) return;
        if (number) {
          const v = Number(input.value.trim());
          if (input.value.trim() === '' || !Number.isFinite(v)) {
            input.setAttribute('aria-invalid', 'true');
            msg.textContent = 'Bir sayı yazın; ondalık ayırıcı noktadır (12.5).';
            return;
          }
          if (v !== Number(n.title)) {
            written = true;
            edit({ op: 'setNumber', node: n.id, value: v });
          }
        } else if (input.value !== unquote(n.title)) {
          written = true;
          edit({ op: 'setText', node: n.id, value: input.value });
        }
      };
      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
          e.preventDefault();
          commit();
        }
      });
      input.addEventListener('change', commit);
      this.value = input;
      return h('label', { class: 'xfi__row' }, h('span', { class: 'xfi__label' }, number ? 'Sayı' : 'Metin'), input, msg);
    }
    if (n.kind === 'constant' && (n.title === 'doğru' || n.title === 'yanlış')) {
      return this.choices(['doğru', 'yanlış'], n.title, (v) => edit({ op: 'setBool', node: n.id, value: v === 'doğru' }), 'Değer');
    }
    if (n.kind === 'field') {
      const names = this.opts.fields().map((f) => f.name);
      if (!names.includes(n.title)) names.unshift(n.title);
      const select = h('select', { class: 'field xfi__input', 'aria-label': 'Alan' }, names.map((name) => h('option', { value: name, selected: name === n.title }, name)));
      select.addEventListener('change', () => edit({ op: 'setField', node: n.id, name: select.value }));
      return h('label', { class: 'xfi__row' }, h('span', { class: 'xfi__label' }, 'Alan'), select);
    }
    if (n.kind === 'variable') {
      const vars = this.opts.catalog().flatMap((s) => s.items.filter((i) => i.kind === 'variable'));
      const select = h('select', { class: 'field xfi__input', 'aria-label': 'Değişken' }, vars.map((v) => h('option', { value: v.label.slice(1), selected: v.label === n.title }, v.label)));
      select.addEventListener('change', () => edit({ op: 'setVariable', node: n.id, name: select.value }));
      return h('label', { class: 'xfi__row' }, h('span', { class: 'xfi__label' }, 'Değişken'), select);
    }
    if (n.kind === 'function') {
      const groups = this.opts.catalog().filter((s) => s.items.some((i) => i.kind === 'function'));
      const select = h(
        'select',
        { class: 'field xfi__input', 'aria-label': 'İşlev' },
        groups.map((g) =>
          h(
            'optgroup',
            { label: g.title },
            g.items.filter((i) => i.kind === 'function').map((f) => h('option', { value: f.label, selected: f.label === n.title }, f.label)),
          ),
        ),
      );
      select.addEventListener('change', () => edit({ op: 'setFunction', node: n.id, name: select.value }));
      return h('label', { class: 'xfi__row' }, h('span', { class: 'xfi__label' }, 'İşlev'), select);
    }
    const family = FAMILIES.find((f) => f.includes(n.title));
    if (family) return this.choices(family, n.title, (v) => edit({ op: 'setOperator', node: n.id, symbol: v }), 'İşleç');
    if (n.negated !== undefined) {
      const box = (label: string, on: boolean, change: (v: boolean) => void) => {
        const c = h('input', { type: 'checkbox', checked: on });
        c.addEventListener('change', () => change(c.checked));
        return h('label', { class: 'xfi__check' }, c, h('span', null, label));
      };
      const like = n.key === 'op:gibi' || n.key === 'op:benzer';
      return h(
        'div',
        { class: 'xfi__row' },
        box(n.key === 'op:boş' || n.key === 'op:boş değil' ? 'Boş değil mi (tersi)' : 'değil (tersi)', n.negated, (v) => edit({ op: 'setNegated', node: n.id, value: v })),
        like ? box('Harf farkı gözetme (benzer)', n.key === 'op:benzer', (v) => edit({ op: 'setFold', node: n.id, value: v })) : null,
      );
    }
    return null;
  }

  private choices(values: readonly string[], current: string, pick: (v: string) => void, label: string): HTMLElement {
    return h(
      'div',
      { class: 'xfi__row' },
      h('span', { class: 'xfi__label' }, label),
      h(
        'div',
        { class: 'xfi__seg', role: 'group', 'aria-label': label },
        values.map((v) => {
          const b = h('button', { class: 'xfi__opt', type: 'button', 'aria-pressed': String(v === current) }, v);
          b.addEventListener('click', () => v !== current && pick(v));
          return b;
        }),
      ),
    );
  }

  /** The node's inputs: what each takes and whether something is connected. */
  private ports(n: FlowNode): HTMLElement {
    const list = h(
      'ul',
      { class: 'xfi__ports' },
      n.ports.map((p, k) => {
        const state = p.from ? 'bağlı' : p.optional ? 'isteğe bağlı, boş' : 'boş';
        const remove = p.removable ? h('button', { class: 'ibtn xfi__premove', type: 'button', title: 'Bu girişi kaldır', 'aria-label': `${p.name} girişini kaldır` }, icon('close', 12)) : null;
        remove?.addEventListener('click', () => this.opts.edit({ op: 'removePort', node: n.id, port: k }));
        return h(
          'li',
          { class: 'xfi__port', 'data-on': p.from ? '' : null, 'data-required': !p.from && !p.optional ? '' : null },
          h('span', { class: 'xfi__pname' }, p.name),
          h('span', { class: 'xfi__ptype', 'data-type': p.type }, TYPE_NAME[p.type]),
          h('span', { class: 'xfi__pstate' }, state),
          remove,
          p.note ? h('span', { class: 'xfi__pnote' }, p.note) : null,
        );
      }),
    );
    const add = n.grows ? h('button', { class: 'btn btn--small', type: 'button' }, icon('plus', 14), n.key === 'op:durum' ? 'Koşul ekle' : 'Giriş ekle') : null;
    add?.addEventListener('click', () => this.opts.edit({ op: 'addPort', node: n.id }));
    return h('div', { class: 'xfi__section' }, h('h4', { class: 'xhelp__h' }, 'Girişler'), list, add);
  }
}
