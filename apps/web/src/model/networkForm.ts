import type { JunctionRole } from '../contracts/generated/JunctionRole';
import type { NetworkConnect } from '../contracts/generated/NetworkConnect';
import type { NetworkCost } from '../contracts/generated/NetworkCost';
import type { NetworkCostKind } from '../contracts/generated/NetworkCostKind';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import type { NetworkKind } from '../contracts/generated/NetworkKind';
import { NETWORK_TOLERANCE, networkProblem } from './networkRules';

/**
 * Ağlar's form (docs/adr/0209 §10): a network as its window shows and takes it, everything as typed. The tolerance in
 * the project's length unit; the direction's values a list with commas or semicolons; a speed cost's default speed
 * (km/sa) empty for 50. `defOf` turns it into the contract's network or says, in words, what is wrong (the form's own
 * words, then the contract's). New networks take their kind's defaults. The desktop's `kentos_interaction::network::
 * form`; both are held to fixtures/network/v1/form.json (scripts/fixtures/network_form_cases.py).
 */

export interface EdgeRow {
  layer: string;
  filter: string;
}

export interface JunctionRow {
  layer: string;
  role: JunctionRole;
  filter: string;
  closed: string;
}

export interface CostRow {
  name: string;
  kind: NetworkCostKind;
  field: string;
  unit: string;
  speed: string;
}

export type DirectionKind = 'both' | 'digitized' | 'field';

export interface NetworkForm {
  id: string;
  name: string;
  kind: NetworkKind;
  edges: EdgeRow[];
  junctions: JunctionRow[];
  connect: NetworkConnect;
  tolerance: string;
  direction: DirectionKind;
  field: string;
  forward: string;
  backward: string;
  shut: string;
  costs: CostRow[];
  closed: string;
}

/** A speed cost's default speed when none is typed, km/sa. */
export const DEFAULT_SPEED = 50;

/** A field's values when Yön takes them from a field and none are typed yet. */
export const DIRECTION_DEFAULTS = { field: 'yon', forward: 'FT, ileri', backward: 'TF, geri', shut: 'N, kapalı' } as const;

const DECIMAL = /^\s*[-+]?(\d+([.,]\d*)?|[.,]\d+)\s*$/;
/** A number as typed: a dot or a comma for the decimals; NaN when it is not one. */
export const decimal = (text: string): number => (DECIMAL.test(text) ? Number(text.trim().replace(',', '.')) : Number.NaN);

/** A number as the form shows it (no noise from a unit's conversion). */
export const shown = (v: number): string => String(Number(v.toPrecision(12)));

/** A list as typed: split at commas and semicolons, trimmed, empty ones dropped. */
export const valuesOf = (text: string): string[] =>
  text
    .split(/[,;]/)
    .map((v) => v.trim())
    .filter(Boolean);

/** A network as the form shows it; `fromMetres` gives the project's length unit. */
export function formOf(def: NetworkDef, fromMetres: (m: number) => number): NetworkForm {
  const d = def.direction;
  return {
    id: def.id,
    name: def.name,
    kind: def.kind,
    edges: def.edges.map((e) => ({ layer: e.layer, filter: e.filter ?? '' })),
    junctions: (def.junctions ?? []).map((j) => ({ layer: j.layer, role: j.role, filter: j.filter ?? '', closed: j.closed ?? '' })),
    connect: def.connect,
    tolerance: shown(fromMetres(def.tolerance)),
    direction: d.kind,
    field: d.kind === 'field' ? d.field : '',
    forward: d.kind === 'field' ? d.forward.join(', ') : '',
    backward: d.kind === 'field' ? d.backward.join(', ') : '',
    shut: d.kind === 'field' ? d.closed.join(', ') : '',
    costs: (def.costs ?? []).map((c) => ({ name: c.name, kind: c.kind, field: c.field, unit: c.unit ?? '', speed: c.speed === undefined || c.speed === null ? '' : shown(c.speed) })),
    closed: def.closed ?? '',
  };
}

/**
 * A new network of `kind` named `name` with id `id`, its first edge layer `layer`: a road network both ways with
 * Süre from the speed field `hiz` (50 km/sa); a utility network in the direction its pipes were drawn, no costs.
 */
export function newForm(id: string, name: string, kind: NetworkKind, layer: string, fromMetres: (m: number) => number): NetworkForm {
  return {
    id,
    name,
    kind,
    edges: layer ? [{ layer, filter: '' }] : [],
    junctions: [],
    connect: 'ends',
    tolerance: shown(fromMetres(NETWORK_TOLERANCE)),
    direction: kind === 'road' ? 'both' : 'digitized',
    field: '',
    forward: '',
    backward: '',
    shut: '',
    costs: kind === 'road' ? [{ name: 'Süre', kind: 'speed', field: 'hiz', unit: '', speed: String(DEFAULT_SPEED) }] : [],
    closed: '',
  };
}

/** The first letter upper case, a full stop at the end: the contract's words as a sentence. */
const sentence = (s: string): string => `${s.charAt(0).toLocaleUpperCase('tr-TR')}${s.slice(1)}.`;

/**
 * The network the form writes, or what is wrong in words: a tolerance or a speed that is not a number (the form's
 * words), then the contract's rules (`networkProblem`, as a sentence). `toMetres`: the project's length unit to metres.
 */
export function defOf(form: NetworkForm, toMetres: (v: number) => number): NetworkDef | { problem: string } {
  const name = form.name.trim();
  const who = name ? `“${name}” ağının` : 'Ağın';
  const t = decimal(form.tolerance);
  if (!Number.isFinite(t)) return { problem: `${who} toleransı bir sayı olmalı (ör. 0,01).` };
  const costs: NetworkCost[] = [];
  for (const c of form.costs) {
    const cost = c.name.trim();
    if (c.kind === 'speed') {
      const typed = c.speed.trim();
      const speed = typed ? decimal(typed) : DEFAULT_SPEED;
      if (!Number.isFinite(speed)) return { problem: `${who} “${cost}” maliyetinin hızı bir sayı olmalı (km/sa).` };
      costs.push({ name: cost, kind: 'speed', field: c.field.trim(), speed });
    } else {
      const unit = c.unit.trim();
      costs.push({ name: cost, kind: 'field', field: c.field.trim(), ...(unit && { unit }) });
    }
  }
  const filter = (s: string) => (s.trim() ? s.trim() : undefined);
  const def: NetworkDef = {
    id: form.id,
    name,
    kind: form.kind,
    edges: form.edges.map((e) => ({ layer: e.layer, ...(filter(e.filter) && { filter: filter(e.filter) }) })),
    ...(form.junctions.length && {
      junctions: form.junctions.map((j) => ({ layer: j.layer, role: j.role, ...(filter(j.filter) && { filter: filter(j.filter) }), ...(filter(j.closed) && { closed: filter(j.closed) }) })),
    }),
    connect: form.connect,
    tolerance: toMetres(t),
    direction:
      form.direction === 'field'
        ? { kind: 'field', field: form.field.trim(), forward: valuesOf(form.forward), backward: valuesOf(form.backward), closed: valuesOf(form.shut) }
        : ({ kind: form.direction } as NetworkDef['direction']),
    ...(costs.length && { costs }),
    ...(filter(form.closed) && { closed: filter(form.closed) }),
  };
  const wrong = networkProblem(def);
  return wrong ? { problem: sentence(wrong) } : def;
}

/** The expressions of a form, each with its place in words (the window compiles them before Kaydet). */
export function formExpressions(form: NetworkForm): { what: string; text: string }[] {
  const out: { what: string; text: string }[] = [];
  form.edges.forEach((e, i) => e.filter.trim() && out.push({ what: `${i + 1}. kenar katmanının süzgeci`, text: e.filter.trim() }));
  form.junctions.forEach((j, i) => {
    if (j.filter.trim()) out.push({ what: `${i + 1}. düğüm katmanının süzgeci`, text: j.filter.trim() });
    if (j.closed.trim()) out.push({ what: `${i + 1}. düğüm katmanının kapalı ifadesi`, text: j.closed.trim() });
  });
  if (form.closed.trim()) out.push({ what: 'Kapalı kenarların ifadesi', text: form.closed.trim() });
  return out;
}

/** A value with its keys in order, so that two networks compare by what they say (`sameNetwork`). */
function canonical(v: unknown): unknown {
  if (Array.isArray(v)) return v.map(canonical);
  if (v && typeof v === 'object')
    return Object.fromEntries(
      Object.keys(v)
        .filter((k) => (v as Record<string, unknown>)[k] !== undefined)
        .sort()
        .map((k) => [k, canonical((v as Record<string, unknown>)[k])]),
    );
  return v;
}

/** Whether two networks say the same (their keys in any order). */
export const sameNetwork = (a: NetworkDef, b: NetworkDef): boolean => JSON.stringify(canonical(a)) === JSON.stringify(canonical(b));

/** Whether a form's costs are a new road network's (Süre from `hiz`, 50 km/sa). */
const roadCosts = (f: NetworkForm): boolean =>
  f.costs.length === 1 && f.costs[0].name === 'Süre' && f.costs[0].kind === 'speed' && f.costs[0].field === 'hiz' && decimal(f.costs[0].speed || String(DEFAULT_SPEED)) === DEFAULT_SPEED;

/**
 * The form with another kind: what is still the old kind's default becomes the new kind's (the direction: both ways
 * or as drawn; the costs: a road network's Süre or none); what was changed stays.
 */
export function retyped(f: NetworkForm, kind: NetworkKind): NetworkForm {
  if (f.kind === kind) return f;
  const out = { ...f, kind };
  if (kind === 'utility') {
    if (f.direction === 'both') out.direction = 'digitized';
    if (roadCosts(f)) out.costs = [];
  } else {
    if (f.direction === 'digitized') out.direction = 'both';
    if (!f.costs.length) out.costs = [{ name: 'Süre', kind: 'speed', field: 'hiz', unit: '', speed: String(DEFAULT_SPEED) }];
  }
  return out;
}
