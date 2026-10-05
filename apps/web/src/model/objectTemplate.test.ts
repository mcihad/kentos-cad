import { describe, expect, it } from 'vitest';
import { LayerStore, type LayerInit } from './layers';
import { lockedTemplateLayerText, templateIssues, templateLayer, type TemplateLayerAnswer } from './objectTemplate';

/**
 * A template's rules (docs/adr/0176 §1) as fixtures/style/v1/object-templates.json holds them, written by hand from the ADR: every
 * problem in the order the fields are read, in the same words as the desktop's `kentos_native_style::template`.
 */

interface TemplateCase {
  id: string;
  template: unknown;
  where?: string;
  issues: string[];
}

const files = import.meta.glob<string>('../../../../fixtures/style/v1/object-templates.json', { query: '?raw', import: 'default', eager: true });
const fixture = JSON.parse(Object.values(files)[0]) as { format: string; version: number; cases: TemplateCase[] };

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

/** A case of fixtures/style/v1/template-layers.json: a tree, a template's layer, where it draws (scripts/fixtures/template_layer_cases.py). */
interface LayerCase {
  name: string;
  tree: LayerInit[];
  layer: { path: string[]; name: string };
  result: TemplateLayerAnswer;
}

const layerFiles = import.meta.glob<string>('../../../../fixtures/style/v1/template-layers.json', { query: '?raw', import: 'default', eager: true });
const layerFixture = JSON.parse(Object.values(layerFiles)[0]) as { format: string; version: number; cases: LayerCase[] };

describe('where a template draws (fixtures/style/v1/template-layers.json)', () => {
  it('is a template-layer-cases v1 file', () => {
    expect([layerFixture.format, layerFixture.version]).toEqual(['kentos.template-layer-cases', 1]);
    expect(layerFixture.cases.length).toBeGreaterThanOrEqual(20);
  });
  for (const c of layerFixture.cases) {
    it(c.name, () => {
      // A layer of its own last, active, since a drawing has one (no case's template names it).
      const layers = new LayerStore([...c.tree, { id: 'zz-test', name: 'Sınama', type: 'layer' }], 'zz-test');
      expect(templateLayer(layers, c.layer)).toEqual(c.result);
    });
  }
  it('says a locked layer or group by its name and the template’s', () => {
    expect(lockedTemplateLayerText({ name: 'Parsel', type: 'layer' }, 'Parsel sınırı')).toBe('“Parsel” katmanı kilitli; “Parsel sınırı” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın.');
    expect(lockedTemplateLayerText({ name: 'Kadastro', type: 'group' }, 'Parsel sınırı')).toBe('“Kadastro” grubu kilitli; “Parsel sınırı” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın.');
  });
});
