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
  { name: 'Web haritası', srid: 3857, plotScale: 2000, workspace: 'gis', drawingFont: 'plex-mono' },
  { name: '', srid: 32636, plotScale: 5000 },
  // A local CAD project: no coordinate system, its origin 0,0 (docs/adr/0165 §2).
  { name: 'Plan', srid: 0, plotScale: 50, workspace: 'cad' },
  // Opened on a province's centre (docs/adr/0165 §3): TUREF and ED50 TM zones, UTM, geographic and web Mercator.
  { name: 'İzmir', srid: 5253, plotScale: 1000, workspace: 'gis', province: 35 },
  { name: 'Trabzon', srid: 2323, plotScale: 2000, province: 61 },
  { name: 'Adana', srid: 32636, plotScale: 5000, province: 1 },
  { name: 'Ankara', srid: 5252, plotScale: 25000, province: 6 },
  { name: 'İstanbul', srid: 3857, plotScale: 2000, province: 34 },
  // A local project in millimetres at full size, on an A3 sheet from 0,0 (docs/adr/0165 §2, §3).
  { name: 'Mil plakası', srid: 0, plotScale: 1, workspace: 'cad', drawingUnit: 'mm' },
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
