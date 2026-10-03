import '../../styles/wizard.css';
import type { AppContext } from '../../app/context';
import { newProjectContent } from '../../model/newProject';
import { blocked, initialDraft, isLocalDraft, optionsOf, STEP_NAMES, stepNote, WIZARD_STEPS, type WizardDraft, type WizardStep } from '../../model/newProjectWizard';
import { h, replaceChildren } from '../dom';
import { icon } from '../icons';
import { note } from '../widgets/controls';
import { Dialog } from '../widgets/Dialog';
import { newProjectNote } from './newProjectNote';
import { coordsStep, detailsStep, typeStep, type StepApi } from './wizardSteps';

/**
 * Dosya → Yeni proje as a wizard (docs/adr/0165 §3, DESIGN.md §7.10): the project's type, its coordinates, its scale
 * and details, one page at a time beside a rail of the steps and what was chosen in each. Nothing changes until
 * Oluştur; unsaved changes of the open drawing are asked about then, over the wizard, whose Vazgeç comes back here.
 * The type chosen, and a local project's unit, are remembered for the next wizard (Uygulama ayarları → Yeni projeler).
 */
export function openNewProjectWizard(ctx: AppContext): void {
  new NewProjectWizard(ctx);
}

class NewProjectWizard {
  private readonly ctx: AppContext;
  private draft: WizardDraft;
  private step: WizardStep = 'type';
  private readonly keep = { query: '' };
  private readonly rail = h('nav', { class: 'wiz__rail', 'aria-label': 'Adımlar' });
  private readonly page = h('div', { class: 'wiz__page' });
  private readonly status = h('p', { class: 'wiz__status', role: 'alert' });
  private readonly back = h('button', { class: 'btn btn--ghost', type: 'button' }, 'Geri');
  private readonly forward = h('button', { class: 'btn btn--primary', type: 'button' }, 'İleri');
  private readonly dialog: Dialog;
  private readonly api: StepApi;
  private creating = false;

  constructor(ctx: AppContext) {
    this.ctx = ctx;
    this.draft = initialDraft({
      type: ctx.prefs.defaultWorkspace.value,
      fallbackSrid: ctx.prefs.defaultSrid.value,
      font: ctx.prefs.defaultDrawingFont.value,
      unit: ctx.prefs.defaultDrawingUnit.value,
    });
    const cancel = h('button', { class: 'btn', type: 'button' }, 'Vazgeç');
    this.dialog = new Dialog({
      title: 'Yeni proje',
      width: 1040,
      className: 'dialog--wizard',
      content: [h('div', { class: 'wiz' }, this.rail, this.page)],
      footer: [this.back, h('div', { class: 'dialog__spacer' }), this.status, cancel, this.forward],
    });
    this.back.addEventListener('click', () => this.go(WIZARD_STEPS.indexOf(this.step) - 1));
    this.forward.addEventListener('click', () => this.next());
    cancel.addEventListener('click', () => this.dialog.close());
    const draft = () => this.draft;
    this.api = {
      get draft() {
        return draft();
      },
      set: (patch, redraw = true) => {
        this.draft = { ...this.draft, ...patch };
        this.status.textContent = '';
        if (redraw) this.renderPage(true);
        this.renderRail();
      },
      next: () => this.next(),
      keep: this.keep,
    };
    this.render();
    this.page.querySelector<HTMLElement>('[aria-checked="true"]')?.focus();
  }

  /** The step at `index`, if every step before it can be left. */
  private go(index: number): void {
    const to = WIZARD_STEPS[index];
    if (!to) return;
    for (const s of WIZARD_STEPS.slice(0, index)) {
      const why = blocked(this.draft, s);
      if (why) {
        this.step = s;
        this.render();
        this.status.textContent = why;
        return;
      }
    }
    this.step = to;
    this.status.textContent = '';
    this.render();
    const first = this.page.querySelector<HTMLElement>('[aria-checked="true"], .wiz__name, .wiz__search');
    first?.focus();
    // The name opens with its text selected: typing replaces it.
    if (first instanceof HTMLInputElement && first.classList.contains('wiz__name')) first.select();
  }

  private next(): void {
    const why = blocked(this.draft, this.step);
    if (why) {
      this.status.textContent = why;
      return;
    }
    const at = WIZARD_STEPS.indexOf(this.step);
    if (at < WIZARD_STEPS.length - 1) this.go(at + 1);
    else void this.create();
  }

  private async create(): Promise<void> {
    const { ctx } = this;
    if (this.creating) return;
    if (ctx.files.busy.value) {
      this.status.textContent = 'Bir dosya işlemi sürüyor; bitince yeniden deneyin.';
      return;
    }
    let content: ReturnType<typeof newProjectContent>;
    try {
      content = newProjectContent(optionsOf(this.draft));
    } catch (e) {
      this.status.textContent = (e as Error).message;
      return;
    }
    this.creating = true;
    this.forward.disabled = true;
    const done = await ctx.files.newProject(content);
    this.creating = false;
    this.forward.disabled = false;
    if (!done) return;
    // The next wizard starts on this type, and a local project's unit.
    ctx.settingsStore.choose({ 'newProjects.workspace': this.draft.type, ...(isLocalDraft(this.draft) ? { 'newProjects.drawingUnit': this.draft.unit } : {}) });
    this.dialog.close();
  }

  private render(): void {
    this.renderRail();
    this.renderPage(false);
    const last = this.step === WIZARD_STEPS[WIZARD_STEPS.length - 1];
    this.forward.textContent = last ? 'Oluştur' : 'İleri';
    this.back.hidden = this.step === WIZARD_STEPS[0];
  }

  private renderRail(): void {
    const at = WIZARD_STEPS.indexOf(this.step);
    replaceChildren(
      this.rail,
      WIZARD_STEPS.map((s, i) => {
        const b = h(
          'button',
          { class: 'wiz__step', type: 'button', 'aria-current': s === this.step ? 'step' : null, dataset: i < at ? { done: '' } : {} },
          h('span', { class: 'wiz__num', 'aria-hidden': 'true' }, i < at ? icon('check', 12) : String(i + 1)),
          h('span', { class: 'wiz__sname' }, STEP_NAMES[s]),
          h('span', { class: 'wiz__snote' }, stepNote(this.draft, s)),
        );
        b.addEventListener('click', () => this.go(i));
        return b;
      }),
    );
  }

  /** The step's page; `keep` keeps the page's scroll and the focused control's place when it is drawn again. */
  private renderPage(keep: boolean): void {
    const top = this.page.scrollTop;
    const focused = keep ? focusKey(this.page) : null;
    const parts =
      this.step === 'type'
        ? typeStep(this.api)
        : this.step === 'coords'
          ? coordsStep(this.api)
          : detailsStep(this.api, this.current());
    replaceChildren(this.page, parts);
    this.page.dataset.step = this.step;
    if (keep) {
      this.page.scrollTop = top;
      if (focused) this.page.querySelector<HTMLElement>(focused)?.focus();
    }
  }

  /** What becomes of the drawing on screen, said before anything is done (newProjectNote.ts). */
  private current() {
    const { ctx } = this;
    const cloud = ctx.cloud.project.value;
    const said = newProjectNote({
      name: ctx.doc.name.value,
      cloud: cloud && { name: cloud.name, autosaves: ctx.cloud.autosaves(), database: cloud.storage === 'database' },
      dirty: ctx.doc.dirty.value,
    });
    return said && note(said.tone, said.text);
  }
}

/** A selector for the focused control, so that a page drawn again focuses its new copy. */
function focusKey(root: HTMLElement): string | null {
  const a = document.activeElement;
  if (!(a instanceof HTMLElement) || !root.contains(a)) return null;
  if (a.dataset.code) return `[data-code="${a.dataset.code}"]`;
  if (a.dataset.srid) return `[data-srid="${a.dataset.srid}"]`;
  if (a.dataset.id) return `[data-id="${a.dataset.id}"]`;
  if (a.classList.contains('wiz__search')) return '.wiz__search';
  if (a.getAttribute('aria-checked') === 'true') return '[aria-checked="true"]';
  return null;
}
