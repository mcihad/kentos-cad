import { describe, expect, it } from 'vitest';
import text from '../../../../fixtures/project/v1/wizard.json?raw';
import { blocked, draftScale, draftSrid, initialDraft, optionsOf, stepNote, summary, systemChoices } from './newProjectWizard';
import { wizardFixture } from './newProjectWizardFixture';

const gis = initialDraft({ type: 'gis', fallbackSrid: 5256, font: 'barlow' });
const cad = initialDraft({ type: 'cad', fallbackSrid: 5256, font: 'barlow' });

describe('the new project wizard (docs/adr/0165 §3)', () => {
  it('is the file shared with the desktop (re-record after a deliberate change)', () => {
    expect(JSON.parse(text)).toEqual(JSON.parse(JSON.stringify(wizardFixture())));
  });

  it('starts on the app’s last type: a CAD project local, in metres, at 1:1; a CBS one in the default zone at 1:1000', () => {
    expect([draftSrid(cad), draftScale(cad), cad.unit]).toEqual([0, 1, 'm']);
    expect([draftSrid(gis), draftScale(gis)]).toEqual([5256, 1000]);
    // A local system or an unknown one as the app's default falls back to TM36.
    expect(initialDraft({ type: 'gis', fallbackSrid: 0, font: 'barlow' }).fallbackSrid).toBe(5256);
  });

  it('suggests the province’s zone, first in the list, until another system is chosen', () => {
    const izmir = { ...gis, province: 35 };
    expect(draftSrid(izmir)).toBe(5253);
    expect(systemChoices(izmir)[0]).toMatchObject({ crs: { srid: 5253 }, suggested: true });
    expect(systemChoices(izmir).filter((c) => c.suggested)).toHaveLength(1);
    expect(systemChoices(gis).some((c) => c.crs.srid === 0)).toBe(false);
    expect(draftSrid({ ...izmir, srid: 2319 })).toBe(2319);
  });

  it('says on the rail what was chosen, and sums it up', () => {
    const plate = { ...cad, unit: 'mm' as const, name: 'Mil plakası' };
    expect([stepNote(plate, 'type'), stepNote(plate, 'coords'), stepNote(plate, 'details')]).toEqual(['CAD · teknik çizim', 'Yerel · mm', '1:1 · Mil plakası']);
    expect(summary(plate)).toContainEqual(['Açılış', 'A3 kâğıt, yatay; 0,0 sol altta']);
    expect(stepNote({ ...gis, province: 6, plotScale: 25_000 }, 'details')).toBe('1:25.000 · Yeni proje');
  });

  it('makes the project chosen: a local drawing in its unit, any other on its province', () => {
    expect(optionsOf({ ...cad, unit: 'mm' })).toEqual({ name: 'Yeni proje', srid: 0, plotScale: 1, workspace: 'cad', drawingFont: 'barlow', drawingUnit: 'mm' });
    expect(optionsOf({ ...gis, province: 35 })).toEqual({ name: 'Yeni proje', srid: 5253, plotScale: 1000, workspace: 'gis', drawingFont: 'barlow', province: 35 });
    // A province chosen for a local drawing is not its: it opens at 0,0.
    expect('province' in optionsOf({ ...cad, province: 35 })).toBe(false);
  });

  it('cannot be finished without a name', () => {
    expect(blocked({ ...gis, name: '  ' }, 'details')).toBe('Proje adı boş olamaz.');
    expect(blocked(gis, 'details')).toBeNull();
    expect(blocked({ ...gis, srid: 1234 }, 'coords')).toContain('EPSG:1234');
  });
});
