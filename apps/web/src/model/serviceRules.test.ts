import { describe, expect, it } from 'vitest';
import type { FeatureFeed } from '../contracts/generated/FeatureFeed';
import type { ServiceConnection } from '../contracts/generated/ServiceConnection';
import type { ServiceLayer } from '../contracts/generated/ServiceLayer';
import { connectionsProblem, feedProblem, linkFaultWords, originOf, serviceLinks, serviceProblem } from './serviceRules';

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');

/** The contract's words for every case (fixtures/services/v1/rules.json, written by the Rust contract). */
const rules = JSON.parse(fs.readFileSync(new URL('../../../../fixtures/services/v1/rules.json', import.meta.url), 'utf8')) as {
  services: { name: string; value: ServiceLayer; problem: string | null }[];
  feeds: { name: string; value: FeatureFeed; problem: string | null }[];
  connections: { name: string; value: ServiceConnection[]; problem: string | null }[];
  origins: { url: string; origin: string | null }[];
};

describe('service rules (docs/adr/0208 §2)', () => {
  it("gives the contract's words for every service, feed and connection list", () => {
    for (const c of rules.services) expect(serviceProblem(c.value), c.name).toBe(c.problem);
    for (const c of rules.feeds) expect(feedProblem(c.value), c.name).toBe(c.problem);
    for (const c of rules.connections) expect(connectionsProblem(c.value), c.name).toBe(c.problem);
  });

  it('gives the origins of addresses as the contract does', () => {
    for (const c of rules.origins) expect(originOf(c.url), c.url).toBe(c.origin);
  });

  it('finds the first broken link of a drawing: both, an unknown connection, an object on a service layer', () => {
    const osm = rules.services[0].value;
    const layer = (id: string, more: object = {}) => ({ id, name: id.toUpperCase(), children: [], ...more });
    expect(serviceLinks([layer('a', { service: osm })], [], [{ layerId: 'b' }])).toBeNull();
    const both = serviceLinks([layer('a', { service: osm, feed: rules.feeds[0].value })], [], []);
    expect(both && linkFaultWords(both)).toBe('“A” katmanı hem servisten çizilir hem nesnelerini bir kaynaktan alır; ikisi birden olmaz');
    const unknown = serviceLinks([layer('g', { children: [layer('a', { service: { ...osm, connection: 'hgm' } })] })], [], []);
    expect(unknown).toEqual({ kind: 'connection', layer: 'A', connection: 'hgm' });
    expect(serviceLinks([layer('a', { service: osm })], [], [{ layerId: 'b' }, { layerId: 'a' }])).toEqual({ kind: 'object', index: 1 });
  });
});
