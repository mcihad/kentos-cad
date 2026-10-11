import { op } from '../wasm/core';
import type { Entity } from './entities';
import { compileExpression, type CompiledExpression, type ExprGeometry } from './expression/expression';

/**
 * A layer's templates (docs/adr/0225 §6): an action's target and the information card's title and text, split by the
 * expression core (`kentos_expression::template`, through WASM) and their expressions compiled once (`$sıra` and
 * `$ölçek` refused: a card does not change with a run or the drawing's scale), then filled for objects with the
 * language's values, a batch at a time. `cad.layers.info` checks with them, the Bilgi tool, the actions and the
 * dialog's preview fill with them. The desktop's is `kentos_native_application::info_templates`.
 */

/** A template's piece as the core splits it: text, an expression (its place 1-based UTF-16), the latitude or longitude. */
export type TemplatePiece = { kind: 'text'; text: string } | { kind: 'expr'; text: string; at: number } | { kind: 'lat' } | { kind: 'lon' };

interface Split {
  pieces: TemplatePiece[];
  encoded: boolean;
  error?: { at: number; message: string };
}

const templateParse = op<(text: string) => Split>('templateParse');
const templateFillMany = op<(text: string, rows: (string | null)[][], places: ([number, number] | null)[]) => string[]>('templateFillMany');
/** What a filled action target is: an `http`, `https` or `mailto` address, a file's path, or neither. */
export const templateTarget = op<(target: string) => 'address' | 'file' | 'refused'>('templateTarget');

/** A template split and its expressions compiled. */
export interface CompiledTemplate {
  readonly text: string;
  readonly pieces: readonly TemplatePiece[];
  readonly encoded: boolean;
  readonly exprs: readonly CompiledExpression[];
}

export type TemplateResult = { ok: true; template: CompiledTemplate } | { ok: false; error: string };

const cache = new Map<string, TemplateResult>();

/** `text` as a template, compiled once per text; why not as the dialogs say it (“12. karakterde: …”, the template's own position). */
export function compileTemplate(text: string): TemplateResult {
  let r = cache.get(text);
  if (r) return r;
  r = compileFresh(text);
  if (cache.size > 500) cache.clear();
  cache.set(text, r);
  return r;
}

function compileFresh(text: string): TemplateResult {
  const split = templateParse(text);
  if (split.error) return { ok: false, error: `${split.error.at}. karakterde: ${split.error.message}` };
  const exprs: CompiledExpression[] = [];
  for (const p of split.pieces) {
    if (p.kind !== 'expr') continue;
    const c = compileExpression(p.text);
    if (!c.ok) return { ok: false, error: `${p.at + c.at - 1}. karakterde: ${c.error}` };
    if (c.expr.needs.index) return { ok: false, error: `${p.at}. karakterde: Şablonda $sıra kullanılamaz.` };
    if (c.expr.needs.scale) return { ok: false, error: `${p.at}. karakterde: Şablonda $ölçek kullanılamaz.` };
    exprs.push(c.expr);
  }
  return { ok: true, template: { text, pieces: split.pieces, encoded: split.encoded, exprs } };
}

/** Whether a template asks for the objects' places in WGS 84 (`{$enlem}`, `{$boylam}`). */
export const needsPlace = (t: CompiledTemplate): boolean => t.pieces.some((p) => p.kind === 'lat' || p.kind === 'lon');

/** Whether a template reads the objects' geometry. */
export const readsGeometry = (t: CompiledTemplate): boolean => t.exprs.some((e) => e.needs.measured || e.needs.centroid || e.needs.bounds || e.needs.vertices);

/**
 * A template's text for each of `entities` (one call to the core per expression and one to fill): `layerName` names a
 * layer by its id (`$katman`), `geometry` is the store the objects are in, `place(i)` object `i`'s latitude and
 * longitude (null: the places are empty).
 */
export function templateTexts(
  t: CompiledTemplate,
  entities: readonly Entity[],
  layerName: (id: string) => string,
  geometry: ExprGeometry | undefined,
  place: (i: number) => [number, number] | null = () => null,
): string[] {
  if (!entities.length) return [];
  const objects = { entities, layerName, ...(geometry && { geometry }) };
  const columns = t.exprs.map((e) => e.evaluateAll(objects, 'text'));
  const rows = entities.map((_, i) => columns.map((c) => {
    const v = c.value(i);
    return v == null ? null : String(v);
  }));
  const places = needsPlace(t) ? entities.map((_, i) => place(i)) : [];
  return templateFillMany(t.text, rows, places);
}
