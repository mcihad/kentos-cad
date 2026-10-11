import type { Entity, EntityKind, NewEntity } from '../../../model/entities';
import type { Route, RoutePart } from '../../../model/ops/lrs';
import type { DefaultsContext, FeatureSet, FileValue } from '../../types';
import { outputStyle } from '../geometry/shared';
import { sourceTable } from '../joinSource';
import { lengthText } from '../proximity/shared';

/**
 * What the Doğrusal referans tools share (docs/adr/0224 §5; the desktop's `builtin/lrs/mod.rs`): a route of an object,
 * the routes of a layer by their id, the number rule of a table's cells (docs/adr/0180, the core's
 * `ops::cogo::read_number`), how a cell that gives no number is said, how numbers are written (the project's length
 * decimals, the display rule) and the tools' output layers. The routes' rules are the core's (`model/ops/lrs`).
 */

/** What a route is: a line or a polyline. */
export const ROUTE_KINDS: readonly EntityKind[] = ['line', 'polyline'];
/** The scopes the tools offer, a layer first. */
export const LRS_SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** An object as a route: a line's two ends and their measures, a polyline's own part and its parts; none of another kind. */
export function routeOf(e: Entity): Route | null {
  if (e.kind === 'line') return { parts: [{ pts: [e.a, e.b], ms: [e.ma ?? null, e.mb ?? null] }] };
  if (e.kind !== 'polyline') return null;
  const part = (q: { pts: RoutePart['pts']; bulges?: number[]; ms?: (number | null)[] }): RoutePart => ({
    pts: q.pts,
    ...(q.bulges && { bulges: q.bulges }),
    ms: q.ms ?? q.pts.map(() => null),
  });
  return { parts: [part(e), ...(e.parts ?? []).map(part)] };
}

/** The routes of objects by their id: the id field's value, trimmed (an empty one no route); of several objects of an id the first. */
export interface Routes {
  readonly ids: string[];
  readonly routes: Route[];
  readonly objects: Entity[];
  readonly index: Map<string, number>;
  /** Objects of an id that came before. */
  readonly again: number;
}

export function routesById(objects: readonly Entity[], field: string): Routes {
  const out: Routes = { ids: [], routes: [], objects: [], index: new Map(), again: 0 };
  let again = 0;
  for (const e of objects) {
    const route = routeOf(e);
    const id = (e.attrs[field] ?? '').trim();
    if (!route || !id) continue;
    if (out.index.has(id)) {
      again++;
      continue;
    }
    out.index.set(id, out.ids.length);
    out.ids.push(id);
    out.routes.push(route);
    out.objects.push(e);
  }
  return { ...out, again };
}

/** The warning of ids that came again. */
export const againText = (again: number): string => `${again} rota kimliği birden çok kez var; ilki kullanıldı.`;

/** A cell's number (docs/adr/0180): a sign, digits and one decimal point or comma; null otherwise. */
export function readMeasure(text: string): number | null {
  const t = text.trim();
  const body = t.startsWith('+') || t.startsWith('-') ? t.slice(1) : t;
  let digits = 0;
  let seps = 0;
  for (const c of body) {
    if (c >= '0' && c <= '9') digits++;
    else if (c === '.' || c === ',') seps++;
    else return null;
  }
  if (digits === 0 || seps > 1) return null;
  return Number(t.replace(/^\+/, '').replace(',', '.'));
}

/** Why a cell gives no number. */
export const cellReason = (field: string, cell: string): string => (cell.trim() ? `“${field}” sayı değil: ${cell.trim()}` : `“${field}” boş`);

/** A measure, a distance written: the project's length decimals, the display rule. */
export const measureText = (units: DefaultsContext, v: number): string => lengthText(units, v);

/** An output layer: a new one of this name and colour by default. */
export const outputLayer = (name: string, color: string) =>
  ({ name: 'layer', label: 'Çıktı katmanı', type: 'layer', default: { newName: name }, newLayerStyle: outputStyle(color), description: 'Bu adda katman yoksa oluşturulur.' }) as const;

/**
 * Where an event table's rows come from: a file, or the objects' attributes; the parameters' names given (`sourceKind`,
 * `file`, `source`; `aSourceKind`, `aFile`, `aSource` …).
 */
export const sourceParams = <K extends string, F extends string, S extends string>(kind: K, file: F, source: S, label: string) =>
  [
    {
      name: kind,
      label: `${label} kaynağı`,
      type: 'enum',
      options: [
        { value: 'file', label: 'Dosya', hint: 'CSV, TXT, Excel ya da ODS dosyası' },
        { value: 'layer', label: 'Katman', hint: 'Nesnelerin öznitelikleri satırdır' },
      ],
      default: 'file',
    },
    {
      name: file,
      label: `${label} dosyası`,
      type: 'file',
      accept: ['.csv', '.txt', '.xlsx', '.ods'],
      visibleWhen: (v: Record<string, unknown>) => v[kind] !== 'layer',
      description: 'İlk satırı sütun adları olan CSV, TXT, Excel (.xlsx) ya da ODS dosyası; her satır bir olay.',
    },
    {
      name: source,
      label: `${label} nesneleri`,
      type: 'features',
      scopes: ['layer', 'all', 'visible', 'selection'],
      visibleWhen: (v: Record<string, unknown>) => v[kind] === 'layer',
      description: 'Öznitelikleri satır olan nesneler; her nesne bir olay.',
    },
  ] as const;

/** A table's rows: its column names, its rows and each row's label (a file's line, the names' line 1; an object's #id). */
export interface EventTable {
  readonly header: string[];
  readonly rows: (readonly string[])[];
  readonly labels: string[];
}

export function eventTable(kind: unknown, file: FileValue | null, source: FeatureSet | null): EventTable {
  const objects = kind === 'layer' ? (source?.entities ?? []) : null;
  const { header, rows } = sourceTable(objects, kind === 'layer' ? null : file);
  const labels = objects ? objects.map((e) => `#${e.id}`) : rows.map((_, i) => String(i + 2));
  return { header: header.map((h) => h.trim()), rows, labels };
}

/** A route's parts as a polyline on `layerId` with `attrs` (a part's bulges only when one bends). */
export function polylineOf(parts: readonly RoutePart[], layerId: string, attrs: Record<string, string>): NewEntity {
  const written = parts.map((q) => ({ pts: q.pts, ...(q.bulges?.some((b) => b !== 0) && { bulges: q.bulges }), ms: q.ms }));
  const [first, ...rest] = written;
  return { kind: 'polyline', layerId, attrs, ...first, ...(rest.length && { parts: rest }) } as NewEntity;
}
