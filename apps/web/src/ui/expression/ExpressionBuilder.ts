import '../../styles/expression.css';
import { exprHelp, exprHelpAt, exprPlace, exprPreview, type ExprCheck, type ExprItem, type ExprItemKind } from '../../model/expression/builder';
import { h } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { BuilderTree } from './BuilderTree';
import type { ExpressionBuilderOptions } from './builderApi';
import { CodeEditor } from './CodeEditor';
import { HelpPane } from './HelpPane';

/**
 * The expression builder (DESIGN.md §7.16, docs/adr/0100 §5), modelled on
 * QGIS's expression dialog: on the left the editor (colours, completion,
 * the call's signature, the error where it is), the operators above it and
 * a live preview on the objects under it, one object at a time; in the
 * middle the searchable tree of fields, values and functions; on the right
 * the help of what is selected or under the cursor. It stacks over the
 * window it was opened from; Tamam writes the text back, Vazgeç (Esc, ×)
 * leaves the field as it was. The desktop's builder is the same (the
 * language services are one core).
 */
export function showBuilder(opts: ExpressionBuilderOptions): void {
  new ExpressionBuilder(opts);
}

/** The operator buttons over the editor: what they insert and how the core places it. */
const OPERATORS: readonly { label: string; key?: string; insert: string; kind: ExprItemKind }[] = [
  { label: '=', key: 'op:=', insert: ' = ', kind: 'operator' },
  { label: '!=', key: 'op:!=', insert: ' != ', kind: 'operator' },
  { label: '<', key: 'op:<', insert: ' < ', kind: 'operator' },
  { label: '>', key: 'op:>', insert: ' > ', kind: 'operator' },
  { label: '+', key: 'op:+', insert: ' + ', kind: 'operator' },
  { label: '-', key: 'op:-', insert: ' - ', kind: 'operator' },
  { label: '*', key: 'op:*', insert: ' * ', kind: 'operator' },
  { label: '/', key: 'op:/', insert: ' / ', kind: 'operator' },
  { label: '^', key: 'op:^', insert: ' ^ ', kind: 'operator' },
  { label: '||', key: 'op:||', insert: ' || ', kind: 'operator' },
  { label: '(', insert: '(', kind: 'operator' },
  { label: ')', insert: ')', kind: 'operator' },
  { label: 've', key: 'op:ve', insert: ' ve ', kind: 'keyword' },
  { label: 'veya', key: 'op:veya', insert: ' veya ', kind: 'keyword' },
  { label: 'değil', key: 'op:değil', insert: ' değil ', kind: 'keyword' },
];

class ExpressionBuilder {
  private readonly opts: ExpressionBuilderOptions;
  private readonly dialog: Dialog;
  private readonly editor: CodeEditor;
  private readonly tree: BuilderTree;
  private readonly help: HelpPane;
  private readonly status: HTMLElement;
  private readonly value: HTMLElement;
  private readonly count: HTMLElement;
  private readonly who: HTMLElement;
  private readonly prev: HTMLButtonElement;
  private readonly next: HTMLButtonElement;
  private readonly ok: HTMLButtonElement;
  private index = 0;
  private error = false;

  constructor(opts: ExpressionBuilderOptions) {
    this.opts = opts;
    const fields = () => opts.fields;
    this.editor = new CodeEditor({
      value: opts.value,
      label: 'İfade',
      fields,
      onChange: (_, check) => this.changed(check),
      onCursor: (at) => this.cursor(at),
      onSubmit: () => this.accept(),
      onActive: (item) => (item ? this.help.show(exprHelp(item.key, fields())) : this.cursor(this.editor.cursor)),
    });
    this.tree = new BuilderTree({
      fields,
      onSelect: (item) => this.help.show(exprHelp(item.key, fields())),
      onInsert: (item) => this.insert(item),
    });
    this.help = new HelpPane({ objects: opts.objects, onInsert: (text) => this.put(text, 'field', text.length) });
    this.status = h('div', { class: 'exprb__status', role: 'status' });
    this.value = h('span', { class: 'exprb__pvalue' });
    this.count = h('span', { class: 'exprb__stepn' });
    this.who = h('span', { class: 'exprb__who' });
    this.prev = h('button', { class: 'ibtn exprb__prev', type: 'button', 'aria-label': 'Önceki nesne', title: 'Önceki nesne' }, icon('chevronRight', 14));
    this.next = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Sonraki nesne', title: 'Sonraki nesne' }, icon('chevronRight', 14));
    this.prev.addEventListener('click', () => this.step(-1));
    this.next.addEventListener('click', () => this.step(1));
    this.ok = h('button', { class: 'btn btn--primary', type: 'button' }, 'Tamam');
    this.ok.addEventListener('click', () => this.accept());
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    cancel.addEventListener('click', () => this.dialog.request());

    const ops = h(
      'div',
      { class: 'exprb__ops', role: 'toolbar', 'aria-label': 'İşleçler' },
      OPERATORS.map((o) => {
        const tip = o.key ? exprHelp(o.key, [])?.description : 'Parantez';
        const b = h('button', { class: `exprb__op${o.kind === 'keyword' ? ' exprb__op--word' : ''}`, type: 'button', title: tip ?? o.label }, o.label);
        // The editor keeps the focus and its selection.
        b.addEventListener('mousedown', (e) => e.preventDefault());
        b.addEventListener('click', () => this.put(o.insert, o.kind, o.insert.length));
        return b;
      }),
    );
    const objects = opts.objects;
    const preview = h(
      'div',
      { class: 'exprb__preview' },
      h('span', { class: 'exprb__plabel' }, 'Önizleme'),
      this.value,
      objects && objects.count > 0 ? h('div', { class: 'exprb__step' }, h('span', { class: 'exprb__steplabel' }, 'Nesne'), this.prev, this.count, this.next, this.who) : null,
    );
    const main = h('div', { class: 'exprb__main' }, ops, this.editor.el, this.status, preview);
    const title = opts.context ? `İfade oluşturucu · ${opts.context}` : 'İfade oluşturucu';
    this.dialog = new Dialog({
      title,
      className: 'dialog--exprb',
      stack: true,
      content: [h('div', { class: 'exprb' }, main, this.tree.el, this.help.el)],
      footer: [h('span', { class: 'exprb__hint' }, 'Ctrl+Boşluk: öneriler · Ctrl+Enter: Tamam'), h('div', { class: 'dialog__spacer' }), cancel, this.ok],
      onClose: () => this.dispose(),
    });
    this.changed(this.editor.checked);
    const end = this.editor.value.length;
    this.editor.focus();
    this.editor.input.setSelectionRange(end, end);
  }

  private dispose(): void {
    this.editor.dispose();
    this.tree.dispose();
    this.help.dispose();
  }

  private changed(check: ExprCheck): void {
    this.error = !!check.error && this.editor.value.trim() !== '';
    const e = check.error;
    const w = check.warnings;
    if (e && this.editor.value.trim() !== '') {
      this.status.className = 'exprb__status exprb__status--error';
      this.status.replaceChildren(icon('error', 14), h('span', null, e.text));
    } else if (w.length) {
      this.status.className = 'exprb__status exprb__status--warn';
      this.status.replaceChildren(icon('warning', 14), h('span', null, w[0].text, w.length > 1 ? ` (${w.length - 1} uyarı daha)` : ''));
    } else {
      this.status.className = 'exprb__status';
      this.status.replaceChildren();
    }
    this.ok.disabled = this.error;
    this.refreshPreview();
  }

  private cursor(at: number): void {
    // The help follows the cursor while one writes; the tree's choice stays until then.
    if (document.activeElement !== this.editor.input) return;
    const help = exprHelpAt(this.editor.value, at, this.opts.fields);
    if (help) this.help.show(help);
  }

  private step(by: number): void {
    const n = this.opts.objects?.count ?? 0;
    if (!n) return;
    this.index = (this.index + by + n) % n;
    this.refreshPreview();
  }

  private refreshPreview(): void {
    const objects = this.opts.objects;
    const src = this.editor.value;
    this.value.classList.remove('exprb__pvalue--dim');
    if (!objects || objects.count === 0) {
      this.value.textContent = 'Önizleme için nesne yok.';
      this.value.classList.add('exprb__pvalue--dim');
      return;
    }
    this.count.textContent = `${this.index + 1} / ${objects.count}`;
    this.who.textContent = objects.describe(this.index);
    this.who.title = this.who.textContent;
    this.prev.disabled = this.next.disabled = objects.count < 2;
    if (!src.trim() || this.error) {
      this.value.textContent = '—';
      this.value.classList.add('exprb__pvalue--dim');
      return;
    }
    const r = objects.value(src, this.index);
    if ('error' in r) {
      this.value.textContent = r.error;
      this.value.classList.add('exprb__pvalue--dim');
      return;
    }
    this.value.textContent = exprPreview(r.value);
    this.value.title = this.value.textContent;
  }

  /** Puts an entry of the tree over the editor's selection. */
  private insert(item: ExprItem): void {
    this.put(item.insert, item.kind, item.caret);
  }

  /** Places text over the selection as the core says (a function wraps it, an operator keeps single spaces). */
  private put(text: string, kind: ExprItemKind, caret: number): void {
    const input = this.editor.input;
    const src = input.value;
    const [start, end] = [input.selectionStart, input.selectionEnd];
    const r = exprPlace(src, start, end, kind, text, caret);
    const after = src.length - end;
    this.editor.replace(start, end, r.text.slice(start, r.text.length - after), r.caret - start);
  }

  private accept(): void {
    if (this.error) return;
    const value = this.editor.value;
    this.dialog.close();
    this.opts.onOk(value);
  }
}
