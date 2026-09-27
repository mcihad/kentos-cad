import type { AppContext } from '../../../app/context';
import type { Disposable } from '../../../core/disposable';
import type { Vec2 } from '../../../model/geometry';
import { checkModel, type ModelIssue, type ProcessingModel } from '../../../processing/model';
import { addInput, addStep, autoLayout, copyModel, newModel, setSource } from '../../../processing/modelEdit';
import { PickPointTool } from '../../../tools/pickPointTool';
import { h, replaceChildren } from '../../dom';
import { icon } from '../../icons';
import { askRemove, askUnsaved } from '../../widgets/confirm';
import { Dialog } from '../../widgets/Dialog';
import { PopupMenu, type MenuItem } from '../../widgets/PopupMenu';
import { openModelDialog } from '../ToolDialog';
import { connectChoices, DESIGNER_HISTORY, DESIGNER_TEXTS, designerStatus, joins, savedLabel, snap, spotNear, type NodeRef } from './designerPlan';
import { ModelCanvas } from './ModelCanvas';
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
  /** JSON of the draft as last saved; null while the model is not in the library. */
  saved: string | null;
  /** JSON of the draft as last saved or as opened (a new model: the empty one; a copy: the copy): what `dirty` compares with. */
  baseline: string;
  builtinCopy: boolean;
  selected: NodeRef | null;
}

export function openModelDesigner(ctx: AppContext, modelId?: string): void {
  const source = modelId ? ctx.processing.model(modelId) : undefined;
  if (modelId && !source) {
    ctx.log.error(T.notFound(modelId));
    return;
  }
  const builtin = !!source && ctx.processing.isBuiltinModel(source.id);
  const draft = source ? (builtin ? copyModel(source) : (JSON.parse(JSON.stringify(source)) as ProcessingModel)) : newModel();
  if (!draft.steps.every((s) => s.position) || !draft.inputs.every((i) => draft.inputPositions?.[i.name])) autoLayout(draft);
  const json = JSON.stringify(draft);
  new ModelDesigner(ctx, { draft, saved: source && !builtin ? json : null, baseline: json, builtinCopy: builtin, selected: null });
}

const T = DESIGNER_TEXTS;

class ModelDesigner implements InspectorHost {
  readonly ctx: AppContext;
  model: ProcessingModel;
  problems: ModelIssue[] = [];
  private savedJson: string | null;
  private baseline: string;
  readonly builtinCopy: boolean;
  private selected: NodeRef | null;
  private readonly past: string[] = [];
  private readonly future: string[] = [];
  private lastKey: { key: string; at: number } | null = null;
  /** Snapshot taken when a box drag starts; pushed as one undo step when it ends. */
  private dragStart: string | null = null;

  private readonly dialog: Dialog;
  private readonly canvas: ModelCanvas;
  private readonly inspector = h('aside', { class: 'mdesign__inspector', 'aria-label': DESIGNER_TEXTS.inspector.label });
  private readonly status = h('div', { class: 'ptool__status mdesign__status', role: 'status', 'aria-live': 'polite' });
  private readonly subs: Disposable[] = [];
  /** The unsaved-changes question is open. */
  private asking = false;

  constructor(ctx: AppContext, state: DesignerState) {
    this.ctx = ctx;
    this.model = state.draft;
    this.savedJson = state.saved;
    this.baseline = state.baseline;
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
            const id = addStep(this.model, toolId, (x) => this.lookup(x), at ? { x: snap(at.x), y: snap(at.y) } : this.spotNear(), from);
            this.selected = { kind: 'step', id };
          });
        },
      },
      this.subs,
    );

    const layout = h('button', { class: 'btn btn--ghost', type: 'button', title: T.footer.layoutTip }, icon('columns', 14), T.footer.layout);
    layout.addEventListener('click', () =>
      this.change(() => autoLayout(this.model), { after: () => queueMicrotask(() => this.canvas.fit()) }),
    );
    const close = h('button', { class: 'btn', type: 'button' }, T.footer.close);
    close.addEventListener('click', () => this.dialog.request());
    const saveRun = h('button', { class: 'btn', type: 'button' }, icon('play', 14), T.footer.saveRun);
    saveRun.addEventListener('click', () => {
      if (!this.save()) return;
      this.dialog.close();
      openModelDialog(ctx, this.model.id);
    });
    const save = h('button', { class: 'btn btn--primary', type: 'button' }, T.footer.save);
    save.addEventListener('click', () => this.save());

    this.dialog = new Dialog({
      title: T.title,
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
    if (!joins(opts.key, this.lastKey, now)) {
      this.past.push(JSON.stringify({ model: this.model, selected: this.selected }));
      if (this.past.length > DESIGNER_HISTORY) this.past.shift();
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

  /**
   * Closes the designer while the user shows a point, then opens it again
   * on the same draft; `also` is a choice set with it (Sahneden seç beside
   * a choice, docs/adr/0088).
   */
  pickPoint(stepId: string, param: string, also?: { param: string; value: string }): void {
    const state: DesignerState = { draft: this.model, saved: this.savedJson, baseline: this.baseline, builtinCopy: this.builtinCopy, selected: { kind: 'step', id: stepId } };
    const def = this.lookup(this.model.steps.find((s) => s.id === stepId)?.tool ?? '')?.parameters.find((p) => p.name === param);
    this.dialog.close();
    const reopen = (p: Vec2 | null) =>
      queueMicrotask(() => {
        if (p) {
          setSource(state.draft, stepId, param, { kind: 'value', value: p });
          if (also) setSource(state.draft, stepId, also.param, { kind: 'value', value: also.value });
        }
        new ModelDesigner(this.ctx, state);
      });
    this.ctx.tools.run(new PickPointTool(this.ctx, def?.label ?? T.pick.point, reopen), T.pick.command(def?.label ?? T.pick.unnamed));
  }

  pickChoice(stepId: string, param: string): void {
    const def = this.lookup(this.model.steps.find((s) => s.id === stepId)?.tool ?? '')?.parameters.find((p) => p.name === param);
    const picks = def?.type === 'enum' ? def.picks : undefined;
    if (picks) this.pickPoint(stepId, picks.point, { param, value: picks.option });
  }

  deleteModel(): void {
    const label = this.model.label;
    void askRemove({ title: T.remove.title, message: T.remove.question(label), action: T.remove.action }).then((yes) => {
      if (!yes) return;
      this.ctx.processing.removeModel(this.model.id);
      this.ctx.log.info(T.remove.done(label));
      this.dialog.close();
    });
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
    this.dialog?.el.querySelector('.dialog__title')?.replaceChildren(T.titleOf(this.model.label, this.dirty));
    const status = designerStatus(this.problems, this.model.steps.length, this.model.inputs.length);
    replaceChildren(this.status, icon(status.kind === 'warn' ? 'warning' : 'success', 16), h('span', { class: 'ptool__status-text' }, status.text));
    this.status.dataset.kind = status.kind;
  }

  private get dirty(): boolean {
    return this.baseline !== JSON.stringify(this.model);
  }

  private exists(ref: NodeRef): boolean {
    return ref.kind === 'input' ? this.model.inputs.some((i) => i.name === ref.name) : this.model.steps.some((s) => s.id === ref.id);
  }

  /** Where a new box goes (designerPlan.ts `spotNear`). */
  private spotNear(): { x: number; y: number } {
    return spotNear(this.model, this.selected);
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

  /** A wire dropped on a step: pick which of its inputs the source feeds (designerPlan.ts `connectChoices`). */
  private connectMenu(from: NodeRef, toStep: string, client: { x: number; y: number }): void {
    const choices = connectChoices(this.model, from, toStep, (id) => this.lookup(id));
    if (!choices) return;
    const items: MenuItem[] = choices.items.map((c) => ({
      label: c.label,
      detail: c.detail,
      checked: c.checked,
      run: () =>
        this.change(() => {
          setSource(this.model, toStep, c.param, c.src);
          this.selected = { kind: 'step', id: toStep };
        }),
    }));
    if (choices.none) items.push({ label: choices.none, disabled: true });
    PopupMenu.open([{ kind: 'header', label: choices.header }, ...items], client, { placement: 'point' });
  }

  // ── Saving, closing, keys ──────────────────────────────────────────────

  private save(): boolean {
    this.model.label = savedLabel(this.model.label);
    this.ctx.processing.saveModel(this.model);
    this.savedJson = this.baseline = JSON.stringify(this.model);
    this.ctx.log.success(T.save.saved(this.model.label, this.problems.length));
    this.render();
    return true;
  }

  /** Unsaved changes are asked about in a window over the designer (DESIGN.md §7.9.1); the answer closes or stays. */
  private beforeClose(): boolean {
    if (!this.dirty) return true;
    if (!this.asking) {
      this.asking = true;
      void askUnsaved({ name: this.model.label || T.save.unnamed, after: T.unsaved.after, verb: T.unsaved.verb }).then((a) => {
        this.asking = false;
        if (a === 'discard') this.dialog.close();
        else if (a === 'save' && this.save()) this.dialog.close();
      });
    }
    return false;
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
