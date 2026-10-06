/**
 * The project's text and dimension styles saved (docs/adr/0183 §1, §5): the table becomes the project's, then the
 * objects follow in one undo step (Yazı stili, Ölçü stili): those of a style whose values changed take the new values
 * where they still held the old ones (`followTextStyle`, `followDimensionStyle`), those of a style deleted lose their
 * link and keep their look. Objects on locked layers are left as they are and counted. The desktop's twin is
 * `apps/desktop/src/style_tables.rs`.
 */
import type { EntityEdit } from '../contracts/generated/EntityEdit';
import type { EntityGeometry as EditGeometry } from '../contracts/generated/EntityGeometry';
import {
  FACE_FIELDS,
  followDimensionStyle,
  followTextStyle,
  LOOK_FIELDS,
  lookOfDimension,
  faceOfText,
  type DimensionStyleDef,
  type TextStyleDef,
} from '../model/annotationStyles';
import type { DimensionEntity, Entity, TextEntity } from '../model/entities';
import { geometryOf } from '../product/entitiesEdit';
import { writeEdit } from '../tools/editCommand';
import type { AppContext } from './context';

/** How many texts (dimensions) follow each style, by its id. */
export function styleUsage(ctx: AppContext, kind: 'text' | 'dimension'): Map<string, number> {
  const out = new Map<string, number>();
  for (const e of ctx.doc.all()) {
    const id = e.kind === 'text' && kind === 'text' ? e.textStyle : e.kind === 'dimension' && kind === 'dimension' ? e.dimStyle : undefined;
    if (id !== undefined) out.set(id, (out.get(id) ?? 0) + 1);
  }
  return out;
}

/** Two styles with the same values, whatever their fields' order. */
const sameStyle = (a: object, b: object): boolean => {
  const sorted = (o: object) => JSON.stringify(Object.entries(o).filter(([, v]) => v !== undefined).sort(([x], [y]) => (x < y ? -1 : x > y ? 1 : 0)));
  return sorted(a) === sorted(b);
};

/** What saving did: the objects changed, and those left on locked layers. */
export interface StylesSaved {
  changed: number;
  locked: number;
}

/** `e`'s geometry with `fields` taken out and `patch` written over them. */
function patched(e: Entity, fields: readonly string[], patch: Record<string, unknown>): EditGeometry {
  const g = geometryOf(e as unknown as EditGeometry);
  for (const k of fields) delete g[k];
  return { ...g, ...patch } as unknown as EditGeometry;
}

/** Writes the changes as one step named `operation`; the objects on locked layers are counted, not written. */
function follow(ctx: AppContext, operation: 'textStyle' | 'dimensionStyle', updates: { e: Entity; geometry: EditGeometry }[]): StylesSaved {
  const layers = ctx.doc.layers;
  const open = updates.filter((u) => !layers.isLocked(u.e.layerId));
  const changes: EntityEdit[] = open.map((u) => ({ kind: 'update', uid: ctx.doc.uidOf(u.e.id) ?? '', geometry: u.geometry }));
  const written = changes.length ? writeEdit(ctx, operation, changes) : null;
  return { changed: written ? changes.length : 0, locked: updates.length - open.length };
}

/** Saves the text styles: the table, then the texts that follow them (Yazı stili). */
export function saveTextStyles(ctx: AppContext, next: readonly TextStyleDef[]): StylesSaved {
  const settings = ctx.doc.settings;
  const before = new Map(settings.textStyles.value.map((s) => [s.id, s]));
  const after = new Map(next.map((s) => [s.id, s]));
  settings.assign({ textStyles: [...next] });
  const scale = settings.plotScale.value;
  const updates: { e: Entity; geometry: EditGeometry }[] = [];
  for (const e of ctx.doc.all()) {
    if (e.kind !== 'text' || e.textStyle === undefined) continue;
    const old = before.get(e.textStyle);
    if (!old) continue;
    const now = after.get(e.textStyle);
    const t = e as TextEntity;
    if (!now) {
      // A style deleted: the text keeps its look, without the link.
      updates.push({ e, geometry: patched(e, ['textStyle'], {}) });
      continue;
    }
    if (sameStyle(old, now)) continue;
    const look = followTextStyle(old, now, { ...faceOfText(t), ...(t.widthFactor !== undefined && { widthFactor: t.widthFactor }), height: t.height }, scale);
    updates.push({ e, geometry: patched(e, [...FACE_FIELDS, 'widthFactor', 'height'], look) });
  }
  return follow(ctx, 'textStyle', updates);
}

/** Saves the dimension styles: the table, then the dimensions that follow them (Ölçü stili). */
export function saveDimensionStyles(ctx: AppContext, next: readonly DimensionStyleDef[]): StylesSaved {
  const settings = ctx.doc.settings;
  const before = new Map(settings.dimensionStyles.value.map((s) => [s.id, s]));
  const after = new Map(next.map((s) => [s.id, s]));
  settings.assign({ dimensionStyles: [...next] });
  const scale = settings.plotScale.value;
  const updates: { e: Entity; geometry: EditGeometry }[] = [];
  for (const e of ctx.doc.all()) {
    if (e.kind !== 'dimension' || e.dimStyle === undefined) continue;
    const old = before.get(e.dimStyle);
    if (!old) continue;
    const now = after.get(e.dimStyle);
    const d = e as DimensionEntity;
    if (!now) {
      updates.push({ e, geometry: patched(e, ['dimStyle'], {}) });
      continue;
    }
    if (sameStyle(old, now)) continue;
    const { look, height } = followDimensionStyle(old, now, lookOfDimension(d), d.height, scale);
    updates.push({ e, geometry: patched(e, [...LOOK_FIELDS, 'height'], { ...look, height }) });
  }
  return follow(ctx, 'dimensionStyle', updates);
}
