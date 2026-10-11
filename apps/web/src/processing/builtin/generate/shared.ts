import type { Entity } from '../../../model/entities';
import type { Vec2 } from '../../../model/geometry';
import type { Feedback } from '../../types';
import { hasElevation } from '../geometry/shared';

/**
 * What the Geometri üretimi tools share (docs/adr/0219; the desktop's `builtin/generate/mod.rs`): their scopes, the
 * points they take (each point of a multi-point object), the grouping of Sınırlayıcı geometri and İçbükey zarf, and
 * the notes about the inputs.
 */

/** The scopes the tools offer, a layer first. */
export const GENERATE_SCOPES = ['layer', 'selection', 'visible', 'all'] as const;

/** A site: a point of an object, its object's place in the input, its part (1, 2 … for a multi-point object) and elevation. */
export interface Site {
  readonly p: Vec2;
  readonly at: number;
  readonly part: number | null;
  readonly z: number | null;
}

/** The points of the input in order, each point of a multi-point object one. */
export function sitesOf(inputs: readonly Entity[]): Site[] {
  const out: Site[] = [];
  inputs.forEach((e, at) => {
    if (e.kind !== 'point') return;
    const many = (e.parts?.length ?? 0) > 0;
    out.push({ p: e.p, at, part: many ? 1 : null, z: e.z ?? null });
    e.parts?.forEach((q, k) => out.push({ p: q.p, at, part: k + 2, z: q.z ?? null }));
  });
  return out;
}

/** How objects are gathered (§4): each its own, all together, or by a field's value. */
export const groupParams = (fallback: 'each' | 'all') =>
  [
    {
      name: 'group',
      label: 'Gruplama',
      type: 'enum',
      options: [
        { value: 'each', label: 'Her nesne', hint: 'Her nesnenin kendisi' },
        { value: 'all', label: 'Hepsi', hint: 'Bütün nesneler birlikte' },
        { value: 'field', label: 'Alana göre', hint: 'Alanın değeri aynı olanlar birlikte' },
      ],
      default: fallback,
    },
    {
      name: 'field',
      label: 'Alan',
      type: 'field',
      of: 'input',
      optional: true,
      visibleWhen: (v: { group?: unknown }) => v.group === 'field',
      description: 'Değeri aynı olan nesneler bir grup olur; boş değer de bir gruptur.',
    },
  ] as const;

/** The groups: each object's number of group, how many, and each group's attributes (the object's; Nesne sayısı and the field's value). */
export function grouping(inputs: readonly Entity[], group: string, field: string) {
  const name = field.trim();
  if (group === 'each') return { groups: inputs.map((_, i) => i), count: inputs.length, attrs: inputs.map((e) => ({ ...e.attrs })) };
  if (group === 'field' && name) {
    const keys: string[] = [];
    const groups = inputs.map((e) => {
      const key = Object.hasOwn(e.attrs, name) ? e.attrs[name] : '';
      const k = keys.indexOf(key);
      return k < 0 ? keys.push(key) - 1 : k;
    });
    const attrs = keys.map((key, k) => ({ [name]: key, 'Nesne sayısı': String(groups.filter((g) => g === k).length) }));
    return { groups, count: keys.length, attrs };
  }
  return { groups: inputs.map(() => 0), count: 1, attrs: [{ 'Nesne sayısı': String(inputs.length) }] };
}

/** The summary of a grouped run. */
export function groupedSummary(inputs: number, count: number, group: string, written: number, what: string): string {
  return group === 'each' ? `${inputs} nesneden ${written} ${what} yazıldı.` : `${inputs} nesne ${count} grupta; ${written} ${what} yazıldı.`;
}

/** The notes about the inputs (§1): curves taken as points within 0.1 mm, elevations not carried. */
export function inputNotes(feedback: Feedback, inputs: readonly Entity[], curved: number): void {
  if (curved) feedback.warn(`${curved} nesnenin yayları ya da eğrileri 0,1 mm içinde noktalarla alındı.`);
  const heights = inputs.filter(hasElevation).length;
  if (heights) feedback.warn(`${heights} nesnenin kotları sonuca taşınmadı.`);
}

/** The note about results that are no area. */
export function emptyNote(feedback: Feedback, empty: number): void {
  if (empty) feedback.warn(`${empty} sonuç alan değil (bir nokta ya da bir doğru); yazılmadı.`);
}
