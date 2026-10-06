import type { AppContext } from '../../app/context';
import { fixed } from '../../core/displayNumber';
import type { GradientShape, HatchEntity, HatchPattern } from '../../model/entities';
import { HATCH_COLOURS } from '../../model/ops/hatchPatterns';
import { choiceList, choiceOf, rechosenPattern } from '../../tools/hatchOptions';
import { colorSwatch } from '../layers/swatch';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometry, setProperties, uidsOf } from './write';

/**
 * A hatch's rows in Öznitelikler (docs/adr/0186 §7): Desen (every choice with its icon, its Ölçek and Açı carried
 * over), Açı, Ölçek (a pattern's, on the paper), Aralık (user lines'), İkinci renk, Degrade biçimi and Ters (a
 * gradient's), İlişkili with İlişkiyi kopar (`cad.entities.set`'s `unlink`, the step “Bağı kopar”). Each change is one
 * step through `cad.entities.edit`'s properties; on a locked layer the rows only show. The desktop's
 * `properties::hatch_rows` are the same.
 */

const SHAPES: readonly (readonly [GradientShape, string, string])[] = [
  ['linear', 'Doğrusal', 'hatchGradientLinear'],
  ['cylinder', 'Silindir', 'hatchGradientCylinder'],
  ['spherical', 'Küre', 'hatchGradientSpherical'],
];

export function hatchRows(ctx: AppContext, e: HatchEntity, locked: boolean): PropRow[] {
  const f = ctx.format;
  const p = e.pattern;
  const plot = ctx.doc.settings.plotScale.value;
  const list = choiceList();
  const now = choiceOf(p);
  const name = now !== null ? list[now].name : (p.name ?? 'Desen');
  const icon = now !== null ? list[now].icon : undefined;
  const setPattern = (pattern: HatchPattern) => setGeometry(ctx, e, { pattern });
  const numEdit = (commit: (x: number) => void) =>
    locked
      ? undefined
      : ({
          type: 'number',
          commit: (t: string) => {
            const x = parseFloat(t.replace(',', '.'));
            if (Number.isFinite(x)) commit(x);
          },
        } as const);
  const rows: PropRow[] = [
    {
      label: 'Desen',
      value: name,
      editor: locked
        ? undefined
        : {
            type: 'select',
            display: () => ({ text: name, icon }),
            items: () => list.map((c, k) => ({ label: c.label, icon: c.icon, preview: c.preview, radio: true, checked: k === now, run: () => setPattern(rechosenPattern(p, k, plot)) })),
          },
    },
    { label: 'Açı', value: fixed(p.angle, 2), numeric: true, unit: '°', editor: numEdit((x) => setPattern({ ...p, angle: x })) },
  ];
  if (p.type === 'pattern') {
    rows.push({ label: 'Ölçek', value: fixed(((p.scale ?? 0) * 1000) / plot, 3), numeric: true, editor: numEdit((x) => x > 0 && setPattern({ ...p, scale: (x * plot) / 1000 })) });
  } else if (p.type === 'lines' || p.type === 'cross') {
    rows.push({ label: 'Aralık', value: f.length(p.spacing, false), numeric: true, unit: f.lengthUnitLabel, editor: numEdit((x) => x > 0 && setPattern({ ...p, spacing: f.toMetres(x) })) });
  }
  const g = p.gradient;
  if (g) {
    const shape = SHAPES.find((s) => s[0] === g.shape) ?? SHAPES[0];
    const yesNo = (b: boolean) => (b ? 'Evet' : 'Hayır');
    rows.push(
      {
        label: 'İkinci renk',
        value: g.color2,
        editor: locked
          ? undefined
          : {
              type: 'select',
              display: () => ({ text: g.color2, swatch: colorSwatch(g.color2, ctx.view.palette) }),
              items: () => HATCH_COLOURS.map(([n, hex]) => ({ label: n, swatch: colorSwatch(hex, ctx.view.palette), radio: true, checked: g.color2 === hex, run: () => setPattern({ ...p, gradient: { ...g, color2: hex } }) })),
            },
      },
      {
        label: 'Degrade biçimi',
        value: shape[1],
        editor: locked
          ? undefined
          : {
              type: 'select',
              display: () => ({ text: shape[1], icon: shape[2] }),
              items: () => SHAPES.map(([s, label, i]) => ({ label, icon: i, radio: true, checked: s === g.shape, run: () => setPattern({ ...p, gradient: { ...g, shape: s } }) })),
            },
      },
      {
        label: 'Ters',
        value: yesNo(g.inverted === true),
        editor: locked
          ? undefined
          : {
              type: 'select',
              display: () => ({ text: yesNo(g.inverted === true) }),
              items: () =>
                [true, false].map((b) => ({
                  label: yesNo(b),
                  radio: true,
                  checked: b === (g.inverted === true),
                  run: () => {
                    const next = { ...g };
                    if (b) next.inverted = true;
                    else delete next.inverted;
                    setPattern({ ...p, gradient: next });
                  },
                })),
            },
      },
    );
  }
  const a = e.assoc;
  const tie = a ? `Evet (${1 + (a.islands?.length ?? 0) + (a.cutouts?.length ?? 0)} nesne)` : 'Hayır';
  rows.push({
    label: 'İlişkili',
    value: tie,
    editor:
      a && !locked
        ? {
            type: 'select',
            display: () => ({ text: tie, icon: 'hatchAssoc' }),
            items: () => [{ label: 'İlişkiyi kopar', run: () => void setProperties(ctx, { uids: uidsOf(ctx, [e.id]), unlink: true, operation: 'unlink' }) }],
          }
        : undefined,
  });
  return rows;
}
