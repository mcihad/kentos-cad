import type { AppContext } from '../../../app/context';
import type { Disposable } from '../../../core/disposable';
import type { Vec2 } from '../../../model/geometry';
import { checkModel, stepName, type ModelIssue, type ProcessingModel } from '../../../processing/model';
import { addInput, addStep, autoLayout, copyModel, newModel, setSource, sourcesFor } from '../../../processing/modelEdit';
import { PickPointTool } from '../../../tools/pickPointTool';
import { h, replaceChildren } from '../../dom';
import { icon } from '../../icons';
import { Dialog } from '../../widgets/Dialog';
import { PopupMenu, type MenuItem } from '../../widgets/PopupMenu';
import { openModelDialog } from '../ToolDialog';
import { ModelCanvas, type NodeRef } from './ModelCanvas';
import { renderInspector, type InspectorHost } from './modelInspector';
import { modelPalette } from './modelPalette';

/**
 * Model tasarımcısı (QGIS Model Designer): builds a model as a flow
 * diagram. Palette on the left (inputs, tools), the diagram in the middle,
 * the selected box's settings on the right. The designer edits a draft;
 * Kaydet writes it to the model library (this browser). It keeps its own
 * undo (Ctrl+Z) for the draft, separate from the drawing's.
 */

interface DesignerState {
  draft: ProcessingModel;
  /** JSON of the draft as last saved; null when never saved (dirty). */
  saved: string | null;
  builtinCopy: boolean;
  selected: NodeRef | null;
}

export function openModelDesigner(ctx: AppContext, modelId?: string): void {
  const source = modelId ? ctx.processing.model(modelId) : undefined;
  if (modelId && !source) {
    ctx.log.error(`Model bulunamadı: ${modelId}.`);
    return;
  }
  const builtin = !!source && ctx.processing.isBuiltinModel(source.id);
  const draft = source ? (builtin ? copyModel(source) : (JSON.parse(JSON.stringify(source)) as ProcessingModel)) : newModel();
  if (!draft.steps.every((s) => s.position) || !draft.inputs.every((i) => draft.inputPositions?.[i.name])) autoLayout(draft);
  new ModelDesigner(ctx, { draft, saved: source && !builtin ? JSON.stringify(draft) : null, builtinCopy: builtin, selected: null });
}

const HISTORY = 100;
/** Typing into the same field within this time is one undo step. */
const COALESCE_MS = 1200;

class ModelDesigner implements InspectorHost {
  readonly ctx: AppContext;
  model: ProcessingModel;
  problems: ModelIssue[] = [];
  private savedJson: string | null;
  readonly builtinCopy: boolean;
  private selected: NodeRef | null;
  private readonly past: string[] = [];
  private readonly future: string[] = [];
  private lastKey: { key: string; at: number } | null = null;
  /** Snapshot taken when a box drag starts; pushed as one undo step when it ends. */
  private dragStart: string | null = null;

  private readonly dialog: Dialog;
  private readonly canvas: ModelCanvas;
  private readonly inspector = h('aside', { class: 'mdesign__inspector', 'aria-label': 'Seçilen kutunun ayarları' });
  private readonly status = h('div', { class: 'ptool__status mdesign__status', role: 'status', 'aria-live': 'polite' });
  private readonly subs: Disposable[] = [];
  private confirming = false;

  constructor(ctx: AppContext, state: DesignerState) {
    this.ctx = ctx;
    this.model = state.draft;
    this.savedJson = state.saved;
    this.builtinCopy = state.builtinCopy;
    this.selected = state.selected;
    this.canvas = new ModelCanvas(
      {
        select: (ref) => this.select(ref),
        move: (ref, at, done) => this.move(ref, at, done),
        connect: (from, to, client) => this.connectMenu(from, to, client),
        open: (ref) => {
          this.select(ref);
          queueMicrotask(() => this.inspector.querySelector<HTMLElement>('input, button.dropdown')?.focus());
        },
      },
      (id) => this.lookup(id),
    );
    const palette = modelPalette(
      ctx.processing.registry,
      {
        addInput: (type, label) =>
          this.change(() => {
            const name = addInput(this.model, type, label, this.spotNear());
            this.selected = { kind: 'input', name };
          }),
        addTool: (toolId, client) => {
          const at = client ? this.canvas.worldAt(client) : null;
          if (client && !at) return; // dropped outside the diagram
          this.change(() => {
            const from = this.selected ?? undefined;
            const id = addStep(this.model, toolId, (x) => this.lookup(x), at ? { x: Math.round(at.x / 10) * 10, y: Math.round(at.y / 10) * 10 } : this.spotNear(), from);
            this.selected = { kind: 'step', id };
          });
        },
      },
      this.subs,
    );

    const layout = h('button', { class: 'btn btn--ghost', type: 'button', title: 'Kutuları bağlantı sırasına göre sütunlara dizer' }, icon('columns', 14), 'Düzenle');
    layout.addEventListener('click', () =>
      this.change(() => autoLayout(this.model), { after: () => queueMicrotask(() => this.canvas.fit()) }),
    );
    const close = h('button', { class: 'btn', type: 'button' }, 'Kapat');
    close.addEventListener('click', () => this.dialog.request());
    const saveRun = h('button', { class: 'btn', type: 'button' }, icon('play', 14), 'Kaydet ve çalıştır…');
    saveRun.addEventListener('click', () => {
      if (!this.save()) return;
      this.dialog.close();
      openModelDialog(ctx, this.model.id);
    });
    const save = h('button', { class: 'btn btn--primary', type: 'button' }, 'Kaydet');
    save.addEventListener('click', () => this.save());

    this.dialog = new Dialog({
      title: 'Model tasarımcısı',
      width: 1400,
      className: 'dialog--designer',
      content: [h('div', { class: 'mdesign' }, palette, this.canvas.el, this.inspector)],
      footer: [layout, this.status, close, saveRun, save],
      beforeClose: () => this.beforeClose(),
      onClose: () => {
        this.subs.forEach((d) => d());
        this.canvas.dispose();
      },
    });
    this.dialog.el.addEventListener('keydown', (e) => this.onKey(e));
    this.render();
    requestAnimationFrame(() => this.canvas.fit());
  }

  // ── InspectorHost ──────────────────────────────────────────────────────

  get saved(): boolean {
    return this.savedJson !== null && !!this.ctx.processing.model(this.model.id);
  }

  /** An arrow so the inspector can pass it around unbound. */
  readonly lookup = (id: string) => this.ctx.processing.registry.get(id);

  /** Every edit goes through here: snapshot for undo, change, redraw. */
  change(fn: () => void, opts: { rerender?: boolean; key?: string; after?: () => void } = {}): void {
    const now = performance.now();
    const joins = opts.key && this.lastKey?.key === opts.key && now - this.lastKey.at < COALESCE_MS;
    if (!joins) {
      this.past.push(JSON.stringify({ model: this.model, selected: this.selected }));
      if (this.past.length > HISTORY) this.past.shift();
      this.future.length = 0;
    }
    this.lastKey = opts.key ? { key: opts.key, at: now } : null;
    fn();
    if (opts.rerender === false) this.refresh();
    else this.render();
    opts.after?.();
  }

  select(ref: NodeRef | null): void {
    this.selected = ref;
    this.render();
  }

  /** Closes the designer while the user shows a point, then opens it again on the same draft. */
  pickPoint(stepId: string, param: string): void {
    const state: DesignerState = { draft: this.model, saved: this.savedJson, builtinCopy: this.builtinCopy, selected: { kind: 'step', id: stepId } };
    const def = this.lookup(this.model.steps.find((s) => s.id === stepId)?.tool ?? '')?.parameters.find((p) => p.name === param);
    this.dialog.close();
    const reopen = (p: Vec2 | null) =>
      queueMicrotask(() => {
        if (p) setSource(state.draft, stepId, param, { kind: 'value', value: p });
        new ModelDesigner(this.ctx, state);
      });
    this.ctx.tools.run(new PickPointTool(this.ctx, def?.label ?? 'Nokta', reopen), `Model tasarımcısı: ${def?.label ?? 'nokta'}`);
  }

  deleteModel(): void {
    this.showConfirm(`“${this.model.label}” modeli silinsin mi? Bu geri alınamaz.`, [
      { label: 'Vazgeç', run: () => this.hideConfirm() },
      {
        label: 'Modeli sil',
        danger: true,
        run: () => {
          this.ctx.processing.removeModel(this.model.id);
          this.ctx.log.info(`“${this.model.label}” modeli silindi.`);
          this.dialog.close();
        },
      },
    ]);
  }

  // ── Rendering ──────────────────────────────────────────────────────────

  private render(): void {
    this.problems = checkModel(this.model, (id) => this.lookup(id));
    const byStep = new Map<string, string>();
    for (const p of this.problems) if (p.step && !byStep.has(p.step)) byStep.set(p.step, p.message);
    if (this.selected && !this.exists(this.selected)) this.selected = null;
    this.canvas.render(this.model, this.selected, byStep);
    replaceChildren(this.inspector, renderInspector(this, this.selected));
    this.refresh();
  }

  /** What changes while typing: the title and the status line. */
  private refresh(): void {
    this.dialog?.el.querySelector('.dialog__title')?.replaceChildren(`Model tasarımcısı: ${this.model.label || 'adsız'}${this.dirty ? ' •' : ''}`);
    if (this.confirming) return;
    const n = this.problems.length;
    replaceChildren(
      this.status,
      n ? icon('warning', 16) : icon('success', 16),
      h(
        'span',
        { class: 'ptool__status-text' },
        n ? `${n} sorun var; model kaydedilebilir ama çalışmaz. ${this.problems[0].message}` : this.model.steps.length ? `${this.model.steps.length} adım, ${this.model.inputs.length} girdi. Model çalışmaya hazır.` : 'Soldan bir girdi ve bir araç ekleyerek başlayın.',
      ),
    );
    this.status.dataset.kind = n ? 'warn' : 'ok';
  }

  private get dirty(): boolean {
    return this.savedJson !== JSON.stringify(this.model);
  }

  private exists(ref: NodeRef): boolean {
    return ref.kind === 'input' ? this.model.inputs.some((i) => i.name === ref.name) : this.model.steps.some((s) => s.id === ref.id);
  }

  /** Where a new box goes: to the right of the selected one, else below the others. */
  private spotNear(): { x: number; y: number } {
    const sel = this.selected;
    const pos = sel?.kind === 'input' ? this.model.inputPositions?.[sel.name] : sel?.kind === 'step' ? this.model.steps.find((s) => s.id === sel.id)?.position : undefined;
    if (pos) return { x: pos.x + 290, y: pos.y };
    const ys = [...this.model.steps.map((s) => s.position?.y ?? 0), ...Object.values(this.model.inputPositions ?? {}).map((p) => p.y)];
    return { x: 40, y: ys.length ? Math.max(...ys) + 100 : 40 };
  }

  private move(ref: NodeRef, at: { x: number; y: number }, done: boolean): void {
    const apply = () => {
      if (ref.kind === 'input') this.model.inputPositions = { ...this.model.inputPositions, [ref.name]: at };
      else {
        const s = this.model.steps.find((x) => x.id === ref.id);
        if (s) s.position = at;
      }
    };
    if (!done) {
      // Live while dragging; the undo step is taken when the drag ends.
      if (!this.dragStart) this.dragStart = JSON.stringify({ model: this.model, selected: this.selected });
      apply();
      this.canvas.render(this.model, this.selected, new Map(this.problems.flatMap((p) => (p.step ? [[p.step, p.message] as const] : []))));
      return;
    }
    apply();
    if (this.dragStart) {
      this.past.push(this.dragStart);
      this.future.length = 0;
      this.dragStart = null;
    }
    this.refresh();
  }

  /** A wire dropped on a step: pick which of its inputs the source feeds. */
  private connectMenu(from: NodeRef, toStep: string, client: { x: number; y: number }): void {
    const step = this.model.steps.find((s) => s.id === toStep);
    const tool = step && this.lookup(step.tool);
    if (!step || !tool) return;
    const sourceStep = from.kind === 'step' ? this.model.steps.find((s) => s.id === from.id) : undefined;
    const outputs = from.kind === 'input' ? [null] : (this.lookup(sourceStep?.tool ?? '')?.outputs ?? []);
    const items: MenuItem[] = [];
    for (const out of outputs) {
      for (const p of tool.parameters) {
        const fits = sourcesFor(this.model, toStep, p, (id) => this.lookup(id)).find((o) =>
          from.kind === 'input' ? o.src.kind === 'input' && o.src.name === from.name : o.src.kind === 'output' && o.src.step === from.id && o.src.output === out?.name,
        );
        if (!fits) continue;
        const current = step.values[p.name];
        items.push({
          label: out ? `${out.label} → ${p.label}` : p.label,
          detail: current && JSON.stringify(current) !== JSON.stringify(fits.src) ? 'Mevcut bağlantının yerine geçer' : undefined,
          checked: JSON.stringify(current) === JSON.stringify(fits.src),
          run: () =>
            this.change(() => {
              setSource(this.model, toStep, p.name, fits.src);
              this.selected = { kind: 'step', id: toStep };
            }),
        });
      }
    }
    if (!items.length) {
      const what = from.kind === 'input' ? this.model.inputs.find((i) => i.name === from.name)?.label : sourceStep && stepName(sourceStep, (id) => this.lookup(id));
      items.push({ label: `“${what}” bu adımın hiçbir girdisine uymuyor`, disabled: true });
    }
    PopupMenu.open([{ kind: 'header', label: `${stepName(step, (id) => this.lookup(id))}: hangi girdi?` }, ...items], client, { placement: 'point' });
  }

  // ── Saving, closing, keys ──────────────────────────────────────────────

  private save(): boolean {
    if (!this.model.label.trim()) {
      this.model.label = 'Adsız model';
    }
    this.ctx.processing.saveModel(this.model);
    this.savedJson = JSON.stringify(this.model);
    this.ctx.log.success(`“${this.model.label}” modeli kaydedildi${this.problems.length ? `; ${this.problems.length} sorun giderilene kadar çalışmaz` : ''}.`);
    this.render();
    return true;
  }

  private beforeClose(): boolean {
    if (!this.dirty || this.confirming) return true;
    this.showConfirm('Kaydedilmemiş değişiklikler var.', [
      {
        label: 'Kaydetmeden kapat',
        run: () => {
          this.confirming = true;
          this.dialog.close();
        },
      },
      { label: 'Vazgeç', run: () => this.hideConfirm() },
      {
        label: 'Kaydet ve kapat',
        primary: true,
        run: () => {
          this.save();
          this.dialog.close();
        },
      },
    ]);
    return false;
  }

  private showConfirm(message: string, actions: { label: string; run: () => void; primary?: boolean; danger?: boolean }[]): void {
    this.confirming = true;
    this.status.dataset.kind = 'warn';
    replaceChildren(
      this.status,
      icon('warning', 16),
      h('span', { class: 'ptool__status-text' }, message),
      actions.map((a) => {
        const b = h('button', { class: `btn btn--small${a.primary ? '' : ' btn--ghost'}${a.danger ? ' mins__danger' : ''}`, type: 'button' }, a.label);
        b.addEventListener('click', a.run);
        return b;
      }),
    );
    (this.status.querySelector('button:last-child') as HTMLElement | null)?.focus();
  }

  private hideConfirm(): void {
    this.confirming = false;
    this.refresh();
  }

  private undo(redo = false): void {
    const from = redo ? this.future : this.past;
    const to = redo ? this.past : this.future;
    const snap = from.pop();
    if (!snap) return;
    to.push(JSON.stringify({ model: this.model, selected: this.selected }));
    const s = JSON.parse(snap) as { model: ProcessingModel; selected: NodeRef | null };
    this.model = s.model;
    this.selected = s.selected;
    this.lastKey = null;
    this.render();
  }

  private onKey(e: KeyboardEvent): void {
    const typing = (e.target as HTMLElement).closest('input, textarea, [contenteditable]');
    const k = e.key.toLowerCase();
    if ((e.ctrlKey || e.metaKey) && k === 's') {
      e.preventDefault();
      this.save();
    } else if ((e.ctrlKey || e.metaKey) && !typing && (k === 'z' || k === 'y')) {
      e.preventDefault();
      this.undo(k === 'y' || e.shiftKey);
    } else if (!typing && (e.key === 'Delete' || e.key === 'Backspace') && this.selected) {
      e.preventDefault();
      this.inspector.querySelector<HTMLButtonElement>('.mins__danger')?.click();
    }
  }
}
