import type { AppContext } from '../../app/context';
import { DisposableStore, listen } from '../../core/disposable';
import { fixed } from '../../core/displayNumber';
import { watchAll } from '../../core/signal';
import type { PaperColors } from '../../render/sheet/paperPainter';
import type { SheetView } from '../../product/sheet/view';
import { Panel } from '../dock/Panel';
import { h } from '../dom';
import { icon } from '../icons';
import { PopupMenu } from '../widgets/PopupMenu';
import { hideTooltip, tooltip } from '../widgets/tooltip';
import type { SheetHost } from './host';
import { newSheetItems, sheetItems } from './menus';
import { thumbCanvas } from './thumb';

/**
 * Paftalar (docs/sheet/design.md §11): the project's sheets as small papers,
 * the one in front marked. A click brings a sheet forward; ↑/↓ move through
 * the list and Enter opens; a right click gives the sheet's menu; “+” offers
 * a new sheet. Each picture is the sheet's own plan from the engine,
 * painted small (its maps' content too), drawn again when a picture arrives.
 */
export class SheetsPanel extends Panel {
  private readonly ctx: AppContext;
  private readonly host: SheetHost;
  private readonly colors: () => PaperColors;
  private readonly list: HTMLElement;
  private readonly rows = new DisposableStore();
  private repaints: (() => void)[] = [];

  constructor(ctx: AppContext, host: SheetHost, colors: () => PaperColors) {
    super({ title: 'Paftalar', className: 'sheet-pages' });
    this.ctx = ctx;
    this.host = host;
    this.colors = colors;
    const add = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Yeni pafta', 'aria-haspopup': 'menu' }, icon('plus', 16));
    add.addEventListener('click', () => PopupMenu.open(newSheetItems(ctx), add.getBoundingClientRect(), { minWidth: 260, owner: add }));
    this.d.add(tooltip(add, () => ({ title: 'Yeni pafta', description: 'Boş pafta, şablondan pafta ya da .kpafta dosyası.', note: host.state.whyNoEngine() ?? undefined })));
    this.el.querySelector('.panel__actions')!.append(add);
    this.list = h('div', { class: 'sheet-pages__list', role: 'listbox', 'aria-label': 'Paftalar' });
    this.body.append(this.list);
    this.d.add(listen<KeyboardEvent>(this.list, 'keydown', (e) => this.onKey(e)));
    this.d.add(watchAll([host.state.book, host.state.open], () => this.render()));
    this.d.add(host.painted.subscribe(() => this.repaints.forEach((p) => p())));
    this.d.add(() => this.rows.dispose());
    this.render();
  }

  /** Draws the list again (the theme changed the pictures' colours). */
  refresh(): void {
    this.render();
  }

  private render(): void {
    const { state } = this.host;
    const sheets = state.book.value.sheets;
    const focused = this.list.contains(document.activeElement);
    hideTooltip(this.list);
    this.rows.dispose();
    this.setMeta(`${sheets.length} pafta`);
    const colors = this.colors();
    this.repaints = [];
    this.list.replaceChildren(...sheets.map((s) => this.row(s, s.id === state.open.value, colors)));
    if (focused) this.list.querySelector<HTMLElement>('[aria-selected="true"]')?.focus();
  }

  private row(s: SheetView, selected: boolean, colors: PaperColors): HTMLElement {
    const p = s.paper;
    const paper = `${p.name} ${p.orientation === 'landscape' ? 'yatay' : 'dikey'} · ${fixed(p.widthMm, 0)} × ${fixed(p.heightMm, 0)} mm`;
    const book = this.host.book();
    const list = book ? this.host.plan(book, s.id) : null;
    const thumb = list ? thumbCanvas(list, 40, this.host.sources, colors, `${s.name}: ${paper}`) : null;
    if (thumb) this.repaints.push(thumb.paint);
    const pic = thumb?.canvas ?? null;
    const b = h(
      'button',
      {
        class: 'sheet-page',
        type: 'button',
        role: 'option',
        'aria-selected': String(selected),
        // Its name once: the sheet and its paper (the picture's own label would say them again).
        'aria-label': `${s.name}, ${paper}${s.template?.newer ? ', yeni sürüm var' : ''}`,
        tabindex: selected ? '0' : '-1',
        dataset: { sheet: s.id },
      },
      h('span', { class: 'sheet-page__thumb' }, pic),
      h('span', null, h('span', { class: 'sheet-page__name' }, s.name), h('span', { class: 'sheet-page__paper' }, paper), s.template?.newer ? h('span', { class: 'tbadge tbadge--newer sheet-page__newer' }, 'Yeni sürüm var') : null),
    );
    b.addEventListener('click', () => this.host.openSheet(s.id));
    b.addEventListener('contextmenu', (e) => {
      e.preventDefault();
      PopupMenu.open(sheetItems(this.ctx, this.host, s.id), { x: e.clientX, y: e.clientY }, { placement: 'point', minWidth: 280 });
    });
    this.rows.add(tooltip(b, () => ({ title: s.name, description: `${paper}${s.master ? ` · ana sayfa: ${s.master}` : ''}${s.template ? ` · şablon: ${s.template.name}${s.template.newer ? ' (yeni sürüm var)' : ''}` : ''}` }), 'right'));
    return b;
  }

  private onKey(e: KeyboardEvent): void {
    const rows = [...this.list.querySelectorAll<HTMLElement>('.sheet-page')];
    const at = rows.indexOf(document.activeElement as HTMLElement);
    if (at < 0) return;
    const step = e.key === 'ArrowDown' ? 1 : e.key === 'ArrowUp' ? -1 : 0;
    if (step) {
      e.preventDefault();
      e.stopPropagation();
      rows[Math.max(0, Math.min(rows.length - 1, at + step))].focus();
    } else if (e.key === 'F2') {
      e.preventDefault();
      e.stopPropagation();
      this.ctx.commands.execute('sheet.rename', { id: rows[at].dataset.sheet });
    }
  }
}
