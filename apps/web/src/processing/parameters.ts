import { compileExpression, expressionError } from '../model/expression/expression';
import type { DefaultsContext, FeaturesValue, LayerValue, ParamDef, ProcessingTool } from './types';

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
  }
}

export function defaultValues(tool: ProcessingTool, ctx: DefaultsContext): Values {
  return Object.fromEntries(tool.parameters.map((p) => [p.name, defaultValue(p, ctx)]));
}

/** Whether the parameter is shown (and checked) for these values. */
export const isVisible = (def: ParamDef, values: Values) => !def.visibleWhen || def.visibleWhen(values);

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
    case 'layer': {
      const l = v as LayerValue;
      return !!l && typeof l === 'object' && ('layerId' in l ? typeof l.layerId === 'string' : typeof l.newName === 'string');
    }
    case 'point': {
      const p = v as { x?: unknown; y?: unknown };
      return !!p && typeof p.x === 'number' && typeof p.y === 'number';
    }
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
  if (v === null || v === undefined) return p.optional ? null : `${name} boş bırakılamaz.`;
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
      const r = compileExpression(src);
      return r.ok ? null : `${name}: ${expressionError(r)}`;
    }
    case 'field': {
      const f = (v as string).trim();
      if (!f) return p.optional ? null : `${name}: bir alan adı seçin${p.allowNew ? ' ya da yazın' : ''}.`;
      if (f.length > 64) return `${name}: alan adı en çok 64 karakter olabilir.`;
      if (/[[\]]/.test(f)) return `${name}: alan adında köşeli parantez kullanılamaz.`;
      return null;
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
