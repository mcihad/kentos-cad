import type { Entity } from '../../model/entities';
import { checkValue } from '../../model/layerFields';
import type { RunContext } from '../types';

/**
 * How the query tools write a value into an object's attribute (docs/adr/0200 §3): through the object's layer's field
 * of that name, when there is one, as the field's canonical text (so a value already there is not written again);
 * none takes the attribute away. A value the field does not take is written as given, and the runner's check refuses
 * the run with the field's reason (processing/writeCheck.ts).
 */

/** The text a value becomes in this object's attribute. */
export function canonical(ctx: RunContext, e: Entity, name: string, value: string): string {
  const field = ctx.field(e.layerId, name);
  if (!field) return value;
  const r = checkValue(field, value);
  return 'value' in r ? r.value : value;
}

/** The object's attributes with `name` set to `value` (none: taken away); null when nothing changes. */
export function withAttr(ctx: RunContext, e: Entity, name: string, value: string | null): Record<string, string> | null {
  const has = Object.hasOwn(e.attrs, name);
  if (value === null) {
    if (!has) return null;
    const { [name]: _gone, ...rest } = e.attrs;
    return rest;
  }
  const text = canonical(ctx, e, name, value);
  if (has && e.attrs[name] === text) return null;
  return { ...e.attrs, [name]: text };
}

/** The scale the mean is rounded to for this object's attribute: its field's (whole numbers: 0), else the rule's own. */
export function meanScale(ctx: RunContext, e: Entity, name: string): number | null {
  const field = ctx.field(e.layerId, name);
  if (field?.kind === 'integer') return 0;
  if (field?.kind === 'decimal') return field.scale ?? null;
  return null;
}
