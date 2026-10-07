import type { CadDocument } from '../model/document';
import { checkValue } from '../model/layerFields';
import type { ChangeSet } from './types';

/**
 * A run's attribute writes against the layers' fields (docs/adr/0199 §1, 0200 §3), before the runner applies them:
 * each value given to a field becomes the field's canonical text, and the first that does not keep the field's rules
 * refuses the whole run with its reason and its object. Updates are checked first, in their order, each one's changed
 * keys in their code points' order (Rust's `BTreeMap`, the desktop's `writes::check`); then new objects, every key
 * they are given. An attribute taken away is checked as empty (a required field refuses it). Objects gone or on
 * locked layers are not checked: the runner leaves them out anyway.
 */

/** Two names in their code points' order. */
function codePointOrder(a: string, b: string): number {
  const x = [...a];
  const y = [...b];
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const d = x[i].codePointAt(0)! - y[i].codePointAt(0)!;
    if (d) return d < 0 ? -1 : 1;
  }
  return x.length - y.length;
}

/** The changes with every value a field takes in its canonical text, or why the run cannot write them. */
export function checkWrites(doc: CadDocument, ch: ChangeSet): { changes: ChangeSet } | { refused: string } {
  const fieldsOf = (layerId: string) => doc.layers.get(layerId)?.fields ?? [];
  const locked = (layerId: string) => doc.layers.isLocked(layerId);
  const update: NonNullable<ChangeSet['update']> = [];
  for (const u of ch.update ?? []) {
    const e = doc.get(u.id);
    const attrs = u.patch.attrs;
    const layerId = u.patch.layerId ?? e?.layerId;
    const fields = layerId ? fieldsOf(layerId) : [];
    if (!e || !attrs || !layerId || locked(e.layerId) || !fields.length) {
      update.push(u);
      continue;
    }
    let next: Record<string, string> | null = null;
    const keys = [...new Set([...Object.keys(e.attrs), ...Object.keys(attrs)])].sort(codePointOrder);
    for (const key of keys) {
      const field = fields.find((f) => f.name === key);
      const given = Object.hasOwn(attrs, key) ? attrs[key] : undefined;
      if (!field || given === (Object.hasOwn(e.attrs, key) ? e.attrs[key] : undefined)) continue;
      const r = checkValue(field, given ?? '');
      if ('error' in r) return { refused: `Öznitelik yazılamadı (#${e.id}): ${r.message}` };
      if (given !== undefined && r.value !== given) (next ??= { ...attrs })[key] = r.value;
    }
    update.push(next ? { id: u.id, patch: { ...u.patch, attrs: next } } : u);
  }
  const add: NonNullable<ChangeSet['add']> = [];
  for (const n of ch.add ?? []) {
    const fields = fieldsOf(n.layerId);
    if (!fields.length || locked(n.layerId)) {
      add.push(n);
      continue;
    }
    let next: Record<string, string> | null = null;
    for (const key of Object.keys(n.attrs).sort(codePointOrder)) {
      const field = fields.find((f) => f.name === key);
      if (!field) continue;
      const r = checkValue(field, n.attrs[key]);
      if ('error' in r) return { refused: `Öznitelik yazılamadı (yeni nesne): ${r.message}` };
      if (r.value !== n.attrs[key]) (next ??= { ...n.attrs })[key] = r.value;
    }
    add.push(next ? { ...n, attrs: next } : n);
  }
  return { changes: { ...ch, update, add } };
}
