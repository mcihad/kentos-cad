import '../../styles/charts.css';
import type { AppContext } from '../../app/context';
import { CHART_KINDS, chartRequests, charts, shownCharts } from '../../app/charts';
import { listen } from '../../core/disposable';
import { watchAll } from '../../core/signal';
import { tableColumns } from '../../model/chartData';
import { type Aggregate, type BarSort, ChartData, type ChartInfo, type ChartKind, type ChartLayout, type ChartSpec, type DataColumn } from '../../model/ops/charts';
import type { TableShow } from '../../model/ops/featureTable';
import { Component } from '../Component';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { Dropdown } from '../widgets/Dropdown';
import { PopupMenu, type MenuItem } from '../widgets/PopupMenu';
import { tooltip } from '../widgets/tooltip';
import { chartFont, drawChart, PAPER, roleColors } from '../charts/drawChart';
import { applyPick, LayerData, layerAtHand } from '../charts/layerData';

/**
 * Grafik, the bottom panel's chart tab (docs/adr/0227 §4–§8; the desktop's `charts/`): a layer's or a İşlemler table's
 * chart, one of six kinds. The core works the chart out and lays it out as a drawing list (`ops::charts`); this tab
 * paints it, shows a tooltip, selects what a click (or Saçılım's dragged box) meets and marks the drawing's selection.
 * Grafik | Tablo shows its numbers; Panoya kopyala copies them; Kaydet saves it as PNG or SVG.
 */

export const CHART_TEXTS = {
  source: 'Kaynak',
  show: 'Göster',
  all: 'Tümü',
  selected: 'Seçililer',
  inView: 'Görünümdekiler',
  table: (t: string) => `İşlem tablosu: ${t}`,
  chartView: 'Grafik',
  tableView: 'Tablo',
  copy: 'Panoya kopyala',
  copyHint: 'Grafiğin sayılarını (Tablo görünümünü) sekmeyle ayrılmış olarak panoya kopyalar',
  copied: 'Grafiğin tablosu panoya kopyalandı.',
  save: 'Kaydet',
  saveHint: 'Grafiği açık temanın renkleriyle beyaz kâğıtta PNG (2×) ya da SVG olarak kaydeder',
  png: 'PNG resmi (2×)',
  svg: 'SVG çizimi',
  saved: (f: string) => `Grafik kaydedildi: ${f}`,
  category: 'Kategori',
  aggregate: 'Toplama',
  value: 'Değer',
  sort: 'Sırala',
  bins: 'Aralık sayısı',
  auto: 'Otomatik',
  guides: 'Ortalama ve ortanca',
  group: 'Grupla',
  none: 'Yok',
  x: 'X',
  y: 'Y',
  series: 'Seri',
  trend: 'Eğilim çizgisi',
  noLayers: 'Çizimde katman yok.',
  noNumbers: 'Bu kaynakta sayı alanı yok.',
} as const;

const SHOWS: readonly { show: TableShow; label: string }[] = [
  { show: 'all', label: CHART_TEXTS.all },
  { show: 'selected', label: CHART_TEXTS.selected },
  { show: 'inView', label: CHART_TEXTS.inView },
];

const AGGREGATES: readonly { a: Aggregate; label: string }[] = [
  { a: 'none', label: 'Yok' },
  { a: 'count', label: 'Sayı' },
  { a: 'sum', label: 'Toplam' },
  { a: 'mean', label: 'Ortalama' },
  { a: 'min', label: 'En az' },
  { a: 'max', label: 'En çok' },
  { a: 'median', label: 'Ortanca' },
];

const SORTS: readonly { s: BarSort; label: string }[] = [
  { s: 'category', label: 'Kategori' },
  { s: 'desc', label: 'Değer azalan' },
  { s: 'asc', label: 'Değer artan' },
];

const BINS = [0, 5, 10, 15, 20, 30, 50, 100];

export class ChartPanel extends Component {
  readonly el: HTMLElement;
  private readonly ctx: AppContext;
  private readonly sourcePick: Dropdown;
  private readonly showPick: Dropdown;
  private readonly kinds = h('div', { class: 'seg seg--icons', role: 'radiogroup', 'aria-label': 'Grafik türü' });
  private readonly views = h('div', { class: 'seg', role: 'radiogroup', 'aria-label': 'Görünüm' });
  private readonly settings = h('div', { class: 'chart__settings' });
  private readonly stage = h('div', { class: 'chart__stage' });
  private readonly canvas = h('canvas', { class: 'chart__canvas' });
  private readonly tip = h('div', { class: 'chart__tip', role: 'tooltip', hidden: true });
  private readonly tableEl = h('div', { class: 'ptable__scroll chart__table' });
  private readonly band = h('div', { class: 'chart__band', hidden: true });
  private layerData: LayerData | null = null;
  private tableData: ChartData | null = null;
  private tableFor: unknown = null;
  private info: ChartInfo | null = null;
  private error: string | null = null;
  private layout: ChartLayout | null = null;
  private points: Float32Array<ArrayBufferLike> = new Float32Array();
  private pointElems: Uint32Array<ArrayBufferLike> = new Uint32Array();
  private hovered = -1;
  private drag: { x: number; y: number } | null = null;
  private pending = false;
  private stale = true;
  private readonly resize: ResizeObserver;

  constructor(ctx: AppContext) {
    super();
    this.ctx = ctx;
    this.sourcePick = new Dropdown({ ariaLabel: CHART_TEXTS.source, items: () => this.sourceItems() });
    this.showPick = new Dropdown({ ariaLabel: CHART_TEXTS.show, items: () => this.showItems() });
    const ibtn = (iconName: string, text: string, hint: string, run: (b: HTMLButtonElement) => void, caret = false) => {
      const b = h('button', { class: 'btn btn--small ptable__btn', type: 'button' }, icon(iconName, 14), h('span', null, text), caret ? icon('chevronDown', 12) : null);
      this.d.add(listen(b, 'click', () => run(b)));
      this.d.add(tooltip(b, () => ({ title: text, description: hint }), 'top'));
      return b;
    };
    const copyBtn = ibtn('copy', CHART_TEXTS.copy, CHART_TEXTS.copyHint, () => this.copy());
    const saveBtn = ibtn(
      'export',
      CHART_TEXTS.save,
      CHART_TEXTS.saveHint,
      (b) =>
        PopupMenu.open(
          [
            { label: CHART_TEXTS.png, icon: 'exportPng', run: () => void this.save('png') },
            { label: CHART_TEXTS.svg, icon: 'exportSvg', run: () => void this.save('svg') },
          ],
          b.getBoundingClientRect(),
          { owner: b },
        ),
      true,
    );
    this.stage.append(this.canvas, this.band, this.tip);
    this.el = h(
      'div',
      { class: 'ptable chart' },
      h(
        'div',
        { class: 'ptable__bar' },
        h('div', { class: 'ptable__group' }, this.kinds),
        h('div', { class: 'ptable__group ptable__group--end' }, this.views, copyBtn, saveBtn),
      ),
      h('div', { class: 'chart__body' }, this.settings, h('div', { class: 'chart__main' }, this.stage, this.tableEl)),
    );
    this.resize = new ResizeObserver(() => this.paint());
    this.resize.observe(this.stage);
    this.d.add(() => this.resize.disconnect());
    this.d.add(listen<PointerEvent>(this.canvas, 'pointermove', (e) => this.onMove(e)));
    this.d.add(listen<PointerEvent>(this.canvas, 'pointerleave', () => this.setHover(-1, null)));
    this.d.add(listen<PointerEvent>(this.canvas, 'pointerdown', (e) => this.onDown(e)));
    this.d.add(listen<PointerEvent>(this.canvas, 'pointerup', (e) => this.onUp(e)));
    const dataChanged = () => ((this.stale = true), this.schedule());
    this.d.add(ctx.doc.events.on('changed', dataChanged));
    this.d.add(ctx.doc.events.on('attrs', dataChanged));
    this.d.add(ctx.doc.layers.version.subscribe(dataChanged));
    // Alan and Uzunluk are in the project's unit (docs/adr/0165 §2): another unit reads them again.
    let unit = ctx.doc.settings.unit;
    this.d.add(ctx.doc.settings.changed.subscribe(() => unit !== ctx.doc.settings.unit && ((unit = ctx.doc.settings.unit), dataChanged())));
    this.d.add(chartRequests.subscribe(() => this.schedule()));
    this.d.add(watchAll([ctx.selection.ids], () => this.onSelection()));
    this.d.add(ctx.view.camera.changed.subscribe(() => charts.show === 'inView' && charts.source === 'layer' && this.schedule()));
    this.d.add(() => this.layerData?.free());
    this.d.add(() => this.tableData?.free());
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

  // ── The data ──────────────────────────────────────────────────────────

  /** The data charted and its rows (Göster's for a layer, every row of a table), or none. */
  private source(): { data: ChartData; rows: Uint32Array } | null {
    if (charts.source === 'table' && charts.table) {
      if (this.tableFor !== charts.table) {
        this.tableData?.free();
        this.tableData = new ChartData(charts.table.rows.length, tableColumns(charts.table));
        this.tableFor = charts.table;
      }
      return { data: this.tableData!, rows: Uint32Array.from(charts.table.rows.keys()) };
    }
    charts.source = 'layer';
    const layer = layerAtHand(this.ctx, charts.layer);
    charts.layer = layer;
    if (!layer) return null;
    if (this.stale || this.layerData?.layer !== layer) {
      this.layerData?.free();
      this.layerData = LayerData.of(this.ctx, layer);
      this.stale = false;
    }
    return { data: this.layerData.data, rows: this.layerData.rows(this.ctx, charts.show) };
  }

  /** The spec kept for the source shown, made whole for its data (a table's first: its tool's suggestion). */
  private spec(data: ChartData): ChartSpec {
    if (charts.source === 'table') {
      charts.tableSpec = data.complete(charts.tableSpec ?? data.forTable(charts.table?.tool ?? ''));
      return charts.tableSpec;
    }
    charts.layerSpec = data.complete(charts.layerSpec ?? { kind: 'bar' });
    return charts.layerSpec;
  }

  private setSpec(change: Partial<ChartSpec>): void {
    if (charts.source === 'table') charts.tableSpec = { ...(charts.tableSpec ?? { kind: 'bar' }), ...change };
    else charts.layerSpec = { ...(charts.layerSpec ?? { kind: 'bar' }), ...change };
    this.schedule();
  }

  private refresh(): void {
    if (!this.el.isConnected) return;
    const src = this.source();
    this.info = null;
    this.error = null;
    if (src) {
      const spec = this.spec(src.data);
      try {
        this.info = src.data.chart(spec, src.rows);
        if (charts.source === 'layer' && this.layerData) src.data.select(this.layerData.selectedRows(this.ctx));
      } catch (e) {
        this.error = e instanceof Error ? e.message : String(e);
      }
      this.renderKinds(spec.kind);
      this.renderSettings(spec, src.data.columns);
    } else {
      replaceChildren(this.settings, ...this.dataRows(), h('div', { class: 'empty empty--inline' }, CHART_TEXTS.noLayers));
    }
    const layer = charts.source === 'layer' ? charts.layer : null;
    this.sourcePick.set(charts.source === 'table' && charts.table ? CHART_TEXTS.table(charts.table.title) : layer ? this.ctx.doc.layers.path(layer) : CHART_TEXTS.noLayers);
    this.showPick.set(SHOWS.find((s) => s.show === charts.show)?.label ?? CHART_TEXTS.all);
    this.renderViews();
    this.paint();
    this.publish();
  }

  /** What the tab shows, for the traces (`shownCharts`): the kind, the numbers, the marks, where an element is pressed. */
  private publish(): void {
    const data = this.data();
    const info = this.info;
    if (!data || !info) return void (shownCharts.chart = null);
    shownCharts.chart = {
      kind: info.kind,
      columns: info.table.columns,
      rows: info.table.rows.map((r) => r.join(' | ')),
      marked: [...data.marked()].map((e) => e + 1),
      at: (e) => {
        const p = data.elementAt(e - 1);
        if (!p) return null;
        const r = this.canvas.getBoundingClientRect();
        return { x: r.left + p.x, y: r.top + p.y };
      },
    };
  }

  private sourceItems(): MenuItem[] {
    const { layers } = this.ctx.doc;
    const items: MenuItem[] = layers.leaves().map((l) => ({
      label: `${layers.path(l.id)} (${this.ctx.view.geometry.filterCounts(l.id)?.passed ?? this.ctx.doc.byLayer(l.id).length})`,
      radio: true,
      checked: charts.source === 'layer' && l.id === charts.layer,
      run: () => {
        // Another layer has other columns: the kind stays, its columns are chosen again.
        if (l.id !== charts.layer && charts.layerSpec) charts.layerSpec = { kind: charts.layerSpec.kind };
        charts.layer = l.id;
        charts.source = 'layer';
        this.stale = true;
        this.schedule();
      },
    }));
    if (charts.table) {
      items.push({ kind: 'separator' }, { label: CHART_TEXTS.table(charts.table.title), icon: 'table', radio: true, checked: charts.source === 'table', run: () => ((charts.source = 'table'), this.schedule()) });
    }
    return items;
  }

  private showItems(): MenuItem[] {
    return SHOWS.map((s) => ({ label: s.label, radio: true, checked: charts.show === s.show, run: () => ((charts.show = s.show), this.schedule()) }));
  }

  // ── The bar's controls ─────────────────────────────────────────────────

  private renderKinds(current: ChartKind): void {
    replaceChildren(
      this.kinds,
      CHART_KINDS.map((k) => {
        const b = h('button', { class: 'seg__opt seg__opt--icon', type: 'button', role: 'radio', 'aria-checked': String(k.kind === current), 'aria-label': k.title }, icon(k.icon, 16));
        // Another kind keeps the columns chosen; its aggregate starts at the kind's own (a line's points each a row).
        b.addEventListener('click', () => k.kind !== current && this.setSpec({ kind: k.kind, aggregate: k.kind === 'line' ? 'none' : 'count' }));
        tooltip(b, () => ({ title: k.title, description: k.description }), 'top');
        return b;
      }),
    );
  }

  private renderViews(): void {
    replaceChildren(
      this.views,
      (['chart', 'table'] as const).map((v) => {
        const b = h('button', { class: 'seg__opt', type: 'button', role: 'radio', 'aria-checked': String(charts.view === v) }, v === 'chart' ? CHART_TEXTS.chartView : CHART_TEXTS.tableView);
        b.addEventListener('click', () => {
          charts.view = v;
          this.renderViews();
          this.paint();
        });
        return b;
      }),
    );
  }

  /** The kind's settings: a column's list, an aggregate, an order; the notes under them. */
  private renderSettings(spec: ChartSpec, columns: readonly DataColumn[]): void {
    const numbers = columns.filter((c) => c.kind === 'number');
    const label = (k?: string) => columns.find((c) => c.key === k)?.label ?? CHART_TEXTS.none;
    const pick = (title: string, value: string, items: () => MenuItem[], disabled = false) => {
      const d = new Dropdown({ ariaLabel: title, width: 220, items });
      d.set(value);
      d.el.disabled = disabled;
      return h('label', { class: 'chart__row' }, h('span', { class: 'chart__label' }, title), d.el);
    };
    const columnItems = (list: readonly DataColumn[], current: string | undefined, set: (k: string | undefined) => void, none = false): MenuItem[] => [
      ...(none ? [{ label: CHART_TEXTS.none, radio: true, checked: !current, run: () => set(undefined) } as MenuItem] : []),
      ...list.map((c) => ({ label: c.label, radio: true, checked: c.key === current, hint: c.key.startsWith('$') ? 'geometri' : undefined, run: () => set(c.key) })),
    ];
    const aggregateItems = (allowed: readonly Aggregate[]): MenuItem[] =>
      AGGREGATES.filter((a) => allowed.includes(a.a)).map((a) => ({ label: a.label, radio: true, checked: spec.aggregate === a.a, run: () => this.setSpec({ aggregate: a.a }) }));
    const aggLabel = AGGREGATES.find((a) => a.a === spec.aggregate)?.label ?? '';
    const check = (title: string, on: boolean, set: (v: boolean) => void) => {
      const box = h('input', { type: 'checkbox', checked: on });
      box.addEventListener('change', () => set(box.checked));
      return h('label', { class: 'chart__check' }, box, h('span', null, title));
    };
    const rows: (HTMLElement | null)[] = [];
    switch (spec.kind) {
      case 'bar':
      case 'pie': {
        rows.push(pick(CHART_TEXTS.category, label(spec.category), () => columnItems(columns, spec.category, (k) => this.setSpec({ category: k }))));
        rows.push(pick(CHART_TEXTS.aggregate, aggLabel, () => aggregateItems(spec.kind === 'pie' ? ['count', 'sum'] : ['count', 'sum', 'mean', 'min', 'max', 'median'])));
        rows.push(pick(CHART_TEXTS.value, spec.aggregate === 'count' ? '—' : label(spec.value), () => columnItems(numbers, spec.value, (k) => this.setSpec({ value: k })), spec.aggregate === 'count' || !numbers.length));
        if (spec.kind === 'bar') rows.push(pick(CHART_TEXTS.sort, SORTS.find((s) => s.s === spec.sort)?.label ?? '', () => SORTS.map((s) => ({ label: s.label, radio: true, checked: spec.sort === s.s, run: () => this.setSpec({ sort: s.s }) }))));
        break;
      }
      case 'histogram':
        rows.push(pick(CHART_TEXTS.value, label(spec.value), () => columnItems(numbers, spec.value, (k) => this.setSpec({ value: k })), !numbers.length));
        rows.push(
          pick(CHART_TEXTS.bins, spec.bins ? String(spec.bins) : CHART_TEXTS.auto, () =>
            BINS.map((b) => ({ label: b ? String(b) : CHART_TEXTS.auto, radio: true, checked: (spec.bins ?? 0) === b, run: () => this.setSpec({ bins: b || undefined }) })),
          ),
        );
        rows.push(check(CHART_TEXTS.guides, spec.guides !== false, (v) => this.setSpec({ guides: v })));
        break;
      case 'box':
        rows.push(pick(CHART_TEXTS.value, label(spec.value), () => columnItems(numbers, spec.value, (k) => this.setSpec({ value: k })), !numbers.length));
        rows.push(pick(CHART_TEXTS.group, spec.group ? label(spec.group) : CHART_TEXTS.none, () => columnItems(columns, spec.group, (k) => this.setSpec({ group: k }), true)));
        break;
      case 'scatter':
        rows.push(pick(CHART_TEXTS.x, label(spec.x), () => columnItems(numbers, spec.x, (k) => this.setSpec({ x: k })), !numbers.length));
        rows.push(pick(CHART_TEXTS.y, label(spec.y?.[0]), () => columnItems(numbers, spec.y?.[0], (k) => this.setSpec({ y: k ? [k] : [] })), !numbers.length));
        rows.push(check(CHART_TEXTS.trend, spec.trend !== false, (v) => this.setSpec({ trend: v })));
        break;
      case 'line': {
        rows.push(pick(CHART_TEXTS.x, label(spec.x), () => columnItems(numbers, spec.x, (k) => this.setSpec({ x: k })), !numbers.length));
        const ys = spec.y ?? [];
        rows.push(
          pick(CHART_TEXTS.y, ys.map(label).join(', ') || CHART_TEXTS.none, () =>
            numbers.map((c) => ({
              label: c.label,
              checked: ys.includes(c.key),
              stay: true,
              run: () => {
                const now = charts.source === 'table' ? (charts.tableSpec?.y ?? []) : (charts.layerSpec?.y ?? []);
                this.setSpec({ y: now.includes(c.key) ? now.filter((k) => k !== c.key) : [...now, c.key].slice(0, 8) });
              },
            })),
          ),
        );
        rows.push(pick(CHART_TEXTS.series, spec.series ? label(spec.series) : CHART_TEXTS.none, () => columnItems(columns, spec.series, (k) => this.setSpec({ series: k }), true), ys.length > 1));
        rows.push(pick(CHART_TEXTS.aggregate, aggLabel, () => aggregateItems(['none', 'count', 'sum', 'mean', 'min', 'max', 'median'])));
        break;
      }
    }
    const notes = [...(this.error ? [this.error] : []), ...(this.info?.notes ?? []), ...(numbers.length ? [] : [CHART_TEXTS.noNumbers])];
    replaceChildren(this.settings, ...this.dataRows(), ...rows, ...notes.map((n) => h('div', { class: 'chart__note' }, icon('warning', 14), h('span', null, n))));
  }

  /** Kaynak and (for a layer) Göster, the column's first rows. */
  private dataRows(): HTMLElement[] {
    const row = (title: string, el: HTMLElement) => h('label', { class: 'chart__row' }, h('span', { class: 'chart__label' }, title), el);
    return charts.source === 'table' ? [row(CHART_TEXTS.source, this.sourcePick.el)] : [row(CHART_TEXTS.source, this.sourcePick.el), row(CHART_TEXTS.show, this.showPick.el)];
  }

  // ── Painting ───────────────────────────────────────────────────────────

  private data(): ChartData | null {
    return charts.source === 'table' ? this.tableData : (this.layerData?.data ?? null);
  }

  private paint(): void {
    const tableView = charts.view === 'table';
    this.stage.hidden = tableView;
    this.tableEl.hidden = !tableView;
    if (tableView) return this.paintTable();
    const data = this.data();
    const w = this.stage.clientWidth;
    const hgt = this.stage.clientHeight;
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = Math.max(1, Math.round(w * dpr));
    this.canvas.height = Math.max(1, Math.round(hgt * dpr));
    this.canvas.style.width = `${w}px`;
    this.canvas.style.height = `${hgt}px`;
    const g = this.canvas.getContext('2d');
    if (!g) return;
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    g.clearRect(0, 0, w, hgt);
    this.layout = null;
    if (!data || !this.info || w < 10 || hgt < 10) return;
    const font = chartFont();
    this.layout = data.layOut({ width: w, height: hgt, font: font.size, family: font.family });
    if (!this.layout) return;
    this.points = data.points();
    this.pointElems = data.pointElements();
    drawChart(g, this.layout, this.points, this.pointElems, roleColors(), font.family, this.hovered);
  }

  private paintTable(): void {
    const t = this.info?.table;
    if (!t) return replaceChildren(this.tableEl, h('div', { class: 'empty empty--inline' }, this.error ?? ''));
    const numeric = t.columns.map((_, j) => t.rows.length > 0 && t.rows.every((r) => !r[j] || /^-?[0-9]+(\.[0-9]+)?$/.test(r[j])));
    replaceChildren(
      this.tableEl,
      h(
        'table',
        { class: 'chart__grid' },
        h('thead', null, h('tr', null, t.columns.map((c, j) => h('th', { class: numeric[j] ? 'num' : null }, c)))),
        h('tbody', null, t.rows.map((r) => h('tr', null, r.map((c, j) => h('td', { class: numeric[j] ? 'num' : null }, c))))),
      ),
    );
  }

  // ── The pointer ────────────────────────────────────────────────────────

  private at(e: PointerEvent): { x: number; y: number } {
    const r = this.canvas.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  private onMove(e: PointerEvent): void {
    const data = this.data();
    if (!data || !this.layout) return;
    const p = this.at(e);
    if (this.drag) {
      const x = Math.min(p.x, this.drag.x);
      const y = Math.min(p.y, this.drag.y);
      Object.assign(this.band.style, { left: `${x}px`, top: `${y}px`, width: `${Math.abs(p.x - this.drag.x)}px`, height: `${Math.abs(p.y - this.drag.y)}px` });
      this.band.hidden = Math.abs(p.x - this.drag.x) < 3 && Math.abs(p.y - this.drag.y) < 3;
      return;
    }
    const el = data.hit(p.x, p.y);
    this.setHover(el, el >= 0 ? p : null);
  }

  private setHover(el: number, at: { x: number; y: number } | null): void {
    const data = this.data();
    if (el !== this.hovered) {
      this.hovered = el;
      const g = this.canvas.getContext('2d');
      const font = chartFont();
      if (g && this.layout) {
        const dpr = window.devicePixelRatio || 1;
        g.setTransform(dpr, 0, 0, dpr, 0, 0);
        g.clearRect(0, 0, this.canvas.width, this.canvas.height);
        drawChart(g, this.layout, this.points, this.pointElems, roleColors(), font.family, el);
      }
    }
    if (!data || el < 0 || !at) return void (this.tip.hidden = true);
    const lines = data.tip(el);
    replaceChildren(
      this.tip,
      lines.map(([label, value], i) => h('div', { class: i === 0 ? 'chart__tipline chart__tipline--lead' : 'chart__tipline' }, h('span', { class: 'chart__tipvalue num' }, value), h('span', { class: 'chart__tiplabel' }, label))),
    );
    this.tip.hidden = false;
    // Beside the pointer, inside the stage.
    const sw = this.stage.clientWidth;
    const tw = this.tip.offsetWidth;
    const th = this.tip.offsetHeight;
    const left = at.x + 14 + tw > sw ? at.x - 14 - tw : at.x + 14;
    const top = Math.max(4, Math.min(at.y - th / 2, this.stage.clientHeight - th - 4));
    Object.assign(this.tip.style, { left: `${Math.max(4, left)}px`, top: `${top}px` });
  }

  private onDown(e: PointerEvent): void {
    if (e.button !== 0) return;
    // Saçılım's box: a drag selects the points within it.
    const spec = charts.source === 'layer' ? charts.layerSpec : null;
    if (spec?.kind === 'scatter') {
      this.drag = this.at(e);
      this.canvas.setPointerCapture(e.pointerId);
    }
  }

  private onUp(e: PointerEvent): void {
    const data = this.data();
    const ld = this.layerData;
    const p = this.at(e);
    const drag = this.drag;
    this.drag = null;
    this.band.hidden = true;
    if (!data || charts.source !== 'layer' || !ld) return;
    if (drag && (Math.abs(p.x - drag.x) >= 3 || Math.abs(p.y - drag.y) >= 3)) {
      applyPick(this.ctx, ld.idsOf(data.rowsWithin(drag.x, drag.y, p.x, p.y)), e);
      return;
    }
    const el = data.hit(p.x, p.y);
    if (el >= 0) applyPick(this.ctx, ld.idsOf(data.elementRows(el)), e);
    else if (!e.shiftKey && !e.ctrlKey && !e.metaKey) this.ctx.selection.set([]);
  }

  /** The drawing's selection marks the chart (Seçililer charts it again). */
  private onSelection(): void {
    if (charts.source !== 'layer') return;
    if (charts.show === 'selected' || !charts.layer) return this.schedule();
    const data = this.data();
    if (data && this.layerData && data.select(this.layerData.selectedRows(this.ctx))) this.paint();
    this.publish();
  }

  // ── Copy and save ─────────────────────────────────────────────────────

  private copy(): void {
    const t = this.info?.table;
    if (!t) return;
    const text = [t.columns, ...t.rows].map((r) => r.join('\t')).join('\n');
    void navigator.clipboard.writeText(text).then(
      () => this.ctx.log.success(CHART_TEXTS.copied),
      () => this.ctx.log.warn('Tablo panoya kopyalanamadı: tarayıcı izin vermedi.'),
    );
  }

  /** The chart on white paper in the light theme's colours: PNG at twice the stage's size, or SVG. */
  private async save(kind: 'png' | 'svg'): Promise<void> {
    const data = this.data();
    if (!data || !this.info) return;
    const font = chartFont();
    const frame = { width: Math.max(480, this.stage.clientWidth), height: Math.max(300, this.stage.clientHeight), font: font.size, family: font.family };
    const name = `${(this.info.yTitle || 'grafik').replace(/[^\p{L}\p{N}]+/gu, '-').replace(/^-|-$/g, '').toLowerCase() || 'grafik'}.${kind}`;
    let blob: Blob;
    if (kind === 'svg') blob = new Blob([data.svg(frame, { ...PAPER, font: font.family })], { type: 'image/svg+xml' });
    else {
      // Painted here with the page's typeface (an SVG drawn as a picture cannot reach the page's fonts).
      const plain = data.plainLayout(frame);
      const c = document.createElement('canvas');
      c.width = frame.width * 2;
      c.height = frame.height * 2;
      const g = c.getContext('2d')!;
      g.scale(2, 2);
      g.fillStyle = PAPER.background;
      g.fillRect(0, 0, frame.width, frame.height);
      drawChart(g, plain, Float32Array.from(plain.points), new Uint32Array(), { ...PAPER }, font.family);
      blob = await new Promise<Blob>((res, rej) => c.toBlob((b) => (b ? res(b) : rej(new Error('PNG yazılamadı'))), 'image/png'));
    }
    const a = h('a', { href: URL.createObjectURL(blob), download: name });
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 1000);
    this.ctx.log.success(CHART_TEXTS.saved(name));
  }
}
