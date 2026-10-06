import type { AppContext } from '../../app/context';
import {
  applyDimensionStyle,
  applyTextStyle,
  FACE_FIELDS,
  faceOfText,
  LOOK_FIELDS,
  lookOfDimension,
  STANDARD_STYLE,
  type DimensionStyleDef,
  type TextStyleDef,
} from '../../model/annotationStyles';
import type { DimensionEntity, TableEntity, TextEntity } from '../../model/entities';
import { stylesShown } from '../../tools/styleOption';
import type { MenuItem } from '../widgets/PopupMenu';
import type { PropRow } from '../widgets/PropertyGrid';
import { setGeometries } from './write';

/**
 * Öznitelikler's Yazı stili and Ölçü stili rows (docs/adr/0183 §6), a CAD project's: the style the objects follow, by
 * name (a link to a style the project no longer has shows Standart), or “Çeşitli”; choosing one applies it to them in
 * one step “Değiştir” (`applyTextStyle`, `applyDimensionStyle`), those already in it left out. On a locked layer the
 * row only shows. The desktop's `properties::style_rows` are the same.
 */

const MIXED = 'Çeşitli';

/** Each field of `fields` set to `look`'s value, or taken away. */
const patchOf = (fields: readonly string[], look: Record<string, unknown>): Record<string, unknown> => Object.fromEntries(fields.map((k) => [k, look[k]]));

function row<S extends { id: string; name: string }>(
  label: string,
  styles: readonly S[],
  ids: readonly (string | undefined)[],
  locked: boolean,
  take: (style: S | null) => void,
): PropRow {
  const known = (id: string | undefined) => styles.find((s) => s.id === id) ?? null;
  const names = ids.map((id) => known(id)?.name ?? STANDARD_STYLE);
  const value = names.every((n) => n === names[0]) ? names[0] : MIXED;
  const items = (): MenuItem[] => [
    { label: STANDARD_STYLE, radio: true, checked: value === STANDARD_STYLE, run: () => take(null) },
    ...styles.map((s) => ({ label: s.name, radio: true, checked: value === s.name, run: () => take(s) })),
  ];
  return { label, value, editor: locked ? undefined : { type: 'select', display: () => ({ text: value }), items } };
}

/**
 * A text's or a table's Yazı stili row (a table's cells are in its face, docs/adr/0184 §6); none outside a CAD project.
 * A table takes the style's face and fixed height, not its width factor.
 */
export function textStyleRows(ctx: AppContext, texts: readonly (TextEntity | TableEntity)[], locked: boolean): PropRow[] {
  if (!texts.length || !stylesShown(ctx)) return [];
  const settings = ctx.doc.settings;
  return [
    row<TextStyleDef>('Yazı stili', settings.textStyles.value, texts.map((t) => t.textStyle), locked, (style) => {
      const scale = settings.plotScale.value;
      setGeometries(
        ctx,
        texts
          .filter((t) => (style ? t.textStyle !== style.id : t.textStyle !== undefined || Object.keys(faceOfText(t)).length > 0))
          .map((t) => {
            const width = t.kind === 'text' && t.widthFactor !== undefined ? { widthFactor: t.widthFactor } : {};
            const look = applyTextStyle(style, { ...faceOfText(t), ...width, height: t.height }, scale);
            const fields = t.kind === 'text' ? [...FACE_FIELDS, 'widthFactor', 'height'] : [...FACE_FIELDS, 'height'];
            return { e: t, patch: patchOf(fields, look as Record<string, unknown>) };
          }),
      );
    }),
  ];
}

/** A dimension's Ölçü stili row; none outside a CAD project. */
export function dimensionStyleRows(ctx: AppContext, dims: readonly DimensionEntity[], locked: boolean): PropRow[] {
  if (!dims.length || !stylesShown(ctx)) return [];
  const settings = ctx.doc.settings;
  return [
    row<DimensionStyleDef>('Ölçü stili', settings.dimensionStyles.value, dims.map((d) => d.dimStyle), locked, (style) => {
      const { look, height } = applyDimensionStyle(style, settings.plotScale.value);
      setGeometries(
        ctx,
        dims
          .filter((d) => (style ? d.dimStyle !== style.id : Object.keys(lookOfDimension(d)).length > 0))
          .map((d) => ({ e: d, patch: { ...patchOf(LOOK_FIELDS, look as Record<string, unknown>), height } })),
      );
    }),
  ];
}
