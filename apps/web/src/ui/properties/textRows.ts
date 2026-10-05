import type { AppContext } from '../../app/context';
import { ENTITY_KIND_LABEL, TEXT_ALIGN_ROWS, textAlignName, type TextAlign, type TextEntity } from '../../model/entities';
import { textRealign } from '../../model/textEdit';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometries, setProperties, uidsOf } from './write';
import { fixed } from '../../core/displayNumber';

/**
 * A text's Hiza, Genişlik çarpanı and Zemin rows in Öznitelikler (docs/adr/0145 §6), for one text or the texts of a
 * selection: their common value, or “Çeşitli”. A new alignment keeps each text where it is (its point moves to that
 * alignment's point of its box, the core's `textRealign`); a new width factor keeps each text's point; the texts are
 * written in one step “Değiştir”, those that already have the value left out. On a locked layer the rows only show.
 * A linked text's Bağlı nesne row follows (docs/adr/0175 §4). The desktop's `properties::text_rows` and `link_rows`
 * are the same.
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
      value: factor === MIXED ? MIXED : String(+fixed(factor, 4)),
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
    ...linkRows(ctx, texts, locked),
  ];
}

/**
 * A linked text's Bağlı nesne row (docs/adr/0175 §4): for one text its object's kind and label, with Nesneyi seç and
 * Bağı kopar (`cad.entities.set`, `unlink`); for several, how many are linked, with Bağı kopar for them all. None when
 * no text of them is linked; on a locked layer only Nesneyi seç.
 */
function linkRows(ctx: AppContext, texts: readonly TextEntity[], locked: boolean): PropRow[] {
  const linked = texts.filter((t) => t.labelOf !== undefined);
  if (!linked.length) return [];
  const object = texts.length === 1 ? ctx.doc.byUid(linked[0].labelOf!) : undefined;
  const value =
    texts.length > 1 ? `${linked.length} yazı bağlı` : !object ? 'Çizimde yok' : object.label ? `${ENTITY_KIND_LABEL[object.kind]} “${object.label}”` : ENTITY_KIND_LABEL[object.kind];
  const items: MenuItem[] = [
    ...(object ? [{ label: 'Nesneyi seç', run: () => ctx.selection.set([object.id]) }] : []),
    ...(locked ? [] : [{ label: 'Bağı kopar', run: () => void setProperties(ctx, { uids: uidsOf(ctx, linked.map((t) => t.id)), unlink: true, operation: 'unlink' }) }]),
  ];
  return [{ label: 'Bağlı nesne', value, editor: items.length ? { type: 'select', display: () => ({ text: value }), items: () => items } : undefined }];
}
