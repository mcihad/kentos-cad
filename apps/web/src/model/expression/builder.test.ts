import { describe, expect, it } from 'vitest';
import { callNamed } from '../../wasm/core';

/**
 * The expression builder's services (fixtures/expression/v2/builder.json)
 * through WASM: the answers the Rust core gives natively
 * (crates/shared/expression/tests/builder.rs), so the web's and the
 * desktop's builders colour, complete and explain alike.
 */

interface Case {
  readonly note?: string;
  readonly op: string;
  readonly args: readonly unknown[];
  readonly result: unknown;
}

const fs = (globalThis as unknown as { process: { getBuiltinModule(id: 'node:fs'): { readFileSync(u: URL, e: 'utf8'): string } } }).process.getBuiltinModule('node:fs');
const file = JSON.parse(fs.readFileSync(new URL('../../../../../fixtures/expression/v2/builder.json', import.meta.url), 'utf8')) as {
  fields: unknown[];
  cases: Case[];
};

/** The answer, or what the core threw (as the Rust test records it). */
function answer(c: Case): unknown {
  const args = c.args.map((a) => (a === '@fields' ? file.fields : a));
  try {
    return callNamed(c.op, args);
  } catch (err) {
    return { thrown: (err as Error).message };
  }
}

describe('ifade oluşturucusunun dil hizmetleri', () => {
  it('has cases for every service', () => {
    const ops = new Set(file.cases.map((c) => c.op));
    expect([...ops].sort()).toEqual(['exprBracket', 'exprBuilderCatalog', 'exprCheck', 'exprComplete', 'exprHelp', 'exprHelpAt', 'exprPlace', 'exprPreview', 'exprSignature', 'exprTokens', 'exprValues']);
  });

  it.each(file.cases.map((c) => [`${c.op} ${JSON.stringify(c.args).slice(0, 70)}`, c] as const))('%s', (_, c) => {
    expect(answer(c)).toEqual(c.result);
  });
});
