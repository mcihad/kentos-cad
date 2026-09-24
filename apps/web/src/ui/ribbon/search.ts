import type { AppContext } from '../../app/context';
import type { Command } from '../../core/commands';
import { listen, type DisposableStore } from '../../core/disposable';
import { formatChord } from '../../core/keymap';
import { h, overlayRoot } from '../dom';
import { icon } from '../icons';
import { tooltip } from '../widgets/tooltip';

/** Where search results come from and what their actions do. */
export interface SearchHost {
  /** "Tab › Panel" of a command in the ribbon, if it has a place there. */
  where(id: string): string | undefined;
  /** Opens the command's tab and marks its button. */
  reveal(id: string): void;
  /** A result ran. */
  afterRun(): void;
}

const LIMIT = 9;

/**
 * Komut ara: finds commands by name or command-line alias (Turkish letters
 * folded, as on the command line) and runs them, or shows where they live
 * in the ribbon (Alt+Enter, or the row's pin button).
 */
export class RibbonSearch {
  readonly el: HTMLElement;
  private readonly input: HTMLInputElement;
  private readonly ctx: AppContext;
  private readonly host: SearchHost;
  private list: HTMLElement | null = null;
  private results: Command[] = [];
  private active = 0;

  constructor(ctx: AppContext, d: DisposableStore, host: SearchHost) {
    this.ctx = ctx;
    this.host = host;
    this.input = h('input', {
      class: 'rsearch__input',
      type: 'search',
      placeholder: 'Komut ara…',
      'aria-label': 'Komut ara',
      role: 'combobox',
      'aria-expanded': 'false',
      'aria-autocomplete': 'list',
      spellcheck: 'false',
      autocomplete: 'off',
    });
    const hint = h('kbd', { class: 'kbd rsearch__kbd' }, formatChord('Alt+Q'));
    const glyph = h('span', { class: 'rsearch__icon' }, icon('search', 14));
    this.el = h('div', { class: 'rsearch' }, glyph, this.input, hint);
    // Narrow windows show only the glass; pressing it opens the field.
    d.add(
      listen(glyph, 'pointerdown', (e) => {
        e.preventDefault();
        this.focus();
      }),
    );
    d.add(listen(this.input, 'input', () => this.update()));
    d.add(
      listen(this.input, 'focus', () => {
        this.el.dataset.focus = '';
        if (this.input.value) this.update();
      }),
    );
    d.add(
      listen(this.input, 'blur', () => {
        delete this.el.dataset.focus;
        this.close();
      }),
    );
    d.add(listen<KeyboardEvent>(this.input, 'keydown', (e) => this.onKey(e)));
    d.add(tooltip(this.el, () => (this.el.dataset.focus !== undefined ? null : { title: 'Komut ara', shortcut: 'Alt+Q', description: 'Bir komutu adıyla ya da komut satırı takma adıyla (ör. L, CIZGI) bulun; Enter çalıştırır, Alt+Enter şeritteki yerini gösterir.' })));
    d.add(() => this.close());
  }

  focus(): void {
    this.input.focus();
    this.input.select();
  }

  private update(): void {
    const q = this.input.value.trim();
    this.results = q ? this.ctx.commands.search(q, LIMIT) : [];
    this.active = 0;
    if (!q) return this.close();
    this.render(q);
  }

  private render(q: string): void {
    if (!this.list) {
      this.list = h('div', { class: 'rsearch__list', role: 'listbox', id: 'ribbon-search-list', 'aria-label': 'Komutlar' });
      // Pressing in the list must not blur the field before the click lands.
      this.list.addEventListener('pointerdown', (e) => e.preventDefault());
      overlayRoot().append(this.list);
      this.input.setAttribute('aria-expanded', 'true');
      this.input.setAttribute('aria-controls', 'ribbon-search-list');
    }
    const rows = this.results.map((cmd, i) => {
      const enabled = this.ctx.commands.isEnabled(cmd.id);
      const chord = this.ctx.keymap.chordFor(cmd.id);
      const where = this.host.where(cmd.id);
      const pin = where ? h('button', { class: 'rsearch__pin', type: 'button', 'aria-label': 'Şeritte göster', title: 'Şeritte göster (Alt+Enter)' }, icon('ribbon', 14)) : null;
      const row = h(
        'div',
        { class: 'rsearch__row', role: 'option', id: `ribbon-search-${i}`, 'aria-selected': String(i === this.active), 'aria-disabled': enabled ? null : 'true' },
        h('span', { class: 'rsearch__glyph' }, icon(cmd.icon ?? 'more', 16)),
        h(
          'span',
          { class: 'rsearch__text' },
          h('span', { class: 'rsearch__title' }, cmd.title),
          h('span', { class: 'rsearch__where' }, cmd.pending ? 'Geliştirme aşamasında' : (where ?? cmd.category ?? '')),
        ),
        chord ? h('span', { class: 'rsearch__chord' }, formatChord(chord)) : null,
        pin,
      );
      row.addEventListener('pointerenter', () => this.setActive(i));
      row.addEventListener('click', (e) => (pin && pin.contains(e.target as Node) ? this.reveal(i) : this.run(i)));
      return row;
    });
    this.list.replaceChildren(
      ...(rows.length ? rows : [h('div', { class: 'rsearch__empty' }, `“${q}” ile eşleşen komut yok. Komut satırındaki takma adlar da aranır (ör. L, CIZGI).`)]),
    );
    this.input.setAttribute('aria-activedescendant', rows.length ? `ribbon-search-${this.active}` : '');
    this.place();
  }

  private place(): void {
    if (!this.list) return;
    const r = this.el.getBoundingClientRect();
    const w = this.list.getBoundingClientRect().width;
    const x = Math.max(8, Math.min(r.right - w, innerWidth - w - 8));
    this.list.style.transform = `translate(${Math.round(x)}px, ${Math.round(r.bottom + 4)}px)`;
  }

  private setActive(i: number): void {
    if (!this.list || !this.results.length) return;
    this.active = (i + this.results.length) % this.results.length;
    [...this.list.children].forEach((row, k) => row.setAttribute('aria-selected', String(k === this.active)));
    this.input.setAttribute('aria-activedescendant', `ribbon-search-${this.active}`);
  }

  private run(i: number): void {
    const cmd = this.results[i];
    if (!cmd || !this.ctx.commands.isEnabled(cmd.id)) return;
    this.clear();
    this.ctx.commands.execute(cmd.id);
    // A tool takes the keyboard next (typed coordinates), as after a toolbox click.
    if (cmd.id.startsWith('tool.')) this.ctx.view.focus();
    else this.input.blur();
    this.host.afterRun();
  }

  private reveal(i: number): void {
    const cmd = this.results[i];
    if (!cmd) return;
    this.clear();
    this.input.blur();
    this.host.reveal(cmd.id);
  }

  private clear(): void {
    this.input.value = '';
    this.results = [];
    this.close();
  }

  private close(): void {
    this.list?.remove();
    this.list = null;
    this.input.setAttribute('aria-expanded', 'false');
    this.input.removeAttribute('aria-activedescendant');
  }

  private onKey(e: KeyboardEvent): void {
    const stop = () => {
      e.preventDefault();
      e.stopPropagation();
    };
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      stop();
      return this.setActive(this.active + (e.key === 'ArrowDown' ? 1 : -1));
    }
    if (e.key === 'Enter') {
      stop();
      return e.altKey ? this.reveal(this.active) : this.run(this.active);
    }
    if (e.key === 'Escape') {
      stop();
      if (this.input.value) return this.clear();
      this.input.blur();
      this.ctx.view.focus();
    }
  }
}
