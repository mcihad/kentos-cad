import { draftScale, draftSrid, draftUnit, initialDraft, optionsOf, stepNote, summary, suggestedSrid, systemChoices, WIZARD_STEPS, type WizardDraft } from './newProjectWizard';

/**
 * The wizard's rules on a few drafts, as a versioned file shared with the desktop (fixtures/project/v1/wizard.json,
 * docs/adr/0165 §3): each case's draft and what the wizard makes of it, the system, scale and unit, the rail's notes,
 * the summary, the suggested system first in the list and the project's options. Built by the recorder
 * (scripts/fixtures/record-new-project.test.ts), compared by newProjectWizard.test.ts; the desktop's
 * kentos_project::wizard reads the same file.
 */

const base = initialDraft({ type: 'gis', fallbackSrid: 5256, font: 'barlow' });

export const WIZARD_CASES: readonly { name: string; draft: WizardDraft }[] = [
  { name: 'CBS, il yok', draft: base },
  { name: 'CBS, İzmir', draft: { ...base, province: 35 } },
  { name: 'CBS, Van, ED50 seçilmiş, 1:5000', draft: { ...base, province: 65, srid: 2324, plotScale: 5000, name: 'Van kadastro' } },
  { name: 'CAD, yerel, mm', draft: { ...base, type: 'cad', unit: 'mm', name: 'Mil plakası' } },
  { name: 'CAD, yerel, cm, 1:20', draft: { ...base, type: 'cad', unit: 'cm', plotScale: 20 } },
  { name: 'CAD, gerçek koordinatlı, Trabzon', draft: { ...base, type: 'cad', coords: 'real', province: 61, name: '  Aplikasyon  ' } },
  { name: 'CAD, gerçek koordinatlı, il yok, UTM', draft: { ...base, type: 'cad', coords: 'real', srid: 32636, font: 'plex-mono' } },
];

export function wizardFixture() {
  return {
    format: 'kentos.new-project-wizard',
    version: 1,
    source: 'src/model/newProjectWizard.ts',
    cases: WIZARD_CASES.map(({ name, draft }) => ({
      name,
      draft,
      srid: draftSrid(draft),
      suggested: suggestedSrid(draft),
      scale: draftScale(draft),
      unit: draftUnit(draft),
      notes: WIZARD_STEPS.map((s) => stepNote(draft, s)),
      summary: summary(draft),
      firstSystem: systemChoices(draft)[0]?.crs.srid ?? null,
      options: optionsOf(draft),
    })),
  };
}
