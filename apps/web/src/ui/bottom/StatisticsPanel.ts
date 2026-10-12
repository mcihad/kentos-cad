import '../../styles/charts.css';
import type { AppContext } from '../../app/context';
import { charts, openChart, shownCharts } from '../../app/charts';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import type { FieldDetail, FieldSummary } from '../../model/ops/charts';
import type { TableShow } from '../../model/ops/featureTable';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';
import type { MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { applyPick, LayerData, layerAtHand } from '../charts/layerData';

/**
 * İstatistik, the bottom panel's statistics tab (docs/adr/0227 §3, §8; the desktop's `statistics/`): a layer's every
 * field in a summary table (dolu, boş, benzersiz, en az, en çok, ortalama, toplam), the chosen field's full statistics
 * and its values with their counts and shares. A value's row selects its objects (Shift adds, Ctrl takes away). The
 * core counts (`ops::charts::fields`); the summaries are counted a few fields a frame so a large layer never freezes it.
 */

export const STATISTICS_TEXTS = {
  layer: 'Katman',
  show: 'Göster',
  all: 'Tümü',
  selected: 'Seçililer',
  inView: 'Görünümdekiler',
  chart: 'Grafik',
  chartHint: 'Seçilen alanın grafiği: sayılarda histogram, öbürlerinde değerlere göre çubuk',
  copy: 'Panoya kopyala',
  copyHint: 'Özet tabloyu ve seçilen alanın istatistiklerini sekmeyle ayrılmış olarak panoya kopyalar',
  copied: 'İstatistikler panoya kopyalandı.',
  noLayers: 'Çizimde katman yok.',
  none: 'Katmanda nesne yok.',
  pick: 'Bütün istatistikleri ve değerleri için bir alan seçin.',
  values: 'Değerler',
  more: (n: number) => `+${n} değer daha`,
  objects: (n: number) => `${n} nesne`,
  columns: ['Alan', 'Tür', 'Dolu', 'Boş', 'Benzersiz', 'En az', 'En çok', 'Ortalama', 'Toplam'],
} as const;

const SHOWS: readonly { show: TableShow; label: string }[] = [
  { show: 'all', label: STATISTICS_TEXTS.all },
  { show: 'selected', label: STATISTICS_TEXTS.selected },
  { show: 'inView', label: STATISTICS_TEXTS.inView },
];

/** A summary's cells in the table's order (the desktop's `summary_cells`). */
function summaryCells(s: FieldSummary): string[] {
  return [s.label, s.kind, String(s.filled), String(s.empty), String(s.distinct), s.min ?? '', s.max ?? '', s.mean ?? '', s.sum ?? ''];
}

/** How long a frame may count summaries (ms): the rest wait for the next frame. */
const FRAME_BUDGET = 8;

export class StatisticsPanel extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly layerPick: Dropdown;
  private readonly showPick: Dropdown;
  private readonly count = h('span', { class: 'ptable__count num' });
  private readonly chartBtn: HTMLButtonElement;
  private readonly body = h('tbody');
  private readonly detailEl = h('div', { class: 'stats__detail' });
  private readonly empty = h('div', { class: 'empty empty--inline ptable__empty' });
  private source: LayerData | null = null;
  private rows: Uint32Array<ArrayBufferLike> = new Uint32Array();
  private summaries: (FieldSummary | null)[] = [];
  private detail: FieldDetail | null = null;
  private frame = 0;
  private pending = false;
  private stale = true;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    this.layerPick = new Dropdown({ ariaLabel: STATISTICS_TEXTS.layer, label: STATISTICS_TEXTS.layer, width: 210, className: 'ftable__layer', items: () => this.layerItems() });
    this.showPick = new Dropdown({ ariaLabel: STATISTICS_TEXTS.show, label: STATISTICS_TEXTS.show, width: 190, items: () => this.showItems() });
    const button = (iconName: string, text: string, hint: string, run: () => void) => {
      const b = h('button', { class: 'btn btn--small ptable__btn', type: 'button' }, icon(iconName, 14), h('span', null, text));
      this.d.add(listen(b, 'click', run));
      this.d.add(tooltip(b, () => ({ title: text, description: hint }), 'top'));
      return b;
    };
    this.chartBtn = button('chart', STATISTICS_TEXTS.chart, STATISTICS_TEXTS.chartHint, () => this.chartOfField());
    const copyBtn = button('copy', STATISTICS_TEXTS.copy, STATISTICS_TEXTS.copyHint, () => this.copy());
    const head = h('tr', null, STATISTICS_TEXTS.columns.map((c, i) => h('th', { class: i >= 2 ? 'num' : null, scope: 'col' }, h('span', { class: 'stats__th' }, c))));
    const scroll = h('div', { class: 'ptable__scroll stats__scroll' }, h('table', { class: 'stats__table' }, h('thead', null, head), this.body), this.empty);
    this.el = h(
      'div',
      { class: 'ptable stats' },
      h(
        'div',
        { class: 'ptable__bar' },
        h('div', { class: 'ptable__group' }, this.layerPick.el, this.showPick.el, this.count),
        h('div', { class: 'ptable__group ptable__group--end' }, this.chartBtn, copyBtn),
      ),
      h('div', { class: 'stats__body' }, scroll, this.detailEl),
    );
    this.d.add(
      listen<MouseEvent>(this.body, 'click', (e) => {
        const tr = (e.target as HTMLElement).closest<HTMLElement>('tr[data-key]');
        if (!tr) return;
        charts.field = tr.dataset.key!;
        this.renderTable();
        this.renderDetail();
      }),
    );
    this.d.add(
      listen<MouseEvent>(this.detailEl, 'click', (e) => {
        const li = (e.target as HTMLElement).closest<HTMLElement>('[data-value]');
        if (!li || !this.source) return;
        applyPick(ctx, this.source.idsOf(this.source.data.valueRows(Number(li.dataset.value))), e);
      }),
    );
    // The data is read again when the drawing changes; Göster's rows when the selection or the view does.
    const dataChanged = () => ((this.stale = true), this.schedule());
    this.d.add(ctx.doc.events.on('changed', dataChanged));
    this.d.add(ctx.doc.events.on('attrs', dataChanged));
    this.d.add(ctx.doc.layers.version.subscribe(dataChanged));
    this.d.add(watchAll([ctx.selection.ids], () => (charts.show === 'selected' || !charts.layer) && this.schedule()));
    this.d.add(ctx.view.camera.changed.subscribe(() => charts.show === 'inView' && this.schedule()));
    this.d.add(() => cancelAnimationFrame(this.frame));
    this.d.add(() => this.source?.free());
    queueMicrotask(() => this.refresh());
  }

  private schedule(): void {
    if (this.pending) return;
    this.pending = true;
    queueMicrotask(() => {
      this.pending = false;
      this.refresh();
    });
  }

  private layerItems(): MenuItem[] {
    const { layers } = this.ctx.doc;
    const current = layerAtHand(this.ctx, charts.layer);
    return layers.leaves().map((l) => ({
      label: `${layers.path(l.id)} (${this.ctx.view.geometry.filterCounts(l.id)?.passed ?? this.ctx.doc.byLayer(l.id).length})`,
      radio: true,
      checked: l.id === current,
      run: () => {
        charts.layer = l.id;
        charts.field = null;
        this.stale = true;
        this.schedule();
      },
    }));
  }

  private showItems(): MenuItem[] {
    return SHOWS.map((s) => ({ label: s.label, radio: true, checked: charts.show === s.show, run: () => ((charts.show = s.show), this.schedule()) }));
  }

  private refresh(): void {
    if (!this.el.isConnected) return;
    const layer = layerAtHand(this.ctx, charts.layer);
    charts.layer = layer;
    if (this.stale || this.source?.layer !== layer) {
      this.source?.free();
      this.source = layer ? LayerData.of(this.ctx, layer) : null;
      this.stale = false;
    }
    this.layerPick.set(layer ? this.ctx.doc.layers.path(layer) : STATISTICS_TEXTS.noLayers);
    this.showPick.set(SHOWS.find((s) => s.show === charts.show)?.label ?? STATISTICS_TEXTS.all);
    const src = this.source;
    this.rows = src ? src.rows(this.ctx, charts.show) : new Uint32Array();
    this.count.textContent = STATISTICS_TEXTS.objects(this.rows.length);
    const columns = src?.data.columns ?? [];
    if (charts.field && !columns.some((c) => c.key === charts.field)) charts.field = null;
    this.summaries = columns.map(() => null);
    shownCharts.statistics = null;
    this.empty.hidden = !!src && src.ids.length > 0;
    replaceChildren(this.empty, !layer ? STATISTICS_TEXTS.noLayers : STATISTICS_TEXTS.none);
    this.renderTable();
    this.renderDetail();
    cancelAnimationFrame(this.frame);
    this.frame = requestAnimationFrame(() => this.count_());
  }

  /** The summaries counted a few fields a frame. */
  private count_(): void {
    const src = this.source;
    if (!src || !this.el.isConnected) return;
    const start = performance.now();
    let i = this.summaries.indexOf(null);
    while (i >= 0) {
      this.summaries[i] = src.data.summary(i, this.rows);
      this.paintRow(i);
      if (performance.now() - start > FRAME_BUDGET) break;
      i = this.summaries.indexOf(null);
    }
    if (this.summaries.includes(null)) this.frame = requestAnimationFrame(() => this.count_());
    // Every row counted: what the tab shows, for the traces (`shownCharts`).
    else shownCharts.statistics = { count: this.count.textContent ?? '', rows: this.summaries.map((s) => (s ? summaryCells(s).join(' | ') : '')) };
  }

  private rowOf(i: number): HTMLElement {
    const c = this.source!.data.columns[i];
    const s = this.summaries[i];
    const cells = s ? summaryCells(s) : [c.label, '', '…', '', '', '', '', '', ''];
    return h(
      'tr',
      { 'data-key': c.key, class: c.key === charts.field ? 'is-selected' : null },
      cells.map((t, j) => h('td', { class: j >= 2 ? 'num' : null, title: j === 0 && c.key.startsWith('$') ? 'Geometriden' : null }, t)),
    );
  }

  private paintRow(i: number): void {
    const old = this.body.children[i];
    if (old) old.replaceWith(this.rowOf(i));
  }

  private renderTable(): void {
    const src = this.source;
    replaceChildren(this.body, src ? src.data.columns.map((_, i) => this.rowOf(i)) : []);
    this.chartBtn.disabled = !src || !charts.field;
  }

  private renderDetail(): void {
    const src = this.source;
    const col = src ? src.data.columns.findIndex((c) => c.key === charts.field) : -1;
    if (!src || col < 0) {
      this.detail = null;
      return replaceChildren(this.detailEl, h('div', { class: 'empty empty--inline' }, STATISTICS_TEXTS.pick));
    }
    const d = src.data.detail(col, this.rows);
    this.detail = d;
    const head = h('div', { class: 'stats__head' }, h('span', { class: 'stats__name' }, d.summary.label), h('span', { class: 'stats__kind' }, d.summary.kind));
    const grid = h(
      'dl',
      { class: 'stats__grid' },
      d.stats.flatMap((s) => [h('dt', null, s.label), h('dd', { class: 'num' }, s.value)]),
    );
    const values = h(
      'ol',
      { class: 'stats__values' },
      d.values.map((v, i) =>
        h(
          'li',
          { 'data-value': String(i), class: v.mark === 'value' ? null : 'stats__value--mark', tabindex: '-1' },
          h('span', { class: 'stats__vtext' }, v.text),
          h('span', { class: 'stats__vcount num' }, String(v.count)),
          h('span', { class: 'stats__vshare num' }, `%${v.share}`),
        ),
      ),
      d.more ? h('li', { class: 'stats__more' }, STATISTICS_TEXTS.more(d.more)) : null,
    );
    const notes = d.notes.map((n) => h('div', { class: 'stats__note' }, icon('warning', 14), h('span', null, n)));
    replaceChildren(this.detailEl, head, grid, h('div', { class: 'stats__subhead' }, STATISTICS_TEXTS.values), values, ...notes);
  }

  /** Grafik: the chosen field's chart in the Grafik tab. */
  private chartOfField(): void {
    const c = this.source?.data.columns.find((x) => x.key === charts.field);
    if (c) openChart(this.ctx, { layer: this.source!.layer, spec: this.source!.data.forColumn(c.key) });
  }

  private copy(): void {
    const cols = STATISTICS_TEXTS.columns;
    const lines = [cols.join('\t')];
    for (const s of this.summaries) if (s) lines.push(summaryCells(s).join('\t'));
    const d = this.detail;
    if (d) {
      lines.push('', d.summary.label, ...d.stats.map((s) => `${s.label}\t${s.value}`), '', ['Değer', 'Nesne', 'Pay (%)'].join('\t'), ...d.values.map((v) => `${v.text}\t${v.count}\t${v.share}`));
    }
    void navigator.clipboard.writeText(lines.join('\n')).then(
      () => this.ctx.log.success(STATISTICS_TEXTS.copied),
      () => this.ctx.log.warn('İstatistikler panoya kopyalanamadı: tarayıcı izin vermedi.'),
    );
  }
}
