import { DisposableStore, listen } from '../../core/disposable';
import {
  exprBracket,
  exprCheck,
  exprComplete,
  exprSignature,
  exprTokens,
  type ExprCheck,
  type ExprCompletion,
  type ExprField,
  type ExprItem,
} from '../../model/expression/builder';
import { h, overlayRoot } from '../dom';
import { KIND_STYLE, TOKEN_STYLE } from './highlight';

/**
 * The expression builder's editor (DESIGN.md §7.16): a text area over a
 * painted copy of its text (the tokens' colours, the error's and the
 * warnings' wavy lines, the parentheses beside the cursor and their pair),
 * the completion list at the cursor (it opens as a name is typed, and on
 * Ctrl+Space) and the call's signature under it. What each piece means is
 * the core's (model/expression/builder.ts); this only draws it and edits
 * the text through the browser, so Ctrl+Z undoes as in any field.
 */
export interface CodeEditorOptions {
  readonly value: string;
  readonly label: string;
  readonly fields: () => readonly ExprField[];
  /** The text changed (typed, completed or inserted). */
  readonly onChange: (source: string, check: ExprCheck) => void;
  /** The cursor moved (or the text changed under it). */
  readonly onCursor: (cursor: number) => void;
  /** Ctrl+Enter. */
  readonly onSubmit: () => void;
  /** The completion entry highlighted (the help shows it); null when the list closes. */
  readonly onActive: (item: ExprItem | null) => void;
}

/** A character that goes on writing a name: the list opens (or stays) as it is typed. */
const NAME_CHAR = /[\p{L}\p{N}_$[]/u;

export class CodeEditor {
  readonly el: HTMLElement;
  readonly input: HTMLTextAreaElement;
  private readonly paint: HTMLElement;
  private readonly sig: HTMLElement;
  private readonly list: HTMLElement;
  private readonly opts: CodeEditorOptions;
  private readonly d = new DisposableStore();
  private check: ExprCheck = { warnings: [] };
  private completion: ExprCompletion | null = null;
  private active = 0;
  private bracket: { at: number; partner?: number } | null = null;
  /** An insertion of ours is under way: it is not typing, so no list opens. */
  private inserting = false;

  constructor(opts: CodeEditorOptions) {
    this.opts = opts;
    this.input = h('textarea', { class: 'xed__input', spellcheck: 'false', autocomplete: 'off', autocapitalize: 'off', 'aria-label': opts.label, 'aria-autocomplete': 'list' });
    this.input.value = opts.value;
    this.paint = h('pre', { class: 'xed__paint', 'aria-hidden': 'true' });
    this.sig = h('div', { class: 'xed__sig', 'aria-live': 'polite' });
    this.list = h('div', { class: 'xed__list', role: 'listbox', 'aria-label': 'Öneriler', hidden: true });
    overlayRoot().append(this.list);
    this.el = h('div', { class: 'xed' }, h('div', { class: 'xed__box' }, this.paint, this.input), this.sig);
    const d = this.d;
    d.add(listen(this.input, 'input', (e) => this.typed(e as InputEvent)));
    d.add(listen<KeyboardEvent>(this.input, 'keydown', (e) => this.key(e)));
    d.add(listen(this.input, 'scroll', () => this.syncScroll()));
    d.add(listen(this.input, 'blur', () => this.close()));
    for (const type of ['keyup', 'click', 'select', 'focus'] as const) d.add(listen(this.input, type, () => this.moved()));
    d.add(listen(document, 'selectionchange', () => document.activeElement === this.input && this.moved()));
    // A click in the list keeps the focus (and the caret) in the text.
    d.add(listen(this.list, 'mousedown', (e) => e.preventDefault()));
    d.add(() => this.list.remove());
    // Painted at once; the owner asks for the first check (`checked`) when it is built.
    this.check = exprCheck(this.input.value, opts.fields());
    this.repaint();
  }

  get value(): string {
    return this.input.value;
  }

  /** The text's error and warnings as last checked. */
  get checked(): ExprCheck {
    return this.check;
  }

  get cursor(): number {
    return this.input.selectionStart;
  }

  focus(): void {
    this.input.focus();
  }

  /** Replaces the whole text (the flow wrote it): checked and painted, the cursor at its end. */
  setValue(text: string): void {
    if (text === this.input.value) return;
    this.input.value = text;
    this.input.setSelectionRange(text.length, text.length);
    this.changed();
  }

  /**
   * Replaces `start..end` with `text` as typing would (the browser's undo
   * keeps it) and puts the cursor at `caret` in it.
   */
  replace(start: number, end: number, text: string, caret: number): void {
    const input = this.input;
    input.focus();
    input.setSelectionRange(start, end);
    this.inserting = true;
    try {
      // execCommand keeps the edit in the field's own undo history; setRangeText is the fallback.
      if (!document.execCommand('insertText', false, text)) {
        input.setRangeText(text, start, end, 'end');
        input.dispatchEvent(new Event('input'));
      }
    } finally {
      this.inserting = false;
    }
    input.setSelectionRange(start + caret, start + caret);
    this.moved();
  }

  dispose(): void {
    this.d.dispose();
  }

  private typed(e: InputEvent): void {
    this.changed();
    if (this.inserting) return;
    const ch = e.data?.slice(-1) ?? '';
    const naming = e.inputType === 'insertText' && NAME_CHAR.test(ch);
    const deleting = (e.inputType ?? '').startsWith('delete');
    if (naming || (deleting && this.completion)) this.complete(false);
    else this.close();
  }

  private changed(): void {
    this.check = exprCheck(this.input.value, this.opts.fields());
    this.bracket = null;
    this.repaint();
    this.opts.onChange(this.input.value, this.check);
    this.moved();
  }

  /** The cursor moved: the signature, the parentheses, the owner's help. */
  private moved(): void {
    const src = this.input.value;
    const at = this.input.selectionStart;
    const b = this.input.selectionStart === this.input.selectionEnd ? exprBracket(src, at) : null;
    if (JSON.stringify(b) !== JSON.stringify(this.bracket)) {
      this.bracket = b;
      this.repaint();
    }
    this.signature(src, at);
    // While the list is open the help follows its highlighted entry instead.
    if (!this.completion) this.opts.onCursor(at);
  }

  private signature(src: string, at: number): void {
    const s = exprSignature(src, at);
    if (!s) {
      this.sig.replaceChildren();
      return;
    }
    const arg = s.active !== undefined ? s.args[s.active] : undefined;
    const u = s.signature;
    const parts = arg ? [u.slice(0, arg.start), h('b', null, u.slice(arg.start, arg.end)), u.slice(arg.end)] : [u];
    const note = arg ? `${arg.name}${arg.optional ? ' (isteğe bağlı)' : ''}: ${arg.description}` : s.description;
    this.sig.replaceChildren(h('code', { class: 'xed__sigcode' }, ...parts), h('span', { class: 'xed__sigdesc', title: note }, note));
  }

  /** Paints the text: a run of characters per class and mark. */
  private repaint(): void {
    const src = this.input.value;
    const n = src.length;
    const cls: string[] = new Array(n).fill('');
    for (const t of exprTokens(src)) for (let i = t.start; i < t.end; i++) cls[i] = TOKEN_STYLE[t.class];
    const mark: string[] = new Array(n).fill('');
    const add = (start: number, end: number, name: string) => {
      for (let i = start; i < Math.min(end, n); i++) mark[i] = mark[i] ? `${mark[i]} ${name}` : name;
    };
    for (const w of this.check.warnings) add(w.start, w.end, 'x-warn');
    const err = this.check.error;
    if (err) add(err.start, err.end, 'x-err');
    const b = this.bracket;
    if (b) {
      const kind = b.partner === undefined ? 'x-unmatched' : 'x-match';
      add(b.at, b.at + 1, kind);
      if (b.partner !== undefined) add(b.partner, b.partner + 1, kind);
    }
    const out: (HTMLElement | string)[] = [];
    let from = 0;
    for (let i = 1; i <= n; i++) {
      if (i < n && cls[i] === cls[from] && mark[i] === mark[from]) continue;
      const text = src.slice(from, i);
      const c = `${cls[from]} ${mark[from]}`.trim();
      out.push(c ? h('span', { class: c }, text) : text);
      from = i;
    }
    // The end of the text: where an error at the end is marked, and a line
    // after a last newline keeps its height, as the text area shows it.
    const atEnd = err && err.start === n;
    out.push(h('span', { class: atEnd ? 'xed__tail x-err' : 'xed__tail' }, ' '));
    this.paint.replaceChildren(...out);
    this.syncScroll();
  }

  private syncScroll(): void {
    this.paint.scrollTop = this.input.scrollTop;
    this.paint.scrollLeft = this.input.scrollLeft;
    if (this.completion) this.place();
  }

  // ── Completion ───────────────────────────────────────────────────

  private complete(explicit: boolean): void {
    const c = exprComplete(this.input.value, this.input.selectionStart, this.opts.fields(), explicit);
    if (!c || !c.items.length) {
      this.close();
      return;
    }
    this.completion = c;
    this.active = 0;
    this.input.dataset.escape = 'local';
    this.input.setAttribute('aria-expanded', 'true');
    this.renderList();
    // Last in the overlay: above the dialog it belongs to.
    overlayRoot().append(this.list);
    this.list.hidden = false;
    this.place();
    this.opts.onActive(c.items[0]);
  }

  private close(): void {
    if (!this.completion) return;
    this.completion = null;
    this.list.hidden = true;
    delete this.input.dataset.escape;
    this.input.setAttribute('aria-expanded', 'false');
    this.opts.onActive(null);
  }

  private renderList(): void {
    const c = this.completion;
    if (!c) return;
    this.list.replaceChildren(
      ...c.items.map((item, i) => {
        const row = h(
          'div',
          { class: 'xed__item', role: 'option', 'aria-selected': String(i === this.active), id: `xed-opt-${i}` },
          h('span', { class: `xed__label ${KIND_STYLE[item.kind]}` }, item.label),
          item.alias ? h('span', { class: 'xed__alias' }, item.alias) : null,
          h('span', { class: 'xed__detail' }, item.detail),
        );
        row.addEventListener('click', () => this.accept(item));
        row.addEventListener('mousemove', () => this.highlight(i));
        return row;
      }),
    );
    this.input.setAttribute('aria-activedescendant', `xed-opt-${this.active}`);
  }

  private highlight(i: number): void {
    const c = this.completion;
    if (!c || i === this.active) return;
    this.active = Math.max(0, Math.min(c.items.length - 1, i));
    this.list.querySelectorAll('.xed__item').forEach((el, k) => el.setAttribute('aria-selected', String(k === this.active)));
    this.list.children[this.active]?.scrollIntoView({ block: 'nearest' });
    this.input.setAttribute('aria-activedescendant', `xed-opt-${this.active}`);
    this.opts.onActive(c.items[this.active]);
  }

  private accept(item: ExprItem): void {
    const c = this.completion;
    if (!c) return;
    this.close();
    this.replace(c.start, c.end, item.insert, item.caret);
    // A function's parentheses are open: show its signature at once.
    this.moved();
  }

  /** Puts the list under the line of the cursor, inside the window. */
  private place(): void {
    const c = this.completion;
    if (!c) return;
    const probe = document.createRange();
    const node = this.textAt(c.start);
    if (!node) return;
    probe.setStart(node.node, node.offset);
    probe.collapse(true);
    const r = probe.getClientRects()[0] ?? probe.getBoundingClientRect();
    const box = this.input.getBoundingClientRect();
    const line = parseFloat(getComputedStyle(this.input).lineHeight) || 20;
    const left = Math.min(Math.max(r.left, box.left), window.innerWidth - this.list.offsetWidth - 8);
    let top = r.top + line;
    if (top + this.list.offsetHeight > window.innerHeight - 8) top = r.top - this.list.offsetHeight - 2;
    this.list.style.left = `${Math.max(8, left)}px`;
    this.list.style.top = `${Math.max(8, top)}px`;
  }

  /** The painted text node and offset at a UTF-16 position. */
  private textAt(pos: number): { node: Node; offset: number } | null {
    const walker = document.createTreeWalker(this.paint, NodeFilter.SHOW_TEXT);
    let seen = 0;
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      const len = node.textContent?.length ?? 0;
      if (pos <= seen + len) return { node, offset: pos - seen };
      seen += len;
    }
    return null;
  }

  private key(e: KeyboardEvent): void {
    const c = this.completion;
    if (e.key === ' ' && e.ctrlKey) {
      e.preventDefault();
      this.complete(true);
      return;
    }
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      this.close();
      this.opts.onSubmit();
      return;
    }
    if (!c) return;
    const page = 7;
    switch (e.key) {
      case 'ArrowDown':
        this.highlight((this.active + 1) % c.items.length);
        break;
      case 'ArrowUp':
        this.highlight((this.active - 1 + c.items.length) % c.items.length);
        break;
      case 'PageDown':
        this.highlight(Math.min(c.items.length - 1, this.active + page));
        break;
      case 'PageUp':
        this.highlight(Math.max(0, this.active - page));
        break;
      case 'Enter':
      case 'Tab':
        this.accept(c.items[this.active]);
        break;
      case 'Escape':
        this.close();
        break;
      default:
        return;
    }
    e.preventDefault();
    e.stopPropagation();
  }
}
