import { CRS_REGISTRY, crsBySrid, crsTitle, LOCAL_SRID, turefZoneFor, type CrsDef } from '../geo/crs';
import { provinceByCode, type Province } from '../geo/provinces';
import { NEW_PROJECT_NAME, type NewProjectOptions } from './newProject';
import type { DrawingFont, DrawingUnit, Workspace } from './projectSettings';

/**
 * What the Yeni proje wizard asks, as data (docs/adr/0165 §3): its choices, what each step offers for them, what
 * the rail and the summary say, and the project they make. The windows draw it: the web's
 * ui/settings/NewProjectWizard.ts and the desktop's project/wizard. The desktop keeps the same rules
 * (kentos_project::wizard), held to them by fixtures/project/v1/wizard.json.
 */

/** The steps, in order: the project's type, its coordinates, its scale and details. */
export const WIZARD_STEPS = ['type', 'coords', 'details'] as const;
export type WizardStep = (typeof WIZARD_STEPS)[number];

/** The steps' names, as the rail writes them. */
export const STEP_NAMES: Record<WizardStep, string> = { type: 'Proje türü', coords: 'Koordinatlar', details: 'Ölçek ve ayrıntılar' };

/** A CAD project's coordinates: none (a drawing of its own, from 0,0) or a real coordinate system. */
export type CadCoords = 'local' | 'real';

/** What the wizard holds while it is open; what is not chosen follows the others. */
export interface WizardDraft {
  type: 'cad' | 'gis';
  /** A CAD project's coordinates; a CBS project always has a system. */
  coords: CadCoords;
  /** A local project's unit. */
  unit: DrawingUnit;
  /** The province's plate code, whose centre the project opens on and whose zone is suggested. */
  province: number | null;
  /** The system chosen; null: the zone suggested for the province, else the app's default. */
  srid: number | null;
  /** The scale chosen (1:N); null: the type's own. */
  plotScale: number | null;
  name: string;
  font: DrawingFont;
  /** The app's default system for new projects (Uygulama ayarları → Yeni projeler). */
  fallbackSrid: number;
}

/** Map scales for a CBS project, drawing scales for a CAD one (1:N). */
export const GIS_SCALES = [500, 1000, 2000, 5000, 10_000, 25_000, 50_000] as const;
export const CAD_SCALES = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000] as const;

/**
 * The plot scales a project's Ölçek offers (docs/adr/0205 §4): its type's (a CAD project's drawing scales, else the map
 * scales), with the current one in its place when it is none of them. The desktop's `wizard::project_scales`.
 */
export function projectScales(cad: boolean, current: number): number[] {
  const out: number[] = [...(cad ? CAD_SCALES : GIS_SCALES)];
  if (Number.isFinite(current) && current > 0 && !out.includes(current)) out.push(current), out.sort((a, b) => a - b);
  return out;
}

/** The units a local project is drawn in, with what each is for. */
export const UNITS: readonly { id: DrawingUnit; name: string; mark: string; note: string }[] = [
  { id: 'mm', name: 'Milimetre', mark: 'mm', note: 'Makine, detay ve imalat çizimleri' },
  { id: 'cm', name: 'Santimetre', mark: 'cm', note: 'İç mekân, mobilya ve doğrama' },
  { id: 'm', name: 'Metre', mark: 'm', note: 'Mimari plan, vaziyet ve altyapı' },
];

/** A new wizard's draft: the app's last type, default system and typeface. */
export function initialDraft(o: { type: Workspace; fallbackSrid: number; font: DrawingFont; unit?: DrawingUnit }): WizardDraft {
  return {
    type: o.type === 'cad' ? 'cad' : 'gis',
    coords: 'local',
    unit: o.unit ?? 'm',
    province: null,
    srid: null,
    plotScale: null,
    name: NEW_PROJECT_NAME,
    font: o.font,
    fallbackSrid: crsBySrid(o.fallbackSrid) && o.fallbackSrid !== LOCAL_SRID ? o.fallbackSrid : 5256,
  };
}

/** Whether the project is a drawing of its own, with no coordinate system. */
export const isLocalDraft = (d: WizardDraft): boolean => d.type === 'cad' && d.coords === 'local';

/** The province chosen, if any. */
export const draftProvince = (d: WizardDraft): Province | null => (d.province === null ? null : (provinceByCode(d.province) ?? null));

/** The TUREF zone of the province's longitude, if a province is chosen. */
export function suggestedSrid(d: WizardDraft): number | null {
  const p = draftProvince(d);
  return p ? (turefZoneFor(p.lon)?.srid ?? null) : null;
}

/** The project's system: the local one, or the one chosen, suggested or defaulted. */
export function draftSrid(d: WizardDraft): number {
  return isLocalDraft(d) ? LOCAL_SRID : (d.srid ?? suggestedSrid(d) ?? d.fallbackSrid);
}

/** The scales offered for the draft. */
export const draftScales = (d: WizardDraft): readonly number[] => (d.type === 'cad' ? CAD_SCALES : GIS_SCALES);

/** The project's scale: the one chosen, else 1:1 for a local drawing and 1:1000 for any other. */
export const draftScale = (d: WizardDraft): number => d.plotScale ?? (isLocalDraft(d) ? 1 : 1000);

/** The project's unit: a local drawing's own, metres for any other. */
export const draftUnit = (d: WizardDraft): DrawingUnit => (isLocalDraft(d) ? d.unit : 'm');

/** The systems a project with coordinates chooses from: the suggested one first, then every other in the registry's order. */
export function systemChoices(d: WizardDraft): { crs: CrsDef; suggested: boolean }[] {
  const s = suggestedSrid(d);
  const all = CRS_REGISTRY.filter((c) => c.srid !== LOCAL_SRID);
  const first = all.filter((c) => c.srid === s);
  return [...first.map((crs) => ({ crs, suggested: true })), ...all.filter((c) => c.srid !== s).map((crs) => ({ crs, suggested: false }))];
}

/** A scale as written: 1:25.000 with Turkish digit groups, as the ribbon's scale field. */
export const scaleText = (n: number): string => `1:${n.toLocaleString('tr-TR')}`;

/** What the rail writes under a step's name: the choice made there. */
export function stepNote(d: WizardDraft, step: WizardStep): string {
  if (step === 'type') return d.type === 'cad' ? 'CAD · teknik çizim' : 'CBS · harita';
  if (step === 'coords') {
    if (isLocalDraft(d)) return `Yerel · ${UNITS.find((u) => u.id === d.unit)?.mark ?? d.unit}`;
    const crs = crsBySrid(draftSrid(d));
    const p = draftProvince(d);
    return [p?.name, crs?.name].filter(Boolean).join(' · ');
  }
  return `${scaleText(draftScale(d))} · ${d.name.trim() || NEW_PROJECT_NAME}`;
}

/** The layers the project starts with, as the summary names them. */
const LAYERS = {
  cad: 'Teknik çizim: Çizim, Ölçü, Yazı, Tarama, Yardımcı, Eksen',
  gis: 'Kadastro paftası: Taslak, Kadastro, Ulaşım, Topografya, Jeodezi, Pafta',
};

/** The summary of the last step, line by line. */
export function summary(d: WizardDraft): [string, string][] {
  const local = isLocalDraft(d);
  const crs = crsBySrid(draftSrid(d));
  const p = draftProvince(d);
  const lines: [string, string][] = [
    ['Tür', d.type === 'cad' ? 'CAD, teknik çizim' : 'CBS, coğrafi bilgi sistemi'],
    ['Koordinatlar', local ? 'Yerel: koordinat sistemi yok, çizim 0,0’dan başlar' : crs ? crsTitle(crs) : `EPSG:${draftSrid(d)}`],
    ['Birim', (UNITS.find((u) => u.id === draftUnit(d))?.name ?? 'Metre').toLocaleLowerCase('tr-TR')],
    // The type's axes and angles (docs/adr/0165 §4).
    ['Eksenler', d.type === 'cad' ? 'X sağa, Y yukarı; açı derece, doğudan saat yönünün tersine' : 'Y doğuya, X kuzeye; semt grad, kuzeyden saat yönünde'],
  ];
  if (!local && p) lines.push(['İl', p.name]);
  lines.push(['Ölçek', scaleText(draftScale(d))], ['Katmanlar', LAYERS[d.type]]);
  lines.push(['Açılış', local ? 'A3 kâğıt, yatay; 0,0 sol altta' : p ? `${p.name} merkezinde bir pafta` : 'Dilimin çalışma alanında bir pafta']);
  return lines;
}

/** Why the step cannot be left forward, or null. */
export function blocked(d: WizardDraft, step: WizardStep): string | null {
  if (step === 'details' && !d.name.trim()) return 'Proje adı boş olamaz.';
  if (step === 'coords' && !isLocalDraft(d) && !crsBySrid(draftSrid(d))) return `EPSG:${draftSrid(d)} bu sürümde tanımlı değil; listeden bir sistem seçin.`;
  return null;
}

/** The project the draft makes. */
export function optionsOf(d: WizardDraft): NewProjectOptions {
  const local = isLocalDraft(d);
  return {
    name: d.name,
    srid: draftSrid(d),
    plotScale: draftScale(d),
    workspace: d.type,
    drawingFont: d.font,
    ...(!local && d.province !== null ? { province: d.province } : {}),
    ...(local ? { drawingUnit: d.unit } : {}),
  };
}
