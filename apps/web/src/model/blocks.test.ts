import { describe, expect, it } from 'vitest';
import { blocksFault, blockFaultMessage, checkDefinitions, checkNesting, nameKey, nameOk, type BlockDefinition } from './blocks';
import type { Entity } from './entities';

/**
 * The block rules (docs/adr/0144) as the Rust contracts' tests have them
 * (crates/shared/contracts/src/blocks.rs): the same cases, the same answers.
 */

const id = (n: number) => `0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d${n.toString(16).padStart(4, '0')}`;

const insert = (block: number, scale = 1): Entity => ({ kind: 'insert', id: 1, layerId: '0', attrs: {}, block: id(block), p: { x: 0, y: 0 }, scale, rotation: 0 });

const block = (n: number, name: string, inside: number[] = []): BlockDefinition => ({ id: id(n), name, base: { x: 0, y: 0 }, entities: inside.map((b) => insert(b)) });

describe('block rules (docs/adr/0144)', () => {
  it('fold names with Turkish case', () => {
    expect(nameKey('IŞIK')).toBe('ışık');
    expect(nameKey('İzmir')).toBe('izmir');
    expect(nameKey('Direk')).toBe(nameKey('DİREK'));
    expect(nameKey('DIREK')).not.toBe(nameKey('direk'));
    expect(blocksFault([block(1, 'Rögar'), block(2, 'RÖGAR')], [])).toEqual({ kind: 'duplicateName', definition: 1, first: 0 });
    // White space as Rust's `char::is_whitespace` has it: U+0085 is, U+FEFF is not.
    expect([nameOk(' \t'), nameOk('\u0085　'), nameOk('﻿'), nameOk('')]).toEqual([false, false, true, false]);
    expect(blocksFault([block(1, ' \t')], [])).toEqual({ kind: 'emptyName', definition: 0 });
  });

  it('keep ids and tags once', () => {
    expect(blocksFault([block(1, 'A'), block(1, 'B')], [])).toEqual({ kind: 'duplicateId', definition: 1, first: 0 });
    const tag = (t: string) => ({ tag: t, p: { x: 0, y: 0 }, height: 1, rotation: 0 });
    expect(blocksFault([{ ...block(1, 'A'), attributes: [tag('NO'), tag('NO')] }], [])).toEqual({ kind: 'duplicateTag', definition: 0, attribute: 1, first: 0 });
    expect(blocksFault([{ ...block(1, 'A'), attributes: [tag('')] }], [])).toEqual({ kind: 'emptyTag', definition: 0, attribute: 0 });
  });

  it('let inserts name known blocks with a positive scale', () => {
    expect(blocksFault([block(1, 'A', [2])], [])).toEqual({ kind: 'unknownBlock', at: { definition: 0, entity: 0 } });
    expect(blocksFault([block(1, 'A')], [insert(1, 2.5)])).toBeNull();
    for (const scale of [0, -1, Infinity, NaN]) expect(blocksFault([block(1, 'A')], [insert(1, scale)])).toEqual({ kind: 'badScale', at: { definition: null, entity: 0 } });
  });

  it('refuse cycles', () => {
    expect(blocksFault([block(1, 'A', [1])], [])).toEqual({ kind: 'cycle', definition: 0 });
    expect(blocksFault([block(1, 'A', [2]), block(2, 'B', [3]), block(3, 'C', [1])], [])).toEqual({ kind: 'cycle', definition: 0 });
  });

  it('nest at most sixteen levels', () => {
    const chain = (levels: number) => Array.from({ length: levels }, (_, i) => block(i + 1, `B${i + 1}`, i + 1 < levels ? [i + 2] : []));
    const ok = chain(16);
    const own = checkDefinitions(ok);
    if (!('index' in own)) throw new Error('definitions');
    const nesting = checkNesting(ok, own.index);
    expect('depth' in nesting && [nesting.depth[0], nesting.depth[15]]).toEqual([16, 1]);
    expect(blocksFault(chain(17), [])).toEqual({ kind: 'tooDeep', definition: 0 });
    // Reached through a definition already measured: the last one listed first.
    const late = chain(17);
    late.push(late.shift()!);
    expect(blocksFault(late, [])).toEqual({ kind: 'tooDeep', definition: 16 });
    // A cycle longer than the limit is too deep.
    const ring = chain(20);
    ring[19].entities.push(insert(1));
    expect(blocksFault(ring, [])).toEqual({ kind: 'tooDeep', definition: 0 });
  });

  it('say their faults in the documents’ words', () => {
    const names = ['Rögar', 'DİREK'];
    const name = (i: number) => names[i];
    expect(blockFaultMessage({ kind: 'duplicateName', definition: 1, first: 0 }, name)).toBe('Çizimde “Rögar” adında bir blok var; başka bir ad verin.');
    expect(blockFaultMessage({ kind: 'cycle', definition: 1 }, name)).toBe('“DİREK” bloğu kendini içeremez (doğrudan ya da başka bloklar yoluyla).');
    expect(blockFaultMessage({ kind: 'tooDeep', definition: 0 }, name)).toBe('Bloklar en çok 16 düzey iç içe olabilir; “Rögar” bloğu daha derin.');
  });
});
