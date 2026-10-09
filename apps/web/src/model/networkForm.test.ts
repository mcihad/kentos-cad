import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/network/v1/form.json?raw';
import type { NetworkDef } from '../contracts/generated/NetworkDef';
import { defOf, formOf, retyped, sameNetwork, type NetworkForm } from './networkForm';

/**
 * Ağlar's form rules (docs/adr/0209 §10) against fixtures/network/v1/form.json (scripts/fixtures/network_form_cases.py,
 * written without KentOS code): what a form writes in a project's unit, the form's own words, the kind's defaults,
 * and networks shown and written back. The desktop plays the same file (kentos_interaction::network::form).
 */
const f = JSON.parse(text) as {
  format: string;
  cases: { name: string; unit: 'm' | 'cm' | 'mm'; form: NetworkForm; network?: NetworkDef; problem?: string }[];
  retype: { name: string; form: NetworkForm; kind: NetworkForm['kind']; expect: NetworkForm }[];
  roundTrip: NetworkDef[];
};
const PER_METRE = { m: 1, cm: 100, mm: 1000 } as const;

describe('Ağlar form (fixtures/network/v1/form.json)', () => {
  it('is the form file', () => expect(f.format).toBe('kentos.network-form'));

  it('writes what the form says, or says what is wrong', () => {
    for (const c of f.cases) {
      const got = defOf(structuredClone(c.form), (v) => v / PER_METRE[c.unit]);
      if (c.problem) expect(got, c.name).toEqual({ problem: c.problem });
      else expect(got, c.name).toEqual(c.network);
    }
  });

  it('turns the kind’s defaults into the other kind’s', () => {
    for (const c of f.retype) expect(retyped(structuredClone(c.form), c.kind), c.name).toEqual(c.expect);
  });

  it('shows a network and writes it back the same', () => {
    for (const n of f.roundTrip) {
      const back = defOf(formOf(n, (m) => m), (v) => v);
      expect('problem' in back, n.name).toBe(false);
      expect(sameNetwork(back as NetworkDef, n), n.name).toBe(true);
    }
  });
});
