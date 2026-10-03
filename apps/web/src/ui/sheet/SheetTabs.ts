import type { AppContext } from '../../app/context';
import { DisposableStore, listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { PopupMenu } from '../widgets/PopupMenu';
import { hideTooltip, tooltip } from '../widgets/tooltip';
import type { SheetHost } from './host';
import { newSheetItems, sheetItems } from './menus';
import { tabsNote } from './tabsPlan';

/**
 * Model | Pafta 1 | … | + (docs/sheet/design.md §11): the tabs under the
 * drawing area. Model is always the first tab and never closes; a sheet's
 * tab brings its sheet forward (the workspace takes the drawing area's
 * place). A right click, the menu key or Shift+F10 opens a tab's menu (Ad
 * ver, Çoğalt, Sola taşı, Sağa taşı, Sil); a double click or F2 names it;
 * ←/→, Home and End move between tabs. “+” offers a new sheet, one from a
 * template, or a `.kpafta` file. A tab whose template has a newer revision
 * carries a dot and says so.
 */
export class SheetTabs extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly host: SheetHost;
  private readonly list: HTMLElement;
  private readonly note: HTMLElement;
  /** The tabs' own listeners and tooltips, released when they are drawn again. */
  private readonly rows = new DisposableStore();

  constructor(ctx: AppContext, host: SheetHost) {
    super();
    this.ctx = ctx;
    this.host = host;
    this.list = h('div', { class: 'sheet-tabs__list', role: 'tablist', 'aria-label': 'Model ve paftalar' });
    const add = h('button', { class: 'ibtn sheet-tabs__add', type: 'button', 'aria-label': 'Yeni pafta', 'aria-haspopup': 'menu' }, icon('plus', 16));
    this.d.add(
      listen<PointerEvent>(add, 'pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        this.openAddMenu(add);
      }),
    );
    this.d.add(
      listen<KeyboardEvent>(add, 'keydown', (e) => {
        if (e.key !== 'Enter' && e.key !== ' ' && e.key !== 'ArrowDown') return;
        e.preventDefault();
        this.openAddMenu(add).focusFirst();
      }),
    );
    this.d.add(tooltip(add, () => ({ title: 'Yeni pafta', description: 'Proje türünün varsayılan şablonundan, şablondan ya da .kpafta dosyasından.', note: host.state.whyNoEngine() ?? undefined }), 'top'));
    this.note = h('span', { class: 'sheet-tabs__note' });
    this.el = h('div', { class: 'sheet-tabs', role: 'navigation', 'aria-label': 'Paftalar' }, this.list, h('span', { class: 'sheet-tabs__sep', 'aria-hidden': 'true' }), add, h('span', { class: 'sheet-tabs__spacer' }), this.note);
    this.d.add(listen<KeyboardEvent>(this.list, 'keydown', (e) => this.onKey(e)));
    this.d.add(watchAll([host.state.book, host.state.open, host.state.engine, host.state.waiting], () => this.render()));
    this.d.add(() => this.rows.dispose());
    this.render();
  }

  private openAddMenu(anchor: HTMLElement): PopupMenu {
    anchor.setAttribute('aria-expanded', 'true');
    return PopupMenu.open(newSheetItems(this.ctx), anchor.getBoundingClientRect(), { minWidth: 260, owner: anchor, onClose: () => anchor.setAttribute('aria-expanded', 'false') });
  }

  private render(): void {
    const { state } = this.host;
    const focused = this.list.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.sheet : undefined;
    hideTooltip(this.list);
    this.rows.dispose();
    const open = state.open.value;
    // Before the engine has read the book, the sheets kept on this device show by name (a click reads them).
    const sheets = state.book.value.source ? state.book.value.sheets.map((s) => ({ id: s.id, name: s.name, newer: !!s.template?.newer })) : state.waiting.value.map((s) => ({ ...s, newer: false }));
    const tabs = [{ id: '', name: 'Model', newer: false }, ...sheets];
    this.list.replaceChildren(
      ...tabs.map((t) => {
        const selected = t.id === '' ? open === null : open === t.id;
        const b = h(
          'button',
          { class: 'sheet-tab', type: 'button', role: 'tab', 'aria-selected': String(selected), tabindex: selected ? '0' : '-1', dataset: { sheet: t.id } },
          h('span', { class: 'sheet-tab__name' }, t.name),
          t.newer ? h('span', { class: 'sheet-tab__newer', 'aria-label': 'Yeni sürüm var' }) : null,
        );
        b.addEventListener('pointerdown', (e) => {
          if (e.button !== 0) return;
          // A click leaves the keyboard where it was (on the paper or the drawing).
          e.preventDefault();
          this.host.openSheet(t.id || null);
        });
        b.addEventListener('click', (e) => {
          // A click that came from the keyboard (Enter, Space) opens too.
          if (e.detail === 0) this.host.openSheet(t.id || null);
        });
        b.addEventListener('dblclick', () => t.id && this.ctx.commands.execute('sheet.rename', { id: t.id }));
        b.addEventListener('contextmenu', (e) => {
          e.preventDefault();
          this.menu(t.id, { x: e.clientX, y: e.clientY });
        });
        this.rows.add(
          tooltip(
            b,
            () =>
              t.id === ''
                ? { title: 'Model', description: 'Çizim alanı: projenin kendisi. Her zaman ilk sekmedir, kapanmaz.' }
                : {
                    title: t.name,
                    description: this.paperLine(t.id),
                    note: t.newer ? 'Yeni sürüm var: şablonunun daha yeni bir sürümü kaydedildi. Pafta kendiliğinden değişmez; güncellemeyi uygulamak sonraki aşamada gelecek.' : undefined,
                  },
            'top',
          ),
        );
        return b;
      }),
    );
    if (focused !== undefined) this.list.querySelector<HTMLElement>(`[data-sheet="${CSS.escape(focused)}"]`)?.focus();
    this.note.textContent = tabsNote(sheets.length, state.engine.value);
  }

  private paperLine(id: string): string {
    const s = this.host.state.book.value.sheets.find((x) => x.id === id);
    if (!s) return '';
    const n = s.items.filter((i) => !i.master && i.kind !== 'group').length;
    return `${s.paper.name} ${s.paper.orientation === 'landscape' ? 'yatay' : 'dikey'} · ${n} öğe${s.layout ? ` · düzen: ${s.layout}` : ''}${s.master ? ` · ana sayfa: ${s.master}` : ''}. Sağ tıklayınca ad ver, çoğalt, taşı, sil.`;
  }

  private menu(id: string, at: { x: number; y: number }): void {
    PopupMenu.open(id ? sheetItems(this.ctx, this.host, id) : newSheetItems(this.ctx), at, { placement: 'point', minWidth: 280 });
  }

  private onKey(e: KeyboardEvent): void {
    const tabs = [...this.list.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
    const at = tabs.indexOf(document.activeElement as HTMLButtonElement);
    if (at < 0) return;
    const id = tabs[at].dataset.sheet ?? '';
    let to = -1;
    if (e.key === 'ArrowRight') to = Math.min(tabs.length - 1, at + 1);
    else if (e.key === 'ArrowLeft') to = Math.max(0, at - 1);
    else if (e.key === 'Home') to = 0;
    else if (e.key === 'End') to = tabs.length - 1;
    if (to >= 0) {
      e.preventDefault();
      e.stopPropagation();
      const next = tabs[to].dataset.sheet ?? '';
      this.host.openSheet(next || null);
      queueMicrotask(() => this.list.querySelector<HTMLElement>(`[data-sheet="${CSS.escape(next)}"]`)?.focus());
      return;
    }
    if (e.key === 'F2' && id) {
      e.preventDefault();
      e.stopPropagation();
      this.ctx.commands.execute('sheet.rename', { id });
    } else if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      e.stopPropagation();
      const r = tabs[at].getBoundingClientRect();
      this.menu(id, { x: r.left, y: r.top });
    }
  }
}
