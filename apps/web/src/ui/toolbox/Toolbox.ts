import type { AppContext } from '../../app/context';
import { listen } from '../../core/disposable';
import { formatChordCompact } from '../../core/keymap';
import { watchAll } from '../../core/signal';
import { TOOL_GROUP_LABEL, type ToolDescriptor, type ToolGroup } from '../../tools/Tool';
import { Component } from '../Component';
import { h } from '../dom';
import { icon } from '../icons';
import { tooltip } from '../widgets/tooltip';

const GROUP_ORDER: ToolGroup[] = ['select', 'draw', 'annotate', 'transform', 'modify', 'area', 'map'];
/** Widest the toolbox grows before it would rather scroll. */
const MAX_COLUMNS = 6;
const EDGE_SNAP = 14;
const MARGIN = 8;

/**
 * Floating drawing toolbox. Every tool is visible, in short titled groups
 * that fold away with a click on their title. Drag by the grip, it sticks
 * to viewport edges; it can also be docked into the left column. Every
 * button shows its shortcut on a key cap, and its tooltip explains the
 * mouse steps. When the chosen column count does not fit the height, the
 * toolbox widens instead of scrolling, so no tool is ever out of sight.
 */
export class Toolbox extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly floatHost: HTMLElement;
  private readonly dockHost: HTMLElement;
  private readonly buttons = new Map<string, HTMLButtonElement>();
  private readonly body: HTMLElement;

  constructor(ctx: AppContext, hosts: { float: HTMLElement; dock: HTMLElement }) {
    super();
    this.ctx = ctx;
    this.floatHost = hosts.float;
    this.dockHost = hosts.dock;
    const { ui } = ctx;

    const grip = h('div', { class: 'toolbox__grip', title: 'Taşımak için sürükleyin' }, icon('grip', 14));
    const colsBtn = h('button', { class: 'toolbox__hbtn', type: 'button', 'aria-label': 'Sütun sayısını değiştir' }, icon('columns', 14));
    const dockBtn = h('button', { class: 'toolbox__hbtn', type: 'button', 'aria-label': 'Kenara sabitle' }, icon('dock', 14));
    const body = (this.body = h('div', { class: 'toolbox__body' }));

    const groups = ctx.tools.byGroup();
    for (const g of GROUP_ORDER) {
      const list = groups.get(g);
      if (list) body.append(this.section(g, list));
    }

    this.el = h(
      'aside',
      { class: 'toolbox', 'aria-label': 'Çizim araçları' },
      h('div', { class: 'toolbox__head' }, grip, h('div', { class: 'toolbox__hactions' }, colsBtn, dockBtn)),
      body,
    );

    this.d.add(
      ctx.tools.activeId.subscribe((id, prev) => {
        this.buttons.get(prev)?.setAttribute('aria-pressed', 'false');
        this.buttons.get(id)?.setAttribute('aria-pressed', 'true');
      }),
    );
    this.buttons.get(ctx.tools.activeId.value)?.setAttribute('aria-pressed', 'true');

    this.d.add(listen(colsBtn, 'click', () => ui.toolboxColumns.set(ui.toolboxColumns.value === 3 ? 2 : 3)));
    this.d.add(listen(dockBtn, 'click', () => ui.toolboxDocked.set(!ui.toolboxDocked.value)));
    this.d.add(tooltip(colsBtn, () => ({ title: ui.toolboxColumns.value === 3 ? 'İki sütun' : 'Üç sütun' }), 'right'));
    this.d.add(tooltip(dockBtn, () => ({ title: ui.toolboxDocked.value ? 'Serbest bırak' : 'Kenara sabitle', shortcut: undefined }), 'right'));
    // Older layouts stored 1 or 2 columns; everything below three is two.
    this.d.add(
      ui.toolboxColumns.subscribe((c) => {
        this.el.dataset.columns = c === 3 ? '3' : '2';
        this.fit();
      }, true),
    );
    this.d.add(ui.toolboxFolded.subscribe(() => this.fit()));
    this.d.add(watchAll([ui.toolboxDocked, ui.toolboxVisible, ui.ribbonToolbox, ctx.prefs.shell], () => this.place()));
    this.place();
    this.bindDrag(grip);
    const ro = new ResizeObserver(() => {
      this.fit();
      this.clamp();
    });
    ro.observe(this.floatHost);
    ro.observe(this.dockHost);
    this.d.add(() => ro.disconnect());
  }

  /** A titled group; clicking the title folds it (remembered with the layout). */
  private section(g: ToolGroup, list: readonly ToolDescriptor[]): HTMLElement {
    const { ui } = this.ctx;
    const grid = h('div', { class: 'toolbox__grid', id: `toolbox-${g}` });
    for (const d of list) grid.append(this.button(d));
    const title = h(
      'button',
      { class: 'toolbox__title', type: 'button', 'aria-controls': `toolbox-${g}` },
      icon('chevronDown', 12),
      h('span', null, TOOL_GROUP_LABEL[g]),
    );
    const section = h('div', { class: 'toolbox__section', role: 'group', 'aria-label': TOOL_GROUP_LABEL[g] }, title, grid);
    const sync = () => {
      const folded = ui.toolboxFolded.value.includes(g);
      section.toggleAttribute('data-folded', folded);
      title.setAttribute('aria-expanded', String(!folded));
      grid.hidden = folded;
    };
    this.d.add(ui.toolboxFolded.subscribe(sync, true));
    this.d.add(
      listen(title, 'click', () => {
        const folded = ui.toolboxFolded.value;
        ui.toolboxFolded.set(folded.includes(g) ? folded.filter((x) => x !== g) : [...folded, g]);
      }),
    );
    this.d.add(tooltip(title, () => ({ title: ui.toolboxFolded.value.includes(g) ? `${TOOL_GROUP_LABEL[g]} grubunu aç` : `${TOOL_GROUP_LABEL[g]} grubunu katla` }), 'right'));
    return section;
  }

  private button(d: ToolDescriptor): HTMLButtonElement {
    const { ctx } = this;
    const chord = d.id === 'select' ? 'Esc' : ctx.keymap.chordFor(`tool.${d.id}`);
    const b = h(
      'button',
      { class: 'toolbox__tool', type: 'button', 'aria-label': d.label, 'aria-pressed': 'false', dataset: { tool: d.id, ready: String(d.ready) } },
      icon(d.icon, 20),
      chord ? h('span', { class: 'toolbox__key', 'aria-hidden': 'true' }, formatChordCompact(chord)) : null,
    );
    this.d.add(
      listen(b, 'click', () => {
        ctx.commands.execute(`tool.${d.id}`);
        ctx.view.focus();
      }),
    );
    this.d.add(
      tooltip(
        b,
        () => ({
          title: d.label,
          shortcut: chord,
          description: d.description,
          steps: d.ready ? d.steps : undefined,
          note: d.ready ? undefined : 'Geliştirme aşamasında',
        }),
        'right',
      ),
    );
    this.buttons.set(d.id, b);
    return b;
  }

  /** Chosen column count, widened one column at a time until every tool fits the height. */
  private fit(): void {
    if (!this.el.isConnected || this.el.hidden) return;
    const base = this.ctx.ui.toolboxColumns.value === 3 ? 3 : 2;
    let cols = base;
    this.el.style.setProperty('--cols', String(cols));
    while (cols < MAX_COLUMNS && this.body.scrollHeight > this.body.clientHeight + 1) {
      cols++;
      this.el.style.setProperty('--cols', String(cols));
    }
  }

  /** Shown with the classic shell unless hidden; next to the ribbon (which holds every tool) only when asked for. */
  private get shown(): boolean {
    const { ui, prefs } = this.ctx;
    return prefs.shell.value === 'ribbon' ? ui.ribbonToolbox.value : ui.toolboxVisible.value;
  }

  private place(): void {
    const { ui } = this.ctx;
    const shown = this.shown;
    this.el.hidden = !shown;
    const docked = ui.toolboxDocked.value;
    this.el.dataset.docked = String(docked);
    this.dockHost.toggleAttribute('data-empty', !docked || !shown);
    if (docked) {
      this.dockHost.append(this.el);
      this.el.style.transform = '';
    } else {
      this.floatHost.append(this.el);
    }
    this.fit();
    if (!docked) this.moveTo(ui.toolboxX.value, ui.toolboxY.value);
  }

  private moveTo(x: number, y: number): void {
    const host = this.floatHost.getBoundingClientRect();
    const w = this.el.offsetWidth || 80;
    const hgt = this.el.offsetHeight || 400;
    const maxX = Math.max(MARGIN, host.width - w - MARGIN);
    const maxY = Math.max(MARGIN, host.height - hgt - MARGIN);
    let nx = Math.min(Math.max(x, MARGIN), maxX);
    let ny = Math.min(Math.max(y, MARGIN), maxY);
    // Magnetic edges.
    if (nx - MARGIN < EDGE_SNAP) nx = MARGIN;
    if (maxX - nx < EDGE_SNAP) nx = maxX;
    if (ny - MARGIN < EDGE_SNAP) ny = MARGIN;
    if (maxY - ny < EDGE_SNAP) ny = maxY;
    this.el.style.transform = `translate(${Math.round(nx)}px, ${Math.round(ny)}px)`;
    this.pos = { x: nx, y: ny };
  }

  private pos = { x: MARGIN, y: MARGIN };

  private clamp(): void {
    const { ui } = this.ctx;
    if (!ui.toolboxDocked.value && this.shown) this.moveTo(ui.toolboxX.value, ui.toolboxY.value);
  }

  private bindDrag(grip: HTMLElement): void {
    let start: { x: number; y: number; px: number; py: number } | null = null;
    this.d.add(
      listen<PointerEvent>(grip, 'pointerdown', (e) => {
        if (e.button !== 0) return;
        e.preventDefault();
        if (this.ctx.ui.toolboxDocked.value) {
          // Dragging a docked toolbox undocks it under the cursor.
          const host = this.floatHost.getBoundingClientRect();
          this.ctx.ui.toolboxX.set(e.clientX - host.left - 20);
          this.ctx.ui.toolboxY.set(e.clientY - host.top - 10);
          this.ctx.ui.toolboxDocked.set(false);
        }
        grip.setPointerCapture(e.pointerId);
        start = { x: e.clientX, y: e.clientY, px: this.pos.x, py: this.pos.y };
        this.el.dataset.dragging = '';
      }),
    );
    this.d.add(
      listen<PointerEvent>(grip, 'pointermove', (e) => {
        if (!start) return;
        this.moveTo(start.px + e.clientX - start.x, start.py + e.clientY - start.y);
      }),
    );
    const end = () => {
      if (!start) return;
      start = null;
      delete this.el.dataset.dragging;
      this.ctx.ui.toolboxX.set(this.pos.x);
      this.ctx.ui.toolboxY.set(this.pos.y);
    };
    this.d.add(listen(grip, 'pointerup', end));
    this.d.add(listen(grip, 'pointercancel', end));
  }
}
