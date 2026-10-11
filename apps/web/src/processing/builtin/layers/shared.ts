import type { Entity, NewEntity } from '../../../model/entities';

/**
 * What the Katman tools share (docs/adr/0217): a copy of an object, an object's value of some fields, and two names in
 * their code points' order.
 */

/**
 * A copy of an object under a layer (docs/adr/0217 §2): every field but its ids; a linked text's copy is a text of its
 * own and an associative hatch's copy follows nothing (docs/adr/0175 §4, 0186 §6). `attrs`: the copy's own.
 */
export function copyOf(e: Entity, layerId: string, attrs?: Record<string, string>): NewEntity {
  const {
    id: _id,
    uid: _uid,
    labelOf: _of,
    labelScale: _scale,
    assoc: _assoc,
    ...rest
  } = e as Entity & { uid?: string; labelOf?: unknown; labelScale?: unknown; assoc?: unknown };
  return { ...rest, layerId, ...(attrs && { attrs }) } as NewEntity;
}

/** An object's value of these fields (docs/adr/0217 §2): each trimmed, “(boş)” for none, joined by “ – ”. */
export function fieldsValue(e: Entity, fields: readonly string[]): string {
  return fields.map((f) => (Object.hasOwn(e.attrs, f) ? e.attrs[f] : '').trim() || '(boş)').join(' – ');
}

/** Two names in their code points' order (Rust's `BTreeMap`, the desktop's). */
export function codePointOrder(a: string, b: string): number {
  const x = [...a];
  const y = [...b];
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const d = x[i].codePointAt(0)! - y[i].codePointAt(0)!;
    if (d) return d < 0 ? -1 : 1;
  }
  return x.length - y.length;
}

/** The object's shape as text with its keys in order: the kind and every field but those every object has. */
export function shapeKey(e: Entity): string {
  const {
    id: _id,
    uid: _uid,
    layerId: _layer,
    attrs: _attrs,
    label: _label,
    color: _color,
    symbol: _symbol,
    lineWeight: _weight,
    labelPins: _pins,
    attachments: _attachments,
    ...shape
  } = e as Entity & { uid?: string; labelPins?: unknown };
  return stable(shape);
}

/** JSON with every object's keys in order, so that equal shapes are equal texts. */
function stable(v: unknown): string {
  if (Array.isArray(v)) return `[${v.map(stable).join(',')}]`;
  if (v && typeof v === 'object') {
    const o = v as Record<string, unknown>;
    return `{${Object.keys(o)
      .filter((k) => o[k] !== undefined)
      .sort()
      .map((k) => `${JSON.stringify(k)}:${stable(o[k])}`)
      .join(',')}}`;
  }
  return JSON.stringify(v) ?? 'null';
}

export const MODE_OPTIONS = [
  { value: 'copy', label: 'Kopyala', hint: 'Nesneler yerinde kalır, kopyaları yazılır' },
  { value: 'move', label: 'Taşı', hint: 'Nesneler katman değiştirir' },
] as const;
