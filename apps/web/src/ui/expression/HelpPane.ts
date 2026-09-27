import { DisposableStore, listen } from '../../core/disposable';
import { exprValues, type ExprHelp, type ExprValueItem } from '../../model/expression/builder';
import type { BuilderObjects } from '../../model/expression/builderObjects';
import { h } from '../dom';
import { highlighted } from './highlight';

/**
 * The builder's help (DESIGN.md §7.16): an entry's group, name, signature,
 * description, arguments (the optional ones marked), examples and other
 * names, all from the core. A field's help lists its values on request
 * (“Örnek değerler”, the first ten found; “Tüm değerler”); a double click
 * or Enter on a value puts it into the expression as the language writes it.
 */
export interface HelpPaneOptions {
  readonly objects?: BuilderObjects;
  readonly onInsert: (text: string) => void;
}

const SAMPLE = 10;
/** Rows listed at most; the rest are counted. */
const SHOWN = 2000;

const TYPE_NAME = { text: 'metin', number: 'sayı', bool: 'doğru/yanlış', date: 'tarih' } as const;
const SOURCE_NAME = { attribute: 'öznitelik', user: 'kullanıcının tanımladığı alan', builtin: 'yerleşik' } as const;

export class HelpPane {
  readonly el: HTMLElement;
  private readonly opts: HelpPaneOptions;
  private readonly d = new DisposableStore();
  private shown: string | null = null;

  constructor(opts: HelpPaneOptions) {
    this.opts = opts;
    this.el = h('div', { class: 'xhelp', 'aria-live': 'polite' });
    this.show(null);
  }

  dispose(): void {
    this.d.dispose();
  }

  /** Shows an entry's help; the same entry again leaves the pane (and its values) as it is. */
  show(help: ExprHelp | null): void {
    if (help && help.key === this.shown) return;
    this.shown = help?.key ?? null;
    if (!help) {
      this.el.replaceChildren(
        h('h3', { class: 'xhelp__title' }, 'Yardım'),
        h('p', { class: 'xhelp__desc' }, 'Ağaçtan bir öğe seçin ya da yazmaya başlayın: imlecin üstündeki adın yardımı burada görünür.'),
        h(
          'ul',
          { class: 'xhelp__keys' },
          h('li', null, h('kbd', null, 'Ctrl+Boşluk'), ' önerileri açar'),
          h('li', null, 'Çift tık ya da ', h('kbd', null, 'Enter'), ' öğeyi imlecin yerine ekler'),
          h('li', null, h('kbd', null, 'Ctrl+Enter'), ' Tamam'),
        ),
      );
      return;
    }
    const parts: (Node | null)[] = [
      h('div', { class: 'xhelp__group' }, help.group),
      h('h3', { class: 'xhelp__title' }, help.title),
      h('code', { class: 'xhelp__sig' }, ...highlighted(help.signature)),
      h('p', { class: 'xhelp__desc' }, help.description),
    ];
    if (help.type) {
      parts.push(h('p', { class: 'xhelp__type' }, `Türü: ${TYPE_NAME[help.type]}${help.source ? `; ${SOURCE_NAME[help.source]}` : ''}.`));
    }
    if (help.args.length) {
      parts.push(
        h('h4', { class: 'xhelp__h' }, 'Argümanlar'),
        h(
          'dl',
          { class: 'xhelp__args' },
          help.args.flatMap((a) => [h('dt', null, a.name, a.optional ? h('span', { class: 'xhelp__opt' }, 'isteğe bağlı') : null), h('dd', null, a.description)]),
        ),
      );
    }
    if (help.examples.length) {
      parts.push(
        h('h4', { class: 'xhelp__h' }, 'Örnekler'),
        h(
          'ul',
          { class: 'xhelp__ex' },
          help.examples.map((x) => h('li', null, h('code', null, ...highlighted(x.expression)), h('span', { class: 'xhelp__res' }, `→ ${x.result}`))),
        ),
      );
    }
    if (help.aliases.length) parts.push(h('p', { class: 'xhelp__aliases' }, `Öbür adları: ${help.aliases.join(', ')}`));
    if (help.kind === 'field') parts.push(this.values(help));
    this.el.replaceChildren(...parts.filter((p): p is Node => p !== null));
    this.el.scrollTop = 0;
  }

  /** A field's values, listed on request. */
  private values(help: ExprHelp): HTMLElement {
    const objects = this.opts.objects;
    const list = h('div', { class: 'xhelp__vlist', role: 'listbox', tabindex: '0', 'aria-label': `${help.title} değerleri`, hidden: true });
    const note = h('div', { class: 'xhelp__vnote' });
    const load = (limit?: number) => {
      if (!objects) return;
      const raw = objects.values(help.title, limit);
      const items = exprValues(raw, help.type ?? 'text');
      this.fill(list, items);
      note.textContent = items.length
        ? `${items.length} değer${items.length > SHOWN ? `; ilk ${SHOWN}'i listelendi` : ''}. Çift tık ya da Enter ifadeye ekler.`
        : 'Bu alanın nesnelerde değeri yok.';
    };
    const sample = h('button', { class: 'btn btn--small', type: 'button', disabled: !objects }, `Örnek değerler (${SAMPLE})`);
    const all = h('button', { class: 'btn btn--small', type: 'button', disabled: !objects }, 'Tüm değerler');
    sample.addEventListener('click', () => load(SAMPLE));
    all.addEventListener('click', () => load());
    if (!objects) note.textContent = 'Değerleri göstermek için nesne yok.';
    return h('div', { class: 'xhelp__values' }, h('h4', { class: 'xhelp__h' }, 'Değerler'), h('div', { class: 'xhelp__vbtns' }, sample, all), list, note);
  }

  private fill(list: HTMLElement, items: readonly ExprValueItem[]): void {
    let current = 0;
    const rows = items.slice(0, SHOWN).map((v, i) => {
      const row = h('div', { class: 'xhelp__value', role: 'option', 'aria-selected': String(i === 0), title: v.insert }, v.text);
      row.addEventListener('click', () => pick(i));
      row.addEventListener('dblclick', () => this.opts.onInsert(v.insert));
      return row;
    });
    const pick = (i: number) => {
      current = Math.max(0, Math.min(rows.length - 1, i));
      rows.forEach((r, k) => r.setAttribute('aria-selected', String(k === current)));
      rows[current]?.scrollIntoView({ block: 'nearest' });
    };
    list.replaceChildren(...rows);
    list.hidden = !rows.length;
    this.d.dispose();
    this.d.add(
      listen<KeyboardEvent>(list, 'keydown', (e) => {
        if (e.key === 'ArrowDown') pick(current + 1);
        else if (e.key === 'ArrowUp') pick(current - 1);
        else if (e.key === 'Enter' && items[current]) this.opts.onInsert(items[current].insert);
        else return;
        e.preventDefault();
        e.stopPropagation();
      }),
    );
  }
}
