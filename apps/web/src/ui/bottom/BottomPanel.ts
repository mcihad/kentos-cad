import type { AppContext } from '../../app/context';
import { BOTTOM_HEIGHT, bottomHeightOn } from '../../app/layoutPlan';
import type { BottomTab, LogEntry } from '../../app/state';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import type { Entity } from '../../model/entities';
import { bearingGrad, dist } from '../../model/geometry';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { splitter } from '../widgets/Splitter';
import { tooltip } from '../widgets/tooltip';
import { tableSpacer, VirtualRows } from '../widgets/VirtualRows';
import { CommandLine } from './CommandLine';
import { vertexListing } from './coordinates';
import { FeatureTable } from './FeatureTable';
import { PointTable } from './PointTable';
import { SearchPanel } from './SearchPanel';
import { isEditable } from './vertexEdit';
import { VERTEX_TEXTS, VertexTable } from './VertexTable';
import { BOTTOM_TABS, BOTTOM_TEXTS, FOLLOW_WITHIN, ICON_SIZE, LEVEL_ICON, listedIn, logTime } from './logPlan';
import { seenNow, unseenWarnings } from './warnings';

/**
 * Bottom region: the command line is always there; the tabbed panel above it
 * opens when needed (F2, or automatically for coordinate lists).
 */
export class BottomPanel extends Component {
  readonly el: HTMLElement;
  readonly commandLine: CommandLine;
  private readonly ctx: AppContext;
  private readonly content: HTMLElement;
  private readonly tabButtons = new Map<BottomTab, HTMLButtonElement>();
  private readonly badge = h('span', { class: 'badge', hidden: true });
  /** The newest log entry's id when the Uyarılar tab was last on screen: the badge counts the warnings after it. */
  private seenUpTo = 0;
  /** The log list on screen, its tab and the entries it shows (new entries are appended, not the whole list rebuilt). */
  private log: { tab: BottomTab; list: HTMLElement; shown: LogEntry[] } | null = null;
  /** The coordinate table's rows, built only in its scroll window. */
  private rows: VirtualRows | null = null;
  /** Noktalar, the point editor (docs/adr/0153), while its tab is on screen. */
  private points: PointTable | null = null;
  /** Köşe tablosu (docs/adr/0172), while the coordinate list shows line work: it follows its object's changes itself. */
  private vertices: VertexTable | null = null;
  /** Arama, the data search (docs/adr/0178), while its tab is on screen. */
  private search: SearchPanel | null = null;
  /** Tablo, the attribute table (docs/adr/0199 §4), while its tab is on screen. */
  private features: FeatureTable | null = null;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const { ui } = ctx;
    this.commandLine = new CommandLine(ctx);
    this.content = h('div', { class: 'bottom__content', role: 'tabpanel' });

    const tabs = h(
      'div',
      { class: 'bottom__tabs', role: 'tablist', 'aria-label': BOTTOM_TEXTS.tabs },
      BOTTOM_TABS.map((t) => {
        const b = h(
          'button',
          { class: 'tab', type: 'button', role: 'tab', 'aria-selected': 'false' },
          icon(t.icon, 15),
          h('span', null, t.label),
          t.id === 'messages' ? this.badge : null,
        );
        b.addEventListener('click', () => ui.bottomTab.set(t.id));
        this.tabButtons.set(t.id, b);
        return b;
      }),
    );
    const clear = h('button', { class: 'ibtn', type: 'button', 'aria-label': BOTTOM_TEXTS.clear }, icon('clear', 16));
    const collapse = h('button', { class: 'ibtn', type: 'button', 'aria-label': BOTTOM_TEXTS.close }, icon('chevronDown', 16));
    this.d.add(listen(clear, 'click', () => ctx.log.clear()));
    this.d.add(listen(collapse, 'click', () => ui.bottomExpanded.set(false)));
    this.d.add(tooltip(clear, () => ({ title: BOTTOM_TEXTS.clear }), 'top'));
    this.d.add(tooltip(collapse, () => ({ title: BOTTOM_TEXTS.close, shortcut: ctx.keymap.chordFor('view.bottomPanel') }), 'top'));

    // A drag starts from the height shown (the kept one may be taller than this window allows).
    let startH = 0;
    const split = splitter({
      orientation: 'horizontal',
      label: BOTTOM_TEXTS.edge,
      onStart: () => (startH = bottomHeightOn(ui.bottomHeight.value, innerHeight)),
      onDrag: (dy) => ui.bottomHeight.set(bottomHeightOn(startH - dy, innerHeight)),
      onReset: () => ui.bottomHeight.set(BOTTOM_HEIGHT.reset),
    });
    this.d.add(split.dispose);

    const expandBtn = h('button', { class: 'ibtn cmdline__expand', type: 'button', 'aria-label': BOTTOM_TEXTS.open }, icon('chevronUp', 16));
    this.d.add(listen(expandBtn, 'click', () => ui.bottomExpanded.set(!ui.bottomExpanded.value)));
    this.d.add(tooltip(expandBtn, () => ({ title: ui.bottomExpanded.value ? BOTTOM_TEXTS.close : BOTTOM_TEXTS.open, shortcut: ctx.keymap.chordFor('view.bottomPanel') }), 'top'));
    this.commandLine.el.append(expandBtn);

    const panel = h(
      'div',
      { class: 'bottom__panel' },
      split.el,
      h('div', { class: 'bottom__bar' }, tabs, h('div', { class: 'bottom__actions' }, clear, collapse)),
      this.content,
    );
    this.el = h('div', { class: 'bottom' }, panel, this.commandLine.el);

    this.d.add(
      ui.bottomExpanded.subscribe((open) => {
        panel.hidden = !open;
        expandBtn.setAttribute('aria-expanded', String(open));
        expandBtn.replaceChildren(icon(open ? 'chevronDown' : 'chevronUp', 16));
        if (open) this.renderContent();
        this.refreshBadge();
      }, true),
    );
    // The kept height, within what the window allows now (app/layoutPlan.ts); a lower window does not change what is kept.
    const height = () => this.el.style.setProperty('--bottom-h', `${bottomHeightOn(ui.bottomHeight.value, innerHeight)}px`);
    this.d.add(ui.bottomHeight.subscribe(height, true));
    this.d.add(listen(window, 'resize', height));
    this.d.add(
      ui.bottomTab.subscribe((t) => {
        this.tabButtons.forEach((b, id) => b.setAttribute('aria-selected', String(id === t)));
        this.renderContent();
        this.refreshBadge();
      }, true),
    );
    this.d.add(watchAll([ctx.log.entries], () => this.onLog()));
    this.d.add(watchAll([ctx.selection.ids, ctx.format.changed], () => ui.bottomTab.value === 'coords' && this.renderContent()));
    // Köşe tablosu follows its object itself (an edit keeps its cell going); anything else builds the list again.
    this.d.add(ctx.doc.events.on('changed', () => ui.bottomTab.value === 'coords' && !this.vertices?.alive && this.renderContent()));
    this.d.add(() => this.rows?.dispose());
    this.d.add(() => this.points?.dispose());
    this.d.add(() => this.vertices?.dispose());
    this.d.add(() => this.search?.dispose());
    this.d.add(() => this.features?.dispose());
  }

  /**
   * The Uyarılar tab's badge: the warnings logged since the tab was last on screen. While it is on
   * screen every warning is seen and the badge is hidden; after Geçmişi temizle it counts from zero.
   */
  private refreshBadge(): void {
    const { ui, log } = this.ctx;
    const entries = log.entries.value;
    this.seenUpTo = seenNow(entries, this.seenUpTo, ui.bottomExpanded.value && ui.bottomTab.value === 'messages');
    const n = unseenWarnings(entries, this.seenUpTo);
    this.badge.hidden = n === 0;
    this.badge.textContent = String(n);
  }

  private onLog(): void {
    this.refreshBadge();
    const tab = this.ctx.ui.bottomTab.value;
    if ((tab === 'history' || tab === 'messages') && !this.appendLog(tab)) this.renderContent();
  }

  private entriesOf(tab: 'history' | 'messages'): LogEntry[] {
    return this.ctx.log.entries.value.filter((e) => listedIn(tab, e.level));
  }

  /**
   * The log grew (and perhaps dropped its oldest entries, it keeps 500): the new rows are added and the
   * dropped ones removed, instead of the whole list being built again at every message. False when the
   * list on screen cannot follow that way (another tab, cleared), and it is built again.
   */
  private appendLog(tab: 'history' | 'messages'): boolean {
    const log = this.log;
    if (!this.ctx.ui.bottomExpanded.value || !log || log.tab !== tab || !log.list.isConnected) return false;
    const entries = this.entriesOf(tab);
    const firstKept = log.shown.findIndex((e) => e.id === entries[0]?.id);
    if (!entries.length || firstKept < 0) return false;
    const last = log.shown.at(-1);
    const at = last ? entries.findIndex((e) => e.id === last.id) : -1;
    if (last && at < 0) return false;
    const follow = log.list.parentElement ? this.content.scrollHeight - this.content.scrollTop - this.content.clientHeight < FOLLOW_WITHIN : true;
    for (let i = 0; i < firstKept; i++) log.list.firstElementChild?.remove();
    const added = entries.slice(at + 1);
    if (added.length) log.list.append(...added.map((e) => this.logRow(e)));
    log.shown = entries;
    if (follow) this.content.scrollTop = this.content.scrollHeight;
    return true;
  }

  private renderContent(): void {
    this.rows?.dispose();
    this.rows = null;
    this.points?.dispose();
    this.points = null;
    this.vertices?.dispose();
    this.vertices = null;
    this.search?.dispose();
    this.search = null;
    this.features?.dispose();
    this.features = null;
    this.log = null;
    if (!this.ctx.ui.bottomExpanded.value) return;
    const tab = this.ctx.ui.bottomTab.value;
    if (tab === 'coords') return replaceChildren(this.content, this.coordinateTable());
    if (tab === 'points') {
      this.points = new PointTable(this.ctx);
      return replaceChildren(this.content, this.points.el);
    }
    if (tab === 'search') {
      this.search = new SearchPanel(this.ctx);
      return replaceChildren(this.content, this.search.el);
    }
    if (tab === 'table') {
      this.features = new FeatureTable(this.ctx);
      return replaceChildren(this.content, this.features.el);
    }
    const entries = this.entriesOf(tab);
    if (!entries.length) {
      return replaceChildren(
        this.content,
        h('div', { class: 'empty empty--inline' }, BOTTOM_TEXTS.empty[tab]),
      );
    }
    const list = h('ol', { class: 'log' }, entries.map((e) => this.logRow(e)));
    replaceChildren(this.content, list);
    this.log = { tab, list, shown: entries };
    this.content.scrollTop = this.content.scrollHeight;
  }

  private logRow(e: LogEntry): HTMLElement {
    const ic = LEVEL_ICON[e.level];
    return h(
      'li',
      { class: `log__row log__row--${e.level}` },
      h('time', { class: 'log__time num' }, logTime(e.time)),
      h('span', { class: 'log__icon' }, ic ? icon(ic, ICON_SIZE) : null),
      h('span', { class: 'log__text' }, e.text),
    );
  }

  private coordinateTable(): HTMLElement {
    const { doc, selection, format: f } = this.ctx;
    const ents = [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
    if (!ents.length) return h('div', { class: 'empty empty--inline' }, 'Koordinat listesi için çizimde bir nesne seçin.');

    if (ents.every((e) => e.kind === 'point')) {
      // Every point of a multi-point object, its name followed by its place in it (docs/adr/0174 §6).
      const points = ents.flatMap((e) => {
        if (e.kind !== 'point') return [];
        const name = e.label ?? `#${e.id}`;
        const layer = doc.layers.get(e.layerId)?.name ?? '';
        return [{ p: e.p, z: e.z }, ...(e.parts ?? [])].map((q, k) => ({ name: k ? `${name} (${k + 1})` : name, p: q.p, z: q.z ?? undefined, layer }));
      });
      return this.table(
        ['Nokta', f.axesText('Y (sağa)'), f.axesText('X (yukarı)'), 'Z (kot)', 'Katman'],
        points.length,
        (i) => {
          const p = points[i];
          return [p.name, f.coord(p.p.x), f.coord(p.p.y), p.z !== undefined ? f.length(p.z, false) : '—', p.layer];
        },
        `${points.length} nokta`,
        [1, 2, 3],
      );
    }

    const e = ents.find((x) => x.kind !== 'point' && x.kind !== 'text') ?? ents[0];
    // Line work in Köşe tablosu (docs/adr/0172 §1): it writes for one object off a locked layer.
    if (isEditable(e)) {
      const locked = doc.layers.isLocked(e.layerId);
      const note = ents.length > 1 ? VERTEX_TEXTS.manyNote : locked ? VERTEX_TEXTS.lockedNote : '';
      this.vertices = new VertexTable(this.ctx, e.id, { writes: ents.length === 1 && !locked, note });
      return this.vertices.el;
    }
    // Edges within each ring; the area and length are the object's own (./coordinates).
    const { pts, next: after, area, length } = vertexListing(e);
    // Rows are formatted when they scroll into view (a contour has hundreds of vertices).
    const row = (i: number) => {
      const p = pts[i];
      const n = after(i);
      const next = n === null ? null : pts[n];
      return [
        String(i + 1),
        f.coord(p.x),
        f.coord(p.y),
        next ? f.length(dist(p, next), false) : '',
        next ? f.direction(bearingGrad(p, next), false) : '',
      ];
    };
    const title = e.label ? `${e.attrs.Ada ? `${e.attrs.Ada} ada ` : ''}${e.label}` : `#${e.id}`;
    const footer =
      area !== null
        ? `${title}   Alan ${f.area(area)}${length !== null ? `   Çevre ${f.length(length)}` : ''}`
        : length !== null
          ? `${title}   Uzunluk ${f.length(length)}`
          : title;
    // East and north as the project's type names them (docs/adr/0165 §4); the edges in the project's unit (§2).
    const headings = ['Köşe', f.axesText('Y (sağa)'), f.axesText('X (yukarı)'), `Kenar (${f.lengthUnitLabel})`, `${f.directionName} (${f.angleUnitLabel})`];
    return this.table(headings, pts.length, row, ents.length > 1 ? `${footer}   (ilk nesne gösteriliyor)` : footer, [0, 1, 2, 3, 4]);
  }

  /** A coordinate table whose rows are built only in the panel's scroll window (ten thousand points stay light). */
  private table(head: string[], count: number, row: (i: number) => string[], footer: string, numeric: number[]): HTMLElement {
    const cls = (i: number) => (numeric.includes(i) ? 'num' : null);
    const body = h('tbody');
    const el = h(
      'div',
      { class: 'ctable' },
      h('table', null, h('thead', null, h('tr', null, head.map((c, i) => h('th', { class: cls(i) }, c)))), body),
      h('div', { class: 'ctable__foot num' }, footer),
    );
    this.rows = new VirtualRows({
      parent: body,
      scroller: this.content,
      row: (i) => h('tr', null, row(i).map((c, j) => h('td', { class: cls(j) }, c))),
      spacer: tableSpacer(head.length),
    });
    // Built once the table is in the page (the first row's height is measured there).
    queueMicrotask(() => this.rows?.set(count));
    return el;
  }
}
