import { describe, expect, it } from 'vitest';
import similar from '../../../../fixtures/selection/v1/similar.json?raw';
import { similarTo, type SimilarFacts } from './selectSimilar';

/**
 * Benzerini seç's rule (docs/adr/0187 §4) on the independent reference's cases (fixtures/selection/v1/similar.json,
 * scripts/fixtures/selection_cases.py); the desktop runs the same file (`kentos_interaction::select_similar`).
 */
const f = JSON.parse(similar);

describe('Benzerini seç (fixtures/selection/v1/similar.json)', () => {
  it('selects the objects equal to an example in every criterion that is on', () => {
    const objects = f.objects as SimilarFacts[];
    for (const c of f.cases) {
      const examples = objects.filter((o) => c.examples.includes(o.id));
      expect(similarTo(objects, examples, c.criteria), c.name).toEqual(c.expect);
    }
  });
});
