import type { AppContext } from '../../app/context';
import { TEXT_ALIGN_ROWS, textAlignName, type TextAlign, type TextEntity } from '../../model/entities';
import { textRealign } from '../../model/textEdit';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometries } from './write';

/**
 * A text's Hiza, Genişlik çarpanı and Zemin rows in Öznitelikler (docs/adr/0145 §6), for one text or the texts of a
 * selection: their common value, or “Çeşitli”. A new alignment keeps each text where it is (its point moves to that
 * alignment's point of its box, the core's `textRealign`); a new width factor keeps each text's point; the texts are
 * written in one step “Değiştir”, those that already have the value left out. On a locked layer the rows only show.
 * The desktop's `properties::text_rows` are the same.
 */

const MIXED = 'Çeşitli';

/** “sol üst” → “Sol üst”. */
const capital = (s: string) => s.charAt(0).toLocaleUpperCase('tr-TR') + s.slice(1);

/** An alignment's icon (`ui/icons.ts`): `textAlign` and its name, the left of the baseline too. */
export const textAlignIcon = (a: TextAlign | null): string => {
  const name = a ?? 'baselineLeft';
  return `textAlign${name.charAt(0).toUpperCase()}${name.slice(1)}`;
};

/** The value every text has, or `MIXED` when they differ. */
function common<T>(texts: readonly TextEntity[], of: (t: TextEntity) => T): T | typeof MIXED {
  const first = of(texts[0]);
  return texts.every((t) => of(t) === first) ? first : MIXED;
}

export function textRows(ctx: AppContext, texts: readonly TextEntity[], locked: boolean): PropRow[] {
  if (!texts.length) return [];
  const align = common(texts, (t) => t.align ?? null);
  const factor = common(texts, (t) => t.widthFactor ?? 1);
  const mask = common(texts, (t) => t.mask === true);
  const font = ctx.doc.settings.drawingFont.value;
  const alignText = align === MIXED ? MIXED : capital(textAlignName(align));
  const maskText = (on: boolean) => (on ? 'Açık' : 'Kapalı');

  const realign = (to: TextAlign | null) =>
    setGeometries(
      ctx,
      texts
        .filter((t) => (t.align ?? null) !== to)
        .map((t) => ({ e: t, patch: { p: textRealign({ ...t, font }, to), align: to ?? undefined } })),
    );
  const alignItems = (): MenuItem[] =>
    TEXT_ALIGN_ROWS.flat().map((a) => ({ label: capital(textAlignName(a)), icon: textAlignIcon(a), radio: true, checked: align === a, run: () => realign(a) }));
  const setMask = (on: boolean) =>
    setGeometries(
      ctx,
      texts.filter((t) => (t.mask === true) !== on).map((t) => ({ e: t, patch: { mask: on || undefined } })),
    );

  return [
    {
      label: 'Hiza',
      value: alignText,
      editor: locked ? undefined : { type: 'select', display: () => ({ text: alignText, ...(align !== MIXED && { icon: textAlignIcon(align) }) }), items: alignItems },
    },
    {
      label: 'Genişlik çarpanı',
      value: factor === MIXED ? MIXED : String(+factor.toFixed(4)),
      numeric: factor !== MIXED,
      editor: locked
        ? undefined
        : {
            type: 'number',
            // Out of its range the command refuses and says why (invalid_width_factor); 1 is written as no field.
            commit: (v: string) => {
              const x = parseFloat(v.replace(',', '.'));
              if (!Number.isFinite(x)) return;
              setGeometries(
                ctx,
                texts.filter((t) => (t.widthFactor ?? 1) !== x).map((t) => ({ e: t, patch: { widthFactor: x } })),
              );
            },
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
            items: () => [true, false].map((on): MenuItem => ({ label: maskText(on), radio: true, checked: mask === on, run: () => setMask(on) })),
          },
    },
  ];
}
