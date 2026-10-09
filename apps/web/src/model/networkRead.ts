/**
 * A project's networks read from the contract's JSON form (docs/adr/0209 §2): field by field (a field the contract
 * does not have stops the read, as the KCAD readers' `unknown_field` does), then each whole and the list by the
 * contract's rules (`networkRules.ts`). The reader that calls these says where (`where`) and how to stop (`fail`): the
 * drawing's open, the server's settings and `cad.network.define`'s input.
 */
import type { JunctionLayer } from '../contracts/generated/JunctionLayer';
import type { JunctionRole } from '../contracts/generated/JunctionRole';
import type { NetworkConnect } from '../contracts/generated/NetworkConnect';
import type { NetworkCost } from '../contracts/generated/NetworkCost';
import type { NetworkCostKind } from '../contracts/generated/NetworkCostKind';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import type { NetworkDirection } from '../contracts/generated/NetworkDirection';
import type { NetworkKind } from '../contracts/generated/NetworkKind';
import { JUNCTION_ROLES, NETWORK_CONNECTS, NETWORK_COST_KINDS, NETWORK_KINDS, networkProblem, networksProblem } from './networkRules';

type Fail = (where: string, what: string) => never;
type Obj = Record<string, unknown>;

const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);

const NETWORK_KEYS = ['id', 'name', 'kind', 'edges', 'junctions', 'connect', 'tolerance', 'direction', 'costs', 'closed'] as const;
const EDGE_KEYS = ['layer', 'filter'] as const;
const JUNCTION_KEYS = ['layer', 'role', 'filter', 'closed'] as const;
const COST_KEYS = ['name', 'kind', 'field', 'unit', 'speed'] as const;

function readers(fail: Fail) {
  const text = (v: unknown, w: string): string => (typeof v === 'string' ? v : fail(w, 'metin olmalı'));
  const number = (v: unknown, w: string): number => (typeof v === 'number' && Number.isFinite(v) ? v : fail(w, 'sonlu bir sayı olmalı'));
  const texts = (v: unknown, w: string): string[] => (Array.isArray(v) ? v.map((x, i) => text(x, `${w} › ${i + 1}`)) : fail(w, 'metin listesi olmalı'));
  const list = <T>(v: unknown, w: string, item: (x: unknown, w: string) => T): T[] =>
    Array.isArray(v) ? v.map((x, i) => item(x, `${w} › ${i + 1}`)) : fail(w, 'liste olmalı');
  const known = (o: Obj, keys: readonly string[], w: string) => {
    const unknown = Object.keys(o).find((k) => !keys.includes(k));
    if (unknown !== undefined) fail(`${w} › ${unknown}`, 'bilinmeyen alan');
  };
  const oneOf = <T extends string>(v: unknown, values: readonly T[], w: string): T =>
    values.includes(v as T) ? (v as T) : fail(w, `şunlardan biri olmalı: ${values.join(', ')}`);
  return { text, number, texts, list, known, oneOf };
}

function readDirection(v: unknown, w: string, fail: Fail): NetworkDirection {
  if (!isObj(v)) return fail(w, 'nesne olmalı');
  const r = readers(fail);
  const kind = r.oneOf(v.kind, ['both', 'digitized', 'field'] as const, `${w} › tür`);
  if (kind !== 'field') {
    r.known(v, ['kind'], w);
    return { kind };
  }
  r.known(v, ['kind', 'field', 'forward', 'backward', 'closed'], w);
  const values = (k: 'forward' | 'backward' | 'closed') => (v[k] === undefined ? [] : r.texts(v[k], `${w} › ${k}`));
  return { kind, field: r.text(v.field, `${w} › alan`), forward: values('forward'), backward: values('backward'), closed: values('closed') };
}

/** A network, its fields then its rules. */
export function readNetwork(v: unknown, w: string, fail: Fail): NetworkDef {
  if (!isObj(v)) return fail(w, 'nesne olmalı');
  const r = readers(fail);
  r.known(v, NETWORK_KEYS, w);
  const n: NetworkDef = {
    id: r.text(v.id, `${w} › id`),
    name: r.text(v.name, `${w} › ad`),
    kind: r.oneOf<NetworkKind>(v.kind, NETWORK_KINDS, `${w} › tür`),
    edges: r.list(v.edges, `${w} › kenarlar`, (e, ew) => {
      if (!isObj(e)) return fail(ew, 'nesne olmalı');
      r.known(e, EDGE_KEYS, ew);
      return { layer: r.text(e.layer, `${ew} › katman`), ...(e.filter === undefined ? {} : { filter: r.text(e.filter, `${ew} › süzgeç`) }) };
    }),
    connect: r.oneOf<NetworkConnect>(v.connect, NETWORK_CONNECTS, `${w} › bağlanma`),
    tolerance: r.number(v.tolerance, `${w} › tolerans`),
    direction: readDirection(v.direction, `${w} › yön`, fail),
  };
  if (v.junctions !== undefined)
    n.junctions = r.list(v.junctions, `${w} › düğümler`, (j, jw): JunctionLayer => {
      if (!isObj(j)) return fail(jw, 'nesne olmalı');
      r.known(j, JUNCTION_KEYS, jw);
      return {
        layer: r.text(j.layer, `${jw} › katman`),
        role: r.oneOf<JunctionRole>(j.role, JUNCTION_ROLES, `${jw} › rol`),
        ...(j.filter === undefined ? {} : { filter: r.text(j.filter, `${jw} › süzgeç`) }),
        ...(j.closed === undefined ? {} : { closed: r.text(j.closed, `${jw} › kapalı`) }),
      };
    });
  if (v.costs !== undefined)
    n.costs = r.list(v.costs, `${w} › maliyetler`, (c, cw): NetworkCost => {
      if (!isObj(c)) return fail(cw, 'nesne olmalı');
      r.known(c, COST_KEYS, cw);
      return {
        name: r.text(c.name, `${cw} › ad`),
        kind: r.oneOf<NetworkCostKind>(c.kind, NETWORK_COST_KINDS, `${cw} › tür`),
        field: r.text(c.field, `${cw} › alan`),
        ...(c.unit === undefined ? {} : { unit: r.text(c.unit, `${cw} › birim`) }),
        ...(c.speed === undefined ? {} : { speed: r.number(c.speed, `${cw} › hız`) }),
      };
    });
  if (v.closed !== undefined) n.closed = r.text(v.closed, `${w} › kapalı`);
  const problem = networkProblem(n);
  return problem ? fail(w, problem) : n;
}

/** A project's networks, each its fields, then the list by its rules. */
export function readNetworks(v: unknown, w: string, fail: Fail): NetworkDef[] {
  if (!Array.isArray(v)) return fail(w, 'liste olmalı');
  const list = v.map((n, i) => readNetwork(n, `${w} › ${i + 1}`, fail));
  const problem = networksProblem(list);
  return problem ? fail(w, problem) : list;
}
