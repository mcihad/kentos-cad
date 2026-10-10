import '../../styles/expression.css';
import { exprBuilderCatalog, exprHelp, exprHelpAt, exprPlace, exprPreview, type ExprCheck, type ExprItem, type ExprItemKind, type ExprSchema, type ExprSection } from '../../model/expression/builder';
import { h } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { BuilderTree } from './BuilderTree';
import type { ExpressionBuilderOptions } from './builderApi';
import { CodeEditor } from './CodeEditor';
import { FlowMode } from './FlowMode';
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
 *
 * Two views of one text (docs/adr/0101): Metin, the editor; Akış, the same
 * expression as nodes (FlowMode). A change in either is the other's at once.
 */
export function showBuilder(opts: ExpressionBuilderOptions): void {
  new ExpressionBuilder(opts);
}

/** The values the flow's palette offers besides the tree's entries: a number and a text to write. */
const VALUES: ExprSection = {
  group: 'values',
  title: 'Sabit değerler',
  items: [
    { kind: 'operator', label: 'Sayı', detail: 'Yazılacak bir sayı: 0', insert: '0', caret: 1, key: 'lit:number' },
    { kind: 'operator', label: 'Metin', detail: "Yazılacak bir metin: ''", insert: "''", caret: 1, key: 'lit:text' },
  ],
};

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

/** The view the builder opened in last (for as long as the page is open). */
let lastMode: 'text' | 'flow' = 'text';

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
  private readonly flow: FlowMode;
  private readonly tabs: HTMLButtonElement[];
  private readonly parens: HTMLButtonElement[] = [];
  /** The keys note at the foot: the text's or the flow's. */
  private readonly hint = h('span', { class: 'exprb__hint' });
  private mode: 'text' | 'flow' = 'text';
  private index = 0;
  private error = false;

  constructor(opts: ExpressionBuilderOptions) {
    this.opts = opts;
    const fields = () => opts.fields;
    // The services' view: the fields, the `@` values, whether other layers are given (docs/adr/0214).
    const schema = (): ExprSchema => ({ fields: opts.fields, variables: opts.variables ?? [], world: opts.world === true });
    this.editor = new CodeEditor({
      value: opts.value,
      label: 'İfade',
      fields,
      schema,
      onChange: (_, check) => this.changed(check),
      onCursor: (at) => this.cursor(at),
      onSubmit: () => this.accept(),
      onActive: (item) => (item ? this.help.show(exprHelp(item.key, schema())) : this.cursor(this.editor.cursor)),
    });
    this.tree = new BuilderTree({
      fields,
      schema,
      onSelect: (item) => this.help.show(exprHelp(item.key, schema())),
      onInsert: (item) => (this.mode === 'flow' ? this.flow.add(item.key) : this.insert(item)),
      extra: () => (this.mode === 'flow' ? [VALUES] : []),
    });
    this.help = new HelpPane({ objects: opts.objects, onInsert: (text) => (this.mode === 'flow' ? this.flow.addText(text) : this.put(text, 'field', text.length)) });
    this.flow = new FlowMode({
      fields,
      schema,
      catalog: () => exprBuilderCatalog(schema(), ''),
      text: () => this.editor.value,
      setText: (text) => this.editor.setValue(text),
      value: (text) => this.previewOf(text),
      showHelp: (key) => this.help.show(key ? exprHelp(key, schema()) : null),
      fail: (message) => this.fail(message),
    });
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
        b.addEventListener('click', () => {
          if (this.mode === 'text') this.put(o.insert, o.kind, o.insert.length);
          else if (o.key) this.flow.add(o.key);
        });
        if (!o.key) this.parens.push(b);
        return b;
      }),
    );
    // Metin | Akış: the same text, written or as nodes.
    this.tabs = (['text', 'flow'] as const).map((m) => {
      const b = h(
        'button',
        { class: 'exprb__tab', type: 'button', role: 'tab', 'aria-selected': 'false', title: m === 'text' ? 'İfadeyi yazarak düzenle' : 'İfadeyi düğümlerle düzenle' },
        icon(m === 'text' ? 'expression' : 'modelNew', 15),
        m === 'text' ? 'Metin' : 'Akış',
      );
      b.addEventListener('click', () => this.setMode(m));
      return b;
    });
    const modes = h('div', { class: 'exprb__modes', role: 'tablist', 'aria-label': 'Görünüm' }, ...this.tabs);
    const objects = opts.objects;
    const preview = h(
      'div',
      { class: 'exprb__preview' },
      h('span', { class: 'exprb__plabel' }, 'Önizleme'),
      this.value,
      objects && objects.count > 0 ? h('div', { class: 'exprb__step' }, h('span', { class: 'exprb__steplabel' }, 'Nesne'), this.prev, this.count, this.next, this.who) : null,
    );
    this.flow.view.el.hidden = true;
    this.flow.view.el.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        this.accept();
      }
    });
    const main = h('div', { class: 'exprb__main' }, modes, ops, this.editor.el, this.flow.view.el, this.status, preview);
    const title = opts.context ? `İfade oluşturucu · ${opts.context}` : 'İfade oluşturucu';
    this.dialog = new Dialog({
      title,
      className: 'dialog--exprb',
      stack: true,
      content: [h('div', { class: 'exprb' }, main, this.tree.el, h('div', { class: 'exprb__side' }, this.flow.inspector.el, this.help.el))],
      footer: [this.hint, h('div', { class: 'dialog__spacer' }), cancel, this.ok],
      onClose: () => this.dispose(),
    });
    this.changed(this.editor.checked);
    this.setMode(lastMode);
    if (this.mode === 'text') {
      const end = this.editor.value.length;
      this.editor.focus();
      this.editor.input.setSelectionRange(end, end);
    }
  }

  private dispose(): void {
    this.editor.dispose();
    this.tree.dispose();
    this.help.dispose();
    this.flow.dispose();
  }

  /** Shows the text or the flow of the same expression. */
  private setMode(mode: 'text' | 'flow'): void {
    this.mode = lastMode = mode;
    this.dialog.el.querySelector('.dialog')?.classList.toggle('dialog--exprb-flow', mode === 'flow');
    this.tabs.forEach((t, i) => t.setAttribute('aria-selected', String((i === 0) === (mode === 'text'))));
    this.editor.el.hidden = mode !== 'text';
    this.flow.view.el.hidden = mode !== 'flow';
    this.parens.forEach((b) => (b.disabled = mode === 'flow'));
    this.hint.textContent = mode === 'flow' ? 'Sürükle: bağla · Delete: düğümü sil · Ctrl+Z: geri al · Ctrl+Enter: Tamam' : 'Ctrl+Boşluk: öneriler · Ctrl+Enter: Tamam';
    this.flow.inspector.el.hidden = true;
    this.tree.refresh();
    this.help.flow = mode === 'flow';
    this.help.show(null);
    if (mode === 'flow') {
      this.flow.view.refit();
      this.flow.refresh();
      this.flow.view.focus();
    } else {
      this.editor.focus();
    }
  }

  /** A change the flow could not make: in the status line until the next change. */
  private fail(message: string): void {
    this.status.className = 'exprb__status exprb__status--error';
    this.status.replaceChildren(icon('error', 14), h('span', null, message));
  }

  /** An expression's value on the previewed object, as the preview writes it. */
  private previewOf(text: string): string | undefined {
    const objects = this.opts.objects;
    if (!objects || objects.count === 0) return undefined;
    const r = objects.value(text, this.index);
    return 'error' in r ? undefined : exprPreview(r.value);
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
    if (this.mode === 'flow') this.flow.refresh();
  }

  private cursor(at: number): void {
    // The help follows the cursor while one writes; the tree's choice stays until then.
    if (document.activeElement !== this.editor.input) return;
    const help = exprHelpAt(this.editor.value, at, { fields: this.opts.fields, variables: this.opts.variables ?? [], world: this.opts.world === true });
    if (help) this.help.show(help);
  }

  private step(by: number): void {
    const n = this.opts.objects?.count ?? 0;
    if (!n) return;
    this.index = (this.index + by + n) % n;
    this.refreshPreview();
    if (this.mode === 'flow') this.flow.preview();
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
