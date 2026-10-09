/**
 * The rules of a project's networks (docs/adr/0209 §2) as the contract says them (`kentos_contracts::network`): the
 * same checks in the same order and the same words, so a command refuses in the browser what it refuses on the
 * desktop and the files refuse. Both are held to fixtures/network/v1/rules.json; tools/kcad/kcad.py reads the files by
 * its own copy.
 */
import type { JunctionRole } from '../contracts/generated/JunctionRole';
import type { NetworkConnect } from '../contracts/generated/NetworkConnect';
import type { NetworkCostKind } from '../contracts/generated/NetworkCostKind';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import type { NetworkKind } from '../contracts/generated/NetworkKind';

/** The tolerance a new network takes (metres). */
export const NETWORK_TOLERANCE = 0.01;
/** The least and the greatest tolerance a network may name (metres). */
export const NETWORK_TOLERANCES = [0.0001, 10] as const;
export const NETWORK_LIMIT = 32;
export const NETWORK_LAYER_LIMIT = 16;
export const NETWORK_COST_LIMIT = 8;
export const NETWORK_VALUE_LIMIT = 16;
export const NETWORK_SPEED_LIMIT = 1000;
/** The cost every network has, its edges' own lengths in metres; no other cost takes its name. */
export const LENGTH_COST = 'Uzunluk';

const ID_LENGTH = 40;
const NAME_LENGTH = 80;
const VALUE_LENGTH = 40;
const FIELD_LENGTH = 64;
const UNIT_LENGTH = 12;
const EXPRESSION_LENGTH = 1000;

export const NETWORK_KINDS: readonly NetworkKind[] = ['road', 'utility'];
export const NETWORK_CONNECTS: readonly NetworkConnect[] = ['ends', 'vertices'];
export const JUNCTION_ROLES: readonly JunctionRole[] = ['junction', 'source', 'valve'];
export const NETWORK_COST_KINDS: readonly NetworkCostKind[] = ['speed', 'field'];

/** A kind's, a connection rule's, a role's and a cost kind's names in the windows. */
export const NETWORK_KIND_LABELS: Record<NetworkKind, string> = { road: 'Yol ağı', utility: 'Şebeke' };
export const NETWORK_CONNECT_LABELS: Record<NetworkConnect, string> = { ends: 'Uçlarda', vertices: 'Köşelerde' };
export const JUNCTION_ROLE_LABELS: Record<JunctionRole, string> = { junction: 'Bağlantı', source: 'Kaynak', valve: 'Vana' };
export const NETWORK_COST_KIND_LABELS: Record<NetworkCostKind, string> = { speed: 'Süre (hızdan)', field: 'Alandan' };

/** Characters, as Rust's `chars().count()` counts them. */
const chars = (s: string): number => [...s].length;

/** A text as the rules compare it: trimmed, upper and lower case the same in Turkish (I is ı's, İ is i's). */
export function networkCaseless(s: string): string {
  let out = '';
  for (const c of s.trim()) {
    if (c === 'I') out += 'ı';
    else if (c === 'İ') out += 'i';
    else {
      const low = c.toLowerCase();
      out += [...low].length === 1 ? low : c;
    }
  }
  return out;
}

/** Whether `id` is a network id: 1–40 of `a`–`z`, `0`–`9` and `-`. */
export function networkIdHolds(id: string): boolean {
  return id.length >= 1 && id.length <= ID_LENGTH && /^[a-z0-9-]+$/.test(id);
}

/** Whether `t` is a tolerance a network may name. */
export function networkToleranceHolds(t: number): boolean {
  return Number.isFinite(t) && t >= NETWORK_TOLERANCES[0] && t <= NETWORK_TOLERANCES[1];
}

function expressionProblem(e: string | undefined, what: string): string | null {
  if (e === undefined) return null;
  if (!e.trim()) return `${what} boş; ifadesi olmayan alan yazılmaz`;
  return chars(e) > EXPRESSION_LENGTH ? `${what} ${EXPRESSION_LENGTH} karakterden uzun` : null;
}

/** What is wrong with the network, in the contract's words; null when it holds. */
export function networkProblem(n: NetworkDef): string | null {
  if (!networkIdHolds(n.id)) return `ağın kimliği “${n.id}”; 1–${ID_LENGTH} küçük harf, rakam ya da tire olmalı`;
  const name = n.name.trim();
  if (!name || name !== n.name || chars(name) > NAME_LENGTH) return `“${n.id}” ağının adı boş, kırpılmamış ya da ${NAME_LENGTH} karakterden uzun`;
  const at = (what: string) => `“${name}” ağının ${what}`;
  const junctions = n.junctions ?? [];
  if (n.edges.length === 0) return at('kenar katmanı yok; en az bir çizgi katmanı olmalı');
  if (n.edges.length > NETWORK_LAYER_LIMIT || junctions.length > NETWORK_LAYER_LIMIT)
    return at(`katmanları çok; en çok ${NETWORK_LAYER_LIMIT} kenar ve ${NETWORK_LAYER_LIMIT} düğüm katmanı`);
  for (let i = 0; i < n.edges.length; i++) {
    const e = n.edges[i];
    if (!e.layer) return at('kenar katmanının kimliği boş');
    const p = expressionProblem(e.filter, at('kenar süzgeci'));
    if (p) return p;
    if (n.edges.slice(0, i).some((o) => o.layer === e.layer && o.filter === e.filter)) return at('aynı kenar katmanı aynı süzgeçle iki kez var');
  }
  for (let i = 0; i < junctions.length; i++) {
    const j = junctions[i];
    if (!j.layer) return at('düğüm katmanının kimliği boş');
    const p = expressionProblem(j.filter, at('düğüm süzgeci')) ?? expressionProblem(j.closed, at('düğümlerin kapalı ifadesi'));
    if (p) return p;
    if (junctions.slice(0, i).some((o) => o.layer === j.layer && o.filter === j.filter)) return at('aynı düğüm katmanı aynı süzgeçle iki kez var');
  }
  if (!networkToleranceHolds(n.tolerance)) return at(`toleransı ${n.tolerance} m; 0,0001 ile 10 m arasında olmalı`);
  const d = n.direction;
  if (d.kind === 'field') {
    if (!d.field.trim() || chars(d.field) > FIELD_LENGTH) return at('yön alanı boş ya da çok uzun');
    if (!d.forward.length && !d.backward.length && !d.closed.length) return at('yön alanının değeri yok; ileri, geri ya da kapalı en az bir değer olmalı');
    const seen: string[] = [];
    for (const list of [d.forward, d.backward, d.closed]) {
      if (list.length > NETWORK_VALUE_LIMIT) return at(`yön değerleri çok; listede en çok ${NETWORK_VALUE_LIMIT} değer`);
      for (const v of list) {
        if (!v.trim() || chars(v) > VALUE_LENGTH) return at(`yön değeri boş ya da ${VALUE_LENGTH} karakterden uzun`);
        const key = networkCaseless(v);
        if (seen.includes(key)) return at(`yön değeri “${v.trim()}” iki kez var`);
        seen.push(key);
      }
    }
  }
  const costs = n.costs ?? [];
  if (costs.length > NETWORK_COST_LIMIT) return at(`maliyetleri çok; en çok ${NETWORK_COST_LIMIT} ek maliyet`);
  for (let i = 0; i < costs.length; i++) {
    const c = costs[i];
    const cost = c.name.trim();
    if (!cost || cost !== c.name || chars(cost) > ID_LENGTH) return at(`maliyetinin adı boş, kırpılmamış ya da ${ID_LENGTH} karakterden uzun`);
    const key = networkCaseless(cost);
    if (key === networkCaseless(LENGTH_COST)) return at(`maliyeti “${cost}”: bu ad ağın uzunluğunundur`);
    if (costs.slice(0, i).some((o) => networkCaseless(o.name) === key)) return at(`maliyeti “${cost}” iki kez var`);
    if (!c.field.trim() || chars(c.field) > FIELD_LENGTH) return at(`“${cost}” maliyetinin alanı boş ya da çok uzun`);
    const unit = c.unit ?? '';
    if (c.kind === 'speed') {
      if (unit) return at(`“${cost}” maliyeti süredir (dakika); birim yazılmaz`);
      const s = c.speed;
      if (s === undefined || !Number.isFinite(s) || s <= 0 || s > NETWORK_SPEED_LIMIT)
        return at(`“${cost}” maliyetinin varsayılan hızı sıfırdan büyük, en çok ${NETWORK_SPEED_LIMIT} km/sa olmalı`);
    } else {
      if (c.speed !== undefined) return at(`“${cost}” maliyeti alandan okunur; hız almaz`);
      if (chars(unit) > UNIT_LENGTH || unit.trim() !== unit) return at(`“${cost}” maliyetinin birimi kırpılmamış ya da ${UNIT_LENGTH} karakterden uzun`);
    }
  }
  return expressionProblem(n.closed, at('kapalı kenarlar ifadesi'));
}

/** What is wrong with a project's networks: too many, an id or a name twice, or one that does not hold. */
export function networksProblem(networks: readonly NetworkDef[]): string | null {
  if (networks.length > NETWORK_LIMIT) return `projede ${networks.length} ağ var; en çok ${NETWORK_LIMIT}`;
  for (let i = 0; i < networks.length; i++) {
    const n = networks[i];
    const p = networkProblem(n);
    if (p) return p;
    const before = networks.slice(0, i);
    if (before.some((o) => o.id === n.id)) return `“${n.id}” kimlikli ağ iki kez var`;
    const key = networkCaseless(n.name);
    if (before.some((o) => networkCaseless(o.name) === key)) return `“${n.name}” adlı ağ iki kez var`;
  }
  return null;
}

/** The networks as a project keeps them: those that hold, the first of an id or a name, at most 32. */
export function sanitizedNetworks(networks: readonly NetworkDef[]): NetworkDef[] {
  const kept: NetworkDef[] = [];
  for (const n of networks) {
    if (kept.length === NETWORK_LIMIT) break;
    const key = networkCaseless(n.name);
    if (networkProblem(n) === null && !kept.some((o) => o.id === n.id || networkCaseless(o.name) === key)) kept.push(structuredClone(n));
  }
  return kept;
}

/** The id a new network takes: the first free `ag-N`. */
export function nextNetworkId(networks: readonly NetworkDef[]): string {
  for (let k = 1; ; k++) {
    const id = `ag-${k}`;
    if (!networks.some((o) => o.id === id)) return id;
  }
}

/** The names of a network's costs in the tools' order: the length first. */
export function costNames(n: NetworkDef): string[] {
  return [LENGTH_COST, ...(n.costs ?? []).map((c) => c.name)];
}

/** The layers a network reads (edges, then junctions), each once in its first place. */
export function networkLayers(n: NetworkDef): string[] {
  const out: string[] = [];
  for (const l of [...n.edges.map((e) => e.layer), ...(n.junctions ?? []).map((j) => j.layer)]) if (!out.includes(l)) out.push(l);
  return out;
}
