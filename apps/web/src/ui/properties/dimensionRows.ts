import type { AppContext } from '../../app/context';
import type { DimensionEntity } from '../../model/entities';
import { dist } from '../../model/geometry';
import { layoutDimension } from '../../model/geom/dimension';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometries } from './write';

/**
 * A dimension's rows of docs/adr/0147 §7 in Öznitelikler, for one dimension or the dimensions of a selection: Zemin;
 * an ordinate's Koordinat (Y or X); a slope's two elevations; an arc length's radius and angle, shown only. Each row
 * is the dimensions' common value, or “Çeşitli”; a change writes them in one step “Değiştir”, those that already have
 * the value left out. On a locked layer the rows only show. The desktop's `properties::dimension_rows` are the same.
 */

const MIXED = 'Çeşitli';

/** The value every dimension has, or `MIXED` when they differ. */
function common<T>(dims: readonly DimensionEntity[], of: (d: DimensionEntity) => T): T | typeof MIXED {
  const first = of(dims[0]);
  return dims.every((d) => of(d) === first) ? first : MIXED;
}

const onOff = (on: boolean) => (on ? 'Açık' : 'Kapalı');
const axisName = (angle: number | undefined) => ((angle ?? 0) === 0 ? 'Y' : 'X');

export function dimensionRows(ctx: AppContext, dims: readonly DimensionEntity[], locked: boolean): PropRow[] {
  if (!dims.length) return [];
  const f = ctx.format;
  /** Each dimension's patch, or none when it already has the value: one step for all. */
  const write = (patch: (d: DimensionEntity) => Record<string, unknown> | null) =>
    setGeometries(
      ctx,
      dims.flatMap((d) => {
        const p = patch(d);
        return p ? [{ e: d, patch: p }] : [];
      }),
    );
  const select = <T,>(value: T | typeof MIXED, text: (v: T) => string, choices: readonly T[], take: (v: T) => void): PropRow['editor'] =>
    locked
      ? undefined
      : {
          type: 'select',
          display: () => ({ text: value === MIXED ? MIXED : text(value) }),
          items: () => choices.map((v): MenuItem => ({ label: text(v), radio: true, checked: value === v, run: () => take(v) })),
        };
  const mask = common(dims, (d) => d.mask === true);
  const rows: PropRow[] = [
    {
      label: 'Zemin',
      value: mask === MIXED ? MIXED : onOff(mask),
      editor: select(mask, onOff, [true, false], (on) => write((d) => ((d.mask === true) === on ? null : { mask: on || undefined }))),
    },
  ];
  // An ordinate's axis: its Y (0) or its X (90).
  if (dims.every((d) => d.style === 'ordinate')) {
    const axis = common(dims, (d) => axisName(d.angle));
    rows.push({
      label: 'Koordinat',
      value: axis === MIXED ? MIXED : axis,
      editor: select(axis, (v) => v, ['Y', 'X'], (v) => write((d) => (axisName(d.angle) === v ? null : { angle: v === 'Y' ? 0 : 90 }))),
    });
  }
  // A slope's two elevations, metres.
  if (dims.every((d) => d.style === 'slope')) {
    for (const [label, key] of [
      ['Birinci kot', 'za'],
      ['İkinci kot', 'zb'],
    ] as const) {
      const z = common(dims, (d) => d[key] ?? NaN);
      rows.push({
        label,
        value: z === MIXED ? MIXED : f.length(z, false),
        numeric: z !== MIXED,
        unit: f.lengthUnitLabel,
        editor: locked
          ? undefined
          : {
              type: 'number',
              commit: (v: string) => {
                // Typed in the project's unit (docs/adr/0165 §2).
                const x = f.toMetres(parseFloat(v.replace(',', '.')));
                if (Number.isFinite(x)) write((d) => (d[key] === x ? null : { [key]: x }));
              },
            },
      });
    }
  }
  // An arc length's radius and the arc's angle, shown only.
  if (dims.every((d) => d.style === 'arcLength')) {
    const radius = common(dims, (d) => (d.c ? dist(d.a, d.c) : NaN));
    // The layout's value is the arc's length: its angle is that over its radius.
    const sweep = common(dims, (d) => {
      const l = layoutDimension(d);
      return l && d.c ? l.value / dist(d.a, d.c) : NaN;
    });
    rows.push(
      { label: 'Yarıçap', value: radius === MIXED ? MIXED : f.length(radius, false), numeric: radius !== MIXED, unit: f.lengthUnitLabel },
      { label: 'Açı', value: sweep === MIXED ? MIXED : f.angle(sweep, false), numeric: sweep !== MIXED, unit: f.angleUnitLabel },
    );
  }
  return rows;
}
