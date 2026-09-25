import { describe, expect, it } from 'vitest';
import type { V1EntityIdentity } from '../contracts/generated/V1EntityIdentity';
import { CadDocument } from '../model/document';
import { LayerStore } from '../model/layers';
import { attachV1Identities, readSnapshot, toSnapshot } from '../model/snapshot';
import { snapshotSampleDocument } from '../model/snapshotSample';
import { formatsBuilt, v1IdentitiesInProcess } from './testFormats';

/**
 * The persistent ids of v1 drawings (docs/adr/0014, TODOS.md DOM-04) as the
 * browser computes them: the formats WASM module on the fixture the
 * independent Python reference wrote (fixtures/document/v1/identity,
 * scripts/fixtures/v1_identity_reference.py), which the native contracts
 * reproduce too (crates/shared/contracts/tests/identity.rs). JSON.stringify
 * spells numbers otherwise than serde_json, so the ids cannot be computed in
 * TypeScript: the Rust contract reads the file.
 */

interface Case {
  name: string;
  inputs: string[];
  sourceSha256: string;
  namespace: string;
  project: string;
  entities: V1EntityIdentity[];
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, enc: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const fixture = (name: string) => fs.readFileSync(new URL(`../../../../fixtures/document/v1/identity/${name}`, import.meta.url), 'utf8');
const expected = JSON.parse(fixture('expected.json')) as { cases: Case[] };

describe.skipIf(!formatsBuilt)('v1 drawing identities in the browser (formats WASM module)', () => {
  it('gives every input of every case the reference ids', async () => {
    expect(expected.cases.map((c) => c.name)).toEqual(['sample', 'edited', 'minimal']);
    for (const c of expected.cases) {
      for (const input of c.inputs) {
        const { sourceSha256, namespace, project, entities } = c;
        expect(await v1IdentitiesInProcess(fixture(input)), `${c.name}: ${input}`).toEqual({ sourceSha256, namespace, project, entities });
      }
    }
  });

  it('reads the drawing the app writes: the compact sample is the app’s own file', () => {
    expect(fixture('sample.compact.kcad')).toBe(`${JSON.stringify(toSnapshot(snapshotSampleDocument()))}\n`);
  });

  it('gives the opened drawing’s objects those ids, each object by its local id', async () => {
    const text = fixture('sample.compact.kcad');
    const read = readSnapshot(text);
    if (!read.ok) throw new Error(read.error);
    expect(attachV1Identities(read.content, await v1IdentitiesInProcess(text))).toBeNull();
    const doc = new CadDocument({ name: 'boş', layers: new LayerStore([{ id: 'x', name: 'X' }], 'x'), origin: { x: 0, y: 0 } });
    doc.replaceWith(read.content);
    for (const { id, uid } of expected.cases[0].entities) expect(doc.byUid(uid)?.id).toBe(id);
    // Written again, then opened again: the same ids (the file is the same drawing).
    const again = JSON.stringify(toSnapshot(doc));
    expect((await v1IdentitiesInProcess(again)).entities).toEqual(expected.cases[0].entities);
  });

  it('refuses ids that do not belong to the drawing read, and a drawing the contract cannot read says why', async () => {
    const read = readSnapshot(fixture('sample.compact.kcad'));
    if (!read.ok) throw new Error(read.error);
    const other = await v1IdentitiesInProcess(fixture('minimal.kcad'));
    expect(attachV1Identities(read.content, other)).toContain('kimlik listesinde 2 nesne var, çizimde 13');
    expect(read.content.entities.some((e) => e.uid)).toBe(false);
    const shuffled = await v1IdentitiesInProcess(fixture('sample.compact.kcad'));
    shuffled.entities.reverse();
    expect(attachV1Identities(read.content, shuffled)).toContain('1. nesnenin kimliği uyuşmuyor');
    await expect(v1IdentitiesInProcess('{"format":"kentos.document","version":2}')).rejects.toThrow('sürümü 2');
  });
});
