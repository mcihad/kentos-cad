import { describe, expect, it } from 'vitest';
import { sameResult, toJson } from './calls/harness';
import { StoreScene, type StoreFile } from './calls/storeCases';

/**
 * The frozen store fixtures (fixtures/geometry/v1/store-*.json: picking,
 * snapping, boxes, labels and grips on a fixed scene, the tool previews and
 * totals, the layer builders' geometry; the processing tools' box test,
 * corner numbering and edge labels), recorded from the TypeScript the
 * store replaced (docs/adr/0008, S1, S4), through the app's path into the
 * WASM geometry store (processing's through a run's own `ObjectStore`);
 * Rust runs the same files natively (crates/shared/geometry-core/tests/store.rs).
 */

const files = import.meta.glob<string>('../../../../fixtures/geometry/v1/store-*.json', { query: '?raw', import: 'default', eager: true });

for (const [path, text] of Object.entries(files)) {
  const file = JSON.parse(text) as StoreFile;
  describe(`WASM: ${path.split('/').pop()}`, () => {
    it('is a store fixture for projected metres', () => {
      expect(file.format).toBe('kentos.geometry-store');
      expect(file.version).toBe(1);
      expect(file.cases.length).toBeGreaterThan(500);
    });
    it('every answer matches', () => {
      const scene = new StoreScene(file);
      const failures = file.cases.map((c) => sameResult(toJson(scene.answer(c.op, c.args)), c.expect, file.tolerance, `${c.op} ${c.name}`)).filter(Boolean);
      scene.dispose();
      expect(failures.slice(0, 5).join('\n')).toBe('');
    });
  });
}
