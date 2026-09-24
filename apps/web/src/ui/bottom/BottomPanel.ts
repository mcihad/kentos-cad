import type { AppContext } from '../../app/context';
import type { BottomTab, LogEntry, LogLevel } from '../../app/state';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { entityVertices, type Entity } from '../../model/entities';
import { bearingGrad, dist, pathLength, signedArea } from '../../model/geometry';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { splitter } from '../widgets/Splitter';
import { tooltip } from '../widgets/tooltip';
import { CommandLine } from './CommandLine';

const TABS: { id: BottomTab; label: string; icon: string }[] = [
  { id: 'history', label: 'Komut geçmişi', icon: 'history' },
  { id: 'coords', label: 'Koordinat listesi', icon: 'table' },
  { id: 'messages', label: 'Uyarılar', icon: 'warning' },
];

const LEVEL_ICON: Record<LogLevel, string | null> = { command: null, info: null, success: 'success', warn: 'warning', error: 'error' };
const time = (d: Date) => d.toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit', second: '2-digit' });

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
  private seenWarnings = 0;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    const { ui } = ctx;
    this.commandLine = new CommandLine(ctx);
    this.content = h('div', { class: 'bottom__content', role: 'tabpanel' });

    const tabs = h(
      'div',
      { class: 'bottom__tabs', role: 'tablist', 'aria-label': 'Alt panel' },
      TABS.map((t) => {
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
    const clear = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Geçmişi temizle' }, icon('clear', 16));
    const collapse = h('button', { class: 'ibtn', type: 'button', 'aria-label': 'Paneli kapat' }, icon('chevronDown', 16));
    this.d.add(listen(clear, 'click', () => ctx.log.clear()));
    this.d.add(listen(collapse, 'click', () => ui.bottomExpanded.set(false)));
    this.d.add(tooltip(clear, () => ({ title: 'Geçmişi temizle' }), 'top'));
    this.d.add(tooltip(collapse, () => ({ title: 'Paneli kapat', shortcut: ctx.keymap.chordFor('view.bottomPanel') }), 'top'));

    let startH = 0;
    const split = splitter({
      orientation: 'horizontal',
      label: 'Alt panel yüksekliği',
      onStart: () => (startH = ui.bottomHeight.value),
      onDrag: (dy) => ui.bottomHeight.set(Math.round(Math.min(Math.max(startH - dy, 96), innerHeight * 0.6))),
      onReset: () => ui.bottomHeight.set(190),
    });
    this.d.add(split.dispose);

    const expandBtn = h('button', { class: 'ibtn cmdline__expand', type: 'button', 'aria-label': 'Komut geçmişini aç' }, icon('chevronUp', 16));
    this.d.add(listen(expandBtn, 'click', () => ui.bottomExpanded.set(!ui.bottomExpanded.value)));
    this.d.add(tooltip(expandBtn, () => ({ title: ui.bottomExpanded.value ? 'Paneli kapat' : 'Komut geçmişini aç', shortcut: ctx.keymap.chordFor('view.bottomPanel') }), 'top'));
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
      }, true),
    );
    this.d.add(ui.bottomHeight.subscribe((v) => this.el.style.setProperty('--bottom-h', `${v}px`), true));
    this.d.add(
      ui.bottomTab.subscribe((t) => {
        this.tabButtons.forEach((b, id) => b.setAttribute('aria-selected', String(id === t)));
        if (t === 'messages') this.seenWarnings = this.warningCount();
        this.renderContent();
      }, true),
    );
    this.d.add(watchAll([ctx.log.entries], () => this.onLog()));
    this.d.add(watchAll([ctx.selection.ids, ctx.format.changed], () => ui.bottomTab.value === 'coords' && this.renderContent()));
    this.d.add(ctx.doc.events.on('changed', () => ui.bottomTab.value === 'coords' && this.renderContent()));
  }

  private warningCount(): number {
    return this.ctx.log.entries.value.filter((e) => e.level === 'warn' || e.level === 'error').length;
  }

  private onLog(): void {
    const n = this.warningCount() - this.seenWarnings;
    this.badge.hidden = n <= 0;
    this.badge.textContent = String(n);
    const tab = this.ctx.ui.bottomTab.value;
    if (tab === 'history' || tab === 'messages') this.renderContent();
    if (tab === 'messages') this.seenWarnings = this.warningCount();
  }

  private renderContent(): void {
    if (!this.ctx.ui.bottomExpanded.value) return;
    const tab = this.ctx.ui.bottomTab.value;
    if (tab === 'coords') return replaceChildren(this.content, this.coordinateTable());
    const entries = this.ctx.log.entries.value.filter((e) => tab === 'history' || e.level === 'warn' || e.level === 'error');
    if (!entries.length) {
      return replaceChildren(
        this.content,
        h('div', { class: 'empty empty--inline' }, tab === 'history' ? 'Henüz komut çalıştırılmadı. Bir araç seçin ya da komut satırına yazın.' : 'Uyarı yok.'),
      );
    }
    const list = h('ol', { class: 'log' }, entries.map((e) => this.logRow(e)));
    replaceChildren(this.content, list);
    list.scrollTop = list.scrollHeight;
  }

  private logRow(e: LogEntry): HTMLElement {
    const ic = LEVEL_ICON[e.level];
    return h(
      'li',
      { class: `log__row log__row--${e.level}` },
      h('time', { class: 'log__time num' }, time(e.time)),
      h('span', { class: 'log__icon' }, ic ? icon(ic, 14) : null),
      h('span', { class: 'log__text' }, e.text),
    );
  }

  private coordinateTable(): HTMLElement {
    const { doc, selection, format: f } = this.ctx;
    const ents = [...selection.ids.value].map((id) => doc.get(id)).filter((e): e is Entity => !!e);
    if (!ents.length) return h('div', { class: 'empty empty--inline' }, 'Koordinat listesi için çizimde bir nesne seçin.');

    const points = ents.filter((e) => e.kind === 'point');
    if (points.length === ents.length) {
      return table(
        ['Nokta', 'Y (sağa)', 'X (yukarı)', 'Z (kot)', 'Katman'],
        points.map((p) => (p.kind === 'point' ? [p.label ?? `#${p.id}`, f.coord(p.p.x), f.coord(p.p.y), p.z !== undefined ? f.length(p.z, false) : '—', doc.layers.get(p.layerId)?.name ?? ''] : [])),
        `${points.length} nokta`,
        [1, 2, 3],
      );
    }

    const e = ents.find((x) => x.kind !== 'point' && x.kind !== 'text') ?? ents[0];
    const pts = entityVertices(e);
    const closed = e.kind === 'polygon';
    const rows = pts.map((p, i) => {
      const next = pts[i + 1] ?? (closed ? pts[0] : null);
      return [
        String(i + 1),
        f.coord(p.x),
        f.coord(p.y),
        next ? f.length(dist(p, next), false) : '',
        next ? f.bearing(bearingGrad(p, next), false) : '',
      ];
    });
    const title = e.label ? `${e.attrs.Ada ? `${e.attrs.Ada} ada ` : ''}${e.label}` : `#${e.id}`;
    const footer = closed
      ? `${title}   Alan ${f.area(Math.abs(signedArea(pts)))}   Çevre ${f.length(pathLength(pts, true))}`
      : `${title}   Uzunluk ${f.length(pathLength(pts))}`;
    return table(['Köşe', 'Y (sağa)', 'X (yukarı)', 'Kenar (m)', `Semt (${f.angleUnitLabel})`], rows, ents.length > 1 ? `${footer}   (ilk nesne gösteriliyor)` : footer, [0, 1, 2, 3, 4]);
  }
}

function table(head: string[], rows: string[][], footer: string, numeric: number[]): HTMLElement {
  const cls = (i: number) => (numeric.includes(i) ? 'num' : null);
  return h(
    'div',
    { class: 'ctable' },
    h(
      'table',
      null,
      h('thead', null, h('tr', null, head.map((c, i) => h('th', { class: cls(i) }, c)))),
      h('tbody', null, rows.map((r) => h('tr', null, r.map((c, i) => h('td', { class: cls(i) }, c))))),
    ),
    h('div', { class: 'ctable__foot num' }, footer),
  );
}
