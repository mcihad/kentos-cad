import type { AppContext } from '../../app/context';
import { DisposableStore } from '../../core/disposable';
import type { Vec2 } from '../../model/geometry';
import { defaultValues, isVisible, restoreValues, type ValidationIssue } from '../../processing/parameters';
import { orderSteps, stepName, type ProcessingModel } from '../../processing/model';
import { MODEL_PREFIX, modelAsTool, runModel } from '../../processing/modelRunner';
import { WORKER_THRESHOLD, type InputSummary, type RunOptions, type RunOutcome, type TargetChoice } from '../../processing/runner';
import type { ExecutionTarget, ParamDef, ProcessingTool } from '../../processing/types';
import { PickPointTool } from '../../tools/pickPointTool';
import { h, replaceChildren, type Child } from '../dom';
import { icon } from '../icons';
import { Dialog } from '../widgets/Dialog';
import { paramControl, type FieldEnv } from './paramFields';

/**
 * The dialog of one processing tool, generated from its definition: the
 * form on the left (Girdi, Ayarlar, Çıktı, Gelişmiş), what the tool does
 * and a live preview on the right, run status in the footer. It stays open
 * after a run so the user can adjust and run again, like QGIS.
 */

export const TARGET_LABEL: Record<ExecutionTarget, string> = {
  client: 'Bu tarayıcıda',
  worker: 'Arka planda (worker)',
  server: 'KentOS sunucusunda',
  postgis: 'PostGIS veritabanında',
};

/** Lower-case, for "şimdi: …" and history rows. */
export const TARGET_SHORT: Record<ExecutionTarget, string> = {
  client: 'bu tarayıcıda',
  worker: 'arka planda',
  server: 'sunucuda',
  postgis: 'PostGIS’te',
};

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
}

type Status =
  | { kind: 'idle' }
  | { kind: 'running'; fraction: number; label: string; where?: ExecutionTarget }
  /** `pick`: objects "Sonuçları seç" selects; `selected`: the run set the selection itself; `undo`: it edited the drawing. */
  | { kind: 'ok'; text: string; pick: readonly number[]; selected: boolean; undo: boolean }
  | { kind: 'error' | 'invalid'; text: string };

class ToolDialog {
  private readonly ctx: AppContext;
  private readonly tool: ProcessingTool;
  private values: Record<string, unknown>;
  private readonly touched = new Set<string>();
  /** After a run attempt every problem shows, not only the ones in fields the user touched. */
  private attempted = false;
  private advancedOpen = false;
  private issues: ValidationIssue[] = [];
  private inputs: Record<string, InputSummary> = {};
  private status: Status = { kind: 'idle' };

  private readonly d = new DisposableStore();
  private readonly dialog: Dialog;
  private readonly form = h('div', { class: 'ptool__form' });
  private readonly preview = h('div', { class: 'ptool__preview-value num' });
  private readonly statusEl = h('div', { class: 'ptool__status', role: 'status', 'aria-live': 'polite' });
  private readonly targetsEl = h('div', { class: 'ptool__targets', role: 'radiogroup', 'aria-label': 'Nerede çalışır' });
  private choice: TargetChoice;
  private readonly runBtn: HTMLButtonElement;
  private readonly closeBtn: HTMLButtonElement;

  private readonly opts: DialogOptions;

  constructor(ctx: AppContext, tool: ProcessingTool, values?: Record<string, unknown>, opts: DialogOptions = {}) {
    this.ctx = ctx;
    this.tool = tool;
    this.opts = opts;
    const runner = ctx.processing.runner;
    this.values = restoreValues(tool, values ?? ctx.processing.lastValues(tool.id), runner.defaults());
    this.choice = ctx.processing.targetChoice(tool.id);
    this.advancedOpen = tool.parameters.some((p) => p.advanced && values && p.name in values && JSON.stringify(values[p.name]) !== JSON.stringify(defaultValues(tool, runner.defaults())[p.name]));

    const reset = h('button', { class: 'btn btn--ghost', type: 'button', title: 'Bütün alanları varsayılan değerlerine döndürür' }, 'Varsayılanlar');
    this.closeBtn = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    this.runBtn = h('button', { class: 'btn btn--primary ptool__run', type: 'button' }, icon('play', 14), 'Çalıştır');
    reset.addEventListener('click', () => {
      this.values = defaultValues(tool, runner.defaults());
      this.touched.clear();
      this.attempted = false;
      this.status = { kind: 'idle' };
      this.render();
    });
    this.closeBtn.addEventListener('click', () => (this.status.kind === 'running' ? runner.cancel() : this.dialog.close()));
    this.runBtn.addEventListener('click', () => void this.run());

    this.dialog = new Dialog({
      title: tool.label,
      width: 940,
      className: 'dialog--ptool',
      content: [h('div', { class: 'ptool' }, h('div', { class: 'ptool__main' }, this.form), this.side())],
      footer: [reset, this.statusEl, this.closeBtn, this.runBtn],
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
        if (!r || r.toolId !== tool.id || this.status.kind !== 'running') return;
        this.status = { ...this.status, fraction: r.fraction, label: r.label };
        this.renderStatus();
      }),
    );
    this.render();
    // Start where the user most likely acts: the first text or number field.
    queueMicrotask(() => this.form.querySelector<HTMLElement>('input, .seg [aria-checked="true"]')?.focus());
  }

  // ── Rendering ────────────────────────────────────────────────────────

  private render(): void {
    const runner = this.ctx.processing.runner;
    this.issues = runner.validate(this.tool, this.values);
    this.inputs = runner.describeInputs(this.tool, this.values);

    // Keep keyboard focus on the same control across the rebuild.
    const active = document.activeElement as HTMLElement | null;
    const row = active && this.form.contains(active) ? active.closest<HTMLElement>('[data-param]') : null;
    const focusName = row?.dataset.param;
    const focusIndex = row ? focusables(row).indexOf(active!) : -1;

    const shown = this.tool.parameters.filter((p) => isVisible(p, this.values));
    const input = shown.filter((p) => !p.advanced && p.type === 'features');
    const output = shown.filter((p) => !p.advanced && p.type === 'layer');
    const main = shown.filter((p) => !p.advanced && p.type !== 'features' && p.type !== 'layer');
    const advanced = shown.filter((p) => p.advanced);
    const advancedIssue = advanced.some((p) => this.issueOf(p.name));

    const toggle = h(
      'button',
      { class: 'pgroup__toggle', type: 'button', 'aria-expanded': String(this.advancedOpen || advancedIssue) },
      icon(this.advancedOpen || advancedIssue ? 'chevronDown' : 'chevronRight', 14),
      'Gelişmiş ayarlar',
      h('span', { class: 'pgroup__count' }, String(advanced.length)),
    );
    toggle.addEventListener('click', () => {
      this.advancedOpen = !this.advancedOpen;
      this.render();
    });

    replaceChildren(
      this.form,
      input.length ? this.group('Girdi', input) : null,
      main.length ? this.group('Ayarlar', main) : null,
      output.length ? this.group('Çıktı', output) : null,
      advanced.length ? h('section', { class: 'pgroup pgroup--advanced' }, toggle, this.advancedOpen || advancedIssue ? h('div', { class: 'pgroup__rows' }, advanced.map((p) => this.row(p))) : null) : null,
    );

    if (focusName) {
      const again = this.form.querySelector<HTMLElement>(`[data-param="${CSS.escape(focusName)}"]`);
      const list = again ? focusables(again) : [];
      (list[Math.max(0, Math.min(focusIndex, list.length - 1))] as HTMLElement | undefined)?.focus();
    }
    this.renderPreview();
    this.renderStatus();
    this.renderTargets();
  }

  /**
   * Nerede çalışır: Otomatik (and what it picks for these inputs now), then
   * each place the tool declares. Places without an executor here are
   * listed as coming, so the user sees what the tool will be able to do.
   */
  private renderTargets(): void {
    const { runner, registry } = this.ctx.processing;
    const model = this.opts.model;
    // A model runs each step where it can: offer every place one of its steps can go.
    const tools = model ? model.steps.flatMap((s) => registry.get(s.tool) ?? []) : [this.tool];
    const available = new Set(tools.flatMap((t) => runner.executorsFor(t).map((e) => e.target)));
    const declared = model ? [...available] : this.tool.targets;
    const auto = model ? null : runner.executorFor(this.tool, 'auto', runner.inputSize(this.tool, this.values))?.target;
    const option = (value: TargetChoice, label: string, note: string | null, disabled = false) => {
      const checked = this.choice === value;
      const b = h(
        'button',
        { class: 'ptool__target', type: 'button', role: 'radio', 'aria-checked': String(checked), disabled, tabindex: checked ? '0' : '-1' },
        h('span', { class: 'ptool__radio' }),
        h('span', { class: 'ptool__target-label' }, label),
        note ? h('span', { class: 'ptool__target-note' }, note) : null,
      );
      b.addEventListener('click', () => {
        this.choice = value;
        this.ctx.processing.setTargetChoice(this.tool.id, value);
        this.renderTargets();
      });
      return b;
    };
    const several = available.size > 1;
    replaceChildren(
      this.targetsEl,
      several ? option('auto', 'Otomatik', auto ? `şimdi: ${TARGET_SHORT[auto]}` : model ? 'adım adım' : null) : null,
      several && this.choice === 'auto' ? h('div', { class: 'ptool__target-hint' }, `${WORKER_THRESHOLD.toLocaleString('tr-TR')} nesneden büyük işler arka planda çalışır; sayfa donmaz.`) : null,
      declared.map((t) =>
        available.has(t) ? option(t, TARGET_LABEL[t], !several ? 'bu çalıştırmada' : null) : option(t, TARGET_LABEL[t], 'yakında', true),
      ),
    );
  }

  private group(title: string, params: ParamDef[]): HTMLElement {
    return h('section', { class: 'pgroup' }, h('div', { class: 'pgroup__title' }, title), h('div', { class: 'pgroup__rows' }, params.map((p) => this.row(p))));
  }

  private row(def: ParamDef): HTMLElement {
    const env: FieldEnv = {
      ctx: this.ctx,
      describe: (name) => this.inputs[name],
      previewExpression: (name) => this.ctx.processing.runner.previewExpression(this.tool, this.values, name),
      pickPoint: (name) => this.pickPoint(name),
    };
    const control = paramControl(def, this.values[def.name], (v, rebuild) => this.set(def.name, v, rebuild), env);
    const stacked = def.type === 'features' || def.type === 'expression';
    return h(
      'div',
      { class: `prow${stacked ? ' prow--stacked' : ''}`, 'data-param': def.name },
      h(
        'div',
        { class: 'prow__text' },
        h('div', { class: 'prow__label' }, def.label, def.optional ? h('span', { class: 'prow__opt' }, 'isteğe bağlı') : null),
        def.description ? h('div', { class: 'prow__desc' }, def.description) : null,
      ),
      h('div', { class: 'prow__control' }, control, h('div', { class: 'prow__issue', role: 'alert' })),
    );
  }

  /** Issue shown under a field: live for touched fields, all of them after a run attempt. */
  private issueOf(name: string): ValidationIssue | undefined {
    if (!this.attempted && !this.touched.has(name)) return undefined;
    return this.issues.find((i) => i.param === name);
  }

  private renderIssues(): void {
    for (const row of this.form.querySelectorAll<HTMLElement>('[data-param]')) {
      const issue = this.issueOf(row.dataset.param!);
      row.toggleAttribute('data-invalid', !!issue);
      const slot = row.querySelector('.prow__issue')!;
      if (issue) replaceChildren(slot, icon('error', 14), h('span', null, issue.message));
      else slot.replaceChildren();
    }
  }

  private renderPreview(): void {
    const valid = !this.issues.some((i) => i.param);
    const text = valid ? this.tool.preview?.(this.values as never) : null;
    this.preview.textContent = text ?? (valid ? '' : 'Önizleme için alanları düzeltin.');
    this.preview.toggleAttribute('data-muted', !text);
    this.renderIssues();
  }

  private renderStatus(): void {
    const s = this.status;
    const running = s.kind === 'running';
    this.runBtn.disabled = running;
    replaceChildren(this.runBtn, icon('play', 14), running ? 'Çalışıyor…' : 'Çalıştır');
    this.closeBtn.textContent = running ? 'Durdur' : 'Kapat';
    const toolIssue = this.attempted ? this.issues.find((i) => !i.param) : undefined;
    const fieldIssues = this.attempted ? this.issues.filter((i) => i.param).length : 0;
    let content: Child[] = [];
    if (running) {
      content = [h('div', { class: 'ptool__progress' }, h('span', { style: `width:${Math.round(s.fraction * 100)}%` })), h('span', { class: 'ptool__status-text' }, s.label || (s.where === 'worker' ? 'Arka planda çalışıyor; sayfayı kullanmaya devam edebilirsiniz.' : 'Çalışıyor…'))];
    } else if (s.kind === 'ok') {
      // A selection result is already applied: offer to look at it; otherwise to select what changed.
      const show = s.selected || s.pick.length ? h('button', { class: 'btn btn--ghost btn--small', type: 'button' }, s.selected ? 'Seçime yakınlaştır' : 'Sonuçları seç') : null;
      show?.addEventListener('click', () => {
        if (!s.selected) this.ctx.selection.set(s.pick);
        this.dialog.close();
        if (this.ctx.selection.size) this.ctx.view.zoomToSelection();
      });
      const undo = s.undo ? h('button', { class: 'btn btn--ghost btn--small', type: 'button' }, 'Geri al') : null;
      undo?.addEventListener('click', () => {
        this.ctx.commands.execute('edit.undo');
        this.status = { kind: 'idle' };
        this.render();
      });
      content = [icon('success', 16), h('span', { class: 'ptool__status-text', title: s.text }, s.text), show, undo];
    } else if (fieldIssues || toolIssue || s.kind === 'invalid') {
      const text = toolIssue?.message ?? (fieldIssues ? `Çalıştırmadan önce ${fieldIssues} alanı düzeltin.` : s.kind === 'invalid' ? s.text : '');
      content = [icon('warning', 16), h('span', { class: 'ptool__status-text', title: text }, text)];
    } else if (s.kind === 'error') {
      content = [icon('error', 16), h('span', { class: 'ptool__status-text', title: s.text }, s.text)];
    }
    this.statusEl.dataset.kind = running ? 'running' : s.kind === 'ok' ? 'ok' : content.length ? (s.kind === 'error' ? 'error' : 'warn') : 'idle';
    replaceChildren(this.statusEl, content);
  }

  private side(): HTMLElement {
    const { registry } = this.ctx.processing;
    const cat = registry.category(this.tool.category);
    const model = this.opts.model;
    const help = model ? [] : (this.tool.help ?? '').split(/\n\s*\n/).filter(Boolean);
    return h(
      'aside',
      { class: 'ptool__side' },
      h('div', { class: 'ptool__crumb' }, icon(model ? 'processing' : (cat?.icon ?? 'processing'), 14), model ? `Modeller › ${registry.categoryPath(this.tool.category) || 'Genel'}` : registry.categoryPath(this.tool.category)),
      h('div', { class: 'ptool__heading' }, h('span', { class: 'ptool__icon' }, icon(this.tool.icon ?? 'processing', 20)), h('p', { class: 'ptool__about' }, this.tool.description)),
      help.map((p) => h('p', { class: 'ptool__help' }, p)),
      model ? this.modelSteps(model) : null,
      this.tool.preview ? h('div', { class: 'ptool__preview' }, h('div', { class: 'ptool__side-title' }, 'Önizleme'), this.preview) : null,
      h(
        'div',
        { class: 'ptool__facts' },
        h('div', { class: 'ptool__side-title' }, 'Nerede çalışır'),
        this.targetsEl,
        this.tool.aliases?.length ? h('div', { class: 'ptool__side-title' }, 'Komut satırından') : null,
        this.tool.aliases?.length ? h('div', { class: 'ptool__aliases' }, this.tool.aliases.map((a) => h('code', null, a))) : null,
      ),
    );
  }

  /** The steps of a model in run order, and the way into the designer. */
  private modelSteps(model: ProcessingModel): HTMLElement {
    const { registry } = this.ctx.processing;
    const lookup = (id: string) => registry.get(id);
    const ids = orderSteps(model);
    const order = Array.isArray(ids) ? ids.map((id) => model.steps.find((s) => s.id === id)!) : model.steps;
    const builtin = this.ctx.processing.isBuiltinModel(model.id);
    const edit = h('button', { class: 'btn btn--small', type: 'button' }, icon('edit', 14), builtin ? 'Kopyasını düzenle' : 'Modeli düzenle');
    edit.addEventListener('click', () => {
      this.dialog.close();
      this.ctx.commands.execute('processing.newModel', model.id);
    });
    return h(
      'div',
      { class: 'ptool__steps' },
      h('div', { class: 'ptool__side-title' }, 'Adımlar'),
      h(
        'ol',
        null,
        order.map((s) => h('li', null, h('span', { class: 'ptool__step-icon' }, icon(lookup(s.tool)?.icon ?? 'processing', 14)), stepName(s, lookup))),
      ),
      edit,
    );
  }

  // ── Editing and running ──────────────────────────────────────────────

  private set(name: string, value: unknown, rebuild = true): void {
    this.values = { ...this.values, [name]: value };
    this.touched.add(name);
    if (this.status.kind === 'ok' || this.status.kind === 'error' || this.status.kind === 'invalid') this.status = { kind: 'idle' };
    if (rebuild) {
      this.render();
      return;
    }
    // Typing: keep the field, refresh only what depends on it.
    this.issues = this.ctx.processing.runner.validate(this.tool, this.values);
    this.renderPreview();
    this.renderStatus();
  }

  /** Hides the dialog while the user shows a point, then opens it again with the point filled in. */
  private pickPoint(name: string): void {
    const def = this.tool.parameters.find((p) => p.name === name)!;
    const values = this.values;
    const reopen = (p: Vec2 | null) => queueMicrotask(() => openToolDialog(this.ctx, this.tool.id, p ? { ...values, [name]: p } : values));
    this.dialog.close();
    this.ctx.tools.run(new PickPointTool(this.ctx, def.label, reopen), `${this.tool.label}: ${def.label}`);
  }

  private async run(): Promise<void> {
    if (this.status.kind === 'running') return;
    const { runner } = this.ctx.processing;
    this.attempted = true;
    this.issues = runner.validate(this.tool, this.values);
    if (this.issues.length) {
      this.advancedOpen ||= this.issues.some((i) => this.tool.parameters.find((p) => p.name === i.param)?.advanced);
      this.render();
      this.focusFirstIssue();
      return;
    }
    this.ctx.processing.remember(this.tool.id, this.values);
    const where = this.opts.model ? undefined : runner.executorFor(this.tool, this.choice, runner.inputSize(this.tool, this.values))?.target;
    this.status = { kind: 'running', fraction: 0, label: '', where };
    this.renderStatus();
    const log = (level: 'info' | 'warn', m: string) => (level === 'warn' ? this.ctx.log.warn(m) : this.ctx.log.info(m));
    const out: RunOutcome = this.opts.run ? await this.opts.run(this.values, { log, target: this.choice }) : await runner.run(this.tool, this.values, { log, target: this.choice });
    switch (out.status) {
      case 'ok': {
        this.status = { kind: 'ok', text: out.record.summary, pick: out.added.length ? out.added : out.touched, selected: !!out.result.select, undo: out.edited };
        this.ctx.log.success(`${this.tool.label}: ${out.record.summary}`);
        this.attempted = false;
        break;
      }
      case 'invalid':
        this.issues = out.issues;
        this.status = { kind: 'invalid', text: out.issues[0]?.message ?? '' };
        break;
      default:
        this.status = { kind: 'error', text: out.message };
        this.ctx.log.warn(out.message);
    }
    this.ctx.view.requestRender();
    if (out.status === 'invalid') {
      // Stopped before running (e.g. nothing selected): show it on the field.
      this.renderIssues();
      this.renderStatus();
      this.focusFirstIssue();
      return;
    }
    this.render();
    this.runBtn.focus();
  }

  private focusFirstIssue(): void {
    const row = this.form.querySelector<HTMLElement>('[data-invalid]');
    row?.scrollIntoView({ block: 'nearest' });
    (row ? focusables(row)[0] : null)?.focus();
  }
}

const focusables = (el: HTMLElement) => [...el.querySelectorAll<HTMLElement>('input, button:not([disabled]), [tabindex="0"]')];
