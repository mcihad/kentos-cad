import { describe, expect, it } from 'vitest';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import { costNames, networkCaseless, networkLayers, networkProblem, networksProblem, nextNetworkId, sanitizedNetworks } from './networkRules';

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

/** The contract's words for every case (fixtures/network/v1/rules.json, written by the Rust contract). */
const rules = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/network/v1/rules.json', import.meta.url), 'utf8')) as {
  networks: { name: string; value: NetworkDef; problem: string | null }[];
  lists: { name: string; value: NetworkDef[]; problem: string | null }[];
};

describe('network rules (docs/adr/0209 §2)', () => {
  it("gives the contract's words for every network and every project's list", () => {
    for (const c of rules.networks) expect(networkProblem(c.value), c.name).toBe(c.problem);
    for (const c of rules.lists) expect(networksProblem(c.value), c.name).toBe(c.problem);
  });

  it('keeps what holds, the first of an id or a name, and names the next network', () => {
    const lists = new Map(rules.lists.map((c) => [c.name, c.value]));
    const two = lists.get('iki ağ') ?? [];
    expect(sanitizedNetworks(two).map((n) => n.id)).toEqual(['ag-1', 'ag-2']);
    expect(sanitizedNetworks(lists.get('aynı ad, büyük küçük harf') ?? []).map((n) => n.id)).toEqual(['a']);
    expect(sanitizedNetworks(lists.get('33 ağ') ?? [])).toHaveLength(32);
    expect(nextNetworkId(two)).toBe('ag-3');
    expect(costNames(two[0])).toEqual(['Uzunluk', 'Süre', 'Ücret']);
    expect(networkLayers(two[1])).toEqual(['su-hatti', 'su-vana', 'su-depo']);
  });

  it('folds Turkish I as Turkish', () => {
    expect(networkCaseless(' IŞIK ')).toBe('ışık');
    expect(networkCaseless('İleri')).toBe('ileri');
  });
});
