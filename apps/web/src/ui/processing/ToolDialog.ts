import type { AppContext } from '../../app/context';
import { DisposableStore } from '../../core/disposable';
import type { Vec2 } from '../../model/geometry';
import type { ProcessingModel } from '../../processing/model';
import { MODEL_PREFIX, modelAsTool, runModel } from '../../processing/modelRunner';
import type { RunOptions, RunOutcome } from '../../processing/runner';
import type { FeaturesValue, ProcessingTool } from '../../processing/types';
import { PickObjectsTool } from '../../tools/pickObjectsTool';
import { PickPointTool } from '../../tools/pickPointTool';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import {
  attempted,
  chosenTarget,
  dialogFrame,
  dialogForm,
  edited,
  effectiveChoice,
  finished,
  hostEnv,
  modelSteps,
  startState,
  picked,
  pickedChoice,
  pickedObjects,
  progressed,
  resetState,
  rowForm,
  started,
  toggledAdvanced,
  undone,
  type DialogFrame,
  type DialogState,
  type SideForm,
  type ViewEnv,
} from './dialogPlan';
import { DIALOG_TEXTS as T } from './dialogTexts';
import { paramControl, type FieldEnv } from './paramFields';

/**
 * The dialog of one processing tool, generated from its definition: the
 * form on the left (Girdi, Ayarlar, Çıktı, Gelişmiş), what the tool does
 * and a live preview on the right, run status in the footer. It stays open
 * after a run so the user can adjust and run again, like QGIS. What it
 * shows and how its state changes are dialogPlan's; this file draws them.
 */

export function openToolDialog(ctx: AppContext, toolId: string, values?: Record<string, unknown>): void {
  if (toolId.startsWith(MODEL_PREFIX)) return openModelDialog(ctx, toolId.slice(MODEL_PREFIX.length), values);
  const tool = ctx.processing.registry.get(toolId);
  if (!tool) {
    ctx.log.error(`İşlem aracı bulunamadı: ${toolId}`);
    return;
  }
  new ToolDialog(ctx, tool, values);
}

/** A model's run dialog: its inputs as the form, its steps on the side; it runs step by step. */
export function openModelDialog(ctx: AppContext, modelId: string, values?: Record<string, unknown>): void {
  const model = ctx.processing.model(modelId);
  if (!model) {
    ctx.log.error(`Model bulunamadı: ${modelId}. Silinmiş olabilir.`);
    return;
  }
  const lookup = (id: string) => ctx.processing.registry.get(id);
  new ToolDialog(ctx, modelAsTool(model, lookup), values, { model, run: (v, opts) => runModel(model, v, ctx.processing.runner, lookup, opts) });
}

interface DialogOptions {
  /** Set for a model: the side panel lists its steps. */
  model?: ProcessingModel;
  /** Runs the subject; a tool runs through the runner by default. */
  run?(values: Record<string, unknown>, opts: RunOptions): Promise<RunOutcome>;
  /** The state to go on from: the dialog coming back after a point was shown on the map. */
  resume?: DialogState;
}

class ToolDialog {
  private readonly ctx: AppContext;
  private readonly tool: ProcessingTool;
  private readonly opts: DialogOptions;
  /** A model's step tools: "Nerede çalışır" offers every place one of them can go. */
  private readonly modelTools?: readonly ProcessingTool[];
  private state: DialogState;
  /** What the values read on the drawing now: problems, inputs, places (recomputed on each full render). */
  private env!: ViewEnv;

  private readonly d = new DisposableStore();
  private readonly dialog: Dialog;
  private readonly form = h('div', { class: 'ptool__form' });
  private readonly preview = h('div', { class: 'ptool__preview-value num' });
  private readonly statusEl = h('div', { class: 'ptool__status', role: 'status', 'aria-live': 'polite' });
  private readonly targetsEl = h('div', { class: 'ptool__targets', role: 'radiogroup', 'aria-label': T.side.targets });
  private readonly runBtn: HTMLButtonElement;
  private readonly closeBtn: HTMLButtonElement;
  private readonly resetBtn: HTMLButtonElement;

  constructor(ctx: AppContext, tool: ProcessingTool, values?: Record<string, unknown>, opts: DialogOptions = {}) {
    this.ctx = ctx;
    this.tool = tool;
    this.opts = opts;
    const { runner, registry } = ctx.processing;
    this.modelTools = opts.model ? opts.model.steps.flatMap((s) => registry.get(s.tool) ?? []) : undefined;
    this.state = opts.resume ?? startState(tool, values, ctx.processing.lastValues(tool.id), runner.defaults(), ctx.processing.targetChoice(tool.id));

    this.resetBtn = h('button', { class: 'btn btn--ghost', type: 'button', title: T.footer.resetTitle }, T.footer.reset);
    this.closeBtn = h('button', { class: 'btn', type: 'button' }, T.footer.close);
    this.runBtn = h('button', { class: 'btn btn--primary ptool__run', type: 'button' }, icon('play', 14), T.footer.run);
    this.resetBtn.addEventListener('click', () => {
      this.state = resetState(this.state, tool, runner.defaults());
      this.render();
    });
    this.closeBtn.addEventListener('click', () => (this.state.status.kind === 'running' ? runner.cancel() : this.dialog.close()));
    this.runBtn.addEventListener('click', () => void this.run());

    const lookup = (id: string) => registry.get(id);
    const form = dialogForm(tool, { categoryPath: (id) => registry.categoryPath(id), categoryIcon: (id) => registry.category(id)?.icon }, opts.model ? { steps: modelSteps(opts.model, lookup), builtin: ctx.processing.isBuiltinModel(opts.model.id) } : undefined);
    this.dialog = new Dialog({
      title: form.title,
      width: 940,
      className: 'dialog--ptool',
      content: [h('div', { class: 'ptool' }, h('div', { class: 'ptool__main' }, this.form), this.side(form.side))],
      footer: [this.resetBtn, this.statusEl, this.closeBtn, this.runBtn],
      onClose: () => this.d.dispose(),
    });
    // Enter in a text field runs the tool; Ctrl+Enter runs from anywhere.
    this.form.addEventListener('keydown', (e) => {
      const t = e.target as HTMLElement;
      if (e.key === 'Enter' && (e.ctrlKey || (t.tagName === 'INPUT' && !e.shiftKey))) {
        e.preventDefault();
        void this.run();
      }
    });
    this.d.add(
      runner.running.subscribe((r) => {
        if (!r || r.toolId !== tool.id || this.state.status.kind !== 'running') return;
        this.state = progressed(this.state, r.fraction, r.label);
        this.paintStatus(this.frame());
      }),
    );
    this.render();
    // Start where the user most likely acts: the first text or number field.
    queueMicrotask(() => this.form.querySelector<HTMLElement>('input, .seg [aria-checked="true"]')?.focus());
  }

  private envNow(): ViewEnv {
    return hostEnv(this.ctx.processing.runner, this.tool, this.state.values, this.ctx.doc.layers, (p) => this.ctx.format.point(p), this.modelTools);
  }

  private frame(): DialogFrame {
    return dialogFrame(this.tool, this.state, this.env);
  }

  // ── Rendering ────────────────────────────────────────────────────────

  private render(): void {
    this.env = this.envNow();
    const frame = this.frame();

    // Keep keyboard focus on the same control across the rebuild.
    const active = document.activeElement as HTMLElement | null;
    const row = active && this.form.contains(active) ? active.closest<HTMLElement>('[data-param]') : null;
    const focusName = row?.dataset.param;
    const focusIndex = row ? focusables(row).indexOf(active!) : -1;

    const adv = frame.sections.advanced;
    let advanced: HTMLElement | null = null;
    if (adv) {
      const toggle = h(
        'button',
        { class: 'pgroup__toggle', type: 'button', 'aria-expanded': String(adv.open) },
        icon(adv.open ? 'chevronDown' : 'chevronRight', 14),
        T.sections.advanced,
        h('span', { class: 'pgroup__count' }, String(adv.rows.length)),
      );
      toggle.addEventListener('click', () => {
        this.state = toggledAdvanced(this.state);
        this.render();
      });
      advanced = h('section', { class: 'pgroup pgroup--advanced' }, toggle, adv.open ? h('div', { class: 'pgroup__rows' }, adv.rows.map((n) => this.row(n))) : null);
    }
    replaceChildren(
      this.form,
      frame.sections.groups.map((g) => h('section', { class: 'pgroup' }, h('div', { class: 'pgroup__title' }, g.title), h('div', { class: 'pgroup__rows' }, g.rows.map((n) => this.row(n))))),
      advanced,
    );

    if (focusName) {
      const again = this.form.querySelector<HTMLElement>(`[data-param="${CSS.escape(focusName)}"]`);
      const list = again ? focusables(again) : [];
      (list[Math.max(0, Math.min(focusIndex, list.length - 1))] as HTMLElement | undefined)?.focus();
    }
    this.paint(frame);
    this.paintTargets(frame);
  }

  /** What changes as the user types: preview, problems under fields, status and footer. */
  private paint(frame: DialogFrame): void {
    const p = frame.preview;
    if (p) {
      this.preview.textContent = p.text;
      this.preview.toggleAttribute('data-muted', p.muted);
    }
    for (const row of this.form.querySelectorAll<HTMLElement>('[data-param]')) {
      const issue = frame.issues[row.dataset.param!];
      row.toggleAttribute('data-invalid', issue !== undefined);
      const slot = row.querySelector('.prow__issue')!;
      if (issue !== undefined) replaceChildren(slot, icon('error', 14), h('span', null, issue));
      else slot.replaceChildren();
    }
    this.paintStatus(frame);
  }

  private paintStatus(frame: DialogFrame): void {
    const { status: s, footer } = frame;
    this.runBtn.disabled = footer.run.disabled;
    replaceChildren(this.runBtn, icon('play', 14), footer.run.label);
    this.closeBtn.textContent = footer.close;
    this.resetBtn.disabled = footer.reset.disabled;
    const content: Child[] = [];
    if (s.kind === 'running') content.push(h('div', { class: 'ptool__progress' }, h('span', { style: `width:${s.progress ?? 0}%` })));
    else if (s.icon) content.push(icon(s.icon, 16));
    if (s.text) content.push(h('span', { class: 'ptool__status-text', title: s.kind === 'running' ? null : s.text }, s.text));
    for (const a of s.actions) {
      const b = h('button', { class: 'btn btn--ghost btn--small', type: 'button' }, T.actions[a]);
      b.addEventListener('click', () => this.act(a, s.pick ?? []));
      content.push(b);
    }
    this.statusEl.dataset.kind = s.kind;
    replaceChildren(this.statusEl, content);
  }

  /** The status line's buttons after a run. */
  private act(action: 'zoom' | 'select' | 'undo', pick: readonly number[]): void {
    if (action === 'undo') {
      this.ctx.commands.execute('edit.undo');
      this.state = undone(this.state);
      this.render();
      return;
    }
    // A selection result is already applied: look at it; otherwise select what the run made or changed.
    if (action === 'select') this.ctx.selection.set(pick);
    this.dialog.close();
    if (this.ctx.selection.size) this.ctx.view.zoomToSelection();
  }

  /**
   * Nerede çalışır: Otomatik (and what it picks for these inputs now), then
   * each place the tool declares. Places without an executor here are
   * listed as coming, so the user sees what the tool will be able to do.
   */
  private paintTargets(frame: DialogFrame): void {
    const t = frame.targets;
    replaceChildren(
      this.targetsEl,
      t.options.map((o, i) => {
        const b = h(
          'button',
          { class: 'ptool__target', type: 'button', role: 'radio', 'aria-checked': String(o.checked), disabled: o.disabled, tabindex: o.checked ? '0' : '-1' },
          h('span', { class: 'ptool__radio' }),
          h('span', { class: 'ptool__target-label' }, o.label),
          o.note ? h('span', { class: 'ptool__target-note' }, o.note) : null,
        );
        b.addEventListener('click', () => {
          this.state = chosenTarget(this.state, o.value);
          this.ctx.processing.setTargetChoice(this.tool.id, o.value);
          this.paintTargets(this.frame());
        });
        // The hint about Otomatik sits under it.
        return i === 0 && t.hint ? [b, h('div', { class: 'ptool__target-hint' }, t.hint)] : b;
      }),
    );
  }

  private row(name: string): HTMLElement {
    const def = this.tool.parameters.find((p) => p.name === name)!;
    const f = rowForm(def);
    const runner = this.ctx.processing.runner;
    const env: FieldEnv = {
      ctx: this.ctx,
      describe: (n) => this.env.inputs[n],
      previewExpression: (n) => runner.previewExpression(this.tool, this.state.values, n),
      pickPoint: (n) => this.pickPoint(n),
      pickChoice: (n) => this.pickChoice(n),
      pickObjects: (n) => this.pickObjects(n),
    };
    const control = paramControl(def, this.state.values[name], (v, rebuild) => this.set(name, v, rebuild), env);
    return h(
      'div',
      { class: `prow${f.stacked ? ' prow--stacked' : ''}`, 'data-param': name },
      h(
        'div',
        { class: 'prow__text' },
        h('div', { class: 'prow__label' }, f.label, f.optional ? h('span', { class: 'prow__opt' }, T.optional) : null),
        f.description ? h('div', { class: 'prow__desc' }, f.description) : null,
      ),
      h('div', { class: 'prow__control' }, control, h('div', { class: 'prow__issue', role: 'alert' })),
    );
  }

  private side(side: SideForm): HTMLElement {
    const model = this.opts.model;
    let edit: HTMLElement | null = null;
    if (model && side.edit) {
      edit = h('button', { class: 'btn btn--small', type: 'button' }, icon('edit', 14), side.edit);
      edit.addEventListener('click', () => {
        this.dialog.close();
        this.ctx.commands.execute('processing.newModel', model.id);
      });
    }
    return h(
      'aside',
      { class: 'ptool__side' },
      h('div', { class: 'ptool__crumb' }, icon(side.crumb.icon, 14), side.crumb.text),
      h('div', { class: 'ptool__heading' }, h('span', { class: 'ptool__icon' }, icon(side.icon, 20)), h('p', { class: 'ptool__about' }, side.about)),
      side.help.map((p) => h('p', { class: 'ptool__help' }, p)),
      side.steps
        ? h(
            'div',
            { class: 'ptool__steps' },
            h('div', { class: 'ptool__side-title' }, T.side.steps),
            h('ol', null, side.steps.map((s) => h('li', null, h('span', { class: 'ptool__step-icon' }, icon(s.icon, 14)), s.name))),
            edit,
          )
        : null,
      side.preview ? h('div', { class: 'ptool__preview' }, h('div', { class: 'ptool__side-title' }, T.side.preview), this.preview) : null,
      h(
        'div',
        { class: 'ptool__facts' },
        h('div', { class: 'ptool__side-title' }, T.side.targets),
        this.targetsEl,
        side.aliases.length ? h('div', { class: 'ptool__side-title' }, T.side.aliases) : null,
        side.aliases.length ? h('div', { class: 'ptool__aliases' }, side.aliases.map((a) => h('code', null, a))) : null,
      ),
    );
  }

  // ── Editing and running ──────────────────────────────────────────────

  /** `rebuild`: a choice (the form is built again); otherwise typing (the field keeps focus, what depends on it is repainted). */
  private set(name: string, value: unknown, rebuild = true): void {
    const next = edited(this.state, name, value, !rebuild);
    if (next === this.state) return;
    this.state = next;
    if (rebuild) {
      this.render();
      return;
    }
    this.env = { ...this.env, issues: this.ctx.processing.runner.validate(this.tool, this.state.values) };
    this.paint(this.frame());
  }

  /** Hides the dialog while the user shows a point, then brings it back as it was, with the point filled in. */
  private pickPoint(name: string): void {
    const def = this.tool.parameters.find((p) => p.name === name)!;
    const state = this.state;
    const back = (p: Vec2 | null) => queueMicrotask(() => new ToolDialog(this.ctx, this.tool, undefined, { ...this.opts, resume: picked(state, name, p) }));
    this.dialog.close();
    this.ctx.tools.run(new PickPointTool(this.ctx, def.label, back), `${this.tool.label}: ${def.label}`);
  }

  /**
   * Sahneden seç beside a choice that picks a point (the numbering's start
   * vertex, docs/adr/0088): the point is shown as for its own field, and
   * the dialog comes back with the point and the choice on its option.
   */
  private pickChoice(name: string): void {
    const def = this.tool.parameters.find((p) => p.name === name);
    const picks = def?.type === 'enum' ? def.picks : undefined;
    const pointDef = picks && this.tool.parameters.find((p) => p.name === picks.point);
    if (!picks || !pointDef) return;
    const state = this.state;
    const back = (p: Vec2 | null) =>
      queueMicrotask(() => new ToolDialog(this.ctx, this.tool, undefined, { ...this.opts, resume: pickedChoice(state, name, picks.option, picks.point, p) }));
    this.dialog.close();
    this.ctx.tools.run(new PickPointTool(this.ctx, pointDef.label, back), `${this.tool.label}: ${pointDef.label}`);
  }

  /**
   * Sahneden seç for input objects (docs/adr/0088): the dialog steps aside
   * and the selection is put by and cleared; objects of the field's kinds
   * (the chosen kind chips, else the tool's) are clicked or boxed. Kept,
   * the field is the selection and the log says how many; left (Esc) or
   * with nothing picked, the selection before comes back.
   */
  private pickObjects(name: string): void {
    const def = this.tool.parameters.find((p) => p.name === name);
    if (def?.type !== 'features') return;
    const { selection, log } = this.ctx;
    const state = this.state;
    const kinds = (state.values[name] as FeaturesValue | undefined)?.kinds ?? def.kinds;
    const before = [...selection.ids.value];
    const back = (keep: boolean) =>
      queueMicrotask(() => {
        const count = keep ? selection.size : 0;
        if (count) log.info(T.pick.picked(count));
        else selection.set(before);
        new ToolDialog(this.ctx, this.tool, undefined, { ...this.opts, resume: pickedObjects(state, name, keep, count) });
      });
    this.dialog.close();
    selection.clear();
    this.ctx.tools.run(new PickObjectsTool(this.ctx, def.label, kinds, back), `${this.tool.label}: ${def.label}`);
  }

  private async run(): Promise<void> {
    if (this.state.status.kind === 'running') return;
    const { runner } = this.ctx.processing;
    const tried = attempted(this.state, this.tool, runner.validate(this.tool, this.state.values));
    this.state = tried.state;
    if (!tried.run) {
      this.render();
      this.focusFirstIssue();
      return;
    }
    const values = this.state.values;
    this.ctx.processing.remember(this.tool.id, values);
    const target = effectiveChoice(this.state.choice, this.env.targets.available) ?? 'auto';
    const where = this.opts.model ? undefined : runner.executorFor(this.tool, target, runner.inputSize(this.tool, values))?.target;
    this.state = started(this.state, where);
    this.paintStatus(this.frame());
    const log = (level: 'info' | 'warn', m: string) => (level === 'warn' ? this.ctx.log.warn(m) : this.ctx.log.info(m));
    const out: RunOutcome = this.opts.run ? await this.opts.run(values, { log, target }) : await runner.run(this.tool, values, { log, target });
    this.state = finished(this.state, out);
    if (out.status === 'ok') this.ctx.log.success(`${this.tool.label}: ${out.record.summary}`);
    else if (out.status !== 'invalid') this.ctx.log.warn(out.message);
    this.ctx.view.requestRender();
    this.render();
    // Stopped before running (e.g. nothing selected): the problem is on its field.
    if (out.status === 'invalid') this.focusFirstIssue();
    else this.runBtn.focus();
  }

  private focusFirstIssue(): void {
    const row = this.form.querySelector<HTMLElement>('[data-invalid]');
    row?.scrollIntoView({ block: 'nearest' });
    (row ? focusables(row)[0] : null)?.focus();
  }
}

const focusables = (el: HTMLElement) => [...el.querySelectorAll<HTMLElement>('input, button:not([disabled]), [tabindex="0"]')];
