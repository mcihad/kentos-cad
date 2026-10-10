import { compileExpression, expressionError } from '../model/expression/expression';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import { costNames as networkCostNames, LENGTH_COST } from '../model/networkRules';
import type { DefaultsContext, FeaturesValue, FieldParam, FileValue, LayerValue, NetworkValue, ParamDef, ProcessingTool, RasterPairs, RasterValues } from './types';

/**
 * Parameter bookkeeping shared by the dialog, the runner and models:
 * defaults, visibility, validation with messages for the user, and
 * restoring stored values (last run, history) safely after a tool's
 * parameters have changed.
 */

type Values = Record<string, unknown>;

const DEFAULT_SCOPES = ['selection', 'visible', 'all', 'layer'] as const;

export function scopesOf(def: Extract<ParamDef, { type: 'features' }>): readonly ('selection' | 'visible' | 'all' | 'layer')[] {
  return def.scopes ?? DEFAULT_SCOPES;
}

export function defaultValue(def: ParamDef, ctx: DefaultsContext): unknown {
  const d = (def as { default?: unknown }).default;
  const v = typeof d === 'function' ? (d as (c: DefaultsContext) => unknown)(ctx) : d;
  if (v !== undefined) return v;
  switch (def.type) {
    case 'features': {
      const scope = scopesOf(def)[0];
      return (scope === 'layer' ? { scope, layerId: ctx.activeLayer } : { scope }) satisfies FeaturesValue;
    }
    case 'number':
      return def.min ?? 0;
    case 'string':
      return '';
    case 'boolean':
      return false;
    case 'enum':
      return def.options[0]?.value;
    case 'layer':
      return { layerId: ctx.activeLayer } satisfies LayerValue;
    case 'point':
      return null;
    case 'expression':
    case 'field':
      return '';
    case 'file':
      return null;
    case 'network': {
      const list = ctx.networks ?? [];
      const n = list.find((x) => x.kind === def.prefers) ?? list[0];
      return n ? ({ network: n.id, cost: LENGTH_COST } satisfies NetworkValue) : null;
    }
    case 'rasterValues':
      return {} satisfies RasterValues;
    case 'rasterPairs':
      return [] satisfies RasterPairs;
  }
}

/** A comparison of two rasters as the pairs hold it (docs/adr/0237 §9): 9 … 2, 1, −2 … −9. */
export const isComparison = (v: number): boolean => Number.isInteger(v) && (v === 1 || (Math.abs(v) >= 2 && Math.abs(v) <= 9));

export function defaultValues(tool: ProcessingTool, ctx: DefaultsContext): Values {
  return Object.fromEntries(tool.parameters.map((p) => [p.name, defaultValue(p, ctx)]));
}

/** Whether the parameter is shown (and checked) for these values. */
export const isVisible = (def: ParamDef, values: Values) => !def.visibleWhen || def.visibleWhen(values);

/** The parameter a field's names come from: the first of `of` that is shown (docs/adr/0200 §6). */
export function fieldSource(tool: ProcessingTool, def: FieldParam, values: Values): string | undefined {
  const names = typeof def.of === 'string' ? [def.of] : def.of;
  return names.find((n) => {
    const p = tool.parameters.find((q) => q.name === n);
    return !!p && isVisible(p, values);
  });
}

/** The names a field parameter holds: one, or several written with commas between them (`multiple`), each trimmed. */
export const fieldNames = (def: Pick<FieldParam, 'multiple'>, value: string): string[] =>
  (def.multiple ? value.split(',') : [value]).map((n) => n.trim()).filter(Boolean);

/** A file parameter's table as fields read it: the first row's names, then the rows. */
export function fileTable(v: FileValue | null | undefined): { header: string[]; rows: (readonly string[])[] } | null {
  if (!v?.rows?.length) return null;
  const [first, ...rows] = v.rows;
  return { header: first.map((c) => c.trim()), rows };
}

/** A value as the last values keep it: a file's name (and path) without its rows. */
export function storedValue(def: ParamDef, v: unknown): unknown {
  if (def.type !== 'file' || !v || typeof v !== 'object') return v;
  const { rows: _rows, ...rest } = v as FileValue;
  return rest;
}

/** Values as the last values keep them (`storedValue` for each). */
export function storedValues(tool: ProcessingTool, values: Values): Values {
  const out: Values = { ...values };
  for (const p of tool.parameters) if (p.name in out) out[p.name] = storedValue(p, out[p.name]);
  return out;
}

/** Whether a stored value still fits the parameter (so it can be restored). */
export function fits(def: ParamDef, v: unknown): boolean {
  if (v === null) return !!def.optional || def.type === 'point';
  switch (def.type) {
    case 'number':
      return typeof v === 'number' && Number.isFinite(v);
    case 'string':
      return typeof v === 'string';
    case 'boolean':
      return typeof v === 'boolean';
    case 'enum':
      return def.options.some((o) => o.value === v);
    case 'features': {
      const f = v as FeaturesValue;
      if (!f || typeof f !== 'object') return false;
      if (f.kinds !== undefined && !(Array.isArray(f.kinds) && f.kinds.every((k) => typeof k === 'string' && (!def.kinds || def.kinds.includes(k))))) return false;
      return f.scope === 'ids' ? Array.isArray(f.ids) : f.scope === 'layer' ? typeof f.layerId === 'string' : scopesOf(def).includes(f.scope);
    }
    case 'expression':
    case 'field':
      return typeof v === 'string';
    case 'file': {
      const f = v as FileValue;
      return !!f && typeof f === 'object' && typeof f.name === 'string' && (f.rows === undefined || (Array.isArray(f.rows) && f.rows.every((r) => Array.isArray(r) && r.every((c) => typeof c === 'string'))));
    }
    case 'layer': {
      const l = v as LayerValue;
      return !!l && typeof l === 'object' && ('layerId' in l ? typeof l.layerId === 'string' : typeof l.newName === 'string');
    }
    case 'point': {
      const p = v as { x?: unknown; y?: unknown };
      return !!p && typeof p.x === 'number' && typeof p.y === 'number';
    }
    case 'network': {
      const n = v as NetworkValue;
      return !!n && typeof n === 'object' && typeof n.network === 'string' && typeof n.cost === 'string';
    }
    case 'rasterValues':
      return (
        !!v &&
        typeof v === 'object' &&
        !Array.isArray(v) &&
        Object.values(v).every((x) => (def.cell === 'number' ? typeof x === 'number' && Number.isFinite(x) : typeof x === 'string'))
      );
    case 'rasterPairs':
      return (
        Array.isArray(v) &&
        v.every((r) => Array.isArray(r) && r.length === 3 && typeof r[0] === 'string' && typeof r[1] === 'string' && typeof r[2] === 'number' && Number.isFinite(r[2]))
      );
  }
}

/** Stored values over defaults, keeping only those that still fit. */
export function restoreValues(tool: ProcessingTool, stored: Values | undefined, ctx: DefaultsContext): Values {
  const out = defaultValues(tool, ctx);
  if (!stored) return out;
  for (const p of tool.parameters) if (p.name in stored && fits(p, stored[p.name])) out[p.name] = stored[p.name];
  return out;
}

export interface ValidationEnv {
  layerExists(id: string): boolean;
  layerLocked(id: string): boolean;
  /** The project's network of an id (docs/adr/0209); absent: none is known. */
  network?(id: string): NetworkDef | undefined;
}

export interface ValidationIssue {
  /** Parameter the message belongs to; absent for tool-level messages. */
  param?: string;
  message: string;
}

/** Problems with the values, in parameter order; empty when the tool can run. */
export function validateValues(tool: ProcessingTool, values: Values, env: ValidationEnv): ValidationIssue[] {
  const issues: ValidationIssue[] = [];
  for (const p of tool.parameters) {
    if (!isVisible(p, values)) continue;
    const message = checkParam(p, values[p.name], env);
    if (message) issues.push({ param: p.name, message });
  }
  if (!issues.length) {
    const m = tool.validate?.(values as never);
    if (m) issues.push({ message: m });
  }
  return issues;
}

function checkParam(p: ParamDef, v: unknown, env: ValidationEnv): string | null {
  const name = `“${p.label}”`;
  if (v === null || v === undefined) {
    if (p.optional) return null;
    if (p.type === 'file') return `${name}: bir dosya seçin.`;
    if (p.type === 'network') return `${name}: projede ağ yok; Ağlar penceresinden yol ya da şebeke ağı tanımlayın.`;
    return `${name} boş bırakılamaz.`;
  }
  if (!fits(p, v)) return `${name} için geçersiz değer.`;
  switch (p.type) {
    case 'number': {
      const n = v as number;
      if (p.integer && !Number.isInteger(n)) return `${name} bir tam sayı olmalı.`;
      if (p.min !== undefined && n < p.min) return `${name} en az ${p.min} olmalı.`;
      if (p.max !== undefined && n > p.max) return `${name} en çok ${p.max} olmalı.`;
      return null;
    }
    case 'string': {
      const s = v as string;
      if (!p.optional && !p.allowEmpty && !s.trim()) return `${name} boş bırakılamaz.`;
      if (p.maxLength !== undefined && s.length > p.maxLength) return `${name} en çok ${p.maxLength} karakter olabilir.`;
      return null;
    }
    case 'features': {
      const f = v as FeaturesValue;
      if (f.scope === 'layer' && !env.layerExists(f.layerId)) return `${name}: seçilen katman artık yok.`;
      if (f.kinds && !f.kinds.length) return `${name}: en az bir nesne türü seçin.`;
      return null;
    }
    case 'expression': {
      const src = (v as string).trim();
      if (!src) return p.optional ? null : `${name}: bir ifade yazın.`;
      // İşlemler give the calls to other layers (docs/adr/0214 §1); the `@` values are read at the run.
      const r = compileExpression(src, { world: true });
      return r.ok ? null : `${name}: ${expressionError(r)}`;
    }
    case 'field': {
      const names = fieldNames(p, v as string);
      if (!names.length) return p.optional ? null : `${name}: bir alan adı seçin${p.allowNew ? ' ya da yazın' : ''}.`;
      if (names.some((f) => f.length > 64)) return `${name}: alan adı en çok 64 karakter olabilir.`;
      if (names.some((f) => /[[\]]/.test(f))) return `${name}: alan adında köşeli parantez kullanılamaz.`;
      return null;
    }
    case 'file': {
      const f = v as FileValue;
      return f.rows ? (f.rows.length ? null : `${name}: “${f.name}” boş; başlık satırı olan bir dosya seçin.`) : `${name}: “${f.name}” dosyasını yeniden seçin; dosyanın içeriği saklanmaz.`;
    }
    case 'network': {
      const n = v as NetworkValue;
      const def = env.network?.(n.network);
      if (!def) return `${name}: “${n.network}” ağı projede yok; Ağlar penceresinden tanımlayın ya da başka ağ seçin.`;
      if (!networkCostNames(def).includes(n.cost)) return `${name}: “${n.cost}” maliyeti “${def.name}” ağında yok.`;
      return null;
    }
    case 'rasterValues': {
      // The rasters in their names' order (as the desktop's map holds them), the first out of bounds said.
      const o = v as RasterValues;
      for (const raster of Object.keys(o).sort()) {
        const n = o[raster];
        if (typeof n !== 'number') continue;
        if ((p.min !== undefined && n < p.min) || (p.max !== undefined && n > p.max))
          return `${name}: ${raster} ${p.min ?? -Number.MAX_VALUE} ile ${p.max ?? Number.MAX_VALUE} arasında olmalı.`;
      }
      return null;
    }
    case 'rasterPairs': {
      const bad = (v as RasterPairs).find((r) => !isComparison(r[2]));
      return bad ? `${name}: ${bad[0]} — ${bad[1]} 1, 2 … 9 ya da −2 … −9 olmalı.` : null;
    }
    case 'layer': {
      const l = v as LayerValue;
      if ('newName' in l) return l.newName.trim() ? null : `${name}: yeni katmanın adını yazın.`;
      if (!env.layerExists(l.layerId)) return `${name}: seçilen katman artık yok.`;
      if (env.layerLocked(l.layerId)) return `${name}: katman kilitli. Kilidi Katmanlar panelinden açın ya da yeni katman seçin.`;
      return null;
    }
    default:
      return null;
  }
}
