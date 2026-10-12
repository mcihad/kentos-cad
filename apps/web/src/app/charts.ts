// Grafikler ve alan istatistikleri's commands and kept choices (docs/adr/0227 §8; the desktop's `charts/` and
// `statistics/`): the bottom panel's İstatistik and Grafik tabs, the six chart kinds, and a İşlemler table opened as a
// chart. The tabs keep their choices for the session; another drawing starts them again.

import { Signal } from '../core/signal';
import type { ChartKind, ChartSpec } from '../model/ops/charts';
import type { TableShow } from '../model/ops/featureTable';
import type { AppContext } from './context';

/** A İşlemler table charted: its tool's name and id (the chart its tool suggests, `specs::for_table`), its columns and rows. */
export interface ChartTable {
  readonly title: string;
  readonly tool: string;
  readonly columns: readonly string[];
  readonly rows: readonly (readonly string[])[];
}

/** The six kinds in the tabs' order, with their names and icons. */
export const CHART_KINDS: readonly { kind: ChartKind; label: string; command: string; icon: string; title: string; description: string; aliases: string[] }[] = [
  {
    kind: 'bar',
    label: 'Çubuk',
    command: 'chart.bar',
    icon: 'chartBar',
    title: 'Çubuk grafik',
    description: 'Bir alanın değerlerine göre nesne sayısı ya da başka bir alanın toplamı, ortalaması, en azı, en çoğu, ortancası; en çok 30 çubuk, gerisi “Diğer”.',
    aliases: ['CUBUKGRAFIK', 'BARCHART'],
  },
  {
    kind: 'pie',
    label: 'Pasta',
    command: 'chart.pie',
    icon: 'chartPie',
    title: 'Pasta grafik',
    description: 'Bir alanın değerlerinin payları (nesne sayısı ya da bir alanın toplamı) halka dilimleriyle; en çok 8 renk, gerisi “Diğer”.',
    aliases: ['PASTAGRAFIK', 'PIECHART'],
  },
  {
    kind: 'histogram',
    label: 'Histogram',
    command: 'chart.histogram',
    icon: 'chartHistogram',
    title: 'Histogram',
    description: 'Bir sayı alanının dağılımı eşit aralıklarla; aralık sayısı kendiliğinden (Sturges) ya da seçilerek; ortalama ve ortanca çizgileri.',
    aliases: ['HISTOGRAMGRAFIK', 'DAGILIMGRAFIGI'],
  },
  {
    kind: 'box',
    label: 'Kutu',
    command: 'chart.box',
    icon: 'chartBox',
    title: 'Kutu grafiği',
    description: 'Bir sayı alanının çeyrekleri, ortancası, bıyıkları ve aykırı değerleri; isteğe bağlı olarak bir alanın gruplarına göre.',
    aliases: ['KUTUGRAFIGI', 'BOXPLOT'],
  },
  {
    kind: 'scatter',
    label: 'Saçılım',
    command: 'chart.scatter',
    icon: 'chartScatter',
    title: 'Saçılım grafiği',
    description: 'İki sayı alanının ilişkisi: her nesne bir nokta; en küçük kareler eğilim çizgisi ve R².',
    aliases: ['SACILIMGRAFIGI', 'SCATTER'],
  },
  {
    kind: 'line',
    label: 'Çizgi',
    command: 'chart.line',
    icon: 'chartLine',
    title: 'Çizgi grafik',
    description: 'Bir sayı alanına göre bir ya da birkaç değer (seriler); her satır bir nokta ya da her X için toplama. ROC, Kesit ve Zaman serisi tabloları için.',
    aliases: ['CIZGIGRAFIK', 'LINECHART'],
  },
];

/** The tabs' choices, kept for the session (leaving a tab and coming back keeps them). */
export const charts: {
  /** The layer charted and counted; null: the first selected object's, else the active one. */
  layer: string | null;
  show: TableShow;
  /** Where the Grafik tab reads: a layer or the İşlemler table below. */
  source: 'layer' | 'table';
  table: ChartTable | null;
  /** The last spec, by source: a layer's and the table's. */
  layerSpec: ChartSpec | null;
  tableSpec: ChartSpec | null;
  /** Grafik | Tablo. */
  view: 'chart' | 'table';
  /** İstatistik's chosen field. */
  field: string | null;
} = { layer: null, show: 'all', source: 'layer', table: null, layerSpec: null, tableSpec: null, view: 'chart', field: null };

/** Asks an open Grafik tab to read the choices again (a command chose a kind, a table came). */
export const chartRequests = new Signal(0);

/** What the Grafik tab shows: its kind, its numbers (the cells joined by “ | ”), the elements the selection marks (1 first). */
export interface ChartShown {
  readonly kind: ChartKind;
  readonly columns: readonly string[];
  readonly rows: readonly string[];
  readonly marked: readonly number[];
  /** Element `e`'s place on the page (1 first), where a press selects it; none off the chart. */
  at(e: number): { x: number; y: number } | null;
}

/** What İstatistik shows: its count and its summary rows (the cells joined by “ | ”), once every row is counted. */
export interface StatisticsShown {
  readonly count: string;
  readonly rows: readonly string[];
}

/** What the tabs show now, written by them as they paint (the traces read it; docs/adr/0227). */
export const shownCharts: { chart: ChartShown | null; statistics: StatisticsShown | null } = { chart: null, statistics: null };

/** The choices as a new session starts them (the pictures start each scene from them). */
export function resetCharts(layer: string | null = null): void {
  Object.assign(charts, { layer, show: 'all', source: 'layer', table: null, layerSpec: null, tableSpec: null, view: 'chart', field: null });
  Object.assign(shownCharts, { chart: null, statistics: null });
}

/** The panel's height the tabs ask for: a chart needs room. */
const CHART_PANEL_HEIGHT = 300;

function openTab(ctx: AppContext, tab: 'statistics' | 'chart'): void {
  const { ui } = ctx;
  ui.bottomTab.set(tab);
  if (ui.bottomHeight.value < CHART_PANEL_HEIGHT) ui.bottomHeight.set(CHART_PANEL_HEIGHT);
  ui.bottomExpanded.set(true);
}

/** İstatistik for a layer (the tree's menu names one), else as it was. */
export function openStatistics(ctx: AppContext, layer?: string): void {
  if (layer) charts.layer = layer;
  openTab(ctx, 'statistics');
}

/** Grafik for a layer, with a kind (its columns chosen by the tab) or a spec. */
export function openChart(ctx: AppContext, opts: { layer?: string; kind?: ChartKind; spec?: ChartSpec } = {}): void {
  // Another layer has other columns: the kind stays, its columns are chosen again.
  if (opts.layer && opts.layer !== charts.layer && charts.layerSpec) charts.layerSpec = { kind: charts.layerSpec.kind };
  if (opts.layer) charts.layer = opts.layer;
  charts.source = 'layer';
  if (opts.spec) charts.layerSpec = opts.spec;
  // Another kind keeps the columns chosen; its aggregate starts at the kind's own (a line's points each a row).
  else if (opts.kind && opts.kind !== charts.layerSpec?.kind) charts.layerSpec = { ...(charts.layerSpec ?? {}), kind: opts.kind, aggregate: opts.kind === 'line' ? 'none' : 'count' };
  charts.view = 'chart';
  openTab(ctx, 'chart');
  chartRequests.set(chartRequests.value + 1);
}

/** A İşlemler table as a chart (its tool's suggestion, else bars). */
export function openTableChart(ctx: AppContext, table: ChartTable): void {
  charts.table = table;
  charts.source = 'table';
  charts.tableSpec = null;
  charts.view = 'chart';
  openTab(ctx, 'chart');
  chartRequests.set(chartRequests.value + 1);
}

export function registerChartCommands(ctx: AppContext): void {
  const V = 'Veri';
  const layerArg = (args: unknown) => (typeof args === 'string' && ctx.doc.layers.get(args)?.type === 'layer' ? args : undefined);
  ctx.commands.registerAll([
    {
      id: 'data.statistics',
      title: 'Alan istatistikleri',
      short: 'İstatistik',
      category: V,
      icon: 'fieldStatistics',
      aliases: ['ALANISTATISTIKLERI', 'ISTATISTIKPANELI', 'FIELDSTATS'],
      description:
        'Alt panelin İstatistik sekmesi: bir katmanın bütün alanlarının dolu, boş ve benzersiz değer sayıları, en azı, en çoğu, ortalaması ve toplamı; seçilen alanın çeyrekleri, ortancası, en sık değeri ve değerlerinin sayıları. Değere tıklamak nesnelerini seçer.',
      run: (args) => openStatistics(ctx, layerArg(args)),
    },
    {
      id: 'data.chart',
      title: 'Grafik',
      category: V,
      icon: 'chart',
      aliases: ['GRAFIK', 'GRAFİK', 'CHART'],
      description:
        'Alt panelin Grafik sekmesi: bir katmanın ya da bir işlem tablosunun çubuk, pasta, histogram, kutu, saçılım ve çizgi grafiği. Grafiğe tıklamak nesneleri seçer, seçim grafikte vurgulanır; Tablo görünümü, Panoya kopyala, PNG ve SVG.',
      run: (args) => openChart(ctx, { layer: layerArg(args) }),
    },
    ...CHART_KINDS.map((k) => ({
      id: k.command,
      title: k.title,
      category: V,
      icon: k.icon,
      aliases: k.aliases,
      description: k.description,
      run: (args: unknown) => openChart(ctx, { layer: layerArg(args), kind: k.kind }),
    })),
  ]);
}
