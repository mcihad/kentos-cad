import { entityGeometry, type Entity, type EntityGeometry, type NewEntity } from '../../../model/entities';
import type { StatsRun } from '../../../model/ops/spatialStats';
import type { LayerRenderer } from '../../../model/style';
import type { Feedback, RunContext, RunResult } from '../../types';

/**
 * What the spatial statistics tools share (docs/adr/0238; the desktop's `builtin/stats/mod.rs`): the objects as the
 * core reads them, the refusal on a geographic project, and a run's answer turned into the tool's result: its notes,
 * the new objects or the objects' copies on the output layer, the table and the numbers.
 */

/** Why a geographic project is refused (§2). */
export const GEOGRAPHIC = 'Mekânsal istatistik projeksiyonlu koordinat ister: projenin sistemi coğrafi.';

/** Whether the project's coordinates are degrees. */
export const geographic = (ctx: RunContext): boolean => ctx.crs?.system.kind === 'geographic';

/** The objects' geometry only: the core reads nothing else. */
export const shapesOf = (list: readonly Entity[]): EntityGeometry[] => list.map(entityGeometry);

/** An attribute's text, or null. */
export const attrOf = (e: Entity, name: string): string | null => (name && Object.hasOwn(e.attrs, name) ? e.attrs[name] : null);

/** A copy of an object on `layerId`: its geometry (elevations too) and attributes with the run's added, in the run's colour. */
function copyOf(e: Entity, layerId: string, added: readonly [string, string][], color: string): NewEntity {
  const { labelPins: _pins, ...geometry } = entityGeometry(e) as EntityGeometry & { labelPins?: unknown };
  const attrs: Record<string, string> = { ...e.attrs };
  for (const [k, v] of added) attrs[k] = v;
  return { ...geometry, layerId, attrs, color } as NewEntity;
}

/**
 * The tool's result from the core's answer: notes said, new objects (or the input's copies) on `layerId`, the table
 * and the numbers among the outputs (`count` with what was written when anything was).
 */
export function resultOf(run: StatsRun, feedback: Feedback, input: readonly Entity[], layerId: string | null): RunResult {
  for (const t of run.infos) feedback.info(t);
  for (const t of run.warnings) feedback.warn(t);
  const add: NewEntity[] = [];
  if (layerId !== null) {
    for (const o of run.objects) add.push({ ...o.shape, layerId, attrs: Object.fromEntries(o.attrs) } as NewEntity);
    for (const c of run.copies) add.push(copyOf(input[c.index], layerId, c.attrs, c.color));
  }
  const outputs: Record<string, unknown> = {};
  if (run.table) outputs.table = { columns: run.table.columns, rows: run.table.rows };
  for (const n of run.numbers) outputs[n.name] = n.value;
  if (layerId !== null) outputs.count = add.length;
  return { ...(add.length ? { changes: { add } } : {}), outputs, summary: run.summary } as RunResult;
}

/** A refusal from the core (its reason) as the tool's result. */
export function refusal(e: unknown): RunResult {
  return { refused: e instanceof Error ? e.message : String(e) };
}

/**
 * Sıcak noktalar' look (§9): a category of “Güven sınıfı” a class, hot to cold, in its colour (ColorBrewer's RdBu) as an
 * area, a line and a point; the legend lists them. The desktop's `hot_renderer`.
 */
const HOT_CLASSES: readonly (readonly [string, string, string])[] = [
  ['3', 'Sıcak nokta, %99 güven', '#B2182B'],
  ['2', 'Sıcak nokta, %95 güven', '#EF8A62'],
  ['1', 'Sıcak nokta, %90 güven', '#FDDBC7'],
  ['0', 'Anlamlı değil', '#D9D9D9'],
  ['-1', 'Soğuk nokta, %90 güven', '#D1E5F0'],
  ['-2', 'Soğuk nokta, %95 güven', '#67A9CF'],
  ['-3', 'Soğuk nokta, %99 güven', '#2166AC'],
];

export const HOT_RENDERER: LayerRenderer = {
  type: 'categorized',
  expr: '[Güven sınıfı]',
  categories: HOT_CLASSES.map(([value, label, color]) => ({
    value,
    label,
    symbols: {
      fill: {
        type: 'fill',
        layers: [
          { id: 'f', type: 'simpleFill', color },
          { id: 'o', type: 'simpleLine', color: '#FFFFFF', width: 0.13, unit: 'mm' },
        ],
      },
      line: { type: 'line', layers: [{ id: 'l', type: 'simpleLine', color, width: 0.6, unit: 'mm', cap: 'round', join: 'round' }] },
      marker: { type: 'marker', layers: [{ id: 'p', type: 'shape', shape: 'circle', size: 8, unit: 'px', fill: color, stroke: '#FFFFFF', strokeWidth: 0.8 }] },
    },
  })),
};

/** The clusters' colours by number, turning round, and the noise's (the core's `CLUSTER_COLORS`, `NOISE_COLOR`). */
const CLUSTER_COLORS = ['#1F77B4', '#FF7F0E', '#2CA02C', '#D62728', '#9467BD', '#8C564B', '#E377C2', '#17BECF', '#BCBD22', '#7F7F7F'] as const;
const NOISE_COLOR = '#BDBDBD';

const clusterSymbols = (color: string) =>
  ({
    fill: {
      type: 'fill',
      layers: [
        { id: 'f', type: 'simpleFill', color },
        { id: 'o', type: 'simpleLine', color: '#FFFFFF', width: 0.13, unit: 'mm' },
      ],
    },
    line: { type: 'line', layers: [{ id: 'l', type: 'simpleLine', color, width: 0.6, unit: 'mm', cap: 'round', join: 'round' }] },
    marker: { type: 'marker', layers: [{ id: 'p', type: 'shape', shape: 'circle', size: 7, unit: 'px', fill: color, stroke: '#FFFFFF', strokeWidth: 0.8 }] },
  }) as const;

/**
 * Kümeler' look (§10): the cluster's colour by `([Küme] - 1) % 10` (the core's colours turning round; JavaScript's
 * remainder keeps the sign, so the noise, 0, is −1), the noise grey; a filled point, an area, a line. The desktop's
 * `cluster_renderer`.
 */
export const CLUSTER_RENDERER: LayerRenderer = {
  type: 'categorized',
  expr: '([Küme] - 1) % 10',
  categories: [
    ...CLUSTER_COLORS.map((color, k) => ({ value: String(k), label: `Küme ${k + 1}, ${k + 11}, ${k + 21} …`, symbols: clusterSymbols(color) })),
    { value: '-1', label: 'Gürültü', symbols: clusterSymbols(NOISE_COLOR) },
  ],
};
