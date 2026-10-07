import type { Vec2 } from '../../model/geometry';
import type { LayerStore } from '../../model/layers';
import type { InputSummary } from '../../processing/features';
import { orderSteps, stepName, type ProcessingModel } from '../../processing/model';
import { defaultValues, fieldSource, isVisible, restoreValues, scopesOf, type ValidationIssue } from '../../processing/parameters';
import type { ProcessingRunner, RunOutcome, TargetChoice } from '../../processing/runner';
import type { DefaultsContext, EnumOption, ExecutionTarget, FeaturesValue, FileValue, LayerValue, ParamDef, ParamUnit, ProcessingTool, TableOutput } from '../../processing/types';
import { DIALOG_TEXTS as T, SCOPE_SHORT } from './dialogTexts';
import {
  attrFieldView,
  expressionView,
  featuresView,
  fileView,
  layerFieldView,
  planLayers,
  pointView,
  type AttrFieldView,
  type ExpressionView,
  type FeaturesView,
  type FileView,
  type LayerFieldView,
  type PlanLayers,
  type PointView,
} from './fieldPlan';
import { TARGET_LABEL } from './targets';

/**
 * The processing dialog without the page: the form a tool's definition
 * makes, which parameters show in which section, when a field shows its
 * problem, the preview, the status line and the footer, the "Nerede
 * çalışır" list, and the dialog's state with what each action does to it;
 * with the fields (fieldPlan) and the words (dialogTexts), what the dialog
 * shows. ToolDialog and paramFields draw from these;
 * fixtures/processing/v1/dialog.json pins them so the desktop's dialog
 * plays the same file (fixtures/processing/README.md).
 */

type Values = Record<string, unknown>;
type ListedScope = Exclude<FeaturesValue['scope'], 'ids'>;

// ── The form ─────────────────────────────────────────────────────────

/** How a parameter is asked for; fixed by its definition. */
export type ControlForm =
  | { type: 'features'; scopes: { value: ListedScope; label: string }[] }
  | { type: 'number'; unit?: ParamUnit }
  | { type: 'string'; short: boolean; maxLength?: number; placeholder?: string }
  | { type: 'boolean' }
  /** picks: one option is a point picked on the drawing, beside the choice (Sahneden seç, docs/adr/0088). */
  | { type: 'enum'; control: 'segmented' | 'dropdown'; options: EnumOption[]; picks?: { option: string; point: string } }
  | { type: 'layer' }
  | { type: 'point' }
  /** combo: a name field with the list beside it (allowNew); dropdown: only the list. `multiple`: several names, the list checks them. */
  | { type: 'field'; control: 'combo' | 'dropdown'; placeholder: string; multiple?: true }
  /** A file chosen with its button (docs/adr/0200 §7); `accept`: the extensions offered. */
  | { type: 'file'; accept: string[] }
  | { type: 'expression'; returns: 'condition' | 'value'; placeholder?: string };

export interface RowForm {
  name: string;
  label: string;
  description?: string;
  /** "isteğe bağlı" after the label. */
  optional: boolean;
  /** The control goes under the label, not beside it. */
  stacked: boolean;
  control: ControlForm;
}

/** A choice of three or fewer short options is a segmented control, anything else a dropdown. */
export const enumControl = (def: Extract<ParamDef, { type: 'enum' }>): 'segmented' | 'dropdown' =>
  def.options.length <= 3 && def.options.every((o) => o.label.length <= 22) ? 'segmented' : 'dropdown';

export function controlForm(def: ParamDef): ControlForm {
  switch (def.type) {
    case 'features':
      return { type: 'features', scopes: scopesOf(def).map((s) => ({ value: s, label: SCOPE_SHORT[s] })) };
    case 'number':
      return def.unit ? { type: 'number', unit: def.unit } : { type: 'number' };
    case 'string':
      return { type: 'string', short: !!def.maxLength && def.maxLength <= 2, ...(def.maxLength ? { maxLength: def.maxLength } : {}), ...(def.placeholder ? { placeholder: def.placeholder } : {}) };
    case 'boolean':
      return { type: 'boolean' };
    case 'enum':
      return { type: 'enum', control: enumControl(def), options: def.options.map((o) => ({ ...o })), ...(def.picks ? { picks: { ...def.picks } } : {}) };
    case 'layer':
      return { type: 'layer' };
    case 'point':
      return { type: 'point' };
    case 'field': {
      const multiple = def.multiple ? { multiple: true as const } : {};
      return def.allowNew ? { type: 'field', control: 'combo', placeholder: T.field.placeholder, ...multiple } : { type: 'field', control: 'dropdown', placeholder: T.field.choose, ...multiple };
    }
    case 'expression':
      return { type: 'expression', returns: def.returns, ...(def.placeholder ? { placeholder: def.placeholder } : {}) };
    case 'file':
      return { type: 'file', accept: [...def.accept] };
  }
}

export function rowForm(def: ParamDef): RowForm {
  return {
    name: def.name,
    label: def.label,
    ...(def.description ? { description: def.description } : {}),
    optional: !!def.optional,
    stacked: def.type === 'features' || def.type === 'expression',
    control: controlForm(def),
  };
}

/** The side of the dialog: where the tool sits, what it does, its help or a model's steps, and the command-line names. */
export interface SideForm {
  crumb: { icon: string; text: string };
  icon: string;
  about: string;
  /** Paragraphs; a model lists its steps instead. */
  help: string[];
  /** A model's steps in run order. */
  steps: { icon: string; name: string }[] | null;
  /** A model's way into the designer: a built-in one is copied first. */
  edit: string | null;
  /** Whether the side has an Önizleme block (the tool has a preview). */
  preview: boolean;
  aliases: string[];
}

export interface DialogForm {
  title: string;
  rows: RowForm[];
  side: SideForm;
}

export interface FormHost {
  categoryPath(id: string): string;
  categoryIcon(id: string): string | undefined;
}

/** A model's steps in run order (the dialog's side), with their tools' icons. */
export function modelSteps(model: ProcessingModel, lookup: (id: string) => ProcessingTool | undefined): { icon: string; name: string }[] {
  const ids = orderSteps(model);
  const order = Array.isArray(ids) ? ids.map((id) => model.steps.find((s) => s.id === id)!) : model.steps;
  return order.map((s) => ({ icon: lookup(s.tool)?.icon ?? 'processing', name: stepName(s, lookup) }));
}

/** `model`: set for a model's dialog (the tool is `modelAsTool`). */
export function dialogForm(tool: ProcessingTool, host: FormHost, model?: { steps: { icon: string; name: string }[]; builtin: boolean }): DialogForm {
  const path = host.categoryPath(tool.category);
  return {
    title: tool.label,
    rows: tool.parameters.map(rowForm),
    side: {
      crumb: model ? { icon: 'processing', text: `${T.side.models} › ${path || T.side.noCategory}` } : { icon: host.categoryIcon(tool.category) ?? 'processing', text: path },
      icon: tool.icon ?? 'processing',
      about: tool.description,
      help: model ? [] : (tool.help ?? '').split(/\n\s*\n/).filter(Boolean),
      steps: model ? model.steps : null,
      edit: model ? (model.builtin ? T.side.editCopy : T.side.editModel) : null,
      preview: !!tool.preview,
      aliases: [...(tool.aliases ?? [])],
    },
  };
}

// ── Sections and problems ────────────────────────────────────────────

export interface Sections {
  /** Girdi (features), Ayarlar (the rest), Çıktı (layers): the ones with rows, in this order. */
  groups: { title: string; rows: string[] }[];
  /** Gelişmiş ayarlar: its rows, shown when open; null when no advanced parameter shows. */
  advanced: { rows: string[]; open: boolean } | null;
}

/** Which parameters show where; Gelişmiş is open when the user opened it or one of its fields shows a problem. */
export function sectionsOf(params: readonly ParamDef[], values: Values, advancedOpen: boolean, issues: Readonly<Record<string, string>>): Sections {
  const shown = params.filter((p) => isVisible(p, values));
  const names = (keep: (p: ParamDef) => boolean) => shown.filter(keep).map((p) => p.name);
  const groups = [
    { title: T.sections.input, rows: names((p) => !p.advanced && p.type === 'features') },
    { title: T.sections.main, rows: names((p) => !p.advanced && p.type !== 'features' && p.type !== 'layer') },
    { title: T.sections.output, rows: names((p) => !p.advanced && p.type === 'layer') },
  ].filter((g) => g.rows.length);
  const advanced = names((p) => !!p.advanced);
  return { groups, advanced: advanced.length ? { rows: advanced, open: advancedOpen || advanced.some((n) => n in issues) } : null };
}

/** The problems shown under fields: a touched field's live, every field's after a run attempt. */
export function shownIssues(issues: readonly ValidationIssue[], touched: readonly string[], attempted: boolean): Record<string, string> {
  const out: Record<string, string> = {};
  for (const i of issues) if (i.param && !(i.param in out) && (attempted || touched.includes(i.param))) out[i.param] = i.message;
  return out;
}

// ── Preview, status and footer ───────────────────────────────────────

/** The side's Önizleme line; null when the tool has none. Muted when there is nothing to show. */
export function previewOf(tool: ProcessingTool, values: Values, issues: readonly ValidationIssue[]): { text: string; muted: boolean } | null {
  if (!tool.preview) return null;
  const valid = !issues.some((i) => i.param);
  const text = valid ? tool.preview(values as never) : null;
  return { text: text ?? (valid ? '' : T.previewBlocked), muted: !text };
}

export type DialogStatus =
  | { kind: 'idle' }
  | { kind: 'running'; fraction: number; label: string; where?: ExecutionTarget }
  /**
   * `pick`: objects "Sonuçları seç" selects; `selected`: the run set the selection itself; `undo`: it edited the
   * drawing; `table`: the run's table output, shown under the form (docs/adr/0200 §7).
   */
  | { kind: 'ok'; text: string; pick: readonly number[]; selected: boolean; undo: boolean; table?: TableOutput }
  | { kind: 'error' | 'invalid'; text: string };

export type StatusAction = 'zoom' | 'select' | 'undo';

export interface StatusLine {
  kind: 'idle' | 'running' | 'ok' | 'warn' | 'error';
  icon: 'success' | 'warning' | 'error' | null;
  text: string;
  /** Running: the bar, 0–100. */
  progress?: number;
  /** After a run: "Seçime yakınlaştır" (the run selected) or "Sonuçları seç", then "Geri al" (it edited). */
  actions: StatusAction[];
  /** What "Sonuçları seç" selects. */
  pick?: number[];
}

export function statusLine(status: DialogStatus, attempted: boolean, issues: readonly ValidationIssue[]): StatusLine {
  if (status.kind === 'running') {
    const text = status.label || (status.where === 'worker' ? T.running.worker : T.running.here);
    return { kind: 'running', icon: null, text, progress: Math.round(status.fraction * 100), actions: [] };
  }
  if (status.kind === 'ok') {
    const actions: StatusAction[] = status.selected ? ['zoom'] : status.pick.length ? ['select'] : [];
    if (status.undo) actions.push('undo');
    return { kind: 'ok', icon: 'success', text: status.text, actions, ...(actions.includes('select') ? { pick: [...status.pick] } : {}) };
  }
  const toolIssue = attempted ? issues.find((i) => !i.param) : undefined;
  const fieldIssues = attempted ? issues.filter((i) => i.param).length : 0;
  if (toolIssue || fieldIssues || status.kind === 'invalid') {
    const text = toolIssue?.message ?? (fieldIssues ? T.fixFields(fieldIssues) : status.kind === 'invalid' ? status.text : '');
    return { kind: 'warn', icon: 'warning', text, actions: [] };
  }
  if (status.kind === 'error') return { kind: 'error', icon: 'error', text: status.text, actions: [] };
  return { kind: 'idle', icon: null, text: '', actions: [] };
}

export interface Footer {
  run: { label: string; disabled: boolean };
  /** Kapat, or Durdur while running (stops the run). */
  close: string;
  /** Varsayılanlar waits while a run is going. */
  reset: { disabled: boolean };
}

export function footerOf(status: DialogStatus): Footer {
  const running = status.kind === 'running';
  return { run: { label: running ? T.footer.running : T.footer.run, disabled: running }, close: running ? T.footer.stop : T.footer.close, reset: { disabled: running } };
}

// ── Nerede çalışır ───────────────────────────────────────────────────

export interface TargetsInput {
  /** The places the tool names, in its order (a model: every place one of its steps can go here). */
  declared: readonly ExecutionTarget[];
  /** The places this host can run it. */
  available: ReadonlySet<ExecutionTarget>;
  /** Where Otomatik sends these inputs now; null for a model (each step decides). */
  auto: ExecutionTarget | null;
  model: boolean;
}

export interface TargetOption {
  value: TargetChoice;
  label: string;
  note: string | null;
  disabled: boolean;
  checked: boolean;
}

/**
 * The choice a run takes: with several places the stored one, Otomatik
 * when that place is not here; with one place that place; null with none.
 */
export function effectiveChoice(choice: TargetChoice, available: ReadonlySet<ExecutionTarget>): TargetChoice | null {
  if (available.size > 1) return choice === 'auto' || available.has(choice) ? choice : 'auto';
  return available.size ? [...available][0] : null;
}

/** Otomatik (and what it picks now) when there is a choice, then each declared place; places not here are "yakında". */
export function targetsView(t: TargetsInput, choice: TargetChoice): { options: TargetOption[]; hint: string | null; choice: TargetChoice | null } {
  const several = t.available.size > 1;
  const current = effectiveChoice(choice, t.available);
  const option = (value: TargetChoice, label: string, note: string | null, disabled = false): TargetOption => ({ value, label, note, disabled, checked: !disabled && current === value });
  const options: TargetOption[] = [];
  if (several) options.push(option('auto', T.targets.auto, t.auto ? T.targets.autoNow(t.auto) : t.model ? T.targets.autoModel : null));
  for (const d of t.declared) options.push(t.available.has(d) ? option(d, TARGET_LABEL[d], several ? null : T.targets.thisRun) : option(d, TARGET_LABEL[d], T.targets.soon, true));
  return { options, hint: several && current === 'auto' ? T.targets.hint : null, choice: current };
}

// ── The dialog's state ───────────────────────────────────────────────

export interface DialogState {
  readonly values: Readonly<Values>;
  /** Fields the user changed: their problems show at once. */
  readonly touched: readonly string[];
  /** After a run attempt every problem shows, until a run succeeds or the values are reset. */
  readonly attempted: boolean;
  readonly advancedOpen: boolean;
  readonly status: DialogStatus;
  /** What a run found that the values do not show (nothing selected): shown until a value changes or another run. */
  readonly runIssues: readonly ValidationIssue[] | null;
  /** The place the user chose to run on (stored per tool). */
  readonly choice: TargetChoice;
}

const IDLE: DialogStatus = { kind: 'idle' };

/**
 * The dialog opening: the given values (history, a point shown on the
 * map) or the last run's, over the defaults, keeping those that still
 * fit. Gelişmiş opens when one of its shown values is not the default, so
 * a changed value is never hidden.
 */
export function startState(tool: ProcessingTool, given: Values | undefined, last: Values | undefined, defaults: DefaultsContext, choice: TargetChoice): DialogState {
  const values = restoreValues(tool, given ?? last, defaults);
  const base = defaultValues(tool, defaults);
  const advancedOpen = tool.parameters.some((p) => p.advanced && isVisible(p, values) && JSON.stringify(values[p.name]) !== JSON.stringify(base[p.name]));
  return { values, touched: [], attempted: false, advancedOpen, status: IDLE, runIssues: null, choice };
}

/**
 * A field changed. Typing marks the field touched even when the value
 * stays; choosing what is already chosen changes nothing. A finished
 * run's line clears; a running one stays.
 */
export function edited(state: DialogState, name: string, value: unknown, typed = false): DialogState {
  if (!typed && JSON.stringify(state.values[name]) === JSON.stringify(value)) return state;
  const k = state.status.kind;
  return {
    ...state,
    values: { ...state.values, [name]: value },
    touched: state.touched.includes(name) ? state.touched : [...state.touched, name],
    status: k === 'ok' || k === 'error' || k === 'invalid' ? IDLE : state.status,
    runIssues: null,
  };
}

/** Varsayılanlar: every value to its default, as if just opened (not while running). */
export function resetState(state: DialogState, tool: ProcessingTool, defaults: DefaultsContext): DialogState {
  if (state.status.kind === 'running') return state;
  return { ...state, values: defaultValues(tool, defaults), touched: [], attempted: false, status: IDLE, runIssues: null };
}

export const toggledAdvanced = (state: DialogState): DialogState => ({ ...state, advancedOpen: !state.advancedOpen });

export const chosenTarget = (state: DialogState, choice: TargetChoice): DialogState => ({ ...state, choice });

/**
 * Çalıştır: with problems (`issues`, the values' check) the run does not
 * start and every problem shows; one in an advanced field opens Gelişmiş.
 */
export function attempted(state: DialogState, tool: ProcessingTool, issues: readonly ValidationIssue[]): { state: DialogState; run: boolean } {
  const next: DialogState = { ...state, attempted: true, runIssues: null };
  if (!issues.length) return { state: next, run: true };
  const advanced = issues.some((i) => tool.parameters.find((p) => p.name === i.param)?.advanced);
  return { state: { ...next, advancedOpen: next.advancedOpen || advanced }, run: false };
}

export const started = (state: DialogState, where?: ExecutionTarget): DialogState => ({ ...state, status: { kind: 'running', fraction: 0, label: '', ...(where ? { where } : {}) } });

export const progressed = (state: DialogState, fraction: number, label: string): DialogState =>
  state.status.kind === 'running' ? { ...state, status: { ...state.status, fraction, label } } : state;

/**
 * A run ended. Done: its summary, what "Sonuçları seç" selects (what it
 * added, else what it changed or selected) and whether there is something
 * to undo; problems are forgotten. Refused before running: the runner's
 * problems show on their fields. Failed or stopped: its message.
 */
export function finished(state: DialogState, out: RunOutcome, tool: ProcessingTool): DialogState {
  switch (out.status) {
    case 'ok': {
      const name = tool.outputs?.find((o) => o.type === 'table')?.name;
      const table = name ? (out.result.outputs?.[name] as TableOutput | undefined) : undefined;
      const status: DialogStatus = { kind: 'ok', text: out.record.summary, pick: out.added.length ? out.added : out.touched, selected: !!out.result.select, undo: out.edited, ...(table ? { table } : {}) };
      return { ...state, attempted: false, runIssues: null, status };
    }
    case 'invalid':
      return { ...state, runIssues: out.issues, status: { kind: 'invalid', text: out.issues[0]?.message ?? '' } };
    default:
      return { ...state, status: { kind: 'error', text: out.message } };
  }
}

/** Geri al after a run: the drawing's last step undone, the line cleared. */
export const undone = (state: DialogState): DialogState => ({ ...state, status: IDLE });

/**
 * Back from showing a point on the map: the dialog as it was, with the
 * point as a choice (null: the user gave up, nothing changes). A run that
 * was going on is not followed any more.
 */
export function picked(state: DialogState, name: string, point: Vec2 | null): DialogState {
  const back = state.status.kind === 'running' ? { ...state, status: IDLE } : state;
  return point ? edited(back, name, point) : back;
}

/**
 * Back from Sahneden seç beside a choice (the numbering's start vertex):
 * the point fills its parameter and the choice takes the option, both
 * touched; null (Esc) changes nothing.
 */
export function pickedChoice(state: DialogState, choice: string, option: string, pointName: string, point: Vec2 | null): DialogState {
  const back = state.status.kind === 'running' ? { ...state, status: IDLE } : state;
  return point ? edited(edited(back, pointName, point, true), choice, option, true) : back;
}

/**
 * Back from Sahneden seç for input objects: kept with `count` objects
 * selected, the field is the selection, its kinds as they were chosen,
 * touched; left (Esc) or with nothing picked, nothing changes.
 */
export function pickedObjects(state: DialogState, name: string, keep: boolean, count: number): DialogState {
  const back = state.status.kind === 'running' ? { ...state, status: IDLE } : state;
  if (!keep || count === 0) return back;
  const kinds = (state.values[name] as FeaturesValue | undefined)?.kinds;
  const value: FeaturesValue = kinds ? { scope: 'selection', kinds: [...kinds] } : { scope: 'selection' };
  return edited(back, name, value, true);
}

// ── What the dialog shows ────────────────────────────────────────────

export interface ViewEnv {
  /** The values' problems (the runner's check). */
  readonly issues: readonly ValidationIssue[];
  /** What each features parameter reads now, and each chosen file's table. */
  readonly inputs: Readonly<Record<string, InputSummary>>;
  /** What a field parameter's source shown now reads (`fieldSource`): the names it offers. */
  sourceOf(name: string): InputSummary | undefined;
  /** An expression parameter's line on its input's objects. */
  previewExpression(name: string): string | null;
  readonly layers: PlanLayers;
  formatPoint(p: Vec2): string;
  readonly targets: TargetsInput;
}

/** The environment of a dialog on this runner and layer tree; `modelTools`: a model's step tools. */
export function hostEnv(runner: ProcessingRunner, tool: ProcessingTool, values: Values, layers: LayerStore, formatPoint: (p: Vec2) => string, modelTools?: readonly ProcessingTool[]): ViewEnv {
  const inputs = runner.describeInputs(tool, values);
  const available = new Set((modelTools ?? [tool]).flatMap((t) => runner.executorsFor(t).map((e) => e.target)));
  const size = Object.values(inputs).reduce((n, s) => n + s.count, 0);
  return {
    issues: runner.validate(tool, values),
    inputs,
    sourceOf: (name) => {
      const def = tool.parameters.find((p) => p.name === name);
      const src = def?.type === 'field' ? fieldSource(tool, def, values) : undefined;
      return src ? inputs[src] : undefined;
    },
    previewExpression: (name) => runner.previewExpression(tool, values, name),
    layers: planLayers(layers),
    formatPoint,
    targets: {
      declared: modelTools ? [...available] : tool.targets,
      available,
      auto: modelTools ? null : (runner.executorFor(tool, 'auto', size)?.target ?? null),
      model: !!modelTools,
    },
  };
}

export type FieldView = FeaturesView | LayerFieldView | PointView | AttrFieldView | ExpressionView | FileView;

/** A shown field's changing parts (number, text, switch and choice fields show only their value). */
export function fieldView(def: ParamDef, value: unknown, env: ViewEnv): FieldView | null {
  switch (def.type) {
    case 'features':
      return featuresView(def, value as FeaturesValue, env.inputs[def.name], env.layers);
    case 'layer':
      return layerFieldView(def, value as LayerValue, env.layers);
    case 'point':
      return pointView(value as Vec2 | null, env.formatPoint);
    case 'field':
      return attrFieldView(def, String(value ?? ''), env.sourceOf(def.name));
    case 'expression':
      return expressionView(def.of ? (env.inputs[def.of]?.fields ?? []) : [], env.previewExpression(def.name));
    case 'file':
      return fileView(value as FileValue | null);
    default:
      return null;
  }
}

/** The dialog around its fields: sections, problems, preview, status, footer and places. */
export interface DialogFrame {
  values: Values;
  sections: Sections;
  /** Problems under fields, by parameter. */
  issues: Record<string, string>;
  preview: { text: string; muted: boolean } | null;
  status: StatusLine;
  /** The last run's table (Özet istatistik), shown under the form; null when it gave none. */
  result: TableOutput | null;
  footer: Footer;
  targets: { options: TargetOption[]; hint: string | null; choice: TargetChoice | null };
}

export interface DialogView extends DialogFrame {
  /** The shown fields' changing parts, by parameter. */
  fields: Record<string, FieldView>;
}

export function dialogFrame(tool: ProcessingTool, state: DialogState, env: ViewEnv): DialogFrame {
  const issues = state.runIssues ?? env.issues;
  const shown = shownIssues(issues, state.touched, state.attempted);
  return {
    values: { ...state.values },
    sections: sectionsOf(tool.parameters, state.values, state.advancedOpen, shown),
    issues: shown,
    // The preview waits for the values' own problems only, not for what a run found.
    preview: previewOf(tool, state.values, env.issues),
    status: statusLine(state.status, state.attempted, issues),
    result: state.status.kind === 'ok' && state.status.table ? { columns: [...state.status.table.columns], rows: state.status.table.rows.map((r) => [...r]) } : null,
    footer: footerOf(state.status),
    targets: targetsView(env.targets, state.choice),
  };
}

/** Everything the dialog shows: the frame and the shown fields (Gelişmiş's only when open). */
export function dialogView(tool: ProcessingTool, state: DialogState, env: ViewEnv): DialogView {
  const frame = dialogFrame(tool, state, env);
  const { groups, advanced } = frame.sections;
  const fields: Record<string, FieldView> = {};
  for (const name of [...groups.flatMap((g) => g.rows), ...(advanced?.open ? advanced.rows : [])]) {
    const def = tool.parameters.find((p) => p.name === name)!;
    const view = fieldView(def, state.values[name], env);
    if (view) fields[name] = view;
  }
  return { ...frame, fields };
}
