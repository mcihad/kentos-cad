import { describe, expect, it } from 'vitest';
import { templateIssues } from './objectTemplate';

/**
 * A template's rules (docs/adr/0176 §1) as fixtures/style/v1/object-templates.json holds them, written by hand from the ADR: every
 * problem in the order the fields are read, in the same words as the desktop's `kentos_native_style::template`.
 */

interface PenCase {
  id: string;
  template: unknown;
  where?: string;
  issues: string[];
}

const files = import.meta.glob<string>('../../../../fixtures/style/v1/object-templates.json', { query: '?raw', import: 'default', eager: true });
const fixture = JSON.parse(Object.values(files)[0]) as { format: string; version: number; cases: PenCase[] };

describe('a template’s rules (fixtures/style/v1/object-templates.json)', () => {
  it('is a template-cases v1 file', () => {
    expect([fixture.format, fixture.version]).toEqual(['kentos.object-template-cases', 1]);
    expect(fixture.cases.length).toBeGreaterThan(40);
  });
  for (const c of fixture.cases) {
    it(c.id, () => {
      expect(templateIssues(c.template, c.where)).toEqual(c.issues);
    });
  }
});
