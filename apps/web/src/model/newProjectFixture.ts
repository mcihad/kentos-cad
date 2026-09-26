import { CadDocument } from './document';
import { LayerStore } from './layers';
import { newProjectContent, type NewProjectOptions } from './newProject';
import { toSnapshot } from './snapshot';

/**
 * The new-project drawings as a versioned file shared with the desktop
 * (fixtures/project/v1/new-project.json): each case's options and the
 * `.kcad` v1 snapshot of the drawing it starts as, with every layer
 * default filled in as the document fills it. Built by the recorder
 * (scripts/fixtures/record-new-project.test.ts), compared by newProject.test.ts.
 */

export const NEW_PROJECT_CASES: readonly NewProjectOptions[] = [
  { name: 'Yeni proje', srid: 5256, plotScale: 1000 },
  { name: '  Ada 101  ', srid: 5254, plotScale: 500, workspace: 'cad', drawingFont: 'arimo' },
  { name: 'Harita', srid: 4326, plotScale: 25000, workspace: 'gis' },
  { name: 'Web haritası', srid: 3857, plotScale: 2000, workspace: 'hybrid', drawingFont: 'plex-mono' },
  { name: '', srid: 32636, plotScale: 5000 },
];

export function newProjectFixture() {
  return {
    format: 'kentos.new-project',
    version: 1,
    source: 'src/model/newProject.ts',
    cases: NEW_PROJECT_CASES.map((options) => {
      const doc = new CadDocument({ name: 't', layers: new LayerStore([], ''), origin: { x: 0, y: 0 } });
      doc.replaceWith(newProjectContent(options));
      return { options, snapshot: toSnapshot(doc) };
    }),
  };
}
