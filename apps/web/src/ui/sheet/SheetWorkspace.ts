import type { AppContext } from '../../app/context';
import { readPaperColors, type PaperColors } from '../../render/sheet/paperPainter';
import { Component } from '../Component';
import { h } from '../dom';
import { splitter } from '../widgets/Splitter';
import type { SheetHost } from './host';
import { ItemTree } from './ItemTree';
import { SheetInspector } from './SheetInspector';
import { SheetsPanel } from './SheetsPanel';
import { SheetStage } from './SheetStage';

/**
 * The sheet workspace (docs/sheet/design.md §11): while a sheet is in front
 * it lies over the drawing area: Paftalar and Öğeler on the left, the desk
 * with its rulers in the middle, Denetçi on the right (the shell's right
 * dock gives way to it). The side columns are dragged wider or narrower and
 * a double click on their edge puts them back (narrower in a narrow window);
 * their widths last while the window is open.
 */

const LEFT = { min: 184, max: 420 };
const RIGHT = { min: 264, max: 480 };

export class SheetWorkspace extends Component {
  readonly el: HTMLElement;
  readonly stage: SheetStage;
  readonly inspector: SheetInspector;
  private readonly parts: Component[] = [];
  private readonly pages: SheetsPanel;

  constructor(ctx: AppContext, host: SheetHost) {
    super();
    // Colours for the small papers, read from the workspace (it carries the sheet tokens) once it is on the page.
    let colors: PaperColors | null = null;
    const paperColors = (): PaperColors => {
      if (colors) return colors;
      // Not on the page yet (the panels draw once while they are built): the fallbacks, not kept.
      const el = this.el as HTMLElement | undefined;
      if (!el?.isConnected) return readPaperColors(document.documentElement);
      return (colors = readPaperColors(el));
    };
    this.pages = this.own(new SheetsPanel(ctx, host, paperColors));
    const items = this.own(new ItemTree(ctx, host));
    this.stage = this.own(new SheetStage(ctx, host));
    const inspector = (this.inspector = this.own(new SheetInspector(ctx, host)));

    // A drag starts from the width shown (the stylesheet narrows the columns in a narrow window until one is dragged).
    let start = 0;
    const width = (side: 'left' | 'right') => this.el.querySelector<HTMLElement>(`.sheet-ws__${side}`)?.getBoundingClientRect().width ?? 0;
    const set = (prop: string, w: number, r: { min: number; max: number }) => this.el.style.setProperty(prop, `${Math.round(Math.min(r.max, Math.max(r.min, w)))}px`);
    const leftEdge = splitter({
      orientation: 'vertical',
      label: 'Paftalar ve öğeler genişliği',
      onStart: () => (start = width('left')),
      onDrag: (dx) => set('--sheet-left-w', start + dx, LEFT),
      onReset: () => this.el.style.removeProperty('--sheet-left-w'),
    });
    const rightEdge = splitter({
      orientation: 'vertical',
      label: 'Denetçi genişliği',
      onStart: () => (start = width('right')),
      onDrag: (dx) => set('--sheet-right-w', start - dx, RIGHT),
      onReset: () => this.el.style.removeProperty('--sheet-right-w'),
    });
    this.d.add(leftEdge.dispose);
    this.d.add(rightEdge.dispose);

    this.el = h(
      'div',
      { class: 'sheet-ws', hidden: true, role: 'region', 'aria-label': 'Pafta düzeni' },
      h('div', { class: 'sheet-ws__left' }, this.pages.el, items.el, leftEdge.el),
      this.stage.el,
      h('div', { class: 'sheet-ws__right' }, rightEdge.el, inspector.el),
    );
    // The theme changes the small papers' colours.
    this.d.add(
      ctx.prefs.theme.subscribe(() => {
        colors = null;
        queueMicrotask(() => this.pages.refresh());
      }),
    );
  }

  /** The inspector's Ön denetim tab in front (the status bar's badge, the command). */
  showPreflight(): void {
    this.inspector.tab.set('preflight');
  }

  /** Shown while a sheet is in front. */
  show(on: boolean): void {
    this.el.hidden = !on;
    if (on) this.pages.refresh();
  }

  private own<T extends Component>(c: T): T {
    this.parts.push(c);
    return c;
  }

  override dispose(): void {
    this.parts.forEach((p) => p.dispose());
    super.dispose();
  }
}
