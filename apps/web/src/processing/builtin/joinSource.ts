import type { Entity } from '../../model/entities';
import { fileTable } from '../parameters';
import type { FileValue } from '../types';

/**
 * A join's source as rows (Anahtarla birleştir, docs/adr/0200 §6; Veri aktar, docs/adr/0216): another layer's objects or
 * a CSV, TXT or Excel file's rows, read the same way by both tools.
 */

/** Two names in their code points' order: a layer source's fields as both platforms list them. */
function codePointOrder(a: string, b: string): number {
  const x = [...a];
  const y = [...b];
  for (let i = 0; i < Math.min(x.length, y.length); i++) {
    const d = x[i].codePointAt(0)! - y[i].codePointAt(0)!;
    if (d) return d < 0 ? -1 : 1;
  }
  return x.length - y.length;
}

/** A source as rows: its column names, then a row per object or line (cells as written; a missing one is empty). */
export function sourceTable(objects: readonly Entity[] | null, file: FileValue | null): { header: string[]; rows: (readonly string[])[] } {
  if (!objects) return fileTable(file) ?? { header: [], rows: [] };
  const header = [...new Set(objects.flatMap((e) => Object.keys(e.attrs)))].sort(codePointOrder);
  return { header, rows: objects.map((e) => header.map((h) => (Object.hasOwn(e.attrs, h) ? e.attrs[h] : ''))) };
}

/** Why a column cannot be read: the names there are. */
export const missing = (name: string, header: readonly string[]): string => {
  const names = header.filter(Boolean);
  return names.length ? `Kaynakta “${name}” alanı yok; alanları: ${names.join(', ')}.` : `Kaynakta “${name}” alanı yok; kaynağın hiç alanı yok.`;
};
