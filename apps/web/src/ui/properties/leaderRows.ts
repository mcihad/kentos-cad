import type { AppContext } from '../../app/context';
import type { LeaderArrow, LeaderEntity } from '../../model/entities';
import { LEADER_ARROW_ROWS } from '../../tools/leaderTool';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometries } from './write';
import { fixed } from '../../core/displayNumber';

/**
 * A leader's Not, Yükseklik, Dönüş, Ok and Zemin rows in Öznitelikler (docs/adr/0146 §7), for one leader or the
 * leaders of a selection: their common value, or “Çeşitli”. The leaders are written in one step “Değiştir”, those that
 * already have the value left out; an emptied note takes the note away (the arrow alone), a height not over 0 is not
 * taken. On a locked layer the rows only show. The desktop's `properties::leader_rows` are the same.
 */

const MIXED = 'Çeşitli';

/** The value every leader has, or `MIXED` when they differ. */
function common<T>(leaders: readonly LeaderEntity[], of: (l: LeaderEntity) => T): T | typeof MIXED {
  const first = of(leaders[0]);
  return leaders.every((l) => of(l) === first) ? first : MIXED;
}

/** An arrowhead's menu label and icon (Ok's menu, the tool's). */
const arrowRow = (a: LeaderArrow | null) => LEADER_ARROW_ROWS.find((r) => r[0] === a) ?? LEADER_ARROW_ROWS[0];

export function leaderRows(ctx: AppContext, leaders: readonly LeaderEntity[], locked: boolean): PropRow[] {
  if (!leaders.length) return [];
  const f = ctx.format;
  const note = common(leaders, (l) => l.text ?? '');
  const height = common(leaders, (l) => l.height);
  const rotation = common(leaders, (l) => l.rotation);
  const arrow = common(leaders, (l) => l.arrow ?? null);
  const mask = common(leaders, (l) => l.mask === true);
  const maskText = (on: boolean) => (on ? 'Açık' : 'Kapalı');
  /** Each leader's patch, or none when it already has the value: one step for all. */
  const write = (patch: (l: LeaderEntity) => Record<string, unknown> | null) =>
    setGeometries(
      ctx,
      leaders.flatMap((l) => {
        const p = patch(l);
        return p ? [{ e: l, patch: p }] : [];
      }),
    );
  const number = (take: (x: number) => void) =>
    locked
      ? undefined
      : ({
          type: 'number',
          commit: (v: string) => {
            const x = parseFloat(v.replace(',', '.'));
            if (Number.isFinite(x)) take(x);
          },
        } as const);
  const arrowText = arrow === MIXED ? MIXED : arrowRow(arrow)[2];
  return [
    {
      label: 'Not',
      value: note === MIXED ? MIXED : note,
      // Trimmed, as the field stores it; emptied, the leader is the arrow alone.
      editor: locked
        ? undefined
        : {
            type: 'text',
            commit: (v: string) => {
              const t = v.trim();
              write((l) => ((l.text ?? '') === t ? null : { text: t || undefined }));
            },
          },
    },
    {
      label: 'Yükseklik',
      value: height === MIXED ? MIXED : f.length(height, false),
      numeric: height !== MIXED,
      unit: 'm',
      editor: number((x) => x > 0 && write((l) => (l.height === x ? null : { height: x }))),
    },
    {
      label: 'Dönüş',
      value: rotation === MIXED ? MIXED : fixed(rotation, 2),
      numeric: rotation !== MIXED,
      unit: '°',
      editor: number((x) => {
        const turn = ((x % 360) + 360) % 360;
        write((l) => (l.rotation === turn ? null : { rotation: turn }));
      }),
    },
    {
      label: 'Ok',
      value: arrowText,
      editor: locked
        ? undefined
        : {
            type: 'select',
            display: () => ({ text: arrowText, ...(arrow !== MIXED && { icon: arrowRow(arrow)[3] }) }),
            items: (): MenuItem[] =>
              LEADER_ARROW_ROWS.map(([a, , label, icon]) => ({
                label,
                icon,
                radio: true,
                checked: arrow === a,
                run: () => write((l) => ((l.arrow ?? null) === a ? null : { arrow: a ?? undefined })),
              })),
          },
    },
    {
      label: 'Zemin',
      value: mask === MIXED ? MIXED : maskText(mask),
      editor: locked
        ? undefined
        : {
            type: 'select',
            display: () => ({ text: mask === MIXED ? MIXED : maskText(mask) }),
            items: () => [true, false].map((on): MenuItem => ({ label: maskText(on), radio: true, checked: mask === on, run: () => write((l) => ((l.mask === true) === on ? null : { mask: on || undefined })) })),
          },
    },
  ];
}
